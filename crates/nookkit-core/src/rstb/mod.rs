mod arena;
mod cache;
mod mods;
mod platform;
mod redirect;

use core::ffi::c_void;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

use nookkit_rstb::sizing::{
    GAME_SIZED, SHADER_ARCHIVE, SHADER_PEAK_MEMORY, SHADER_RESIDENT_MEMORY, Sizer, TABLE_MEMORY,
    UNSUPPORTED, is_model_archive, sizing_memory, table_name,
};
use nookkit_rstb::table::{PATH as TABLE, Table, Update};
use nookkit_rstb::yaz0;
use nookkit_sym::{SymError, sym};

use cache::Cache;
use mods::{Copies, Folder};

const CONFIG: &str = "sd:/config/nookkit/rstb";
// The table the game reads instead of its own
const MODDED_TABLE: &str = "sd:/config/nookkit/rstb/ResourceSizeTable.srsizetable";
const VANILLA_TABLE: &str = "sd:/config/nookkit/rstb/vanilla.srsizetable";
const CACHE: &str = "sd:/config/nookkit/rstb/cache.tsv";
const LOG: &str = "sd:/config/nookkit/rstb/log.txt";
const TEMPORARY: &str = "sd:/config/nookkit/rstb/table.tmp";
// The fingerprint of the files the written table was built from
const STATE: &str = "sd:/config/nookkit/rstb/state.txt";

const ERROR_CODE: u32 = 2;

// game::ResourceMgr's ResourceSizeTable, and that table's RSTC and RSTB entry pointers
const MGR_TABLE: usize = 0x820;
const TABLE_RSTC_ENTRIES: usize = 0x70;
const TABLE_RSTB_ENTRIES: usize = 0x50;

static UPDATED: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct Log(String);

impl Log {
    fn line(&mut self, text: impl Display) {
        let text = text.to_string();
        println!("[NookKit] rstb: {text}");
        self.0 += &text;
        self.0.push('\n');
    }

    // The log file holds the last launch, which starts with a fresh one
    fn append(&mut self) {
        static STARTED: AtomicBool = AtomicBool::new(false);
        let mut text = match STARTED.swap(true, Ordering::Relaxed) {
            true => platform::read_optional(LOG)
                .ok()
                .flatten()
                .unwrap_or_default(),
            false => Vec::new(),
        };
        text.extend_from_slice(self.0.as_bytes());
        if let Err(error) = platform::write(LOG, &text) {
            println!("[NookKit] rstb: {error}");
        }
        self.0.clear();
    }
}

pub fn install() -> Result<(), SymError> {
    table_load::install()?;

    // Plugins start from nn::vi::CreateLayer, which nothing orders before the resource manager's start
    if table_loaded() {
        println!(
            "[NookKit] rstb: the game read its table before NookKit started, the update applies from the next launch"
        );
        update_once(false);
    }
    Ok(())
}

fn table_loaded() -> bool {
    let Ok(instance) = nookkit_sym::data::<usize>(&sym::game::ResourceMgr::sInstance) else {
        return false;
    };
    let mgr = unsafe { *instance };
    if mgr == 0 {
        return false;
    }
    let entries = |offset: usize| unsafe { ((mgr + MGR_TABLE + offset) as *const usize).read() };
    entries(TABLE_RSTC_ENTRIES) != 0 || entries(TABLE_RSTB_ENTRIES) != 0
}

fn update_once(redirect: bool) {
    if UPDATED.swap(true, Ordering::AcqRel) {
        return;
    }
    let mut log = Log::default();
    match scan(&mut log) {
        Ok(Some(scan)) => {
            log.append();
            let (failed, peak) = arena::with_borrowed_memory(scan.need, |loan| update(loan, &scan));
            log.line(format!(
                "Used {} of the {} MiB needed",
                peak.div_ceil(1 << 20),
                scan.need >> 20
            ));
            if !failed {
                let _ = platform::write(STATE, scan.fingerprint.as_bytes());
            }
        }
        Ok(None) => {}
        Err(error) => fail(&mut log, &error),
    }
    log.append();
    // A table from an earlier launch still beats the game's own when this update failed
    if redirect && platform::file_exists(MODDED_TABLE) {
        redirect::install(TABLE, MODDED_TABLE);
    }
}

nookkit_sym::hook! {
    pub table_load = sym::game::ResourceSizeTable::load;
    fn(table: *mut c_void, args: *const c_void) -> bool {
        update_once(true);
        redirect::while_active(|| unsafe { original(table, args) })
    }
}

fn fail(log: &mut Log, error: &str) {
    log.line(format!("Failed: {error}"));
    skyline::error::show_error(
        ERROR_CODE,
        "NookKit could not update the resource size table.",
        &format!("{error}\n\nModded files may crash the game. Details are in {LOG}."),
    );
}

struct Resource {
    relative: String,
    name: String,
    path: String,
    length: u64,
    stamp: Option<String>,
    cached: Option<i64>,
}

// What the update needs, gathered on malloc before any memory is borrowed
struct Scan {
    folders: Vec<Folder>,
    files: BTreeMap<String, Copies>,
    resources: Vec<Resource>,
    epoch: String,
    fingerprint: String,
    need: usize,
}

// `None` when nothing changed since the table was last written
fn scan(log: &mut Log) -> Result<Option<Scan>, String> {
    platform::create_dirs(CONFIG)?;
    let folders = mods::folders()?;
    let (files, counts) = mods::files(&folders)?;
    let listed: Vec<_> = folders
        .iter()
        .zip(&counts)
        .map(|(folder, count)| format!("{} ({count})", folder.root))
        .collect();
    log.line(format!("Modded files: {}", listed.join(", ")));

    let shader = match files.get(SHADER_ARCHIVE) {
        Some(copies) => format!(
            "sd shader {} {}",
            mods::length(copies),
            mods::stamp(copies)?
        ),
        None => "game shader".to_string(),
    };
    let epoch = format!(
        "nookkit {} rstb {} {shader}",
        env!("CARGO_PKG_VERSION"),
        nookkit_rstb::VERSION
    );

    // Every copy's length and stamp, mod tables included, and the written table's
    let mut fingerprint = crc32fast::Hasher::new();
    fingerprint.update(epoch.as_bytes());
    for (relative, copies) in &files {
        fingerprint.update(relative.as_bytes());
        fingerprint.update(&mods::length(copies).to_le_bytes());
        fingerprint.update(mods::stamp(copies).unwrap_or_default().as_bytes());
    }
    if platform::file_exists(MODDED_TABLE) {
        fingerprint.update(platform::stamp(MODDED_TABLE)?.as_bytes());
    }
    let fingerprint = format!("{:08x}", fingerprint.finalize());
    if platform::file_exists(MODDED_TABLE)
        && platform::read_optional(STATE)?.is_some_and(|state| state == fingerprint.as_bytes())
    {
        log.line("Nothing changed since the table was written");
        return Ok(None);
    }

    let cache = Cache::load(CACHE, epoch.clone());
    let mut resources = Vec::new();
    let (mut need, mut largest, mut models) = (TABLE_MEMORY, 0, false);
    for (relative, copies) in &files {
        // Each mod's table is parsed for hints
        if relative == TABLE {
            need += TABLE_MEMORY * copies.len();
        }
        let Some(name) = table_name(relative) else {
            continue;
        };
        let Some(path) = source(log, relative) else {
            continue;
        };
        let length = mods::length(copies);
        let stamp = mods::stamp(copies).ok();
        let cached = stamp
            .as_deref()
            .and_then(|stamp| cache.get(relative, length, stamp));
        if cached.is_none() {
            let (length, decompressed) = platform::lengths(&path);
            largest = largest.max(sizing_memory(name, length, decompressed));
            models |= is_model_archive(name);
        }
        resources.push(Resource {
            relative: relative.clone(),
            name: name.to_string(),
            path,
            length,
            stamp,
            cached,
        });
    }
    need += match models {
        true => SHADER_PEAK_MEMORY.max(SHADER_RESIDENT_MEMORY + largest),
        false => largest,
    };
    // Headroom for talc's fragmentation
    need = (need + need / 4 + (16 << 20)).next_multiple_of(1 << 20);

    Ok(Some(Scan {
        folders,
        files,
        resources,
        epoch,
        fingerprint,
        need,
    }))
}

// Whether it failed
fn update(loan: &arena::Loan, scan: &Scan) -> bool {
    let mut log = Log::default();
    log.line(loan.report.trim_end());
    let outcome = match loan.size {
        0 => Err("No game heap could lend memory to size files".to_string()),
        size if size < loan.need => Err(format!(
            "The game's heaps could lend {} MiB of the {} MiB needed",
            size >> 20,
            loan.need >> 20
        )),
        _ => run(&mut log, scan),
    };
    let failed = outcome.is_err();
    if let Err(error) = outcome {
        fail(&mut log, &error);
    }
    log.append();
    failed
}

fn run(log: &mut Log, scan: &Scan) -> Result<(), String> {
    let mut table = base_table(log, &scan.folders)?;
    size_files(log, &mut table, scan)?;

    if platform::read_optional(MODDED_TABLE)?
        .is_some_and(|bytes| parse_table(&bytes).is_ok_and(|current| current == table))
    {
        log.line("The table is up to date");
        return Ok(());
    }
    let bytes = yaz0::compress(&table.serialize());
    platform::replace(MODDED_TABLE, TEMPORARY, &bytes)?;
    log.line(format!("Wrote the table ({} bytes)", bytes.len()));
    Ok(())
}

// The saved copy of the game's own table, else the romfs table. That one is the game's own only while
// no romfs folder has a table, and a mod's table, vanilla plus its own entries, still serves as the base
// since every modded file is sized again on top
fn base_table(log: &mut Log, folders: &[Folder]) -> Result<Table, String> {
    if let Some(bytes) = platform::read_optional(VANILLA_TABLE)? {
        match parse_table(&bytes) {
            Ok(table) => return Ok(table),
            Err(error) => log.line(format!(
                "The saved copy of the game's table is invalid ({error}), so it is read again"
            )),
        }
    }
    let modded: Vec<_> = folders
        .iter()
        .map(|folder| format!("{}/{TABLE}", folder.root))
        .filter(|path| platform::file_exists(path))
        .collect();
    let bytes = platform::read(&platform::rom_path(TABLE))?;
    let table = parse_table(&bytes)?;
    if modded.is_empty() {
        platform::write(VANILLA_TABLE, &bytes)?;
    } else {
        log.line(format!(
            "The base is a mod's table ({}), the game's own is saved on a launch without one",
            modded.join(", ")
        ));
    }
    Ok(table)
}

fn parse_table(bytes: &[u8]) -> Result<Table, String> {
    yaz0::decompress(bytes)
        .and_then(|data| Table::parse(&data))
        .map_err(|error| error.0)
}

// The game's romfs holds the copy the game loads when several folders replace a file
fn source(log: &mut Log, relative: &str) -> Option<String> {
    let path = platform::rom_path(relative);
    if platform::file_exists(&path) {
        return Some(path);
    }
    log.line(format!(
        "{relative}: not in the game's romfs, so its mod is not loaded, skipped"
    ));
    None
}

fn size_files(log: &mut Log, table: &mut Table, scan: &Scan) -> Result<(), String> {
    let hints: Vec<_> = scan
        .folders
        .iter()
        .filter_map(|folder| {
            let bytes = platform::read_optional(&format!("{}/{TABLE}", folder.root)).ok()??;
            Some((format!("{}/", folder.root), parse_table(&bytes).ok()?))
        })
        .collect();

    let mut cache = Cache::load(CACHE, scan.epoch.clone());
    let mut sizer = Sizer::new(platform::rom_path(SHADER_ARCHIVE));
    let (mut cached, mut changed, mut added, mut removed) = (0, 0, 0, 0);
    for resource in &scan.resources {
        let (relative, name) = (&resource.relative, resource.name.as_str());
        let size = match resource.cached {
            Some(size) => {
                cached += 1;
                size
            }
            None => sizer
                .file_size(&resource.path, name)
                .unwrap_or_else(|error| {
                    log.line(format!("{relative}: {error}, the game sizes it itself"));
                    GAME_SIZED
                }),
        };
        if let Some(stamp) = &resource.stamp {
            cache.insert(relative, resource.length, stamp.clone(), size);
        }
        let update = if size == UNSUPPORTED {
            // The table the file's own mod ships is the best guess left
            let copies = &scan.files[relative];
            let hint = hints
                .iter()
                .filter(|(root, _)| {
                    copies
                        .iter()
                        .any(|(path, _)| path.starts_with(root.as_str()))
                })
                .filter_map(|(_, hints)| hints.get(name))
                .map(|entry| entry.size)
                .max();
            let Some(hint) = hint else {
                // Without an entry the game sizes the file from its decompressed length, which beats
                // an entry the file can't even fit in (game::ResourceMgr::getFallbackLoadSize)
                let (_, decompressed) = platform::lengths(&resource.path);
                if table
                    .get(name)
                    .is_some_and(|entry| decompressed > entry.size as u64)
                {
                    log.line(format!(
                        "{relative}: no traced size and larger than its entry, which is removed so the game sizes it"
                    ));
                    table.set(name, GAME_SIZED).map_err(|error| error.0)?;
                    removed += 1;
                } else {
                    log.line(format!(
                        "{relative}: no traced size, its table entry was left as it is"
                    ));
                }
                continue;
            };
            log.line(format!(
                "{relative}: no traced size, raised to its mod's entry {hint} at least"
            ));
            table.raise(name, hint)
        } else {
            table.set(name, size)
        };
        match update {
            Ok(Update::Changed) => changed += 1,
            Ok(Update::Added) => added += 1,
            Ok(Update::Removed) => removed += 1,
            Ok(Update::None) => {}
            Err(error) => log.line(format!("{relative}: {error}")),
        }
    }
    if let Some(error) = sizer.shader_error() {
        log.line(format!(
            "The shader archive could not be read ({error}), the game sizes geometry models itself"
        ));
    }
    if let Err(error) = cache.save(CACHE) {
        log.line(error);
    }
    log.line(format!(
        "{} resources, {} sized, {cached} from the cache, against the game's table {changed} entries changed, {added} added, {removed} removed",
        scan.resources.len(),
        scan.resources.len() - cached
    ));
    Ok(())
}

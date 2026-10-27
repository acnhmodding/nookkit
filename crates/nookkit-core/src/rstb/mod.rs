mod arena;
mod cache;
mod mods;
mod platform;
mod redirect;

use core::ffi::c_void;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};

use nookkit_rstb::sizing::{GAME_SIZED, SHADER_ARCHIVE, Sizer, UNSUPPORTED, table_name};
use nookkit_rstb::table::{PATH as TABLE, Table, Update};
use nookkit_rstb::yaz0;
use nookkit_sym::{SymError, sym};

use cache::Cache;
use mods::Folder;

const CONFIG: &str = "sd:/config/nookkit/rstb";
// The table the game reads instead of its own
const MODDED_TABLE: &str = "sd:/config/nookkit/rstb/ResourceSizeTable.srsizetable";
// Read while no table in a romfs folder hides the game's own
const VANILLA_TABLE: &str = "sd:/config/nookkit/rstb/vanilla.srsizetable";
const CACHE: &str = "sd:/config/nookkit/rstb/cache.tsv";
const LOG: &str = "sd:/config/nookkit/rstb/log.txt";
const TEMPORARY: &str = "sd:/config/nookkit/rstb/table.tmp";

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

    fn save(&self) {
        if let Err(error) = platform::write(LOG, self.0.as_bytes()) {
            println!("[NookKit] rstb: {error}");
        }
    }
}

enum Outcome {
    Ready,
    // The game would read a file that is gone this launch, so it must start again
    Restart(&'static str),
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
    if arena::with_borrowed_memory(update) {
        skyline::nn::oe::exit_application();
    }
    // A table from an earlier launch still beats the game's own when this update failed
    if redirect && platform::file_exists(MODDED_TABLE) {
        redirect::install(TABLE, MODDED_TABLE);
    }
}

nookkit_sym::hook! {
    pub table_load = sym::game::ResourceSizeTable::load;
    fn(table: *mut c_void, args: *const c_void) -> bool {
        update_once(true);
        unsafe { original(table, args) }
    }
}

// Whether the game must close
fn update(loan: &arena::Loan) -> bool {
    let mut log = Log::default();
    log.line(loan.report.trim_end());
    let outcome = match loan.size {
        // malloc's 32 MiB, which the game shares, is too little for model and shader data
        0 => Err("No game heap could lend enough memory to size files".to_string()),
        _ => run(&mut log),
    };
    match outcome {
        Ok(Outcome::Ready) => {
            log.save();
            false
        }
        Ok(Outcome::Restart(reason)) => {
            log.line(reason);
            log.save();
            skyline::error::show_error(ERROR_CODE, "NookKit: please start the game again.", reason);
            true
        }
        Err(error) => {
            log.line(format!("Failed: {error}"));
            log.save();
            skyline::error::show_error(
                ERROR_CODE,
                "NookKit could not update the resource size table.",
                &format!("{error}\n\nModded files may crash the game. Details are in {LOG}."),
            );
            false
        }
    }
}

fn run(log: &mut Log) -> Result<Outcome, String> {
    platform::create_dirs(CONFIG)?;
    let folders = mods::folders()?;
    let Some(vanilla) = vanilla_table(log, &folders)? else {
        return Ok(Outcome::Restart(
            "A resource size table in a romfs folder hid the game's own, so it was moved to the config folder. The game's table can be read from the next launch.",
        ));
    };

    let mut table = vanilla;
    size_files(log, &mut table, &folders)?;

    if platform::read_optional(MODDED_TABLE)?
        .is_some_and(|bytes| parse_table(&bytes).is_ok_and(|current| current == table))
    {
        log.line("The table is up to date");
        return Ok(Outcome::Ready);
    }
    let bytes = yaz0::compress(&table.serialize());
    platform::replace(MODDED_TABLE, TEMPORARY, &bytes)?;
    log.line(format!("Wrote the table ({} bytes)", bytes.len()));
    Ok(Outcome::Ready)
}

// The saved copy, else the romfs table, which is the game's own only while no romfs folder has one.
// Tables found there are moved away and `None` returned, as the romfs keeps showing them this launch
fn vanilla_table(log: &mut Log, folders: &[Folder]) -> Result<Option<Table>, String> {
    let modded: Vec<_> = folders
        .iter()
        .map(|folder| (folder, format!("{}/{TABLE}", folder.root)))
        .filter(|(_, path)| platform::file_exists(path))
        .collect();
    if let Some(bytes) = platform::read_optional(VANILLA_TABLE)? {
        match parse_table(&bytes) {
            Ok(table) => {
                for (_, path) in &modded {
                    log.line(format!(
                        "{path} is ignored, the game reads {MODDED_TABLE}, built from its own table"
                    ));
                }
                return Ok(Some(table));
            }
            Err(error) => log.line(format!(
                "The saved copy of the game's table is invalid ({error}), so it is read again"
            )),
        }
    }
    if !modded.is_empty() {
        for (folder, path) in &modded {
            let moved = format!("{CONFIG}/moved-from-{}.srsizetable", folder.name);
            platform::rename(path, &moved)?;
            log.line(format!("Moved {path} to {moved}"));
        }
        return Ok(None);
    }
    let bytes = platform::read(&platform::rom_path(TABLE))?;
    let table = parse_table(&bytes)?;
    platform::write(VANILLA_TABLE, &bytes)?;
    Ok(Some(table))
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

fn size_files(log: &mut Log, table: &mut Table, folders: &[Folder]) -> Result<(), String> {
    let (files, counts) = mods::files(folders)?;
    let listed: Vec<_> = folders
        .iter()
        .zip(&counts)
        .map(|(folder, count)| format!("{} ({count})", folder.root))
        .collect();
    log.line(format!("Modded files: {}", listed.join(", ")));

    let shader_copies = files.get(SHADER_ARCHIVE);
    let shader_path = platform::rom_path(SHADER_ARCHIVE);
    let shader = match shader_copies {
        Some(copies) => format!(
            "sd shader {} {}",
            mods::length(copies),
            mods::stamp(copies)?
        ),
        None => "game shader".to_string(),
    };
    let mut cache = Cache::load(
        CACHE,
        format!(
            "nookkit {} rstb {} {shader}",
            env!("CARGO_PKG_VERSION"),
            nookkit_rstb::VERSION
        ),
    );
    let mut sizer = Sizer::new(shader_path);
    let (mut sized, mut cached, mut changed, mut added, mut removed) = (0, 0, 0, 0, 0);
    for (relative, copies) in &files {
        let Some(name) = table_name(relative) else {
            continue;
        };
        let Some(path) = source(log, relative) else {
            continue;
        };
        let length = mods::length(copies);
        let stamp = mods::stamp(copies);
        let size = match stamp
            .as_deref()
            .ok()
            .and_then(|stamp| cache.get(relative, length, stamp))
        {
            Some(size) => {
                cached += 1;
                size
            }
            None => sizer.file_size(&path, name).unwrap_or_else(|error| {
                log.line(format!("{relative}: {error}, the game sizes it itself"));
                GAME_SIZED
            }),
        };
        if let Ok(stamp) = stamp {
            cache.insert(relative, length, stamp, size);
        }
        if size == UNSUPPORTED {
            log.line(format!(
                "{relative}: no traced size, its table entry was left as it is"
            ));
            continue;
        }
        match table.set(name, size) {
            Ok(Update::Changed) => changed += 1,
            Ok(Update::Added) => added += 1,
            Ok(Update::Removed) => removed += 1,
            Ok(Update::None) => {}
            Err(error) => log.line(format!("{relative}: {error}")),
        }
        sized += 1;
    }
    if let Some(error) = sizer.shader_error() {
        log.line(format!(
            "The shader archive could not be read ({error}), the game sizes geometry models itself"
        ));
    }
    if let Err(error) = cache.save(CACHE) {
        log.line(error);
    }
    log.line(format!("{} files, {sized} sized ({cached} from the cache), against the game's table {changed} entries changed, {added} added, {removed} removed", files.len()));
    Ok(())
}

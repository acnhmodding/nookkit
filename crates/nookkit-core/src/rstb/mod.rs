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
    arena::with_borrowed_memory(update);
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

fn update(loan: &arena::Loan) {
    let mut log = Log::default();
    log.line(loan.report.trim_end());
    let outcome = match loan.size {
        // malloc's 32 MiB, which the game shares, is too little for model and shader data
        0 => Err("No game heap could lend enough memory to size files".to_string()),
        _ => run(&mut log),
    };
    if let Err(error) = outcome {
        log.line(format!("Failed: {error}"));
        skyline::error::show_error(
            ERROR_CODE,
            "NookKit could not update the resource size table.",
            &format!("{error}\n\nModded files may crash the game. Details are in {LOG}."),
        );
    }
    log.save();
}

fn run(log: &mut Log) -> Result<(), String> {
    platform::create_dirs(CONFIG)?;
    let folders = mods::folders()?;
    let mut table = base_table(log, &folders)?;
    size_files(log, &mut table, &folders)?;

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

fn size_files(log: &mut Log, table: &mut Table, folders: &[Folder]) -> Result<(), String> {
    let (files, counts) = mods::files(folders)?;
    let hints: Vec<_> = folders
        .iter()
        .filter_map(|folder| {
            let bytes = platform::read_optional(&format!("{}/{TABLE}", folder.root)).ok()??;
            Some((format!("{}/", folder.root), parse_table(&bytes).ok()?))
        })
        .collect();
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
        let update = if size == UNSUPPORTED {
            // The table the file's own mod ships is the best guess left
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
                log.line(format!(
                    "{relative}: no traced size, its table entry was left as it is"
                ));
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

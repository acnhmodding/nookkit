use core::ffi::c_void;
use std::collections::BTreeMap;
use std::sync::{Mutex, OnceLock};

use nookkit_byml::PatchSet;
use nookkit_sym::sym;
use nookkit_types::game::file_cache::LoadRequest;
use nookkit_types::sead::SafeString;

const FOLDER: &str = "content:/NookKit/Pack";
const EXTENSION: &str = ".pack";

static PATCHES: OnceLock<PatchSet> = OnceLock::new();
// The patched archive each thread is loading, since `insert` only gets the file's own path
static LOADING: Mutex<BTreeMap<usize, &'static str>> = Mutex::new(BTreeMap::new());

/// Every patch, a broken one is reported and skipped
fn load_patches() -> PatchSet {
    let mut set = PatchSet::default();
    let files = match crate::content::json_files(FOLDER) {
        Ok(files) => files,
        Err(e) => {
            println!("[NookKit] pack patches: {e}");
            return set;
        }
    };

    let mut read = 0;
    for (name, json) in files {
        let json = match json {
            Ok(json) => json,
            Err(e) => {
                println!("[NookKit] pack patch {name}: {e}");
                continue;
            }
        };
        read += 1;

        match set.add(&name, &json) {
            Ok(replaced) => {
                for replaced in replaced {
                    println!("[NookKit] pack patch {name} {replaced}");
                }
            }
            Err(e) => println!("[NookKit] pack patch {e}"),
        }
    }

    if !set.is_empty() {
        println!(
            "[NookKit] {} patched pack files from {} files",
            set.len(),
            read
        );
    }
    set
}

fn patches() -> Option<&'static PatchSet> {
    Some(PATCHES.get_or_init(load_patches)).filter(|set| !set.is_empty())
}

/// "Pack/StaticParam.pack" names the archive "StaticParam"
fn archive_name(path: &str) -> Option<&str> {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    name.strip_suffix(EXTENSION)
}

fn current_thread() -> usize {
    unsafe { skyline::nn::os::GetCurrentThread() as usize }
}

/// Marks this thread as loading `archive`, returning what it was loading before
fn set_loading(archive: Option<&'static str>) -> Option<&'static str> {
    let mut loading = LOADING.lock().unwrap_or_else(|e| e.into_inner());
    let thread = current_thread();
    match archive {
        Some(archive) => loading.insert(thread, archive),
        None => loading.remove(&thread),
    }
}

fn loading() -> Option<&'static str> {
    let loading = LOADING.lock().unwrap_or_else(|e| e.into_inner());
    loading.get(&current_thread()).copied()
}

/// The patched file, none when no patch takes it or it doesn't apply
fn patch_file(archive: &str, file: &str, game_file: &[u8]) -> Option<Vec<u8>> {
    let entry = patches()?.get(archive, file)?;
    let bytes = match entry.apply(game_file) {
        Ok(bytes) => bytes,
        Err(e) => {
            println!(
                "[NookKit] pack patch {} on {archive}/{file}: {e}",
                entry.patch
            );
            return None;
        }
    };
    if u32::try_from(bytes.len()).is_err() {
        println!(
            "[NookKit] pack patch {} on {archive}/{file}: too large",
            entry.patch
        );
        return None;
    }
    println!(
        "[NookKit] {archive}/{file}: from {}, {} bytes, was {}",
        entry.patch,
        bytes.len(),
        game_file.len()
    );
    Some(bytes)
}

// The game reads every file of an archive through this, once per archive it hasn't cached
nookkit_sym::hook! {
    pub load = sym::game::FileCache::load;
    fn(cache: *mut c_void, request: *const LoadRequest<'static>) -> *mut c_void {
        let archive = patches().and_then(|patches| {
            let path = unsafe { request.as_ref()?.path.as_str()? };
            let name = archive_name(path)?;
            patches.archives().find(|archive| *archive == name)
        });
        let Some(archive) = archive else {
            return unsafe { original(cache, request) };
        };

        let previous = set_loading(Some(archive));
        let node = unsafe { original(cache, request) };
        set_loading(previous);
        node
    }
}

// Copies the file into the game's own memory, so the patched bytes can go right after
nookkit_sym::hook! {
    pub insert = sym::game::FileCache::insert;
    fn(cache: *mut c_void, request: *const LoadRequest<'static>, holder: *mut c_void, path: *const SafeString<'static>, data: *const u8, size: u32) -> *mut c_void {
        let patched = loading().and_then(|archive| {
            let file = unsafe { path.as_ref()?.as_str()? };
            if data.is_null() {
                return None;
            }
            let game_file = unsafe { core::slice::from_raw_parts(data, size as usize) };
            patch_file(archive, file, game_file)
        });

        match patched {
            Some(bytes) => unsafe { original(cache, request, holder, path, bytes.as_ptr(), bytes.len() as u32) },
            None => unsafe { original(cache, request, holder, path, data, size) },
        }
    }
}

pub fn install() -> Result<(), nookkit_sym::SymError> {
    load::install()?;
    insert::install()
}

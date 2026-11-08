use core::ffi::c_void;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Mutex, MutexGuard, OnceLock};

use nookkit_byml::PatchSet;
use nookkit_sym::sym;
use nookkit_types::game::file_cache::LoadRequest;
use nookkit_types::sead::SafeString;

const FOLDER: &str = "content:/NookKit/Pack";
const EXTENSION: &str = ".pack";

static PATCHES: OnceLock<PatchSet> = OnceLock::new();
// What each thread is loading, since `insert` only gets the file's own path
static LOADING: Mutex<BTreeMap<usize, Loading>> = Mutex::new(BTreeMap::new());
// The archives whose patches were checked against the files the game cached
static CHECKED: Mutex<BTreeSet<&'static str>> = Mutex::new(BTreeSet::new());

/// A patched archive being loaded, and what the game cached from it so far
struct Loading {
    archive: &'static str,
    cached: usize,
    patched: BTreeSet<String>,
}

impl Loading {
    fn new(archive: &'static str) -> Self {
        Self {
            archive,
            cached: 0,
            patched: BTreeSet::new(),
        }
    }
}

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
    let patches = PATCHES.get_or_init(load_patches);
    if patches.is_empty() {
        return None;
    }
    Some(patches)
}

/// The patched archive `request` loads, none when it isn't one
fn patched_archive(request: *const LoadRequest<'static>) -> Option<&'static str> {
    let patches = patches()?;
    let request = unsafe { request.as_ref()? };
    let path = unsafe { request.path.as_str()? };
    let name = archive_name(path)?;
    patches.archives().find(|archive| *archive == name)
}

/// "Pack/StaticParam.pack" names the archive "StaticParam"
fn archive_name(path: &str) -> Option<&str> {
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    name.strip_suffix(EXTENSION)
}

fn current_thread() -> usize {
    unsafe { skyline::nn::os::GetCurrentThread() as usize }
}

fn loading() -> MutexGuard<'static, BTreeMap<usize, Loading>> {
    LOADING.lock().unwrap_or_else(|e| e.into_inner())
}

/// Notes `file` as cached from the archive this thread is loading, returning the archive
fn cache_file(file: &str) -> Option<&'static str> {
    let mut loading = loading();
    let loading = loading.get_mut(&current_thread())?;
    loading.cached += 1;
    if patches()?.get(loading.archive, file).is_some() {
        loading.patched.insert(file.to_owned());
    }
    Some(loading.archive)
}

/// Reports the patches of a file the archive doesn't have, once per archive. JSON only
/// replaces files, a new one goes in the romfs as a loose file
fn check_missing(loaded: &Loading) {
    // Nothing was cached when the game already had the archive
    if loaded.cached == 0 {
        return;
    }
    let mut checked = CHECKED.lock().unwrap_or_else(|e| e.into_inner());
    if !checked.insert(loaded.archive) {
        return;
    }
    let Some(patches) = patches() else {
        return;
    };
    for (file, entry) in patches.files(loaded.archive) {
        if !loaded.patched.contains(file) {
            println!(
                "[NookKit] pack patch {} on {}/{file}: not in the archive, add a new file as a loose file",
                entry.patch, loaded.archive
            );
        }
    }
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

/// The patched bytes of the file the game is caching, none when it keeps its own
fn patched_insert(path: *const SafeString<'static>, data: *const u8, size: u32) -> Option<Vec<u8>> {
    let path = unsafe { path.as_ref()? };
    let file = unsafe { path.as_str()? };
    let archive = cache_file(file)?;
    if data.is_null() {
        return None;
    }
    let game_file = unsafe { core::slice::from_raw_parts(data, size as usize) };
    patch_file(archive, file, game_file)
}

// The game reads every file of an archive through this, once per archive it hasn't cached
nookkit_sym::hook! {
    pub load = sym::game::FileCache::load;
    fn(cache: *mut c_void, request: *const LoadRequest<'static>) -> *mut c_void {
        let Some(archive) = patched_archive(request) else {
            return unsafe { original(cache, request) };
        };

        let thread = current_thread();
        let previous = loading().insert(thread, Loading::new(archive));
        let node = unsafe { original(cache, request) };

        let mut loading = loading();
        let loaded = match previous {
            Some(previous) => loading.insert(thread, previous),
            None => loading.remove(&thread),
        };
        drop(loading);
        if let Some(loaded) = loaded {
            check_missing(&loaded);
        }
        node
    }
}

// Copies the file into the game's own memory, so the patched bytes can go right after
nookkit_sym::hook! {
    pub insert = sym::game::FileCache::insert;
    fn(cache: *mut c_void, request: *const LoadRequest<'static>, holder: *mut c_void, path: *const SafeString<'static>, data: *const u8, size: u32) -> *mut c_void {
        let Some(bytes) = patched_insert(path, data, size) else {
            return unsafe { original(cache, request, holder, path, data, size) };
        };
        unsafe { original(cache, request, holder, path, bytes.as_ptr(), bytes.len() as u32) }
    }
}

pub fn install() -> Result<(), nookkit_sym::SymError> {
    load::install()?;
    insert::install()
}

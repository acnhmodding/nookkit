// nn::fs::OpenFile and GetEntryType are SDK functions, and the game reads the table through them as
// "<mount>:/System/Resource/...", so the read never reaches LayeredFS, which keeps each file's length
// from when the game started
use std::ffi::{CStr, CString};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

use skyline::nn::fs::{DirectoryEntryType, FileHandle};

struct Redirect {
    table: &'static str,
    target: CString,
}

static REDIRECT: OnceLock<Redirect> = OnceLock::new();
static USED: AtomicBool = AtomicBool::new(false);
// Only while the game loads its table, so every other open skips the path compare
static ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn install(table: &'static str, target: &str) {
    let Ok(target) = CString::new(target) else {
        return;
    };
    if REDIRECT.set(Redirect { table, target }).is_ok() {
        skyline::install_hooks!(open_file, get_entry_type);
    }
}

pub fn while_active<T>(f: impl FnOnce() -> T) -> T {
    ACTIVE.store(REDIRECT.get().is_some(), Ordering::Release);
    let result = f();
    ACTIVE.store(false, Ordering::Release);
    result
}

fn redirect(path: *const u8) -> *const u8 {
    if !ACTIVE.load(Ordering::Acquire) {
        return path;
    }
    let Some(redirect) = REDIRECT.get() else {
        return path;
    };
    if path.is_null() {
        return path;
    }
    let bytes = unsafe { CStr::from_ptr(path as _) }.to_bytes();
    let Some(mount_end) = bytes.windows(2).position(|w| w == b":/") else {
        return path;
    };
    let relative = &bytes[mount_end + 2..];
    let relative = &relative[relative.iter().take_while(|&&b| b == b'/').count()..];
    if relative != redirect.table.as_bytes() {
        return path;
    }
    if !USED.swap(true, Ordering::Relaxed) {
        println!(
            "[NookKit] rstb: the game reads its table from {}",
            redirect.target.to_string_lossy()
        );
    }
    redirect.target.as_ptr() as _
}

#[skyline::hook(replace = skyline::nn::fs::OpenFile)]
fn open_file(handle: *mut FileHandle, path: *const u8, mode: i32) -> u32 {
    original!()(handle, redirect(path), mode)
}

#[skyline::hook(replace = skyline::nn::fs::GetEntryType)]
fn get_entry_type(entry_type: *mut DirectoryEntryType, path: *const u8) -> u32 {
    original!()(entry_type, redirect(path))
}

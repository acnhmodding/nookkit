use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};

use skyline::nn;

const ENTRY_FILE: u32 = 1;

// The game's romfs with its mods applied, mounted by sead::FileDeviceMgr before the table loads
pub fn rom_path(relative: &str) -> String {
    format!("content:/{relative}")
}

fn c_path(path: &str) -> Option<CString> {
    CString::new(path).ok()
}

pub fn file_exists(path: &str) -> bool {
    let Some(path) = c_path(path) else {
        return false;
    };
    let mut entry_type = 0;
    unsafe {
        nn::fs::GetEntryType(&mut entry_type, path.as_ptr() as _) == 0 && entry_type == ENTRY_FILE
    }
}

pub fn read(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|e| format!("Reading {path} failed: {e}"))
}

pub fn read_optional(path: &str) -> Result<Option<Vec<u8>>, String> {
    if !file_exists(path) {
        return Ok(None);
    }
    read(path).map(Some)
}

// Sized first, as Skyline's std can't write past a file's end
pub fn write(path: &str, data: &[u8]) -> Result<(), String> {
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)?;
        file.set_len(data.len() as u64)?;
        file.write_all(data)?;
        file.flush()
    })();
    result.map_err(|e| format!("Writing {path} failed: {e}"))
}

// Written completely at `temporary` first, so an interrupted write never leaves a partial file
pub fn replace(path: &str, temporary: &str, data: &[u8]) -> Result<(), String> {
    write(temporary, data)?;
    rename(temporary, path)
}

pub fn rename(from: &str, to: &str) -> Result<(), String> {
    remove(to)?;
    fs::rename(from, to).map_err(|e| format!("Moving {from} to {to} failed: {e}"))
}

pub fn remove(path: &str) -> Result<(), String> {
    if !file_exists(path) {
        return Ok(());
    }
    fs::remove_file(path).map_err(|e| format!("Deleting {path} failed: {e}"))
}

pub fn create_dirs(path: &str) -> Result<(), String> {
    let root = path.find(":/").map_or(0, |at| at + 2);
    for (at, _) in path
        .match_indices('/')
        .filter(|&(at, _)| at > root)
        .chain([(path.len(), "")])
    {
        // Existing directories fail to be created, the final check covers real failures
        let _ = fs::create_dir(&path[..at]);
    }
    if fs::read_dir(path).is_err() {
        return Err(format!("Creating {path} failed."));
    }
    Ok(())
}

pub fn dir_exists(path: &str) -> bool {
    fs::read_dir(path).is_ok()
}

pub fn dirs(root: &str) -> Result<Vec<String>, String> {
    let Ok(entries) = fs::read_dir(root) else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| format!("Listing {root} failed: {e}"))?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry
            .metadata()
            .map_err(|e| format!("Reading {root}/{name} failed: {e}"))?
            .is_dir()
        {
            out.push(name);
        }
    }
    out.sort();
    Ok(out)
}

// Paths relative to `root` with their lengths, none when `root` doesn't exist
pub fn files(root: &str) -> Result<Vec<(String, u64)>, String> {
    fn walk(root: &str, relative: &str, out: &mut Vec<(String, u64)>) -> Result<(), String> {
        let dir = if relative.is_empty() {
            root.to_string()
        } else {
            format!("{root}/{relative}")
        };
        let entries = fs::read_dir(&dir).map_err(|e| format!("Listing {dir} failed: {e}"))?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("Listing {dir} failed: {e}"))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            let path = if relative.is_empty() {
                name
            } else {
                format!("{relative}/{name}")
            };
            let metadata = entry
                .metadata()
                .map_err(|e| format!("Reading {dir}/{path} failed: {e}"))?;
            if metadata.is_dir() {
                walk(root, &path, out)?;
            } else {
                out.push((path, metadata.len()));
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    if fs::read_dir(root).is_ok() {
        walk(root, "", &mut out)?;
    }
    out.sort();
    Ok(out)
}

// Identifies a file's contents for the size cache by its time stamps, or a hash of its bytes when the
// file system has none
pub fn stamp(path: &str) -> Result<String, String> {
    let mut times = nn::fs::FileTimeStamp::new();
    if let Some(c) = c_path(path)
        && unsafe { nn::fs::GetFileTimeStampForDebug(&mut times, c.as_ptr() as _) } == 0
        && times.modify.time != 0
    {
        return Ok(format!("t{:x}-{:x}", times.create.time, times.modify.time));
    }
    let mut file = fs::File::open(path).map_err(|e| format!("Reading {path} failed: {e}"))?;
    let mut hasher = crc32fast::Hasher::new();
    let mut buffer = vec![0u8; 0x10000];
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|e| format!("Reading {path} failed: {e}"))?;
        if read == 0 {
            return Ok(format!("c{:08x}", hasher.finalize()));
        }
        hasher.update(&buffer[..read]);
    }
}

// sead::FileDeviceMgr mounts the romfs, mods included, as "content"
use std::ffi::CString;

const ENTRY_DIRECTORY: u32 = 0;

// nn::fs aborts the game on some failures, GetEntryType reports a missing folder instead
fn folder_exists(folder: &str) -> bool {
    let Ok(path) = CString::new(folder) else {
        return false;
    };
    let mut kind = u32::MAX;
    unsafe {
        skyline::nn::fs::GetEntryType(&mut kind, path.as_ptr() as _) == 0 && kind == ENTRY_DIRECTORY
    }
}

/// The `.json` files in `folder` in name order, as (file name, text). A file that can't be read is
/// reported as a `what` and skipped
pub fn json_files(folder: &str, what: &str) -> Vec<(String, String)> {
    if !folder_exists(folder) {
        return Vec::new();
    }

    let mut paths: Vec<_> = match std::fs::read_dir(folder) {
        Ok(dir) => dir.filter_map(|e| Some(e.ok()?.path())).collect(),
        Err(_) => Vec::new(),
    };
    paths.retain(|p| p.extension().is_some_and(|e| e == "json"));
    paths.sort();

    paths
        .iter()
        .filter_map(|path| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            match std::fs::read_to_string(path) {
                Ok(json) => Some((name, json)),
                Err(e) => {
                    println!("[NookKit] {what} {name}: {e}");
                    None
                }
            }
        })
        .collect()
}

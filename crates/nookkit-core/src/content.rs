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
        skyline::nn::fs::GetEntryType(&raw mut kind, path.as_ptr().cast()) == 0
            && kind == ENTRY_DIRECTORY
    }
}

/// The `.json` files in `folder` in name order, as (file name, text), none when there's no
/// folder. Each is read once reached, so one text is in memory at a time
pub fn json_files(
    folder: &str,
) -> std::io::Result<impl Iterator<Item = (String, std::io::Result<String>)>> {
    let mut paths = Vec::new();
    if folder_exists(folder) {
        for entry in std::fs::read_dir(folder)? {
            paths.push(entry?.path());
        }
    }
    paths.retain(|p| p.extension().is_some_and(|e| e == "json"));
    paths.sort();

    Ok(paths.into_iter().map(|path| {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();

        (name, std::fs::read_to_string(&path))
    }))
}

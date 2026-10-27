use std::collections::BTreeMap;

use super::platform;

const TITLE: &str = "01006F8002326000";
// Atmosphere's LayeredFS reads `<title>/romfs`, which mod managers merge mods into. Emulators also load
// `<title>/<mod>/romfs` from it
const ATMOSPHERE: &str = "sd:/atmosphere/contents";
// Emulators load each mod from `<mod folder>/<title>/<mod>/romfs` outside the emulated SD card, which a
// link from the SD card's `load` folder makes visible here
const MOD_LOAD: &str = "sd:/load";

pub struct Folder {
    pub root: String,
}

// An emulator's SD card is a host folder, case-sensitive on Linux, so the title id may be either case
fn title_dir(parent: &str) -> Result<Option<String>, String> {
    let name = platform::dirs(parent)?
        .into_iter()
        .find(|name| name.eq_ignore_ascii_case(TITLE));
    Ok(name.map(|name| format!("{parent}/{name}")))
}

fn mod_folders(title: &str, folders: &mut Vec<Folder>) -> Result<(), String> {
    for name in platform::dirs(title)? {
        let root = format!("{title}/{name}/romfs");
        if !["romfs", "exefs"].contains(&name.as_str()) && platform::dir_exists(&root) {
            folders.push(Folder { root });
        }
    }
    Ok(())
}

pub fn folders() -> Result<Vec<Folder>, String> {
    let mut folders = Vec::new();
    if let Some(title) = title_dir(ATMOSPHERE)? {
        folders.push(Folder {
            root: format!("{title}/romfs"),
        });
        mod_folders(&title, &mut folders)?;
    }
    if let Some(title) = title_dir(MOD_LOAD)? {
        mod_folders(&title, &mut folders)?;
    }
    Ok(folders)
}

// (path, length) per copy, in folder order
pub type Copies = Vec<(String, u64)>;

// By romfs path, with each folder's file count
pub fn files(folders: &[Folder]) -> Result<(BTreeMap<String, Copies>, Vec<usize>), String> {
    let mut files = BTreeMap::<String, Copies>::new();
    let mut counts = Vec::with_capacity(folders.len());
    for folder in folders {
        let listed = platform::files(&folder.root)?;
        counts.push(listed.len());
        for (relative, length) in listed {
            let path = format!("{}/{relative}", folder.root);
            files.entry(relative).or_default().push((path, length));
        }
    }
    Ok((files, counts))
}

pub fn length(copies: &Copies) -> u64 {
    copies.iter().map(|(_, length)| length).sum()
}

pub fn stamp(copies: &Copies) -> Result<String, String> {
    let stamps: Result<Vec<_>, _> = copies
        .iter()
        .map(|(path, _)| platform::stamp(path))
        .collect();
    Ok(stamps?.join(","))
}

use std::collections::BTreeMap;

use super::platform;

// Atmosphere's LayeredFS romfs, which mod managers merge mods into
const ATMOSPHERE: &str = "sd:/atmosphere/contents/01006F8002326000/romfs";
// Emulators load each mod from `<mod folder>/01006F8002326000/<mod>/romfs` outside the emulated SD
// card, which a link from the SD card's `load` folder makes visible here
const MOD_LOAD_ROOT: &str = "sd:/load/01006F8002326000";

pub struct Folder {
    pub name: String,
    pub root: String,
}

pub fn folders() -> Result<Vec<Folder>, String> {
    let mut folders = vec![Folder {
        name: "romfs".to_string(),
        root: ATMOSPHERE.to_string(),
    }];
    for name in platform::dirs(MOD_LOAD_ROOT)? {
        let root = format!("{MOD_LOAD_ROOT}/{name}/romfs");
        if platform::dir_exists(&root) {
            folders.push(Folder {
                name: format!("mod-{name}"),
                root,
            });
        }
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

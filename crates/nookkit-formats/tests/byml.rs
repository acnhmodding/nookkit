#![expect(clippy::unwrap_used, reason = "test helpers fail the test")]

use std::path::{Path, PathBuf};

use nookkit_formats::byml::{Byml, Endian, Node, ParseError};

/// Every `.byml` under `folder`, and every one in a `.pack`, as (name, data)
fn files(romfs: &Path) -> Vec<(String, Vec<u8>)> {
    fn walk(folder: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(folder).unwrap() {
            let path = entry.unwrap().path();

            if path.is_dir() {
                walk(&path, out);
            } else {
                out.push(path);
            }
        }
    }

    let mut paths = Vec::new();
    walk(romfs, &mut paths);
    paths.sort();

    let mut files = Vec::new();

    for path in paths {
        let name = path
            .strip_prefix(romfs)
            .unwrap()
            .to_string_lossy()
            .into_owned();

        match path.extension().and_then(|e| e.to_str()) {
            Some("byml") => files.push((name, std::fs::read(&path).unwrap())),
            Some("pack") => {
                let data = std::fs::read(&path).unwrap();
                for member in nookkit_formats::sarc::read(&data).unwrap() {
                    if let Some(file) = member.name.as_deref().filter(|n| n.ends_with(".byml")) {
                        files.push((format!("{name}/{file}"), member.data(&data).to_vec()));
                    }
                }
            }
            _ => {}
        }
    }

    files
}

/// The header and table checks `sead::ByamlIter` makes before reading a file
fn game_accepts(data: &[u8]) -> bool {
    let big = data.starts_with(b"BY");

    let u32_at = |at: usize| {
        let bytes: [u8; 4] = data[at..at + 4].try_into().unwrap();

        if big {
            u32::from_be_bytes(bytes)
        } else {
            u32::from_le_bytes(bytes)
        }
    };

    let u24_at = |at: usize| {
        let [a, b, c] = data[at..at + 3].try_into().unwrap();

        if big {
            u32::from_be_bytes([0, a, b, c])
        } else {
            u32::from_le_bytes([a, b, c, 0])
        }
    };

    let cstr = |at: usize| {
        let end = at + data[at..].iter().position(|&b| b == 0).unwrap();
        &data[at..end]
    };

    // The table's end, or None when the game refuses it
    let table = |offset: u32| -> Option<u32> {
        if offset == 0 {
            return Some(0);
        }
        let at = offset as usize;
        let count = u24_at(at + 1) as usize;
        if data[at] != 0xc2 || count == 0 {
            return None;
        }
        let offsets: Vec<u32> = (0..=count).map(|i| u32_at(at + 4 + i * 4)).collect();
        let sorted_offsets = offsets.windows(2).all(|w| w[0] < w[1]);
        let terminated = offsets[1..].iter().all(|&o| data[at + o as usize - 1] == 0);
        let sorted = offsets[..count]
            .windows(2)
            .all(|w| cstr(at + w[0] as usize) <= cstr(at + w[1] as usize));
        let first = offsets[0] == count as u32 * 4 + 8;
        (sorted_offsets && terminated && sorted && first).then(|| offset + offsets[count])
    };

    let (keys, strings, root) = (u32_at(4), u32_at(8), u32_at(0xc));

    let (Some(keys_end), Some(strings_end)) = (table(keys), table(strings)) else {
        return false;
    };

    if root == 0 {
        return keys == 0 && strings == 0;
    }

    if keys != 0 && ((strings != 0 && strings < keys_end) || root < keys_end) {
        return false;
    }

    strings == 0 || strings_end <= root
}

#[test]
fn every_byml_round_trips() {
    let Ok(romfs) = std::env::var("NOOKKIT_ROMFS") else {
        eprintln!("NOOKKIT_ROMFS unset, skipped");
        return;
    };

    let files = files(Path::new(&romfs));
    let (mut skipped, mut as_json, mut big) = (Vec::new(), 0usize, 0usize);
    let mut failures = Vec::new();

    for (name, data) in &files {
        let byml = match Byml::parse(data) {
            Ok(byml) => byml,
            Err(ParseError::Version(version)) => {
                skipped.push(format!("{name}: version {version}"));
                continue;
            }
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };

        big += (byml.format.endian == Endian::Big) as usize;

        let written = match byml.to_bytes() {
            Ok(written) => written,
            Err(e) => {
                failures.push(format!("{name}: {e}"));
                continue;
            }
        };

        if !game_accepts(&written) {
            failures.push(format!("{name}: the game would refuse it"));
        }

        if Byml::parse(&written).as_ref() != Ok(&byml) {
            failures.push(format!("{name}: reads back differently"));
        }

        if let Ok(json) = serde_json::to_string(&byml.root) {
            as_json += 1;

            if serde_json::from_str::<Node>(&json).ok().as_ref() != Some(&byml.root) {
                failures.push(format!("{name}: its JSON reads back differently"));
            }
        }
    }

    eprintln!(
        "{} files, {big} big endian, {as_json} as JSON, skipped {skipped:?}",
        files.len()
    );

    failures.truncate(20);
    assert!(failures.is_empty(), "{failures:#?}");
}

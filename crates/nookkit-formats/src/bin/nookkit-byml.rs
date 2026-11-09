use std::path::Path;
use std::process::ExitCode;

use nookkit_formats::byml::{Byml, PatchSet};

fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn json(path: &str, member: Option<&str>) -> Result<(), String> {
    let data = read(Path::new(path))?;

    let (name, byml) = match member {
        Some(member) => {
            let members = nookkit_formats::sarc::read(&data).map_err(|e| format!("{path}: {e}"))?;

            let found = members
                .iter()
                .find(|m| m.is(member))
                .ok_or_else(|| format!("{path}: no {member}"))?;

            (member, Byml::parse(found.data(&data)))
        }
        None => (path, Byml::parse(&data)),
    };

    let byml = byml.map_err(|e| format!("{name}: {e}"))?;
    let json = serde_json::to_string_pretty(&byml.root).map_err(|e| format!("{name}: {e}"))?;
    println!("{json}");

    Ok(())
}

fn check(romfs: &str, paths: &[String]) -> Result<(), String> {
    let mut set = PatchSet::default();
    let mut failed = false;

    for path in paths {
        let json = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;

        match set.add(path, &json) {
            Ok(replaced) => {
                for e in replaced {
                    eprintln!("{path}: {e}");
                }
            }
            Err(e) => {
                eprintln!("{e}");
                failed = true;
            }
        }
    }

    for archive in set.archives() {
        let path = Path::new(romfs)
            .join("Pack")
            .join(format!("{archive}.pack"));

        let data = read(&path)?;

        let members =
            nookkit_formats::sarc::read(&data).map_err(|e| format!("{}: {e}", path.display()))?;

        for (file, entry) in set.files(archive) {
            let Some(member) = members.iter().find(|m| m.is(file)) else {
                eprintln!("{archive}/{file}: the pack has no such file");
                failed = true;
                continue;
            };

            let original = member.data(&data);

            match entry.apply(original) {
                Ok(patched) => eprintln!(
                    "{archive}/{file}: {} bytes, was {}",
                    patched.len(),
                    original.len()
                ),
                Err(e) => {
                    eprintln!("{archive}/{file}: {e}");
                    failed = true;
                }
            }
        }
    }

    if failed {
        return Err("some patches don't apply".into());
    }

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = match args.as_slice() {
        [command, path] if command == "json" => json(path, None),
        [command, pack, member] if command == "json" => json(pack, Some(member)),
        [command, romfs, paths @ ..] if command == "check" && !paths.is_empty() => {
            check(romfs, paths)
        }
        _ => Err("usage: nookkit-byml json <file.byml | file.pack member>\n       nookkit-byml check <romfs> <patch.json…>".into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

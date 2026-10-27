use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

use nookkit_msbt::msbt::Msbt;
use nookkit_msbt::{Message, Patch, PatchSet, sarc, tags};

fn unzstd(path: &Path) -> Result<Vec<u8>, String> {
    let file = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut out = Vec::new();

    ruzstd::decoding::StreamingDecoder::new(file.as_slice())
        .map_err(|e| format!("{}: {e}", path.display()))?
        .read_to_end(&mut out)
        .map_err(|e| format!("{}: {e}", path.display()))?;

    Ok(out)
}

/// Every locale's copy of `file` in `romfs/Message/<archive>_<locale>.sarc.zs`
fn locales(romfs: &Path, archive: &str, file: &str) -> Result<BTreeMap<String, Msbt>, String> {
    let folder = romfs.join("Message");
    let prefix = format!("{archive}_");
    let wanted = format!("{file}.msbt");
    let mut out = BTreeMap::new();

    let mut names: Vec<String> = std::fs::read_dir(&folder)
        .map_err(|e| format!("{}: {e}", folder.display()))?
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter(|name| name.starts_with(&prefix) && name.ends_with(".sarc.zs"))
        .collect();

    names.sort();

    for name in names {
        let locale = &name[prefix.len()..name.len() - ".sarc.zs".len()];
        let archive_bytes = unzstd(&folder.join(&name))?;
        let files = sarc::read(&archive_bytes).ok_or_else(|| format!("{name}: not a SARC"))?;

        if let Some((_, data)) = files.iter().find(|(path, _)| *path == wanted) {
            let msbt = Msbt::parse(data).ok_or_else(|| format!("{name}/{wanted}: not an MSBT"))?;
            out.insert(locale.to_owned(), msbt);
        }
    }

    if out.is_empty() {
        return Err(format!(
            "no {wanted} in {}/{prefix}*.sarc.zs",
            folder.display()
        ));
    }

    Ok(out)
}

fn dump(romfs: &str, archive: &str, file: &str, only: &[String]) -> Result<(), String> {
    let locales = locales(Path::new(romfs), archive, file)?;

    let mut labels: Vec<&String> = locales.values().flat_map(|m| m.labels.keys()).collect();
    labels.sort();
    labels.dedup();

    let mut messages = Vec::new();
    for label in labels {
        if !only.is_empty() && !only.contains(label) {
            continue;
        }

        let mut text = BTreeMap::new();
        for (locale, msbt) in &locales {
            if let Some(units) = msbt.text(label) {
                let decoded = tags::decode(units)
                    .ok_or_else(|| format!("{locale} {label}: malformed text"))?;

                text.insert(locale.clone(), decoded);
            }
        }

        messages.push(Message {
            label: label.clone(),
            text,
            attributes: None,
        });
    }

    let patch = Patch {
        file: file.to_owned(),
        archive: Some(archive.to_owned()),
        messages,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&patch).map_err(|e| e.to_string())?
    );
    Ok(())
}

fn check(paths: &[String]) -> Result<(), String> {
    let mut set = PatchSet::default();
    let mut failed = false;

    for path in paths {
        let json = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;

        match set.add(path, &json) {
            Ok(replaced) => {
                for label in replaced {
                    eprintln!("{path}: replaces {label}");
                }
            }
            Err(e) => {
                eprintln!("{e}");
                failed = true;
            }
        }
    }

    if failed {
        return Err("some patches don't load".into());
    }

    eprintln!("{} labels in {} patches", set.len(), paths.len());
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = match args.as_slice() {
        [command, romfs, archive, file, only @ ..] if command == "dump" => {
            dump(romfs, archive, file, only)
        }
        [command, paths @ ..] if command == "check" && !paths.is_empty() => check(paths),
        _ => Err("usage: nookkit-msbt dump <romfs> <archive> <file> [label…]\n       nookkit-msbt check <patch.json…>".into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

//! Every message of an unpacked 3.0.3 romfs survives decode then encode unchanged. Set
//! NOOKKIT_ROMFS to the romfs folder, without it the test passes without checking anything
use std::io::Read;
use std::path::Path;

use nookkit_msbt::msbt::Msbt;
use nookkit_msbt::{sarc, tags};

#[test]
fn every_message_round_trips() {
    let Ok(romfs) = std::env::var("NOOKKIT_ROMFS") else {
        eprintln!("NOOKKIT_ROMFS unset, skipped");
        return;
    };

    let folder = Path::new(&romfs).join("Message");
    let mut archives: Vec<_> = std::fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.to_string_lossy().ends_with(".sarc.zs"))
        .collect();
    archives.sort();

    let (mut messages, mut failures) = (0usize, Vec::new());

    for path in &archives {
        let compressed = std::fs::read(path).unwrap();
        let mut data = Vec::new();
        ruzstd::decoding::StreamingDecoder::new(compressed.as_slice())
            .unwrap()
            .read_to_end(&mut data)
            .unwrap();

        for (name, file) in sarc::read(&data).unwrap() {
            if !name.ends_with(".msbt") {
                continue;
            }
            let msbt = Msbt::parse(file).unwrap_or_else(|| panic!("{name} isn't an MSBT"));

            for (index, units) in msbt.texts.iter().enumerate() {
                messages += 1;
                let decoded = tags::decode(units);
                let encoded = decoded.as_deref().map(tags::encode);

                if encoded.as_ref().map(|e| e.as_ref().ok()) != Some(Some(units)) && failures.len() < 10 {
                    failures.push(format!(
                        "{} {name} #{index}: {decoded:?}",
                        path.file_name().unwrap().to_string_lossy()
                    ));
                }
            }
        }
    }

    eprintln!("{messages} messages in {} archives", archives.len());
    assert!(failures.is_empty(), "{failures:#?}");
}

use std::path::Path;

use nookkit_formats::bcsv::Table;

#[test]
fn every_table_round_trips() {
    let Ok(romfs) = std::env::var("NOOKKIT_ROMFS") else {
        eprintln!("NOOKKIT_ROMFS unset, skipped");
        return;
    };

    let mut paths: Vec<_> = std::fs::read_dir(Path::new(&romfs).join("Bcsv"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "bcsv"))
        .collect();

    paths.sort();

    let mut failures = Vec::new();

    for path in &paths {
        let data = std::fs::read(path).unwrap();
        let name = path.file_name().unwrap().to_string_lossy();

        match Table::parse(&data) {
            Ok(table) if table.to_bytes(table.row_count).as_deref() == Ok(&data[..]) => {}
            Ok(_) => failures.push(format!("{name}: written differently")),
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }

    eprintln!("{} tables", paths.len());
    assert!(failures.is_empty(), "{failures:#?}");
}

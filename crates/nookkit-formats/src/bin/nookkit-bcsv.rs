use std::path::Path;
use std::process::ExitCode;

use nookkit_formats::bcsv::{PatchSet, Table};

fn check(romfs: &str, paths: &[String]) -> Result<(), String> {
    let mut set = PatchSet::default();
    let mut failed = false;

    for path in paths {
        let json = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;

        if let Err(e) = set.add(path, &json) {
            eprintln!("{e}");
            failed = true;
        }
    }

    for file in set.files() {
        let path = Path::new(romfs).join("Bcsv").join(format!("{file}.bcsv"));
        let data = std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let table = Table::parse(&data).map_err(|e| format!("{}: {e}", path.display()))?;

        let plan = set.plan(file, &table);

        for e in &plan.skipped {
            eprintln!("{e}");
        }

        failed |= !plan.skipped.is_empty();

        eprintln!(
            "{file}: {} rows replaced, {} added",
            plan.replaced, plan.added
        );
    }

    if failed {
        return Err("some patches don't apply".into());
    }

    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let result = match args.as_slice() {
        [command, romfs, paths @ ..] if command == "check" && !paths.is_empty() => {
            check(romfs, paths)
        }
        _ => Err("usage: nookkit-bcsv check <romfs> <patch.json…>".into()),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

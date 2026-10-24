mod parse;

#[cfg(feature = "skyline")]
mod runtime;

pub use parse::{Kind, ParseError, SymbolFile, parse};

#[cfg(feature = "skyline")]
pub use runtime::*;

pub const FINGERPRINT_OFFSET: usize = 0x3f6_0000;
pub const FINGERPRINT_LEN: usize = 0x1_0000;

pub fn fingerprint(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |hash, &b| {
        (hash ^ b as u64).wrapping_mul(0x0100_0000_01b3)
    })
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SymError {
    #[error("no symbol file loaded")]
    NotLoaded,
    #[error("symbol \"{name}\" missing from [{}]", kind.section())]
    Missing { kind: Kind, name: String },
    #[error(
        "unsupported game build: version {version}, fingerprint {actual:#x} (file expects {expected:#x})"
    )]
    UnsupportedBuild {
        version: String,
        expected: u64,
        actual: u64,
    },
    #[error("bad symbol file {source_name}: {error}")]
    Parse {
        source_name: String,
        error: ParseError,
    },
}

pub fn manifest(text: &str) -> impl Iterator<Item = Result<(Kind, &str), &str>> {
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| {
            let (kind, name) = l.split_once('\t').ok_or(l)?;
            let kind = [
                Kind::Fn,
                Kind::Got,
                Kind::Data,
                Kind::TypeInfo,
                Kind::TypeInfoGuard,
            ]
            .into_iter()
            .find(|k| k.section() == kind)
            .ok_or(l)?;
            Ok((kind, name))
        })
}

pub fn missing<'a>(
    file: &SymbolFile,
    manifest_text: &'a str,
) -> Vec<Result<(Kind, &'a str), &'a str>> {
    manifest(manifest_text)
        .filter(|entry| match entry {
            Ok((kind, name)) => file.get(*kind, name).is_none(),
            Err(_) => true,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_fnv1a() {
        assert_eq!(fingerprint(b""), 0xcbf2_9ce4_8422_2325);
        assert_eq!(fingerprint(b"a"), 0xaf63_dc4c_8601_ec8c);
    }

    #[test]
    fn shipped_file_covers_the_manifests() {
        let file = parse(include_str!("../../../offsets/3.0.3.toml")).unwrap();
        assert_eq!(file.game_version, "3.0.3");
        for manifest in [
            include_str!("../../acnh-game/symbols.txt"),
            include_str!("../../acnh-ui/symbols.txt"),
            include_str!("../../acnh-mouth/symbols.txt"),
        ] {
            assert_eq!(missing(&file, manifest), vec![]);
        }
    }

    #[test]
    fn manifest_reports_missing_and_malformed() {
        let file =
            parse("game_version = \"3.0.3\"\nrodata_hash = 0x1\n[fn]\n\"A::b\" = 0x10\n").unwrap();
        let text = "# core\nfn\tA::b\nfn\tA::c\ntype_info\tB\nbogus line\n";
        let missing = missing(&file, text);
        assert_eq!(
            missing,
            vec![
                Ok((Kind::Fn, "A::c")),
                Ok((Kind::TypeInfo, "B")),
                Err("bogus line")
            ]
        );
    }
}

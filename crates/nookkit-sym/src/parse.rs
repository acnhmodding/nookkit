use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Fn,

    /// The address an object's vptr holds
    Vtable,
}

impl Kind {
    pub fn section(self) -> &'static str {
        match self {
            Kind::Fn => "fn",
            Kind::Vtable => "vtable",
        }
    }

    pub fn from_section(name: &str) -> Option<Self> {
        [Kind::Fn, Kind::Vtable]
            .into_iter()
            .find(|k| k.section() == name)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error("line {line}: {reason}")]
    Syntax { line: usize, reason: &'static str },
    #[error("line {line}: unknown section [{section}]")]
    UnknownSection { line: usize, section: String },
    #[error("line {line}: \"{name}\" is listed twice under [{section}]")]
    Duplicate {
        line: usize,
        section: &'static str,
        name: String,
    },
    #[error("missing header field `{0}`")]
    MissingHeader(&'static str),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolFile {
    pub game_version: String,
    pub rodata_hash: u64,
    entries: HashMap<(Kind, String), u64>,
}

impl SymbolFile {
    pub fn get(&self, kind: Kind, name: &str) -> Option<u64> {
        self.entries.get(&(kind, name.to_owned())).copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (Kind, &str, u64)> {
        self.entries
            .iter()
            .map(|((kind, name), &offset)| (*kind, name.as_str(), offset))
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// The line without its `#` comment, which can't start inside a quoted name like `{lambda()#1}`
pub fn strip_comment(line: &str) -> &str {
    let mut quoted = false;

    for (i, c) in line.char_indices() {
        match c {
            '"' => quoted = !quoted,
            '#' if !quoted => return &line[..i],
            _ => {}
        }
    }

    line
}

fn parse_hex(value: &str) -> Option<u64> {
    u64::from_str_radix(value.strip_prefix("0x")?, 16).ok()
}

fn parse_quoted(value: &str) -> Option<&str> {
    value.strip_prefix('"')?.strip_suffix('"')
}

pub fn parse(text: &str) -> Result<SymbolFile, ParseError> {
    let mut game_version = None;
    let mut rodata_hash = None;
    let mut section = None;
    let mut entries = HashMap::new();

    for (index, raw) in text.lines().enumerate() {
        let line = index + 1;
        let content = strip_comment(raw).trim();

        if content.is_empty() {
            continue;
        }

        if let Some(name) = content.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            section = Some(
                Kind::from_section(name).ok_or_else(|| ParseError::UnknownSection {
                    line,
                    section: name.to_owned(),
                })?,
            );
            continue;
        }

        let (key, value) = content
            .split_once('=')
            .map(|(k, v)| (k.trim(), v.trim()))
            .ok_or(ParseError::Syntax {
                line,
                reason: "expected `key = value`",
            })?;

        let Some(kind) = section else {
            match key {
                "game_version" => {
                    let v = parse_quoted(value).ok_or(ParseError::Syntax {
                        line,
                        reason: "game_version must be a string",
                    })?;
                    game_version = Some(v.to_owned());
                }
                "rodata_hash" => {
                    rodata_hash = Some(parse_hex(value).ok_or(ParseError::Syntax {
                        line,
                        reason: "rodata_hash must be hex",
                    })?);
                }
                _ => {
                    return Err(ParseError::Syntax {
                        line,
                        reason: "unknown header field",
                    });
                }
            }
            continue;
        };

        let name = parse_quoted(key)
            .filter(|name| !name.is_empty())
            .ok_or(ParseError::Syntax {
                line,
                reason: "symbol names must be quoted and not empty",
            })?;
        let offset = parse_hex(value).ok_or(ParseError::Syntax {
            line,
            reason: "offsets must be 0x hex",
        })?;

        if entries.insert((kind, name.to_owned()), offset).is_some() {
            return Err(ParseError::Duplicate {
                line,
                section: kind.section(),
                name: name.to_owned(),
            });
        }
    }

    Ok(SymbolFile {
        game_version: game_version.ok_or(ParseError::MissingHeader("game_version"))?,
        rodata_hash: rodata_hash.ok_or(ParseError::MissingHeader("rodata_hash"))?,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
game_version = "3.0.3"
rodata_hash = 0x1234abcd

[fn]
"Lp::UI::Control::findByPath" = 0x376e6b0  # comment
[vtable]
"Lp::UI::UniteBtnCtrl::vtable" = 0x4a92730
"#;

    #[test]
    fn parses_header_and_sections() {
        let f = parse(SAMPLE).unwrap();
        assert_eq!(f.game_version, "3.0.3");
        assert_eq!(f.rodata_hash, 0x1234abcd);
        assert_eq!(
            f.get(Kind::Fn, "Lp::UI::Control::findByPath"),
            Some(0x376e6b0)
        );
        assert_eq!(
            f.get(Kind::Vtable, "Lp::UI::UniteBtnCtrl::vtable"),
            Some(0x4a92730)
        );
        assert_eq!(f.get(Kind::Fn, "Lp::UI::UniteBtnCtrl::vtable"), None);
        assert_eq!(f.len(), 2);
    }

    #[test]
    fn keeps_hashes_inside_names() {
        let text = "game_version = \"1\"\nrodata_hash = 0x1\n[fn]\n\"A::{lambda()#1}\" = 0x10 # c\n";
        assert_eq!(parse(text).unwrap().get(Kind::Fn, "A::{lambda()#1}"), Some(0x10));
        assert!(matches!(
            parse("game_version = \"1\"\nrodata_hash = 0x1\n[fn]\n\"\" = 0x1\n"),
            Err(ParseError::Syntax { .. })
        ));
    }

    #[test]
    fn rejects_duplicates_and_bad_lines() {
        let dup = format!("{SAMPLE}\n[fn]\n\"Lp::UI::Control::findByPath\" = 0x1\n");
        assert!(matches!(parse(&dup), Err(ParseError::Duplicate { .. })));
        assert!(matches!(
            parse("game_version = \"1\"\nrodata_hash = 0x1\n[nope]\n"),
            Err(ParseError::UnknownSection { .. })
        ));
        assert!(matches!(
            parse("game_version = \"1\"\n"),
            Err(ParseError::MissingHeader("rodata_hash"))
        ));
        assert!(matches!(
            parse("game_version = \"1\"\nrodata_hash = 0x1\n[fn]\nname = 0x1\n"),
            Err(ParseError::Syntax { .. })
        ));
    }
}

//! Message text with `{{Name key="value"}}` tags, to and from MsgStd UTF-16 (0x0E opens a tag with
//! group, type, byte size and params, 0x0F closes one with group and type)

use std::fmt::{self, Write};

const OPEN: u16 = 0x0E;
const CLOSE: u16 = 0x0F;

// LibMessageStudio's system group, the only one with known names
const SYSTEM: u16 = 0;
const RUBY: u16 = 0;
const FONT: u16 = 1;
const SIZE: u16 = 2;
const COLOR: u16 = 3;
const PAGE_BREAK: u16 = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagError {
    pub at: usize,
    pub reason: String,
}

impl fmt::Display for TagError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: {}", self.at, self.reason)
    }
}

impl std::error::Error for TagError {}

fn fail<T>(at: usize, reason: impl Into<String>) -> Result<T, TagError> {
    Err(TagError {
        at,
        reason: reason.into(),
    })
}

/// `text` as NUL-terminated MsgStd UTF-16
pub fn encode(text: &str) -> Result<Vec<u16>, TagError> {
    let mut out = Vec::with_capacity(text.len() + 1);
    let mut rest = text;

    while !rest.is_empty() {
        let at = text.len() - rest.len();

        if let Some(after) = rest.strip_prefix("{{{{") {
            out.extend("{{".encode_utf16());
            rest = after;
        } else if let Some(after) = rest.strip_prefix("{{") {
            let Some(end) = after.find("}}") else {
                return fail(at, "a tag without its closing }}");
            };
            encode_tag(&after[..end], at, &mut out)?;
            rest = &after[end + 2..];
        } else {
            let next = rest.find("{{").unwrap_or(rest.len());
            out.extend(rest[..next].encode_utf16());
            rest = &rest[next..];
        }
    }

    out.push(0);
    Ok(out)
}

struct Attrs<'a> {
    pairs: Vec<(&'a str, String)>,
}

impl<'a> Attrs<'a> {
    fn parse(mut s: &'a str, at: usize) -> Result<Self, TagError> {
        let mut pairs = Vec::new();

        loop {
            s = s.trim_start();
            if s.is_empty() {
                return Ok(Self { pairs });
            }

            let Some(eq) = s.find('=') else {
                return fail(at, format!("expected key=\"value\" in {s:?}"));
            };
            let key = s[..eq].trim();
            s = s[eq + 1..].trim_start();

            let value = if let Some(quoted) = s.strip_prefix('"') {
                let mut value = String::new();
                let mut chars = quoted.char_indices();
                let end = loop {
                    match chars.next() {
                        Some((i, '"')) => break i,
                        Some((_, '\\')) => match chars.next() {
                            Some((_, c)) => value.push(c),
                            None => return fail(at, "an escape at the end of a value"),
                        },
                        Some((_, c)) => value.push(c),
                        None => return fail(at, format!("unterminated value for {key}")),
                    }
                };
                s = &quoted[end + 1..];
                value
            } else {
                let end = s.find(char::is_whitespace).unwrap_or(s.len());
                let value = s[..end].to_owned();
                s = &s[end..];
                value
            };

            pairs.push((key, value));
        }
    }

    fn take(&mut self, key: &str, at: usize) -> Result<String, TagError> {
        match self.pairs.iter().position(|(k, _)| *k == key) {
            Some(i) => Ok(self.pairs.remove(i).1),
            None => fail(at, format!("missing {key}")),
        }
    }

    fn take_u16(&mut self, key: &str, at: usize) -> Result<u16, TagError> {
        let value = self.take(key, at)?;
        match value.strip_prefix("0x") {
            Some(hex) => u16::from_str_radix(hex, 16),
            None => value.parse(),
        }
        .or_else(|_| fail(at, format!("{key}={value:?} isn't a u16")))
    }

    fn done(self, at: usize) -> Result<(), TagError> {
        match self.pairs.first() {
            Some((key, _)) => fail(at, format!("unknown attribute {key}")),
            None => Ok(()),
        }
    }
}

fn push_tag(
    out: &mut Vec<u16>,
    group: u16,
    kind: u16,
    params: &[u8],
    at: usize,
) -> Result<(), TagError> {
    if !params.len().is_multiple_of(2) || params.len() > u16::MAX as usize {
        return fail(at, "tag params must be an even number of bytes");
    }

    out.extend([OPEN, group, kind, params.len() as u16]);
    out.extend(
        params
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]])),
    );
    Ok(())
}

fn encode_tag(body: &str, at: usize, out: &mut Vec<u16>) -> Result<(), TagError> {
    let body = body.trim();
    let (name, rest) = body.split_once(char::is_whitespace).unwrap_or((body, ""));
    let mut attrs = Attrs::parse(rest, at)?;

    match name {
        "Ruby" => {
            let span = attrs.take_u16("byteSpan", at)?;
            let value: Vec<u8> = attrs
                .take("value", at)?
                .encode_utf16()
                .flat_map(u16::to_le_bytes)
                .collect();

            let mut params = Vec::with_capacity(4 + value.len());
            params.extend(span.to_le_bytes());
            params.extend((value.len() as u16).to_le_bytes());
            params.extend(value);
            push_tag(out, SYSTEM, RUBY, &params, at)?;
        }
        "Font" => push_tag(out, SYSTEM, FONT, &attrs.take_u16("face", at)?.to_le_bytes(), at)?,
        "Size" => push_tag(out, SYSTEM, SIZE, &attrs.take_u16("percent", at)?.to_le_bytes(), at)?,
        "Color" => push_tag(out, SYSTEM, COLOR, &attrs.take_u16("index", at)?.to_le_bytes(), at)?,
        "PageBreak" => push_tag(out, SYSTEM, PAGE_BREAK, &[], at)?,
        "Tag" => {
            let group = attrs.take_u16("group", at)?;
            let kind = attrs.take_u16("type", at)?;
            let data = match attrs.take("data", at) {
                Ok(hex) => from_hex(&hex).ok_or_else(|| TagError {
                    at,
                    reason: format!("data={hex:?} isn't hex"),
                })?,
                Err(_) => Vec::new(),
            };
            push_tag(out, group, kind, &data, at)?;
        }
        "/Tag" => out.extend([
            CLOSE,
            attrs.take_u16("group", at)?,
            attrs.take_u16("type", at)?,
        ]),
        _ => return fail(at, format!("unknown tag {name}")),
    }

    attrs.done(at)
}

pub(crate) fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

pub(crate) fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::new(), |mut out, b| {
        let _ = write!(out, "{b:02x}");
        out
    })
}

fn quoted(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
}

fn params_u16(params: &[u8]) -> Option<u16> {
    (params.len() == 2).then(|| u16::from_le_bytes([params[0], params[1]]))
}

fn ruby(params: &[u8]) -> Option<(u16, String)> {
    let span = u16::from_le_bytes([*params.first()?, *params.get(1)?]);
    let len = u16::from_le_bytes([*params.get(2)?, *params.get(3)?]) as usize;
    let value = params.get(4..)?;

    if value.len() != len || !len.is_multiple_of(2) {
        return None;
    }
    let units: Vec<u16> = value
        .chunks_exact(2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .collect();
    let value = String::from_utf16(&units).ok()?;

    // Only when it reads back the same, a reading holding a NUL or "}}" stays raw
    (!value.contains('\0') && !value.contains("}}")).then_some((span, value))
}

fn decode_tag(out: &mut String, group: u16, kind: u16, params: &[u8]) {
    let named = match (group, kind) {
        (SYSTEM, RUBY) => ruby(params).map(|(span, value)| {
            let mut tag = format!("{{{{Ruby byteSpan=\"{span}\" value=");
            quoted(&mut tag, &value);
            tag
        }),
        (SYSTEM, FONT) => params_u16(params).map(|v| format!("{{{{Font face=\"{v}\"")),
        (SYSTEM, SIZE) => params_u16(params).map(|v| format!("{{{{Size percent=\"{v}\"")),
        (SYSTEM, COLOR) => params_u16(params).map(|v| format!("{{{{Color index=\"{v}\"")),
        (SYSTEM, PAGE_BREAK) if params.is_empty() => Some("{{PageBreak".to_owned()),
        _ => None,
    };

    match named {
        Some(tag) => out.push_str(&tag),
        None => {
            let _ = write!(out, "{{{{Tag group=\"{group}\" type=\"{kind}\"");
            if !params.is_empty() {
                let _ = write!(out, " data=\"{}\"", to_hex(params));
            }
        }
    }
    out.push_str("}}");
}

fn flush(out: &mut String, plain: &mut Vec<u16>) -> Option<()> {
    let text = String::from_utf16(plain).ok()?;
    out.push_str(&text.replace("{{", "{{{{"));
    plain.clear();
    Some(())
}

/// MsgStd UTF-16 as tagged text, up to its NUL (tags hold zeros of their own). `None` when a tag
/// runs past the end or the text isn't valid UTF-16
pub fn decode(units: &[u16]) -> Option<String> {
    let mut out = String::with_capacity(units.len());
    let mut plain = Vec::new();
    let mut i = 0;

    while i < units.len() {
        match units[i] {
            OPEN => {
                flush(&mut out, &mut plain)?;
                let (group, kind, size) = (
                    *units.get(i + 1)?,
                    *units.get(i + 2)?,
                    *units.get(i + 3)? as usize,
                );
                if !size.is_multiple_of(2) {
                    return None;
                }
                let params: Vec<u8> = units
                    .get(i + 4..i + 4 + size / 2)?
                    .iter()
                    .flat_map(|u| u.to_le_bytes())
                    .collect();
                decode_tag(&mut out, group, kind, &params);
                i += 4 + size / 2;
            }
            CLOSE => {
                flush(&mut out, &mut plain)?;
                let (group, kind) = (*units.get(i + 1)?, *units.get(i + 2)?);
                let _ = write!(out, "{{{{/Tag group=\"{group}\" type=\"{kind}\"}}}}");
                i += 3;
            }
            0 => break,
            unit => {
                plain.push(unit);
                i += 1;
            }
        }
    }

    flush(&mut out, &mut plain)?;
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn round_trip(text: &str) {
        let units = encode(text).unwrap();
        assert_eq!(decode(&units).as_deref(), Some(text));
    }

    #[test]
    fn plain_and_newlines() {
        round_trip("Island Sketch");
        round_trip("two\nlines");
        assert_eq!(encode("ab").unwrap(), vec![b'a' as u16, b'b' as u16, 0]);
    }

    #[test]
    fn system_tags() {
        round_trip("a{{PageBreak}}b");
        round_trip("{{Ruby byteSpan=\"4\" value=\"げんご\"}}言語");
        round_trip("{{Color index=\"1\"}}red{{Color index=\"65535\"}}");
        round_trip("{{Size percent=\"150\"}}big");
    }

    #[test]
    fn ruby_bytes() {
        // A real JPja ruby: 4 bytes of base text read なまえ
        let units = encode("{{Ruby byteSpan=\"4\" value=\"なまえ\"}}").unwrap();
        assert_eq!(&units[..4], &[OPEN, 0, 0, 10]);
        assert_eq!(&units[4..6], &[4, 6]);
    }

    #[test]
    fn raw_tags() {
        round_trip("{{Tag group=\"10\" type=\"2\"}}");
        round_trip("{{Tag group=\"115\" type=\"0\" data=\"00cd\"}}x{{/Tag group=\"10\" type=\"5\"}}");
    }

    #[test]
    fn escapes() {
        round_trip("{{{{not a tag}}");
        round_trip("{{Ruby byteSpan=\"2\" value=\"a\\\"b\"}}");
    }

    #[test]
    fn errors() {
        assert!(encode("{{Nope}}").is_err());
        assert!(encode("{{Ruby value=\"x\"}}").is_err());
        assert!(encode("{{PageBreak").is_err());
        assert!(encode("{{Tag group=\"1\" type=\"2\" data=\"abc\"}}").is_err());
    }
}

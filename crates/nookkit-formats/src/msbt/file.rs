use std::collections::BTreeMap;

use crate::bytes::{OutOfBounds, Reader};

pub struct Msbt {
    /// Label to message index
    pub labels: BTreeMap<String, u32>,

    /// Each message as raw UTF-16, NUL included
    pub texts: Vec<Vec<u16>>,

    /// Each message's ATR1 entry, empty when the file has none
    pub attributes: Vec<Vec<u8>>,
    pub attribute_size: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ParseError {
    #[error(transparent)]
    OutOfBounds(#[from] OutOfBounds),

    #[error("not an MSBT")]
    NotMsbt,

    #[error("no {0} section")]
    MissingSection(&'static str),

    #[error("the label at 0x{offset:X} isn't UTF-8")]
    LabelUtf8 { offset: usize },
}

fn sections(b: &[u8]) -> Result<BTreeMap<[u8; 4], &[u8]>, ParseError> {
    let reader = Reader::new(b);

    if !reader.magic_at(0, b"MsgStdBn")? || reader.u16_at(8)? != 0xFEFF {
        return Err(ParseError::NotMsbt);
    }

    let count = reader.u16_at(0xE)?;
    let mut at = 0x20;
    let mut out = BTreeMap::new();

    for _ in 0..count {
        let magic = reader.array_at::<4>(at)?;
        let size = reader.u32_at(at + 4)? as usize;
        out.insert(magic, reader.bytes_at(at + 16, size)?);
        at = (at + 16 + size + 15) & !15;
    }

    Ok(out)
}

fn section<'a>(
    sections: &BTreeMap<[u8; 4], &'a [u8]>,
    name: &'static str,
) -> Result<Reader<'a>, ParseError> {
    let Some(&data) = name
        .as_bytes()
        .first_chunk()
        .and_then(|magic| sections.get(magic))
    else {
        return Err(ParseError::MissingSection(name));
    };

    Ok(Reader::new(data))
}

impl Msbt {
    pub fn parse(b: &[u8]) -> Result<Self, ParseError> {
        let sections = sections(b)?;

        let lbl = section(&sections, "LBL1")?;
        let mut labels = BTreeMap::new();

        for bucket in 0..lbl.u32_at(0)? as usize {
            let count = lbl.u32_at(4 + bucket * 8)?;
            let mut at = lbl.u32_at(8 + bucket * 8)? as usize;

            for _ in 0..count {
                let len = lbl.u8_at(at)? as usize;

                let Ok(name) = std::str::from_utf8(lbl.bytes_at(at + 1, len)?) else {
                    return Err(ParseError::LabelUtf8 { offset: at });
                };

                labels.insert(name.to_owned(), lbl.u32_at(at + 1 + len)?);
                at += 1 + len + 4;
            }
        }

        let txt = section(&sections, "TXT2")?;
        let count = txt.u32_at(0)? as usize;

        let mut offsets = Vec::with_capacity(count + 1);

        for i in 0..count {
            offsets.push(txt.u32_at(4 + i * 4)? as usize);
        }

        offsets.push(txt.len());

        let mut texts = Vec::with_capacity(count);

        for pair in offsets.windows(2) {
            let (start, end) = (pair[0], pair[1]);

            let Some(len) = end.checked_sub(start) else {
                return Err(OutOfBounds {
                    offset: start,
                    len: 0,
                }
                .into());
            };

            let bytes = txt.bytes_at(start, len)?;

            texts.push(
                bytes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|&c| u16::from_le_bytes(c))
                    .collect(),
            );
        }

        let (attributes, attribute_size) = match section(&sections, "ATR1") {
            Ok(atr) => {
                let size = atr.u32_at(4)?;

                let mut entries = Vec::new();

                for i in 0..atr.u32_at(0)? as usize {
                    let at = 8 + i * size as usize;
                    entries.push(atr.bytes_at(at, size as usize)?.to_vec());
                }

                (entries, size)
            }
            Err(_) => (Vec::new(), 0),
        };

        Ok(Self {
            labels,
            texts,
            attributes,
            attribute_size,
        })
    }

    pub fn text(&self, label: &str) -> Option<&[u16]> {
        self.texts
            .get(*self.labels.get(label)? as usize)
            .map(Vec::as_slice)
    }

    pub fn attribute(&self, label: &str) -> Option<&[u8]> {
        self.attributes
            .get(*self.labels.get(label)? as usize)
            .map(Vec::as_slice)
    }
}

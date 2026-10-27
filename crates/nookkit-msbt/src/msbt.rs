//! Reading MsgStdBn files: labels, texts and ATR1 attributes

use std::collections::BTreeMap;

pub struct Msbt {
    /// Label to message index
    pub labels: BTreeMap<String, u32>,
    /// Each message as raw UTF-16, NUL included
    pub texts: Vec<Vec<u16>>,
    /// Each message's ATR1 entry, empty when the file has none
    pub attributes: Vec<Vec<u8>>,
    pub attribute_size: u32,
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn sections(b: &[u8]) -> Option<BTreeMap<[u8; 4], &[u8]>> {
    if b.get(..8)? != b"MsgStdBn" || u16_at(b, 8)? != 0xFEFF {
        return None;
    }

    let count = u16_at(b, 0xE)?;
    let mut at = 0x20;
    let mut out = BTreeMap::new();

    for _ in 0..count {
        let magic: [u8; 4] = b.get(at..at + 4)?.try_into().ok()?;
        let size = u32_at(b, at + 4)? as usize;
        out.insert(magic, b.get(at + 16..at + 16 + size)?);
        at = (at + 16 + size + 15) & !15;
    }

    Some(out)
}

impl Msbt {
    pub fn parse(b: &[u8]) -> Option<Self> {
        let sections = sections(b)?;

        let lbl = sections.get(b"LBL1")?;
        let mut labels = BTreeMap::new();
        for bucket in 0..u32_at(lbl, 0)? as usize {
            let count = u32_at(lbl, 4 + bucket * 8)?;
            let mut at = u32_at(lbl, 8 + bucket * 8)? as usize;
            for _ in 0..count {
                let len = *lbl.get(at)? as usize;
                let name = std::str::from_utf8(lbl.get(at + 1..at + 1 + len)?).ok()?;
                labels.insert(name.to_owned(), u32_at(lbl, at + 1 + len)?);
                at += 1 + len + 4;
            }
        }

        let txt = sections.get(b"TXT2")?;
        let count = u32_at(txt, 0)? as usize;
        let mut offsets = (0..count)
            .map(|i| u32_at(txt, 4 + i * 4).map(|o| o as usize))
            .collect::<Option<Vec<_>>>()?;
        offsets.push(txt.len());
        let texts = offsets
            .windows(2)
            .map(|w| {
                let bytes = txt.get(w[0]..w[1])?;
                Some(
                    bytes
                        .chunks_exact(2)
                        .map(|c| u16::from_le_bytes([c[0], c[1]]))
                        .collect(),
                )
            })
            .collect::<Option<Vec<Vec<u16>>>>()?;

        let (attributes, attribute_size) = match sections.get(b"ATR1") {
            Some(atr) => {
                let size = u32_at(atr, 4)?;
                let entries = (0..u32_at(atr, 0)? as usize)
                    .map(|i| {
                        let at = 8 + i * size as usize;
                        atr.get(at..at + size as usize).map(<[u8]>::to_vec)
                    })
                    .collect::<Option<Vec<_>>>()?;
                (entries, size)
            }
            None => (Vec::new(), 0),
        };

        Some(Self {
            labels,
            texts,
            attributes,
            attribute_size,
        })
    }

    pub fn text(&self, label: &str) -> Option<&[u16]> {
        self.texts.get(*self.labels.get(label)? as usize).map(Vec::as_slice)
    }

    pub fn attribute(&self, label: &str) -> Option<&[u8]> {
        self.attributes
            .get(*self.labels.get(label)? as usize)
            .map(Vec::as_slice)
    }
}

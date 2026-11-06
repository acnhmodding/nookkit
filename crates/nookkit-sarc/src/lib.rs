//! The game's SARC archives: the files inside one, read in place

use std::fmt;

/// Why an archive could not be read
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub &'static str);

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Error {}

/// A file in an archive, as where its data sits in the archive
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    /// `None` for a member stored without a name
    pub name: Option<String>,
    pub offset: usize,
    pub length: usize,
}

impl Member {
    pub fn data<'a>(&self, archive: &'a [u8]) -> &'a [u8] {
        &archive[self.offset..self.offset + self.length]
    }

    pub fn is(&self, name: &str) -> bool {
        self.name.as_deref() == Some(name)
    }
}

const OUT_OF_BOUNDS: Error = Error("Resource data exceeds its bounds.");

fn range(data: &[u8], at: usize, count: usize) -> Result<&[u8]> {
    data.get(at..at.checked_add(count).ok_or(OUT_OF_BOUNDS)?)
        .ok_or(OUT_OF_BOUNDS)
}

fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(range(data, at, 2)?.try_into().unwrap()))
}

fn u32_at(data: &[u8], at: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(range(data, at, 4)?.try_into().unwrap()))
}

fn magic(data: &[u8], at: usize, magic: &[u8]) -> Result<bool> {
    Ok(range(data, at, magic.len())? == magic)
}

/// Every member of a little-endian `archive`, in the order its node table lists them
pub fn read(archive: &[u8]) -> Result<Vec<Member>> {
    let length = archive.len();
    if !magic(archive, 0, b"SARC")?
        || u16_at(archive, 4)? != 0x14
        || u16_at(archive, 6)? != 0xfeff
        || u32_at(archive, 8)? as usize != length
        || u16_at(archive, 16)? != 0x100
        || !magic(archive, 20, b"SFAT")?
        || u16_at(archive, 24)? != 12
    {
        return Err(Error("Expected a little-endian SARC archive."));
    }

    let count = u16_at(archive, 26)? as usize;
    let names = 32 + count * 16;
    let data = u32_at(archive, 12)? as usize;
    if names > length || data > length {
        return Err(Error("SARC nodes exceed the archive."));
    }
    let has_names = names + 8 <= length && magic(archive, names, b"SFNT")?;

    let mut members = Vec::with_capacity(count);
    for i in 0..count {
        let node = 32 + i * 16;
        let attributes = u32_at(archive, node + 4)?;
        let start = data + u32_at(archive, node + 8)? as usize;
        let end = data + u32_at(archive, node + 12)? as usize;
        if end < start || end > length {
            return Err(Error("SARC member exceeds the archive."));
        }
        let mut name = None;
        if attributes >> 24 != 0 {
            let at = names + 8 + (attributes & 0xffffff) as usize * 4;
            let terminator = match has_names && at < data {
                true => range(archive, at, data - at)?.iter().position(|&b| b == 0),
                false => None,
            };
            let Some(terminator) = terminator else {
                return Err(Error("SARC member name is invalid."));
            };
            // ASCII as .NET decodes it, other bytes become ?
            let bytes = range(archive, at, terminator)?;
            name = Some(
                bytes
                    .iter()
                    .map(|&b| if b < 0x80 { b as char } else { '?' })
                    .collect(),
            );
        }
        members.push(Member {
            name,
            offset: start,
            length: end - start,
        });
    }
    Ok(members)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut names: Vec<u8> = Vec::new();
        let mut data: Vec<u8> = Vec::new();
        let mut nodes = Vec::new();
        for (name, bytes) in members {
            nodes.extend(0u32.to_le_bytes()); // Name hash, unused by the reader
            nodes.extend((0x0100_0000 | (names.len() as u32 / 4)).to_le_bytes());
            nodes.extend((data.len() as u32).to_le_bytes());
            nodes.extend(((data.len() + bytes.len()) as u32).to_le_bytes());
            names.extend(name.as_bytes());
            names.resize((names.len() + 4) & !3, 0);
            data.extend(*bytes);
        }
        let data_offset = 0x14 + 12 + nodes.len() + 8 + names.len();
        let mut out = b"SARC".to_vec();
        out.extend(0x14u16.to_le_bytes());
        out.extend(0xfeffu16.to_le_bytes());
        out.extend(((data_offset + data.len()) as u32).to_le_bytes());
        out.extend((data_offset as u32).to_le_bytes());
        out.extend(0x100u32.to_le_bytes());
        out.extend(b"SFAT");
        out.extend(12u16.to_le_bytes());
        out.extend((members.len() as u16).to_le_bytes());
        out.extend(0x65u32.to_le_bytes());
        out.extend(nodes);
        out.extend(b"SFNT\x08\0\0\0");
        out.extend(names);
        out.extend(data);
        out
    }

    #[test]
    fn reads_members() {
        let sarc = archive(&[("output.bfres", b"model"), ("a/b.txt", b"")]);
        let members = read(&sarc).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members[0].is("output.bfres"));
        assert_eq!(members[0].data(&sarc), b"model");
        assert!(members[1].is("a/b.txt"));
        assert_eq!(members[1].data(&sarc), b"");
    }

    #[test]
    fn rejects_invalid_archives() {
        let sarc = archive(&[("output.bfres", b"model")]);
        assert!(read(&sarc[..sarc.len() - 1]).is_err());
        assert!(read(b"Yaz0").is_err());
    }
}

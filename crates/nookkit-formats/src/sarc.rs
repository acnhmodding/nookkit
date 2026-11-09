use crate::bytes::{OutOfBounds, Reader};

/// Why an archive could not be read
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    OutOfBounds(#[from] OutOfBounds),

    #[error("expected a little-endian SARC archive")]
    NotSarc,

    #[error("SARC nodes exceed the archive")]
    NodesOutOfBounds,

    #[error("SARC member exceeds the archive")]
    MemberOutOfBounds,

    #[error("SARC member name is invalid")]
    InvalidName,
}

pub type Result<T> = std::result::Result<T, Error>;

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

/// Every member of a little-endian `archive`, in the order its node table lists them
pub fn read(archive: &[u8]) -> Result<Vec<Member>> {
    let reader = Reader::new(archive);
    let length = archive.len();

    if !reader.magic_at(0, b"SARC")?
        || reader.u16_at(4)? != 0x14
        || reader.u16_at(6)? != 0xfeff
        || reader.u32_at(8)? as usize != length
        || reader.u16_at(16)? != 0x100
        || !reader.magic_at(20, b"SFAT")?
        || reader.u16_at(24)? != 12
    {
        return Err(Error::NotSarc);
    }

    let count = reader.u16_at(26)? as usize;
    let names = 32 + count * 16;
    let data = reader.u32_at(12)? as usize;

    if names > length || data > length {
        return Err(Error::NodesOutOfBounds);
    }

    let has_names = names + 8 <= length && reader.magic_at(names, b"SFNT")?;

    let mut members = Vec::with_capacity(count);

    for i in 0..count {
        let node = 32 + i * 16;
        let attributes = reader.u32_at(node + 4)?;
        let start = data + reader.u32_at(node + 8)? as usize;
        let end = data + reader.u32_at(node + 12)? as usize;

        if end < start || end > length {
            return Err(Error::MemberOutOfBounds);
        }

        let mut name = None;

        if attributes >> 24 != 0 {
            let at = names + 8 + (attributes & 0xffffff) as usize * 4;

            let terminator = if has_names && at < data {
                reader.bytes_at(at, data - at)?.iter().position(|&b| b == 0)
            } else {
                None
            };

            let Some(terminator) = terminator else {
                return Err(Error::InvalidName);
            };

            // ASCII as .NET decodes it, other bytes become ?
            let bytes = reader.bytes_at(at, terminator)?;

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
    use crate::bytes::Writer;

    fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
        let mut nodes = Writer::default();
        let mut names = Writer::default();
        let mut data = Writer::default();

        for (name, bytes) in members {
            // Name hash, unused by the reader
            nodes.write_u32(0);
            nodes.write_u32(0x0100_0000 | (names.pos() as u32 / 4));
            nodes.write_u32(data.pos() as u32);
            nodes.write_u32((data.pos() + bytes.len()) as u32);

            names.write_bytes(name.as_bytes());
            names.write_u8(0);
            names.align(4);
            data.write_bytes(bytes);
        }

        let (nodes, names, data) = (nodes.into_bytes(), names.into_bytes(), data.into_bytes());
        let data_offset = 0x14 + 12 + nodes.len() + 8 + names.len();

        let mut out = Writer::default();
        out.write_bytes(b"SARC");
        out.write_u16(0x14);
        out.write_u16(0xfeff);
        out.write_u32((data_offset + data.len()) as u32);
        out.write_u32(data_offset as u32);
        out.write_u32(0x100);
        out.write_bytes(b"SFAT");
        out.write_u16(12);
        out.write_u16(members.len() as u16);
        out.write_u32(0x65);
        out.write_bytes(&nodes);
        out.write_bytes(b"SFNT\x08\0\0\0");
        out.write_bytes(&names);
        out.write_bytes(&data);

        out.into_bytes()
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
        assert_eq!(read(&sarc[..sarc.len() - 1]), Err(Error::NotSarc));
        assert_eq!(read(b"Yaz0"), Err(Error::NotSarc));

        assert_eq!(
            read(b"SA"),
            Err(Error::OutOfBounds(OutOfBounds { offset: 0, len: 4 }))
        );
    }
}

use std::collections::{BTreeMap, BTreeSet};

use crate::error::{ParseError, WriteError};

/// How deep the reader follows containers, which keeps a file that loops from recursing forever
pub const MAX_DEPTH: usize = 256;

const STRING: u8 = 0xa0;
const BINARY: u8 = 0xa1;
const ARRAY: u8 = 0xc0;
const DICT: u8 = 0xc1;
const STRING_TABLE: u8 = 0xc2;
const BOOL: u8 = 0xd0;
const INT: u8 = 0xd1;
const FLOAT: u8 = 0xd2;
const UINT: u8 = 0xd3;
const INT64: u8 = 0xd4;
const UINT64: u8 = 0xd5;
const NULL: u8 = 0xff;

const MAX_COUNT: usize = 0xff_ffff;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    /// `YB`
    Little,

    /// `BY`
    Big,
}

/// The byte order and version (1 to 4) a file is written in
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Format {
    pub endian: Endian,
    pub version: u16,
}

impl Format {
    pub fn read(data: &[u8]) -> Result<Self, ParseError> {
        let (endian, version) = match data {
            [b'Y', b'B', a, b, ..] => (Endian::Little, u16::from_le_bytes([*a, *b])),
            [b'B', b'Y', a, b, ..] => (Endian::Big, u16::from_be_bytes([*a, *b])),
            _ => return Err(ParseError::Magic),
        };

        if !(1..=4).contains(&version) {
            return Err(ParseError::Version(version));
        }

        Ok(Self { endian, version })
    }

    fn magic(self) -> [u8; 2] {
        match self.endian {
            Endian::Little => *b"YB",
            Endian::Big => *b"BY",
        }
    }
}

/// A value in a document. Dictionaries keep their keys sorted, as the game looks them up by
/// binary search
#[derive(Debug, Clone)]
pub enum Node {
    String(String),
    Binary(Vec<u8>),
    Array(Vec<Node>),
    Dict(BTreeMap<String, Node>),
    Bool(bool),
    Int(i32),
    Float(f32),
    UInt(u32),
    Int64(i64),
    UInt64(u64),
    Null,
}

impl Node {
    fn node_type(&self) -> u8 {
        match self {
            Node::String(_) => STRING,
            Node::Binary(_) => BINARY,
            Node::Array(_) => ARRAY,
            Node::Dict(_) => DICT,
            Node::Bool(_) => BOOL,
            Node::Int(_) => INT,
            Node::Float(_) => FLOAT,
            Node::UInt(_) => UINT,
            Node::Int64(_) => INT64,
            Node::UInt64(_) => UINT64,
            Node::Null => NULL,
        }
    }
}

impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Node::String(a), Node::String(b)) => a == b,
            (Node::Binary(a), Node::Binary(b)) => a == b,
            (Node::Array(a), Node::Array(b)) => a == b,
            (Node::Dict(a), Node::Dict(b)) => a == b,
            (Node::Bool(a), Node::Bool(b)) => a == b,
            (Node::Int(a), Node::Int(b)) => a == b,
            (Node::Float(a), Node::Float(b)) => a.to_bits() == b.to_bits(),
            (Node::UInt(a), Node::UInt(b)) => a == b,
            (Node::Int64(a), Node::Int64(b)) => a == b,
            (Node::UInt64(a), Node::UInt64(b)) => a == b,
            (Node::Null, Node::Null) => true,
            _ => false,
        }
    }
}

/// A whole file. A null root is a file with no nodes
#[derive(Debug, Clone, PartialEq)]
pub struct Byml {
    pub format: Format,
    pub root: Node,
}

// An offset of 0 means the table or root is missing
#[derive(Debug, Clone, Copy)]
struct Header {
    format: Format,
    keys: u32,
    strings: u32,
    root: u32,
}

impl Header {
    const SIZE: usize = 0x10;

    fn read(data: &[u8]) -> Result<Self, ParseError> {
        let format = Format::read(data)?;

        let bytes = Bytes {
            data,
            endian: format.endian,
        };

        Ok(Self {
            format,
            keys: bytes.u32(4)?,
            strings: bytes.u32(8)?,
            root: bytes.u32(0xc)?,
        })
    }

    fn write(self, out: &mut Out) {
        out.buf[..2].copy_from_slice(&self.format.magic());
        out.set_u16(2, self.format.version);
        out.set_u32(4, self.keys);
        out.set_u32(8, self.strings);
        out.set_u32(0xc, self.root);
    }
}

#[derive(Debug, Clone, Copy)]
struct ContainerHeader {
    node_type: u8,
    count: usize,
}

impl ContainerHeader {
    fn read(bytes: Bytes, offset: usize) -> Result<Self, ParseError> {
        Ok(Self {
            node_type: bytes.u8(offset)?,
            count: bytes.u24(offset + 1)? as usize,
        })
    }

    fn write(self, out: &mut Out) -> Result<(), WriteError> {
        out.u8(self.node_type);
        out.u24(count(self.count)?);

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct DictEntry {
    key: u32,
    node_type: u8,
    value: u32,
}

impl DictEntry {
    const SIZE: usize = 8;

    fn read(bytes: Bytes, offset: usize) -> Result<Self, ParseError> {
        Ok(Self {
            key: bytes.u24(offset)?,
            node_type: bytes.u8(offset + 3)?,
            value: bytes.u32(offset + 4)?,
        })
    }
}

fn array_values(offset: usize, count: usize) -> usize {
    offset + 4 + count.next_multiple_of(4)
}

#[derive(Clone, Copy)]
struct Bytes<'a> {
    data: &'a [u8],
    endian: Endian,
}

impl<'a> Bytes<'a> {
    fn slice(self, offset: usize, len: usize) -> Result<&'a [u8], ParseError> {
        offset
            .checked_add(len)
            .and_then(|end| self.data.get(offset..end))
            .ok_or(ParseError::OutOfBounds { offset })
    }

    fn array<const N: usize>(self, offset: usize) -> Result<[u8; N], ParseError> {
        let slice = self.slice(offset, N)?;
        let mut array = [0; N];
        array.copy_from_slice(slice);

        if self.endian == Endian::Big {
            array.reverse();
        }

        Ok(array)
    }

    fn u8(self, offset: usize) -> Result<u8, ParseError> {
        Ok(self.array::<1>(offset)?[0])
    }

    fn u24(self, offset: usize) -> Result<u32, ParseError> {
        let [a, b, c] = self.array(offset)?;

        Ok(u32::from_le_bytes([a, b, c, 0]))
    }

    fn u32(self, offset: usize) -> Result<u32, ParseError> {
        self.array(offset).map(u32::from_le_bytes)
    }

    fn u64(self, offset: usize) -> Result<u64, ParseError> {
        self.array(offset).map(u64::from_le_bytes)
    }
}

// A string table: a count, the offset of each string and of the end, then the strings with a
// NUL after each
fn read_table(bytes: Bytes, offset: u32) -> Result<Vec<String>, ParseError> {
    if offset == 0 {
        return Ok(Vec::new());
    }

    let offset = offset as usize;
    let header = ContainerHeader::read(bytes, offset)?;

    if header.node_type != STRING_TABLE {
        return Err(ParseError::NotStringTable { offset });
    }

    bytes.slice(offset + 4, (header.count + 1) * 4)?;

    let mut strings = Vec::with_capacity(header.count);

    for index in 0..header.count {
        let start = offset + bytes.u32(offset + 4 + index * 4)? as usize;
        let end = offset + bytes.u32(offset + 8 + index * 4)? as usize;
        let text = bytes.slice(start, end.saturating_sub(start))?;

        let Some(nul) = text.iter().position(|&b| b == 0) else {
            return Err(ParseError::NotStringTable { offset });
        };

        let text =
            std::str::from_utf8(&text[..nul]).map_err(|_| ParseError::Utf8 { offset: start })?;

        strings.push(text.to_owned());
    }

    Ok(strings)
}

struct Reader<'a> {
    bytes: Bytes<'a>,
    keys: Vec<String>,
    strings: Vec<String>,
}

impl Reader<'_> {
    fn node(
        &self,
        node_type: u8,
        value: u32,
        parent: usize,
        depth: usize,
    ) -> Result<Node, ParseError> {
        let offset = value as usize;

        Ok(match node_type {
            STRING => Node::String(
                self.strings
                    .get(offset)
                    .ok_or(ParseError::StringIndex { index: value })?
                    .clone(),
            ),
            BINARY => {
                let len = self.bytes.u32(offset)? as usize;
                Node::Binary(self.bytes.slice(offset + 4, len)?.to_vec())
            }
            ARRAY | DICT => self.container(node_type, offset, depth)?,
            BOOL => Node::Bool(value != 0),
            INT => Node::Int(value as i32),
            FLOAT => Node::Float(f32::from_bits(value)),
            UINT => Node::UInt(value),
            INT64 => Node::Int64(self.bytes.u64(offset)? as i64),
            UINT64 => Node::UInt64(self.bytes.u64(offset)?),
            NULL => Node::Null,
            _ => {
                return Err(ParseError::Type {
                    offset: parent,
                    node_type,
                });
            }
        })
    }

    fn container(&self, node_type: u8, offset: usize, depth: usize) -> Result<Node, ParseError> {
        if depth >= MAX_DEPTH {
            return Err(ParseError::TooDeep);
        }

        let header = ContainerHeader::read(self.bytes, offset)?;

        if header.node_type != node_type {
            return Err(ParseError::Header {
                offset,
                expected: node_type,
                found: header.node_type,
            });
        }

        let count = header.count;

        if node_type == ARRAY {
            let types = self.bytes.slice(offset + 4, count)?;
            let values = array_values(offset, count);
            let mut items = Vec::with_capacity(count);

            for (index, &item) in types.iter().enumerate() {
                let value = self.bytes.u32(values + index * 4)?;
                items.push(self.node(item, value, offset, depth + 1)?);
            }

            return Ok(Node::Array(items));
        }

        self.bytes.slice(offset + 4, count * DictEntry::SIZE)?;
        let mut dict = BTreeMap::new();

        for index in 0..count {
            let entry = DictEntry::read(self.bytes, offset + 4 + index * DictEntry::SIZE)?;

            let key = self
                .keys
                .get(entry.key as usize)
                .ok_or(ParseError::KeyIndex {
                    offset,
                    index: entry.key,
                })?;

            let node = self.node(entry.node_type, entry.value, offset, depth + 1)?;
            dict.insert(key.clone(), node);
        }

        Ok(Node::Dict(dict))
    }
}

impl Byml {
    pub fn parse(data: &[u8]) -> Result<Self, ParseError> {
        let header = Header::read(data)?;
        let format = header.format;

        if header.root == 0 {
            return Ok(Self {
                format,
                root: Node::Null,
            });
        }

        let bytes = Bytes {
            data,
            endian: format.endian,
        };

        let reader = Reader {
            bytes,
            keys: read_table(bytes, header.keys)?,
            strings: read_table(bytes, header.strings)?,
        };

        let root = header.root as usize;
        let node_type = bytes.u8(root)?;

        if !matches!(node_type, ARRAY | DICT) {
            return Err(ParseError::Root);
        }

        Ok(Self {
            format,
            root: reader.container(node_type, root, 0)?,
        })
    }

    /// Writes the header, the key table, the string table, then each container before what it
    /// holds. Both tables are sorted, which the game checks
    pub fn to_bytes(&self) -> Result<Vec<u8>, WriteError> {
        if !matches!(self.root, Node::Array(_) | Node::Dict(_) | Node::Null) {
            return Err(WriteError::Root);
        }

        let (mut keys, mut strings) = (BTreeSet::new(), BTreeSet::new());
        collect(&self.root, &mut keys, &mut strings)?;
        let keys: Vec<&str> = keys.into_iter().collect();
        let strings: Vec<&str> = strings.into_iter().collect();

        let mut out = Out {
            buf: vec![0; Header::SIZE],
            endian: self.format.endian,
        };

        let mut header = Header {
            format: self.format,
            keys: write_table(&mut out, &keys)?,
            strings: write_table(&mut out, &strings)?,
            root: 0,
        };

        if !matches!(self.root, Node::Null) {
            let mut writer = Writer { out, keys, strings };
            header.root = writer.value(&self.root)?;
            out = writer.out;
        }

        header.write(&mut out);

        Ok(out.buf)
    }
}

fn collect<'n>(
    node: &'n Node,
    keys: &mut BTreeSet<&'n str>,
    strings: &mut BTreeSet<&'n str>,
) -> Result<(), WriteError> {
    let check = |text: &str| {
        if text.contains('\0') {
            return Err(WriteError::Nul {
                text: text.to_owned(),
            });
        }

        Ok(())
    };

    match node {
        Node::String(text) => {
            check(text)?;
            strings.insert(text);
        }
        Node::Array(items) => {
            for item in items {
                collect(item, keys, strings)?;
            }
        }
        Node::Dict(dict) => {
            for (key, item) in dict {
                check(key)?;
                keys.insert(key);
                collect(item, keys, strings)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn count(count: usize) -> Result<u32, WriteError> {
    if count > MAX_COUNT {
        return Err(WriteError::TooMany { count });
    }

    Ok(count as u32)
}

// `table` is sorted and holds `text`
fn index(table: &[&str], text: &str) -> u32 {
    table.partition_point(|t| *t < text) as u32
}

fn write_table(out: &mut Out, table: &[&str]) -> Result<u32, WriteError> {
    if table.is_empty() {
        return Ok(0);
    }

    let start = out.pos()?;

    ContainerHeader {
        node_type: STRING_TABLE,
        count: table.len(),
    }
    .write(out)?;

    let mut at = 4 + (table.len() + 1) * 4;

    for text in table {
        out.u32(u32::try_from(at).map_err(|_| WriteError::TooLarge)?);
        at += text.len() + 1;
    }

    out.u32(u32::try_from(at).map_err(|_| WriteError::TooLarge)?);

    for text in table {
        out.buf.extend_from_slice(text.as_bytes());
        out.u8(0);
    }

    out.align();

    Ok(start)
}

struct Out {
    buf: Vec<u8>,
    endian: Endian,
}

impl Out {
    fn pos(&self) -> Result<u32, WriteError> {
        u32::try_from(self.buf.len()).map_err(|_| WriteError::TooLarge)
    }

    fn bytes<const N: usize>(&self, mut le: [u8; N]) -> [u8; N] {
        if self.endian == Endian::Big {
            le.reverse();
        }

        le
    }

    fn u8(&mut self, value: u8) {
        self.buf.push(value);
    }

    fn u24(&mut self, value: u32) {
        let [a, b, c, _] = value.to_le_bytes();
        let bytes = self.bytes([a, b, c]);
        self.buf.extend_from_slice(&bytes);
    }

    fn u32(&mut self, value: u32) {
        let bytes = self.bytes(value.to_le_bytes());
        self.buf.extend_from_slice(&bytes);
    }

    fn u64(&mut self, value: u64) {
        let bytes = self.bytes(value.to_le_bytes());
        self.buf.extend_from_slice(&bytes);
    }

    fn set_u16(&mut self, at: usize, value: u16) {
        let bytes = self.bytes(value.to_le_bytes());
        self.buf[at..at + 2].copy_from_slice(&bytes);
    }

    fn set_u32(&mut self, at: usize, value: u32) {
        let bytes = self.bytes(value.to_le_bytes());
        self.buf[at..at + 4].copy_from_slice(&bytes);
    }

    fn align(&mut self) {
        let len = self.buf.len().next_multiple_of(4);
        self.buf.resize(len, 0);
    }
}

struct Writer<'n> {
    out: Out,
    keys: Vec<&'n str>,
    strings: Vec<&'n str>,
}

impl<'n> Writer<'n> {
    fn value(&mut self, node: &'n Node) -> Result<u32, WriteError> {
        Ok(match node {
            Node::String(text) => index(&self.strings, text),
            Node::Binary(data) => {
                let at = self.out.pos()?;

                self.out
                    .u32(u32::try_from(data.len()).map_err(|_| WriteError::TooLarge)?);

                self.out.buf.extend_from_slice(data);
                self.out.align();
                at
            }
            Node::Array(items) => {
                let at = self.out.pos()?;

                ContainerHeader {
                    node_type: ARRAY,
                    count: items.len(),
                }
                .write(&mut self.out)?;

                for item in items {
                    self.out.u8(item.node_type());
                }

                self.out.align();
                let values = self.out.buf.len();
                self.out.buf.resize(values + items.len() * 4, 0);

                for (index, item) in items.iter().enumerate() {
                    let value = self.value(item)?;
                    self.out.set_u32(values + index * 4, value);
                }

                at
            }
            Node::Dict(dict) => {
                let at = self.out.pos()?;

                ContainerHeader {
                    node_type: DICT,
                    count: dict.len(),
                }
                .write(&mut self.out)?;

                let entries = self.out.buf.len();

                for (key, item) in dict {
                    self.out.u24(index(&self.keys, key));
                    self.out.u8(item.node_type());
                    self.out.u32(0);
                }

                for (index, item) in dict.values().enumerate() {
                    let value = self.value(item)?;

                    self.out
                        .set_u32(entries + index * DictEntry::SIZE + 4, value);
                }

                at
            }
            Node::Bool(value) => *value as u32,
            Node::Int(value) => *value as u32,
            Node::Float(value) => value.to_bits(),
            Node::UInt(value) => *value,
            Node::Int64(value) => self.u64(*value as u64)?,
            Node::UInt64(value) => self.u64(*value)?,
            Node::Null => 0,
        })
    }

    fn u64(&mut self, value: u64) -> Result<u32, WriteError> {
        let at = self.out.pos()?;
        self.out.u64(value);

        Ok(at)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(endian: Endian) -> Byml {
        let mut inner = BTreeMap::new();
        inner.insert("b".to_owned(), Node::Int64(-5_000_000_000));
        inner.insert("a".to_owned(), Node::Binary(vec![1, 2, 3]));
        let mut root = BTreeMap::new();

        root.insert(
            "list".to_owned(),
            Node::Array(vec![
                Node::String("x".into()),
                Node::Float(1.5),
                Node::Bool(true),
                Node::UInt(7),
                Node::UInt64(u64::MAX),
                Node::Null,
                Node::Dict(inner),
            ]),
        );

        root.insert("name".to_owned(), Node::String("abc".into()));
        root.insert("n".to_owned(), Node::Int(-3));

        Byml {
            format: Format { endian, version: 4 },
            root: Node::Dict(root),
        }
    }

    #[test]
    fn round_trips_in_both_orders() {
        for endian in [Endian::Little, Endian::Big] {
            let byml = document(endian);
            let data = byml.to_bytes().unwrap();
            assert_eq!(Byml::parse(&data).unwrap(), byml);
        }
    }

    #[test]
    fn writes_sorted_tables() {
        let data = document(Endian::Little).to_bytes().unwrap();
        assert_eq!(&data[..4], b"YB\x04\x00");
        assert_eq!(u32::from_le_bytes(data[4..8].try_into().unwrap()), 0x10);

        let bytes = Bytes {
            data: &data,
            endian: Endian::Little,
        };

        let keys = read_table(bytes, 0x10).unwrap();
        assert_eq!(keys, ["a", "b", "list", "n", "name"]);
        let strings = read_table(bytes, bytes.u32(8).unwrap()).unwrap();
        assert_eq!(strings, ["abc", "x"]);
    }

    #[test]
    fn writes_an_empty_file_for_null() {
        let byml = Byml {
            format: Format {
                endian: Endian::Little,
                version: 2,
            },
            root: Node::Null,
        };

        let data = byml.to_bytes().unwrap();
        assert_eq!(data, b"YB\x02\x00\0\0\0\0\0\0\0\0\0\0\0\0");
        assert_eq!(Byml::parse(&data).unwrap(), byml);
    }

    #[test]
    fn refuses_what_the_game_wont_read() {
        assert_eq!(Byml::parse(b"XX\x04\x00"), Err(ParseError::Magic));

        assert_eq!(
            Byml::parse(b"BY\x00\x05\0\0\0\0\0\0\0\0\0\0\0\0"),
            Err(ParseError::Version(5))
        );

        let byml = Byml {
            root: Node::Int(1),
            ..document(Endian::Little)
        };

        assert_eq!(byml.to_bytes(), Err(WriteError::Root));

        let byml = Byml {
            root: Node::Array(vec![Node::String("a\0b".into())]),
            ..document(Endian::Little)
        };

        assert_eq!(
            byml.to_bytes(),
            Err(WriteError::Nul {
                text: "a\0b".into()
            })
        );
    }

    #[test]
    fn stops_at_a_loop() {
        let mut data = b"YB\x04\x00\0\0\0\0\0\0\0\0\x10\0\0\0".to_vec();
        data.extend_from_slice(&[ARRAY, 1, 0, 0, ARRAY, 0, 0, 0, 0x10, 0, 0, 0]);
        assert_eq!(Byml::parse(&data), Err(ParseError::TooDeep));
    }
}

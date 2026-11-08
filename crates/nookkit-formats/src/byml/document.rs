use std::collections::{BTreeMap, BTreeSet};

use crate::byml::error::{ParseError, WriteError};
use crate::bytes::{self, Slot};

pub use crate::bytes::Endian;

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
    fn read(data: &[u8]) -> Result<Self, ParseError> {
        let format = Format::read(data)?;

        let bytes = bytes::Reader::with_endian(data, format.endian);

        Ok(Self {
            format,
            keys: bytes.u32_at(4)?,
            strings: bytes.u32_at(8)?,
            root: bytes.u32_at(0xc)?,
        })
    }

    // The key table, string table and root offsets, filled in once they are written
    fn reserve(format: Format, out: &mut bytes::Writer) -> [Slot<4>; 3] {
        out.write_bytes(&format.magic());
        out.write_u16(format.version);

        [out.reserve(), out.reserve(), out.reserve()]
    }
}

#[derive(Debug, Clone, Copy)]
struct ContainerHeader {
    node_type: u8,
    count: usize,
}

impl ContainerHeader {
    fn read(bytes: bytes::Reader, offset: usize) -> Result<Self, ParseError> {
        Ok(Self {
            node_type: bytes.u8_at(offset)?,
            count: bytes.u24_at(offset + 1)? as usize,
        })
    }

    fn write(self, out: &mut bytes::Writer) -> Result<(), WriteError> {
        out.write_u8(self.node_type);
        out.write_u24(count(self.count)?);

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

    fn read(bytes: bytes::Reader, offset: usize) -> Result<Self, ParseError> {
        Ok(Self {
            key: bytes.u24_at(offset)?,
            node_type: bytes.u8_at(offset + 3)?,
            value: bytes.u32_at(offset + 4)?,
        })
    }
}

fn array_values(offset: usize, count: usize) -> usize {
    offset + 4 + count.next_multiple_of(4)
}

// A string table: a count, the offset of each string and of the end, then the strings with a
// NUL after each
fn read_table(bytes: bytes::Reader, offset: u32) -> Result<Vec<String>, ParseError> {
    if offset == 0 {
        return Ok(Vec::new());
    }

    let offset = offset as usize;
    let header = ContainerHeader::read(bytes, offset)?;

    if header.node_type != STRING_TABLE {
        return Err(ParseError::NotStringTable { offset });
    }

    bytes.bytes_at(offset + 4, (header.count + 1) * 4)?;

    let mut strings = Vec::with_capacity(header.count);

    for index in 0..header.count {
        let start = offset + bytes.u32_at(offset + 4 + index * 4)? as usize;
        let end = offset + bytes.u32_at(offset + 8 + index * 4)? as usize;
        let text = bytes.bytes_at(start, end.saturating_sub(start))?;

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
    bytes: bytes::Reader<'a>,
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
                let len = self.bytes.u32_at(offset)? as usize;
                Node::Binary(self.bytes.bytes_at(offset + 4, len)?.to_vec())
            }
            ARRAY | DICT => self.container(node_type, offset, depth)?,
            BOOL => Node::Bool(value != 0),
            INT => Node::Int(value as i32),
            FLOAT => Node::Float(f32::from_bits(value)),
            UINT => Node::UInt(value),
            INT64 => Node::Int64(self.bytes.u64_at(offset)? as i64),
            UINT64 => Node::UInt64(self.bytes.u64_at(offset)?),
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
            let types = self.bytes.bytes_at(offset + 4, count)?;
            let values = array_values(offset, count);
            let mut items = Vec::with_capacity(count);

            for (index, &item) in types.iter().enumerate() {
                let value = self.bytes.u32_at(values + index * 4)?;
                items.push(self.node(item, value, offset, depth + 1)?);
            }

            return Ok(Node::Array(items));
        }

        self.bytes.bytes_at(offset + 4, count * DictEntry::SIZE)?;
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

        let bytes = bytes::Reader::with_endian(data, format.endian);

        let reader = Reader {
            bytes,
            keys: read_table(bytes, header.keys)?,
            strings: read_table(bytes, header.strings)?,
        };

        let root = header.root as usize;
        let node_type = bytes.u8_at(root)?;

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

        let mut out = bytes::Writer::new(self.format.endian);
        let [keys_slot, strings_slot, root_slot] = Header::reserve(self.format, &mut out);

        let keys_at = write_table(&mut out, &keys)?;
        out.fill_u32(keys_slot, keys_at);

        let strings_at = write_table(&mut out, &strings)?;
        out.fill_u32(strings_slot, strings_at);

        if !matches!(self.root, Node::Null) {
            let mut writer = Writer { out, keys, strings };
            let root = writer.value(&self.root)?;

            out = writer.out;
            out.fill_u32(root_slot, root);
        }

        Ok(out.into_bytes())
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

fn offset(out: &bytes::Writer) -> Result<u32, WriteError> {
    u32::try_from(out.pos()).map_err(|_| WriteError::TooLarge)
}

fn write_table(out: &mut bytes::Writer, table: &[&str]) -> Result<u32, WriteError> {
    if table.is_empty() {
        return Ok(0);
    }

    let start = offset(out)?;

    ContainerHeader {
        node_type: STRING_TABLE,
        count: table.len(),
    }
    .write(out)?;

    let mut at = 4 + (table.len() + 1) * 4;

    for text in table {
        out.write_u32(u32::try_from(at).map_err(|_| WriteError::TooLarge)?);
        at += text.len() + 1;
    }

    out.write_u32(u32::try_from(at).map_err(|_| WriteError::TooLarge)?);

    for text in table {
        out.write_bytes(text.as_bytes());
        out.write_u8(0);
    }

    out.align(4);

    Ok(start)
}

struct Writer<'n> {
    out: bytes::Writer,
    keys: Vec<&'n str>,
    strings: Vec<&'n str>,
}

impl<'n> Writer<'n> {
    fn value(&mut self, node: &'n Node) -> Result<u32, WriteError> {
        Ok(match node {
            Node::String(text) => index(&self.strings, text),
            Node::Binary(data) => {
                let at = offset(&self.out)?;

                self.out
                    .write_u32(u32::try_from(data.len()).map_err(|_| WriteError::TooLarge)?);

                self.out.write_bytes(data);
                self.out.align(4);
                at
            }
            Node::Array(items) => {
                let at = offset(&self.out)?;

                ContainerHeader {
                    node_type: ARRAY,
                    count: items.len(),
                }
                .write(&mut self.out)?;

                for item in items {
                    self.out.write_u8(item.node_type());
                }

                self.out.align(4);

                let slots: Vec<Slot<4>> = items.iter().map(|_| self.out.reserve()).collect();

                for (slot, item) in slots.into_iter().zip(items) {
                    let value = self.value(item)?;
                    self.out.fill_u32(slot, value);
                }

                at
            }
            Node::Dict(dict) => {
                let at = offset(&self.out)?;

                ContainerHeader {
                    node_type: DICT,
                    count: dict.len(),
                }
                .write(&mut self.out)?;

                let mut slots = Vec::with_capacity(dict.len());

                for (key, item) in dict {
                    self.out.write_u24(index(&self.keys, key));
                    self.out.write_u8(item.node_type());
                    slots.push(self.out.reserve());
                }

                for (slot, item) in slots.into_iter().zip(dict.values()) {
                    let value = self.value(item)?;
                    self.out.fill_u32(slot, value);
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
        let at = offset(&self.out)?;
        self.out.write_u64(value);

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

        let bytes = bytes::Reader::new(&data);

        let keys = read_table(bytes, 0x10).unwrap();
        assert_eq!(keys, ["a", "b", "list", "n", "name"]);
        let strings = read_table(bytes, bytes.u32_at(8).unwrap()).unwrap();
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

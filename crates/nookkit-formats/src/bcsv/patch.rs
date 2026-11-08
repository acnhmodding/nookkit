use std::collections::BTreeMap;

use crate::hash::crc32;

use crate::bcsv::error::{CellError, Invalid, ReadError, Skipped, WriteError};
use crate::bcsv::table::{Column, Table};

// The columns that identify a row, the first one a table has. A patched row replaces the row with
// the same value there, and is added after the others otherwise
const KEY_COLUMNS: &[&str] = &[
    "UniqueID u16",
    "Label string8",
    "ItemUniqueID u16",
    "StateName string48",
    "NumberingId u16",
    "ResourceName string33",
    "StageName string32",
    "ItemFrom string32",
    "TVProgramName.hshCstringRef",
    "GroundAttributeUniqueID u16",
    "NpcRoleSetID u16",
];

// The key of the MessageCardSelect tables, whose name isn't known
const UNNAMED_KEY_COLUMN: u32 = 0x3757_1146;

// A cell's kind, in the low bits of its tag
pub(crate) const UNSIGNED: u32 = 0;
pub(crate) const SIGNED: u32 = 1;
pub(crate) const FLOAT: u32 = 2;
pub(crate) const HASH: u32 = 3;
pub(crate) const TEXT: u32 = 4;
pub(crate) const BYTES: u32 = 5;
const KIND_BITS: u32 = 3;

/// A cell in eight bytes, since a patch set has tens of thousands and the game little memory. The
/// tag holds the column key's id and the kind, the value the number or where the text or bytes
/// are in the source's `bytes`
#[derive(Debug, Clone, Copy)]
pub(crate) struct Cell {
    tag: u32,
    value: u32,
}

impl Cell {
    pub(crate) fn new(key: u32, kind: u32, value: u32) -> Self {
        Self {
            tag: key << KIND_BITS | kind,
            value,
        }
    }

    fn key(self) -> usize {
        (self.tag >> KIND_BITS) as usize
    }

    fn kind(self) -> u32 {
        self.tag & ((1 << KIND_BITS) - 1)
    }

    fn integer(self) -> i64 {
        match self.kind() {
            SIGNED => self.value as i32 as i64,
            _ => self.value as i64,
        }
    }
}

/// The rows one file has for one table
#[derive(Debug, Default)]
pub(crate) struct Source {
    pub(crate) name: String,
    pub(crate) cells: Vec<Cell>,

    /// Where each row's cells end
    pub(crate) ends: Vec<u32>,

    /// Text and bytes, each after its length as a u32
    pub(crate) bytes: Vec<u8>,
}

impl Source {
    fn rows(&self) -> impl Iterator<Item = &[Cell]> {
        let mut start = 0;

        self.ends.iter().map(move |&end| {
            let row = &self.cells[start..end as usize];
            start = end as usize;
            row
        })
    }

    fn blob_at(&self, at: u32) -> &[u8] {
        let Some(blob) = self.bytes.get(at as usize..) else {
            return &[];
        };

        let Some((len, rest)) = blob.split_first_chunk::<4>() else {
            return &[];
        };

        rest.get(..u32::from_le_bytes(*len) as usize)
            .unwrap_or_default()
    }

    /// Starts a blob, whose length [`Self::end_blob`] writes
    pub(crate) fn start_blob(&mut self) -> u32 {
        let at = self.bytes.len();
        self.bytes.extend_from_slice(&[0; 4]);

        at as u32
    }

    pub(crate) fn end_blob(&mut self, at: u32) {
        let at = at as usize;
        let len = (self.bytes.len() - at - 4) as u32;
        self.bytes[at..at + 4].copy_from_slice(&len.to_le_bytes());
    }

    fn fits(&self, cell: Cell, key: &str, size: usize) -> Result<(), CellError> {
        let key = || key.to_owned();

        match cell.kind() {
            UNSIGNED | SIGNED => {
                // A u8 that ends a row is padded, so only its first four bytes are the number
                let value = cell.integer();
                let bits = size as u32 * 8;

                let fits = (1..=4).contains(&size)
                    && (-(1i64 << (bits - 1))..(1i64 << bits)).contains(&value);

                if !fits {
                    return Err(CellError::Integer {
                        key: key(),
                        value,
                        size,
                    });
                }
            }
            FLOAT | HASH if size != 4 => return Err(CellError::NotFourBytes { key: key(), size }),
            TEXT => {
                let len = self.blob_at(cell.value).len();

                if len >= size {
                    return Err(CellError::TextTooLong {
                        key: key(),
                        len,
                        size,
                    });
                }
            }
            BYTES => {
                let len = self.blob_at(cell.value).len();

                if len != size {
                    return Err(CellError::ByteCount {
                        key: key(),
                        len,
                        size,
                    });
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn write(&self, cell: Cell, out: &mut [u8]) {
        out.fill(0);

        match cell.kind() {
            UNSIGNED | SIGNED => {
                let len = out.len();
                out.copy_from_slice(&cell.integer().to_le_bytes()[..len]);
            }
            FLOAT | HASH => out.copy_from_slice(&cell.value.to_le_bytes()),
            _ => {
                let bytes = self.blob_at(cell.value);
                out[..bytes.len()].copy_from_slice(bytes);
            }
        }
    }
}

#[derive(Debug)]
struct Key {
    name: Box<str>,
    hash: u32,

    /// A plain string is text for a `… stringN` column, and hex bytes otherwise
    text: bool,
}

/// Every column key the patches name, once, by the id their cells keep
#[derive(Debug, Default)]
pub(crate) struct Keys {
    ids: BTreeMap<Box<str>, u32>,
    keys: Vec<Key>,
}

impl Keys {
    pub(crate) fn id(&mut self, key: &str) -> Result<u32, Invalid> {
        if let Some(&id) = self.ids.get(key) {
            return Ok(id);
        }

        let hash = match key.strip_prefix("0x") {
            Some(hex) if hex.len() == 8 => {
                u32::from_str_radix(hex, 16).map_err(|_| Invalid::ColumnHash { key: key.into() })?
            }
            _ => crc32(key),
        };

        let id = self.keys.len() as u32;

        if id >= 1 << (32 - KIND_BITS) {
            return Err(Invalid::TooManyKeys);
        }

        self.keys.push(Key {
            name: key.into(),
            hash,
            text: key.contains(" string"),
        });

        self.ids.insert(key.into(), id);

        Ok(id)
    }

    pub(crate) fn name(&self, id: u32) -> &str {
        &self.keys[id as usize].name
    }

    pub(crate) fn is_text(&self, id: u32) -> bool {
        self.keys[id as usize].text
    }
}

/// Every loaded patch, by table. Patches apply in the order they were added, so a row patched
/// twice keeps the later one
///
/// A file holds one patch or an array of them, each `{ "file": "ItemParam", "rows": [...] }`. A
/// row maps column names, or their hash as `0x%08X`, to a number, a string (text for a
/// `… stringN` column and hex bytes otherwise) or a string's hash as `{ "crc32": … }` or
/// `{ "mmh3": … }`
#[derive(Debug, Default)]
pub struct PatchSet {
    files: BTreeMap<String, Vec<Source>>,
    keys: Keys,
}

impl PatchSet {
    /// Adds the patches in `json`, one patch or an array of them, from the file `patch`. Nothing
    /// is added unless every patch reads
    pub fn add(&mut self, patch: &str, json: &str) -> Result<(), ReadError> {
        let read =
            crate::bcsv::read::patches(&mut self.keys, patch, json).map_err(|error| ReadError {
                patch: patch.to_owned(),
                error,
            })?;

        for (file, source) in read {
            self.files.entry(file).or_default().push(source);
        }

        Ok(())
    }

    /// The patched table named `file`, borrowed from the set
    pub fn file(&self, file: &str) -> Option<&str> {
        self.files
            .get_key_value(file)
            .map(|(name, _)| name.as_str())
    }

    pub fn files(&self) -> impl Iterator<Item = &str> {
        self.files.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Finds the row each patch for `file` goes to in `table`. A patch with a cell that doesn't
    /// fit the table is skipped whole. Only this bookkeeping is allocated, [`Plan::write`] writes
    /// the rows straight to a buffer
    pub fn plan<'p>(&'p self, file: &str, table: &'p Table<'p>) -> Plan<'p> {
        let columns: Vec<_> = self
            .keys
            .keys
            .iter()
            .map(|k| table.column(k.hash))
            .collect();

        let mut skipped = Vec::new();

        let sources: Vec<&Source> = self
            .files
            .get(file)
            .into_iter()
            .flatten()
            .filter(|source| match self.check(source, &columns) {
                Ok(()) => true,
                Err(error) => {
                    skipped.push(Skipped {
                        patch: source.name.clone(),
                        table: file.to_owned(),
                        error,
                    });

                    false
                }
            })
            .collect();

        let mut plan = Plan {
            added: 0,
            replaced: 0,
            skipped,
            table,
            sources,
            columns,
            targets: Vec::new(),
        };

        plan.find_targets();

        plan
    }

    fn check(&self, source: &Source, columns: &[Option<Column>]) -> Result<(), CellError> {
        for &cell in source.rows().flatten() {
            let key = self.keys.name(cell.key() as u32);

            let column = columns[cell.key()].ok_or_else(|| CellError::MissingColumn {
                key: key.to_owned(),
            })?;

            source.fits(cell, key, column.size)?;
        }

        Ok(())
    }
}

/// Where every patch for one table goes, from [`PatchSet::plan`]
#[derive(Debug)]
pub struct Plan<'p> {
    pub added: usize,
    pub replaced: usize,

    /// Patches that didn't fit the table and change nothing
    pub skipped: Vec<Skipped>,
    table: &'p Table<'p>,
    sources: Vec<&'p Source>,

    /// The table's column for each key id
    columns: Vec<Option<Column>>,

    /// The row each patched row goes to, in order
    targets: Vec<u32>,
}

impl Plan<'_> {
    fn patched_rows(&self) -> impl Iterator<Item = (&Source, &[Cell])> {
        self.sources
            .iter()
            .flat_map(|&source| source.rows().map(move |row| (source, row)))
    }

    // Finds each row by the key it has once written, among the table's rows and those added
    // before it
    fn find_targets(&mut self) {
        let table = self.table;
        let key = key_column(table);
        let mut probe = vec![0; key.map_or(0, |c| c.size)];
        let mut added_keys = Vec::new();
        let mut targets = Vec::new();
        let (mut added, mut replaced) = (0, 0);

        for (source, row) in self.patched_rows() {
            let mut found = None;

            if let Some(column) = key {
                probe.fill(0);

                let cell = row
                    .iter()
                    .rev()
                    .find(|c| self.columns[c.key()] == Some(column));

                if let Some(&cell) = cell {
                    source.write(cell, &mut probe);
                    let range = column.offset..column.offset + column.size;

                    found = table
                        .rows()
                        .position(|r| r.get(range.clone()) == Some(&probe[..]));

                    if found.is_none() {
                        found = added_keys
                            .chunks_exact(column.size)
                            .position(|k| k == &probe[..])
                            .map(|index| table.row_count + index);
                    }
                }
            }

            let target = match found {
                Some(index) => {
                    replaced += 1;
                    index
                }
                None => {
                    added_keys.extend_from_slice(&probe);
                    added += 1;
                    table.row_count + added - 1
                }
            };

            targets.push(target as u32);
        }

        (self.added, self.replaced, self.targets) = (added, replaced, targets);
    }

    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }

    pub fn rows(&self) -> usize {
        self.table.row_count + self.added
    }

    pub fn size(&self) -> usize {
        self.table.size(self.rows())
    }

    /// Writes the patched table to `out`, which is [`Self::size`] bytes
    pub fn write(&self, out: &mut [u8]) -> Result<(), WriteError> {
        let table = self.table;
        table.write(self.rows(), out)?;

        for ((source, row), &target) in self.patched_rows().zip(&self.targets) {
            let at = table.row_at(target as usize);
            let out = &mut out[at..at + table.row_size];

            for &cell in row {
                if let Some(column) = self.columns[cell.key()] {
                    source.write(cell, &mut out[column.offset..column.offset + column.size]);
                }
            }
        }

        Ok(())
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, WriteError> {
        let mut out = vec![0; self.size()];
        self.write(&mut out)?;

        Ok(out)
    }
}

fn key_column(table: &Table) -> Option<Column> {
    KEY_COLUMNS
        .iter()
        .map(|name| crc32(name))
        .chain([UNNAMED_KEY_COLUMN])
        .find_map(|hash| table.column(hash))
        .filter(|column| column.size > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    // `UniqueID u16` at 4, `Label string8` at 8 and an unnamed hash column at 16, two rows
    fn table() -> Vec<u8> {
        let columns = [
            (crc32("UniqueID u16"), 4u32),
            (crc32("Label string8"), 8),
            (0xbea8_4522, 16),
        ];

        let mut data = Vec::new();
        data.extend_from_slice(&2u32.to_le_bytes());
        data.extend_from_slice(&20u32.to_le_bytes());
        data.extend_from_slice(&(columns.len() as u16).to_le_bytes());
        data.extend_from_slice(&[0, 1]);

        for (hash, offset) in columns {
            data.extend_from_slice(&hash.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
        }

        for (id, label) in [(1u16, b"cat01"), (2, b"cat02")] {
            let mut row = vec![0u8; 20];
            row[4..6].copy_from_slice(&id.to_le_bytes());
            row[8..13].copy_from_slice(label);
            data.extend_from_slice(&row);
        }

        data
    }

    fn apply(set: &PatchSet) -> (usize, usize, Vec<Skipped>, Vec<u8>) {
        let data = table();
        let table = Table::parse(&data).unwrap();
        let plan = set.plan("T", &table);
        let out = plan.to_bytes().unwrap();

        (plan.replaced, plan.added, plan.skipped, out)
    }

    #[test]
    fn replaces_and_adds_rows() {
        let mut set = PatchSet::default();

        set.add(
            "a.json",
            r#"[{ "file": "T", "rows": [
                { "UniqueID u16": 2, "Label string8": "wol13", "0xbea84522": { "crc32": "Normal" } },
                { "UniqueID u16": 9, "Label string8": "squ22" }
            ] }]"#,
        )
        .unwrap();

        let (replaced, added, skipped, data) = apply(&set);
        assert_eq!((replaced, added), (1, 1));
        assert!(skipped.is_empty());

        let table = Table::parse(&data).unwrap();
        assert_eq!(table.row_count, 3);
        let row = |index| table.row(index).unwrap();
        assert_eq!(&row(1)[8..16], b"wol13\0\0\0");
        assert_eq!(&row(1)[16..20], &crc32("Normal").to_le_bytes());
        assert_eq!(&row(2)[0..4], &0x4cu32.to_le_bytes());
        assert_eq!(&row(2)[4..6], &9u16.to_le_bytes());
        assert_eq!(&row(0)[8..13], b"cat01");
    }

    #[test]
    fn later_patch_wins() {
        let mut set = PatchSet::default();

        for (patch, label) in [("a.json", "first"), ("b.json", "second")] {
            let json = format!(
                r#"{{ "file": "T", "rows": [{{ "UniqueID u16": 1, "Label string8": "{label}" }}] }}"#
            );
            set.add(patch, &json).unwrap();
        }

        let (replaced, _, _, data) = apply(&set);
        assert_eq!(replaced, 2);
        let table = Table::parse(&data).unwrap();
        assert_eq!(&table.row(0).unwrap()[8..15], b"second\0");
    }

    #[test]
    fn added_rows_are_patched_again() {
        let mut set = PatchSet::default();

        set.add(
            "a.json",
            r#"{ "file": "T", "rows": [
                { "UniqueID u16": 9, "Label string8": "first" },
                { "UniqueID u16": 9, "Label string8": "second" }
            ] }"#,
        )
        .unwrap();

        let (replaced, added, _, data) = apply(&set);
        assert_eq!((replaced, added), (1, 1));
        let table = Table::parse(&data).unwrap();
        assert_eq!(table.row_count, 3);
        assert_eq!(&table.row(2).unwrap()[8..15], b"second\0");
    }

    #[test]
    fn skips_patches_that_dont_fit() {
        let mut set = PatchSet::default();

        set.add(
            "bad.json",
            r#"{ "file": "T", "rows": [
                { "UniqueID u16": 5 },
                { "UniqueID u16": 6, "Label string8": "too long!" }
            ] }"#,
        )
        .unwrap();

        set.add(
            "good.json",
            r#"{ "file": "T", "rows": [{ "UniqueID u16": 7 }] }"#,
        )
        .unwrap();

        let (_, added, skipped, data) = apply(&set);
        assert_eq!(added, 1);

        assert_eq!(
            skipped,
            [Skipped {
                patch: "bad.json".into(),
                table: "T".into(),
                error: CellError::TextTooLong {
                    key: "Label string8".into(),
                    len: 9,
                    size: 8
                },
            }]
        );

        assert_eq!(Table::parse(&data).unwrap().row_count, 3);
    }

    #[test]
    fn plans_nothing_without_patches() {
        let data = table();
        let table = Table::parse(&data).unwrap();
        let set = PatchSet::default();
        let plan = set.plan("T", &table);
        assert!(plan.is_empty());
        assert_eq!(plan.to_bytes().unwrap(), data);

        assert_eq!(
            plan.write(&mut [0; 4]),
            Err(WriteError::BufferSize {
                len: 4,
                size: data.len()
            })
        );
    }

    #[test]
    fn checks_cell_sizes() {
        let mut source = Source::default();

        let mut blob = |bytes: &[u8]| {
            let at = source.start_blob();
            source.bytes.extend_from_slice(bytes);
            source.end_blob(at);
            at
        };

        let (ab, abcd, bytes) = (blob(b"ab"), blob(b"abcd"), blob(&[1, 2]));

        let fits = |cell, size| source.fits(cell, "k", size);
        let integer = |n: i64| Cell::new(0, if n < 0 { SIGNED } else { UNSIGNED }, n as u32);
        assert!(fits(integer(65535), 2).is_ok());
        assert!(fits(integer(-32768), 2).is_ok());

        assert_eq!(
            fits(integer(65536), 2),
            Err(CellError::Integer {
                key: "k".into(),
                value: 65536,
                size: 2
            })
        );

        assert!(fits(integer(1), 8).is_err());
        assert!(fits(integer(1), 0).is_err());
        assert!(fits(Cell::new(0, FLOAT, 0), 2).is_err());
        assert!(fits(Cell::new(0, TEXT, abcd), 4).is_err());
        assert!(fits(Cell::new(0, BYTES, bytes), 3).is_err());
        assert!(source.blob_at(u32::MAX).is_empty());

        let mut out = [9; 4];
        source.write(integer(3), &mut out);
        assert_eq!(out, [3, 0, 0, 0]);
        source.write(integer(-2), &mut out[..2]);
        assert_eq!(out[..2], [0xfe, 0xff]);
        source.write(Cell::new(0, TEXT, ab), &mut out);
        assert_eq!(&out, b"ab\0\0");
    }

    #[test]
    fn rejects_bad_patches() {
        let mut set = PatchSet::default();
        assert!(set.add("x", "{}").is_err());

        assert!(
            set.add("x", r#"{"file":"T","rows":[{"0xzz112233":1}]}"#)
                .is_err()
        );

        assert!(
            set.add("x", r#"{"file":"T","rows":[{"Bits u8":"zz"}]}"#)
                .is_err()
        );

        assert!(
            set.add("x", r#"{"file":"T","rows":[{"A":{"sha1":"x"}}]}"#)
                .is_err()
        );

        assert!(set.add("x", r#"{"file":"T","rows":[{"A":1e30}]}"#).is_ok());

        assert!(
            set.add("y", r#"{"file":"T","rows":[{"A":5000000000}]}"#)
                .is_err()
        );

        assert!(set.add("x", r#"{"file":"T","extra":1,"rows":[]}"#).is_err());

        assert!(
            set.add("x", r#"[{"file":"T","rows":[]}, {"file":"U"}]"#)
                .is_err()
        );

        assert_eq!(set.files().collect::<Vec<_>>(), ["T"]);

        let error = set
            .add("z.json", r#"{"file":"T","rows":[{"B":"q"}]}"#)
            .unwrap_err();

        assert_eq!(error.patch, "z.json");

        assert!(
            error.to_string().starts_with(r#"z.json: B: "q" isn't hex"#),
            "{error}"
        );
    }
}

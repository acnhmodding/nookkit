use std::collections::BTreeMap;
use std::fmt;

use nookkit_hash::{crc32, mmh3};
use serde::{Deserialize, Serialize};

use crate::table::Table;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    /// The table without `.bcsv`, e.g. `NmlNpcParam`
    pub file: String,
    pub rows: Vec<BTreeMap<String, Value>>,
}

/// A cell as JSON. The column's name tells what a plain string is: text for `… stringN`, and hex
/// bytes otherwise
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(String),
    Hash(Hash),
}

/// A string stored as its hash. It's always tagged, since no column name is known to say which
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Hash {
    Crc32(String),
    Mmh3(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatchError {
    pub source: String,
    pub reason: String,
}

impl fmt::Display for PatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.source, self.reason)
    }
}

impl std::error::Error for PatchError {}

/// The columns that identify a row, the first one a table has. A patched row replaces the row with
/// the same value there, and is added after the others otherwise
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

/// The key of the MessageCardSelect tables, whose name isn't known
const UNNAMED_KEY_COLUMN: u32 = 0x3757_1146;

/// A cell, typed as far as the JSON tells
#[derive(Debug, Clone)]
enum Cell {
    Integer(i64),
    Float(f32),
    /// Written with a NUL after it
    Text(Vec<u8>),
    Bytes(Vec<u8>),
    Hash(u32),
}

#[derive(Debug, Clone)]
struct Row {
    /// The column's key as written, its hash and the cell
    cells: Vec<(String, u32, Cell)>,
}

#[derive(Debug)]
struct Source {
    name: String,
    rows: Vec<Row>,
}

/// Every loaded patch, by table. Patches apply in the order they were added, so a row patched
/// twice keeps the later one
#[derive(Debug, Default)]
pub struct PatchSet {
    files: BTreeMap<String, Vec<Source>>,
}

/// What [`PatchSet::apply`] did to one table
#[derive(Debug, Default, PartialEq)]
pub struct Applied {
    pub added: usize,
    pub replaced: usize,
    /// Patches that didn't fit the table and changed nothing
    pub errors: Vec<PatchError>,
}

impl PatchSet {
    /// Adds the patches in `json`, one patch or an array of them, named `source` in errors.
    /// Nothing is added unless every patch reads
    pub fn add(&mut self, source: &str, json: &str) -> Result<(), PatchError> {
        let error = |reason: String| PatchError {
            source: source.to_owned(),
            reason,
        };

        // Picked by the first character rather than an untagged enum, which would drop serde's
        // line and column from errors
        let patches: Vec<Patch> = if json.trim_start().starts_with('[') {
            serde_json::from_str(json)
        } else {
            serde_json::from_str(json).map(|patch| vec![patch])
        }
        .map_err(|e| error(e.to_string()))?;

        let read = patches
            .into_iter()
            .map(|patch| {
                let rows = patch
                    .rows
                    .into_iter()
                    .map(|row| read_row(row).map_err(|e| error(format!("{} {e}", patch.file))))
                    .collect::<Result<Vec<_>, _>>()?;
                Ok((patch.file, rows))
            })
            .collect::<Result<Vec<_>, PatchError>>()?;

        for (file, rows) in read {
            self.files.entry(file).or_default().push(Source {
                name: source.to_owned(),
                rows,
            });
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

    /// Applies every patch for `file` to `table`. A patch with a row that doesn't fit the table is
    /// skipped whole and reported
    pub fn apply(&self, file: &str, table: &mut Table) -> Applied {
        let mut applied = Applied::default();
        let key = key_column(table);

        for source in self.files.get(file).into_iter().flatten() {
            let resolved = source
                .rows
                .iter()
                .map(|row| resolve(table, row))
                .collect::<Result<Vec<_>, _>>();

            let rows = match resolved {
                Ok(rows) => rows,
                Err(reason) => {
                    applied.errors.push(PatchError {
                        source: source.name.clone(),
                        reason: format!("{file} {reason}"),
                    });
                    continue;
                }
            };

            for writes in rows {
                let index = key.and_then(|(offset, size)| {
                    let (_, value) = writes.iter().find(|(at, _)| *at == offset)?;
                    table
                        .rows
                        .iter()
                        .position(|row| row[offset..offset + size] == value[..])
                });

                let row = match index {
                    Some(index) => {
                        applied.replaced += 1;
                        &mut table.rows[index]
                    }
                    None => {
                        applied.added += 1;
                        table.rows.push(vec![0; table.row_size]);
                        table.rows.last_mut().unwrap()
                    }
                };

                for (offset, bytes) in writes {
                    row[offset..offset + bytes.len()].copy_from_slice(&bytes);
                }
            }
        }

        applied
    }
}

fn key_column(table: &Table) -> Option<(usize, usize)> {
    KEY_COLUMNS
        .iter()
        .map(|name| crc32(name))
        .chain([UNNAMED_KEY_COLUMN])
        .find_map(|hash| table.column(hash))
        .map(|column| (column.offset, column.size))
}

fn read_row(row: BTreeMap<String, Value>) -> Result<Row, String> {
    let cells = row
        .into_iter()
        .map(|(key, value)| {
            let hash = match key.strip_prefix("0x") {
                Some(hex) if hex.len() == 8 => {
                    u32::from_str_radix(hex, 16).map_err(|_| format!("{key}: not a column hash"))?
                }
                _ => crc32(&key),
            };

            let cell = match value {
                Value::Integer(n) => Cell::Integer(n),
                Value::Float(f) => Cell::Float(f as f32),
                Value::Hash(Hash::Crc32(text)) => Cell::Hash(crc32(&text)),
                Value::Hash(Hash::Mmh3(text)) => Cell::Hash(mmh3(&text)),
                Value::String(text) if key.contains(" string") => Cell::Text(text.into_bytes()),
                Value::String(hex) => {
                    Cell::Bytes(from_hex(&hex).ok_or_else(|| format!("{key}: {hex:?} isn't hex"))?)
                }
            };

            Ok((key, hash, cell))
        })
        .collect::<Result<_, String>>()?;

    Ok(Row { cells })
}

/// The row's cells as bytes at their offsets
fn resolve(table: &Table, row: &Row) -> Result<Vec<(usize, Vec<u8>)>, String> {
    row.cells
        .iter()
        .map(|(key, hash, cell)| {
            let column = table
                .column(*hash)
                .ok_or_else(|| format!("has no {key} column"))?;
            let bytes = encode(cell, column.size).map_err(|e| format!("{key}: {e}"))?;
            Ok((column.offset, bytes))
        })
        .collect()
}

fn encode(cell: &Cell, size: usize) -> Result<Vec<u8>, String> {
    let mut bytes = match cell {
        Cell::Integer(n) => {
            // A u8 that ends a row is padded, so only its first four bytes are the number
            let width = size.min(4);
            let bits = width as u32 * 8;
            let fits = (-(1i64 << (bits - 1))..(1i64 << bits)).contains(n);
            if size > 4 || !fits {
                return Err(format!("{n} doesn't fit {size} bytes"));
            }
            n.to_le_bytes()[..width].to_vec()
        }
        Cell::Float(f) if size == 4 => f.to_le_bytes().to_vec(),
        Cell::Hash(h) if size == 4 => h.to_le_bytes().to_vec(),
        Cell::Float(_) | Cell::Hash(_) => return Err(format!("the cell is {size} bytes, not 4")),
        Cell::Text(text) if text.len() < size => text.clone(),
        Cell::Text(text) => {
            return Err(format!(
                "{} bytes of text don't fit {size} with the NUL",
                text.len()
            ));
        }
        Cell::Bytes(bytes) if bytes.len() == size => bytes.clone(),
        Cell::Bytes(bytes) => return Err(format!("{} bytes for a {size} byte cell", bytes.len())),
    };

    bytes.resize(size, 0);
    Ok(bytes)
}

fn from_hex(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(hex.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::Table;

    /// `UniqueID u16` at 4, `Label string8` at 8 and an unnamed hash column at 16, two rows
    fn table() -> Table {
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
        Table::parse(&data).unwrap()
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

        let mut table = table();
        let applied = set.apply("T", &mut table);
        assert_eq!((applied.replaced, applied.added), (1, 1));
        assert!(applied.errors.is_empty());

        assert_eq!(table.rows.len(), 3);
        assert_eq!(&table.rows[1][8..16], b"wol13\0\0\0");
        assert_eq!(&table.rows[1][16..20], &crc32("Normal").to_le_bytes());
        assert_eq!(&table.rows[2][4..6], &9u16.to_le_bytes());
        assert_eq!(&table.rows[0][8..13], b"cat01");
    }

    #[test]
    fn later_patch_wins() {
        let mut set = PatchSet::default();
        for (source, label) in [("a.json", "first"), ("b.json", "second")] {
            let json = format!(
                r#"{{ "file": "T", "rows": [{{ "UniqueID u16": 1, "Label string8": "{label}" }}] }}"#
            );
            set.add(source, &json).unwrap();
        }

        let mut table = table();
        assert_eq!(set.apply("T", &mut table).replaced, 2);
        assert_eq!(&table.rows[0][8..15], b"second\0");
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

        let mut table = table();
        let applied = set.apply("T", &mut table);
        assert_eq!(applied.added, 1);
        assert_eq!(applied.errors.len(), 1);
        assert_eq!(applied.errors[0].source, "bad.json");
        assert!(applied.errors[0].reason.starts_with("T Label string8:"));
        assert_eq!(table.rows.len(), 3);
    }

    #[test]
    fn checks_cell_sizes() {
        assert!(encode(&Cell::Integer(65535), 2).is_ok());
        assert!(encode(&Cell::Integer(-32768), 2).is_ok());
        assert!(encode(&Cell::Integer(65536), 2).is_err());
        assert_eq!(encode(&Cell::Integer(3), 4).unwrap(), [3, 0, 0, 0]);
        assert!(encode(&Cell::Integer(1), 8).is_err());
        assert!(encode(&Cell::Float(1.0), 2).is_err());
        assert_eq!(encode(&Cell::Text(b"ab".to_vec()), 4).unwrap(), b"ab\0\0");
        assert!(encode(&Cell::Text(b"abcd".to_vec()), 4).is_err());
        assert!(encode(&Cell::Bytes(vec![1, 2]), 3).is_err());
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
        assert!(set.add("x", r#"{"file":"T","extra":1,"rows":[]}"#).is_err());
        assert!(set.is_empty());
    }
}

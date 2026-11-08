//! Reads patch JSON straight into a set's sources, without a value for each cell first

use std::fmt;

use nookkit_hash::{crc32, mmh3};
use serde::Deserialize;
use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::error::Invalid;
use crate::patch::{BYTES, Cell, FLOAT, HASH, Keys, SIGNED, Source, TEXT, UNSIGNED};

/// The patches in `json`, one patch or an array of them, as (table, rows). The column keys they
/// name are added to `keys`
pub(crate) fn patches(
    keys: &mut Keys,
    patch: &str,
    json: &str,
) -> Result<Vec<(String, Source)>, serde_json::Error> {
    let mut read = Vec::new();
    let mut de = serde_json::Deserializer::from_str(json);
    de.deserialize_any(Patches {
        keys,
        name: patch,
        read: &mut read,
    })?;
    de.end()?;
    Ok(read)
}

struct Patches<'s> {
    keys: &'s mut Keys,
    name: &'s str,
    read: &'s mut Vec<(String, Source)>,
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum Field {
    File,
    Rows,
}

impl<'de> Visitor<'de> for Patches<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a patch or an array of patches")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<(), A::Error> {
        while let Some(patch) = seq.next_element_seed(PatchSeed {
            keys: &mut *self.keys,
            name: self.name,
        })? {
            self.read.push(patch);
        }

        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<(), A::Error> {
        let patch = PatchSeed {
            keys: self.keys,
            name: self.name,
        }
        .visit_map(map)?;

        self.read.push(patch);

        Ok(())
    }
}

struct PatchSeed<'s> {
    keys: &'s mut Keys,
    name: &'s str,
}

impl<'de> DeserializeSeed<'de> for PatchSeed<'_> {
    type Value = (String, Source);

    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        de.deserialize_struct("Patch", &["file", "rows"], self)
    }
}

impl<'de> Visitor<'de> for PatchSeed<'_> {
    type Value = (String, Source);

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a patch")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut file = None;
        let mut source = None;

        while let Some(field) = map.next_key()? {
            match field {
                Field::File if file.is_some() => return Err(de::Error::duplicate_field("file")),
                Field::Rows if source.is_some() => return Err(de::Error::duplicate_field("rows")),
                Field::File => file = Some(map.next_value::<String>()?),
                Field::Rows => {
                    source = Some(map.next_value_seed(RowsSeed {
                        keys: &mut *self.keys,
                        name: self.name,
                    })?);
                }
            }
        }

        let file = file.ok_or_else(|| de::Error::missing_field("file"))?;
        let mut source: Source = source.ok_or_else(|| de::Error::missing_field("rows"))?;
        source.cells.shrink_to_fit();
        source.ends.shrink_to_fit();
        source.bytes.shrink_to_fit();

        Ok((file, source))
    }
}

struct RowsSeed<'s> {
    keys: &'s mut Keys,
    name: &'s str,
}

impl<'de> DeserializeSeed<'de> for RowsSeed<'_> {
    type Value = Source;

    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Source, D::Error> {
        de.deserialize_seq(self)
    }
}

impl<'de> Visitor<'de> for RowsSeed<'_> {
    type Value = Source;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("an array of rows")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Source, A::Error> {
        let mut source = Source {
            name: self.name.to_owned(),
            ..Source::default()
        };

        while seq
            .next_element_seed(RowSeed {
                keys: &mut *self.keys,
                source: &mut source,
            })?
            .is_some()
        {}

        Ok(source)
    }
}

struct RowSeed<'s> {
    keys: &'s mut Keys,
    source: &'s mut Source,
}

impl<'de> DeserializeSeed<'de> for RowSeed<'_> {
    type Value = ();

    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<(), D::Error> {
        de.deserialize_map(self)
    }
}

impl<'de> Visitor<'de> for RowSeed<'_> {
    type Value = ();

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a row of cells by column")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while let Some(key) = map.next_key_seed(KeySeed(&mut *self.keys))? {
            let cell = map.next_value_seed(CellSeed {
                keys: &*self.keys,
                key,
                source: &mut *self.source,
            })?;

            self.source.cells.push(cell);
        }

        let end = self.source.cells.len() as u32;
        self.source.ends.push(end);

        Ok(())
    }
}

struct KeySeed<'s>(&'s mut Keys);

impl<'de> DeserializeSeed<'de> for KeySeed<'_> {
    type Value = u32;

    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<u32, D::Error> {
        de.deserialize_str(self)
    }
}

impl<'de> Visitor<'de> for KeySeed<'_> {
    type Value = u32;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a column name or hash")
    }

    fn visit_str<E: de::Error>(self, key: &str) -> Result<u32, E> {
        self.0.id(key).map_err(E::custom)
    }
}

struct CellSeed<'s> {
    keys: &'s Keys,
    key: u32,
    source: &'s mut Source,
}

impl CellSeed<'_> {
    fn key(&self) -> String {
        self.keys.name(self.key).to_owned()
    }

    fn integer<E: de::Error>(self, value: i128) -> Result<Cell, E> {
        if (0..1 << 32).contains(&value) {
            Ok(Cell::new(self.key, UNSIGNED, value as u32))
        } else if (i32::MIN as i128..0).contains(&value) {
            Ok(Cell::new(self.key, SIGNED, value as i32 as u32))
        } else {
            Err(E::custom(Invalid::Integer {
                key: self.key(),
                value,
            }))
        }
    }
}

impl<'de> DeserializeSeed<'de> for CellSeed<'_> {
    type Value = Cell;

    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Cell, D::Error> {
        de.deserialize_any(self)
    }
}

#[derive(Deserialize)]
#[serde(field_identifier, rename_all = "lowercase")]
enum HashKind {
    Crc32,
    Mmh3,
}

impl<'de> Visitor<'de> for CellSeed<'_> {
    type Value = Cell;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(r#"a number, a string or { "crc32" | "mmh3": string }"#)
    }

    fn visit_i64<E: de::Error>(self, n: i64) -> Result<Cell, E> {
        self.integer(n.into())
    }

    fn visit_u64<E: de::Error>(self, n: u64) -> Result<Cell, E> {
        self.integer(n.into())
    }

    fn visit_f64<E: de::Error>(self, f: f64) -> Result<Cell, E> {
        Ok(Cell::new(self.key, FLOAT, (f as f32).to_bits()))
    }

    fn visit_str<E: de::Error>(self, text: &str) -> Result<Cell, E> {
        let at = self.source.start_blob();

        if self.keys.is_text(self.key) {
            self.source.bytes.extend_from_slice(text.as_bytes());
            self.source.end_blob(at);
            return Ok(Cell::new(self.key, TEXT, at));
        }

        let pairs = text.as_bytes().chunks(2).map(|pair| {
            let pair = std::str::from_utf8(pair).ok().filter(|p| p.len() == 2)?;
            u8::from_str_radix(pair, 16).ok()
        });

        for byte in pairs {
            let Some(byte) = byte else {
                self.source.bytes.truncate(at as usize);

                return Err(E::custom(Invalid::Hex {
                    key: self.key(),
                    text: text.to_owned(),
                }));
            };

            self.source.bytes.push(byte);
        }

        self.source.end_blob(at);

        Ok(Cell::new(self.key, BYTES, at))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Cell, A::Error> {
        let Some(kind) = map.next_key::<HashKind>()? else {
            return Err(de::Error::custom(Invalid::HashKind { key: self.key() }));
        };

        let text = map.next_value::<String>()?;

        if map.next_key::<HashKind>()?.is_some() {
            return Err(de::Error::custom(Invalid::HashStrings { key: self.key() }));
        }

        let hash = match kind {
            HashKind::Crc32 => crc32(&text),
            HashKind::Mmh3 => mmh3(&text),
        };

        Ok(Cell::new(self.key, HASH, hash))
    }
}

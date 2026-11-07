use std::collections::BTreeMap;
use std::fmt;

use serde::Deserialize;
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

use crate::byml::{Byml, Format, Node};
use crate::error::{ApplyError, ReadError, Replaced};

/// Replaces one BYML file in a pack, as Heaven Tool's `PackPatch` writes it
#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    /// The file's path in the pack, e.g. `Param/NpcHouse/wol13.byml`
    pub file: String,
    /// The pack's name, `StaticParam` for `Pack/StaticParam.pack`
    pub archive: String,
    /// The whole file as plain JSON
    pub data: Node,
}

/// A patch file holds one patch or an array of them
struct Patches(Vec<Patch>);

impl<'de> Deserialize<'de> for Patches {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        de.deserialize_any(PatchesVisitor)
    }
}

struct PatchesVisitor;

impl<'de> Visitor<'de> for PatchesVisitor {
    type Value = Patches;

    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("a patch or an array of patches")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Patches, A::Error> {
        let mut patches = Vec::new();
        while let Some(patch) = seq.next_element()? {
            patches.push(patch);
        }
        Ok(Patches(patches))
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Patches, A::Error> {
        let patch = Patch::deserialize(de::value::MapAccessDeserializer::new(map))?;
        Ok(Patches(vec![patch]))
    }
}

/// A file's replacement
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    /// The patch file it came from
    pub patch: String,
    pub root: Node,
}

impl Entry {
    /// The replacement for `original`, in its byte order and version
    pub fn apply(&self, original: &[u8]) -> Result<Vec<u8>, ApplyError> {
        let byml = Byml {
            format: Format::read(original)?,
            root: self.root.clone(),
        };
        Ok(byml.to_bytes()?)
    }
}

/// Every loaded patch, by pack then file. A file patched twice keeps the later one
#[derive(Debug, Default)]
pub struct PatchSet {
    archives: BTreeMap<String, BTreeMap<String, Entry>>,
}

impl PatchSet {
    /// Adds the patches in `json`, from the patch file `patch`, returning the files they take
    /// from earlier patches. A file that doesn't read adds nothing
    pub fn add(&mut self, patch: &str, json: &str) -> Result<Vec<Replaced>, ReadError> {
        let Patches(patches) = serde_json::from_str(json).map_err(|error| ReadError {
            patch: patch.to_owned(),
            error,
        })?;

        let mut replaced = Vec::new();
        for Patch {
            file,
            archive,
            data,
        } in patches
        {
            let entry = Entry {
                patch: patch.to_owned(),
                root: data,
            };
            let files = self.archives.entry(archive.clone()).or_default();
            if let Some(earlier) = files.insert(file.clone(), entry) {
                replaced.push(Replaced {
                    archive,
                    file,
                    patch: earlier.patch,
                });
            }
        }
        Ok(replaced)
    }

    pub fn get(&self, archive: &str, file: &str) -> Option<&Entry> {
        self.archives.get(archive)?.get(file)
    }

    /// The patched files in `archive`
    pub fn files(&self, archive: &str) -> impl Iterator<Item = (&str, &Entry)> {
        self.archives
            .get(archive)
            .into_iter()
            .flatten()
            .map(|(file, entry)| (file.as_str(), entry))
    }

    pub fn archives(&self) -> impl Iterator<Item = &str> {
        self.archives.keys().map(String::as_str)
    }

    pub fn len(&self) -> usize {
        self.archives.values().map(BTreeMap::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.archives.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::byml::Endian;

    const HOUSE: &str = r#"[{
        "file": "Param/NpcHouse/wol13.byml",
        "archive": "StaticParam",
        "data": { "a": 1, "b": [2.5, "x"] }
    }]"#;

    #[test]
    fn reads_one_patch_or_an_array() {
        let mut set = PatchSet::default();
        assert_eq!(set.add("house.json", HOUSE).unwrap(), []);
        let single = r#"{ "file": "Param/Gmo/a.byml", "archive": "StaticParam", "data": [] }"#;
        assert_eq!(set.add("gmo.json", single).unwrap(), []);
        assert_eq!(set.len(), 2);
        assert_eq!(set.archives().collect::<Vec<_>>(), ["StaticParam"]);

        let entry = set.get("StaticParam", "Param/Gmo/a.byml").unwrap();
        assert_eq!(entry.root, Node::Array(vec![]));
        assert_eq!(entry.patch, "gmo.json");
    }

    #[test]
    fn keeps_the_later_patch() {
        let mut set = PatchSet::default();
        set.add("first.json", HOUSE).unwrap();
        let replaced = set.add("second.json", HOUSE).unwrap();
        assert_eq!(
            replaced,
            [Replaced {
                archive: "StaticParam".into(),
                file: "Param/NpcHouse/wol13.byml".into(),
                patch: "first.json".into(),
            }]
        );
        assert_eq!(set.files("StaticParam").count(), 1);
        assert_eq!(
            set.get("StaticParam", "Param/NpcHouse/wol13.byml")
                .unwrap()
                .patch,
            "second.json"
        );
    }

    #[test]
    fn refuses_a_broken_file_whole() {
        let mut set = PatchSet::default();
        let json = r#"[{ "file": "a", "archive": "b", "data": {} }, { "file": "c" }]"#;
        let error = set.add("broken.json", json).unwrap_err();
        assert_eq!(error.patch, "broken.json");
        assert!(set.is_empty());
    }

    #[test]
    fn applies_in_the_original_format() {
        let mut set = PatchSet::default();
        set.add("house.json", HOUSE).unwrap();
        let entry = set.get("StaticParam", "Param/NpcHouse/wol13.byml").unwrap();

        let original = Byml {
            format: Format {
                endian: Endian::Big,
                version: 3,
            },
            root: Node::Null,
        }
        .to_bytes()
        .unwrap();
        let patched = Byml::parse(&entry.apply(&original).unwrap()).unwrap();
        assert_eq!(patched.format, Format::read(&original).unwrap());
        assert_eq!(patched.root, entry.root);

        assert_eq!(
            entry.apply(b"nope"),
            Err(ApplyError::Parse(crate::ParseError::Magic))
        );
    }
}

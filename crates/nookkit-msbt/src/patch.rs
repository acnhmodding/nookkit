//! Message patches: one JSON file per mod, each label carrying its text in every locale

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

use crate::tags;

/// The text used for a locale a message doesn't list
pub const DEFAULT_LOCALE: &str = "default";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    /// The msbt as the game asks for it, without `.msbt`, e.g. `MenuDevice`
    pub file: String,
    /// The `Message/<archive>_<locale>` archive holding the file, for tools
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
    pub messages: Vec<Message>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Message {
    pub label: String,
    /// Locale code (`USen`, `JPja`, …) or `default` to tagged text
    pub text: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Attributes>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase", deny_unknown_fields)]
pub enum Attributes {
    /// Another label's ATR1 entry in the same file
    Like(String),
    /// The ATR1 entry itself, the file's attribute size in bytes
    Hex(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    Like(String),
    Bytes(Vec<u8>),
}

#[derive(Debug, Clone)]
pub struct Entry {
    /// Encoded MsgStd UTF-16 per locale, NUL included
    pub text: BTreeMap<String, Vec<u16>>,
    pub attribute: Option<Attribute>,
    /// Which patch it came from
    pub source: String,
}

impl Entry {
    pub fn text_for(&self, locale: &str) -> Option<&[u16]> {
        self.text
            .get(locale)
            .or_else(|| self.text.get(DEFAULT_LOCALE))
            .map(Vec::as_slice)
    }
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

/// Every loaded patch, by file then label. A label patched twice keeps the later one
#[derive(Debug, Default)]
pub struct PatchSet {
    files: BTreeMap<String, BTreeMap<String, Entry>>,
}

impl PatchSet {
    /// Adds the patch in `json`, named `source` in errors and warnings. Returns the labels it took
    /// over from earlier patches
    pub fn add(&mut self, source: &str, json: &str) -> Result<Vec<String>, PatchError> {
        let error = |reason: String| PatchError {
            source: source.to_owned(),
            reason,
        };

        let patch: Patch = serde_json::from_str(json).map_err(|e| error(e.to_string()))?;
        let mut entries = Vec::with_capacity(patch.messages.len());

        for message in patch.messages {
            let mut text = BTreeMap::new();
            for (locale, value) in message.text {
                let units = tags::encode(&value).map_err(|e| {
                    error(format!("{} {} [{locale}] {e}", patch.file, message.label))
                })?;
                text.insert(locale, units);
            }

            let attribute = match message.attributes {
                None => None,
                Some(Attributes::Like(label)) => Some(Attribute::Like(label)),
                Some(Attributes::Hex(hex)) => Some(Attribute::Bytes(tags::from_hex(&hex).ok_or_else(
                    || error(format!("{} {}: attributes {hex:?} isn't hex", patch.file, message.label)),
                )?)),
            };

            entries.push((
                message.label,
                Entry {
                    text,
                    attribute,
                    source: source.to_owned(),
                },
            ));
        }

        let labels = self.files.entry(patch.file.clone()).or_default();
        let mut replaced = Vec::new();

        for (label, entry) in entries {
            if let Some(old) = labels.insert(label.clone(), entry) {
                replaced.push(format!("{} {label} from {}", patch.file, old.source));
            }
        }

        Ok(replaced)
    }

    /// The patched file named `file`, borrowed from the set
    pub fn file(&self, file: &str) -> Option<&str> {
        self.files.get_key_value(file).map(|(name, _)| name.as_str())
    }

    pub fn get(&self, file: &str, label: &str) -> Option<&Entry> {
        self.files.get(file)?.get(label)
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn len(&self) -> usize {
        self.files.values().map(BTreeMap::len).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISLAND_SKETCH: &str = r#"{
        "file": "MenuDevice",
        "archive": "LayoutMsg",
        "messages": [
            { "label": "IslandSketch", "text": { "USen": "Island Sketch", "default": "Island" },
              "attributes": { "like": "0002" } }
        ]
    }"#;

    #[test]
    fn loads_and_falls_back() {
        let mut set = PatchSet::default();
        assert!(set.add("a.json", ISLAND_SKETCH).unwrap().is_empty());

        let entry = set.get("MenuDevice", "IslandSketch").unwrap();
        assert_eq!(entry.text_for("USen"), Some(&tags::encode("Island Sketch").unwrap()[..]));
        assert_eq!(entry.text_for("JPja"), Some(&tags::encode("Island").unwrap()[..]));
        assert_eq!(entry.attribute, Some(Attribute::Like("0002".into())));
        assert_eq!(set.file("MenuDevice"), Some("MenuDevice"));
        assert!(set.get("MenuDevice", "0002").is_none());
    }

    #[test]
    fn later_patch_wins() {
        let mut set = PatchSet::default();
        set.add("a.json", ISLAND_SKETCH).unwrap();
        let replaced = set.add("b.json", ISLAND_SKETCH).unwrap();

        assert_eq!(replaced, vec!["MenuDevice IslandSketch from a.json".to_owned()]);
        assert_eq!(set.get("MenuDevice", "IslandSketch").unwrap().source, "b.json");
    }

    #[test]
    fn rejects_bad_patches() {
        let mut set = PatchSet::default();
        assert!(set.add("x", "{}").is_err());
        assert!(set.add("x", r#"{"file":"F","messages":[{"label":"L","text":{"USen":"{{Nope}}"}}]}"#).is_err());
        assert!(set.add("x", r#"{"file":"F","messages":[{"label":"L","text":{},"attributes":{"hex":"zz"}}]}"#).is_err());
        assert!(set.add("x", r#"{"file":"F","extra":1,"messages":[]}"#).is_err());
        assert!(set.is_empty());
    }
}

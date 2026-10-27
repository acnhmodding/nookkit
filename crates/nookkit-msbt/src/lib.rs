//! MSBT message patches: JSON files of labels with their text in every locale, the tag syntax
//! they're written in, and the readers tools use to dump the game's own messages

pub mod msbt;
pub mod patch;
pub mod sarc;
pub mod tags;

pub use patch::{Attribute, Attributes, DEFAULT_LOCALE, Entry, Message, Patch, PatchError, PatchSet};

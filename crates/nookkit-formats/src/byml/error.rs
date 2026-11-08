use thiserror::Error;

use crate::byml::document::MAX_DEPTH;

/// A file that doesn't read as a BYML the game accepts
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("not a BYML")]
    Magic,

    #[error("version {0}, the game reads 1 to 4")]
    Version(u16),

    #[error("{offset:#x} is past the end")]
    OutOfBounds { offset: usize },

    #[error("{offset:#x}: not a string table")]
    NotStringTable { offset: usize },

    #[error("{offset:#x}: a string isn't UTF-8")]
    Utf8 { offset: usize },

    #[error("{offset:#x}: no key {index}")]
    KeyIndex { offset: usize, index: u32 },

    #[error("no string {index}")]
    StringIndex { index: u32 },

    #[error("{offset:#x}: unknown node type {node_type:#x}")]
    Type { offset: usize, node_type: u8 },

    #[error("{offset:#x}: a {expected:#x} node holds a {found:#x} header")]
    Header {
        offset: usize,
        expected: u8,
        found: u8,
    },

    #[error("the root isn't an array or a dictionary")]
    Root,

    #[error("containers nest deeper than {MAX_DEPTH}")]
    TooDeep,
}

/// A document that can't be written as a BYML
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WriteError {
    #[error("{text:?} holds a NUL")]
    Nul { text: String },

    #[error("the root isn't an array, a dictionary or null")]
    Root,

    #[error("{count} items don't fit a 24 bit count")]
    TooMany { count: usize },

    #[error("the file passes 4 GiB")]
    TooLarge,
}

/// A node plain JSON can't hold, or JSON that holds no node. Reported through the JSON error,
/// which adds where it is
#[derive(Debug, Clone, PartialEq, Error)]
pub(crate) enum Invalid {
    #[error("a {0} node has no plain JSON form")]
    Unsupported(&'static str),

    #[error("{0} can't be written as a JSON number")]
    NotFinite(f32),

    #[error("{0} doesn't fit an Int64")]
    Integer(u64),

    #[error("the key {0:?} is given twice")]
    DuplicateKey(String),
}

/// A patch file that doesn't read, so none of its patches were added
#[derive(Debug, Error)]
#[error("{patch}: {error}")]
pub struct ReadError {
    /// The patch file's name
    pub patch: String,
    #[source]
    pub error: serde_json::Error,
}

/// A file a patch replaces that an earlier patch replaced too
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("replaces {archive}/{file} from {patch}")]
pub struct Replaced {
    pub archive: String,
    pub file: String,

    /// The earlier patch file's name
    pub patch: String,
}

/// A patch that can't replace the game's file
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ApplyError {
    #[error("the game's file: {0}")]
    Parse(#[from] ParseError),

    #[error(transparent)]
    Write(#[from] WriteError),
}

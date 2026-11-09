use thiserror::Error;

use crate::bytes::OutOfBounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error(transparent)]
    OutOfBounds(#[from] OutOfBounds),

    #[error("the ext header doesn't start with VSCB")]
    ExtHeader,

    #[error("the rows' total size overflows")]
    RowsOverflow,

    #[error("column {index} starts after the next one")]
    ColumnOrder { index: usize },
}

#[derive(Debug, Error)]
#[error("{patch}: {error}")]
pub struct ReadError {
    /// The patch file's name
    pub patch: String,

    #[source]
    pub error: serde_json::Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum Invalid {
    #[error("{key}: not a column hash")]
    ColumnHash { key: String },

    #[error("{key}: {text:?} isn't hex")]
    Hex { key: String, text: String },

    #[error("{key}: {value} doesn't fit 4 bytes")]
    Integer { key: String, value: i128 },

    #[error("{key}: a hash needs crc32 or mmh3")]
    HashKind { key: String },

    #[error("{key}: a hash has one string")]
    HashStrings { key: String },

    #[error("too many column keys")]
    TooManyKeys,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum CellError {
    #[error("has no {key} column")]
    MissingColumn { key: String },

    #[error("{key}: {value} doesn't fit {size} bytes")]
    Integer {
        key: String,
        value: i64,
        size: usize,
    },

    #[error("{key}: the cell is {size} bytes, not 4")]
    NotFourBytes { key: String, size: usize },

    #[error("{key}: {len} bytes of text don't fit {size} with the NUL")]
    TextTooLong {
        key: String,
        len: usize,
        size: usize,
    },

    #[error("{key}: {len} bytes for a {size} byte cell")]
    ByteCount {
        key: String,
        len: usize,
        size: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{patch}: {table} {error}")]
pub struct Skipped {
    /// The patch file's name
    pub patch: String,

    pub table: String,

    #[source]
    pub error: CellError,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WriteError {
    #[error("{rows} rows can't hold the table's {count}")]
    TooFewRows { rows: usize, count: usize },

    #[error("the buffer is {len} bytes, not {size}")]
    BufferSize { len: usize, size: usize },
}

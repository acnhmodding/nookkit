use thiserror::Error;

/// A patch file that doesn't read, so none of its patches were added
#[derive(Debug, Error)]
#[error("{patch}: {error}")]
pub struct ReadError {
    /// The patch file's name
    pub patch: String,
    #[source]
    pub error: serde_json::Error,
}

/// A row in a patch that the JSON allows but a patch can't hold. Reported through the JSON error,
/// which adds where it is
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

/// A cell that doesn't fit its table
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

/// A patch skipped whole since one of its cells doesn't fit the table
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{patch}: {table} {error}")]
pub struct Skipped {
    /// The patch file's name
    pub patch: String,
    pub table: String,
    #[source]
    pub error: CellError,
}

/// A table that can't be written to a buffer
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum WriteError {
    #[error("{rows} rows can't hold the table's {count}")]
    TooFewRows { rows: usize, count: usize },
    #[error("the buffer is {len} bytes, not {size}")]
    BufferSize { len: usize, size: usize },
}

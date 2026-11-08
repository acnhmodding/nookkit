mod bfres;
pub mod error;
mod geometry;
mod heap;
mod physics;
mod reader;
mod shader;
pub mod sizing;
pub mod table;
pub mod yaz0;
mod zstd;

pub use zstd::declared_length;

pub use error::{Error, Result};

/// Bump whenever calculated sizes may change, so cached sizes are recalculated
pub const VERSION: &str = "0.2.0";

pub(crate) use crate::hash::crc32;

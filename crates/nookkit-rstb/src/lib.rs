mod bfres;
mod geometry;
mod heap;
mod physics;
mod reader;
mod sarc;
mod shader;
pub mod sizing;
pub mod table;
pub mod yaz0;
mod zstd;

pub use reader::{Error, Result};

/// Changes whenever calculated sizes may change, so cached sizes are recalculated
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub(crate) fn crc32(name: &str) -> u32 {
    crc32fast::hash(name.as_bytes())
}

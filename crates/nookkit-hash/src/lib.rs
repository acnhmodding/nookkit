//! The game's two string hashes. Most names, bcsv columns among them, are CRC32. Some tables
//! hash their values with MurmurHash3 instead

/// CRC32 of `text`
pub fn crc32(text: &str) -> u32 {
    crc32fast::hash(text.as_bytes())
}

/// MurmurHash3 x86_32 of `text`, with seed 0
pub fn mmh3(text: &str) -> u32 {
    const C1: u32 = 0xcc9e_2d51;
    const C2: u32 = 0x1b87_3593;

    let bytes = text.as_bytes();
    let mut h = 0u32;
    let mut blocks = bytes.chunks_exact(4);

    for block in &mut blocks {
        let k = block
            .iter()
            .rev()
            .fold(0u32, |k, &b| (k << 8) | u32::from(b));

        h ^= k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
        h = h.rotate_left(13).wrapping_mul(5).wrapping_add(0xe654_6b64);
    }

    let tail = blocks.remainder();

    if !tail.is_empty() {
        let k = tail
            .iter()
            .rev()
            .fold(0u32, |k, &b| (k << 8) | u32::from(b));

        h ^= k.wrapping_mul(C1).rotate_left(15).wrapping_mul(C2);
    }

    h ^= bytes.len() as u32;
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);

    h ^ (h >> 16)
}

#[cfg(test)]
mod tests {
    use super::*;

    // Values from the game's StructureFacilityModel and StructureWherearenFacility tables
    #[test]
    fn matches_the_game() {
        assert_eq!(mmh3("OfficeLevel1"), 0x28f6_92dd);
        assert_eq!(mmh3("MuseumTent"), 0x7803_7942);
        assert_eq!(mmh3("WS"), 0x5d78_5775);
        assert_eq!(crc32("UniqueID u16"), 0x5470_6054);
    }
}

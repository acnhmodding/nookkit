pub fn read(b: &[u8]) -> Option<Vec<(String, &[u8])>> {
    let u16_at = |at: usize| Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?));
    let u32_at = |at: usize| Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?));

    if b.get(..4)? != b"SARC" {
        return None;
    }

    let count = u16_at(0x1A)? as usize;
    let data = u32_at(0xC)? as usize;
    let names = 0x20 + count * 16 + 8;

    (0..count)
        .map(|i| {
            let entry = 0x20 + i * 16;
            let name_at = names + (u32_at(entry + 4)? & 0xFF_FFFF) as usize * 4;
            let end = name_at + b.get(name_at..)?.iter().position(|&c| c == 0)?;
            let name = std::str::from_utf8(b.get(name_at..end)?).ok()?.to_owned();
            let (start, stop) = (u32_at(entry + 8)? as usize, u32_at(entry + 12)? as usize);
            Some((name, b.get(data + start..data + stop)?))
        })
        .collect()
}

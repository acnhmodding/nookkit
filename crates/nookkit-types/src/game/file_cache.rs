use crate::sead::SafeString;

/// What `FileCache::load` reads a file or an archive by. For each file of an archive, `insert`
/// gets a copy whose path is the file's
#[repr(C)]
pub struct LoadRequest<'a> {
    _unk_0: [u8; 0x8],
    pub path: SafeString<'a>,
}

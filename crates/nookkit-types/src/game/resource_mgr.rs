use core::ffi::c_void;
use core::mem::offset_of;

#[repr(C)]
pub struct ResourceMgr {
    _unk_0: [u8; 0x820],
    pub size_table: ResourceSizeTable,
}

const _: () = assert!(offset_of!(ResourceMgr, size_table) == 0x820);

/// Prefix only
#[repr(C)]
pub struct ResourceSizeTable {
    _unk_0: [u8; 0x8],
    pub is_rstc: bool,
    _unk_9: [u8; 0x3f],
    pub rstb_hash_count: u32,
    pub rstb_hash_entries: *const c_void,
    pub rstb_name_count: u32,
    pub rstb_name_entries: *const c_void,
    pub rstc_hash_count: u32,
    pub rstc_hash_entries: *const c_void,
    pub rstc_name_count: u32,
    pub rstc_name_entries: *const c_void,
}

const _: () = {
    assert!(offset_of!(ResourceSizeTable, is_rstc) == 0x8);
    assert!(offset_of!(ResourceSizeTable, rstb_hash_entries) == 0x50);
    assert!(offset_of!(ResourceSizeTable, rstb_name_entries) == 0x60);
    assert!(offset_of!(ResourceSizeTable, rstc_hash_entries) == 0x70);
    assert!(offset_of!(ResourceSizeTable, rstc_name_entries) == 0x80);
};

impl ResourceSizeTable {
    pub fn is_loaded(&self) -> bool {
        !self.rstb_hash_entries.is_null() || !self.rstc_hash_entries.is_null()
    }
}

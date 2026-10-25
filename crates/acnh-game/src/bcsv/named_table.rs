use core::mem::offset_of;

use native_vtable::vtable_impl;

use crate::bcsv::{MemberFn, Table, VftTable};
use crate::sead::{Buffer, PtrArray, SafeString};

#[repr(C)]
pub struct NameIndexEntry {
    pub row_index: u32,
    pub name: SafeString<'static>,
    pub name_hash: u32,
    _unk_1c: [u8; 0x4],
    pub sort_key: u32,
}

const _: () = {
    assert!(offset_of!(NameIndexEntry, sort_key) == 0x20);
    assert!(core::mem::size_of::<NameIndexEntry>() == 0x28);
};

/// Rows indexed by the hash of a name column
#[repr(C)]
pub struct NamedTable {
    pub base: Table,
    pub entries: Buffer<NameIndexEntry>,
    pub sorted: PtrArray<NameIndexEntry>,
    pub sort_enabled: bool,
    pub is_sorted: bool,
    pub get_row_name: MemberFn,
    pub get_row_key: MemberFn,
}

const _: () = {
    assert!(offset_of!(NamedTable, sorted) == 0x1e8);
    assert!(offset_of!(NamedTable, get_row_name) == 0x200);
    assert!(core::mem::size_of::<NamedTable>() == 0x220);
};

crate::derive_base!(NamedTable => Table);

#[vtable_impl]
unsafe impl VftTable for NamedTable {}

impl NamedTable {
    pub unsafe fn entry_by_hash(&self, name_hash: u32) -> Option<&NameIndexEntry> {
        unsafe {
            self.entries
                .as_slice()
                .iter()
                .find(|e| e.name_hash == name_hash)
        }
    }
}

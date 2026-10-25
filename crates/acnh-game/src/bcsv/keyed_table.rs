use core::mem::offset_of;

use native_vtable::vtable_impl;

use crate::bcsv::{MemberFn, Table, VftTable};
use crate::sead::Buffer;

/// Rows indexed by a u16 key column
#[repr(C)]
pub struct KeyedTable {
    pub base: Table,
    pub rows_by_key: Buffer<*mut u8>,
    pub get_row_key: MemberFn,
    pub min_key: u16,
    pub max_key: u16,
    pub key_range_valid: bool,
}

const _: () = {
    assert!(offset_of!(KeyedTable, rows_by_key) == 0x1d8);
    assert!(offset_of!(KeyedTable, min_key) == 0x1f8);
    assert!(core::mem::size_of::<KeyedTable>() == 0x200);
};

crate::derive_base!(KeyedTable => Table);

#[vtable_impl]
unsafe impl VftTable for KeyedTable {}

impl KeyedTable {
    pub unsafe fn row_by_key(&self, key: u16) -> *const u8 {
        if !self.key_range_valid || key < self.min_key || key > self.max_key {
            return core::ptr::null();
        }

        unsafe {
            self.rows_by_key
                .as_slice()
                .get((key - self.min_key) as usize)
                .map_or(core::ptr::null(), |&row| row.cast_const())
        }
    }
}

use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::bcsv::{Bcsv, VftBcsv};
use crate::sead::{Buffer, Delegate, FixedSafeString, Heap, SafeString};

#[vtable(path = bcsv::table::VftTable)]
pub unsafe trait VftTable: VftBcsv {
    unsafe fn clear(&mut self);
    unsafe fn setup_rows(&mut self, name: *const SafeString<'static>, heap: *mut Heap);
}

/// An Itanium pointer to member function
#[repr(C)]
pub struct MemberFn {
    pub ptr: usize,
    pub adj: isize,
}

#[repr(C)]
pub struct Table {
    pub base: Bcsv,
    pub unk_c0: u32,
    pub unk_c4: u8,
    pub row_enabled: Buffer<u8>,
    pub hook_d8: Delegate,
    pub row_filter: Delegate,
    pub name_to_index: Delegate,
    pub unk_name: FixedSafeString<64>,
    pub indexed_count: i32,
    pub get_row_name: MemberFn,
    pub sort_compare: *mut c_void,
    pub ordered_rows: Buffer<*mut u8>,

    /// -1 while the rows come straight from the file
    pub ordered_count: i32,
    pub row_to_ordered: Buffer<u16>,
}

const _: () = {
    assert!(offset_of!(Table, row_enabled) == 0xc8);
    assert!(offset_of!(Table, unk_name) == 0x138);
    assert!(offset_of!(Table, get_row_name) == 0x198);
    assert!(offset_of!(Table, ordered_rows) == 0x1b0);
    assert!(offset_of!(Table, ordered_count) == 0x1c0);
    assert!(core::mem::size_of::<Table>() == 0x1d8);
    assert!(VftTableLayout::SLOT_COUNT == 9);
};

crate::derive_base!(Table => Bcsv);

#[vtable_impl]
unsafe impl VftTable for Table {}

impl Table {
    pub unsafe fn len(&self) -> usize {
        if self.ordered_count > 0 {
            return self.ordered_count as usize;
        }

        unsafe { self.header().map_or(0, |h| h.row_count as usize) }
    }

    pub unsafe fn is_empty(&self) -> bool {
        unsafe { self.len() == 0 }
    }

    /// Null for a disabled or missing row, the same as the game's own lookup
    pub unsafe fn row(&self, index: usize) -> *const u8 {
        unsafe {
            if self.ordered_count > 0 {
                if index >= self.ordered_count as usize {
                    return core::ptr::null();
                }

                return self
                    .ordered_rows
                    .as_slice()
                    .get(index)
                    .map_or(core::ptr::null(), |&row| row.cast_const());
            }

            if self.row_enabled.as_slice().get(index) == Some(&0) {
                return core::ptr::null();
            }

            self.header().map_or(core::ptr::null(), |h| h.row(index))
        }
    }

    pub unsafe fn rows(&self) -> impl Iterator<Item = *const u8> + '_ {
        unsafe {
            (0..self.len())
                .map(|i| self.row(i))
                .filter(|row| !row.is_null())
        }
    }
}

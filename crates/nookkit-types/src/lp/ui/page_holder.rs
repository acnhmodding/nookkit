use core::ffi::c_void;
use core::mem::offset_of;

use crate::game::ui::ViewPage;
use crate::lp::ui::Name;
use crate::sead::{IDisposer, OffsetList};

#[repr(C)]
pub struct PageHolder {
    pub base: IDisposer,
    _unk_20: [u8; 0x10],
    pub name: Name,
    pub heap: *mut c_void,
    pub heap_slots: OffsetList,
    pub current_heap_slot: *mut c_void,
    pub owns_heap: bool,
    _unk_71: [u8; 0x7],
    pub pages: OffsetList,
    pub extra: *mut c_void,
}

const _: () = {
    assert!(offset_of!(PageHolder, name) == 0x30);
    assert!(offset_of!(PageHolder, pages) == 0x78);
    assert!(core::mem::size_of::<PageHolder>() == 0x98);
};

crate::derive_base!(PageHolder => IDisposer);

impl PageHolder {
    pub unsafe fn pages(&self) -> impl Iterator<Item = *mut ViewPage> + '_ {
        unsafe { self.pages.iter() }
    }
}

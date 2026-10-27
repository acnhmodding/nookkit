use core::ffi::c_void;
use core::mem::offset_of;

use crate::game::ui::{SequenceBase, ViewPage};
use crate::native::{Class, Fn, sym};

#[repr(C)]
pub struct StorageSeq {
    pub base: SequenceBase,
    pub page: *mut ViewPage,
    _unk_218: [u8; 0x8],

    /// 0 and 1 are the house's own storage flows
    pub mode: i32,
    _unk_224: [u8; 0xaa6d4],

    /// Their +0xbc byte is an enable flag that Play_Exit clears
    pub category_arrows: [*mut c_void; 2],
    _unk_aa908: [u8; 0xd8],

    /// Bit 12 is X, the accent-wall options
    pub events: u64,
}

const _: () = {
    assert!(offset_of!(StorageSeq, page) == 0x210);
    assert!(offset_of!(StorageSeq, mode) == 0x220);
    assert!(offset_of!(StorageSeq, category_arrows) == 0xaa8f8);
    assert!(offset_of!(StorageSeq, events) == 0xaa9e0);
};

crate::derive_base!(StorageSeq => SequenceBase);

unsafe impl Class for StorageSeq {
    const RUNTIME_TYPE_INFO: Fn = sym::game::UI::StorageSeq::getRuntimeTypeInfo.erase();
}

impl StorageSeq {
    /// Clears the category arrows' enable flags, as Play_Exit does
    pub unsafe fn disable_category_arrows(&mut self) {
        for arrow in self.category_arrows {
            if let Some(enabled) = unsafe { arrow.cast::<u8>().add(0xbc).as_mut() } {
                *enabled = 0;
            }
        }
    }
}

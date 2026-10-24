use core::mem::offset_of;

use crate::game::ui::ViewPage;

#[repr(C)]
pub struct SequenceBase {
    _unk_0: [u8; 0x210],
    pub page: *mut ViewPage,
}

const _: () = assert!(offset_of!(SequenceBase, page) == 0x210);

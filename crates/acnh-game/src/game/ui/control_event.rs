use core::ffi::c_void;
use core::mem::offset_of;

use crate::lp::ui::Control;

pub const CONTROL_EVENT_FOCUS: i32 = 0;
pub const CONTROL_EVENT_DECIDE: i32 = 5;

#[repr(C)]
pub struct ControlEvent {
    _unk_0: [u8; 0x8],
    pub control: *mut Control,
    _unk_10: [u8; 0x4],
    pub kind: i32,
    _unk_18: [u8; 0x8],
    pub index: i32,
    _unk_24: [u8; 0xc],
}

const _: () = {
    assert!(offset_of!(ControlEvent, control) == 0x8);
    assert!(offset_of!(ControlEvent, kind) == 0x14);
    assert!(offset_of!(ControlEvent, index) == 0x20);
    assert!(core::mem::size_of::<ControlEvent>() == 0x30);
};

#[repr(C)]
pub struct LayoutBuiltEvent {
    pub parent: *mut Control,
    pub builder: *mut c_void,
}

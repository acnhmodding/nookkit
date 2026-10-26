use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::native;

use crate::nn::ui2d::TextBox;

#[repr(C)]
pub struct TextPaneHandle {
    pub vtable: *const c_void,
    pub pane: *mut TextBox,
    _unk_10: [u8; 0x8],

    /// The function table the game calls pane operations through, with `pane` as the receiver
    pub pane_ops: *const usize,
}

#[repr(C)]
pub struct TextAnimCtrl {
    _unk_0: [u8; 0x48],
    pub pane: *mut TextPaneHandle,
    _unk_50: [u8; 0x8],

    /// AdjustToText / alignment flags applied by `refresh_layout`
    pub layout_flags: u16,
}

const _: () = {
    assert!(offset_of!(TextAnimCtrl, pane) == 0x48);
    assert!(offset_of!(TextAnimCtrl, layout_flags) == 0x58);
};

#[native(resolve = crate::native::resolve)]
impl TextAnimCtrl {
    /// Re-applies the layout flags after the text changed
    #[native(symbol = "Lp::UI::TextAnimCtrl::refreshLayout")]
    pub unsafe fn refresh_layout(&mut self);
}

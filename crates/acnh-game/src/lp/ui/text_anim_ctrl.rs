use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::native;

use crate::nn::ui2d::TextBox;

#[repr(C)]
pub struct TextPaneHandle {
    pub vtable: *const c_void,
    pub pane: *mut TextBox,
    _unk_10: [u8; 0x8],

    /// The pane's vtable, which the game calls through
    pub pane_vtable: *const usize,
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

impl TextAnimCtrl {
    /// `text` is UTF-16 and must end with a 0
    pub unsafe fn set_text(&mut self, text: &[u16]) -> crate::native::NativeResult {
        unsafe {
            let Some(handle) = self.pane.as_ref() else {
                return Ok(());
            };

            if handle.pane.is_null() || handle.pane_vtable.is_null() || text.last() != Some(&0) {
                return Ok(());
            }

            TextBox::set_string_with(handle.pane, handle.pane_vtable, text.as_ptr());
            self.refresh_layout()?;
        }

        Ok(())
    }
}

use core::mem::offset_of;

const FLAG_VISIBLE: u8 = 1 << 0;

#[repr(C)]
pub struct Pane {
    _unk_0: [u8; 0x58],
    pub flag: u8,
}

const _: () = assert!(offset_of!(Pane, flag) == 0x58);

impl Pane {
    pub fn is_visible(&self) -> bool {
        self.flag & FLAG_VISIBLE != 0
    }

    pub fn set_visible(&mut self, visible: bool) {
        if visible {
            self.flag |= FLAG_VISIBLE;
        } else {
            self.flag &= !FLAG_VISIBLE;
        }
    }
}

const TEXT_BOX_SET_STRING: usize = 0x5e0;

#[repr(C)]
pub struct TextBox {
    pub base: Pane,
}

crate::derive_base!(TextBox => Pane);

impl TextBox {
    /// Calls `SetString(text, 0)` through `vtable`, the one the pane's owner caches
    pub unsafe fn set_string_with(this: *mut Self, vtable: *const usize, text: *const u16) -> u16 {
        unsafe {
            let set_string: unsafe extern "C" fn(*mut Self, *const u16, u16) -> u16 =
                core::mem::transmute(*vtable.add(TEXT_BOX_SET_STRING / 8));

            set_string(this, text, 0)
        }
    }
}

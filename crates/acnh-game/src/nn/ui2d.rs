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

#[repr(C)]
pub struct TextBox {
    pub base: Pane,
}

crate::derive_base!(TextBox => Pane);

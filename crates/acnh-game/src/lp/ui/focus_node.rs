use core::ffi::c_void;
use core::mem::offset_of;

use crate::sead::Delegate;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavDirection {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

#[repr(C)]
pub struct FocusNode {
    _unk_0: [u8; 0x10],
    pub ptr_10: *mut c_void,
    pub focus_root: *mut c_void,
    _unk_20: [u8; 0x20],
    pub focus_delegate: Delegate,
    _unk_60: [u8; 0x1400],

    /// Indexed by [`NavDirection`]
    pub links: [*mut FocusNode; 4],

    pub links_extra: [*mut FocusNode; 4],
    pub saved_14a0: i32,
    pub saved_14a4: i32,
}

const _: () = {
    assert!(offset_of!(FocusNode, focus_delegate) == 0x40);
    assert!(offset_of!(FocusNode, links) == 0x1460);
    assert!(core::mem::size_of::<FocusNode>() == 0x14a8);
};

impl FocusNode {
    pub fn link(&self, direction: NavDirection) -> *mut FocusNode {
        self.links[direction as usize]
    }

    pub fn set_link(&mut self, direction: NavDirection, target: *mut FocusNode) {
        self.links[direction as usize] = target;
    }
}

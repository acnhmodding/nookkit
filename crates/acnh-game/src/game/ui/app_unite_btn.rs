use core::mem::offset_of;

use crate::game::ui::ListBtnGroup;
use crate::lp::ui::{Control, FocusNode, NavDirection, VftControlLayout};
use crate::native::{NativeResult, fun_addr, vtable};
use crate::sead::Delegate;

#[repr(C)]
pub struct ChildList {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct AppUniteBtnCtrl {
    pub vtable: *const VftControlLayout,
    _unk_8: [u8; 0x30],
    pub children: ChildList,
    _unk_38: [u8; 0x1e0],
    pub focus_node: *mut FocusNode,
    _unk_220: [u8; 0x90],
    pub list_mode: i32,
    _unk_2b4: [u8; 0xc],
    pub mode_bits: u8,
    _unk_2c1: [u8; 0xdf],
    pub selectable_delegate: Delegate,
}

const _: () = {
    assert!(offset_of!(AppUniteBtnCtrl, children) == 0x38);
    assert!(offset_of!(AppUniteBtnCtrl, focus_node) == 0x218);
    assert!(offset_of!(AppUniteBtnCtrl, list_mode) == 0x2b0);
    assert!(offset_of!(AppUniteBtnCtrl, mode_bits) == 0x2c0);
    assert!(offset_of!(AppUniteBtnCtrl, selectable_delegate) == 0x3a0);
};

impl AppUniteBtnCtrl {
    pub fn as_control(&mut self) -> *mut Control {
        (self as *mut Self).cast()
    }

    pub fn set_list_mode(&mut self) {
        self.mode_bits = (self.mode_bits & 0xf8) | 1;
        self.list_mode = 2;
    }

    pub fn bind_to_group(&mut self, group: *mut ListBtnGroup) -> NativeResult {
        self.selectable_delegate = Delegate::new(
            vtable("game::UI::ListBtnGroup::SelectableDelegate::vtable")?,
            group,
            fun_addr("game::UI::ListBtnGroup::isSelectable")?,
        );

        Ok(())
    }

    pub unsafe fn set_link(&mut self, direction: NavDirection, target: *mut AppUniteBtnCtrl) {
        unsafe {
            let (Some(node), Some(target)) = (self.focus_node.as_mut(), target.as_ref()) else {
                return;
            };

            if !target.focus_node.is_null() {
                node.set_link(direction, target.focus_node);
            }
        }
    }
}

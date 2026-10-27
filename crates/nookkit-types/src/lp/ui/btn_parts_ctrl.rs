use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::lp::ui::{FocusNode, KeyHandler, NavDirection, PartsCtrl, VftPartsCtrl};
use crate::native::{Class, Fn, sym};

#[vtable(path = lp::ui::btn_parts_ctrl::VftBtnPartsCtrl)]
pub unsafe trait VftBtnPartsCtrl: VftPartsCtrl {
    unsafe fn is_select(&self) -> bool;
    unsafe fn is_decide(&self) -> bool;
    unsafe fn is_decide_trig(&self) -> bool;
    unsafe fn is_end_decide(&self, frames: u32) -> bool;
    unsafe fn force_decide(&mut self) -> bool;
    unsafe fn cancel_decide(&mut self) -> bool;
    unsafe fn on_cursor_handler(&mut self, kind: i32, event: *mut c_void);
    unsafe fn on_cursor_key_handler(&mut self, kind: i32, event: *mut c_void);
    unsafe fn on_point_handler(&mut self, kind: i32, event: *mut c_void);
    unsafe fn on_key_handler(&mut self, kind: i32, event: *mut *mut c_void);
}

#[repr(C)]
pub struct BtnPartsCtrl {
    pub base: PartsCtrl,
    pub pointer_handler_count: u32,
    pub key_handler_count: u32,
    _unk_204: [u8; 0x4],
    pub ptr_208: *mut c_void,
    pub ptr_210: *mut c_void,
    pub focus_node: *mut FocusNode,

    /// Pointer handlers have their player at +0x20 and enable flag at +0x3a
    pub pointer_handler_capacity: u32,
    _unk_224: [u8; 0x4],
    pub pointer_handlers: *mut *mut c_void,
    pub key_handler_capacity: u32,
    _unk_234: [u8; 0x4],
    pub key_handlers: *mut *mut KeyHandler,
    pub ptr_240: *mut c_void,
    pub ptr_248: *mut c_void,
    pub handler_250: *mut c_void,
    pub handler_258: *mut c_void,
    pub ptrs_260: [*mut c_void; 4],
}

const _: () = {
    assert!(offset_of!(BtnPartsCtrl, pointer_handler_count) == 0x1fc);
    assert!(offset_of!(BtnPartsCtrl, pointer_handlers) == 0x228);
    assert!(offset_of!(BtnPartsCtrl, key_handlers) == 0x238);
    assert!(offset_of!(BtnPartsCtrl, focus_node) == 0x218);
    assert!(offset_of!(BtnPartsCtrl, ptrs_260) == 0x260);
    assert!(core::mem::size_of::<BtnPartsCtrl>() == 0x280);
    assert!(VftBtnPartsCtrlLayout::SLOT_COUNT == 40);
};

crate::derive_base!(BtnPartsCtrl => PartsCtrl);

#[vtable_impl]
unsafe impl VftBtnPartsCtrl for BtnPartsCtrl {}

unsafe impl Class for BtnPartsCtrl {
    const RUNTIME_TYPE_INFO: Fn = sym::Lp::UI::BtnPartsCtrl::getRuntimeTypeInfo.erase();
}

impl BtnPartsCtrl {
    pub unsafe fn focus_node(&mut self) -> Option<&mut FocusNode> {
        unsafe { self.focus_node.as_mut() }
    }

    /// Moving `direction` from this button focuses `target`, or nothing when `None`
    pub unsafe fn set_link(&mut self, direction: NavDirection, target: Option<&BtnPartsCtrl>) {
        let target = target.map_or(core::ptr::null_mut(), |t| t.focus_node);

        if let Some(node) = unsafe { self.focus_node() } {
            node.set_link(direction, target);
        }
    }
}

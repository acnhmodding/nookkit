use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::lp::ui::{BtnPartsCtrl, VftBtnPartsCtrl};
use crate::native::{Class, Fn, sym};

#[vtable(path = lp::ui::unite_btn_ctrl::VftUniteBtnCtrl)]
pub unsafe trait VftUniteBtnCtrl: VftBtnPartsCtrl {
    unsafe fn set_disp_valid(&mut self, valid: bool, immediate: bool);
    unsafe fn is_disp_invalid(&self) -> bool;
    unsafe fn on_input_event(&mut self, event: *mut c_void);
}

#[repr(C)]
pub struct UniteBtnCtrl {
    pub base: BtnPartsCtrl,
    pub disp_valid_anim: *mut c_void,
    pub select_tag: *mut c_void,
    pub unselect_tag: *mut c_void,
    pub decide_tag: *mut c_void,
    _unk_2a0: [u8; 0x10],

    /// 2 inside a scroll list
    pub list_mode: i32,

    pub unk_2b4: u32,
    _unk_2b8: [u8; 0x8],
    pub mode_bits: u8,
    _unk_2c1: [u8; 0x2],
    pub disp_valid: u8,
    _unk_2c4: [u8; 0x4],
}

const _: () = {
    assert!(offset_of!(UniteBtnCtrl, disp_valid_anim) == 0x280);
    assert!(offset_of!(UniteBtnCtrl, list_mode) == 0x2b0);
    assert!(offset_of!(UniteBtnCtrl, mode_bits) == 0x2c0);
    assert!(offset_of!(UniteBtnCtrl, disp_valid) == 0x2c3);
    assert!(core::mem::size_of::<UniteBtnCtrl>() == 0x2c8);
    assert!(VftUniteBtnCtrlLayout::SLOT_COUNT == 43);
};

crate::derive_base!(UniteBtnCtrl => BtnPartsCtrl);

#[vtable_impl]
unsafe impl VftUniteBtnCtrl for UniteBtnCtrl {}

unsafe impl Class for UniteBtnCtrl {
    const RUNTIME_TYPE_INFO: Fn = sym::Lp::UI::UniteBtnCtrl::getRuntimeTypeInfo.erase();
}

impl UniteBtnCtrl {
    pub fn set_list_mode(&mut self) {
        self.mode_bits = (self.mode_bits & 0xf8) | 1;
        self.list_mode = 2;
    }
}

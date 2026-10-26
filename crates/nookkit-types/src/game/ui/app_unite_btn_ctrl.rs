use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::vtable_impl;

use crate::game::ui::ListBtnGroup;
use crate::lp::ui::{UniteBtnCtrl, VftUniteBtnCtrl};
use crate::native::{Class, NativeResult, fun_addr, vtable};
use crate::sead::Delegate;

#[repr(C)]
pub struct AppUniteBtnCtrl {
    pub base: UniteBtnCtrl,
    pub unk_2c8: u32,
    _unk_2cc: [u8; 0x4],
    pub ptr_2d0: *mut c_void,
    pub unk_2d8: u32,
    _unk_2dc: [u8; 0x4],
    pub ptr_2e0: *mut c_void,
    pub unk_2e8: u32,
    _unk_2ec: [u8; 0x4],
    pub ptr_2f0: *mut c_void,
    pub unk_2f8: u32,
    _unk_2fc: [u8; 0x4],
    pub ptr_300: *mut c_void,
    pub ptr_308: *mut c_void,
    _unk_310: [u8; 0x4],
    pub skip_next_anim: bool,
    pub force_invalid: bool,
    pub flag_316: u8,
    _unk_317: u8,
    pub ptr_318: *mut c_void,
    pub unk_320: u32,
    _unk_324: [u8; 0x4],
    pub ptr_328: *mut c_void,
    pub unk_330: u32,
    _unk_334: [u8; 0x4],
    pub ptr_338: *mut c_void,
    pub unk_340: u32,
    _unk_344: [u8; 0x4],
    pub ptr_348: *mut c_void,
    pub unk_350: u32,
    _unk_354: [u8; 0x4],
    pub ptr_358: *mut c_void,
    pub index_360: i32,
    _unk_364: [u8; 0x4],
    pub delegate_368: Delegate,
    pub check_state_anim: *mut c_void,
    pub ptr_390: *mut c_void,
    pub anim_398: *mut c_void,

    /// Asks the button's group whether it can be selected
    pub group_delegate: Delegate,

    pub sub_3c0_vtable: *const c_void,
    _unk_3c8: [u8; 0x18],
    pub ptr_3e0: *mut c_void,
    pub ptr_3e8: *mut c_void,
    pub ptr_3f0: *mut c_void,
    pub sub_3f8_vtable: *const c_void,
    pub ptr_400: *mut c_void,
}

const _: () = {
    assert!(offset_of!(AppUniteBtnCtrl, unk_2c8) == 0x2c8);
    assert!(offset_of!(AppUniteBtnCtrl, skip_next_anim) == 0x314);
    assert!(offset_of!(AppUniteBtnCtrl, delegate_368) == 0x368);
    assert!(offset_of!(AppUniteBtnCtrl, group_delegate) == 0x3a0);
    assert!(offset_of!(AppUniteBtnCtrl, sub_3c0_vtable) == 0x3c0);
    assert!(offset_of!(AppUniteBtnCtrl, ptr_400) == 0x400);
    assert!(core::mem::size_of::<AppUniteBtnCtrl>() == 0x408);
};

crate::derive_base!(AppUniteBtnCtrl => UniteBtnCtrl);

#[vtable_impl]
unsafe impl VftUniteBtnCtrl for AppUniteBtnCtrl {}

unsafe impl Class for AppUniteBtnCtrl {
    const RUNTIME_TYPE_INFO: &'static str = "game::UI::AppUniteBtnCtrl::getRuntimeTypeInfo";
}

impl AppUniteBtnCtrl {
    pub fn bind_to_group(&mut self, group: *mut ListBtnGroup) -> NativeResult {
        self.group_delegate = Delegate::new(
            vtable("game::UI::ListBtnGroup::SelectableDelegate::vtable")?,
            group,
            fun_addr("game::UI::ListBtnGroup::isSelectable")?,
        );

        Ok(())
    }
}

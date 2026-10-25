use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::vtable_impl;

use crate::lp::ui::{PartsCtrl, TextAnimCtrl, VftPartsCtrl};
use crate::native::Class;
use crate::sead::PtrArray;

#[repr(C)]
pub struct AppLocatorCtrl {
    pub base: PartsCtrl,
    pub construct_arg: *mut c_void,
    pub unk_208: u32,
    _unk_20c: [u8; 0x4],
    pub ptr_210: *mut c_void,
    pub texts: PtrArray<TextAnimCtrl>,
    pub anims: PtrArray<c_void>,
    pub unk_238: u32,
    _unk_23c: [u8; 0x4],
    pub ptr_240: *mut c_void,
    pub ptr_248: *mut c_void,
    pub unk_250: u32,
    pub unk_254: u32,
    pub sub_258: [u8; 0x30],
}

const _: () = {
    assert!(offset_of!(AppLocatorCtrl, construct_arg) == 0x200);
    assert!(offset_of!(AppLocatorCtrl, texts) == 0x218);
    assert!(offset_of!(AppLocatorCtrl, anims) == 0x228);
    assert!(offset_of!(AppLocatorCtrl, sub_258) == 0x258);
    assert!(core::mem::size_of::<AppLocatorCtrl>() == 0x288);
};

crate::derive_base!(AppLocatorCtrl => PartsCtrl);

#[vtable_impl]
unsafe impl VftPartsCtrl for AppLocatorCtrl {}

unsafe impl Class for AppLocatorCtrl {
    const RUNTIME_TYPE_INFO: &'static str = "game::UI::AppLocatorCtrl::getRuntimeTypeInfo";
}

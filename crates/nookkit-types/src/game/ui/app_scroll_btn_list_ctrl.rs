use native_vtable::vtable_impl;

use crate::game::ui_sys::{ScrollBtnListCtrl, VftScrollBtnListCtrl};
use crate::native::{Class, Fn, sym};

#[repr(C)]
pub struct AppScrollBtnListCtrl {
    pub base: ScrollBtnListCtrl,
    _unk_4f8: [u8; 0xa8],
}

const _: () = assert!(core::mem::size_of::<AppScrollBtnListCtrl>() == 0x5a0);

crate::derive_base!(AppScrollBtnListCtrl => ScrollBtnListCtrl);

#[vtable_impl]
unsafe impl VftScrollBtnListCtrl for AppScrollBtnListCtrl {}

unsafe impl Class for AppScrollBtnListCtrl {
    const RUNTIME_TYPE_INFO: Fn = sym::game::UI::AppScrollBtnListCtrl::getRuntimeTypeInfo.erase();
}

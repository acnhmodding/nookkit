use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::game::ui::{VftViewPage, ViewPage};
use crate::native::Class;

#[vtable(path = game::ui::page_base::VftPageBase)]
pub unsafe trait VftPageBase: VftViewPage {
    unsafe fn play_layout_in_with(&mut self, index: u32, mode: i32);
    unsafe fn play_layout_out_with(&mut self, index: u32, mode: i32);
    unsafe fn vf_260_retnull(&mut self) -> *mut c_void;
    unsafe fn vf_268(&mut self);
    unsafe fn vf_270_retnull(&mut self) -> *mut c_void;
}

#[repr(C)]
pub struct PageBase {
    pub base: ViewPage,
    pub flag_568: u8,
    pub post_animate_root: u8,
    pub pre_wake_hook: u8,
    _unk_56b: [u8; 0x5],
}

const _: () = {
    assert!(offset_of!(PageBase, flag_568) == 0x568);
    assert!(core::mem::size_of::<PageBase>() == 0x570);
    assert!(VftPageBaseLayout::SLOT_COUNT == 79);
};

crate::derive_base!(PageBase => ViewPage);

#[vtable_impl]
unsafe impl VftPageBase for PageBase {}

unsafe impl Class for PageBase {
    const RUNTIME_TYPE_INFO: &'static str = "game::UI::PageBase::getRuntimeTypeInfo";
}

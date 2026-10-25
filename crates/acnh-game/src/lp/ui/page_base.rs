use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::lp::ui::{Job, Name};
use crate::sead::runtime_type_info::Interface;
use crate::sead::{Delegate, OffsetList, ReadWriteLock, SafeString};

#[vtable(path = lp::ui::page_base::VftPageBase)]
pub unsafe trait VftPageBase {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const Interface) -> bool;
    unsafe fn get_runtime_type_info(&self) -> *const Interface;

    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);

    unsafe fn get_class_info(&self) -> *const SafeString<'static>;
    unsafe fn end_initialize_impl(&mut self);
    unsafe fn finalize_impl(&mut self);
    unsafe fn wake_impl(&mut self);
    unsafe fn sleep_impl(&mut self);
    unsafe fn prior_manipulate_impl(&mut self, input: *mut c_void, flag: bool);
    unsafe fn manipulate_impl(&mut self, input: *mut c_void, flag: bool);
    unsafe fn pre_calc_impl(&mut self);
    unsafe fn post_calc_impl(&mut self);
    unsafe fn update_binding_impl(&mut self);
    unsafe fn chain_post_calc_impl(&mut self, job: *mut c_void, flag: bool);
    unsafe fn draw_layout_impl(&mut self, draw_info: *mut c_void);
    unsafe fn on_pre_wake(&mut self);
    unsafe fn on_post_wake(&mut self);
    unsafe fn on_pre_sleep(&mut self);
    unsafe fn on_post_sleep(&mut self);
    unsafe fn on_pre_calc(&mut self);
    unsafe fn on_post_calc(&mut self);
}

#[repr(C)]
pub struct PageBase {
    pub vtable: *const VftPageBaseLayout,
    pub ptr_08: *mut c_void,
    pub ptr_10: *mut c_void,
    pub ptr_18: *mut c_void,
    pub ptr_20: *mut c_void,
    pub name: Name,
    pub construct_arg_10: *mut c_void,
    pub state: i32,
    pub ui_mgr_value: u32,
    pub flag_50: u8,
    _unk_51: [u8; 0x3],
    pub index_54: i32,
    pub ptr_58: *mut c_void,
    pub list_60: OffsetList,
    pub post_calc_job: Job,
    pub post_calc_delegate: Delegate,
    pub lock: ReadWriteLock,
}

const _: () = {
    assert!(offset_of!(PageBase, name) == 0x28);
    assert!(offset_of!(PageBase, post_calc_job) == 0x78);
    assert!(offset_of!(PageBase, lock) == 0x110);
    assert!(core::mem::size_of::<PageBase>() == 0x1c8);
    assert!(VftPageBaseLayout::SLOT_COUNT == 22);
};

#[vtable_impl]
unsafe impl VftPageBase for PageBase {}

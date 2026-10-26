use core::ffi::c_void;

use crate::sead::OffsetList;

#[repr(C)]
pub struct Job {
    pub vtable: *const c_void,
    pub ptr_08: *mut c_void,
    pub ptr_10: *mut c_void,
    pub list_18: OffsetList,
    pub list_30: OffsetList,
    pub list_48: OffsetList,
    pub list_60: OffsetList,
}

const _: () = assert!(core::mem::size_of::<Job>() == 0x78);

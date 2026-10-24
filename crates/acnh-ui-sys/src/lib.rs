#![no_std]

use core::ffi::{c_char, c_void};

pub const API_NAME: &core::ffi::CStr = c"acnh-ui";
pub const API_VERSION: u32 = 1;

// Exported by the skyline-acnh exefs (ApiRegistry.cpp); stock skyline lacks them.
unsafe extern "C" {
    pub fn sl_api_publish(name: *const c_char, version: u32, vtable: *const c_void);
    pub fn sl_api_get(name: *const c_char, version: u32) -> *const c_void;
}

#[repr(C)]
pub struct AcnhUiV1 {
    /// Lets a client built against a newer table detect entries this framework lacks.
    pub size: usize,
    pub framework_version: u32,
}

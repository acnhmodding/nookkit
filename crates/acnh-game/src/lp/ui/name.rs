use core::ffi::CStr;

use crate::sead::SafeString;

#[repr(C)]
pub struct Name {
    pub string: SafeString<'static>,
    pub hash: u32,
    pub length: u16,
    pub is_owned: bool,
    _pad_17: u8,
}

const _: () = assert!(core::mem::size_of::<Name>() == 0x18);

impl Name {
    pub unsafe fn as_c_str(&self) -> Option<&CStr> {
        unsafe { self.string.as_c_str() }
    }

    pub unsafe fn as_str(&self) -> Option<&str> {
        unsafe { self.string.as_str() }
    }
}

use core::ffi::{CStr, c_char, c_void};
use core::marker::PhantomData;

use crate::native::{NativeResult, vtable};

#[repr(C)]
pub struct SafeString<'a> {
    pub vtable: *const c_void,
    pub string_top: *const c_char,
    _string: PhantomData<&'a CStr>,
}

const _: () = assert!(core::mem::size_of::<SafeString>() == 0x10);

impl<'a> SafeString<'a> {
    pub fn new(string: &'a CStr) -> NativeResult<Self> {
        Ok(Self {
            vtable: vtable("sead::SafeStringBase<char>::vtable")?,
            string_top: string.as_ptr(),
            _string: PhantomData,
        })
    }

    pub unsafe fn as_c_str(&self) -> Option<&'a CStr> {
        unsafe { (!self.string_top.is_null()).then(|| CStr::from_ptr(self.string_top)) }
    }

    pub unsafe fn as_str(&self) -> Option<&'a str> {
        unsafe { self.as_c_str()?.to_str().ok() }
    }
}

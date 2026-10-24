use core::ffi::{CStr, c_char};
use core::marker::PhantomData;

use crate::native::{NativeResult, vtable};

#[repr(C)]
pub struct SafeString<'a> {
    pub vtable: *const u8,
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
}

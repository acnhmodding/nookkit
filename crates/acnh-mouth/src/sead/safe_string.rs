use core::ffi::{CStr, c_char};
use core::marker::PhantomData;

use crate::native::got_vtable;

const GOT_SAFE_STRING_VTABLE: usize = 0x4ab2020;

#[repr(C)]
pub struct SafeString<'a> {
    pub vtable: *const u8,
    pub string_top: *const c_char,
    _string: PhantomData<&'a CStr>,
}

const _: () = assert!(core::mem::size_of::<SafeString>() == 0x10);

impl<'a> SafeString<'a> {
    pub fn new(string: &'a CStr) -> Self {
        Self {
            vtable: unsafe { got_vtable(GOT_SAFE_STRING_VTABLE) },
            string_top: string.as_ptr(),
            _string: PhantomData,
        }
    }
}

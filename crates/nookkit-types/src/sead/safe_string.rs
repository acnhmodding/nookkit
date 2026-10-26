use core::ffi::{CStr, c_char, c_void};
use core::marker::PhantomData;

use crate::native::{NativeResult, Vtable, sym, vtable};

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
            vtable: vtable(&sym::sead::SafeStringBase_char::vtable)?,
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

#[repr(C)]
pub struct FixedSafeString<const N: usize> {
    pub vtable: *const c_void,
    pub string_top: *const c_char,
    pub buffer_size: i32,
    pub buffer: [u8; N],
}

/// Sizes whose vtable is in the symbol file
pub trait FixedSafeStringSize {
    const VTABLE: Vtable;
}

impl FixedSafeStringSize for FixedSafeString<64> {
    const VTABLE: Vtable = sym::sead::FixedSafeString_64::vtable;
}

impl<const N: usize> FixedSafeString<N> {
    /// Truncates to `N - 1` bytes
    pub fn new(string: &str) -> NativeResult<Self>
    where
        Self: FixedSafeStringSize,
    {
        let mut fixed = Self {
            vtable: vtable(&Self::VTABLE)?,
            string_top: core::ptr::null(),
            buffer_size: N as i32,
            buffer: [0; N],
        };

        let len = string.len().min(N - 1);
        fixed.buffer[..len].copy_from_slice(&string.as_bytes()[..len]);

        Ok(fixed)
    }

    /// `string_top` points into the buffer, so call this once the string is at its final address
    pub fn pin(&mut self) {
        self.string_top = self.buffer.as_ptr().cast();
    }

    pub fn as_str(&self) -> Option<&str> {
        let len = self.buffer.iter().position(|&b| b == 0)?;

        core::str::from_utf8(&self.buffer[..len]).ok()
    }
}

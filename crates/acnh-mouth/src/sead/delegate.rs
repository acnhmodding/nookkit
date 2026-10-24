use core::ffi::c_void;

use native_vtable::{vtable, vtable_impl};

#[vtable(path = sead::delegate::VftDelegate)]
pub unsafe trait VftDelegate {
    #[vtable(offset = 0x10)]
    unsafe fn is_bound(&self) -> bool;
}

const _: () = assert!(VftDelegateLayout::OFFSET_IS_BOUND == 0x10);

#[repr(C)]
pub struct Delegate {
    pub vtable: *const VftDelegateLayout,
    pub object: *mut c_void,
    pub method: usize,
    pub method_adj: usize,
}

const _: () = assert!(core::mem::size_of::<Delegate>() == 0x20);

#[vtable_impl]
unsafe impl VftDelegate for Delegate {}

impl Delegate {
    pub fn new<T>(vtable: *const VftDelegateLayout, object: *mut T, method: usize) -> Self {
        Self {
            vtable,
            object: object.cast(),
            method,
            method_adj: 0,
        }
    }
}

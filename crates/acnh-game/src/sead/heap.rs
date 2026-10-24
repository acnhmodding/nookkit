use core::ffi::c_void;

use native_vtable::{vtable, vtable_impl};

#[vtable(path = sead::heap::VftHeap)]
pub unsafe trait VftHeap {
    #[vtable(offset = 0x30)]
    unsafe fn try_alloc(&mut self, size: usize, alignment: i32) -> *mut c_void;
}

const _: () = assert!(VftHeapLayout::OFFSET_TRY_ALLOC == 0x30);

#[repr(C)]
pub struct Heap {
    pub vtable: *const VftHeapLayout,
}

#[vtable_impl]
unsafe impl VftHeap for Heap {}

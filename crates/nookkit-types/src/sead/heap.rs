use core::ffi::c_void;

use native_vtable::{vtable, vtable_impl};

use crate::sead::runtime_type_info::Interface;
use crate::sead::{IDisposer, ListNode, OffsetList, SafeString};

#[vtable(path = sead::heap::VftHeap)]
pub unsafe trait VftHeap {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const Interface) -> bool;
    unsafe fn get_runtime_type_info(&self) -> *const Interface;

    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);

    unsafe fn destroy(&mut self);
    unsafe fn adjust(&mut self) -> usize;
    unsafe fn try_alloc(&mut self, size: usize, alignment: i32) -> *mut c_void;
    unsafe fn free(&mut self, ptr: *mut c_void);
    unsafe fn resize_front(&mut self, ptr: *mut c_void, size: usize) -> *mut c_void;
    unsafe fn resize_back(&mut self, ptr: *mut c_void, size: usize) -> *mut c_void;
    unsafe fn try_realloc(&mut self, ptr: *mut c_void, size: usize, alignment: i32) -> *mut c_void;
    unsafe fn free_all(&mut self);
    unsafe fn get_start_address(&self) -> usize;
    unsafe fn get_end_address(&self) -> usize;
    unsafe fn get_size(&self) -> usize;
    unsafe fn get_free_size(&self) -> usize;
    unsafe fn get_max_allocatable_size(&self, alignment: i32) -> usize;
    unsafe fn is_include(&self, ptr: *const c_void) -> bool;
    unsafe fn is_empty(&self) -> bool;
    unsafe fn is_freeable(&self) -> bool;
    unsafe fn is_resizable(&self) -> bool;
    unsafe fn is_adjustable(&self) -> bool;
    unsafe fn dump(&self);
    unsafe fn dump_yaml(&self, stream: *mut c_void, indent: i32);
    unsafe fn gen_information(&mut self, context: *mut c_void);
    unsafe fn make_meta_string(&mut self, string: *mut c_void);
    unsafe fn push_back_child(&mut self, child: *mut Heap);
}

const _: () = {
    assert!(VftHeapLayout::OFFSET_TRY_ALLOC == 0x30);
    assert!(VftHeapLayout::OFFSET_PUSH_BACK_CHILD == 0xd0);
};

#[repr(C)]
pub struct Heap {
    pub base: IDisposer,
    pub name: SafeString<'static>,
    pub start: *mut c_void,
    pub size: usize,
    pub parent: *mut Heap,
    pub children: OffsetList,
    pub list_node: ListNode,
}

const _: () = {
    assert!(core::mem::offset_of!(Heap, name) == 0x20);
    assert!(core::mem::offset_of!(Heap, size) == 0x38);
    assert!(core::mem::offset_of!(Heap, children) == 0x48);
    assert!(core::mem::offset_of!(Heap, list_node) == 0x60);
};

#[vtable_impl]
unsafe impl VftHeap for Heap {}

impl Heap {
    pub unsafe fn children(&self) -> impl Iterator<Item = *mut Heap> + '_ {
        unsafe { self.children.iter() }
    }

    pub unsafe fn alloc<T>(&mut self) -> *mut T {
        unsafe {
            self.try_alloc(core::mem::size_of::<T>(), core::mem::align_of::<T>() as i32)
                .cast()
        }
    }
}

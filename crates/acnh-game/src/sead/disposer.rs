use native_vtable::{vtable, vtable_impl};

use crate::sead::{Heap, ListNode};

#[vtable(path = sead::disposer::VftIDisposer)]
pub unsafe trait VftIDisposer {
    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);
}

#[repr(C)]
pub struct IDisposer {
    pub vtable: *const VftIDisposerLayout,
    pub disposer_heap: *mut Heap,
    pub list_node: ListNode,
}

const _: () = assert!(core::mem::size_of::<IDisposer>() == 0x20);

#[vtable_impl]
unsafe impl VftIDisposer for IDisposer {}

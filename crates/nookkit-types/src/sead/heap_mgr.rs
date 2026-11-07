use crate::native::{NativeResult, data, sym};
use crate::sead::Heap;

/// Only reached through its functions, its layout isn't needed
#[repr(C)]
pub struct HeapMgr {
    _opaque: [u8; 0],
}

crate::native! {
    impl HeapMgr {
        /// The heap whose memory holds `ptr`, null when none does
        pub unsafe fn find_contain_heap(&mut self, ptr: *const u8) -> *mut Heap
            = sym::sead::HeapMgr::findContainHeap;
    }
}

impl HeapMgr {
    /// Null until sead creates it
    pub fn instance() -> NativeResult<*mut HeapMgr> {
        Ok(unsafe { *data::<*mut HeapMgr>(&sym::sead::HeapMgr::sInstance)? })
    }
}

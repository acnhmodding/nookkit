#[repr(C)]
pub struct PtrArray<T> {
    pub size: i32,
    pub capacity: i32,
    pub ptrs: *mut *mut T,
}

#[repr(C)]
pub struct ObjArray<T> {
    pub base: PtrArray<T>,
    pub free_list: *mut u64,
}

const _: () = assert!(core::mem::size_of::<ObjArray<u32>>() == 0x18);

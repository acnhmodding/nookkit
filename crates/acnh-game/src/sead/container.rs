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

#[repr(C)]
pub struct ListNode {
    pub prev: *mut ListNode,
    pub next: *mut ListNode,
}

#[repr(C)]
pub struct OffsetList {
    pub start_end: ListNode,
    pub count: i32,
    pub offset: i32,
}

const _: () = assert!(core::mem::size_of::<OffsetList>() == 0x18);

impl OffsetList {
    pub unsafe fn iter<T>(&self) -> impl Iterator<Item = *mut T> + '_ {
        let end = &self.start_end as *const ListNode;
        let mut node = self.start_end.next.cast_const();

        core::iter::from_fn(move || {
            if node.is_null() || node == end {
                return None;
            }

            let item = unsafe { node.cast::<u8>().sub(self.offset as usize) };
            node = unsafe { (*node).next };

            Some(item.cast_mut().cast())
        })
    }
}

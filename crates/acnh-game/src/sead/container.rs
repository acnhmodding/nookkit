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

impl<T> PtrArray<T> {
    pub fn len(&self) -> usize {
        self.size.max(0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub unsafe fn get(&self, index: usize) -> *mut T {
        if index >= self.len() || self.ptrs.is_null() {
            return core::ptr::null_mut();
        }

        unsafe { *self.ptrs.add(index) }
    }

    pub unsafe fn iter(&self) -> impl Iterator<Item = *mut T> + '_ {
        (0..self.len()).map(|i| unsafe { self.get(i) })
    }
}

impl<T> core::ops::Deref for ObjArray<T> {
    type Target = PtrArray<T>;

    fn deref(&self) -> &PtrArray<T> {
        &self.base
    }
}

impl<T> core::ops::DerefMut for ObjArray<T> {
    fn deref_mut(&mut self) -> &mut PtrArray<T> {
        &mut self.base
    }
}

#[repr(C)]
pub struct ListNode {
    pub prev: *mut ListNode,
    pub next: *mut ListNode,
}

#[repr(C)]
pub struct OffsetList {
    pub start_end: ListNode,
    pub count: i32,

    /// Of the node inside each item
    pub offset: i32,
}

const _: () = assert!(core::mem::size_of::<OffsetList>() == 0x18);

impl OffsetList {
    pub fn len(&self) -> usize {
        self.count.max(0) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

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

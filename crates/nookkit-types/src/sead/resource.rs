use native_vtable::{vtable, vtable_impl};

use crate::sead::Heap;
use crate::sead::runtime_type_info::Interface;

#[vtable(path = sead::resource::VftResource)]
pub unsafe trait VftResource {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const Interface) -> bool;
    unsafe fn get_runtime_type_info(&self) -> *const Interface;

    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);
}

#[vtable(path = sead::resource::VftDirectResource)]
pub unsafe trait VftDirectResource: VftResource {
    unsafe fn get_load_data_alignment(&self) -> i32;
    unsafe fn do_create(&mut self, data: *mut u8, size: u32, heap: *mut Heap);
}

#[repr(C)]
pub struct Resource {
    pub vtable: *const VftResourceLayout,
}

#[vtable_impl]
unsafe impl VftResource for Resource {}

#[repr(C)]
pub struct DirectResource {
    pub base: Resource,
    pub raw_data: *mut u8,
    pub raw_size: u32,
    pub buffer_size: u32,

    /// [`DirectResource::OWNS_DATA`] and others
    pub setting_flag: u32,
}

const _: () = assert!(core::mem::size_of::<DirectResource>() == 0x20);

crate::derive_base!(DirectResource => Resource);

#[vtable_impl]
unsafe impl VftDirectResource for DirectResource {}

impl DirectResource {
    /// Set when the destructor frees `raw_data`, from the heap that holds it
    pub const OWNS_DATA: u32 = 1 << 0;

    pub fn owns_data(&self) -> bool {
        self.setting_flag & Self::OWNS_DATA != 0
    }

    /// Hands over `size` bytes at `data` for the destructor to free
    pub fn set_owned_data(&mut self, data: *mut u8, size: u32) {
        self.raw_data = data;
        self.raw_size = size;
        self.buffer_size = size;
        self.setting_flag |= Self::OWNS_DATA;
    }

    pub unsafe fn data(&self) -> &[u8] {
        if self.raw_data.is_null() {
            return &[];
        }

        unsafe { core::slice::from_raw_parts(self.raw_data, self.raw_size as usize) }
    }
}

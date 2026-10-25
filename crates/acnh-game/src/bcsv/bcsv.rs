use core::ffi::c_char;
use core::mem::offset_of;

use crate::bcsv::FileHeader;
use crate::sead::{
    DirectResource, FixedSafeString, Heap, SafeString, runtime_type_info::Interface,
};
use native_vtable::{vtable, vtable_impl};

#[vtable(path = bcsv::bcsv::VftBcsv)]
pub unsafe trait VftBcsv {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const Interface) -> bool;
    unsafe fn get_runtime_type_info(&self) -> *const Interface;

    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);

    /// Shares the rows of an already loaded table with the same name
    unsafe fn load(&mut self, name: *const SafeString<'static>, heap: *mut Heap) -> bool;
    unsafe fn reload(&mut self) -> bool;
    unsafe fn get_file_name(&self) -> *const c_char;
}

#[repr(C)]
pub struct Bcsv {
    pub vtable: *const VftBcsvLayout,
    pub name: FixedSafeString<128>,
    pub name_hash: u32,
    pub heap: *mut Heap,
    pub resource: *mut DirectResource,

    /// The table whose rows this one shares, if any
    pub source_param: *mut Bcsv,
}

const _: () = {
    assert!(offset_of!(Bcsv, resource) == 0xb0);
    assert!(core::mem::size_of::<Bcsv>() == 0xc0);
};

#[vtable_impl]
unsafe impl VftBcsv for Bcsv {}

impl Bcsv {
    pub unsafe fn header(&self) -> Option<&FileHeader> {
        unsafe {
            self.resource
                .as_ref()?
                .raw_data
                .cast::<FileHeader>()
                .as_ref()
        }
    }
}

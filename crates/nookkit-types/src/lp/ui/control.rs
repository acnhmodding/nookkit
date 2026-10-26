use core::ffi::{CStr, c_void};
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::lp::ui::Name;
use crate::native::{Class, Fn, NativeResult, sym, type_info};
use crate::sead::runtime_type_info::Interface;
use crate::sead::{ListNode, OffsetList, SafeString};

#[vtable(path = lp::ui::control::VftControl)]
pub unsafe trait VftControl {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const Interface) -> bool;
    unsafe fn get_runtime_type_info(&self) -> *const Interface;

    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);

    unsafe fn get_class_info(&self) -> *const SafeString<'static>;
    unsafe fn on_bound_sight(&mut self);
    unsafe fn on_pre_wake(&mut self);
    unsafe fn on_post_wake(&mut self);
    unsafe fn on_pre_sleep(&mut self);
    unsafe fn on_post_sleep(&mut self);
    unsafe fn on_pre_calc(&mut self);
    unsafe fn on_post_calc(&mut self);
}

#[repr(C)]
pub struct Control {
    pub vtable: *const VftControlLayout,
    pub child_node: ListNode,
    pub name: Name,
    pub parent: *mut Control,
    pub child_list: OffsetList,
    pub construct_arg_10: *mut c_void,
    pub ptr_58: *mut c_void,
    pub list_60: OffsetList,
    pub list_78: OffsetList,
    pub content_list: OffsetList,
    pub content: [u8; 0x48],
    pub list_f0: OffsetList,
    pub list_108: OffsetList,
    pub list_120: OffsetList,
    pub list_138: OffsetList,
    pub list_150: OffsetList,
    pub ptr_168: *mut c_void,
    pub ptr_170: *mut c_void,
    pub list_178: OffsetList,
    pub list_190: OffsetList,
    pub flag_1a8: u8,
    _unk_1a9: [u8; 0x7],
}

const _: () = {
    assert!(offset_of!(Control, name) == 0x18);
    assert!(offset_of!(Control, child_list) == 0x38);
    assert!(offset_of!(Control, content) == 0xa8);
    assert!(offset_of!(Control, flag_1a8) == 0x1a8);
    assert!(core::mem::size_of::<Control>() == 0x1b0);
    assert!(VftControlLayout::SLOT_COUNT == 12);
};

#[vtable_impl]
unsafe impl VftControl for Control {}

unsafe impl Class for Control {
    const RUNTIME_TYPE_INFO: Fn = sym::Lp::UI::Control::getRuntimeTypeInfo.erase();
}

crate::native! {
    impl Control {
        unsafe fn find_by_path_native(&mut self, path: *const SafeString<'_>) -> *mut Control
            = sym::Lp::UI::Control::findByPath;

        pub unsafe fn set_focusable(&mut self, player: u32, enable: bool)
            = sym::Lp::UI::Control::setFocusable;
    }
}

impl Control {
    pub unsafe fn name(&self) -> Option<&CStr> {
        unsafe { self.name.as_c_str() }
    }

    pub unsafe fn children(&self) -> impl Iterator<Item = *mut Control> + '_ {
        unsafe { self.child_list.iter() }
    }

    pub unsafe fn is_derived(&self, type_info: *const Interface) -> bool {
        unsafe { !type_info.is_null() && self.check_derived_runtime_type_info(type_info) }
    }

    pub unsafe fn is<T: Class>(&self) -> NativeResult<bool> {
        Ok(unsafe { self.is_derived(type_info::<T>()?) })
    }

    /// `None` when the control isn't a `T`
    pub unsafe fn cast<T: Class>(&mut self) -> NativeResult<Option<&mut T>> {
        Ok(unsafe { self.is::<T>()? }.then(|| unsafe { &mut *(self as *mut Self).cast::<T>() }))
    }

    pub unsafe fn cast_ptr<'a, T: Class>(control: *mut Control) -> NativeResult<Option<&'a mut T>> {
        match unsafe { control.as_mut() } {
            Some(control) => unsafe { control.cast() },
            None => Ok(None),
        }
    }

    /// Searches the subtree, e.g. `/L_BtnOk_00`
    pub unsafe fn find_by_path(&mut self, path: &CStr) -> NativeResult<*mut Control> {
        let path = SafeString::new(path)?;

        Ok(unsafe { self.find_by_path_native(&path)? })
    }

    pub unsafe fn find<T: Class>(&mut self, path: &CStr) -> NativeResult<Option<&mut T>> {
        unsafe { Self::cast_ptr(self.find_by_path(path)?) }
    }
}

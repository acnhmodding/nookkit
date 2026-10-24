use core::ffi::{CStr, c_void};

use native_vtable::{vtable, vtable_impl};

use crate::sead::SafeString;

mod native {
    use super::Control;
    use crate::sead::SafeString;

    crate::native::native_fns! {
        fn find_by_path(parent: *mut Control, path: *const SafeString<'_>) -> *mut Control = 0x376e6b0;
        fn set_focusable(control: *mut Control, player: u32, enable: bool) = 0x3c5d0e0;
    }
}

#[vtable(path = lp::ui::control::VftControl)]
pub unsafe trait VftControl {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const c_void) -> bool;
}

const _: () = assert!(VftControlLayout::OFFSET_CHECK_DERIVED_RUNTIME_TYPE_INFO == 0);

#[repr(C)]
pub struct Control {
    pub vtable: *const VftControlLayout,
}

#[vtable_impl]
unsafe impl VftControl for Control {}

impl Control {
    /// `type_info` comes from `native::guarded_type_info`; `None` (not initialized yet) is no match.
    pub unsafe fn is_derived(&self, type_info: Option<*const c_void>) -> bool {
        unsafe { type_info.is_some_and(|info| self.check_derived_runtime_type_info(info)) }
    }

    pub unsafe fn find_by_path(&mut self, path: &CStr) -> *mut Control {
        let path = SafeString::new(path);

        unsafe { native::find_by_path(self, &path) }
    }

    pub unsafe fn set_focusable(&mut self, player: u32, enable: bool) {
        unsafe { native::set_focusable(self, player, enable) }
    }
}

#[derive(Clone, Copy)]
pub enum NavDirection {
    Up = 0,
    Down = 1,
}

#[repr(C)]
pub struct FocusNode {
    _unk_0: [u8; 0x1460],
    pub links: [*mut FocusNode; 2],
}

const _: () = assert!(core::mem::offset_of!(FocusNode, links) == 0x1460);

impl FocusNode {
    pub fn link(&self, direction: NavDirection) -> *mut FocusNode {
        self.links[direction as usize]
    }

    pub fn set_link(&mut self, direction: NavDirection, target: *mut FocusNode) {
        self.links[direction as usize] = target;
    }
}

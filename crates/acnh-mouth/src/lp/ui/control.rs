use core::ffi::{CStr, c_void};

use native_vtable::{native, vtable, vtable_impl};

use crate::native::NativeResult;
use crate::sead::SafeString;

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

#[native(resolve = crate::native::resolve)]
impl Control {
    #[native(symbol = "Lp::UI::Control::findByPath")]
    unsafe fn find_by_path_native(&mut self, path: *const SafeString<'_>) -> *mut Control;

    #[native(symbol = "Lp::UI::Control::setFocusable")]
    pub unsafe fn set_focusable(&mut self, player: u32, enable: bool);
}

impl Control {
    /// `type_info` comes from `native::guarded_type_info`; `None` (not initialized yet) is no match.
    pub unsafe fn is_derived(&self, type_info: Option<*const c_void>) -> bool {
        unsafe { type_info.is_some_and(|info| self.check_derived_runtime_type_info(info)) }
    }

    pub unsafe fn find_by_path(&mut self, path: &CStr) -> NativeResult<*mut Control> {
        let path = SafeString::new(path);

        unsafe { self.find_by_path_native(&path) }
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
    pub fn set_link(&mut self, direction: NavDirection, target: *mut FocusNode) {
        self.links[direction as usize] = target;
    }
}

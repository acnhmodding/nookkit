#![no_std]

use core::ffi::{c_char, c_void};

pub const API_NAME: &core::ffi::CStr = c"acnh-ui";
pub const API_VERSION: u32 = 1;

unsafe extern "C" {
    pub fn sl_api_publish(name: *const c_char, version: u32, vtable: *const c_void);
    pub fn sl_api_get(name: *const c_char, version: u32) -> *const c_void;
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok = 0,
    InvalidArg,
    PaneNotFound,
    TypeMismatch,
    NoEventTable,
    EventTableFull,
    LockFailed,
    MissingSymbol,
    UnsupportedBuild,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Str {
    pub ptr: *const u8,
    pub len: usize,
}

impl Str {
    pub const fn new(s: &str) -> Self {
        Self {
            ptr: s.as_ptr(),
            len: s.len(),
        }
    }

    /// # Safety
    /// `ptr` must point to `len` bytes of UTF-8 that outlive `'a`.
    pub unsafe fn as_str<'a>(self) -> Option<&'a str> {
        if self.ptr.is_null() {
            return (self.len == 0).then_some("");
        }

        let bytes = unsafe { core::slice::from_raw_parts(self.ptr, self.len) };
        core::str::from_utf8(bytes).ok()
    }
}

#[repr(C)]
pub struct Callback {
    pub func: unsafe extern "C" fn(user: *mut c_void, arg: *const c_void),
    pub user: *mut c_void,
    pub drop: Option<unsafe extern "C" fn(user: *mut c_void)>,
}

unsafe impl Send for Callback {}

impl Callback {
    /// # Safety
    /// `arg` must be what the registering entry documents for this callback.
    pub unsafe fn call(&self, arg: *const c_void) {
        unsafe { (self.func)(self.user, arg) }
    }
}

impl Drop for Callback {
    fn drop(&mut self) {
        if let Some(drop) = self.drop {
            unsafe { drop(self.user) };
        }
    }
}

pub const EVENT_FOCUS: u32 = 0;
pub const EVENT_DECIDE: u32 = 5;

#[repr(C)]
pub struct ControlEvent {
    pub page: *mut c_void,
    pub control: *mut c_void,
    pub kind: u32,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}

#[repr(C)]
pub struct AcnhUiV1 {
    pub size: usize,
    pub framework_version: u32,

    /// Empty `layout` or `class` matches any. `cb` gets the page pointer after it is built
    pub on_page_built: unsafe extern "C" fn(layout: Str, class: Str, cb: Callback) -> Status,

    /// Empty `class` skips the type check
    pub find_pane: unsafe extern "C" fn(
        page: *mut c_void,
        path: Str,
        class: Str,
        out: *mut *mut c_void,
    ) -> Status,

    /// `cb` gets a `*const ControlEvent`. Dropped when the page is built again
    pub on_control_event:
        unsafe extern "C" fn(page: *mut c_void, kind: u32, cb: Callback) -> Status,

    pub link:
        unsafe extern "C" fn(from: *mut c_void, direction: Direction, to: *mut c_void) -> Status,
    pub set_focusable: unsafe extern "C" fn(pane: *mut c_void, player: u32, enable: bool) -> Status,
    pub lock_player: unsafe extern "C" fn(player: u8) -> Status,
    pub release_player: unsafe extern "C" fn(player: u8) -> Status,
    pub set_visible: unsafe extern "C" fn(pane: *mut c_void, visible: bool) -> Status,
}

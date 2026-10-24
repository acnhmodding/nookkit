use core::ffi::{CStr, c_char};
use core::mem::offset_of;

use native_vtable::{vtable, vtable_impl};

use crate::lp::ui::Control;
use crate::sead::{Delegate, SafeString};

#[vtable(path = game::ui::view_page::VftViewPage)]
pub unsafe trait VftViewPage {
    #[vtable(offset = 0x20)]
    unsafe fn class_name(&self) -> *const SafeString<'static>;
    #[vtable(offset = 0x170)]
    unsafe fn layout_name(&self) -> *const c_char;
}

const _: () = {
    assert!(VftViewPageLayout::OFFSET_CLASS_NAME == 0x20);
    assert!(VftViewPageLayout::OFFSET_LAYOUT_NAME == 0x170);
};

#[repr(C)]
pub struct ViewPage {
    pub vtable: *const VftViewPageLayout,
    _unk_8: [u8; 0x3e8],
    pub root_count: i32,
    _pad_3f4: u32,
    pub roots: *mut *mut Control,
    _unk_400: [u8; 0x30],

    /// `Delegate[kinds][cols]`, the game takes the first free column of a kind
    pub event_cols: i32,

    pub event_kinds: i32,
    pub events: *mut Delegate,
}

const _: () = {
    assert!(offset_of!(ViewPage, root_count) == 0x3f0);
    assert!(offset_of!(ViewPage, roots) == 0x3f8);
    assert!(offset_of!(ViewPage, event_cols) == 0x430);
    assert!(offset_of!(ViewPage, event_kinds) == 0x434);
    assert!(offset_of!(ViewPage, events) == 0x438);
};

#[vtable_impl]
unsafe impl VftViewPage for ViewPage {}

impl ViewPage {
    pub unsafe fn layout(&self) -> Option<&'static CStr> {
        unsafe {
            let name = self.layout_name();
            (!name.is_null()).then(|| CStr::from_ptr(name))
        }
    }

    pub unsafe fn class(&self) -> Option<&'static CStr> {
        unsafe {
            let name = self.class_name().as_ref()?;
            (!name.string_top.is_null()).then(|| CStr::from_ptr(name.string_top))
        }
    }

    pub unsafe fn root(&self) -> *mut Control {
        unsafe {
            if self.root_count < 1 || self.roots.is_null() {
                return core::ptr::null_mut();
            }

            *self.roots
        }
    }

    pub unsafe fn event_slots(&mut self, kind: u32) -> Option<&mut [Delegate]> {
        if self.events.is_null() || self.event_cols < 1 || kind >= self.event_kinds as u32 {
            return None;
        }

        let cols = self.event_cols as usize;

        unsafe {
            Some(core::slice::from_raw_parts_mut(
                self.events.add(kind as usize * cols),
                cols,
            ))
        }
    }
}

#[repr(C)]
pub struct ControlEvent {
    _unk_0: [u8; 0x8],
    pub control: *mut Control,
    _unk_10: [u8; 0x4],
    pub kind: u32,
}

const _: () = {
    assert!(offset_of!(ControlEvent, control) == 0x8);
    assert!(offset_of!(ControlEvent, kind) == 0x14);
};

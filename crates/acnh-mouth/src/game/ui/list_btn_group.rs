use core::ffi::c_void;

use native_vtable::{native, vtable, vtable_impl};

use crate::lp::ui::ScrollBtnListCtrl;
use crate::native::{NativeResult, vtable};
use crate::sead::Delegate;

#[vtable(path = game::ui::list_btn_group::VftListBtnGroup)]
pub unsafe trait VftListBtnGroup {
    #[vtable(offset = 0x10)]
    unsafe fn selected(&self) -> i32;
    unsafe fn set_selected(&mut self, index: i32);
    unsafe fn is_changed(&self, index: i32) -> bool;
    unsafe fn clear_selection(&mut self, list: *mut ScrollBtnListCtrl);
}

const _: () = {
    assert!(VftListBtnGroupLayout::OFFSET_SELECTED == 0x10);
    assert!(VftListBtnGroupLayout::OFFSET_SET_SELECTED == 0x18);
    assert!(VftListBtnGroupLayout::OFFSET_IS_CHANGED == 0x20);
    assert!(VftListBtnGroupLayout::OFFSET_CLEAR_SELECTION == 0x28);
};

#[repr(C)]
pub struct ListBtnGroup {
    pub vtable: *const VftListBtnGroupLayout,
    pub enabled: bool,
    pub keep: i32,
    pub locked: bool,
    pub toggle: bool,
    pub selected: i32,
}

const _: () = {
    assert!(core::mem::size_of::<ListBtnGroup>() == 0x18);
    assert!(core::mem::offset_of!(ListBtnGroup, keep) == 0xc);
    assert!(core::mem::offset_of!(ListBtnGroup, locked) == 0x10);
    assert!(core::mem::offset_of!(ListBtnGroup, selected) == 0x14);
};

#[vtable_impl]
unsafe impl VftListBtnGroup for ListBtnGroup {}

impl ListBtnGroup {
    pub const fn empty() -> Self {
        Self {
            vtable: core::ptr::null(),
            enabled: false,
            keep: 0,
            locked: false,
            toggle: false,
            selected: 0,
        }
    }

    /// The group setupControls gives the eye list.
    pub fn new() -> NativeResult<Self> {
        Ok(Self {
            vtable: vtable("game::UI::ListBtnGroup::vtable")?,
            enabled: true,
            toggle: true,
            ..Self::empty()
        })
    }
}

#[native(resolve = crate::native::resolve)]
impl ListBtnGroup {
    #[native(symbol = "game::UI::CharaMakeSeq::refreshListGroup")]
    pub unsafe fn refresh(&mut self, list: *mut ScrollBtnListCtrl);
}

#[repr(C)]
pub struct ScrollInfo {
    _unk_0: [u8; 0xc],
    pub first_item: i32,
}

#[derive(PartialEq, Eq)]
pub enum ItemOp {
    Render,
    Move,
    Clear,
}

#[repr(C)]
pub struct ItemEvent {
    pub op: i32,
    pub slot: i32,
    pub from_slot: i32,
    _unk_c: i32,
    pub scroll: *const ScrollInfo,
}

const _: () = assert!(core::mem::offset_of!(ItemEvent, scroll) == 0x10);

impl ItemEvent {
    pub fn op(&self) -> Option<ItemOp> {
        match self.op {
            0 => Some(ItemOp::Render),
            1 => Some(ItemOp::Move),
            2 => Some(ItemOp::Clear),
            _ => None,
        }
    }

    pub unsafe fn item(&self) -> i32 {
        unsafe { self.slot + (*self.scroll).first_item }
    }
}

// Turns a list's scroll events into per-slot ItemEvents for its delegate (layout of seq+0x1cf70).
#[repr(C)]
pub struct ItemUpdater {
    pub count: u32,
    _pad_4: u32,
    pub delegate: Delegate,
}

impl ItemUpdater {
    pub const fn empty() -> Self {
        Self {
            count: 0,
            _pad_4: 0,
            delegate: Delegate {
                vtable: core::ptr::null(),
                object: core::ptr::null_mut(),
                method: 0,
                method_adj: 0,
            },
        }
    }

    pub fn new(count: u32, delegate: Delegate) -> Self {
        Self {
            count,
            _pad_4: 0,
            delegate,
        }
    }

    pub unsafe fn update(&mut self, scroll_event: *mut c_void) -> NativeResult {
        unsafe { self.update_native(&scroll_event) }.map_err(Into::into)
    }
}

#[native(resolve = crate::native::resolve)]
impl ItemUpdater {
    #[native(symbol = "ListItemUpdater::update")]
    unsafe fn update_native(&mut self, event: *const *mut c_void);
}

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use native_vtable::{VTableLayout, native, vtable, vtable_impl};

use crate::game::ui::AppUniteBtnCtrl;
use crate::lp::ui::{Control, PartsCtrl, VftPartsCtrl};
use crate::lp::utl::StateMachine;
use crate::native::{Class, NativeResult, type_info_named, vtable};
use crate::sead::Delegate;

pub const STATE_PLAY: i32 = 2;
pub const STATE_PAUSE: i32 = 3;
const PAUSE_KEEP_BUTTONS: i32 = 2;

#[vtable(path = game::ui_sys::scroll_btn_list_ctrl::VftScrollBtnListCtrl)]
pub unsafe trait VftScrollBtnListCtrl: VftPartsCtrl {
    unsafe fn register_child(&mut self, child: *mut Control);
    unsafe fn on_attach(&mut self, params: *const AttachParams);
    unsafe fn on_set_content(&mut self, params: *const SetContentParams);
    unsafe fn vf_108(&mut self);
    unsafe fn on_slot_focus(&mut self, slot: *mut u32);
    unsafe fn unfocus_slot(&mut self, slot: u32);
    unsafe fn on_scroll(&mut self, from: i32, to: i32);
    unsafe fn vf_128(&mut self);
    unsafe fn vf_130(&mut self);
    unsafe fn vf_138(&mut self);
}

#[repr(C)]
pub struct PointerHandler {
    _unk_0: [u8; 0x14],
    pub unk_14: i32,
}

#[repr(C)]
pub struct ScrollBtnListCtrl {
    pub base: PartsCtrl,
    pub scroll_bar: *mut c_void,
    pub button_count: i32,
    _unk_20c: [u8; 0x4],
    pub buttons: *mut *mut AppUniteBtnCtrl,
    _unk_218: [u8; 0x58],
    pub slide_area: *mut [*mut c_void; 2],
    pub next_scroll_sign: *mut c_void,
    pub prev_scroll_sign: *mut c_void,
    _unk_288: [u8; 0x8],

    /// [`STATE_PLAY`] handles the D-pad
    pub state_machine: StateMachine,

    pub focus_manager: *mut c_void,
    pub player: u32,

    /// Button mode under [`STATE_PAUSE`], 2 keeps them focusable
    pub pause_button_mode: i32,

    pub line_size: i32,
    pub visible_lines: i32,
    _unk_2f0: [u8; 0x8],
    pub slot_count: i32,
    _unk_2fc: [u8; 0x34],
    pub scroll_delegate: Delegate,

    /// Called when the D-pad leaves the list, writes the target control
    pub leave_delegate: Delegate,

    _unk_370: [u8; 0x5c],
    pub item_count: i32,
    _unk_3d0: [u8; 0x4],
    pub padded_item_count: i32,
    pub scroll_range: i32,
    _unk_3dc: [u8; 0x54],

    /// Ring buffer: visible line C is button line `first_button_line + C` and item line `first_line + C`
    pub first_line: i32,

    pub line_count: i32,
    pub first_button_line: i32,

    /// -1 for none
    pub focused_slot: i32,

    _unk_440: [u8; 0x10],
    pub anim_450: *mut c_void,
    _unk_458: [u8; 0x28],
    pub pointer_handler: *mut PointerHandler,
    _unk_488: [u8; 0x70],
}

const _: () = {
    assert!(offset_of!(ScrollBtnListCtrl, scroll_bar) == 0x200);
    assert!(offset_of!(ScrollBtnListCtrl, button_count) == 0x208);
    assert!(offset_of!(ScrollBtnListCtrl, buttons) == 0x210);
    assert!(offset_of!(ScrollBtnListCtrl, slide_area) == 0x270);
    assert!(offset_of!(ScrollBtnListCtrl, state_machine) == 0x290);
    assert!(offset_of!(ScrollBtnListCtrl, focus_manager) == 0x2d8);
    assert!(offset_of!(ScrollBtnListCtrl, player) == 0x2e0);
    assert!(offset_of!(ScrollBtnListCtrl, pause_button_mode) == 0x2e4);
    assert!(offset_of!(ScrollBtnListCtrl, line_size) == 0x2e8);
    assert!(offset_of!(ScrollBtnListCtrl, visible_lines) == 0x2ec);
    assert!(offset_of!(ScrollBtnListCtrl, slot_count) == 0x2f8);
    assert!(offset_of!(ScrollBtnListCtrl, scroll_delegate) == 0x330);
    assert!(offset_of!(ScrollBtnListCtrl, leave_delegate) == 0x350);
    assert!(offset_of!(ScrollBtnListCtrl, item_count) == 0x3cc);
    assert!(offset_of!(ScrollBtnListCtrl, padded_item_count) == 0x3d4);
    assert!(offset_of!(ScrollBtnListCtrl, scroll_range) == 0x3d8);
    assert!(offset_of!(ScrollBtnListCtrl, first_line) == 0x430);
    assert!(offset_of!(ScrollBtnListCtrl, first_button_line) == 0x438);
    assert!(offset_of!(ScrollBtnListCtrl, focused_slot) == 0x43c);
    assert!(offset_of!(ScrollBtnListCtrl, anim_450) == 0x450);
    assert!(offset_of!(ScrollBtnListCtrl, pointer_handler) == 0x480);
    assert!(core::mem::size_of::<ScrollBtnListCtrl>() == 0x4f8);
    assert!(VftScrollBtnListCtrlLayout::SLOT_COUNT == 40);
};

crate::derive_base!(ScrollBtnListCtrl => PartsCtrl);

#[vtable_impl]
unsafe impl VftScrollBtnListCtrl for ScrollBtnListCtrl {}

unsafe impl Class for ScrollBtnListCtrl {
    const RUNTIME_TYPE_INFO: &'static str = "game::UISys::ScrollBtnListCtrl::getRuntimeTypeInfo";
}

#[derive(Clone, Copy)]
pub struct Content {
    pub item_count: i32,
    pub scroll_item: i32,
    pub focus_item: i32,
    pub flags: [u8; 0xc],
    pub unk_20: i32,
    pub unk_24: i32,
    pub unk_28: i32,
}

#[repr(C)]
pub struct SetContentParams {
    pub vtable: *const c_void,
    pub item_count: i32,
    pub scroll_item: i32,
    pub focus_item: i32,
    pub flags: [u8; 0xc],
    pub unk_20: i32,
    pub unk_24: i32,
    pub unk_28: i32,
    pub player: u8,
    _pad_2d: [u8; 0x3],
}

const _: () = assert!(core::mem::size_of::<SetContentParams>() == 0x30);

#[derive(Clone, Copy)]
pub struct Enable {
    pub update_cursor_button: bool,
    pub cursor_button_arg0: u8,
    pub cursor_button_arg1: u8,
    pub focus_cursor: bool,
    pub cursor_item: i32,
}

#[repr(C)]
pub struct EnableParams {
    pub vtable: *const c_void,
    pub update_cursor_button: bool,
    pub cursor_button_arg0: u8,
    pub cursor_button_arg1: u8,
    pub focus_cursor: bool,
    pub cursor_item: i32,
}

const _: () = assert!(core::mem::size_of::<EnableParams>() == 0x10);

#[repr(C)]
pub struct AttachParams {
    pub page: *mut c_void,
    pub player: u32,
    _pad_c: u32,
    pub root: *mut Control,

    /// Extra key IDs, -1 for none
    pub key_18: i32,

    pub key_1c: i32,
}

const _: () = assert!(core::mem::size_of::<AttachParams>() == 0x20);

#[repr(C)]
pub struct LeaveEvent {
    _unk_0: [u8; 0x10],
    pub direction: i32,
}

pub enum LeaveDirection {
    Up,
    Down,
}

impl LeaveEvent {
    pub fn direction(&self) -> Option<LeaveDirection> {
        match self.direction {
            0 => Some(LeaveDirection::Up),
            1 => Some(LeaveDirection::Down),
            _ => None,
        }
    }
}

#[native(resolve = crate::native::resolve)]
impl ScrollBtnListCtrl {
    #[native(symbol = "game::UISys::ScrollBtnListCtrl::bindButtons")]
    pub unsafe fn bind_buttons(&mut self, builder: *mut c_void);

    #[native(symbol = "game::UISys::ScrollBtnListCtrl::attach")]
    unsafe fn attach_native(&mut self, params: *const AttachParams);

    #[native(symbol = "game::UISys::ScrollBtnListCtrl::setContent")]
    unsafe fn set_content_native(&mut self, params: *const SetContentParams);

    #[native(symbol = "game::UISys::ScrollBtnListCtrl::enable")]
    unsafe fn enable_native(&mut self, params: *const EnableParams);

    #[native(symbol = "game::UISys::ScrollBtnListCtrl::disable")]
    pub unsafe fn disable(&mut self, mode: u32);

    #[native(symbol = "game::UISys::ScrollBtnListCtrl::itemIndexOf")]
    pub unsafe fn item_index_of(&mut self, control: *mut Control) -> i32;
}

impl ScrollBtnListCtrl {
    pub unsafe fn attach(
        &mut self,
        page: *mut c_void,
        root: *mut Control,
        player: u32,
    ) -> NativeResult {
        let root_type = type_info_named("game::UI::AppBodyCtrl")?;
        let is_root = unsafe { root.as_ref().is_some_and(|root| root.is_derived(root_type)) };
        let params = AttachParams {
            page,
            player,
            _pad_c: 0,
            root: if is_root { root } else { null_mut() },
            key_18: -1,
            key_1c: -1,
        };

        unsafe { self.attach_native(&params) }.map_err(Into::into)
    }

    pub unsafe fn set_content(&mut self, content: &Content, player: u8) -> NativeResult {
        let params = SetContentParams {
            vtable: vtable("game::UISys::ScrollBtnListCtrl::SetContentParams::vtable")?,
            item_count: content.item_count,
            scroll_item: content.scroll_item,
            focus_item: content.focus_item,
            flags: content.flags,
            unk_20: content.unk_20,
            unk_24: content.unk_24,
            unk_28: content.unk_28,
            player: if player < 8 { player } else { 0xff },
            _pad_2d: [0; 3],
        };

        unsafe { self.set_content_native(&params) }.map_err(Into::into)
    }

    pub unsafe fn enable(&mut self, enable: &Enable) -> NativeResult {
        let params = EnableParams {
            vtable: vtable("game::UISys::ScrollBtnListCtrl::EnableParams::vtable")?,
            update_cursor_button: enable.update_cursor_button,
            cursor_button_arg0: enable.cursor_button_arg0,
            cursor_button_arg1: enable.cursor_button_arg1,
            focus_cursor: enable.focus_cursor,
            cursor_item: enable.cursor_item,
        };

        unsafe { self.enable_native(&params) }.map_err(Into::into)
    }

    pub unsafe fn play(&mut self) -> NativeResult {
        unsafe { self.state_machine.change_state(STATE_PLAY) }.map_err(Into::into)
    }

    pub unsafe fn is_ready_to_play(&self) -> bool {
        unsafe {
            self.slot_count != 0
                && self.button_count != 0
                && self
                    .slide_area
                    .as_ref()
                    .is_some_and(|slots| !slots[1].is_null())
                && self
                    .pointer_handler
                    .as_ref()
                    .is_none_or(|handler| handler.unk_14 != -1)
                && !self.focus_manager.is_null()
                && self.scroll_range > 0
                && self.padded_item_count != 0
        }
    }

    /// When focus leaves, the list stops taking the D-pad
    pub unsafe fn pause(&mut self) -> NativeResult {
        self.focused_slot = -1;

        if self.state_machine.current_state == STATE_PLAY {
            self.pause_button_mode = PAUSE_KEEP_BUTTONS;

            unsafe { self.state_machine.change_state(STATE_PAUSE)? };
        }

        Ok(())
    }

    pub fn button_count(&self) -> usize {
        self.button_count.max(0) as usize
    }

    pub fn slot_count(&self) -> usize {
        self.slot_count.max(0) as usize
    }

    pub unsafe fn button(&self, slot: usize) -> *mut AppUniteBtnCtrl {
        if slot >= self.button_count() {
            return null_mut();
        }

        unsafe { *self.buttons.add(slot) }
    }

    pub unsafe fn visible_button(&self, line: usize, pos: usize) -> *mut AppUniteBtnCtrl {
        let line_size = self.line_size.max(1) as usize;
        let lines = (self.button_count() / line_size).max(1);
        let first = self.first_button_line.max(0) as usize;

        unsafe { self.button((first + line) % lines * line_size + pos) }
    }

    pub fn focused_visible_line(&self) -> Option<usize> {
        let line_size = self.line_size.max(1) as usize;
        let lines = (self.button_count() / line_size).max(1);
        let slot = usize::try_from(self.focused_slot).ok()?;
        let first = self.first_button_line.max(0) as usize;

        Some((slot / line_size + lines - first % lines) % lines)
    }
}

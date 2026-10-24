use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use crate::game::ui::AppUniteBtnCtrl;
use crate::lp::ui::{Control, VftControlLayout};
use crate::lp::utl::StateMachine;
use crate::native::{got_vtable, guarded_type_info, text_vtable};
use crate::sead::Delegate;

mod native {
    use super::{AttachParams, EnableParams, ScrollBtnListCtrl, SetContentParams};
    use crate::lp::ui::Control;

    crate::native::native_fns! {
        fn bind_buttons(list: *mut ScrollBtnListCtrl, builder: *mut core::ffi::c_void) = 0x2ab1110;
        fn attach(list: *mut ScrollBtnListCtrl, params: *const AttachParams) = 0x3763fe0;
        fn set_content(list: *mut ScrollBtnListCtrl, params: *const SetContentParams) = 0x3764290;
        fn enable(list: *mut ScrollBtnListCtrl, params: *const EnableParams) = 0x3764fe0;
        fn disable(list: *mut ScrollBtnListCtrl, mode: u32) = 0x335b550;
        fn item_index_of(list: *mut ScrollBtnListCtrl, button: *mut Control) -> i32 = 0x3765200;
    }
}

const GOT_TYPE_INFO: usize = 0x4ac4d68;
const GOT_TYPE_INFO_GUARD: usize = 0x4ac4d70;
const GOT_ENABLE_PARAMS_VTABLE: usize = 0x4ac9c80;
const GOT_PAGE_ROOT_TYPE_INFO: usize = 0x4ac4e58;
const GOT_PAGE_ROOT_TYPE_INFO_GUARD: usize = 0x4ac4e60;
const SET_CONTENT_PARAMS_VTABLE: usize = 0x4a2b020;

const STATE_PLAY: u32 = 2;
const STATE_PAUSE: u32 = 3;
const PAUSE_KEEP_BUTTONS: i32 = 2;

#[repr(C)]
pub struct InputClaim {
    _unk_0: [u8; 0x14],
    pub unk_14: i32,
}

// The buttons are a ring the list scrolls through: visible line c is button line
// `first_button_line + c` and item line `first_line + c`. Lines are columns in CharaMake.
#[repr(C)]
pub struct ScrollBtnListCtrl {
    pub vtable: *const VftControlLayout,
    _unk_8: [u8; 0x200],
    pub button_count: i32,
    _unk_20c: [u8; 0x4],
    pub buttons: *mut *mut AppUniteBtnCtrl,
    _unk_218: [u8; 0x58],
    pub unk_270: *mut [*mut c_void; 2],
    _unk_278: [u8; 0x18],
    // 2 = cPlay (handles the D-pad itself), 3 = cPause.
    pub state_machine: StateMachine,
    pub focus_manager: *mut c_void,
    pub player: u32,
    // Button mode cPause applies; 2 keeps the buttons focusable.
    pub pause_button_mode: i32,
    pub line_size: i32,
    pub visible_lines: i32,
    _unk_2f0: [u8; 0x4],
    pub visible_items: i32,
    pub slot_count: i32,
    _unk_2fc: [u8; 0x34],
    pub scroll_delegate: Delegate,
    // Invoked with a LeaveEvent when the D-pad moves out of the list; writes the target control.
    pub leave_delegate: Delegate,
    _unk_370: [u8; 0x5c],
    pub item_count: i32,
    _unk_3d0: [u8; 0x4],
    pub padded_item_count: i32,
    pub unk_3d8: i32,
    _unk_3dc: [u8; 0x54],
    pub first_line: i32,
    pub last_first_line: i32,
    pub first_button_line: i32,
    // Slot the list believes is focused, -1 for none.
    pub focused_slot: i32,
    pub cursor_line: i32,
    pub cursor_pos: i32,
    _unk_448: [u8; 0x38],
    pub input_claim: *mut InputClaim,
}

const _: () = {
    assert!(offset_of!(ScrollBtnListCtrl, button_count) == 0x208);
    assert!(offset_of!(ScrollBtnListCtrl, buttons) == 0x210);
    assert!(offset_of!(ScrollBtnListCtrl, unk_270) == 0x270);
    assert!(offset_of!(ScrollBtnListCtrl, state_machine) == 0x290);
    assert!(offset_of!(ScrollBtnListCtrl, focus_manager) == 0x2d8);
    assert!(offset_of!(ScrollBtnListCtrl, player) == 0x2e0);
    assert!(offset_of!(ScrollBtnListCtrl, pause_button_mode) == 0x2e4);
    assert!(offset_of!(ScrollBtnListCtrl, line_size) == 0x2e8);
    assert!(offset_of!(ScrollBtnListCtrl, visible_lines) == 0x2ec);
    assert!(offset_of!(ScrollBtnListCtrl, visible_items) == 0x2f4);
    assert!(offset_of!(ScrollBtnListCtrl, slot_count) == 0x2f8);
    assert!(offset_of!(ScrollBtnListCtrl, scroll_delegate) == 0x330);
    assert!(offset_of!(ScrollBtnListCtrl, leave_delegate) == 0x350);
    assert!(offset_of!(ScrollBtnListCtrl, item_count) == 0x3cc);
    assert!(offset_of!(ScrollBtnListCtrl, padded_item_count) == 0x3d4);
    assert!(offset_of!(ScrollBtnListCtrl, unk_3d8) == 0x3d8);
    assert!(offset_of!(ScrollBtnListCtrl, first_line) == 0x430);
    assert!(offset_of!(ScrollBtnListCtrl, first_button_line) == 0x438);
    assert!(offset_of!(ScrollBtnListCtrl, focused_slot) == 0x43c);
    assert!(offset_of!(ScrollBtnListCtrl, cursor_pos) == 0x444);
    assert!(offset_of!(ScrollBtnListCtrl, input_claim) == 0x480);
};

#[repr(C)]
struct SetContentParams {
    vtable: *const c_void,
    item_count: i32,
    scroll_item: i32,
    focus_item: i32,
    flags: [u8; 0xc],
    unk_20: i32,
    unk_24: i32,
    unk_28: i32,
    player: u8,
    _pad_2d: [u8; 0x3],
}

const _: () = assert!(core::mem::size_of::<SetContentParams>() == 0x30);

#[repr(C)]
struct EnableParams {
    vtable: *const c_void,
    update_cursor_button: bool,
    cursor_button_arg0: u8,
    cursor_button_arg1: u8,
    focus_cursor: bool,
    cursor_item: i32,
}

const _: () = assert!(core::mem::size_of::<EnableParams>() == 0x10);

#[repr(C)]
struct AttachParams {
    page: *mut c_void,
    player: u32,
    _pad_c: u32,
    root: *mut Control,
    unk_18: i32,
    unk_1c: i32,
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

impl ScrollBtnListCtrl {
    pub unsafe fn cast(control: *mut Control) -> Option<&'static mut Self> {
        unsafe {
            let control = control.as_mut()?;

            control
                .is_derived(guarded_type_info(GOT_TYPE_INFO, GOT_TYPE_INFO_GUARD))
                .then(|| &mut *(control as *mut Control).cast::<Self>())
        }
    }

    /// Clones the template buttons into slots; the page does this for its lists while building.
    pub unsafe fn bind_buttons(&mut self, builder: *mut c_void) {
        unsafe { native::bind_buttons(self, builder) }
    }

    /// What the page's list attach routine (0x7102b99900) does for its own lists on every entry:
    /// sets the player and focus manager, links the buttons into the focus graph.
    pub unsafe fn attach(&mut self, page: *mut c_void, root: *mut Control) {
        let root_type = unsafe { guarded_type_info(GOT_PAGE_ROOT_TYPE_INFO, GOT_PAGE_ROOT_TYPE_INFO_GUARD) };
        let is_root = unsafe { root.as_ref().is_some_and(|root| root.is_derived(root_type)) };
        let params = AttachParams {
            page,
            player: 0,
            _pad_c: 0,
            root: if is_root { root } else { null_mut() },
            unk_18: -1,
            unk_1c: -1,
        };

        unsafe { native::attach(self, &params) }
    }

    /// Same values the game passes for the eye list on page refresh.
    pub unsafe fn set_content(&mut self, item_count: i32, selected: i32, player: u8) {
        let mut flags = [0; 0xc];

        flags[0x16 - 0x14] = 1;
        flags[0x19 - 0x14] = 1;

        let params = SetContentParams {
            vtable: text_vtable(SET_CONTENT_PARAMS_VTABLE),
            item_count,
            scroll_item: selected,
            focus_item: selected,
            flags,
            unk_20: 0,
            unk_24: 0xc,
            unk_28: 6,
            player: if player < 8 { player } else { 0xff },
            _pad_2d: [0; 3],
        };

        unsafe { native::set_content(self, &params) }
    }

    /// Same as the game's group switch does for the eye list (group 3).
    pub unsafe fn set_enabled(&mut self, enable: bool, mode: u32) {
        unsafe {
            if !enable {
                native::disable(self, mode);
            } else if self.state_machine.current_state > 1 {
                let params = EnableParams {
                    vtable: got_vtable(GOT_ENABLE_PARAMS_VTABLE),
                    update_cursor_button: false,
                    cursor_button_arg0: 1,
                    cursor_button_arg1: 0,
                    focus_cursor: true,
                    cursor_item: -1,
                };

                native::enable(self, &params);
            } else if self.is_ready_to_play() {
                self.state_machine.change_state(STATE_PLAY);
            }
        }
    }

    unsafe fn is_ready_to_play(&self) -> bool {
        unsafe {
            self.slot_count != 0
                && self.button_count != 0
                && self.unk_270.as_ref().is_some_and(|slots| !slots[1].is_null())
                && self.input_claim.as_ref().is_none_or(|claim| claim.unk_14 != -1)
                && !self.focus_manager.is_null()
                && self.unk_3d8 > 0
                && self.padded_item_count != 0
        }
    }

    /// What the list does itself when focus leaves it (State_710375F2AC::cPlay_Exec): forget the
    /// focused slot and pause with its buttons still focusable, so it stops taking the D-pad and
    /// navigation back into it goes through the links.
    pub unsafe fn pause(&mut self) {
        self.focused_slot = -1;

        if self.state_machine.current_state == STATE_PLAY {
            self.pause_button_mode = PAUSE_KEEP_BUTTONS;

            unsafe { self.state_machine.change_state(STATE_PAUSE) };
        }
    }

    /// Pausing keeps only the line focus last left from enabled (vanilla re-enters a list where it
    /// was left). Here neighbours link into every visible line, so keep all of them focusable.
    pub unsafe fn enable_visible_buttons(&mut self) {
        let line_size = self.line_size.max(1);
        let visible = self.first_line..self.first_line + self.visible_lines;

        for slot in 0..self.button_count() {
            let Some(button) = (unsafe { self.button(slot).as_mut() }) else {
                continue;
            };

            let item = unsafe { self.item_index_of(button.as_control()) };

            if item >= 0 && item < self.padded_item_count && visible.contains(&(item / line_size)) {
                unsafe { (*button.as_control()).set_focusable(self.player, true) };
            }
        }
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

    pub unsafe fn item_index_of(&mut self, control: *mut Control) -> i32 {
        unsafe { native::item_index_of(self, control) }
    }
}

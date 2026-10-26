use core::ffi::c_void;
use core::mem::offset_of;

use crate::lp::ui::FocusNode;
use crate::sead::{Delegate, ListNode, OffsetList};

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonId {
    Invalid = -1,
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
    A = 4,
    B = 5,
    X = 6,
    Y = 7,
    L = 8,
    R = 9,
    Zl = 10,
    Zr = 11,
    LeftStick = 12,
    RightStick = 13,
    Plus = 14,
    Minus = 15,
}

unsafe impl crate::native::Abi for ButtonId {
    type Abi = nookkit_sym::abi::I32;
}

impl ButtonId {
    pub fn from_raw(raw: i32) -> Option<Self> {
        Some(match raw {
            -1 => Self::Invalid,
            0 => Self::Up,
            1 => Self::Down,
            2 => Self::Left,
            3 => Self::Right,
            4 => Self::A,
            5 => Self::B,
            6 => Self::X,
            7 => Self::Y,
            8 => Self::L,
            9 => Self::R,
            10 => Self::Zl,
            11 => Self::Zr,
            12 => Self::LeftStick,
            13 => Self::RightStick,
            14 => Self::Plus,
            15 => Self::Minus,
            _ => return None,
        })
    }
}

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyEventKind {
    Trigger = 0,
    Release = 1,

    /// The key was taken by an earlier handler, or the handler was disabled, while held
    Cancel = 2,

    /// At press, after `InputRoot::repeat_delay` frames, then every `repeat_interval`
    Repeat = 3,

    /// Every held frame
    Hold = 4,
}

impl KeyEventKind {
    pub const COUNT: usize = 5;

    pub fn from_raw(raw: u32) -> Option<Self> {
        Some(match raw {
            0 => Self::Trigger,
            1 => Self::Release,
            2 => Self::Cancel,
            3 => Self::Repeat,
            4 => Self::Hold,
            _ => return None,
        })
    }
}

pub const KEY_STATE_READY: i32 = -1;
pub const KEY_STATE_BLOCKED: i32 = -2;

#[repr(C)]
pub struct KeyHandler {
    pub list_node: ListNode,
    pub player: u32,
    pub key: ButtonId,

    /// Indexed by [`KeyEventKind`], invoked with `(kind, *const KeyEvent)`
    pub event_delegates: [Delegate; KeyEventKind::COUNT],

    /// [`KEY_STATE_READY`], [`KEY_STATE_BLOCKED`], -3 cancelled while held, or a repeat countdown
    pub state: i32,

    pub enabled: bool,

    /// Adds `key` to the player's consumed keys every frame, so later handlers skip it
    pub consume_key: bool,

    pub consume_all: bool,
    _pad_bf: u8,
    pub anim_tag: *mut c_void,
}

const _: () = {
    assert!(offset_of!(KeyHandler, player) == 0x10);
    assert!(offset_of!(KeyHandler, event_delegates) == 0x18);
    assert!(offset_of!(KeyHandler, state) == 0xb8);
    assert!(offset_of!(KeyHandler, enabled) == 0xbc);
    assert!(offset_of!(KeyHandler, anim_tag) == 0xc0);
    assert!(core::mem::size_of::<KeyHandler>() == 0xc8);
};

impl KeyHandler {
    pub const fn new(player: u32, key: ButtonId) -> Self {
        Self {
            list_node: ListNode {
                prev: core::ptr::null_mut(),
                next: core::ptr::null_mut(),
            },
            player,
            key,
            event_delegates: [const { Delegate::empty() }; KeyEventKind::COUNT],
            state: KEY_STATE_BLOCKED,
            enabled: true,
            consume_key: false,
            consume_all: false,
            _pad_bf: 0,
            anim_tag: core::ptr::null_mut(),
        }
    }
}

#[repr(C)]
pub struct PlayerInputState {
    pub held_keys: u32,
    pub sticks: [[f32; 2]; 4],
    pub pointer_down: bool,
    pub pointer_flag_25: u8,
    _pad_26: [u8; 0x2],
    pub pointer_pos: [i32; 2],
    pub pointer_pos_2: [i32; 2],
    _unk_38: [u8; 0x4],
    pub consumed_keys: u32,
    pub consumed_analog: u32,
    pub consumed_all: bool,
    _pad_45: [u8; 0x3],
}

const _: () = {
    assert!(offset_of!(PlayerInputState, consumed_keys) == 0x3c);
    assert!(core::mem::size_of::<PlayerInputState>() == 0x48);
};

impl PlayerInputState {
    pub fn is_held(&self, key: ButtonId) -> bool {
        key != ButtonId::Invalid && self.held_keys & (1 << key as u32) != 0
    }
}

#[repr(C)]
pub struct InputFrame {
    _unk_0: [u8; 0x8],
    pub player_count: u32,
    _pad_c: u32,
    pub players: *mut PlayerInputState,
}

impl InputFrame {
    pub unsafe fn player(&self, player: u32) -> Option<&PlayerInputState> {
        let index = if player < self.player_count {
            player
        } else {
            0
        };

        unsafe {
            self.players
                .as_ref()
                .map(|_| &*self.players.add(index as usize))
        }
    }
}

#[repr(C)]
pub struct KeyEvent {
    pub handler: *mut KeyHandler,
    pub input: *const *const InputFrame,
}

#[repr(C)]
pub struct InputRoot {
    pub pointer_handlers: OffsetList,
    pub key_handlers: OffsetList,
    pub analog_handlers: OffsetList,
    pub focus_navigator: *mut FocusNavigator,

    /// Players whose input this root takes in the main pass
    pub swallow_input_player_mask: u32,

    /// Set: the root is inactive, its handlers keep state but do not fire
    pub disabled: bool,
    _pad_55: [u8; 0x3],
    /// Inactive while it overlaps the UIMgr inhibited-context mask (UIMgr+0x418 -> +0x10)
    pub context_mask: u32,
    pub repeat_delay: i32,
    pub repeat_interval: i32,
    pub use_own_repeat: bool,
    _pad_65: [u8; 0x3],
    pub pre_input_delegate: Delegate,
    pub post_input_delegate: Delegate,
}

const _: () = {
    assert!(offset_of!(InputRoot, key_handlers) == 0x18);
    assert!(offset_of!(InputRoot, focus_navigator) == 0x48);
    assert!(offset_of!(InputRoot, repeat_delay) == 0x5c);
    assert!(offset_of!(InputRoot, pre_input_delegate) == 0x68);
    assert!(core::mem::size_of::<InputRoot>() == 0xa8);
};

/// One player's place in a focus navigator
#[repr(C)]
pub struct FocusCursor {
    _unk_0: [u8; 0x10],
    pub node: *mut FocusNode,
}

#[repr(C)]
pub struct FocusNavigator {
    _unk_0: [u8; 0x18],
    pub cursor_count: u32,
    _unk_1c: [u8; 0x4],
    pub cursors: *mut *mut FocusCursor,
}

const _: () = {
    assert!(offset_of!(FocusCursor, node) == 0x10);
    assert!(offset_of!(FocusNavigator, cursor_count) == 0x18);
    assert!(offset_of!(FocusNavigator, cursors) == 0x20);
};

impl FocusNavigator {
    /// The node `player` has focused, null for none
    pub unsafe fn focused(&self, player: u32) -> *mut FocusNode {
        if player >= self.cursor_count || self.cursors.is_null() {
            return core::ptr::null_mut();
        }

        unsafe { (*self.cursors.add(player as usize)).as_ref() }
            .map_or(core::ptr::null_mut(), |cursor| cursor.node)
    }
}

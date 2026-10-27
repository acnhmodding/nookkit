#![no_std]

use core::ffi::{c_char, c_void};

pub const API_NAME: &core::ffi::CStr = c"nookkit";
pub const API_VERSION: u32 = 2;

unsafe extern "C" {
    pub fn sl_api_publish(name: *const c_char, version: u32, vtable: *const c_void);
    pub fn sl_api_get(name: *const c_char, version: u32) -> *const c_void;
}

// The ABI's enums are plain integers so a newer side's values are never UB for an older one
macro_rules! code {
    ($(#[$attr:meta])* $name:ident($repr:ty) { $($(#[$vattr:meta])* $variant:ident = $value:expr),* $(,)? }) => {
        $(#[$attr])*
        #[repr(transparent)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub $repr);

        impl $name {
            $($(#[$vattr])* pub const $variant: Self = Self($value);)*
        }
    };
}

code! {
    Status(i32) {
        OK = 0,
        INVALID_ARG = 1,
        PANE_NOT_FOUND = 2,
        TYPE_MISMATCH = 3,
        NO_EVENT_TABLE = 4,
        EVENT_TABLE_FULL = 5,
        LOCK_FAILED = 6,
        MISSING_SYMBOL = 7,
        UNSUPPORTED_BUILD = 8,

        /// The page behind a handle was destroyed
        STALE = 9,
    }
}

code! {
    Direction(u32) {
        UP = 0,
        DOWN = 1,
        LEFT = 2,
        RIGHT = 3,
    }
}

code! {
    ControlEventKind(u32) {
        FOCUS = 0,
        DECIDE = 5,
    }
}

code! {
    SeqEvent(u32) {
        READY = 0,
        ACTIVATE = 1,
        DEACTIVATE = 2,
        UPDATE = 3,
    }
}

impl SeqEvent {
    pub const COUNT: usize = 4;
}

code! {
    StatePhase(u32) {
        ENTER = 0,
        EXEC = 1,
        EXIT = 2,
    }
}

impl StatePhase {
    pub const COUNT: usize = 3;
}

code! {
    ListEvent(u32) {
        ITEM = 0,
        FOCUS = 1,
        DECIDE = 2,
        LEAVE = 3,
    }
}

impl ListEvent {
    pub const COUNT: usize = 4;
}

code! {
    ItemOp(u32) {
        RENDER = 0,
        MOVE = 1,
        CLEAR = 2,
    }
}

code! {
    KeyEventKind(u32) {
        TRIGGER = 0,
        RELEASE = 1,
        CANCEL = 2,
        REPEAT = 3,
        HOLD = 4,
    }
}

impl KeyEventKind {
    pub const COUNT: usize = 5;
}

code! {
    KeyFlags(u32) {
        NONE = 0,

        /// Takes the key from the page's own handlers
        CONSUME = 1 << 0,

        /// The binding takes its player from the page's seq each time the seq activates
        SEQ_PLAYER = 1 << 1,
    }
}

impl KeyFlags {
    pub const fn contains(self, other: Self) -> bool {
        self.0 & other.0 == other.0
    }
}

impl core::ops::BitOr for KeyFlags {
    type Output = Self;

    fn bitor(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

code! {
    SymbolKind(u32) {
        FN = 0,
        VTABLE = 1,
    }
}

/// A local player, 0 to 7
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Player(pub u8);

impl Player {
    pub const COUNT: u8 = 8;

    pub const fn new(index: u8) -> Option<Self> {
        if index < Self::COUNT {
            Some(Self(index))
        } else {
            None
        }
    }

    pub const fn is_valid(self) -> bool {
        self.0 < Self::COUNT
    }
}

/// A page as one build of it, so a handle to a destroyed page fails with `STALE` even when a new
/// page lands at the same address
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Handle {
    pub raw: *mut c_void,
    pub build: u64,
}

impl Handle {
    pub const NULL: Self = Self {
        raw: core::ptr::null_mut(),
        build: 0,
    };
}

/// A control of a page, valid while the page is
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PaneHandle {
    pub page: Handle,
    pub raw: *mut c_void,
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

#[repr(C)]
pub struct ControlEvent {
    pub page: Handle,
    pub control: *mut c_void,
    pub kind: ControlEventKind,
}

#[repr(C)]
pub struct ItemUpdate {
    pub op: ItemOp,
    pub slot: u32,
    pub from_slot: u32,

    /// -1 for `ItemOp::CLEAR`
    pub item: i32,
}

pub const NO_LINE: u32 = u32::MAX;

#[repr(C)]
pub struct KeyEvent {
    pub kind: KeyEventKind,
    pub player: Player,

    /// ButtonId
    pub key: i32,
}

#[repr(C)]
pub struct LeaveQuery {
    pub direction: Direction,

    /// Visible line focus leaves from, `NO_LINE` if unknown
    pub line: u32,

    /// Set to the control focus moves to
    pub target: *mut *mut c_void,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ListTarget {
    pub line: u32,

    /// Last button of the line instead of the first
    pub last: bool,
}

#[repr(C)]
pub struct NookKitV2 {
    pub size: usize,
    pub framework_version: u32,

    /// `false` when the running game build isn't the supported one, every other entry then fails
    pub started: unsafe extern "C" fn() -> bool,

    /// Absolute address of a symbol in the framework's table, 0 when missing
    pub resolve: unsafe extern "C" fn(name: Str, kind: SymbolKind) -> usize,

    /// Empty `layout` or `class` matches any. `cb` gets a `*const Handle` after the page is built
    pub on_page_built: unsafe extern "C" fn(layout: Str, class: Str, cb: Callback) -> Status,

    /// `STALE` once the page is destroyed
    pub page_check: unsafe extern "C" fn(page: Handle) -> Status,

    /// Empty `class` skips the type check
    pub find_pane: unsafe extern "C" fn(
        page: Handle,
        path: Str,
        class: Str,
        out: *mut *mut c_void,
    ) -> Status,

    /// `cb` gets a `*const ControlEvent`. Dropped with the page
    pub on_control_event:
        unsafe extern "C" fn(page: Handle, kind: ControlEventKind, cb: Callback) -> Status,

    /// A null `to.raw` unlinks
    pub link:
        unsafe extern "C" fn(from: PaneHandle, direction: Direction, to: PaneHandle) -> Status,
    pub set_focusable:
        unsafe extern "C" fn(pane: PaneHandle, player: Player, enable: bool) -> Status,
    pub lock_player: unsafe extern "C" fn(player: Player) -> Status,
    pub release_player: unsafe extern "C" fn(player: Player) -> Status,
    pub set_visible: unsafe extern "C" fn(pane: PaneHandle, visible: bool) -> Status,

    /// `cb` gets the seq pointer. Dropped with the page
    pub on_seq_event: unsafe extern "C" fn(page: Handle, kind: SeqEvent, cb: Callback) -> Status,

    pub list_get: unsafe extern "C" fn(page: Handle, path: Str, out: *mut *mut c_void) -> Status,

    /// `cb` gets `*const ItemUpdate`, `*const u32` item for focus and decide, or `*const LeaveQuery`
    pub list_on: unsafe extern "C" fn(list: *mut c_void, kind: ListEvent, cb: Callback) -> Status,

    pub list_set_items: unsafe extern "C" fn(
        list: *mut c_void,
        count: u32,
        selected: u32,
        player: Player,
    ) -> Status,
    pub list_set_enabled:
        unsafe extern "C" fn(list: *mut c_void, enable: bool, mode: u32) -> Status,
    pub list_link_into: unsafe extern "C" fn(
        from: PaneHandle,
        direction: Direction,
        list: *mut c_void,
        target: ListTarget,
    ) -> Status,
    pub list_relink: unsafe extern "C" fn(list: *mut c_void) -> Status,
    pub list_slot_count: unsafe extern "C" fn(list: *mut c_void, out: *mut u32) -> Status,
    pub list_button:
        unsafe extern "C" fn(list: *mut c_void, slot: u32, out: *mut *mut c_void) -> Status,

    /// One binding per page, player and key; binding again returns it. Dropped with the page
    pub key_bind: unsafe extern "C" fn(
        page: Handle,
        player: Player,
        key: i32,
        flags: KeyFlags,
        out: *mut *mut c_void,
    ) -> Status,

    /// `cb` gets a `*const KeyEvent`
    pub key_on: unsafe extern "C" fn(key: *mut c_void, kind: KeyEventKind, cb: Callback) -> Status,

    pub key_set_enabled: unsafe extern "C" fn(key: *mut c_void, enable: bool) -> Status,

    /// Shows the key in the key-guide bar with literal `text`, or a `label` from
    /// LayoutMsg/KeyGuide.msbt when `text` is empty; both empty removes it. Takes effect at the
    /// page's next guide refresh
    pub key_set_guide: unsafe extern "C" fn(key: *mut c_void, text: Str, label: Str) -> Status,

    /// A pane in the page's layout shown while the binding is enabled, empty for none
    pub key_set_hint_pane: unsafe extern "C" fn(key: *mut c_void, path: Str) -> Status,

    /// Drops the game's own key-guide entries for `key` while the page's seq is active
    pub page_hide_guide: unsafe extern "C" fn(page: Handle, key: i32, hide: bool) -> Status,

    /// A disabled page's input root keeps its handlers' state but fires nothing, the game's own
    /// handlers and bindings alike
    pub page_set_input_enabled: unsafe extern "C" fn(page: Handle, enable: bool) -> Status,

    /// Opens the resident pocket menu for `player` over whatever is open
    pub open_pocket_menu: unsafe extern "C" fn(player: Player, close_enabled: bool) -> Status,

    /// The most recently built page of `class` the game hasn't destroyed, `Handle::NULL` for none
    pub find_page: unsafe extern "C" fn(class: Str, out: *mut Handle) -> Status,

    /// The seq that built `page`, null before it is ready
    pub page_seq: unsafe extern "C" fn(page: Handle, out: *mut *mut c_void) -> Status,

    /// The page a seq built, `Handle::NULL` for a seq without a live page
    pub seq_page: unsafe extern "C" fn(seq: *mut c_void, out: *mut Handle) -> Status,

    /// `cb` gets the seq pointer before or `after` the page's seq runs `phase` of `state`. Dropped
    /// with the page
    pub on_state: unsafe extern "C" fn(
        page: Handle,
        state: i32,
        phase: StatePhase,
        after: bool,
        cb: Callback,
    ) -> Status,

    /// The button `player` has focused on the page, null for none
    pub page_focused:
        unsafe extern "C" fn(page: Handle, player: Player, out: *mut *mut c_void) -> Status,

    /// Presses a button as if decided: its Decide animation and the page's decide event, so the
    /// game reacts as to A. `out` is `false` when it was already deciding
    pub pane_decide: unsafe extern "C" fn(pane: PaneHandle, out: *mut bool) -> Status,

    /// The button's valid look, the one the game greys out when an entry can't be used
    pub pane_set_valid: unsafe extern "C" fn(pane: PaneHandle, valid: bool) -> Status,
    pub pane_is_valid: unsafe extern "C" fn(pane: PaneHandle, out: *mut bool) -> Status,
}

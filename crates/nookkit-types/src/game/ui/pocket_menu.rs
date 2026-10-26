use core::mem::offset_of;

use crate::game::ui::{SequenceBase, ViewPage};
use crate::lp::ui::UniteBtnCtrl;
use crate::native::{Class, Fn, NativeResult, sym};

pub const POCKET_SLOTS: usize = 40;

/// Asks Play to latch `command_slot` and open the item's command popup, like pressing A on it
pub const POCKET_EVENT_OPEN_COMMANDS: u16 = 4;

#[repr(C)]
pub struct PocketOpenOptions {
    /// 0 for the normal pockets
    pub mode: u32,
    pub close_enabled: bool,
}

const _: () = assert!(core::mem::size_of::<PocketOpenOptions>() == 8);

crate::native! {
    unsafe fn request_open(player: *const u8, options: *const PocketOpenOptions) = sym::game::UI::PocketMenuSeq::requestOpen;
}

/// Opens the resident pocket menu (UIRoot+0x88) for `player` in its normal mode, over whatever is open
pub unsafe fn open_pocket_menu(player: u8, close_enabled: bool) -> NativeResult<()> {
    let options = PocketOpenOptions {
        mode: 0,
        close_enabled,
    };

    unsafe { request_open(&player, &options) }
}

#[repr(C)]
pub struct PocketMenuPage {
    pub base: ViewPage,
    _unk: [u8; 0x590 - core::mem::size_of::<ViewPage>()],
    pub slots: [*mut UniteBtnCtrl; POCKET_SLOTS],
}

const _: () = assert!(offset_of!(PocketMenuPage, slots) == 0x590);

/// The resident pocket menu, prefix only
#[repr(C)]
pub struct PocketMenuSeq {
    pub base: SequenceBase,
    pub page: *mut PocketMenuPage,
    pub command_select: *mut CommandSelectSeq,
    _unk_220: [u8; 0x1c2d0],

    /// Bit per slot the current mode doesn't allow
    pub blocked_slots: u64,
    _unk_1c4f8: [u8; 0x4],

    /// 0 for the normal pockets
    pub mode: i32,

    pub capacity: u32,
    _unk_1c504: [u8; 0x10],

    /// The slot Play latched for the open command popup
    pub latched_slot: u8,

    pub focused_slot: u8,
    _unk_1c516: [u8; 0x2],
    pub command_slot: u8,
    _unk_1c519: u8,
    pub events: u16,
    _unk_1c51c: u8,
    pub close_enabled: bool,
}

const _: () = {
    assert!(offset_of!(PocketMenuSeq, page) == 0x210);
    assert!(offset_of!(PocketMenuSeq, command_select) == 0x218);
    assert!(offset_of!(PocketMenuSeq, blocked_slots) == 0x1c4f0);
    assert!(offset_of!(PocketMenuSeq, mode) == 0x1c4fc);
    assert!(offset_of!(PocketMenuSeq, capacity) == 0x1c500);
    assert!(offset_of!(PocketMenuSeq, latched_slot) == 0x1c514);
    assert!(offset_of!(PocketMenuSeq, focused_slot) == 0x1c515);
    assert!(offset_of!(PocketMenuSeq, command_slot) == 0x1c518);
    assert!(offset_of!(PocketMenuSeq, events) == 0x1c51a);
    assert!(offset_of!(PocketMenuSeq, close_enabled) == 0x1c51d);
};

crate::derive_base!(PocketMenuSeq => SequenceBase);

unsafe impl Class for PocketMenuSeq {
    const RUNTIME_TYPE_INFO: Fn = sym::game::UI::PocketMenuSeq::getRuntimeTypeInfo.erase();
}

impl PocketMenuSeq {
    pub fn is_blocked(&self, slot: u8) -> bool {
        slot >= 64 || self.blocked_slots & (1 << slot) != 0
    }

    /// The slot's button, null past the page's slots
    pub unsafe fn slot_button(&self, slot: u8) -> *mut UniteBtnCtrl {
        unsafe { self.page.as_ref() }
            .and_then(|page| page.slots.get(slot as usize).copied())
            .unwrap_or(core::ptr::null_mut())
    }
}

#[repr(C)]
pub struct CommandRecord {
    _unk_0: [u8; 0x168],

    /// The command id, e.g. 0x1e put down
    pub command: i32,
    _unk_16c: [u8; 0x3cc],
}

const _: () = assert!(core::mem::size_of::<CommandRecord>() == 0x538);

pub const COMMAND_RECORDS: usize = 7;

/// The pocket item command popup, prefix only
#[repr(C)]
pub struct CommandSelectSeq {
    pub base: SequenceBase,
    _unk_210: [u8; 0x28],
    pub records: [CommandRecord; COMMAND_RECORDS],
    _unk_26c0: [u8; 0x9],

    /// Buttons count from the last record, 0xff for none
    pub selected_button: u8,
    _unk_26ca: [u8; 0xe],
    pub count: u32,
}

const _: () = {
    assert!(offset_of!(CommandSelectSeq, records) == 0x238);
    assert!(offset_of!(CommandSelectSeq, selected_button) == 0x26c9);
    assert!(offset_of!(CommandSelectSeq, count) == 0x26d8);
};

crate::derive_base!(CommandSelectSeq => SequenceBase);

unsafe impl Class for CommandSelectSeq {
    const RUNTIME_TYPE_INFO: Fn = sym::game::UI::CommandSelectSeq::getRuntimeTypeInfo.erase();
}

impl CommandSelectSeq {
    /// The command ids in record order, empty when the count is out of range
    pub fn commands(&self) -> impl Iterator<Item = i32> + '_ {
        self.records
            .get(..self.count as usize)
            .unwrap_or_default()
            .iter()
            .map(|record| record.command)
    }

    /// Selects the record at `index`, `false` past the records
    pub fn choose(&mut self, index: usize) -> bool {
        let count = self.count as usize;
        if index >= count || count > COMMAND_RECORDS {
            return false;
        }

        self.selected_button = (count - 1 - index) as u8;
        true
    }

    /// Selects no command, so the popup closes without running one
    pub fn cancel(&mut self) {
        self.selected_button = 0xff;
    }
}

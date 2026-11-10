use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use crate::native::{NativeResult, data, sym};

include!(concat!(env!("OUT_DIR"), "/save.rs"));

/// Binds a save buffer to one schema struct, its payload is at `buffer + payload_offset`. Nodes form a
/// tree mirroring the schema, binding a buffer or guard walks `first_child` and each `next_sibling`
#[repr(C)]
pub struct SaveIoNode {
    _vtable: *const c_void,
    pub buffer: *mut u8,
    pub payload_offset: usize,
    pub first_child: *mut SaveIoNode,
    pub next_sibling: *mut SaveIoNode,
    /// Callbacks run around payload reads and writes
    pub guard: *mut c_void,
    pub state: u8,
}

const _: () = {
    assert!(size_of::<SaveIoNode>() == 0x38);
    assert!(offset_of!(SaveIoNode, buffer) == 0x8);
    assert!(offset_of!(SaveIoNode, payload_offset) == 0x10);
    assert!(offset_of!(SaveIoNode, first_child) == 0x18);
    assert!(offset_of!(SaveIoNode, next_sibling) == 0x20);
    assert!(offset_of!(SaveIoNode, guard) == 0x28);
    assert!(offset_of!(SaveIoNode, state) == 0x30);
};

impl SaveIoNode {
    /// Null until a buffer is bound
    pub fn payload(&self) -> *mut u8 {
        if self.buffer.is_null() {
            return null_mut();
        }

        self.buffer.wrapping_add(self.payload_offset)
    }
}

/// Owns the SaveMain buffers, only reached through its functions
#[repr(C)]
pub struct SaveMgr {
    _opaque: [u8; 0],
}

crate::native! {
    impl SaveMgr {
        /// The SaveLand node, or the replaceable buffer's while one is in use, null before a save loads
        pub unsafe fn land_node(&mut self) -> *mut SaveIoNode = sym::game::SaveMgr::getLandNode;
    }
}

impl SaveMgr {
    /// Null until the game creates it
    pub fn instance() -> NativeResult<*mut SaveMgr> {
        Ok(unsafe { *data::<*mut SaveMgr>(&sym::game::SaveMgr::sInstance)? })
    }

    /// Null before a save loads
    pub unsafe fn land(&mut self) -> NativeResult<*mut SaveLand> {
        let node = unsafe { self.land_node()? };

        if node.is_null() {
            return Ok(null_mut());
        }

        Ok(unsafe { (*node).payload() }.cast())
    }
}

/// A local player's SavePlayer and SavePlayerOther node trees, each root followed by its descendants
#[repr(C)]
pub struct PlayerSaveSlot {
    pub player: SaveIoNode,
    _player_nodes: [u8; 0x19478],
    pub other: SaveIoNode,
    _other_nodes: [u8; 0x2d330],
}

const _: () = {
    assert!(size_of::<PlayerSaveSlot>() == 0x46818);
    assert!(offset_of!(PlayerSaveSlot, other) == 0x194b0);
};

#[repr(C, align(16))]
pub struct PlayerSaveMgr {
    // Set to 0xff on init
    _unk_0: [u8; 0x20],
    // Per-player runtime objects, not save data
    _unk_20: [[u8; 0x740]; 8],
    /// What init binds `slots[i].player` to, read through `player` instead
    pub players: [SavePlayer; 8],
    /// What init binds every `slots[i].other` to
    pub other: SavePlayerOther,
    pub slots: [PlayerSaveSlot; 8],
    pub available_player_mask: u32,
    _unk_431fc4: [u8; 0x4c],
    _disposer: [u8; 0x20],
}

const _: () = {
    assert!(size_of::<PlayerSaveMgr>() == 0x432030);
    assert!(offset_of!(PlayerSaveMgr, players) == 0x3a20);
    assert!(offset_of!(PlayerSaveMgr, other) == 0x1c10a0);
    assert!(offset_of!(PlayerSaveMgr, slots) == 0x1fdf00);
    assert!(offset_of!(PlayerSaveMgr, available_player_mask) == 0x431fc0);
    assert!(offset_of!(PlayerSaveMgr, _disposer) == 0x432010);
};

// What 451 of getPlayerSlot's 490 callers pass
const DEFAULT_LOOKUP: i32 = 1;

crate::native! {
    impl PlayerSaveMgr {
        /// Local players index `slots`, remote and share-play ones are redirected to their own buffers
        pub unsafe fn player_slot(&mut self, player: *const u8, lookup: i32) -> *mut PlayerSaveSlot = sym::game::PlayerSaveMgr::getPlayerSlot;
    }
}

impl PlayerSaveMgr {
    /// Null until the game creates it
    pub fn instance() -> NativeResult<*mut PlayerSaveMgr> {
        Ok(unsafe { *data::<*mut PlayerSaveMgr>(&sym::game::PlayerSaveMgr::sInstance)? })
    }

    /// `player`'s save as the game looks it up, null when it has none
    pub unsafe fn player(&mut self, player: u8) -> NativeResult<*mut SavePlayer> {
        let slot = unsafe { self.player_slot(&raw const player, DEFAULT_LOOKUP)? };

        if slot.is_null() {
            return Ok(null_mut());
        }

        Ok(unsafe { (*slot).player.payload() }.cast())
    }

    /// Whether local slot `index` holds a player
    pub fn is_available(&self, index: u8) -> bool {
        index < 8 && self.available_player_mask >> index & 1 != 0
    }
}

/// A fixed char16 buffer such as a name, read up to its first zero
pub trait SaveText {
    fn units(&self) -> &[u16];

    fn units_mut(&mut self) -> &mut [u16];

    /// Unpaired surrogates become U+FFFD
    fn to_string_lossy(&self) -> String {
        let units = self.units();

        let len = match units.iter().position(|&unit| unit == 0) {
            Some(len) => len,
            None => units.len(),
        };

        char::decode_utf16(units.iter().take(len).copied())
            .map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect()
    }

    /// False, leaving the buffer as it was, when `text` doesn't fit or holds a zero
    fn set(&mut self, text: &str) -> bool {
        let units = self.units_mut();

        if text.encode_utf16().count() > units.len() || text.contains('\0') {
            return false;
        }

        units.fill(0);

        for (unit, value) in units.iter_mut().zip(text.encode_utf16()) {
            *unit = value;
        }

        true
    }
}

macro_rules! bit_flag {
    ($($ty:ident($int:ty)),*) => {
        $(
            impl $ty {
                /// False past the last bit
                pub fn is_on_bit(self, bit: u32) -> bool {
                    match <$int>::checked_shl(1, bit) {
                        Some(mask) => self.0 & mask != 0,
                        None => false,
                    }
                }

                /// Ignored past the last bit
                pub fn change_bit(&mut self, bit: u32, on: bool) {
                    let Some(mask) = <$int>::checked_shl(1, bit) else {
                        return;
                    };

                    if on {
                        self.0 |= mask;
                    } else {
                        self.0 &= !mask;
                    }
                }
            }
        )*
    };
}

bit_flag!(BitFlag8(u8), BitFlag16(u16), BitFlag32(u32), BitFlag64(u64));

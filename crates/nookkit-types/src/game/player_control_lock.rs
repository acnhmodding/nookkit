use core::ffi::c_void;

use crate::native::{NativeResult, data, sym};

// What FieldMenuRequestSeq::tryLockPlayer uses for a plain menu (UI id 1)
const MENU_CATEGORY: i32 = 4;
const MENU_LOCK: i32 = 4;

crate::native! {
    unsafe fn is_player_locked(player: *const u8, unk: i32) -> bool = sym::game::PlayerControlLockMgr::isPlayerLocked;

    unsafe fn begin_category(category: i32, player: *const u8, param: u32) -> bool = sym::game::PlayerAction::beginCategory;

    unsafe fn acquire(mgr: *mut c_void, player: *const u8, owner: i64, mode: i32) -> bool = sym::game::PlayerControlLockMgr::acquire;

    unsafe fn release_menu_lock(player: *const u8) -> bool = sym::game::PlayerControlLockMgr::releaseMenuLock;
}

// An opening menu's freeze without tryLockPlayer's "can a menu open now" check. Only from UI
// update time, mid actor-update the player reads as invalid and the action is refused
pub unsafe fn lock_menu(player: u8) -> NativeResult<bool> {
    unsafe {
        if is_player_locked(&player, 0)? || !begin_category(MENU_CATEGORY, &player, 0)? {
            return Ok(false);
        }

        let mgr = *data::<*mut c_void>(&sym::game::PlayerControlLockMgr::sInstance)?;
        if mgr.is_null() {
            return Ok(false);
        }

        acquire(mgr, &player, -1, MENU_LOCK)
    }
}

pub unsafe fn release_menu(player: u8) -> NativeResult<bool> {
    Ok(unsafe { release_menu_lock(&player)? })
}

use crate::native::{NativeResult, sym};

// The plain menu lock, PlayerControlLockMgr mode 4
const UI_ID_MENU: i32 = 1;

crate::native! {
    unsafe fn try_lock_player(ui_id: i32, player: *const u8) -> bool = sym::game::UI::FieldMenuRequestSeq::tryLockPlayer;

    unsafe fn release_menu_lock(player: *const u8) -> bool = sym::game::PlayerControlLockMgr::releaseMenuLock;
}

/// `false` when the player is already locked or the current context disallows menus
pub unsafe fn lock_menu(player: u8) -> NativeResult<bool> {
    Ok(unsafe { try_lock_player(UI_ID_MENU, &player)? })
}

pub unsafe fn release_menu(player: u8) -> NativeResult<bool> {
    Ok(unsafe { release_menu_lock(&player)? })
}

use native_vtable::native;

use crate::native::NativeResult;

// The plain menu lock, PlayerControlLockMgr mode 4
const UI_ID_MENU: i32 = 1;

#[native(resolve = crate::native::resolve)]
unsafe extern "C" {
    #[native(symbol = "game::UI::FieldMenuRequestSeq::tryLockPlayer")]
    fn try_lock_player(ui_id: i32, player: *const u8) -> bool;

    #[native(symbol = "game::PlayerControlLockMgr::releaseMenuLock")]
    fn release_menu_lock(player: *const u8) -> bool;
}

/// `false` when the player is already locked or the current context disallows menus
pub unsafe fn lock_menu(player: u8) -> NativeResult<bool> {
    Ok(unsafe { try_lock_player(UI_ID_MENU, &player)? })
}

pub unsafe fn release_menu(player: u8) -> NativeResult<bool> {
    Ok(unsafe { release_menu_lock(&player)? })
}

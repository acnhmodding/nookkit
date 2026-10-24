use core::ffi::c_void;

use native_vtable::native;

use crate::native::{NativeResult, data};

const MODE_MENU: u32 = 4;

#[native(resolve = crate::native::resolve)]
unsafe extern "C" {
    #[native(symbol = "game::PlayerControlLockMgr::acquire")]
    fn acquire(mgr: *mut c_void, player: *const u8, owner: i64, mode: u32) -> bool;

    #[native(symbol = "game::PlayerControlLockMgr::releaseMenuLock")]
    fn release_menu_lock(player: *const u8) -> bool;
}

pub unsafe fn lock_menu(player: u8) -> NativeResult<bool> {
    unsafe {
        let mgr = *data::<*mut c_void>("ACNH_Singleton_7104CA4AB8::sInstance")?;

        if mgr.is_null() {
            return Ok(false);
        }

        Ok(acquire(mgr, &player, -1, MODE_MENU)?)
    }
}

pub unsafe fn release_menu(player: u8) -> NativeResult<bool> {
    Ok(unsafe { release_menu_lock(&player)? })
}

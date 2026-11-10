use core::sync::atomic::{AtomicUsize, Ordering};

use nookkit_sys::Player;
use nookkit_types::game::save::{PlayerSaveMgr, SaveLand, SaveMgr, SavePlayer};

use crate::{Result, Ui, UiError};

impl Ui {
    /// Runs `f` on the loaded island's live save data. Only from the game's thread, e.g. a page callback
    pub fn with_land<R>(&self, f: impl FnOnce(&mut SaveLand) -> R) -> Result<R> {
        let mgr = SaveMgr::instance().map_err(|_| UiError::MissingSymbol)?;

        let Some(mgr) = (unsafe { mgr.as_mut() }) else {
            return Err(UiError::SaveNotLoaded);
        };

        let land = unsafe { mgr.land() }.map_err(|_| UiError::MissingSymbol)?;

        unsafe { lend(land, f) }
    }

    /// Runs `f` on `player`'s save data, as the game looks it up. Only from the game's thread
    pub fn with_player_save<R>(
        &self,
        player: Player,
        f: impl FnOnce(&mut SavePlayer) -> R,
    ) -> Result<R> {
        if !player.is_valid() {
            return Err(UiError::InvalidArg);
        }

        let mgr = player_save_mgr()?;
        let save = unsafe { mgr.player(player.0) }.map_err(|_| UiError::MissingSymbol)?;

        unsafe { lend(save, f) }
    }

    /// The local players that have save data, e.g. the island's residents
    pub fn players_with_save(&self) -> Result<Vec<Player>> {
        let mgr = player_save_mgr()?;
        let mut players = Vec::new();

        for index in 0..Player::COUNT {
            if mgr.is_available(index) {
                players.push(Player(index));
            }
        }

        Ok(players)
    }
}

fn player_save_mgr() -> Result<&'static mut PlayerSaveMgr> {
    let mgr = PlayerSaveMgr::instance().map_err(|_| UiError::MissingSymbol)?;

    match unsafe { mgr.as_mut() } {
        Some(mgr) => Ok(mgr),
        None => Err(UiError::SaveNotLoaded),
    }
}

// Payloads lent to a `with_*` callback, so a nested call can't hand out a second `&mut` to one
static LENT: [AtomicUsize; 4] = [const { AtomicUsize::new(0) }; 4];

struct Loan(&'static AtomicUsize);

impl Loan {
    fn take(addr: usize) -> Result<Self> {
        for slot in &LENT {
            if slot.load(Ordering::Acquire) == addr {
                return Err(UiError::SaveBorrowed);
            }
        }

        for slot in &LENT {
            if slot
                .compare_exchange(0, addr, Ordering::AcqRel, Ordering::Acquire)
                .is_ok()
            {
                return Ok(Self(slot));
            }
        }

        Err(UiError::SaveBorrowed)
    }
}

impl Drop for Loan {
    fn drop(&mut self) {
        self.0.store(0, Ordering::Release);
    }
}

// A misaligned payload means the schema doesn't fit this build
unsafe fn lend<T, R>(ptr: *mut T, f: impl FnOnce(&mut T) -> R) -> Result<R> {
    if ptr.is_null() {
        return Err(UiError::SaveNotLoaded);
    }

    if !ptr.is_aligned() {
        return Err(UiError::UnsupportedBuild);
    }

    let _loan = Loan::take(ptr.addr())?;

    Ok(f(unsafe { &mut *ptr }))
}

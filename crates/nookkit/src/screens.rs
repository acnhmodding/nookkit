//! Typed views of game screens' own state, for what the page API doesn't cover. Get one with
//! [`Seq::downcast`], which checks the seq's class with the game's RTTI

use nookkit_types::game::ui::{
    CommandSelectSeq, POCKET_EVENT_OPEN_COMMANDS, PocketMenuSeq, StorageSeq,
};
use nookkit_types::game::ui_sys::Sequence;
use nookkit_types::native::Class;

use crate::{Result, Seq, UiError};

pub trait Screen: Sized {
    #[doc(hidden)]
    fn wrap(seq: Seq) -> Result<Option<Self>>;
}

impl Seq {
    /// `None` when the seq isn't a `T`
    pub fn downcast<T: Screen>(&self) -> Result<Option<T>> {
        T::wrap(*self)
    }
}

fn is<T: Class>(seq: Seq) -> Result<bool> {
    seq.check()?;

    let Some(base) = (unsafe { seq.raw().cast::<Sequence>().as_mut() }) else {
        return Ok(false);
    };

    let cast = unsafe { base.cast::<T>() }.map_err(|_| UiError::MissingSymbol)?;

    Ok(cast.is_some())
}

macro_rules! screen {
    ($(#[$attr:meta])* $name:ident => $native:ty) => {
        $(#[$attr])*
        #[derive(Clone, Copy)]
        pub struct $name(Seq);

        impl Screen for $name {
            fn wrap(seq: Seq) -> Result<Option<Self>> {
                Ok(is::<$native>(seq)?.then_some(Self(seq)))
            }
        }

        impl $name {
            pub fn seq(&self) -> Seq {
                self.0
            }

            // Checked against the class in wrap and against its page here, then only borrowed for
            // the one access
            fn native(&self) -> Result<*mut $native> {
                self.0.check()?;

                Ok(self.0.raw().cast())
            }
        }
    };
}

screen! {
    /// The pocket menu, resident and reused for every open
    PocketMenu => PocketMenuSeq
}

screen! {
    /// The item command popup the pocket menu opens, e.g. Eat / Drop / Plant
    CommandSelect => CommandSelectSeq
}

screen! {
    /// The storage menu
    Storage => StorageSeq
}

impl PocketMenu {
    /// 0 for the normal pockets, others for selling, donating, …
    pub fn mode(&self) -> Result<i32> {
        Ok(unsafe { (*self.native()?).mode })
    }

    pub fn capacity(&self) -> Result<u32> {
        Ok(unsafe { (*self.native()?).capacity })
    }

    pub fn focused_slot(&self) -> Result<u8> {
        Ok(unsafe { (*self.native()?).focused_slot })
    }

    /// The slot whose command popup is open
    pub fn latched_slot(&self) -> Result<u8> {
        Ok(unsafe { (*self.native()?).latched_slot })
    }

    /// The current mode doesn't allow the slot
    pub fn is_blocked(&self, slot: u8) -> Result<bool> {
        Ok(unsafe { (*self.native()?).is_blocked(slot) })
    }

    /// The slot's button looks usable, not greyed out
    pub fn is_slot_valid(&self, slot: u8) -> Result<bool> {
        let button = unsafe { (*self.native()?).slot_button(slot) };

        Ok(unsafe { button.as_ref() }.is_some_and(|b| b.disp_valid & 1 != 0))
    }

    /// Requests already queued for Play, 0 for none
    pub fn events(&self) -> Result<u16> {
        Ok(unsafe { (*self.native()?).events })
    }

    /// Opens the slot's command popup on Play's next step, as A does. `false` while another
    /// request is queued
    pub fn open_commands(&self, slot: u8) -> Result<bool> {
        let native = self.native()?;

        unsafe {
            if (*native).events != 0 {
                return Ok(false);
            }

            (*native).command_slot = slot;
            (*native).events = POCKET_EVENT_OPEN_COMMANDS;
        }

        Ok(true)
    }

    pub fn command_select(&self) -> Result<Option<CommandSelect>> {
        let raw = unsafe { (*self.native()?).command_select };

        match self.0.page().ui().seq_at(raw.cast())? {
            Some(seq) => seq.downcast(),
            None => Ok(None),
        }
    }
}

impl CommandSelect {
    /// The command ids in order, e.g. 0x1e put down, once Appear has built them
    pub fn commands(&self) -> Result<Vec<i32>> {
        Ok(unsafe { (*self.native()?).commands().collect() })
    }

    /// Picks the command at `index` of [`commands`](Self::commands), `false` past them
    pub fn choose(&self, index: usize) -> Result<bool> {
        Ok(unsafe { (*self.native()?).choose(index) })
    }

    /// Closes without running a command
    pub fn cancel(&self) -> Result {
        unsafe { (*self.native()?).cancel() };

        Ok(())
    }

    /// Drops the built commands, so Appear builds them again from nothing
    pub fn clear(&self) -> Result {
        unsafe { (*self.native()?).count = 0 };

        Ok(())
    }
}

impl Storage {
    /// 0 and 1 are the house's own storage flows
    pub fn mode(&self) -> Result<i32> {
        Ok(unsafe { (*self.native()?).mode })
    }

    /// Requests already queued for Play, bit 12 is X's accent-wall options
    pub fn events(&self) -> Result<u64> {
        Ok(unsafe { (*self.native()?).events })
    }

    pub fn clear_events(&self) -> Result {
        unsafe { (*self.native()?).events = 0 };

        Ok(())
    }

    /// Clears the category arrows' enable flags, as Play_Exit does
    pub fn disable_category_arrows(&self) -> Result {
        unsafe { (*self.native()?).disable_category_arrows() };

        Ok(())
    }
}

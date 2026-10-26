//! Typed views of game screens' own state, for what the page API doesn't cover. Get one with
//! [`Seq::downcast`], which checks the seq's class with the game's RTTI

use nookkit_types::game::ui::{
    CommandSelectSeq, POCKET_EVENT_OPEN_COMMANDS, PocketMenuSeq, StorageSeq,
};
use nookkit_types::game::ui_sys::Sequence;
use nookkit_types::native::Class;

use crate::Seq;

pub trait Screen: Sized {
    #[doc(hidden)]
    fn wrap(seq: Seq) -> Option<Self>;
}

impl Seq {
    /// `None` when the seq isn't a `T`
    pub fn downcast<T: Screen>(&self) -> Option<T> {
        T::wrap(*self)
    }
}

fn is<T: Class>(seq: Seq) -> bool {
    let Some(base) = (unsafe { seq.raw().cast::<Sequence>().as_mut() }) else {
        return false;
    };

    matches!(unsafe { base.cast::<T>() }, Ok(Some(_)))
}

macro_rules! screen {
    ($(#[$attr:meta])* $name:ident => $native:ty) => {
        $(#[$attr])*
        #[derive(Clone, Copy)]
        pub struct $name(Seq);

        impl Screen for $name {
            fn wrap(seq: Seq) -> Option<Self> {
                is::<$native>(seq).then_some(Self(seq))
            }
        }

        impl $name {
            pub fn seq(&self) -> Seq {
                self.0
            }

            // Checked against the class in wrap, and the seq outlives its callbacks
            #[allow(clippy::mut_from_ref)]
            fn native(&self) -> &mut $native {
                unsafe { &mut *self.0.raw().cast::<$native>() }
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
    pub fn mode(&self) -> i32 {
        self.native().mode
    }

    pub fn capacity(&self) -> u32 {
        self.native().capacity
    }

    pub fn focused_slot(&self) -> u8 {
        self.native().focused_slot
    }

    /// The slot whose command popup is open
    pub fn latched_slot(&self) -> u8 {
        self.native().latched_slot
    }

    /// The current mode doesn't allow the slot
    pub fn is_blocked(&self, slot: u8) -> bool {
        self.native().is_blocked(slot)
    }

    /// The slot's button looks usable, not greyed out
    pub fn is_slot_valid(&self, slot: u8) -> bool {
        unsafe { self.native().slot_button(slot).as_ref() }.is_some_and(|b| b.disp_valid & 1 != 0)
    }

    /// Requests already queued for Play, 0 for none
    pub fn events(&self) -> u16 {
        self.native().events
    }

    /// Opens the slot's command popup on Play's next step, as A does. `false` while another
    /// request is queued
    pub fn open_commands(&self, slot: u8) -> bool {
        let native = self.native();
        if native.events != 0 {
            return false;
        }

        native.command_slot = slot;
        native.events = POCKET_EVENT_OPEN_COMMANDS;
        true
    }

    pub fn command_select(&self) -> Option<CommandSelect> {
        let seq = self.native().command_select;
        (!seq.is_null())
            .then(|| Seq::new(seq.cast()).downcast())
            .flatten()
    }
}

impl CommandSelect {
    /// The command ids in order, e.g. 0x1e put down, once Appear has built them
    pub fn commands(&self) -> Vec<i32> {
        self.native().commands().collect()
    }

    /// Picks the command at `index` of [`commands`](Self::commands), `false` past them
    pub fn choose(&self, index: usize) -> bool {
        self.native().choose(index)
    }

    /// Drops the built commands, so Appear builds them again from nothing
    pub fn clear(&self) {
        self.native().count = 0;
    }
}

impl Storage {
    /// 0 and 1 are the house's own storage flows
    pub fn mode(&self) -> i32 {
        self.native().mode
    }

    /// Requests already queued for Play, bit 12 is X's accent-wall options
    pub fn events(&self) -> u64 {
        self.native().events
    }

    pub fn clear_events(&self) {
        self.native().events = 0;
    }

    /// Clears the category arrows' enable flags, as Play_Exit does
    pub fn disable_category_arrows(&self) {
        unsafe { self.native().disable_category_arrows() };
    }
}

#![allow(clippy::missing_safety_doc)]

pub mod game;
pub mod lp;
pub mod native;
pub mod nn;
pub mod sead;

/// `Deref`/`DerefMut` from a class to the `base` field it derives from
#[macro_export]
macro_rules! derive_base {
    ($derived:ty => $base:ty) => {
        impl core::ops::Deref for $derived {
            type Target = $base;

            // Raw, so a packed base works too, since game objects are 8-aligned
            fn deref(&self) -> &$base {
                unsafe { &*(&raw const self.base) }
            }
        }

        impl core::ops::DerefMut for $derived {
            fn deref_mut(&mut self) -> &mut $base {
                unsafe { &mut *(&raw mut self.base) }
            }
        }
    };
}

use skyline::hooks::{Region, getRegionAddress};

pub fn text() -> *mut u8 {
    unsafe { getRegionAddress(Region::Text) as *mut u8 }
}

pub unsafe fn got<T>(offset: usize) -> *mut T {
    unsafe { *(text().add(offset) as *const *mut T) }
}

/// A vtable through a GOT entry pointing at its typeinfo header (offset-to-top, typeinfo).
pub unsafe fn got_vtable<T>(offset: usize) -> *const T {
    unsafe { got::<u8>(offset).add(0x10) as *const T }
}

pub fn text_vtable<T>(offset: usize) -> *const T {
    unsafe { text().add(offset) as *const T }
}

/// A guarded function-local static `sead::RuntimeTypeInfo`. The game initializes it the first time
/// its own code runs the check, so `None` means it hasn't yet.
pub unsafe fn guarded_type_info(info: usize, guard: usize) -> Option<*const core::ffi::c_void> {
    unsafe {
        if *got::<u8>(guard) & 1 == 0 {
            None
        } else {
            Some(got::<core::ffi::c_void>(info))
        }
    }
}

/// Declares wrappers for game functions: `fn name(args) -> ret = offset;`.
macro_rules! native_fns {
    ($( $(#[$meta:meta])* fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? = $offset:literal; )*) => {
        $(
            $(#[$meta])*
            pub(crate) unsafe fn $name($($arg: $ty),*) $(-> $ret)? {
                unsafe {
                    let f: unsafe extern "C" fn($($ty),*) $(-> $ret)? =
                        core::mem::transmute($crate::native::text().add($offset));

                    f($($arg),*)
                }
            }
        )*
    };
}

pub(crate) use native_fns;

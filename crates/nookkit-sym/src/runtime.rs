use std::sync::OnceLock;

use skyline::hooks::{Region, getRegionAddress};

use crate::{FINGERPRINT_LEN, FINGERPRINT_OFFSET, Kind, SymError, SymbolFile, fingerprint, parse};

static TABLE: OnceLock<SymbolFile> = OnceLock::new();

pub fn module_base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}

fn running_fingerprint() -> u64 {
    let window = unsafe {
        core::slice::from_raw_parts(
            (module_base() + FINGERPRINT_OFFSET) as *const u8,
            FINGERPRINT_LEN,
        )
    };
    fingerprint(window)
}

pub fn init(text: &str) -> Result<&'static SymbolFile, SymError> {
    if let Some(table) = TABLE.get() {
        return Ok(table);
    }

    let file = parse(text)?;
    let actual = running_fingerprint();

    if file.rodata_hash != actual {
        return Err(SymError::UnsupportedBuild {
            expected: file.rodata_hash,
            actual,
        });
    }

    Ok(TABLE.get_or_init(|| file))
}

pub fn table() -> Result<&'static SymbolFile, SymError> {
    TABLE.get().ok_or(SymError::NotLoaded)
}

pub fn offset(kind: Kind, name: &str) -> Result<usize, SymError> {
    table()?
        .get(kind, name)
        .map(|o| o as usize)
        .ok_or_else(|| SymError::Missing {
            kind,
            name: name.to_owned(),
        })
}

pub fn address(kind: Kind, name: &str) -> Result<usize, SymError> {
    Ok(module_base() + offset(kind, name)?)
}

pub fn resolve(name: &'static str) -> Option<usize> {
    address(Kind::Fn, name).ok()
}

pub fn fun_addr<S>(symbol: &crate::Fn<S>) -> Result<usize, SymError> {
    address(Kind::Fn, symbol.name)
}

pub fn vtable<T>(symbol: &crate::Vtable) -> Result<*const T, SymError> {
    Ok(address(Kind::Vtable, symbol.name)? as *const T)
}

#[doc(hidden)]
pub use skyline::hooks::A64HookFunction as __hook_function;

/// ```ignore
/// nookkit_sym::hook! {
///     pub view_page_initialize = nookkit_sym::sym::game::UI::ViewPage::initialize;
///     fn(page: *mut c_void) {
///         unsafe { original(page) };
///     }
/// }
/// view_page_initialize::install()?;
/// ```
#[macro_export]
macro_rules! hook {
    (
        $vis:vis $name:ident = $symbol:path;
        fn($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? $body:block
    ) => {
        $vis mod $name {
            #[allow(unused_imports)]
            use super::*;

            const _: () = $crate::check::<_, ($($ty,)*), $crate::__ret_or_unit!($($ret)?)>(&$symbol);

            static ORIGINAL: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);
            static INSTALLED: ::core::sync::atomic::AtomicBool = ::core::sync::atomic::AtomicBool::new(false);

            #[allow(dead_code)]
            pub unsafe fn original($($arg: $ty),*) $(-> $ret)? {
                let f: unsafe extern "C" fn($($ty),*) $(-> $ret)? = unsafe {
                    ::core::mem::transmute(ORIGINAL.load(::core::sync::atomic::Ordering::Acquire))
                };
                unsafe { f($($arg),*) }
            }

            extern "C" fn replacement($($arg: $ty),*) $(-> $ret)? $body

            pub fn install() -> ::core::result::Result<(), $crate::SymError> {
                // Hooking twice would make original() call the replacement
                if INSTALLED.swap(true, ::core::sync::atomic::Ordering::AcqRel) {
                    return Ok(());
                }

                let target = match $crate::fun_addr(&$symbol) {
                    Ok(target) => target,
                    Err(e) => {
                        INSTALLED.store(false, ::core::sync::atomic::Ordering::Release);
                        return Err(e);
                    }
                };

                // Written straight into ORIGINAL, so a call racing the patch never sees 0
                unsafe {
                    $crate::__hook_function(
                        target as *const _,
                        replacement as *const _,
                        ORIGINAL.as_ptr().cast(),
                    );
                }

                if ORIGINAL.load(::core::sync::atomic::Ordering::Acquire) == 0 {
                    return Err($crate::SymError::HookFailed(target));
                }

                Ok(())
            }
        }
    };
}

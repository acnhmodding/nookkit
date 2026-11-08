use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use skyline::hooks::{Region, getRegionAddress};

use crate::{FINGERPRINT_LEN, FINGERPRINT_OFFSET, Kind, SymError, SymbolFile, fingerprint, parse};

static TABLE: OnceLock<SymbolFile> = OnceLock::new();

pub fn module_base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}

// 0 when the window runs past the module's mapped image, as on a smaller executable
fn running_fingerprint() -> u64 {
    let start = module_base() + FINGERPRINT_OFFSET;
    let end = unsafe { getRegionAddress(Region::Bss) as usize };

    if start + FINGERPRINT_LEN > end {
        return 0;
    }

    let window = unsafe { core::slice::from_raw_parts(start as *const u8, FINGERPRINT_LEN) };

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

pub fn data<T>(symbol: &crate::Data) -> Result<*mut T, SymError> {
    Ok(address(Kind::Data, symbol.name)? as *mut T)
}

/// Installs the hook behind `hook!`, once. `original` receives the trampoline to the game's function
#[doc(hidden)]
pub fn __install(
    installed: &AtomicBool,
    original: &AtomicUsize,
    target: Result<usize, SymError>,
    replacement: usize,
) -> Result<(), SymError> {
    // Hooking twice would make original() call the replacement
    if installed.swap(true, Ordering::AcqRel) {
        return Ok(());
    }

    let target = match target {
        Ok(target) => target,
        Err(e) => {
            installed.store(false, Ordering::Release);
            return Err(e);
        }
    };

    // Written straight into `original`, so a call racing the patch never sees 0
    unsafe {
        skyline::hooks::A64HookFunction(
            target as *const _,
            replacement as *const _,
            original.as_ptr().cast(),
        );
    }

    if original.load(Ordering::Acquire) == 0 {
        return Err(SymError::HookFailed(target));
    }

    Ok(())
}

#[macro_export]
macro_rules! hook {
    (
        $vis:vis $name:ident = $symbol:path;
        fn($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? $body:block
    ) => {
        $vis mod $name {
            #[allow(unused_imports, reason = "the hook body may not use the parent scope")]
            use super::*;

            const _: () = $crate::check::<_, ($($ty,)*), $crate::__ret_or_unit!($($ret)?)>(&$symbol);

            static ORIGINAL: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);
            static INSTALLED: ::core::sync::atomic::AtomicBool = ::core::sync::atomic::AtomicBool::new(false);

            #[allow(dead_code, reason = "not every hook calls the original")]
            pub unsafe fn original($($arg: $ty),*) $(-> $ret)? {
                let f: unsafe extern "C" fn($($ty),*) $(-> $ret)? = unsafe {
                    ::core::mem::transmute(ORIGINAL.load(::core::sync::atomic::Ordering::Acquire))
                };

                unsafe { f($($arg),*) }
            }

            extern "C" fn replacement($($arg: $ty),*) $(-> $ret)? $body

            pub fn install() -> ::core::result::Result<(), $crate::SymError> {
                $crate::__install(&INSTALLED, &ORIGINAL, $crate::fun_addr(&$symbol), replacement as *const () as usize)
            }
        }
    };
}

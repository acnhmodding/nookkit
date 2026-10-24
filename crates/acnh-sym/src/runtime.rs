use core::ffi::c_void;
use std::sync::OnceLock;

use skyline::hooks::{Region, getRegionAddress};

use crate::{FINGERPRINT_LEN, FINGERPRINT_OFFSET, Kind, SymError, SymbolFile, fingerprint, parse};

const SD_DIR: &str = "sd:/atmosphere/contents/01006F8002326000/acnh-ui/offsets";

static TABLE: OnceLock<SymbolFile> = OnceLock::new();

pub fn module_base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}

pub fn display_version() -> String {
    let mut version = skyline::nn::oe::DisplayVersion { name: [0; 16] };
    unsafe { skyline::nn::oe::GetDisplayVersion(&mut version) };
    let len = version.name.iter().position(|&b| b == 0).unwrap_or(version.name.len());
    String::from_utf8_lossy(&version.name[..len]).into_owned()
}

fn running_fingerprint() -> u64 {
    let window = unsafe { core::slice::from_raw_parts((module_base() + FINGERPRINT_OFFSET) as *const u8, FINGERPRINT_LEN) };
    fingerprint(window)
}

pub fn init(embedded: &str) -> Result<&'static SymbolFile, SymError> {
    if let Some(table) = TABLE.get() {
        return Ok(table);
    }

    let version = display_version();
    let path = format!("{SD_DIR}/{version}.toml");
    let (source_name, text) = match std::fs::read_to_string(&path) {
        Ok(text) => (path, text),
        Err(_) => ("embedded".to_owned(), embedded.to_owned()),
    };

    let file = parse(&text).map_err(|error| SymError::Parse { source_name, error })?;
    let actual = running_fingerprint();

    if file.rodata_hash != actual {
        return Err(SymError::UnsupportedBuild { version, expected: file.rodata_hash, actual });
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
        .ok_or_else(|| SymError::Missing { kind, name: name.to_owned() })
}

pub fn address(kind: Kind, name: &str) -> Result<usize, SymError> {
    Ok(module_base() + offset(kind, name)?)
}

pub fn resolve(name: &'static str) -> Option<usize> {
    address(Kind::Fn, name).ok()
}

pub fn fun_addr(name: &str) -> Result<usize, SymError> {
    address(Kind::Fn, name)
}

pub fn got<T>(name: &str) -> Result<*mut T, SymError> {
    Ok(unsafe { *(address(Kind::Got, name)? as *const *mut T) })
}

pub fn data<T>(name: &str) -> Result<*mut T, SymError> {
    Ok(address(Kind::Data, name)? as *mut T)
}

/// A `::vtable` label marks the address an object's vptr holds (slot 0), not the typeinfo header.
pub fn vtable<T>(name: &str) -> Result<*const T, SymError> {
    Ok(address(Kind::Data, name)? as *const T)
}

/// Guarded statics are only initialised once the game runs their check, so `Ok(None)` means "not yet".
pub fn type_info(name: &str) -> Result<Option<*const c_void>, SymError> {
    let info = address(Kind::TypeInfo, name)? as *const c_void;

    match address(Kind::TypeInfoGuard, name) {
        Ok(guard) => Ok((unsafe { *(guard as *const u8) } & 1 != 0).then_some(info)),
        Err(SymError::Missing { .. }) => Ok(Some(info)),
        Err(e) => Err(e),
    }
}

#[doc(hidden)]
pub use skyline::hooks::A64HookFunction as __hook_function;

/// ```ignore
/// acnh_sym::hook! {
///     pub view_page_initialize = "game::UI::ViewPage::initialize";
///     fn(page: *mut c_void) {
///         unsafe { original(page) };
///     }
/// }
/// view_page_initialize::install()?;
/// ```
#[macro_export]
macro_rules! hook {
    (
        $vis:vis $name:ident = $symbol:literal;
        fn($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? $body:block
    ) => {
        $vis mod $name {
            #[allow(unused_imports)]
            use super::*;

            static ORIGINAL: ::core::sync::atomic::AtomicUsize = ::core::sync::atomic::AtomicUsize::new(0);

            #[allow(dead_code)]
            pub unsafe fn original($($arg: $ty),*) $(-> $ret)? {
                let f: unsafe extern "C" fn($($ty),*) $(-> $ret)? = unsafe {
                    ::core::mem::transmute(ORIGINAL.load(::core::sync::atomic::Ordering::Acquire))
                };
                unsafe { f($($arg),*) }
            }

            extern "C" fn replacement($($arg: $ty),*) $(-> $ret)? $body

            pub fn install() -> ::core::result::Result<(), $crate::SymError> {
                if ORIGINAL.load(::core::sync::atomic::Ordering::Acquire) != 0 {
                    return Ok(());
                }

                let target = $crate::fun_addr($symbol)?;
                let mut original = ::core::ptr::null_mut();

                unsafe {
                    $crate::__hook_function(target as *const _, replacement as *const _, &mut original);
                }

                ORIGINAL.store(original as usize, ::core::sync::atomic::Ordering::Release);
                Ok(())
            }
        }
    };
}

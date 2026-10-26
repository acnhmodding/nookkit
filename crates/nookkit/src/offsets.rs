use skyline::hooks::{Region, getRegionAddress};

#[doc(hidden)]
pub use skyline::hooks::A64HookFunction as __hook_function;

pub fn module_base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}

/// A mod's own offsets into the supported game build, for what the framework doesn't bind
///
/// ```ignore
/// mod offsets {
///     acnh_ui_api::offsets! {
///         fill_mouth_list = 0x2b8fe60,
///     }
/// }
///
/// #[native(resolve = crate::offsets::resolve)]
/// unsafe extern "C" {
///     #[native(symbol = "fill_mouth_list")]
///     fn fill_mouth_list(values: *mut c_void);
/// }
/// ```
#[macro_export]
macro_rules! offsets {
    ($($name:ident = $offset:literal),* $(,)?) => {
        pub fn resolve(name: &'static str) -> ::core::option::Option<usize> {
            let offset: usize = match name {
                $(::core::stringify!($name) => $offset,)*
                _ => return ::core::option::Option::None,
            };

            ::core::option::Option::Some($crate::module_base() + offset)
        }
    };
}

/// An inline hook on a function the resolver finds
///
/// ```ignore
/// acnh_ui_api::hook! {
///     resolve = crate::offsets::resolve;
///     pub fill = "fill_mouth_list";
///     fn(values: *mut c_void) {
///         unsafe { original(values) };
///     }
/// }
///
/// fill::install()?;
/// ```
#[macro_export]
macro_rules! hook {
    (
        resolve = $resolve:path;
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

            pub fn install() -> $crate::Result {
                if ORIGINAL.load(::core::sync::atomic::Ordering::Acquire) != 0 {
                    return Ok(());
                }

                let target = $resolve($symbol).ok_or($crate::UiError::MissingSymbol)?;
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

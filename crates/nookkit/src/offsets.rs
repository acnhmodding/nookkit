pub use nookkit_sym::module_base;
#[doc(hidden)]
pub use nookkit_sym::{__install, Kind as __Kind, SymError as __SymError};

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

#[macro_export]
macro_rules! hook {
    (
        resolve = $resolve:path;
        $vis:vis $name:ident = $symbol:literal;
        fn($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? $body:block
    ) => {
        $vis mod $name {
            #[allow(unused_imports, reason = "the hook body may not use the parent scope")]
            use super::*;

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

            pub fn install() -> $crate::Result {
                let target = $resolve($symbol).ok_or_else(|| $crate::__SymError::Missing {
                    kind: $crate::__Kind::Fn,
                    name: ::std::string::String::from($symbol),
                });

                $crate::__install(&INSTALLED, &ORIGINAL, target, replacement as *const () as usize)
                    .map_err($crate::UiError::from)
            }
        }
    };
}

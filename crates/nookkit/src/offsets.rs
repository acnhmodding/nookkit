use skyline::hooks::{Region, getRegionAddress};

#[doc(hidden)]
pub use skyline::hooks::A64HookFunction as __hook_function;

pub fn module_base() -> usize {
    unsafe { getRegionAddress(Region::Text) as usize }
}

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
            #[allow(unused_imports)]
            use super::*;

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

            pub fn install() -> $crate::Result {
                // Hooking twice would make original() call the replacement
                if INSTALLED.swap(true, ::core::sync::atomic::Ordering::AcqRel) {
                    return Ok(());
                }

                let Some(target) = $resolve($symbol) else {
                    INSTALLED.store(false, ::core::sync::atomic::Ordering::Release);
                    return Err($crate::UiError::MissingSymbol);
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
                    return Err($crate::UiError::HookFailed);
                }

                Ok(())
            }
        }
    };
}

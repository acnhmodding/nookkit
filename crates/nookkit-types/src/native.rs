use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sead::runtime_type_info::Interface;

#[doc(hidden)]
pub use nookkit_sym as __sym;
pub use nookkit_sym::{Abi, Data, Fn, Vtable, check, sym};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,

    /// The address an object's vptr holds
    Vtable,

    /// A global variable
    Data,
}

/// Returns the absolute address of a symbol, or `None` when the symbol file lacks it
pub type Resolver = fn(name: &str, kind: SymbolKind) -> Option<usize>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// From a mod's own `#[native]` functions
    #[error(transparent)]
    Native(#[from] native_vtable::Unresolved),
    #[error("symbol \"{0}\" is missing")]
    Missing(String),
}

pub type NativeResult<T = ()> = Result<T, Error>;

static RESOLVER: AtomicUsize = AtomicUsize::new(0);

/// Set once by whoever owns the symbol table, the framework core or `nookkit::connect`
pub fn set_resolver(resolver: Resolver) {
    RESOLVER.store(resolver as usize, Ordering::Release);
}

fn lookup(name: &str, kind: SymbolKind) -> NativeResult<usize> {
    let resolver = RESOLVER.load(Ordering::Acquire);

    if resolver != 0 {
        let resolver: Resolver = unsafe { core::mem::transmute(resolver) };

        if let Some(address) = resolver(name, kind) {
            return Ok(address);
        }
    }

    Err(Error::Missing(name.to_owned()))
}

/// A game class with sead runtime type info
pub unsafe trait Class {
    /// `<Class>::getRuntimeTypeInfo`, which ignores `this`
    const RUNTIME_TYPE_INFO: Fn;
}

pub fn vtable<T>(symbol: &Vtable) -> NativeResult<*const T> {
    lookup(symbol.name, SymbolKind::Vtable).map(|address| address as *const T)
}

/// The global's address, not its value
pub fn data<T>(symbol: &Data) -> NativeResult<*mut T> {
    lookup(symbol.name, SymbolKind::Data).map(|address| address as *mut T)
}

pub fn fun_addr<S>(symbol: &Fn<S>) -> NativeResult<usize> {
    lookup(symbol.name, SymbolKind::Fn)
}

/// From a `<Class>::getRuntimeTypeInfo` symbol
pub fn type_info_of<S>(get_runtime_type_info: &Fn<S>) -> NativeResult<*const Interface> {
    type_info_by_name(get_runtime_type_info.name)
}

fn type_info_by_name(get_runtime_type_info: &str) -> NativeResult<*const Interface> {
    let get = lookup(get_runtime_type_info, SymbolKind::Fn)?;
    let get: unsafe extern "C" fn(*const ()) -> *const Interface =
        unsafe { core::mem::transmute(get) };

    Ok(unsafe { get(core::ptr::null()) })
}

pub fn type_info<T: Class>() -> NativeResult<*const Interface> {
    type_info_of(&T::RUNTIME_TYPE_INFO)
}

/// For a class only known at runtime, e.g. from a mod: `Lp::UI::BtnPartsCtrl`
pub fn type_info_named(class: &str) -> NativeResult<*const Interface> {
    type_info_by_name(&format!("{class}::getRuntimeTypeInfo"))
}

/// Declares game functions by their [`sym`] constant, checking the declaration against the exported signature.
/// Each returns [`NativeResult`], failing when the running symbol table lacks the function
///
/// ```ignore
/// nookkit_types::native! {
///     impl Page {
///         pub unsafe fn add_key_handler(&mut self, player: u32, key: ButtonId, handler: *mut KeyHandler)
///             = sym::Lp::UI::Page::addKeyHandler;
///     }
/// }
/// ```
#[macro_export]
macro_rules! native {
    (impl $self_ty:ty {
        $(
            $(#[$attr:meta])*
            $vis:vis unsafe fn $name:ident(&mut self $(, $arg:ident: $ty:ty)* $(,)?) $(-> $ret:ty)? = $symbol:path;
        )*
    }) => {
        impl $self_ty {
            $(
                $(#[$attr])*
                $vis unsafe fn $name(&mut self $(, $arg: $ty)*)
                    -> $crate::native::NativeResult<$crate::native::__sym::__ret_or_unit!($($ret)?)>
                {
                    const _: () = $crate::native::check::<
                        _,
                        (*mut $self_ty, $($ty,)*),
                        $crate::native::__sym::__ret_or_unit!($($ret)?),
                    >(&$symbol);

                    let f: unsafe extern "C" fn(*mut $self_ty $(, $ty)*) $(-> $ret)? =
                        unsafe { ::core::mem::transmute($crate::native::fun_addr(&$symbol)?) };

                    Ok(unsafe { f(self $(, $arg)*) })
                }
            )*
        }
    };

    ($(
        $(#[$attr:meta])*
        $vis:vis unsafe fn $name:ident($($arg:ident: $ty:ty),* $(,)?) $(-> $ret:ty)? = $symbol:path;
    )*) => {
        $(
            $(#[$attr])*
            $vis unsafe fn $name($($arg: $ty),*)
                -> $crate::native::NativeResult<$crate::native::__sym::__ret_or_unit!($($ret)?)>
            {
                const _: () = $crate::native::check::<
                    _,
                    ($($ty,)*),
                    $crate::native::__sym::__ret_or_unit!($($ret)?),
                >(&$symbol);

                let f: unsafe extern "C" fn($($ty),*) $(-> $ret)? =
                    unsafe { ::core::mem::transmute($crate::native::fun_addr(&$symbol)?) };

                Ok(unsafe { f($($arg),*) })
            }
        )*
    };
}

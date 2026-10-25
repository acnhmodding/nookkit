use core::sync::atomic::{AtomicUsize, Ordering};

use crate::sead::runtime_type_info::Interface;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SymbolKind {
    Fn,

    /// The address an object's vptr holds
    Vtable,
}

/// Returns the absolute address of a symbol, or `None` when the symbol file lacks it
pub type Resolver = fn(name: &str, kind: SymbolKind) -> Option<usize>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Native(#[from] native_vtable::Unresolved),
    #[error("symbol \"{0}\" is missing")]
    Missing(String),
}

pub type NativeResult<T = ()> = Result<T, Error>;

static RESOLVER: AtomicUsize = AtomicUsize::new(0);

/// Set once by whoever owns the symbol table, the framework core or `acnh_ui_api::connect`
pub fn set_resolver(resolver: Resolver) {
    RESOLVER.store(resolver as usize, Ordering::Release);
}

fn lookup(name: &str, kind: SymbolKind) -> Option<usize> {
    let resolver = RESOLVER.load(Ordering::Acquire);

    if resolver == 0 {
        return None;
    }

    let resolver: Resolver = unsafe { core::mem::transmute(resolver) };

    resolver(name, kind)
}

pub fn resolve(name: &'static str) -> Option<usize> {
    lookup(name, SymbolKind::Fn)
}

/// A game class with sead runtime type info
pub unsafe trait Class {
    /// `<Class>::getRuntimeTypeInfo`, which ignores `this`
    const RUNTIME_TYPE_INFO: &'static str;
}

pub fn vtable<T>(name: &str) -> NativeResult<*const T> {
    lookup(name, SymbolKind::Vtable)
        .map(|address| address as *const T)
        .ok_or_else(|| Error::Missing(name.to_owned()))
}

pub fn fun_addr(name: &str) -> NativeResult<usize> {
    lookup(name, SymbolKind::Fn).ok_or_else(|| Error::Missing(name.to_owned()))
}

pub fn type_info_of(get_runtime_type_info: &str) -> NativeResult<*const Interface> {
    let get = fun_addr(get_runtime_type_info)?;
    let get: unsafe extern "C" fn(*const ()) -> *const Interface =
        unsafe { core::mem::transmute(get) };

    Ok(unsafe { get(core::ptr::null()) })
}

pub fn type_info<T: Class>() -> NativeResult<*const Interface> {
    type_info_of(T::RUNTIME_TYPE_INFO)
}

/// For a class known only by name, e.g. `Lp::UI::BtnPartsCtrl`
pub fn type_info_named(class: &str) -> NativeResult<*const Interface> {
    type_info_of(&format!("{class}::getRuntimeTypeInfo"))
}

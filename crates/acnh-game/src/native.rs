use crate::sead::runtime_type_info::Interface;

pub use acnh_sym::resolve;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Native(#[from] native_vtable::Unresolved),
    #[error(transparent)]
    Symbol(#[from] acnh_sym::SymError),
}

pub type NativeResult<T = ()> = Result<T, Error>;

/// A game class with sead runtime type info
pub unsafe trait Class {
    /// `<Class>::getRuntimeTypeInfo`, which ignores `this`
    const RUNTIME_TYPE_INFO: &'static str;
}

pub fn vtable<T>(name: &str) -> NativeResult<*const T> {
    Ok(acnh_sym::vtable(name)?)
}

pub fn data<T>(name: &str) -> NativeResult<*mut T> {
    Ok(acnh_sym::data(name)?)
}

pub fn fun_addr(name: &str) -> NativeResult<usize> {
    Ok(acnh_sym::fun_addr(name)?)
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

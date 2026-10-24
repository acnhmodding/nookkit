use core::ffi::c_void;

pub use acnh_sym::resolve;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Native(#[from] native_vtable::Unresolved),
    #[error(transparent)]
    Symbol(#[from] acnh_sym::SymError),
}

pub type NativeResult<T = ()> = Result<T, Error>;

pub fn vtable<T>(name: &str) -> NativeResult<*const T> {
    Ok(acnh_sym::vtable(name)?)
}

pub fn data<T>(name: &str) -> NativeResult<*mut T> {
    Ok(acnh_sym::data(name)?)
}

pub fn fun_addr(name: &str) -> NativeResult<usize> {
    Ok(acnh_sym::fun_addr(name)?)
}

pub fn type_info(name: &str) -> NativeResult<Option<*const c_void>> {
    Ok(acnh_sym::type_info(name)?)
}

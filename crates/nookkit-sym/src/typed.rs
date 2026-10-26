use core::marker::PhantomData;
use core::ptr::NonNull;

pub struct Fn<S = Unchecked> {
    pub name: &'static str,
    _sig: PhantomData<S>,
}

impl<S> Fn<S> {
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            _sig: PhantomData,
        }
    }

    /// For places holding symbols of different signatures
    pub const fn erase(&self) -> Fn {
        Fn::new(self.name)
    }
}

impl<S> Clone for Fn<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Fn<S> {}

#[derive(Clone, Copy)]
pub struct Vtable {
    pub name: &'static str,
}

impl Vtable {
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }
}

pub struct Sig<Args, Ret>(PhantomData<fn() -> (Args, Ret)>);

pub struct Unchecked;

/// Only what the calling convention tells apart, so `u32` and `i32` match
pub mod abi {
    pub struct Ptr;
    pub struct I8;
    pub struct I16;
    pub struct I32;
    pub struct I64;
    pub struct F32;
    pub struct F64;
    pub struct Void;
}

/// # Safety
/// `Abi` must be what the type looks like to the calling convention
pub unsafe trait Abi {
    type Abi;
}

macro_rules! abi {
    ($($abi:ident: $($ty:ty),*;)*) => {
        $($(unsafe impl Abi for $ty {
            type Abi = abi::$abi;
        })*)*
    };
}

abi! {
    I8: u8, i8, bool;
    I16: u16, i16;
    I32: u32, i32, char;
    I64: u64, i64, usize, isize;
    F32: f32;
    F64: f64;
    Void: ();
}

unsafe impl<T> Abi for *mut T {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for *const T {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for &T {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for &mut T {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for Option<&T> {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for Option<&mut T> {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for NonNull<T> {
    type Abi = abi::Ptr;
}

unsafe impl<T> Abi for Option<NonNull<T>> {
    type Abi = abi::Ptr;
}

macro_rules! fn_pointers {
    ($($arg:ident)*) => {
        unsafe impl<R, $($arg),*> Abi for extern "C" fn($($arg),*) -> R {
            type Abi = abi::Ptr;
        }

        unsafe impl<R, $($arg),*> Abi for unsafe extern "C" fn($($arg),*) -> R {
            type Abi = abi::Ptr;
        }

        unsafe impl<R, $($arg),*> Abi for Option<extern "C" fn($($arg),*) -> R> {
            type Abi = abi::Ptr;
        }

        unsafe impl<R, $($arg),*> Abi for Option<unsafe extern "C" fn($($arg),*) -> R> {
            type Abi = abi::Ptr;
        }
    };
}

fn_pointers!();
fn_pointers!(A);
fn_pointers!(A B);
fn_pointers!(A B C);
fn_pointers!(A B C D);
fn_pointers!(A B C D E);
fn_pointers!(A B C D E F);

pub trait AbiArgs {
    type Abi;
}

macro_rules! abi_args {
    ($($arg:ident)*) => {
        impl<$($arg: Abi),*> AbiArgs for ($($arg,)*) {
            type Abi = ($($arg::Abi,)*);
        }
    };
}

abi_args!();
abi_args!(A);
abi_args!(A B);
abi_args!(A B C);
abi_args!(A B C D);
abi_args!(A B C D E);
abi_args!(A B C D E F);
abi_args!(A B C D E F G);
abi_args!(A B C D E F G H);
abi_args!(A B C D E F G H I);
abi_args!(A B C D E F G H I J);
abi_args!(A B C D E F G H I J K);
abi_args!(A B C D E F G H I J K L);

#[diagnostic::on_unimplemented(
    message = "the declared signature doesn't match the symbol's exported signature",
    label = "this symbol's signature is `{Self}`",
    note = "declared arguments `{A}`, returning `{R}`"
)]
pub trait Matches<A, R> {}

impl<A: AbiArgs, R: Abi> Matches<A, R> for Sig<A::Abi, R::Abi> {}

impl<A, R> Matches<A, R> for Unchecked {}

/// Compile-time check that a declaration with arguments `A` and return `R` fits the symbol
pub const fn check<S: Matches<A, R>, A, R>(_: &Fn<S>) {}

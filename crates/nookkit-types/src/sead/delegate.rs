use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{native, vtable, vtable_impl};

use crate::sead::{Heap, VftHeap};

#[vtable(path = sead::delegate::VftDelegate)]
pub unsafe trait VftDelegate {
    // Takes whatever arguments the delegate's owner invokes it with
    #[vtable(raw_this)]
    unsafe fn invoke(this: *mut Self);

    unsafe fn clone(&self, heap: *mut Heap) -> *mut Self;
    unsafe fn is_bound(&self) -> bool;
}

const _: () = assert!(VftDelegateLayout::OFFSET_IS_BOUND == 0x10);

#[repr(C)]
pub struct Delegate {
    pub vtable: *const VftDelegateLayout,
    pub object: *mut c_void,
    pub method: usize,
    pub method_adj: usize,
}

const _: () = assert!(core::mem::size_of::<Delegate>() == 0x20);

#[vtable_impl]
unsafe impl VftDelegate for Delegate {}

#[native(resolve = crate::native::resolve)]
unsafe extern "C" {
    #[native(symbol = "operator_new")]
    fn operator_new(size: usize) -> *mut c_void;
}

#[repr(C)]
struct RustVtable {
    invoke: unsafe extern "C" fn(),
    clone: unsafe extern "C" fn(*const Delegate, *mut Heap) -> *mut Delegate,
    is_bound: unsafe extern "C" fn(*const Delegate) -> bool,
}

static RUST_VTABLE: RustVtable = RustVtable {
    invoke,
    clone,
    is_bound,
};

// Tail-calls `method(object, ...)` with every other argument register untouched
#[unsafe(naked)]
unsafe extern "C" fn invoke() {
    core::arch::naked_asm!(
        "ldr x16, [x0, #{method}]",
        "ldr x0, [x0, #{object}]",
        "cbz x0, 1f",
        "cbz x16, 1f",
        "br x16",
        "1:",
        "ret",
        method = const offset_of!(Delegate, method),
        object = const offset_of!(Delegate, object),
    )
}

// Allocates like the game's delegates so whoever frees the clone uses the matching heap
unsafe extern "C" fn clone(this: *const Delegate, heap: *mut Heap) -> *mut Delegate {
    unsafe {
        let Some(this) = this.as_ref() else {
            return core::ptr::null_mut();
        };

        let size = core::mem::size_of::<Delegate>();
        let memory = match heap.as_mut() {
            Some(heap) => heap.try_alloc(size, 8),
            None => operator_new(size).unwrap_or_default(),
        }
        .cast::<Delegate>();

        if !memory.is_null() {
            memory.write(core::ptr::read(this));
        }

        memory
    }
}

unsafe extern "C" fn is_bound(this: *const Delegate) -> bool {
    unsafe {
        this.as_ref()
            .is_some_and(|d| !d.object.is_null() && d.method != 0)
    }
}

impl Delegate {
    pub const fn empty() -> Self {
        Self {
            vtable: core::ptr::null(),
            object: core::ptr::null_mut(),
            method: 0,
            method_adj: 0,
        }
    }

    pub fn new<T>(vtable: *const VftDelegateLayout, object: *mut T, method: usize) -> Self {
        Self {
            vtable,
            object: object.cast(),
            method,
            method_adj: 0,
        }
    }

    /// `method` must be an `extern "C" fn(*mut T, ...)` taking what the game invokes this delegate with
    pub fn bind<T>(object: *mut T, method: usize) -> Self {
        Self::new((&RUST_VTABLE as *const RustVtable).cast(), object, method)
    }
}

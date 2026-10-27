use core::cell::{Cell, UnsafeCell};
use core::ffi::c_void;

use nookkit_sys::Callback;

use crate::{Result, Ui, UiError};

struct Boxed<F> {
    ui: Ui,
    running: Cell<bool>,
    f: UnsafeCell<F>,
}

pub fn raw<F: FnMut(*const c_void) -> Result + 'static>(ui: Ui, f: F) -> Callback {
    unsafe extern "C" fn call<F: FnMut(*const c_void) -> Result>(
        user: *mut c_void,
        arg: *const c_void,
    ) {
        let Some(boxed) = (unsafe { user.cast::<Boxed<F>>().as_ref() }) else {
            return;
        };

        // A callback that fires its own event, e.g. decide inside on_decide, would alias its closure
        if boxed.running.replace(true) {
            boxed.ui.report(Err(UiError::Reentrant));
            return;
        }

        let result = unsafe { (*boxed.f.get())(arg) };
        boxed.running.set(false);
        boxed.ui.report(result);
    }

    unsafe extern "C" fn drop_boxed<F>(user: *mut c_void) {
        drop(unsafe { Box::from_raw(user.cast::<Boxed<F>>()) });
    }

    Callback {
        func: call::<F>,
        user: Box::into_raw(Box::new(Boxed {
            ui,
            running: Cell::new(false),
            f: UnsafeCell::new(f),
        }))
        .cast(),
        drop: Some(drop_boxed::<F>),
    }
}

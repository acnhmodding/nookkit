use core::ffi::c_void;

use nookkit_sys::Callback;

use crate::{Result, Ui};

struct Boxed<F> {
    ui: Ui,
    f: F,
}

pub fn raw<F: FnMut(*const c_void) -> Result + 'static>(ui: Ui, f: F) -> Callback {
    unsafe extern "C" fn call<F: FnMut(*const c_void) -> Result>(
        user: *mut c_void,
        arg: *const c_void,
    ) {
        let Some(boxed) = (unsafe { user.cast::<Boxed<F>>().as_mut() }) else {
            return;
        };

        let result = (boxed.f)(arg);
        boxed.ui.report(result);
    }

    unsafe extern "C" fn drop_boxed<F>(user: *mut c_void) {
        drop(unsafe { Box::from_raw(user.cast::<Boxed<F>>()) });
    }

    Callback {
        func: call::<F>,
        user: Box::into_raw(Box::new(Boxed { ui, f })).cast(),
        drop: Some(drop_boxed::<F>),
    }
}

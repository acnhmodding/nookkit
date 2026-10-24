use core::ffi::c_void;

use acnh_ui_sys::{Callback, ControlEvent};

use crate::{Event, Page, Result, Ui};

struct Boxed<F> {
    ui: Ui,
    f: F,
}

fn boxed<F>(ui: Ui, f: F, func: unsafe extern "C" fn(*mut c_void, *const c_void)) -> Callback {
    Callback {
        func,
        user: Box::into_raw(Box::new(Boxed { ui, f })).cast(),
        drop: Some(drop_boxed::<F>),
    }
}

unsafe extern "C" fn drop_boxed<F>(user: *mut c_void) {
    drop(unsafe { Box::from_raw(user.cast::<Boxed<F>>()) });
}

pub fn page<F: FnMut(Page) -> Result + Send + 'static>(ui: Ui, f: F) -> Callback {
    unsafe extern "C" fn call<F: FnMut(Page) -> Result>(user: *mut c_void, arg: *const c_void) {
        let Some(boxed) = (unsafe { user.cast::<Boxed<F>>().as_mut() }) else {
            return;
        };

        let result = (boxed.f)(Page::new(boxed.ui, arg.cast_mut()));
        boxed.ui.report(result);
    }

    boxed(ui, f, call::<F>)
}

pub fn event<F: FnMut(Event) -> Result + Send + 'static>(ui: Ui, f: F) -> Callback {
    unsafe extern "C" fn call<F: FnMut(Event) -> Result>(user: *mut c_void, arg: *const c_void) {
        let (Some(boxed), Some(event)) = (unsafe { user.cast::<Boxed<F>>().as_mut() }, unsafe {
            arg.cast::<ControlEvent>().as_ref()
        }) else {
            return;
        };

        let result = (boxed.f)(Event::new(boxed.ui, event));
        boxed.ui.report(result);
    }

    boxed(ui, f, call::<F>)
}

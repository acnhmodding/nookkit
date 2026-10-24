use core::ffi::c_void;
use core::sync::atomic::Ordering;
use std::ffi::CString;

use acnh_game::game::player_control_lock::{lock_menu, release_menu};
use acnh_game::game::ui::ViewPage;
use acnh_game::lp::ui::{BtnPartsCtrl, Control, PartsCtrl};
use acnh_game::native::{Error, NativeResult, type_info};
use acnh_sym::SymError;
use acnh_ui_sys::{AcnhUiV1, Callback, Direction, ListTarget, Status, Str};

use crate::lists::{self, ListState};
use crate::{STARTED, pages, seqs};

pub static TABLE: AcnhUiV1 = AcnhUiV1 {
    size: core::mem::size_of::<AcnhUiV1>(),
    framework_version: 1,
    on_page_built,
    find_pane,
    on_control_event,
    link,
    set_focusable,
    lock_player,
    release_player,
    set_visible,
    on_seq_event,
    list_get,
    list_on,
    list_set_items,
    list_set_enabled,
    list_link_into,
    list_relink,
    list_slot_count,
    list_button,
};

unsafe fn list_ref<'a>(list: *mut c_void) -> Option<&'a ListState> {
    unsafe { list.cast::<ListState>().as_ref() }
}

fn status(context: &str, result: NativeResult<Status>) -> Status {
    match result {
        Ok(status) => status,
        Err(e) => {
            println!("[acnh-ui] {context}: {e}");

            match e {
                Error::Symbol(SymError::Missing { .. }) | Error::Native(_) => Status::MissingSymbol,
                Error::Symbol(_) => Status::UnsupportedBuild,
            }
        }
    }
}

fn guarded(context: &str, f: impl FnOnce() -> NativeResult<Status>) -> Status {
    if !STARTED.load(Ordering::Acquire) {
        return Status::UnsupportedBuild;
    }

    status(context, f())
}

unsafe extern "C" fn on_page_built(layout: Str, class: Str, cb: Callback) -> Status {
    guarded("on_page_built", || {
        let (Some(layout), Some(class)) = (unsafe { layout.as_str() }, unsafe { class.as_str() })
        else {
            return Ok(Status::InvalidArg);
        };

        Ok(pages::on_page_built(layout, class, cb))
    })
}

unsafe extern "C" fn find_pane(
    page: *mut c_void,
    path: Str,
    class: Str,
    out: *mut *mut c_void,
) -> Status {
    guarded("find_pane", || unsafe {
        let (Some(page), Some(path), Some(class), false) = (
            page.cast::<ViewPage>().as_ref(),
            path.as_str(),
            class.as_str(),
            out.is_null(),
        ) else {
            return Ok(Status::InvalidArg);
        };

        let Ok(path) = CString::new(path) else {
            return Ok(Status::InvalidArg);
        };

        let Some(root) = page.root().as_mut() else {
            return Ok(Status::PaneNotFound);
        };

        let Some(pane) = root.find_by_path(&path)?.as_ref() else {
            return Ok(Status::PaneNotFound);
        };

        if !class.is_empty() && !pane.is_derived(type_info(class)?) {
            return Ok(Status::TypeMismatch);
        }

        *out = (pane as *const Control).cast_mut().cast();

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn on_control_event(page: *mut c_void, kind: u32, cb: Callback) -> Status {
    guarded("on_control_event", || unsafe {
        pages::on_control_event(page.cast(), kind, cb)
    })
}

unsafe extern "C" fn link(from: *mut c_void, direction: Direction, to: *mut c_void) -> Status {
    guarded("link", || unsafe {
        let Some(from) = BtnPartsCtrl::cast(from.cast())? else {
            return Ok(Status::TypeMismatch);
        };

        let target = if to.is_null() {
            core::ptr::null_mut()
        } else {
            match BtnPartsCtrl::cast(to.cast())? {
                Some(to) => to.focus_node,
                None => return Ok(Status::TypeMismatch),
            }
        };

        let Some(node) = from.focus_node.as_mut() else {
            return Ok(Status::InvalidArg);
        };

        node.links[direction as usize] = target;

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn set_focusable(pane: *mut c_void, player: u32, enable: bool) -> Status {
    guarded("set_focusable", || unsafe {
        let Some(pane) = pane.cast::<Control>().as_mut() else {
            return Ok(Status::InvalidArg);
        };

        pane.set_focusable(player, enable)?;

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn lock_player(player: u8) -> Status {
    guarded("lock_player", || unsafe {
        Ok(if lock_menu(player)? {
            Status::Ok
        } else {
            Status::LockFailed
        })
    })
}

unsafe extern "C" fn release_player(player: u8) -> Status {
    guarded("release_player", || unsafe {
        Ok(if release_menu(player)? {
            Status::Ok
        } else {
            Status::LockFailed
        })
    })
}

unsafe extern "C" fn set_visible(pane: *mut c_void, visible: bool) -> Status {
    guarded("set_visible", || unsafe {
        let Some(parts) = PartsCtrl::cast(pane.cast())? else {
            return Ok(Status::TypeMismatch);
        };

        let Some(pane) = parts.pane.as_mut() else {
            return Ok(Status::InvalidArg);
        };

        pane.set_visible(visible);

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn on_seq_event(page: *mut c_void, kind: u32, cb: Callback) -> Status {
    guarded("on_seq_event", || {
        Ok(seqs::on_seq_event(page.cast(), kind, cb))
    })
}

unsafe extern "C" fn list_get(page: *mut c_void, path: Str, out: *mut *mut c_void) -> Status {
    guarded("list_get", || unsafe {
        let (Some(path), false) = (path.as_str(), out.is_null()) else {
            return Ok(Status::InvalidArg);
        };

        Ok(match lists::get(page.cast(), path)? {
            Ok(list) => {
                *out = (list as *const ListState).cast_mut().cast();
                Status::Ok
            }
            Err(status) => status,
        })
    })
}

unsafe extern "C" fn list_on(list: *mut c_void, kind: u32, cb: Callback) -> Status {
    guarded("list_on", || unsafe {
        Ok(list_ref(list).map_or(Status::InvalidArg, |list| lists::on(list, kind, cb)))
    })
}

unsafe extern "C" fn list_set_items(
    list: *mut c_void,
    count: u32,
    selected: u32,
    player: u8,
) -> Status {
    guarded("list_set_items", || unsafe {
        let Some(list) = list_ref(list) else {
            return Ok(Status::InvalidArg);
        };

        list.set_items(count, selected, player)?;

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn list_set_enabled(list: *mut c_void, enable: bool, mode: u32) -> Status {
    guarded("list_set_enabled", || unsafe {
        let Some(list) = list_ref(list) else {
            return Ok(Status::InvalidArg);
        };

        list.set_enabled(enable, mode)?;

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn list_link_into(
    from: *mut c_void,
    direction: Direction,
    list: *mut c_void,
    target: ListTarget,
) -> Status {
    guarded("list_link_into", || unsafe {
        match list_ref(list) {
            Some(list) => lists::link_into(from, direction, list, target),
            None => Ok(Status::InvalidArg),
        }
    })
}

unsafe extern "C" fn list_relink(list: *mut c_void) -> Status {
    guarded("list_relink", || unsafe {
        let Some(list) = list_ref(list) else {
            return Ok(Status::InvalidArg);
        };

        list.relink();

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn list_slot_count(list: *mut c_void, out: *mut u32) -> Status {
    guarded("list_slot_count", || unsafe {
        let (Some(list), false) = (list_ref(list), out.is_null()) else {
            return Ok(Status::InvalidArg);
        };

        *out = lists::slot_count(list);

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn list_button(list: *mut c_void, slot: u32, out: *mut *mut c_void) -> Status {
    guarded("list_button", || unsafe {
        let (Some(list), false) = (list_ref(list), out.is_null()) else {
            return Ok(Status::InvalidArg);
        };

        let Some(button) = lists::button(list, slot) else {
            return Ok(Status::PaneNotFound);
        };

        *out = button;

        Ok(Status::Ok)
    })
}

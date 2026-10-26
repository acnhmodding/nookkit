use core::ffi::c_void;
use core::sync::atomic::Ordering;
use std::ffi::CString;

use nookkit_sym::Kind;
use nookkit_sys::{
    Callback, Direction, ListTarget, NookKitV1, SYMBOL_FN, SYMBOL_VTABLE, Status, Str,
};
use nookkit_types::game::player_control_lock::{lock_menu, release_menu};
use nookkit_types::game::ui::ViewPage;
use nookkit_types::lp::ui::{BtnPartsCtrl, Control, PartsCtrl};
use nookkit_types::native::{Error, NativeResult, type_info_named};

use crate::keys::{self, KeyState};
use crate::lists::{self, ListState};
use crate::{STARTED, pages, seqs};

pub static TABLE: NookKitV1 = NookKitV1 {
    size: core::mem::size_of::<NookKitV1>(),
    framework_version: 1,
    started,
    resolve,
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
    key_bind,
    key_on,
    key_set_enabled,
    key_set_guide,
    key_set_hint_pane,
};

unsafe fn key_ref<'a>(key: *mut c_void) -> Option<&'a KeyState> {
    unsafe { key.cast::<KeyState>().as_ref() }
}

unsafe fn list_ref<'a>(list: *mut c_void) -> Option<&'a ListState> {
    unsafe { list.cast::<ListState>().as_ref() }
}

fn status(context: &str, result: NativeResult<Status>) -> Status {
    match result {
        Ok(status) => status,
        Err(e) => {
            println!("[NookKit] {context}: {e}");

            match e {
                Error::Missing(_) => Status::MissingSymbol,
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

unsafe extern "C" fn started() -> bool {
    STARTED.load(Ordering::Acquire)
}

unsafe extern "C" fn resolve(name: Str, kind: u32) -> usize {
    let kind = match kind {
        SYMBOL_FN => Kind::Fn,
        SYMBOL_VTABLE => Kind::Vtable,
        _ => return 0,
    };

    match unsafe { name.as_str() } {
        Some(name) if STARTED.load(Ordering::Acquire) => {
            nookkit_sym::address(kind, name).unwrap_or(0)
        }
        _ => 0,
    }
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

        if !class.is_empty() && !pane.is_derived(type_info_named(class)?) {
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
        let Some(from) = Control::cast_ptr::<BtnPartsCtrl>(from.cast())? else {
            return Ok(Status::TypeMismatch);
        };

        let target = if to.is_null() {
            None
        } else {
            match Control::cast_ptr::<BtnPartsCtrl>(to.cast())? {
                Some(to) => Some(&*to),
                None => return Ok(Status::TypeMismatch),
            }
        };

        if from.focus_node.is_null() {
            return Ok(Status::InvalidArg);
        }

        from.set_link(crate::nav(direction), target);

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
        let Some(parts) = Control::cast_ptr::<PartsCtrl>(pane.cast())? else {
            return Ok(Status::TypeMismatch);
        };

        let Some(pane) = parts.pane() else {
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

unsafe extern "C" fn key_bind(
    page: *mut c_void,
    player: u32,
    key: i32,
    flags: u32,
    out: *mut *mut c_void,
) -> Status {
    guarded("key_bind", || unsafe {
        if out.is_null() {
            return Ok(Status::InvalidArg);
        }

        Ok(match keys::bind(page.cast(), player, key, flags)? {
            Ok(key) => {
                *out = key.cast();
                Status::Ok
            }
            Err(status) => status,
        })
    })
}

unsafe extern "C" fn key_on(key: *mut c_void, kind: u32, cb: Callback) -> Status {
    guarded("key_on", || unsafe {
        Ok(key_ref(key).map_or(Status::InvalidArg, |key| keys::on(key, kind, cb)))
    })
}

unsafe extern "C" fn key_set_enabled(key: *mut c_void, enable: bool) -> Status {
    guarded("key_set_enabled", || unsafe {
        let Some(key) = key_ref(key) else {
            return Ok(Status::InvalidArg);
        };

        keys::set_enabled(key, enable);

        Ok(Status::Ok)
    })
}

unsafe extern "C" fn key_set_guide(key: *mut c_void, text: Str, label: Str) -> Status {
    guarded("key_set_guide", || unsafe {
        let (Some(key), Some(text), Some(label)) = (key_ref(key), text.as_str(), label.as_str())
        else {
            return Ok(Status::InvalidArg);
        };

        Ok(keys::set_guide(key, text, label))
    })
}

unsafe extern "C" fn key_set_hint_pane(key: *mut c_void, path: Str) -> Status {
    guarded("key_set_hint_pane", || unsafe {
        let (Some(key_state), Some(path)) = (key_ref(key), path.as_str()) else {
            return Ok(Status::InvalidArg);
        };

        let Some(page) = keys::page_of(key_state) else {
            return Ok(Status::InvalidArg);
        };

        keys::set_hint_pane(page, key_state, path)
    })
}

use core::ffi::c_void;
use core::sync::atomic::Ordering;
use std::ffi::CString;

use nookkit_sym::Kind;
use nookkit_sys::{
    Callback, ControlEventKind, Direction, Handle, KeyEventKind, KeyFlags, ListEvent, ListTarget,
    NookKitV2, PaneHandle, Player, SeqEvent, StatePhase, Status, Str, SymbolKind,
};
use nookkit_types::game::player_control_lock::{lock_menu, release_menu};
use nookkit_types::game::ui::{ViewPage, open_pocket_menu as request_pocket_menu};
use nookkit_types::lp::ui::{
    BtnPartsCtrl, Control, FocusNode, PartsCtrl, UniteBtnCtrl, VftBtnPartsCtrl, VftUniteBtnCtrl,
};
use nookkit_types::native::{Error, NativeResult, type_info_named};

use crate::keys::{self, KeyState};
use crate::lists::{self, ListState};
use crate::{STARTED, pages, seqs};

pub static TABLE: NookKitV2 = NookKitV2 {
    size: core::mem::size_of::<NookKitV2>(),
    framework_version: 2,
    started,
    resolve,
    on_page_built,
    page_check,
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
    page_hide_guide,
    page_set_input_enabled,
    open_pocket_menu,
    find_page,
    page_seq,
    seq_page,
    on_state,
    page_focused,
    pane_decide,
    pane_set_valid,
    pane_is_valid,
};

/// Unwraps a check, returning its status from a `guarded` closure
macro_rules! checked {
    ($e:expr) => {
        match $e {
            Ok(value) => value,
            Err(status) => return Ok(status),
        }
    };
}

fn page(handle: Handle) -> Result<&'static mut ViewPage, Status> {
    let page = pages::check(handle)?;

    unsafe { page.as_mut() }.ok_or(Status::INVALID_ARG)
}

fn pane(handle: PaneHandle) -> Result<*mut Control, Status> {
    pages::check(handle.page)?;

    if handle.raw.is_null() {
        return Err(Status::INVALID_ARG);
    }

    Ok(handle.raw.cast())
}

// Key states are freed with their page, so only one still in the registry is safe to touch
fn key(key: *mut c_void) -> Result<(&'static KeyState, *mut ViewPage), Status> {
    if key.is_null() {
        return Err(Status::INVALID_ARG);
    }

    let page = keys::page_of(key.cast()).ok_or(Status::STALE)?;

    Ok((unsafe { &*key.cast::<KeyState>() }, page))
}

// Retired lists stay allocated but their control may be gone with the page
fn list(list: *mut c_void) -> Result<&'static ListState, Status> {
    let list = unsafe { list.cast::<ListState>().as_ref() }.ok_or(Status::INVALID_ARG)?;

    if list.is_alive() {
        Ok(list)
    } else {
        Err(Status::STALE)
    }
}

fn player(player: Player) -> Result<Player, Status> {
    if player.is_valid() {
        Ok(player)
    } else {
        Err(Status::INVALID_ARG)
    }
}

fn status(context: &str, result: NativeResult<Status>) -> Status {
    match result {
        Ok(status) => status,
        Err(e) => {
            println!("[NookKit] {context}: {e}");

            match e {
                Error::Missing(_) | Error::Native(_) => Status::MISSING_SYMBOL,
            }
        }
    }
}

fn guarded(context: &str, f: impl FnOnce() -> NativeResult<Status>) -> Status {
    if !STARTED.load(Ordering::Acquire) {
        return Status::UNSUPPORTED_BUILD;
    }

    status(context, f())
}

unsafe extern "C" fn started() -> bool {
    STARTED.load(Ordering::Acquire)
}

unsafe extern "C" fn resolve(name: Str, kind: SymbolKind) -> usize {
    let kind = match kind {
        SymbolKind::FN => Kind::Fn,
        SymbolKind::VTABLE => Kind::Vtable,
        SymbolKind::DATA => Kind::Data,
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
            return Ok(Status::INVALID_ARG);
        };

        Ok(pages::on_page_built(layout, class, cb))
    })
}

unsafe extern "C" fn page_check(handle: Handle) -> Status {
    guarded("page_check", || {
        checked!(pages::check(handle));

        Ok(Status::OK)
    })
}

unsafe extern "C" fn find_pane(
    handle: Handle,
    path: Str,
    class: Str,
    out: *mut *mut c_void,
) -> Status {
    guarded("find_pane", || unsafe {
        let page = checked!(page(handle));

        let (Some(path), Some(class), false) = (path.as_str(), class.as_str(), out.is_null())
        else {
            return Ok(Status::INVALID_ARG);
        };

        let Ok(path) = CString::new(path) else {
            return Ok(Status::INVALID_ARG);
        };

        let Some(root) = page.root().as_mut() else {
            return Ok(Status::PANE_NOT_FOUND);
        };

        let Some(pane) = root.find_by_path(&path)?.as_ref() else {
            return Ok(Status::PANE_NOT_FOUND);
        };

        if !class.is_empty() {
            let Ok(type_info) = type_info_named(class) else {
                return Ok(Status::INVALID_ARG);
            };

            if !pane.is_derived(type_info) {
                return Ok(Status::TYPE_MISMATCH);
            }
        }

        *out = (pane as *const Control).cast_mut().cast();

        Ok(Status::OK)
    })
}

unsafe extern "C" fn on_control_event(
    handle: Handle,
    kind: ControlEventKind,
    cb: Callback,
) -> Status {
    guarded("on_control_event", || unsafe {
        let page = checked!(pages::check(handle));

        pages::on_control_event(page, kind, cb)
    })
}

unsafe extern "C" fn link(from: PaneHandle, direction: Direction, to: PaneHandle) -> Status {
    guarded("link", || unsafe {
        let from = checked!(pane(from));
        let Some(direction) = crate::nav(direction) else {
            return Ok(Status::INVALID_ARG);
        };

        let Some(from) = Control::cast_ptr::<BtnPartsCtrl>(from)? else {
            return Ok(Status::TYPE_MISMATCH);
        };

        let target = if to.raw.is_null() {
            None
        } else {
            match Control::cast_ptr::<BtnPartsCtrl>(checked!(pane(to)))? {
                Some(to) => Some(&*to),
                None => return Ok(Status::TYPE_MISMATCH),
            }
        };

        // A target without a focus node would silently unlink
        if from.focus_node.is_null() || target.is_some_and(|to| to.focus_node.is_null()) {
            return Ok(Status::INVALID_ARG);
        }

        from.set_link(direction, target);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn set_focusable(handle: PaneHandle, player_: Player, enable: bool) -> Status {
    guarded("set_focusable", || unsafe {
        let pane = checked!(pane(handle));
        let player = checked!(player(player_));

        let Some(pane) = pane.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        pane.set_focusable(u32::from(player.0), enable)?;

        Ok(Status::OK)
    })
}

unsafe extern "C" fn lock_player(player_: Player) -> Status {
    guarded("lock_player", || unsafe {
        let player = checked!(player(player_));

        Ok(if lock_menu(player.0)? {
            Status::OK
        } else {
            Status::LOCK_FAILED
        })
    })
}

unsafe extern "C" fn release_player(player_: Player) -> Status {
    guarded("release_player", || unsafe {
        let player = checked!(player(player_));

        Ok(if release_menu(player.0)? {
            Status::OK
        } else {
            Status::LOCK_FAILED
        })
    })
}

unsafe extern "C" fn set_visible(handle: PaneHandle, visible: bool) -> Status {
    guarded("set_visible", || unsafe {
        let Some(parts) = Control::cast_ptr::<PartsCtrl>(checked!(pane(handle)))? else {
            return Ok(Status::TYPE_MISMATCH);
        };

        let Some(pane) = parts.pane() else {
            return Ok(Status::INVALID_ARG);
        };

        pane.set_visible(visible);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn on_seq_event(handle: Handle, kind: SeqEvent, cb: Callback) -> Status {
    guarded("on_seq_event", || {
        let page = checked!(pages::check(handle));

        Ok(seqs::on_seq_event(page, kind, cb))
    })
}

unsafe extern "C" fn list_get(handle: Handle, path: Str, out: *mut *mut c_void) -> Status {
    guarded("list_get", || unsafe {
        let page = checked!(pages::check(handle));

        let (Some(path), false) = (path.as_str(), out.is_null()) else {
            return Ok(Status::INVALID_ARG);
        };

        Ok(match lists::get(page, path)? {
            Ok(list) => {
                *out = (list as *const ListState).cast_mut().cast();
                Status::OK
            }
            Err(status) => status,
        })
    })
}

unsafe extern "C" fn list_on(list_: *mut c_void, kind: ListEvent, cb: Callback) -> Status {
    guarded("list_on", || unsafe {
        let list = checked!(list(list_));

        Ok(lists::on(list, kind, cb))
    })
}

unsafe extern "C" fn list_set_items(
    list_: *mut c_void,
    count: u32,
    selected: u32,
    player_: Player,
) -> Status {
    guarded("list_set_items", || unsafe {
        let list = checked!(list(list_));
        let player = checked!(player(player_));

        list.set_items(count, selected, player)?;

        Ok(Status::OK)
    })
}

unsafe extern "C" fn list_set_enabled(list_: *mut c_void, enable: bool, mode: u32) -> Status {
    guarded("list_set_enabled", || unsafe {
        let list = checked!(list(list_));

        list.set_enabled(enable, mode)?;

        Ok(Status::OK)
    })
}

unsafe extern "C" fn list_link_into(
    from: PaneHandle,
    direction: Direction,
    list_: *mut c_void,
    target: ListTarget,
) -> Status {
    guarded("list_link_into", || unsafe {
        let from = checked!(pane(from));
        let list = checked!(list(list_));

        if crate::nav(direction).is_none() {
            return Ok(Status::INVALID_ARG);
        }

        lists::link_into(from.cast(), direction, list, target)
    })
}

unsafe extern "C" fn list_relink(list_: *mut c_void) -> Status {
    guarded("list_relink", || unsafe {
        let list = checked!(list(list_));

        list.relink();

        Ok(Status::OK)
    })
}

unsafe extern "C" fn list_slot_count(list_: *mut c_void, out: *mut u32) -> Status {
    guarded("list_slot_count", || unsafe {
        let list = checked!(list(list_));

        let Some(out) = out.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        *out = lists::slot_count(list);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn list_button(list_: *mut c_void, slot: u32, out: *mut *mut c_void) -> Status {
    guarded("list_button", || unsafe {
        let list = checked!(list(list_));

        let Some(out) = out.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        let Some(button) = lists::button(list, slot) else {
            return Ok(Status::PANE_NOT_FOUND);
        };

        *out = button;

        Ok(Status::OK)
    })
}

unsafe extern "C" fn key_bind(
    handle: Handle,
    player: Player,
    key: i32,
    flags: KeyFlags,
    out: *mut *mut c_void,
) -> Status {
    guarded("key_bind", || unsafe {
        let page = checked!(pages::check(handle));

        let Some(out) = out.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        Ok(match keys::bind(page, player, key, flags)? {
            Ok(key) => {
                *out = key.cast();
                Status::OK
            }
            Err(status) => status,
        })
    })
}

unsafe extern "C" fn key_on(key_: *mut c_void, kind: KeyEventKind, cb: Callback) -> Status {
    guarded("key_on", || {
        let (key, _) = checked!(key(key_));

        Ok(keys::on(key, kind, cb))
    })
}

unsafe extern "C" fn key_set_enabled(key_: *mut c_void, enable: bool) -> Status {
    guarded("key_set_enabled", || {
        let (key, _) = checked!(key(key_));

        keys::set_enabled(key, enable);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn key_set_guide(key_: *mut c_void, text: Str, label: Str) -> Status {
    guarded("key_set_guide", || unsafe {
        let (key, _) = checked!(key(key_));

        let (Some(text), Some(label)) = (text.as_str(), label.as_str()) else {
            return Ok(Status::INVALID_ARG);
        };

        Ok(keys::set_guide(key, text, label))
    })
}

unsafe extern "C" fn key_set_hint_pane(key_: *mut c_void, path: Str) -> Status {
    guarded("key_set_hint_pane", || unsafe {
        let (key, page) = checked!(key(key_));

        let Some(path) = path.as_str() else {
            return Ok(Status::INVALID_ARG);
        };

        keys::set_hint_pane(page, key, path)
    })
}

unsafe extern "C" fn page_hide_guide(handle: Handle, key: i32, hide: bool) -> Status {
    guarded("page_hide_guide", || {
        let page = checked!(pages::check(handle));

        Ok(keys::hide_guide(page, key, hide))
    })
}

unsafe extern "C" fn page_set_input_enabled(handle: Handle, enable: bool) -> Status {
    guarded("page_set_input_enabled", || unsafe {
        let Some(root) = checked!(page(handle)).input_root() else {
            return Ok(Status::INVALID_ARG);
        };

        root.disabled = !enable;

        Ok(Status::OK)
    })
}

unsafe extern "C" fn open_pocket_menu(player_: Player, close_enabled: bool) -> Status {
    guarded("open_pocket_menu", || unsafe {
        let player = checked!(player(player_));

        request_pocket_menu(player.0, close_enabled)?;

        Ok(Status::OK)
    })
}

unsafe extern "C" fn find_page(class: Str, out: *mut Handle) -> Status {
    guarded("find_page", || unsafe {
        let (Some(class), Some(out)) = (class.as_str(), out.as_mut()) else {
            return Ok(Status::INVALID_ARG);
        };

        *out = pages::find_live(class).unwrap_or(Handle::NULL);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn page_seq(handle: Handle, out: *mut *mut c_void) -> Status {
    guarded("page_seq", || unsafe {
        let page = checked!(pages::check(handle));

        let Some(out) = out.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        *out = pages::live_seq(page).map_or(core::ptr::null_mut(), |seq| seq as *mut c_void);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn seq_page(seq: *mut c_void, out: *mut Handle) -> Status {
    guarded("seq_page", || unsafe {
        let (false, Some(out)) = (seq.is_null(), out.as_mut()) else {
            return Ok(Status::INVALID_ARG);
        };

        *out = pages::page_of_seq(seq as usize).unwrap_or(Handle::NULL);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn on_state(
    handle: Handle,
    state: i32,
    phase: StatePhase,
    after: bool,
    cb: Callback,
) -> Status {
    guarded("on_state", || {
        let page = checked!(pages::check(handle));

        Ok(crate::states::on_state(page, state, phase, after, cb))
    })
}

// The button whose focus node is `node`, anywhere under `control`
unsafe fn owner_of(control: *mut Control, node: *mut FocusNode) -> NativeResult<*mut Control> {
    unsafe {
        let Some(control_ref) = control.as_mut() else {
            return Ok(core::ptr::null_mut());
        };

        if let Some(button) = control_ref.cast::<BtnPartsCtrl>()?
            && core::ptr::eq(button.focus_node, node)
        {
            return Ok(control);
        }

        for child in control_ref.children() {
            let found = owner_of(child, node)?;
            if !found.is_null() {
                return Ok(found);
            }
        }

        Ok(core::ptr::null_mut())
    }
}

unsafe extern "C" fn page_focused(handle: Handle, player_: Player, out: *mut *mut c_void) -> Status {
    guarded("page_focused", || unsafe {
        let page = checked!(page(handle));
        let player = checked!(player(player_));

        let Some(out) = out.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        *out = core::ptr::null_mut();

        let node = page
            .input_root()
            .and_then(|root| root.focus_navigator.as_ref())
            .map_or(core::ptr::null_mut(), |nav| nav.focused(u32::from(player.0)));
        if node.is_null() {
            return Ok(Status::OK);
        }

        for &root in page.roots() {
            let found = owner_of(root, node)?;
            if !found.is_null() {
                *out = found.cast();
                break;
            }
        }

        Ok(Status::OK)
    })
}

unsafe extern "C" fn pane_decide(handle: PaneHandle, out: *mut bool) -> Status {
    guarded("pane_decide", || unsafe {
        let Some(button) = Control::cast_ptr::<BtnPartsCtrl>(checked!(pane(handle)))? else {
            return Ok(Status::TYPE_MISMATCH);
        };

        let decided = button.force_decide();
        if let Some(out) = out.as_mut() {
            *out = decided;
        }

        Ok(Status::OK)
    })
}

unsafe extern "C" fn pane_set_valid(handle: PaneHandle, valid: bool) -> Status {
    guarded("pane_set_valid", || unsafe {
        let Some(button) = Control::cast_ptr::<UniteBtnCtrl>(checked!(pane(handle)))? else {
            return Ok(Status::TYPE_MISMATCH);
        };

        button.set_disp_valid(valid, false);

        Ok(Status::OK)
    })
}

unsafe extern "C" fn pane_is_valid(handle: PaneHandle, out: *mut bool) -> Status {
    guarded("pane_is_valid", || unsafe {
        let pane = checked!(pane(handle));

        let Some(out) = out.as_mut() else {
            return Ok(Status::INVALID_ARG);
        };

        let Some(button) = Control::cast_ptr::<UniteBtnCtrl>(pane)? else {
            return Ok(Status::TYPE_MISMATCH);
        };

        *out = !button.is_disp_invalid();

        Ok(Status::OK)
    })
}

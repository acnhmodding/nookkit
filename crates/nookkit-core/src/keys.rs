use core::cell::{Cell, RefCell, UnsafeCell};
use core::ffi::c_void;
use nookkit_sym::sym;
use std::collections::BTreeMap;
use std::ffi::CString;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use nookkit_sys::{Callback, KeyEvent, KeyEventKind, KeyFlags, Player, Status};
use nookkit_types::game::ui::{KeyGuideEntry, KeyGuideSeq, ViewPage};
use nookkit_types::lp::ui::{ButtonId, Control, KeyEvent as GameKeyEvent, KeyHandler, PartsCtrl};
use nookkit_types::native::NativeResult;
use nookkit_types::sead::{Delegate, SafeString};

use crate::pages::{PAGES, call, snapshot, with_page};
use crate::report;

const SENTINEL: &str = "nookkit:";

enum Guide {
    Text(u32),
    Label(String),
}

pub struct KeyState {
    handler: UnsafeCell<KeyHandler>,
    cbs: [RefCell<Vec<Callback>>; KeyEventKind::COUNT],
    guide: RefCell<Option<Guide>>,
    hint_pane: Cell<*mut PartsCtrl>,
    seq_player: bool,

    /// As bound, since a seq-player binding's handler is rewritten on activation
    player: Player,
}

unsafe impl Send for KeyState {}

static TEXTS: Mutex<BTreeMap<u32, Vec<u16>>> = Mutex::new(BTreeMap::new());
static NEXT_TEXT: AtomicU32 = AtomicU32::new(0);

impl KeyState {
    #[allow(clippy::mut_from_ref)]
    fn handler(&self) -> &mut KeyHandler {
        unsafe { &mut *self.handler.get() }
    }

    fn as_user(&self) -> *mut c_void {
        (self as *const Self).cast_mut().cast()
    }

    fn set_guide(&self, guide: Option<Guide>) {
        if let Some(Guide::Text(id)) = self.guide.replace(guide)
            && let Ok(mut texts) = TEXTS.lock()
        {
            texts.remove(&id);
        }
    }

    fn sync_hint_pane(&self) {
        if let Some(pane) = unsafe { self.hint_pane.get().as_mut().and_then(|p| p.pane()) } {
            pane.set_visible(self.handler().enabled);
        }
    }

    unsafe fn guide_entry(&self) -> NativeResult<Option<KeyGuideEntry>> {
        let handler = self.handler();

        if !handler.enabled {
            return Ok(None);
        }

        let label = match &*self.guide.borrow() {
            Some(Guide::Text(id)) => format!("{SENTINEL}{id}"),
            Some(Guide::Label(label)) => label.clone(),
            None => return Ok(None),
        };

        Ok(Some(KeyGuideEntry::new(
            handler.key,
            &label,
            handler.player as u8,
        )?))
    }
}

impl Drop for KeyState {
    fn drop(&mut self) {
        self.set_guide(None);
    }
}

extern "C" fn dispatch(user: *mut c_void, kind: u32, event: *const GameKeyEvent) {
    let (Some(state), Some(event)) = (unsafe { user.cast::<KeyState>().as_ref() }, unsafe {
        event.as_ref()
    }) else {
        return;
    };

    let Some(cbs) = state.cbs.get(kind as usize) else {
        return;
    };

    let Some(handler) = (unsafe { event.handler.as_ref() }) else {
        return;
    };

    let event = KeyEvent {
        kind: KeyEventKind(kind),
        player: Player(handler.player as u8),
        key: handler.key as i32,
    };

    let cbs = snapshot(&cbs.borrow());

    unsafe { call(cbs, (&event as *const KeyEvent).cast()) };
}

pub unsafe fn bind(
    page: *mut ViewPage,
    player: Player,
    key: i32,
    flags: KeyFlags,
) -> NativeResult<Result<*mut KeyState, Status>> {
    let (Some(page_ref), Some(key)) = (unsafe { page.as_mut() }, ButtonId::from_raw(key)) else {
        return Ok(Err(Status::INVALID_ARG));
    };

    if key == ButtonId::Invalid || !player.is_valid() {
        return Ok(Err(Status::INVALID_ARG));
    }

    let existing = with_page(page, |state| {
        state
            .keys
            .iter()
            .find(|k| k.player == player && k.handler().key == key)
            .map(|k| &**k as *const KeyState as *mut KeyState)
    })
    .flatten();

    if let Some(existing) = existing {
        return Ok(Ok(existing));
    }

    let state = Box::new(KeyState {
        handler: UnsafeCell::new(KeyHandler::new(u32::from(player.0), key)),
        cbs: Default::default(),
        guide: RefCell::new(None),
        hint_pane: Cell::new(core::ptr::null_mut()),
        seq_player: flags.contains(KeyFlags::SEQ_PLAYER),
        player,
    });

    let handler = state.handler();

    for delegate in &mut handler.event_delegates {
        *delegate = Delegate::bind(state.as_user(), dispatch as *const () as usize);
    }

    handler.consume_key = flags.contains(KeyFlags::CONSUME);

    unsafe { page_ref.add_key_handler(u32::from(player.0), key, handler)? };

    // The page's own handlers come first in the list, so only a handler ahead of them can take the key
    if handler.consume_key
        && let Some(root) = unsafe { page_ref.input_root() }
    {
        unsafe { root.key_handlers.move_to_front(&mut handler.list_node) };
    }

    let ptr = &*state as *const KeyState as *mut KeyState;

    with_page(page, |page_state| page_state.keys.push(state));

    Ok(Ok(ptr))
}

/// Points the page's seq-player bindings at `player` when its seq activates
pub fn follow_player(page: *mut ViewPage, player: u8) {
    let player = if player < 8 { u32::from(player) } else { 0 };

    // Every seq activates through here, so only pages someone registered on are touched
    if let Ok(pages) = PAGES.lock()
        && let Some(state) = pages.get(&(page as usize))
    {
        for key in state.keys.iter().filter(|k| k.seq_player) {
            key.handler().player = player;
        }
    }
}

pub fn on(key: &KeyState, kind: KeyEventKind, cb: Callback) -> Status {
    match key.cbs.get(kind.0 as usize) {
        Some(cbs) => {
            cbs.borrow_mut().push(cb);
            Status::OK
        }
        None => Status::INVALID_ARG,
    }
}

pub fn set_enabled(key: &KeyState, enable: bool) {
    key.handler().enabled = enable;
    key.sync_hint_pane();
}

pub fn set_guide(key: &KeyState, text: &str, label: &str) -> Status {
    if !text.is_empty() {
        let id = NEXT_TEXT.fetch_add(1, Ordering::Relaxed);
        let utf16 = text.encode_utf16().chain([0]).collect();

        if let Ok(mut texts) = TEXTS.lock() {
            texts.insert(id, utf16);
        }

        key.set_guide(Some(Guide::Text(id)));
    } else if !label.is_empty() {
        if label.len() >= 64 {
            return Status::INVALID_ARG;
        }

        key.set_guide(Some(Guide::Label(label.to_owned())));
    } else {
        key.set_guide(None);
    }

    Status::OK
}

pub unsafe fn set_hint_pane(
    page: *mut ViewPage,
    key: &KeyState,
    path: &str,
) -> NativeResult<Status> {
    if path.is_empty() {
        key.hint_pane.set(core::ptr::null_mut());
        return Ok(Status::OK);
    }

    let (Some(page), Ok(path)) = (unsafe { page.as_ref() }, CString::new(path)) else {
        return Ok(Status::INVALID_ARG);
    };

    let Some(root) = (unsafe { page.root().as_mut() }) else {
        return Ok(Status::PANE_NOT_FOUND);
    };

    let control = unsafe { root.find_by_path(&path)? };

    if control.is_null() {
        return Ok(Status::PANE_NOT_FOUND);
    }

    let Some(parts) = (unsafe { Control::cast_ptr::<PartsCtrl>(control)? }) else {
        return Ok(Status::TYPE_MISMATCH);
    };

    key.hint_pane.set(parts);
    key.sync_hint_pane();

    Ok(Status::OK)
}

pub fn page_of(key: *const KeyState) -> Option<*mut ViewPage> {
    let pages = PAGES.lock().ok()?;

    pages
        .iter()
        .find(|(_, state)| state.keys.iter().any(|k| core::ptr::eq(&**k, key)))
        .map(|(&page, _)| page as *mut ViewPage)
}

// The bar's entries for the pages whose seq is active
fn active_guides() -> Vec<KeyGuideEntry> {
    let Ok(pages) = PAGES.lock() else {
        return Vec::new();
    };

    pages
        .values()
        .filter(|state| state.active)
        .flat_map(|state| state.keys.iter())
        .filter_map(|key| {
            let entry = unsafe { key.guide_entry() };
            report_value("key guide", entry).flatten()
        })
        .collect()
}

pub fn hide_guide(page: *mut ViewPage, key: i32, hide: bool) -> Status {
    if ButtonId::from_raw(key).is_none() {
        return Status::INVALID_ARG;
    }

    with_page(page, |state| {
        state.hidden_guides.retain(|&k| k != key);

        if hide {
            state.hidden_guides.push(key);
        }
    })
    .map_or(Status::INVALID_ARG, |()| Status::OK)
}

fn guide_hidden(key: i32) -> bool {
    PAGES.lock().is_ok_and(|pages| {
        pages
            .values()
            .any(|state| state.active && state.hidden_guides.contains(&key))
    })
}

fn report_value<T>(context: &str, result: NativeResult<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(e) => {
            report(context, Err(e));
            None
        }
    }
}

nookkit_sym::hook! {
    pub key_guide_add = sym::game::UI::KeyGuideSeq::addGuide;
    fn(seq: *mut KeyGuideSeq, entry: *const KeyGuideEntry, flag: bool) {
        let Some(seq_ref) = (unsafe { seq.as_mut() }) else {
            unsafe { original(seq, entry, flag) };
            return;
        };

        let before = seq_ref.pending.len();
        let hidden = unsafe { entry.as_ref() }.is_some_and(|e| guide_hidden(e.key));

        if !hidden {
            unsafe { original(seq, entry, flag) };
        }

        // A page starts each rebuild of the bar with its first entry, hidden or not
        if before == 0 && (hidden || seq_ref.pending.len() == 1) {
            for mut guide in active_guides() {
                guide.label.pin();
                unsafe { original(seq, &guide, flag) };
            }
        }
    }
}

#[repr(C)]
struct WideBuffer {
    vtable: *const c_void,
    buffer: *mut u16,
    size: i32,
}

// Every message lookup goes through here, so a sentinel label is answered with its literal text
nookkit_sym::hook! {
    pub message_find = sym::Lp::UI::UIMgr::findMessage;
    fn(dst: *mut WideBuffer, file: *const SafeString<'static>, label: *const SafeString<'static>) -> bool {
        match unsafe { literal_text(dst, label) } {
            Some(found) => found,
            None => unsafe { original(dst, file, label) },
        }
    }
}

unsafe fn literal_text(dst: *mut WideBuffer, label: *const SafeString<'static>) -> Option<bool> {
    let label = unsafe { label.as_ref()?.as_c_str()? }.to_bytes();
    let id = core::str::from_utf8(label.strip_prefix(SENTINEL.as_bytes())?)
        .ok()?
        .parse::<u32>()
        .ok()?;

    let texts = TEXTS.lock().ok()?;
    let Some(text) = texts.get(&id) else {
        return Some(false);
    };

    let dst = unsafe { dst.as_mut()? };

    if dst.buffer.is_null() || dst.size <= 0 {
        return Some(false);
    }

    let len = text.len().min(dst.size as usize);

    unsafe {
        core::ptr::copy_nonoverlapping(text.as_ptr(), dst.buffer, len);
        *dst.buffer.add(len - 1) = 0;
    }

    Some(true)
}

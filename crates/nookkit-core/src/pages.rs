use core::ffi::c_void;
use nookkit_sym::sym;
use std::any::Any;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use nookkit_sys::{Callback, ControlEvent as Event, ControlEventKind, Handle, SeqEvent, Status};
use nookkit_types::game::ui::{ControlEvent, LayoutBuiltEvent, ViewPage};
use nookkit_types::game::ui_sys::ScrollBtnListCtrl;
use nookkit_types::lp::ui::Control;
use nookkit_types::native::NativeResult;
use nookkit_types::sead::{Delegate, VftDelegate};

use crate::keys::KeyState;
use crate::lists::ListState;
use crate::report;
use crate::states::StateHook;

struct PageListener {
    layout: String,
    class: String,
    cb: Callback,
}

static PAGE_LISTENERS: Mutex<Vec<PageListener>> = Mutex::new(Vec::new());

// Every page the game has built and not destroyed, whether or not a mod registered on it
struct Live {
    class: String,
    seq: usize,
    built: u64,
}

static LIVE: Mutex<BTreeMap<usize, Live>> = Mutex::new(BTreeMap::new());
static BUILT: AtomicU64 = AtomicU64::new(0);

/// The most recently built live page of `class`
pub fn find_live(class: &str) -> Option<Handle> {
    let live = LIVE.lock().ok()?;

    live.iter()
        .filter(|(_, l)| l.class == class)
        .max_by_key(|(_, l)| l.built)
        .map(|(&page, l)| Handle {
            raw: page as *mut c_void,
            build: l.built,
        })
}

/// The page behind `handle` while that build of it lives
pub fn check(handle: Handle) -> Result<*mut ViewPage, Status> {
    if handle.raw.is_null() {
        return Err(Status::INVALID_ARG);
    }

    let live = LIVE.lock().map_err(|_| Status::INVALID_ARG)?;

    match live.get(&(handle.raw as usize)) {
        Some(l) if l.built == handle.build => Ok(handle.raw.cast()),
        _ => Err(Status::STALE),
    }
}

/// `Handle::NULL` once the page is destroyed
pub fn handle_of(page: *mut ViewPage) -> Handle {
    LIVE.lock()
        .ok()
        .and_then(|live| {
            live.get(&(page as usize)).map(|l| Handle {
                raw: page.cast(),
                build: l.built,
            })
        })
        .unwrap_or(Handle::NULL)
}

/// The live page whose seq is `seq`
pub fn page_of_seq(seq: usize) -> Option<Handle> {
    let live = LIVE.lock().ok()?;

    live.iter()
        .find(|(_, l)| l.seq == seq && seq != 0)
        .map(|(&page, l)| Handle {
            raw: page as *mut c_void,
            build: l.built,
        })
}

pub fn live_seq(page: *mut ViewPage) -> Option<usize> {
    let live = LIVE.lock().ok()?;

    live.get(&(page as usize))
        .map(|l| l.seq)
        .filter(|&seq| seq != 0)
}

pub fn set_live_seq(page: *mut ViewPage, seq: usize) {
    if let Ok(mut live) = LIVE.lock()
        && let Some(l) = live.get_mut(&(page as usize))
    {
        l.seq = seq;
    }
}

struct EventSlot {
    page: Handle,
    kind: ControlEventKind,
    cbs: Mutex<Vec<Callback>>,
}

unsafe impl Send for EventSlot {}

#[derive(Default)]
pub struct PageState {
    // Boxed so the delegates in the game's tables keep pointing at them
    #[allow(clippy::vec_box)]
    events: Vec<Box<EventSlot>>,

    pub seq: [Vec<Callback>; SeqEvent::COUNT],
    pub lists: Vec<&'static ListState>,

    // Boxed so the key handlers in the page's input root keep pointing at them
    #[allow(clippy::vec_box)]
    pub keys: Vec<Box<KeyState>>,

    /// Its seq is active, so its key guides belong in the bar
    pub active: bool,

    /// ButtonIds whose game-made guide entries are dropped while the page is active
    pub hidden_guides: Vec<i32>,

    /// State hooks waiting for the page's seq
    pub states: Vec<crate::states::StateReg>,

    /// Hooked entries of the page's seq, to retire with the page
    pub state_hooks: Vec<&'static StateHook>,
}

impl PageState {
    fn retire(&self) {
        for list in &self.lists {
            list.retire();
        }

        for hook in &self.state_hooks {
            hook.retire();
        }
    }
}

pub static PAGES: Mutex<BTreeMap<usize, PageState>> = Mutex::new(BTreeMap::new());

pub type Snapshot = Vec<(
    unsafe extern "C" fn(*mut c_void, *const c_void),
    *mut c_void,
)>;

static DEPTH: AtomicU32 = AtomicU32::new(0);
static GRAVEYARD: Mutex<Vec<Box<dyn Any + Send>>> = Mutex::new(Vec::new());

/// While one lives, removed callbacks wait in the graveyard, since snapshots still point at them
pub struct Dispatch(());

pub fn dispatching() -> Dispatch {
    DEPTH.fetch_add(1, Ordering::AcqRel);
    Dispatch(())
}

impl Drop for Dispatch {
    fn drop(&mut self) {
        if DEPTH.fetch_sub(1, Ordering::AcqRel) == 1 {
            let dead = GRAVEYARD
                .lock()
                .map(|mut dead| core::mem::take(&mut *dead))
                .unwrap_or_default();

            drop(dead);
        }
    }
}

/// Drops `dead` now, or after the running dispatch. Never called with a registry lock held, since
/// a mod's drop may call back in
pub fn bury(dead: impl Any + Send) {
    if DEPTH.load(Ordering::Acquire) == 0 {
        drop(dead);
        return;
    }

    match GRAVEYARD.lock() {
        Ok(mut graveyard) => graveyard.push(Box::new(dead)),
        Err(_) => drop(dead),
    }
}

/// Callbacks may register more while they run, so they are called from a copy taken under the lock
pub fn snapshot(cbs: &[Callback]) -> Snapshot {
    cbs.iter().map(|cb| (cb.func, cb.user)).collect()
}

pub unsafe fn call(cbs: Snapshot, arg: *const c_void) {
    let _dispatch = dispatching();

    for (func, user) in cbs {
        unsafe { func(user, arg) };
    }
}

/// For registrations, so it adds the page
pub fn with_page<R>(page: *mut ViewPage, f: impl FnOnce(&mut PageState) -> R) -> Option<R> {
    let mut pages = PAGES.lock().ok()?;

    Some(f(pages.entry(page as usize).or_default()))
}

/// `None` for pages nobody registered on
pub fn with_registered<R>(page: *mut ViewPage, f: impl FnOnce(&mut PageState) -> R) -> Option<R> {
    let mut pages = PAGES.lock().ok()?;

    pages.get_mut(&(page as usize)).map(f)
}

fn forget(page: *mut ViewPage) {
    let stale = PAGES
        .lock()
        .ok()
        .and_then(|mut pages| pages.remove(&(page as usize)));

    if let Some(stale) = stale {
        stale.retire();
        bury(stale);
    }
}

nookkit_sym::hook! {
    pub view_page_initialize = sym::game::UI::ViewPage::initialize;
    fn(page: *mut ViewPage, config: *mut c_void) {
        unsafe { original(page, config) };
        unsafe { built(page) };
    }
}

// Every page's destructor ends in this one. It frees the event tables and input root without
// walking them, so our slots and key handlers go after it
nookkit_sym::hook! {
    pub view_page_destroy = sym::game::UI::ViewPage::_ViewPage;
    fn(page: *mut c_void) {
        if let Ok(mut live) = LIVE.lock() {
            live.remove(&(page as usize));
        }

        unsafe { original(page) };
        forget(page.cast());
    }
}

nookkit_sym::hook! {
    pub view_page_layout_built = sym::game::UI::ViewPage::dispatchLayoutBuilt;
    fn(page: *mut ViewPage, event: *mut LayoutBuiltEvent) {
        unsafe { original(page, event) };

        if let Some(event) = unsafe { event.as_ref() } {
            report("auto-bind", unsafe { bind_unbound_lists(event.parent, event.builder) });
        }
    }
}

// The page binds its own lists in the original, so any list still without buttons came from a layout mod
unsafe fn bind_unbound_lists(control: *mut Control, builder: *mut c_void) -> NativeResult {
    unsafe {
        let Some(control_ref) = control.as_mut() else {
            return Ok(());
        };

        if let Some(list) = control_ref.cast::<ScrollBtnListCtrl>()? {
            if list.button_count() == 0 {
                list.bind_buttons(builder)?;

                let name = list.name().and_then(|n| n.to_str().ok()).unwrap_or("?");
                println!(
                    "[NookKit] bound list {name} ({} buttons)",
                    list.button_count()
                );
            }

            return Ok(());
        }

        for child in control_ref.children() {
            bind_unbound_lists(child, builder)?;
        }

        Ok(())
    }
}

unsafe fn built(page: *mut ViewPage) {
    // Pages are initialized once, so anything left at this address belonged to a destroyed page
    forget(page);

    let Some(page_ref) = (unsafe { page.as_ref() }) else {
        return;
    };

    let layout = unsafe { page_ref.layout() }
        .and_then(|s| s.to_str().ok())
        .unwrap_or("");
    let class = unsafe { page_ref.class() }
        .and_then(|s| s.to_str().ok())
        .unwrap_or("");

    // From 1, so build 0 never matches
    let built = BUILT.fetch_add(1, Ordering::Relaxed) + 1;

    if let Ok(mut live) = LIVE.lock() {
        live.insert(
            page as usize,
            Live {
                class: class.to_owned(),
                seq: 0,
                built,
            },
        );
    }

    let handle = Handle {
        raw: page.cast(),
        build: built,
    };

    let matching: Snapshot = match PAGE_LISTENERS.lock() {
        Ok(listeners) => listeners
            .iter()
            .filter(|l| {
                (l.layout.is_empty() || l.layout == layout)
                    && (l.class.is_empty() || l.class == class)
            })
            .map(|l| (l.cb.func, l.cb.user))
            .collect(),
        Err(_) => return,
    };

    unsafe { call(matching, (&handle as *const Handle).cast()) };
}

pub fn on_page_built(layout: &str, class: &str, cb: Callback) -> Status {
    let Ok(mut listeners) = PAGE_LISTENERS.lock() else {
        return Status::INVALID_ARG;
    };

    listeners.push(PageListener {
        layout: layout.to_owned(),
        class: class.to_owned(),
        cb,
    });

    Status::OK
}

extern "C" fn dispatch(slot: *mut EventSlot, event: *const ControlEvent) {
    let Some(slot) = (unsafe { slot.as_ref() }) else {
        return;
    };

    let control = unsafe { event.as_ref() }.map_or(core::ptr::null_mut(), |e| e.control);
    let event = Event {
        page: slot.page,
        control: control.cast(),
        kind: slot.kind,
    };

    let Some(cbs) = slot.cbs.lock().ok().map(|cbs| snapshot(&cbs)) else {
        return;
    };

    unsafe { call(cbs, (&event as *const Event).cast()) };
}

pub unsafe fn on_control_event(
    page: *mut ViewPage,
    kind: ControlEventKind,
    cb: Callback,
) -> NativeResult<Status> {
    if page.is_null() {
        return Ok(Status::INVALID_ARG);
    }

    let waiting = live_seq(page).is_none();
    let handle = handle_of(page);
    let Ok(mut pages) = PAGES.lock() else {
        return Ok(Status::INVALID_ARG);
    };

    let events = &mut pages.entry(page as usize).or_default().events;

    if let Some(slot) = events.iter().find(|s| s.kind == kind) {
        if let Ok(mut cbs) = slot.cbs.lock() {
            cbs.push(cb);
        }

        return Ok(Status::OK);
    }

    let Some(page_ref) = (unsafe { page.as_mut() }) else {
        return Ok(Status::INVALID_ARG);
    };

    let Some(table) = (unsafe { page_ref.control_events.kind(kind.0) }) else {
        return Ok(Status::NO_EVENT_TABLE);
    };

    // The page's seq registers with the first free column once it is set up, so taking the last
    // one leaves it room and makes ours run after the game's handler
    let mut columns = table.iter_mut().rev().filter(|d| unsafe { !d.is_bound() });

    let Some(free) = columns.next() else {
        return Ok(Status::EVENT_TABLE_FULL);
    };

    if waiting && columns.next().is_none() {
        return Ok(Status::EVENT_TABLE_FULL);
    }

    let mut slot = Box::new(EventSlot {
        page: handle,
        kind,
        cbs: Mutex::new(vec![cb]),
    });

    *free = Delegate::bind(&mut *slot as *mut EventSlot, dispatch as *const () as usize);
    events.push(slot);

    Ok(Status::OK)
}

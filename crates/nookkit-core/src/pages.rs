use core::ffi::c_void;
use nookkit_sym::sym;
use std::collections::BTreeMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use nookkit_sys::{Callback, ControlEvent as Event, SEQ_EVENT_KINDS, Status};
use nookkit_types::game::ui::{ControlEvent, LayoutBuiltEvent, ViewPage};
use nookkit_types::game::ui_sys::ScrollBtnListCtrl;
use nookkit_types::lp::ui::Control;
use nookkit_types::native::NativeResult;
use nookkit_types::sead::{Delegate, VftDelegate};

use crate::keys::KeyState;
use crate::lists::ListState;
use crate::report;

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
pub fn find_live(class: &str) -> Option<usize> {
    let live = LIVE.lock().ok()?;

    live.iter()
        .filter(|(_, l)| l.class == class)
        .max_by_key(|(_, l)| l.built)
        .map(|(&page, _)| page)
}

pub fn live_seq(page: *mut ViewPage) -> Option<usize> {
    let live = LIVE.lock().ok()?;

    live.get(&(page as usize))
        .map(|l| l.seq)
        .filter(|&seq| seq != 0)
}

/// Which build of `page` this is, to tell a rebuild over a live seq apart
pub fn live_build(page: *mut ViewPage) -> Option<u64> {
    let live = LIVE.lock().ok()?;

    live.get(&(page as usize)).map(|l| l.built)
}

pub fn set_live_seq(page: *mut ViewPage, seq: usize) {
    if let Ok(mut live) = LIVE.lock()
        && let Some(l) = live.get_mut(&(page as usize))
    {
        l.seq = seq;
    }
}

struct EventSlot {
    page: *mut ViewPage,
    kind: u32,
    cbs: Mutex<Vec<Callback>>,
}

unsafe impl Send for EventSlot {}

#[derive(Default)]
pub struct PageState {
    // Boxed so the delegates in the game's tables keep pointing at them
    #[allow(clippy::vec_box)]
    events: Vec<Box<EventSlot>>,

    pub seq: [Vec<Callback>; SEQ_EVENT_KINDS],
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
}

pub static PAGES: Mutex<BTreeMap<usize, PageState>> = Mutex::new(BTreeMap::new());

pub type Snapshot = Vec<(
    unsafe extern "C" fn(*mut c_void, *const c_void),
    *mut c_void,
)>;

/// Callbacks may register more while they run, so they are called from a copy taken under the lock
pub fn snapshot(cbs: &[Callback]) -> Snapshot {
    cbs.iter().map(|cb| (cb.func, cb.user)).collect()
}

pub unsafe fn call(cbs: Snapshot, arg: *const c_void) {
    for (func, user) in cbs {
        unsafe { func(user, arg) };
    }
}

pub fn with_page<R>(page: *mut ViewPage, f: impl FnOnce(&mut PageState) -> R) -> Option<R> {
    let mut pages = PAGES.lock().ok()?;

    Some(f(pages.entry(page as usize).or_default()))
}

nookkit_sym::hook! {
    pub view_page_initialize = sym::game::UI::ViewPage::initialize;
    fn(page: *mut ViewPage, config: *mut c_void) {
        unsafe { original(page, config) };
        unsafe { built(page) };
    }
}

// Every page's destructor ends in this one
nookkit_sym::hook! {
    pub view_page_destroy = sym::game::UI::ViewPage::_ViewPage;
    fn(page: *mut c_void) {
        if let Ok(mut live) = LIVE.lock() {
            live.remove(&(page as usize));
        }

        unsafe { original(page) };
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
    // A page built again at this address has a new event table, so its old slots are unreachable
    let stale = PAGES
        .lock()
        .ok()
        .and_then(|mut pages| pages.remove(&(page as usize)));

    if let Some(stale) = stale {
        for list in &stale.lists {
            list.retire();
        }
    }

    let Some(page_ref) = (unsafe { page.as_ref() }) else {
        return;
    };

    let layout = unsafe { page_ref.layout() }
        .and_then(|s| s.to_str().ok())
        .unwrap_or("");
    let class = unsafe { page_ref.class() }
        .and_then(|s| s.to_str().ok())
        .unwrap_or("");

    if let Ok(mut live) = LIVE.lock() {
        let built = BUILT.fetch_add(1, Ordering::Relaxed);
        live.insert(
            page as usize,
            Live {
                class: class.to_owned(),
                seq: 0,
                built,
            },
        );
    }

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

    unsafe { call(matching, page.cast()) };
}

pub fn on_page_built(layout: &str, class: &str, cb: Callback) -> Status {
    let Ok(mut listeners) = PAGE_LISTENERS.lock() else {
        return Status::InvalidArg;
    };

    listeners.push(PageListener {
        layout: layout.to_owned(),
        class: class.to_owned(),
        cb,
    });

    Status::Ok
}

extern "C" fn dispatch(slot: *mut EventSlot, event: *const ControlEvent) {
    let Some(slot) = (unsafe { slot.as_ref() }) else {
        return;
    };

    let control = unsafe { event.as_ref() }.map_or(core::ptr::null_mut(), |e| e.control);
    let event = Event {
        page: slot.page.cast(),
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
    kind: u32,
    cb: Callback,
) -> NativeResult<Status> {
    let Ok(mut pages) = PAGES.lock() else {
        return Ok(Status::InvalidArg);
    };

    let events = &mut pages.entry(page as usize).or_default().events;

    if let Some(slot) = events.iter().find(|s| s.kind == kind) {
        if let Ok(mut cbs) = slot.cbs.lock() {
            cbs.push(cb);
        }

        return Ok(Status::Ok);
    }

    let Some(page_ref) = (unsafe { page.as_mut() }) else {
        return Ok(Status::InvalidArg);
    };

    let Some(table) = (unsafe { page_ref.control_events.kind(kind) }) else {
        return Ok(Status::NoEventTable);
    };

    // The page's seq registers with the first free column after the page is built, so taking the
    // last one leaves it room and makes ours run after the game's handler
    let Some(free) = table.iter_mut().rev().find(|d| unsafe { !d.is_bound() }) else {
        return Ok(Status::EventTableFull);
    };

    let mut slot = Box::new(EventSlot {
        page,
        kind,
        cbs: Mutex::new(vec![cb]),
    });

    *free = Delegate::bind(&mut *slot as *mut EventSlot, dispatch as *const () as usize);
    events.push(slot);

    Ok(Status::Ok)
}

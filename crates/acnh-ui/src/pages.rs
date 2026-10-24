use core::ffi::c_void;
use std::collections::BTreeMap;
use std::sync::Mutex;

use acnh_game::game::ui::{ControlEvent, ViewPage};
use acnh_game::native::NativeResult;
use acnh_game::sead::{Delegate, VftDelegate};
use acnh_ui_sys::{Callback, ControlEvent as Event, Status};

struct PageListener {
    layout: String,
    class: String,
    cb: Callback,
}

static PAGE_LISTENERS: Mutex<Vec<PageListener>> = Mutex::new(Vec::new());

struct EventSlot {
    page: *mut ViewPage,
    kind: u32,
    cbs: Mutex<Vec<Callback>>,
}

unsafe impl Send for EventSlot {}

// Boxed so the delegates in the game's tables keep pointing at them
#[allow(clippy::vec_box)]
static PAGES: Mutex<BTreeMap<usize, Vec<Box<EventSlot>>>> = Mutex::new(BTreeMap::new());

acnh_sym::hook! {
    pub view_page_initialize = "game::UI::ViewPage::initialize";
    fn(page: *mut ViewPage, config: *mut c_void) {
        unsafe { original(page, config) };
        unsafe { built(page) };
    }
}

unsafe fn built(page: *mut ViewPage) {
    // A page built again at this address has a new event table, so its old slots are unreachable
    let stale = PAGES
        .lock()
        .ok()
        .and_then(|mut pages| pages.remove(&(page as usize)));
    drop(stale);

    let Some(page_ref) = (unsafe { page.as_ref() }) else {
        return;
    };

    let layout = unsafe { page_ref.layout() }
        .and_then(|s| s.to_str().ok())
        .unwrap_or("");
    let class = unsafe { page_ref.class() }
        .and_then(|s| s.to_str().ok())
        .unwrap_or("");

    let matching: Vec<_> = match PAGE_LISTENERS.lock() {
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

    // Listeners are never removed, so their user data outlives the lock
    for (func, user) in matching {
        unsafe { func(user, page.cast()) };
    }
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

    let cbs: Vec<_> = match slot.cbs.lock() {
        Ok(cbs) => cbs.iter().map(|cb| (cb.func, cb.user)).collect(),
        Err(_) => return,
    };

    for (func, user) in cbs {
        unsafe { func(user, (&event as *const Event).cast()) };
    }
}

pub unsafe fn on_control_event(
    page: *mut ViewPage,
    kind: u32,
    cb: Callback,
) -> NativeResult<Status> {
    let Ok(mut pages) = PAGES.lock() else {
        return Ok(Status::InvalidArg);
    };

    let slots = pages.entry(page as usize).or_default();

    if let Some(slot) = slots.iter().find(|s| s.kind == kind) {
        if let Ok(mut cbs) = slot.cbs.lock() {
            cbs.push(cb);
        }

        return Ok(Status::Ok);
    }

    let Some(page_ref) = (unsafe { page.as_mut() }) else {
        return Ok(Status::InvalidArg);
    };

    let Some(table) = (unsafe { page_ref.event_slots(kind) }) else {
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
    slots.push(slot);

    Ok(Status::Ok)
}

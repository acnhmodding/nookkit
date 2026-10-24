use core::ffi::c_void;

use acnh_game::game::ui::{SequenceBase, ViewPage};
use acnh_ui_sys::{
    Callback, SEQ_ACTIVATE, SEQ_DEACTIVATED, SEQ_EVENT_KINDS, SEQ_READY, SEQ_UPDATE, Status,
};

use crate::pages::{PAGES, call, snapshot, with_page};
use crate::report;

acnh_sym::hook! {
    pub sequence_create = "game::UISys::Sequence::create";
    fn(seq: *mut SequenceBase, arg: *mut c_void) {
        unsafe { original(seq, arg) };
        unsafe { ready(seq) };
    }
}

// onActivated is a 12-byte thunk, too short for an inline hook
acnh_sym::hook! {
    pub sequence_on_activate = "game::UI::SequenceBase::onActivate";
    fn(seq: *mut SequenceBase) {
        unsafe { original(seq) };
        unsafe { fire(seq, SEQ_ACTIVATE) };
    }
}

acnh_sym::hook! {
    pub sequence_on_deactivated = "game::UI::SequenceBase::onDeactivated";
    fn(seq: *mut SequenceBase) {
        unsafe { original(seq) };

        for list in unsafe { lists_of(seq) } {
            list.set_active(false);
        }

        unsafe { fire(seq, SEQ_DEACTIVATED) };
    }
}

acnh_sym::hook! {
    pub sequence_update = "game::UI::SequenceBase::update";
    fn(seq: *mut SequenceBase) {
        unsafe { original(seq) };
        unsafe { fire(seq, SEQ_UPDATE) };
    }
}

unsafe fn page_of(seq: *mut SequenceBase) -> *mut ViewPage {
    unsafe { seq.as_ref() }.map_or(core::ptr::null_mut(), |seq| seq.page)
}

unsafe fn lists_of(seq: *mut SequenceBase) -> Vec<&'static crate::lists::ListState> {
    let page = unsafe { page_of(seq) } as usize;

    PAGES
        .lock()
        .ok()
        .and_then(|pages| pages.get(&page).map(|state| state.lists.clone()))
        .unwrap_or_default()
}

unsafe fn ready(seq: *mut SequenceBase) {
    for list in unsafe { lists_of(seq) } {
        report("list setup", unsafe { list.setup() });
    }

    unsafe { fire(seq, SEQ_READY) };
}

// Only pages someone registered on are in PAGES, so seqs without a SequenceBase page never match
unsafe fn fire(seq: *mut SequenceBase, kind: u32) {
    let page = unsafe { page_of(seq) } as usize;

    let cbs = PAGES.lock().ok().and_then(|pages| {
        let state = pages.get(&page)?;
        let cbs = state.seq.get(kind as usize)?;

        (!cbs.is_empty()).then(|| snapshot(cbs))
    });

    if let Some(cbs) = cbs {
        unsafe { call(cbs, seq.cast()) };
    }
}

pub fn on_seq_event(page: *mut ViewPage, kind: u32, cb: Callback) -> Status {
    if kind as usize >= SEQ_EVENT_KINDS || page.is_null() {
        return Status::InvalidArg;
    }

    with_page(page, |state| state.seq[kind as usize].push(cb))
        .map_or(Status::InvalidArg, |()| Status::Ok)
}

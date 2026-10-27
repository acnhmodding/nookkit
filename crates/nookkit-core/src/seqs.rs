use core::ffi::c_void;
use nookkit_sym::sym;

use nookkit_sys::{Callback, SeqEvent, Status};
use nookkit_types::game::ui::{SequenceBase, ViewPage};
use nookkit_types::game::ui_sys::Sequence;

use crate::pages::{PAGES, call, snapshot, with_page};
use crate::report;

nookkit_sym::hook! {
    pub sequence_create = sym::game::UISys::Sequence::create;
    fn(seq: *mut Sequence, arg: *mut c_void) {
        unsafe { original(seq, arg) };

        // Only a SequenceBase has the state machine and player the rest relies on
        match unsafe { seq.as_mut().map(|seq| seq.cast::<SequenceBase>()) } {
            Some(Ok(Some(base))) => unsafe { ready(base) },
            Some(Err(e)) => report("sequence create", Err(e)),
            _ => {}
        }
    }
}

// onActivated is a 12-byte thunk, too short for an inline hook
nookkit_sym::hook! {
    pub sequence_on_activate = sym::game::UI::SequenceBase::onActivate;
    fn(seq: *mut SequenceBase) {
        unsafe { original(seq) };
        unsafe { set_active(seq, true) };

        if let Some(seq_ref) = unsafe { seq.as_ref() } {
            crate::keys::follow_player(unsafe { page_of(seq) }, seq_ref.player);
        }
        unsafe { fire(seq, SeqEvent::ACTIVATE) };
    }
}

nookkit_sym::hook! {
    pub sequence_on_deactivated = sym::game::UI::SequenceBase::onDeactivated;
    fn(seq: *mut SequenceBase) {
        unsafe { original(seq) };

        for list in unsafe { lists_of(seq) } {
            list.set_active(false);
        }

        unsafe { set_active(seq, false) };

        unsafe { fire(seq, SeqEvent::DEACTIVATE) };
    }
}

nookkit_sym::hook! {
    pub sequence_update = sym::game::UI::SequenceBase::update;
    fn(seq: *mut SequenceBase) {
        unsafe { original(seq) };
        unsafe { fire(seq, SeqEvent::UPDATE) };
    }
}

unsafe fn page_of(seq: *mut SequenceBase) -> *mut ViewPage {
    unsafe { seq.as_ref() }.map_or(core::ptr::null_mut(), |seq| unsafe { seq.page() })
}

unsafe fn set_active(seq: *mut SequenceBase, active: bool) {
    let page = unsafe { page_of(seq) } as usize;

    if let Ok(mut pages) = PAGES.lock()
        && let Some(state) = pages.get_mut(&page)
    {
        state.active = active;
    }
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
    let page = unsafe { page_of(seq) };
    crate::pages::set_live_seq(page, seq as usize);
    unsafe { crate::states::ready(page, seq) };

    for list in unsafe { lists_of(seq) } {
        report("list setup", unsafe { list.setup() });
    }

    unsafe { fire(seq, SeqEvent::READY) };
}

// Only pages someone registered on are in PAGES, so seqs without a SequenceBase page never match
unsafe fn fire(seq: *mut SequenceBase, kind: SeqEvent) {
    let page = unsafe { page_of(seq) } as usize;

    let cbs = PAGES.lock().ok().and_then(|pages| {
        let state = pages.get(&page)?;
        let cbs = state.seq.get(kind.0 as usize)?;

        (!cbs.is_empty()).then(|| snapshot(cbs))
    });

    if let Some(cbs) = cbs {
        unsafe { call(cbs, seq.cast()) };
    }
}

pub fn on_seq_event(page: *mut ViewPage, kind: SeqEvent, cb: Callback) -> Status {
    let kind = kind.0 as usize;

    if kind >= SeqEvent::COUNT || page.is_null() {
        return Status::INVALID_ARG;
    }

    with_page(page, |state| state.seq[kind].push(cb))
        .map_or(Status::INVALID_ARG, |()| Status::OK)
}

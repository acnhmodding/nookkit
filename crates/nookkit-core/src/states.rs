use core::ffi::c_void;
use std::sync::Mutex;

use nookkit_sys::{Callback, StatePhase, Status};
use nookkit_types::game::ui::{SequenceBase, ViewPage};
use nookkit_types::lp::utl::{StateEntry, StateVtable};

use crate::pages::{bury, call, dispatching, live_seq, snapshot, with_page, with_registered};

pub struct StateReg {
    state: i32,
    phase: StatePhase,
    after: bool,
    cb: Callback,
}

/// One hooked state entry, whose vtable points at `vt`. Leaked like ListState: the entry can be
/// run after its page is gone, so a retired hook only forwards to the original
#[repr(C)]
pub struct StateHook {
    vt: StateVtable,
    original: *const StateVtable,
    seq: usize,
    cbs: Mutex<[[Vec<Callback>; 2]; StatePhase::COUNT]>,
}

unsafe impl Send for StateHook {}
unsafe impl Sync for StateHook {}

impl StateHook {
    pub fn retire(&self) {
        let cbs = self
            .cbs
            .lock()
            .map(|mut cbs| core::mem::take(&mut *cbs))
            .unwrap_or_default();

        bury(cbs);
    }
}

fn hook_of(entry: &StateEntry) -> Option<&'static StateHook> {
    let vt = unsafe { entry.vtable.as_ref() }?;

    (vt.enter as *const () == enter as *const ()).then(|| unsafe { &*entry.vtable.cast::<StateHook>() })
}

pub fn on_state(
    page: *mut ViewPage,
    state: i32,
    phase: StatePhase,
    after: bool,
    cb: Callback,
) -> Status {
    if phase.0 as usize >= StatePhase::COUNT || state < 0 {
        return Status::INVALID_ARG;
    }

    let reg = StateReg {
        state,
        phase,
        after,
        cb,
    };

    // Seqs are created after their page is built, so most registrations wait for ready
    match live_seq(page) {
        Some(seq) => unsafe { apply(page, seq as *mut SequenceBase, reg) },
        None => with_page(page, |state| state.states.push(reg))
            .map_or(Status::INVALID_ARG, |()| Status::OK),
    }
}

/// At ready, for the registrations made before the page's seq existed
pub unsafe fn ready(page: *mut ViewPage, seq: *mut SequenceBase) {
    let regs =
        with_registered(page, |state| core::mem::take(&mut state.states)).unwrap_or_default();

    for reg in regs {
        let state = reg.state;
        let status = unsafe { apply(page, seq, reg) };

        if status != Status::OK {
            println!("[NookKit] on_state({state}): {status:?}");
        }
    }
}

unsafe fn apply(page: *mut ViewPage, seq: *mut SequenceBase, reg: StateReg) -> Status {
    let Some(seq_ref) = (unsafe { seq.as_mut() }) else {
        return Status::INVALID_ARG;
    };

    let entry = seq_ref.state_machine.entry(reg.state);
    let Some(entry_ref) = (unsafe { entry.as_mut() }) else {
        return Status::INVALID_ARG;
    };

    // A native vtable here means a new state table, from a new seq or a seq reset
    let hook = match hook_of(entry_ref) {
        Some(hook) => hook,
        None if entry_ref.vtable.is_null() => return Status::INVALID_ARG,
        None => {
            let hook: &'static StateHook = Box::leak(Box::new(StateHook {
                vt: StateVtable { enter, exec, exit },
                original: entry_ref.vtable,
                seq: seq as usize,
                cbs: Mutex::new(Default::default()),
            }));

            entry_ref.vtable = &hook.vt;
            hook
        }
    };

    with_page(page, |state| {
        if !state.state_hooks.iter().any(|h| core::ptr::eq(*h, hook)) {
            state.state_hooks.push(hook);
        }
    });

    let Ok(mut cbs) = hook.cbs.lock() else {
        return Status::INVALID_ARG;
    };

    cbs[reg.phase.0 as usize][reg.after as usize].push(reg.cb);

    Status::OK
}

unsafe fn dispatch(entry: *mut StateEntry, phase: usize, run: impl FnOnce(&StateVtable)) {
    // Only reachable through a hook's vtable
    let Some(hook) = unsafe { entry.as_ref() }.and_then(hook_of) else {
        return;
    };

    // The callbacks after the original are snapshotted before it, and it may close the page
    let _dispatch = dispatching();

    let (before, after) = hook
        .cbs
        .lock()
        .map(|cbs| {
            let [before, after] = &cbs[phase];
            (snapshot(before), snapshot(after))
        })
        .unwrap_or_default();

    let seq = hook.seq as *const c_void;

    unsafe { call(before, seq) };

    if let Some(original) = unsafe { hook.original.as_ref() } {
        run(original);
    }

    unsafe { call(after, seq) };
}

unsafe extern "C" fn enter(entry: *mut StateEntry) {
    unsafe { dispatch(entry, 0, |original| (original.enter)(entry)) };
}

unsafe extern "C" fn exec(entry: *mut StateEntry) {
    unsafe { dispatch(entry, 1, |original| (original.exec)(entry)) };
}

unsafe extern "C" fn exit(entry: *mut StateEntry, next: i32) {
    unsafe { dispatch(entry, 2, |original| (original.exit)(entry, next)) };
}

use core::ffi::c_void;
use std::collections::BTreeMap;
use std::sync::Mutex;

use nookkit_sys::{Callback, STATE_PHASES, Status};
use nookkit_types::game::ui::{SequenceBase, ViewPage};
use nookkit_types::lp::utl::{StateEntry, StateVtable};

use crate::pages::{call, live_build, live_seq, snapshot, with_page};

pub struct StateReg {
    state: i32,
    phase: u32,
    after: bool,
    cb: Callback,
}

// One hooked state entry. Never removed: a live entry may still carry HOOKED, and its original
// slots are only known here
struct Hook {
    original: *const StateVtable,
    build: u64,
    seq: usize,
    cbs: [[Vec<Callback>; 2]; STATE_PHASES],
}

unsafe impl Send for Hook {}

static HOOKS: Mutex<BTreeMap<usize, Hook>> = Mutex::new(BTreeMap::new());

static HOOKED: StateVtable = StateVtable { enter, exec, exit };

pub fn on_state(page: *mut ViewPage, state: i32, phase: u32, after: bool, cb: Callback) -> Status {
    if phase as usize >= STATE_PHASES || state < 0 {
        return Status::InvalidArg;
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
            .map_or(Status::InvalidArg, |()| Status::Ok),
    }
}

/// At ready, for the registrations made before the page's seq existed
pub unsafe fn ready(page: *mut ViewPage, seq: *mut SequenceBase) {
    let regs = with_page(page, |state| core::mem::take(&mut state.states)).unwrap_or_default();

    for reg in regs {
        unsafe { apply(page, seq, reg) };
    }
}

unsafe fn apply(page: *mut ViewPage, seq: *mut SequenceBase, reg: StateReg) -> Status {
    let Some(seq_ref) = (unsafe { seq.as_mut() }) else {
        return Status::InvalidArg;
    };

    let entry = seq_ref.state_machine.entry(reg.state);
    let Some(entry_ref) = (unsafe { entry.as_mut() }) else {
        return Status::InvalidArg;
    };

    let build = live_build(page).unwrap_or(0);
    let Ok(mut hooks) = HOOKS.lock() else {
        return Status::InvalidArg;
    };

    // A native vtable here means a new state table, from a new seq or a seq reset
    let hooked = core::ptr::eq(entry_ref.vtable, &HOOKED);
    if !hooked {
        if entry_ref.vtable.is_null() {
            return Status::InvalidArg;
        }

        hooks.insert(
            entry as usize,
            Hook {
                original: entry_ref.vtable,
                build,
                seq: seq as usize,
                cbs: Default::default(),
            },
        );
        entry_ref.vtable = &HOOKED;
    }

    let Some(hook) = hooks.get_mut(&(entry as usize)) else {
        return Status::InvalidArg;
    };

    // The page was built again over a live seq, so its old registrations are gone with the build
    if hook.build != build {
        hook.build = build;
        hook.cbs = Default::default();
    }

    hook.cbs[reg.phase as usize][reg.after as usize].push(reg.cb);

    Status::Ok
}

unsafe fn dispatch(entry: *mut StateEntry, phase: usize, run: impl FnOnce(&StateVtable)) {
    let found = HOOKS.lock().ok().and_then(|hooks| {
        let hook = hooks.get(&(entry as usize))?;
        let [before, after] = &hook.cbs[phase];

        Some((hook.original, hook.seq, snapshot(before), snapshot(after)))
    });

    let Some((original, seq, before, after)) = found else {
        return;
    };

    unsafe { call(before, seq as *const c_void) };

    if let Some(original) = unsafe { original.as_ref() } {
        run(original);
    }

    unsafe { call(after, seq as *const c_void) };
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

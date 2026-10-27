use crate::native::sym;
use core::ffi::c_void;
use core::mem::offset_of;

#[repr(C)]
pub struct StateMachine {
    pub vtable: *const c_void,
    pub current_state: i32,

    /// Execs of the current state so far, reset to 0 on every change
    pub frame: i32,

    pub prev_state: i32,
    pub prev_frame: i32,
    pub next_state: i32,
    pub unk_1c: i32,
    pub has_next: bool,

    /// Set by every change
    pub changed: bool,
    _unk_22: [u8; 0x6],
    pub state_count: i32,
    _unk_2c: [u8; 0x4],
    pub states: *mut c_void,
    _unk_38: [u8; 0x10],
}

const _: () = {
    assert!(offset_of!(StateMachine, current_state) == 0x8);
    assert!(offset_of!(StateMachine, frame) == 0xc);
    assert!(offset_of!(StateMachine, prev_state) == 0x10);
    assert!(offset_of!(StateMachine, next_state) == 0x18);
    assert!(offset_of!(StateMachine, has_next) == 0x20);
    assert!(offset_of!(StateMachine, states) == 0x30);
    assert!(core::mem::size_of::<StateMachine>() == 0x48);
};

crate::native! {
    impl StateMachine {
        pub unsafe fn change_state(&mut self, state: i32)
            = sym::Lp::Utl::StateMachine::changeState;
    }
}

#[repr(C)]
pub struct StateVtable {
    pub enter: unsafe extern "C" fn(entry: *mut StateEntry),
    pub exec: unsafe extern "C" fn(entry: *mut StateEntry),
    pub exit: unsafe extern "C" fn(entry: *mut StateEntry, next: i32),
}

#[repr(C)]
pub struct StateEntry {
    pub vtable: *const StateVtable,
    pub obj: *mut c_void,
    pub enter: [usize; 2],
    pub exec: [usize; 2],
    pub exit: [usize; 2],
}

const _: () = assert!(core::mem::size_of::<StateEntry>() == 0x40);

impl StateMachine {
    /// Null for a state the machine doesn't have
    pub fn entry(&self, state: i32) -> *mut StateEntry {
        if state < 0 || state >= self.state_count || self.states.is_null() {
            return core::ptr::null_mut();
        }

        unsafe { self.states.cast::<StateEntry>().add(state as usize) }
    }
}

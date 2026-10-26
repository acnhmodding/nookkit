use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::native;

#[repr(C)]
pub struct StateMachine {
    pub vtable: *const c_void,
    pub current_state: i32,
    pub prev_state: i32,
    pub unk_10: i32,
    pub unk_14: i32,
    pub next_state: i32,
    pub unk_1c: i32,
    pub has_next: bool,
    pub unk_21: u8,
    _unk_22: [u8; 0x6],
    pub state_count: i32,
    _unk_2c: [u8; 0x4],
    pub states: *mut c_void,
    _unk_38: [u8; 0x10],
}

const _: () = {
    assert!(offset_of!(StateMachine, current_state) == 0x8);
    assert!(offset_of!(StateMachine, states) == 0x30);
    assert!(core::mem::size_of::<StateMachine>() == 0x48);
};

#[native(resolve = crate::native::resolve)]
impl StateMachine {
    #[native(symbol = "Lp::Utl::StateMachine::changeState")]
    pub unsafe fn change_state(&mut self, state: i32);
}

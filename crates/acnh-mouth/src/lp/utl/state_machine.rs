mod native {
    crate::native::native_fns! {
        fn change_state(machine: *mut super::StateMachine, state: u32) = 0x3dbb160;
    }
}

#[repr(C)]
pub struct StateMachine {
    _unk_0: [u8; 0x8],
    pub current_state: u32,
    _unk_c: [u8; 0x3c],
}

const _: () = assert!(core::mem::size_of::<StateMachine>() == 0x48);

impl StateMachine {
    /// Runs the current state's exit and the new state's enter.
    pub unsafe fn change_state(&mut self, state: u32) {
        unsafe { native::change_state(self, state) }
    }
}

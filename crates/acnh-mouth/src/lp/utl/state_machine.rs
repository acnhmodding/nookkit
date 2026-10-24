use native_vtable::native;

#[repr(C)]
pub struct StateMachine {
    _unk_0: [u8; 0x8],
    pub current_state: u32,
    _unk_c: [u8; 0x3c],
}

const _: () = assert!(core::mem::size_of::<StateMachine>() == 0x48);

#[native(resolve = crate::native::resolve)]
impl StateMachine {
    #[native(symbol = "Lp::Utl::StateMachine::changeState")]
    pub unsafe fn change_state(&mut self, state: u32);
}

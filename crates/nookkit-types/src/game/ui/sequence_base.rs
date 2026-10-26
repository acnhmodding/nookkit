use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::game::ui::ViewPage;
use crate::game::ui_sys::{Sequence, VftSequence};
use crate::lp::utl::StateMachine;
use crate::native::{Class, Fn, sym};

#[vtable(path = game::ui::sequence_base::VftSequenceBase)]
pub unsafe trait VftSequenceBase: VftSequence {
    unsafe fn prepare_layout_build(&mut self, params: *mut c_void);
    unsafe fn on_page_layout_built(&mut self, page: *mut ViewPage, layout: *mut c_void);
    unsafe fn on_layout_built(&mut self, layout: *mut c_void);
    unsafe fn on_control_built(&mut self, info: *mut c_void);
    unsafe fn on_parts_built(&mut self, parent: *mut c_void, builder: *mut c_void);
    unsafe fn on_page_created(&mut self, page: *mut ViewPage);
    unsafe fn get_registered_object(&mut self) -> *mut c_void;
    unsafe fn on_initialized(&mut self);
    unsafe fn on_terminate_hook(&mut self);
    unsafe fn on_activate_hook(&mut self);
    unsafe fn on_activated_hook(&mut self);
    unsafe fn on_deactivate_hook(&mut self);
    unsafe fn on_deactivated_hook(&mut self);
    unsafe fn on_start_hook(&mut self);
    unsafe fn on_started_hook(&mut self);
    unsafe fn on_finalize_hook(&mut self);
    unsafe fn on_finalized_hook(&mut self);
    unsafe fn on_update(&mut self);
    unsafe fn on_refer_scene_hook(&mut self);
    unsafe fn on_refer_layout_hook(&mut self);
    unsafe fn request_appear(&mut self);
    unsafe fn request_disappear(&mut self);
    unsafe fn reset(&mut self);
    unsafe fn has_any_key_guide(&self) -> bool;
    unsafe fn has_key_guide(&self, slot: i32) -> bool;
    unsafe fn init_state_machine(&mut self, machine: *mut StateMachine);
    unsafe fn clear_request(&mut self);
    unsafe fn is_silent_focus_on_restore(&mut self) -> u64;
    unsafe fn is_silent_focus_forced(&mut self) -> u64;
    unsafe fn get_ui3d_display_id(&mut self) -> i32;
}

#[repr(C)]
pub struct SequenceBase {
    pub base: Sequence,
    pub state_machine: StateMachine,
    pub requests: [i32; 2],
    pub player: u8,
    _unk_1f9: [u8; 0x7],
    pub request_slot: u8,
    pub is_closing: bool,
    pub skip_unregister: bool,
    _unk_203: [u8; 0x5],
    pub key_guide: *mut c_void,
}

const _: () = {
    assert!(offset_of!(SequenceBase, state_machine) == 0x1a8);
    assert!(offset_of!(SequenceBase, player) == 0x1f8);
    assert!(offset_of!(SequenceBase, request_slot) == 0x200);
    assert!(offset_of!(SequenceBase, key_guide) == 0x208);
    assert!(core::mem::size_of::<SequenceBase>() == 0x210);
    assert!(VftSequenceBaseLayout::SLOT_COUNT == 65);
};

crate::derive_base!(SequenceBase => Sequence);

#[vtable_impl]
unsafe impl VftSequenceBase for SequenceBase {}

unsafe impl Class for SequenceBase {
    const RUNTIME_TYPE_INFO: Fn = sym::game::UI::SequenceBase::getRuntimeTypeInfo.erase();
}

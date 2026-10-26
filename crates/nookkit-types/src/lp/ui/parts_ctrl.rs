use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::lp::ui::{Control, VftControl};
use crate::native::{Class, Fn, sym};
use crate::nn::ui2d::Pane;
use crate::sead::OffsetList;

#[vtable(path = lp::ui::parts_ctrl::VftPartsCtrl)]
pub unsafe trait VftPartsCtrl: VftControl {
    unsafe fn is_fade_in(&self) -> bool;
    unsafe fn is_fade_out(&self) -> bool;
    unsafe fn has_in_out_anims(&self) -> bool;
    unsafe fn play_loop_anim(&mut self);
    unsafe fn play_current_loop_anim(&mut self);
    unsafe fn stop_loop_anim(&mut self);
    unsafe fn stop_current_loop_anim(&mut self);
    unsafe fn play_loop_random_anim(&mut self);
    unsafe fn stop_loop_random_anim(&mut self);
    unsafe fn play_loop_anim_for_in(&mut self);
    unsafe fn play_wait_anim(&mut self);
    unsafe fn play_current_wait_anim(&mut self);
    unsafe fn on_create_clone_parts_ctrl_inner(&mut self);
    unsafe fn on_initialize_x_link_inner(&mut self);
    unsafe fn on_initialized_child(&mut self);
    unsafe fn bind_anims(&mut self, layout: *mut c_void, anims: *mut c_void);
    unsafe fn bind_anim_tags(&mut self, anims: *mut c_void);
    unsafe fn on_built(&mut self);
}

#[repr(C, packed(4))]
pub struct PartsCtrl {
    pub base: Control,
    pub in_out_state: *mut c_void,
    pub loop_anim_set: *mut c_void,
    pub wait_anim: *mut c_void,
    pub region_anim_set: *mut c_void,
    pub root_pane: *mut Pane,
    pub list_1d8: *mut OffsetList,
    pub ptrs_1e0: [*mut c_void; 3],
    pub flag_1f8: u8,
    _unk_1f9: [u8; 0x3],
}

const _: () = {
    assert!(offset_of!(PartsCtrl, root_pane) == 0x1d0);
    assert!(offset_of!(PartsCtrl, flag_1f8) == 0x1f8);
    assert!(core::mem::size_of::<PartsCtrl>() == 0x1fc);
    assert!(VftPartsCtrlLayout::SLOT_COUNT == 30);
};

crate::derive_base!(PartsCtrl => Control);

#[vtable_impl]
unsafe impl VftPartsCtrl for PartsCtrl {}

unsafe impl Class for PartsCtrl {
    const RUNTIME_TYPE_INFO: Fn = sym::Lp::UI::PartsCtrl::getRuntimeTypeInfo.erase();
}

impl PartsCtrl {
    pub unsafe fn pane(&mut self) -> Option<&mut Pane> {
        unsafe { self.root_pane.as_mut() }
    }
}

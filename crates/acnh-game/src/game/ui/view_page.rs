use core::ffi::{CStr, c_char, c_void};
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::game::ui::{ControlEvent, LayoutBuiltEvent};
use crate::lp::ui::{Control, PriorityPage, VftPage, VftPageBase as _};
use crate::native::Class;
use crate::sead::{Delegate, OffsetList};

#[vtable(path = game::ui::view_page::VftViewPage)]
pub unsafe trait VftViewPage: VftPage {
    unsafe fn process_pending_layout_in(&mut self);
    unsafe fn process_pending_layout_out_ready(&mut self);
    unsafe fn on_initialize(&mut self);
    unsafe fn on_layout_built(&mut self, parent: *mut Control, builder: *mut c_void);
    unsafe fn on_layout_built_post(&mut self, event: *mut LayoutBuiltEvent);
    unsafe fn on_control_built(&mut self, control: *mut Control);
    unsafe fn pre_wake(&mut self);
    unsafe fn post_wake(&mut self);
    unsafe fn pre_sleep(&mut self);
    unsafe fn post_sleep(&mut self);
    unsafe fn pre_calc(&mut self);
    unsafe fn post_calc(&mut self);
    unsafe fn post_animate(&mut self);
    unsafe fn play_layout_in(&mut self, index: u32);
    unsafe fn play_layout_out(&mut self, index: u32);
    unsafe fn on_layout_out_ready(&mut self);
    unsafe fn is_layout_playing_in(&self, index: u32) -> bool;
    unsafe fn is_layout_playing_out(&self, index: u32) -> bool;
    unsafe fn is_layout_out_ready(&self, index: u32) -> bool;
    unsafe fn is_layout_in_ready(&self, index: u32) -> bool;
    unsafe fn vf_158_ret0(&mut self) -> u64;
    unsafe fn vf_160_ret0(&mut self) -> u64;
    unsafe fn get_layout_name_at(&self, index: u32) -> *const c_char;
    unsafe fn get_layout_name(&self) -> *const c_char;
    unsafe fn vf_178_retnull(&mut self) -> *mut c_void;
    unsafe fn vf_180_retnull(&mut self) -> *mut c_void;
    unsafe fn get_key_binding_handler(&mut self) -> *mut c_void;
    unsafe fn get_extra_key_binding_handler(&mut self) -> *mut c_void;
    unsafe fn vf_198_ret1(&mut self) -> u64;
    unsafe fn on_control_created(&mut self, control: *mut Control);
    unsafe fn get_control_group_index(&mut self) -> i32;
    unsafe fn get_key_binding_index_of(&mut self) -> i32;
    unsafe fn make_control_event(&mut self, event: *mut ControlEvent, control: *mut Control);
    unsafe fn prepare_control_event(&mut self, event: *mut ControlEvent);
    unsafe fn should_dispatch_control_event(&mut self, event: *mut ControlEvent) -> bool;
    unsafe fn on_control_event_dispatched(&mut self, event: *mut ControlEvent);
    unsafe fn make_key_event(&mut self, event: *mut c_void, kind: u32, arg: *mut c_void);
    unsafe fn prepare_key_event(&mut self);
    unsafe fn should_dispatch_key_event(&mut self) -> bool;
    unsafe fn on_key_event_dispatched(&mut self);
    unsafe fn create_helper_1f8(&mut self) -> *mut c_void;
    unsafe fn create_key_binding_controller(&mut self) -> *mut c_void;
    unsafe fn make_builder_event(&mut self, event: *mut c_void, a: u32, b: u32);
    unsafe fn prepare_builder_event(&mut self);
    unsafe fn should_dispatch_builder_event(&mut self) -> bool;
    unsafe fn on_builder_event_dispatched(&mut self);
    unsafe fn make_analog_event(&mut self, event: *mut c_void, a: *mut c_void, b: *mut c_void);
    unsafe fn prepare_analog_event(&mut self);
    unsafe fn should_dispatch_analog_event(&mut self) -> bool;
    unsafe fn on_analog_event_dispatched(&mut self);
    unsafe fn is_control_selectable(&mut self, control: *mut Control) -> bool;
}

#[repr(C)]
pub struct EventTable {
    pub slots: u32,
    pub kinds: u32,

    /// `[kinds][slots]`, the game takes the first free slot of a kind
    pub delegates: *mut Delegate,
}

impl EventTable {
    pub unsafe fn kind(&mut self, kind: u32) -> Option<&mut [Delegate]> {
        if self.delegates.is_null() || self.slots == 0 || kind >= self.kinds {
            return None;
        }

        let slots = self.slots as usize;

        unsafe {
            Some(core::slice::from_raw_parts_mut(
                self.delegates.add(kind as usize * slots),
                slots,
            ))
        }
    }
}

#[repr(C)]
pub struct ViewPage {
    pub base: PriorityPage,
    pub layout_count: u32,
    _unk_3f4: [u8; 0x4],
    pub root_controls: *mut *mut Control,
    pub root_pane_count: u32,
    _unk_404: [u8; 0x4],
    pub root_panes: *mut *mut c_void,
    pub on_control_created_delegate: *mut Delegate,
    pub on_layout_built_delegate: *mut Delegate,
    pub on_control_built_delegate: *mut Delegate,
    pub on_control_event_delegate: *mut Delegate,

    /// Kind 0 is focus, kind 5 decide
    pub control_events: EventTable,

    pub on_builder_event_delegate: *mut Delegate,
    pub builder_event_count: u32,
    _unk_44c: [u8; 0x4],
    pub builder_event_table: *mut Delegate,
    pub key_events: EventTable,
    pub key_events_alt: EventTable,
    pub analog_events: EventTable,
    pub on_prior_pre_input_delegate: Delegate,
    pub on_pre_input_delegate: Delegate,
    pub on_prior_post_input_delegate: Delegate,
    pub on_post_input_delegate: Delegate,
    pub pre_input_pass_count: i32,
    pub post_input_pass_count: i32,
    pub pending_in_count: u32,
    _unk_514: [u8; 0x4],
    pub pending_in: *mut u8,
    pub pending_out_ready_notify_count: u32,
    _unk_524: [u8; 0x4],
    pub pending_out_ready_notify: *mut u8,
    pub build_flags: u32,
    _unk_534: [u8; 0x4],
    pub list_538: OffsetList,
    pub key_binding_controllers: OffsetList,
}

const _: () = {
    assert!(offset_of!(ViewPage, layout_count) == 0x3f0);
    assert!(offset_of!(ViewPage, root_controls) == 0x3f8);
    assert!(offset_of!(ViewPage, control_events) == 0x430);
    assert!(offset_of!(ViewPage, builder_event_table) == 0x450);
    assert!(offset_of!(ViewPage, key_events) == 0x458);
    assert!(offset_of!(ViewPage, analog_events) == 0x478);
    assert!(offset_of!(ViewPage, on_prior_pre_input_delegate) == 0x488);
    assert!(offset_of!(ViewPage, pending_in) == 0x518);
    assert!(offset_of!(ViewPage, key_binding_controllers) == 0x550);
    assert!(core::mem::size_of::<ViewPage>() == 0x568);
    assert!(VftViewPageLayout::SLOT_COUNT == 74);
};

crate::derive_base!(ViewPage => PriorityPage);

#[vtable_impl]
unsafe impl VftViewPage for ViewPage {}

unsafe impl Class for ViewPage {
    const RUNTIME_TYPE_INFO: &'static str = "game::UI::ViewPage::getRuntimeTypeInfo";
}

impl ViewPage {
    /// The bflyt name, e.g. `CharaMake`
    pub unsafe fn layout(&self) -> Option<&'static CStr> {
        unsafe {
            let name = self.get_layout_name();
            (!name.is_null()).then(|| CStr::from_ptr(name))
        }
    }

    /// The C++ class name, e.g. `CharaMakePage`
    pub unsafe fn class(&self) -> Option<&'static CStr> {
        unsafe { self.get_class_info().as_ref()?.as_c_str() }
    }

    pub unsafe fn roots(&self) -> &[*mut Control] {
        if self.root_controls.is_null() {
            return &[];
        }

        unsafe { core::slice::from_raw_parts(self.root_controls, self.layout_count as usize) }
    }

    pub unsafe fn root(&self) -> *mut Control {
        unsafe { self.roots() }
            .first()
            .copied()
            .unwrap_or(core::ptr::null_mut())
    }
}

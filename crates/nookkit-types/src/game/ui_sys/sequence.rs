use core::ffi::{CStr, c_void};
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::game::ui::ViewPage;
use crate::lp::ui::{Name, PageHolder};
use crate::native::{Class, Fn, sym};
use crate::sead::runtime_type_info::Interface;
use crate::sead::{CriticalSection, ListNode, OffsetList};

#[vtable(path = game::ui_sys::sequence::VftSequence)]
pub unsafe trait VftSequence {
    unsafe fn check_derived_runtime_type_info(&self, type_info: *const Interface) -> bool;
    unsafe fn get_runtime_type_info(&self) -> *const Interface;

    #[vtable(raw_this)]
    unsafe fn destructor(this: *mut Self);

    #[vtable(raw_this)]
    unsafe fn destructor_deleting(this: *mut Self);

    unsafe fn process_pending_focus_restore(&mut self);
    unsafe fn initialize_thunk(&mut self);
    unsafe fn on_terminate(&mut self);
    unsafe fn create_page_thunk(&mut self, arg: *mut c_void);
    unsafe fn setup_controls_thunk(&mut self, arg: *mut c_void);
    unsafe fn on_created(&mut self);
    unsafe fn on_activate(&mut self);
    unsafe fn on_activated(&mut self);
    unsafe fn on_deactivate(&mut self);
    unsafe fn on_deactivated(&mut self);
    unsafe fn on_start(&mut self);
    unsafe fn on_started(&mut self);
    unsafe fn on_finalize(&mut self);
    unsafe fn on_finalized(&mut self);
    unsafe fn update(&mut self);
    unsafe fn on_refer_scene(&mut self);
    unsafe fn on_refer_layout(&mut self);
    unsafe fn restore_focus(&mut self);
    unsafe fn request_close_and_return_focus(&mut self);
    unsafe fn force_close(&mut self);
    unsafe fn initialize(&mut self);
    unsafe fn create_page(&mut self, arg: *mut c_void);
    unsafe fn setup_controls(&mut self, arg: *mut c_void);
    unsafe fn has_page(&self) -> bool;
    unsafe fn is_appearing(&self) -> bool;
    unsafe fn is_idle(&self) -> bool;
    unsafe fn is_closed(&self) -> bool;
    unsafe fn is_open(&self) -> bool;
    unsafe fn vf_100(&mut self) -> u64;
    unsafe fn vf_108(&mut self) -> u64;
    unsafe fn vf_110(&mut self);
}

#[repr(C)]
pub struct SequenceJobNode {
    pub node: ListNode,
    _unk_10: [u8; 0x20],
    pub priority_group: i32,
    pub priority: i32,
}

const _: () = assert!(core::mem::size_of::<SequenceJobNode>() == 0x38);

#[repr(C)]
pub struct Sequence {
    pub vtable: *const VftSequenceLayout,
    pub job_delegate_count: i32,
    _unk_c: [u8; 0x4],
    pub job_delegates: *mut c_void,
    _unk_18: [u8; 0x10],
    pub child_node: ListNode,
    pub active_node: ListNode,
    pub parent: *mut Sequence,
    pub children: OffsetList,
    pub active_children: OffsetList,
    pub name: Name,
    pub context: *mut c_void,
    pub page_holder: *mut PageHolder,
    pub activation_state: i32,
    _unk_ac: [u8; 0x4],
    pub job_node_0: SequenceJobNode,
    pub job_node_1: SequenceJobNode,
    pub job_mask: u32,
    _unk_124: [u8; 0x4],
    pub jobs_0: OffsetList,
    pub jobs_1: OffsetList,
    pub priority: i32,
    pub priority_group: i32,
    pub layer: i32,
    pub jobs_enabled: bool,
    pub pending_focus_restore: bool,
    pub page_holder_flag_a: bool,
    pub page_holder_flag_b: bool,
    pub cs: CriticalSection,
}

const _: () = {
    assert!(offset_of!(Sequence, child_node) == 0x28);
    assert!(offset_of!(Sequence, name) == 0x80);
    assert!(offset_of!(Sequence, page_holder) == 0xa0);
    assert!(offset_of!(Sequence, job_node_0) == 0xb0);
    assert!(offset_of!(Sequence, job_mask) == 0x120);
    assert!(offset_of!(Sequence, priority) == 0x158);
    assert!(offset_of!(Sequence, cs) == 0x168);
    assert!(core::mem::size_of::<Sequence>() == 0x1a8);
    assert!(VftSequenceLayout::SLOT_COUNT == 35);
};

#[vtable_impl]
unsafe impl VftSequence for Sequence {}

unsafe impl Class for Sequence {
    const RUNTIME_TYPE_INFO: Fn = sym::game::UISys::Sequence::getRuntimeTypeInfo.erase();
}

impl Sequence {
    pub unsafe fn name(&self) -> Option<&CStr> {
        unsafe { self.name.as_c_str() }
    }

    pub unsafe fn children(&self) -> impl Iterator<Item = *mut Sequence> + '_ {
        unsafe { self.children.iter() }
    }

    pub unsafe fn pages(&self) -> impl Iterator<Item = *mut ViewPage> + '_ {
        unsafe { self.page_holder.as_ref() }
            .into_iter()
            .flat_map(|holder| unsafe { holder.pages() })
    }

    /// The first page this sequence created
    pub unsafe fn page(&self) -> *mut ViewPage {
        unsafe { self.pages() }
            .next()
            .unwrap_or(core::ptr::null_mut())
    }
}

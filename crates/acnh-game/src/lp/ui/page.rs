use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::{VTableLayout, vtable, vtable_impl};

use crate::lp::ui::{Control, Job, PageBase, VftPageBase};
use crate::sead::{Delegate, OffsetList};

#[vtable(path = lp::ui::page::VftPage)]
pub unsafe trait VftPage: VftPageBase {
    unsafe fn on_post_animate(&mut self);
}

#[repr(C)]
pub struct Page {
    pub base: PageBase,
    pub control_list: OffsetList,
    pub layout_holder_list: OffsetList,
    pub list_1f8: OffsetList,
    pub input_root: *mut c_void,
    pub cstm_value: u32,
    pub flags_21c: u16,
    pub flag_21e: u8,
    _unk_21f: u8,
    pub animate_job: Job,
    pub animate_delegate: Delegate,
    pub update_job: Job,
    pub update_delegate: Delegate,
    pub xlink_job: Job,
    pub xlink_delegate: Delegate,
}

const _: () = {
    assert!(offset_of!(Page, input_root) == 0x210);
    assert!(offset_of!(Page, animate_job) == 0x220);
    assert!(offset_of!(Page, xlink_delegate) == 0x3c8);
    assert!(core::mem::size_of::<Page>() == 0x3e8);
    assert!(VftPageLayout::SLOT_COUNT == 23);
};

crate::derive_base!(Page => PageBase);

#[vtable_impl]
unsafe impl VftPage for Page {}

impl Page {
    pub unsafe fn controls(&self) -> impl Iterator<Item = *mut Control> + '_ {
        unsafe { self.control_list.iter() }
    }
}

#[repr(C)]
pub struct PriorityPage {
    pub base: Page,
    pub create_id: i32,
    pub priority: i32,
}

const _: () = assert!(core::mem::size_of::<PriorityPage>() == 0x3f0);

crate::derive_base!(PriorityPage => Page);

#[vtable_impl]
unsafe impl VftPage for PriorityPage {}

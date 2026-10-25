use core::mem::offset_of;

use acnh_game::game::ui::{PageBase, SequenceBase, VftPageBase, VftSequenceBase};
use native_vtable::vtable_impl;

use crate::game::ui::AppUniteBtnCtrl;
use crate::sead::ObjArray;

// The four old mouth buttons
pub const GROUP_MOUTH: u32 = 6;

pub const FACE_PARAM_MOUTH: usize = 5;

pub type FaceParams = [u8; 12];

#[repr(C)]
pub struct IconTexturePool {
    _opaque: [u8; 0],
}

#[repr(C)]
pub struct CharaMakePage {
    pub base: PageBase,
    _unk_570: [u8; 0x28],
    pub nose_buttons: [*mut AppUniteBtnCtrl; 3],
    pub mouth_buttons: [*mut AppUniteBtnCtrl; 4],
    _unk_5d0: [u8; 0x38],
    pub ok_button: *mut AppUniteBtnCtrl,
}

const _: () = {
    assert!(offset_of!(CharaMakePage, nose_buttons) == 0x598);
    assert!(offset_of!(CharaMakePage, mouth_buttons) == 0x5b0);
    assert!(offset_of!(CharaMakePage, ok_button) == 0x608);
};

acnh_game::derive_base!(CharaMakePage => PageBase);

#[vtable_impl]
unsafe impl VftPageBase for CharaMakePage {}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum RequestState {
    None = -1,
    Focus = 2,
    Decide = 3,

    // Links OK up to the focused button of the tab
    UpdateAnchor = 5,
}

pub const CATEGORY_MOUTH: i32 = 6;

#[repr(C)]
pub struct Request {
    pub state: i32,
    pub index: i32,
    pub category: i32,
}

impl Request {
    pub fn is(&self, state: RequestState) -> bool {
        self.state == state as i32
    }

    pub fn post(&mut self, state: RequestState, index: i32, category: i32) {
        if self.is(RequestState::None) {
            *self = Self {
                state: state as i32,
                index,
                category,
            };
        }
    }
}

#[repr(C)]
pub struct CharaMakeSeq {
    pub base: SequenceBase,
    pub page: *mut CharaMakePage,
    _unk_218: [u8; 0xfc],
    pub face_params: FaceParams,
    _unk_320: [u8; 0x8a0],
    pub mouth_values: ObjArray<u32>,
    _unk_bd8: [u8; 0x418],
    pub icon_texture_pool: IconTexturePool,
    _unk_ff0: [u8; 0x1bfdc],
    pub request: Request,
}

const _: () = {
    assert!(offset_of!(CharaMakeSeq, page) == 0x210);
    assert!(offset_of!(CharaMakeSeq, face_params) == 0x314);
    assert!(offset_of!(CharaMakeSeq, mouth_values) == 0xbc0);
    assert!(offset_of!(CharaMakeSeq, icon_texture_pool) == 0xff0);
    assert!(offset_of!(CharaMakeSeq, request) == 0x1cfcc);
};

acnh_game::derive_base!(CharaMakeSeq => SequenceBase);

#[vtable_impl]
unsafe impl VftSequenceBase for CharaMakeSeq {}

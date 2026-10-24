use core::mem::offset_of;

use crate::game::ui::AppUniteBtnCtrl;
use crate::game::ui::chara_make::{FaceParams, IconTexturePool};

mod native {
    use super::{IconPart, PlayerIcon};
    use crate::game::ui::app_unite_btn::ChildList;
    use crate::game::ui::chara_make::FaceParams;

    crate::native::native_fns! {
        fn find(children: *mut ChildList) -> *mut PlayerIcon = 0x2baaf20;
        fn reset_part(part: *mut IconPart) = 0x2fc3694;
        fn render(icon: *mut PlayerIcon, params: *const FaceParams) = 0x2fc37ac;
        fn move_from(icon: *mut PlayerIcon, source: *mut PlayerIcon, params: *const FaceParams) = 0x2fc3ff0;
        fn release(icon: *mut PlayerIcon) = 0x2fc4398;
        fn tick(icon: *mut PlayerIcon) = 0x2fc47d8;
    }
}

#[repr(C)]
pub struct IconPart {
    _unk_0: [u8; 0x50],
}

// Face, hair, eye, pupil, nose, mouth, cheek.
#[repr(C)]
pub struct PlayerIcon {
    _unk_0: [u8; 0x208],
    pub texture_pool: *mut IconTexturePool,
    _unk_210: [u8; 0x18],
    pub parts: [IconPart; 7],
}

const _: () = {
    assert!(offset_of!(PlayerIcon, texture_pool) == 0x208);
    assert!(offset_of!(PlayerIcon, parts) == 0x228);
};

impl PlayerIcon {
    /// The button's icon, prepared the way setupControls does for the eye list's icons.
    pub unsafe fn create(button: &mut AppUniteBtnCtrl, pool: *mut IconTexturePool) -> *mut PlayerIcon {
        unsafe {
            let Some(icon) = native::find(&mut button.children).as_mut() else {
                return core::ptr::null_mut();
            };

            icon.texture_pool = pool;

            for part in &mut icon.parts {
                native::reset_part(part);
            }

            icon
        }
    }

    /// Loads the part textures for `params` and shows them once they're ready.
    pub unsafe fn render(&mut self, params: &FaceParams) {
        unsafe { native::render(self, params) }
    }

    /// Takes over `source`'s textures, loading only the parts that differ.
    pub unsafe fn move_from(&mut self, source: *mut PlayerIcon, params: &FaceParams) {
        unsafe { native::move_from(self, source, params) }
    }

    pub unsafe fn release(&mut self) {
        unsafe { native::release(self) }
    }

    pub unsafe fn tick(&mut self) {
        unsafe { native::tick(self) }
    }
}

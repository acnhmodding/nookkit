use core::mem::offset_of;

use native_vtable::native;

use crate::game::ui::AppUniteBtnCtrl;
use crate::game::ui::app_unite_btn::ChildList;
use crate::game::ui::chara_make::{FaceParams, IconTexturePool};
use crate::native::NativeResult;

#[native(resolve = crate::native::resolve)]
unsafe extern "C" {
    #[native(symbol = "CharaMake_findPlayerIcon")]
    fn find_player_icon(children: *mut ChildList) -> *mut PlayerIcon;
}

#[repr(C)]
pub struct IconPart {
    _unk_0: [u8; 0x50],
}

#[native(resolve = crate::native::resolve)]
impl IconPart {
    #[native(symbol = "PlayerIcon_resetPart")]
    unsafe fn reset(&mut self);
}

#[repr(C)]
pub struct PlayerIcon {
    _unk_0: [u8; 0x208],
    pub texture_pool: *mut IconTexturePool,
    _unk_210: [u8; 0x18],
    // Face, hair, eye, pupil, nose, mouth, cheek
    pub parts: [IconPart; 7],
}

const _: () = {
    assert!(offset_of!(PlayerIcon, texture_pool) == 0x208);
    assert!(offset_of!(PlayerIcon, parts) == 0x228);
};

impl PlayerIcon {
    pub unsafe fn create(
        button: &mut AppUniteBtnCtrl,
        pool: *mut IconTexturePool,
    ) -> NativeResult<*mut PlayerIcon> {
        unsafe {
            let Some(icon) = find_player_icon(&mut button.children)?.as_mut() else {
                return Ok(core::ptr::null_mut());
            };

            icon.texture_pool = pool;

            for part in &mut icon.parts {
                part.reset()?;
            }

            Ok(icon)
        }
    }
}

#[native(resolve = crate::native::resolve)]
impl PlayerIcon {
    #[native(symbol = "PlayerIcon_render")]
    pub unsafe fn render(&mut self, params: *const FaceParams);

    /// Only loads the parts that differ from `source`
    #[native(symbol = "PlayerIcon_moveFrom")]
    pub unsafe fn move_from(&mut self, source: *mut PlayerIcon, params: *const FaceParams);

    #[native(symbol = "PlayerIcon_release")]
    pub unsafe fn release(&mut self);

    #[native(symbol = "PlayerIcon_tick")]
    pub unsafe fn tick(&mut self);
}

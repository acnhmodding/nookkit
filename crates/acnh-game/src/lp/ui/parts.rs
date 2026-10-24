use core::mem::offset_of;

use crate::lp::ui::{Control, VftControlLayout};
use crate::native::{NativeResult, type_info};
use crate::nn::ui2d::Pane;

#[repr(C)]
pub struct PartsCtrl {
    pub vtable: *const VftControlLayout,
    _unk_8: [u8; 0x1c8],
    pub pane: *mut Pane,
}

const _: () = assert!(offset_of!(PartsCtrl, pane) == 0x1d0);

impl PartsCtrl {
    pub unsafe fn cast(control: *mut Control) -> NativeResult<Option<&'static mut Self>> {
        unsafe {
            let Some(control) = control.as_mut() else {
                return Ok(None);
            };

            let parts_type = type_info("Lp::UI::PartsCtrl")?;

            Ok(control
                .is_derived(parts_type)
                .then(|| &mut *(control as *mut Control).cast::<Self>()))
        }
    }
}

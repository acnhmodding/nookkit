use core::mem::offset_of;

use crate::lp::ui::{Control, FocusNode, VftControlLayout};
use crate::native::{NativeResult, type_info};

#[repr(C)]
pub struct BtnPartsCtrl {
    pub vtable: *const VftControlLayout,
    _unk_8: [u8; 0x210],
    pub focus_node: *mut FocusNode,
}

const _: () = assert!(offset_of!(BtnPartsCtrl, focus_node) == 0x218);

impl BtnPartsCtrl {
    pub unsafe fn cast(control: *mut Control) -> NativeResult<Option<&'static mut Self>> {
        unsafe {
            let Some(control) = control.as_mut() else {
                return Ok(None);
            };

            let btn_type = type_info("Lp::UI::BtnPartsCtrl")?;

            Ok(control
                .is_derived(btn_type)
                .then(|| &mut *(control as *mut Control).cast::<Self>()))
        }
    }
}

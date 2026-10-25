use core::ffi::c_void;
use core::mem::offset_of;

use native_vtable::native;

use crate::native::NativeResult;

#[repr(C)]
struct BcsvHeader {
    row_count: u32,
}

#[repr(C)]
struct BcsvResource {
    _unk_0: [u8; 0x8],
    header: *const BcsvHeader,
}

#[repr(C)]
struct BcsvTable {
    _unk_0: [u8; 0xb0],
    resource: *const BcsvResource,
    _unk_b8: [u8; 0x108],
    ordered_count: i32,
}

const _: () = {
    assert!(offset_of!(BcsvTable, resource) == 0xb0);
    assert!(offset_of!(BcsvTable, ordered_count) == 0x1c0);
};

#[repr(C)]
struct CharaMakeParamMgr {
    _unk_0: [u8; 0x38],
    mouth_type: *const BcsvTable,
}

#[native(resolve = crate::offsets::resolve)]
unsafe extern "C" {
    // Creates the manager on `heap` unless it exists already
    #[native(symbol = "chara_make_param_mgr")]
    fn chara_make_param_mgr(heap: *mut c_void) -> *const CharaMakeParamMgr;
}

pub unsafe fn mouth_type_count() -> NativeResult<usize> {
    unsafe {
        let mgr = chara_make_param_mgr(core::ptr::null_mut())?;
        let Some(table) = mgr.as_ref().and_then(|mgr| mgr.mouth_type.as_ref()) else {
            return Ok(0);
        };

        if table.ordered_count > 0 {
            return Ok(table.ordered_count as usize);
        }

        Ok(table
            .resource
            .as_ref()
            .and_then(|resource| resource.header.as_ref())
            .map_or(0, |header| header.row_count as usize))
    }
}

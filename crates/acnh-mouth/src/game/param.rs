use core::mem::offset_of;

use crate::native::{NativeResult, data};

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

/// Same lookup as the game's mouth accessor
pub unsafe fn mouth_type_count() -> NativeResult<usize> {
    unsafe {
        let mgr = *data::<*const CharaMakeParamMgr>("ACNH_Singleton_7104BF00A0::sInstance")?;
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

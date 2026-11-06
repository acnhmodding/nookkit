use core::ffi::c_void;
use core::mem::offset_of;

use crate::bcsv::{Bcsv, Table};
use crate::sead::{CriticalSection, FixedSafeString, Heap, IDisposer};

/// The manager every table registers in once it loads
#[repr(C)]
pub struct BcsvMgr {
    pub vtable: *const c_void,

    /// Each loaded table in the first free slot
    pub tables: [*mut Bcsv; 0x100],
    _unk_808: [u8; 0x20],

    /// The tables that loaded their own file rather than sharing another's rows
    pub own_table_count: i32,
    pub table_count: i32,
    pub unk_name: FixedSafeString<256>,
    _unk_948: [u8; 0x8],

    /// Copied into the load request of every table file
    pub unk_950: *mut c_void,
    _unk_958: [u8; 0x78],
    pub cs: CriticalSection,
    _unk_a10: [u8; 0x98],

    /// The default heap, the one the vtable's fourth slot returns
    pub heap: *mut Heap,
    pub release_version_param: Table,
    pub singleton_disposer: IDisposer,
}

const _: () = {
    assert!(offset_of!(BcsvMgr, tables) == 0x8);
    assert!(offset_of!(BcsvMgr, own_table_count) == 0x828);
    assert!(offset_of!(BcsvMgr, table_count) == 0x82c);
    assert!(offset_of!(BcsvMgr, unk_name) == 0x830);
    assert!(offset_of!(BcsvMgr, unk_950) == 0x950);
    assert!(offset_of!(BcsvMgr, cs) == 0x9d0);
    assert!(offset_of!(BcsvMgr, heap) == 0xaa8);
    assert!(offset_of!(BcsvMgr, release_version_param) == 0xab0);
    assert!(offset_of!(BcsvMgr, singleton_disposer) == 0xc88);
    assert!(core::mem::size_of::<BcsvMgr>() == 0xca8);
};

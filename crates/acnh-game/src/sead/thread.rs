use crate::sead::IDisposer;

#[repr(C)]
pub struct MutexType {
    _data: [u64; 4],
}

#[repr(C)]
pub struct CriticalSection {
    pub base: IDisposer,
    pub mutex: MutexType,
}

const _: () = assert!(core::mem::size_of::<CriticalSection>() == 0x40);

#[repr(C)]
pub struct ReadWriteLock {
    _unk_0: [u8; 0xb8],
}

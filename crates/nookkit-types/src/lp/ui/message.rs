use core::mem::offset_of;

#[repr(C)]
pub struct MessageFile {
    pub binary: *mut MsgStdBinary,
    pub text_count: u32,
}

const _: () = assert!(core::mem::size_of::<MessageFile>() == 0x10);

#[repr(C)]
pub struct MsgStdBinary {
    pub data: *const u8,
    pub file_size: u64,
    pub encoding: u8,
    _unk_11: u8,
    pub block_count: u16,
    _unk_14: [u8; 0x4],
    pub blocks: *mut MsgBlock,

    /// Block indices, -1 when the file has no such block
    pub lbl1: i32,

    pub txt2: i32,
    pub atr1: i32,
    pub ato1: i32,
    pub tsy1: i32,
}

const _: () = {
    assert!(core::mem::size_of::<MsgStdBinary>() == 0x38);
    assert!(offset_of!(MsgStdBinary, block_count) == 0x12);
    assert!(offset_of!(MsgStdBinary, blocks) == 0x18);
    assert!(offset_of!(MsgStdBinary, atr1) == 0x28);
    assert!(offset_of!(MsgStdBinary, tsy1) == 0x30);
};

#[repr(C)]
pub struct MsgBlock {
    pub data: *const u8,
    pub magic: [u8; 4],
    pub size: u32,
    _unk_10: u16,
}

const _: () = assert!(core::mem::size_of::<MsgBlock>() == 0x18);

#[repr(C)]
pub struct Atr1Header {
    pub count: u32,
    pub entry_size: u32,
}

impl MsgStdBinary {
    pub unsafe fn block(&self, index: i32) -> Option<&MsgBlock> {
        let index = u16::try_from(index).ok()?;

        if index >= self.block_count || self.blocks.is_null() {
            return None;
        }

        unsafe { self.blocks.add(usize::from(index)).as_ref() }
    }

    /// The byte size of one ATR1 entry, 0 without attributes
    pub unsafe fn attribute_size(&self) -> usize {
        let Some(block) = (unsafe { self.block(self.atr1) }) else {
            return 0;
        };

        if block.size < size_of::<Atr1Header>() as u32 {
            return 0;
        }

        let header = unsafe { block.data.cast::<Atr1Header>().as_ref() };

        header.map_or(0, |header| header.entry_size as usize)
    }
}

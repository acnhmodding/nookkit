#[repr(C)]
pub struct FileHeader {
    pub row_count: u32,
    pub row_size: u32,
    pub field_count: u16,

    /// Adds `ExtHeader`, moving the fields from 0xc to 0x1c
    pub has_ext_header: bool,

    /// 1 for the English name tables, 0 for the Japanese ones
    pub lang: u8,
}

#[repr(C)]
pub struct ExtHeader {
    pub magic: [u8; 4],
    pub version: u32,
    _unk_8: [u8; 0x8],
}

#[repr(C)]
pub struct FieldInfo {
    pub hash: u32,

    /// Within a row
    pub offset: u32,
}

const _: () = {
    assert!(core::mem::size_of::<FileHeader>() == 0xc);
    assert!(core::mem::size_of::<ExtHeader>() == 0x10);
    assert!(core::mem::size_of::<FieldInfo>() == 0x8);
};

impl FileHeader {
    fn fields_offset(&self) -> usize {
        if self.has_ext_header { 0x1c } else { 0xc }
    }

    fn base(&self) -> *const u8 {
        (self as *const Self).cast()
    }

    pub unsafe fn ext_header(&self) -> Option<&ExtHeader> {
        self.has_ext_header
            .then(|| unsafe { &*self.base().add(0xc).cast::<ExtHeader>() })
    }

    pub unsafe fn fields(&self) -> &[FieldInfo] {
        unsafe {
            let fields = self.base().add(self.fields_offset()).cast::<FieldInfo>();
            core::slice::from_raw_parts(fields, self.field_count as usize)
        }
    }

    pub unsafe fn field_offset(&self, hash: u32) -> Option<u32> {
        unsafe {
            self.fields()
                .iter()
                .find(|f| f.hash == hash)
                .map(|f| f.offset)
        }
    }

    /// Null past the end or for a table without rows or fields
    pub unsafe fn row(&self, index: usize) -> *const u8 {
        if index >= self.row_count as usize || self.row_size == 0 || self.field_count == 0 {
            return core::ptr::null();
        }

        let offset = self.fields_offset()
            + self.field_count as usize * size_of::<FieldInfo>()
            + index * self.row_size as usize;

        unsafe { self.base().add(offset) }
    }
}

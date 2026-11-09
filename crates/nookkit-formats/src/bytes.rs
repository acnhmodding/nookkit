#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    #[default]
    Little,
    Big,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{len} bytes at 0x{offset:X} run past the end of the data")]
pub struct OutOfBounds {
    pub offset: usize,
    pub len: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    endian: Endian,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self::with_endian(data, Endian::Little)
    }

    pub fn with_endian(data: &'a [u8], endian: Endian) -> Self {
        Self {
            data,
            pos: 0,
            endian,
        }
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    pub fn seek(&mut self, offset: usize) -> Result<(), OutOfBounds> {
        if offset > self.data.len() {
            return Err(OutOfBounds { offset, len: 0 });
        }

        self.pos = offset;

        Ok(())
    }

    pub fn bytes_at(&self, offset: usize, len: usize) -> Result<&'a [u8], OutOfBounds> {
        let Some(end) = offset.checked_add(len) else {
            return Err(OutOfBounds { offset, len });
        };

        self.data
            .get(offset..end)
            .ok_or(OutOfBounds { offset, len })
    }

    pub fn array_at<const N: usize>(&self, offset: usize) -> Result<[u8; N], OutOfBounds> {
        let Some(&array) = self.bytes_at(offset, N)?.first_chunk() else {
            return Err(OutOfBounds { offset, len: N });
        };

        Ok(array)
    }

    pub fn magic_at(&self, offset: usize, magic: &[u8]) -> Result<bool, OutOfBounds> {
        Ok(self.bytes_at(offset, magic.len())? == magic)
    }

    fn le_at<const N: usize>(&self, offset: usize) -> Result<[u8; N], OutOfBounds> {
        let mut array = self.array_at(offset)?;

        if self.endian == Endian::Big {
            array.reverse();
        }

        Ok(array)
    }

    pub fn u8_at(&self, offset: usize) -> Result<u8, OutOfBounds> {
        self.le_at(offset).map(u8::from_le_bytes)
    }

    pub fn u16_at(&self, offset: usize) -> Result<u16, OutOfBounds> {
        self.le_at(offset).map(u16::from_le_bytes)
    }

    pub fn u24_at(&self, offset: usize) -> Result<u32, OutOfBounds> {
        let [a, b, c] = self.le_at(offset)?;

        Ok(u32::from_le_bytes([a, b, c, 0]))
    }

    pub fn u32_at(&self, offset: usize) -> Result<u32, OutOfBounds> {
        self.le_at(offset).map(u32::from_le_bytes)
    }

    pub fn u64_at(&self, offset: usize) -> Result<u64, OutOfBounds> {
        self.le_at(offset).map(u64::from_le_bytes)
    }

    pub fn read_bytes(&mut self, len: usize) -> Result<&'a [u8], OutOfBounds> {
        let bytes = self.bytes_at(self.pos, len)?;
        self.pos += len;

        Ok(bytes)
    }

    pub fn read_u8(&mut self) -> Result<u8, OutOfBounds> {
        let value = self.u8_at(self.pos)?;
        self.pos += 1;

        Ok(value)
    }

    pub fn read_u16(&mut self) -> Result<u16, OutOfBounds> {
        let value = self.u16_at(self.pos)?;
        self.pos += 2;

        Ok(value)
    }

    pub fn read_u32(&mut self) -> Result<u32, OutOfBounds> {
        let value = self.u32_at(self.pos)?;
        self.pos += 4;

        Ok(value)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Slot<const N: usize>(usize);

#[derive(Debug, Default)]
pub struct Writer {
    buffer: Vec<u8>,
    endian: Endian,
}

impl Writer {
    pub fn new(endian: Endian) -> Self {
        Self {
            buffer: Vec::new(),
            endian,
        }
    }

    pub fn with_capacity(endian: Endian, capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            endian,
        }
    }

    pub fn pos(&self) -> usize {
        self.buffer.len()
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buffer
    }

    fn ordered<const N: usize>(&self, mut le: [u8; N]) -> [u8; N] {
        if self.endian == Endian::Big {
            le.reverse();
        }

        le
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) {
        self.buffer.extend_from_slice(bytes);
    }

    pub fn write_u8(&mut self, value: u8) {
        self.buffer.push(value);
    }

    pub fn write_u16(&mut self, value: u16) {
        let bytes = self.ordered(value.to_le_bytes());
        self.write_bytes(&bytes);
    }

    pub fn write_u24(&mut self, value: u32) {
        let [a, b, c, _] = value.to_le_bytes();
        let bytes = self.ordered([a, b, c]);
        self.write_bytes(&bytes);
    }

    pub fn write_u32(&mut self, value: u32) {
        let bytes = self.ordered(value.to_le_bytes());
        self.write_bytes(&bytes);
    }

    pub fn write_u64(&mut self, value: u64) {
        let bytes = self.ordered(value.to_le_bytes());
        self.write_bytes(&bytes);
    }

    pub fn align(&mut self, alignment: usize) {
        let len = self.buffer.len().next_multiple_of(alignment);
        self.buffer.resize(len, 0);
    }

    pub fn reserve<const N: usize>(&mut self) -> Slot<N> {
        let slot = Slot(self.pos());
        self.buffer.resize(self.pos() + N, 0);

        slot
    }

    fn fill<const N: usize>(&mut self, slot: Slot<N>, le: [u8; N]) {
        let bytes = self.ordered(le);
        self.buffer[slot.0..slot.0 + N].copy_from_slice(&bytes);
    }

    pub fn fill_u16(&mut self, slot: Slot<2>, value: u16) {
        self.fill(slot, value.to_le_bytes());
    }

    pub fn fill_u32(&mut self, slot: Slot<4>, value: u32) {
        self.fill(slot, value.to_le_bytes());
    }
}

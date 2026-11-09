use crate::bcsv::error::{ParseError, WriteError};
use crate::bytes::{OutOfBounds, Reader, Writer};

/// A column: its name's CRC32 and where its cell sits in a row. The cell runs to the next column or
/// the end of the row
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub hash: u32,
    pub offset: usize,
    pub size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Header {
    row_count: u32,
    row_size: u32,
    field_count: u16,

    /// An `ExtHeader` follows, before the fields
    has_ext_header: bool,
    lang: u8,
}

// `ExtHeader`, written back as read
const EXT_HEADER_SIZE: usize = 0x10;
const EXT_MAGIC: &[u8] = b"VSCB";

#[derive(Debug, Clone, Copy)]
struct FieldInfo {
    hash: u32,
    offset: u32,
}

impl FieldInfo {
    fn read(reader: &mut Reader) -> Result<Self, OutOfBounds> {
        Ok(Self {
            hash: reader.read_u32()?,
            offset: reader.read_u32()?,
        })
    }
}

impl Header {
    const SIZE: usize = 0xc;

    fn read(reader: &mut Reader) -> Result<Self, OutOfBounds> {
        Ok(Self {
            row_count: reader.read_u32()?,
            row_size: reader.read_u32()?,
            field_count: reader.read_u16()?,
            has_ext_header: reader.read_u8()? != 0,
            lang: reader.read_u8()?,
        })
    }

    fn to_bytes(self) -> Vec<u8> {
        let mut out = Writer::default();
        out.write_u32(self.row_count);
        out.write_u32(self.row_size);
        out.write_u16(self.field_count);
        out.write_u8(self.has_ext_header as u8);
        out.write_u8(self.lang);

        out.into_bytes()
    }
}

/// A BCSV table read in place, without knowing its column types
#[derive(Debug, Clone)]
pub struct Table<'a> {
    header: Header,

    /// The ext header and fields, written back as read
    head: &'a [u8],
    rows: &'a [u8],
    pub columns: Vec<Column>,
    pub row_size: usize,
    pub row_count: usize,
}

impl<'a> Table<'a> {
    pub fn parse(data: &'a [u8]) -> Result<Self, ParseError> {
        let mut reader = Reader::new(data);
        let header = Header::read(&mut reader)?;
        let row_count = header.row_count as usize;
        let row_size = header.row_size as usize;

        let head_start = reader.pos();

        if header.has_ext_header && !reader.read_bytes(EXT_HEADER_SIZE)?.starts_with(EXT_MAGIC) {
            return Err(ParseError::ExtHeader);
        }

        let mut fields = Vec::with_capacity(header.field_count as usize + 1);

        for _ in 0..header.field_count {
            fields.push(FieldInfo::read(&mut reader)?);
        }

        let head = reader.bytes_at(head_start, reader.pos() - head_start)?;

        let Some(rows_len) = row_count.checked_mul(row_size) else {
            return Err(ParseError::RowsOverflow);
        };

        let rows = reader.read_bytes(rows_len)?;

        // Cell sizes come from the offsets, which the files list in order
        fields.push(FieldInfo {
            hash: 0,
            offset: header.row_size,
        });

        let mut columns = Vec::with_capacity(fields.len());

        for (index, pair) in fields.windows(2).enumerate() {
            let (field, next) = (pair[0], pair[1]);

            let Some(size) = next.offset.checked_sub(field.offset) else {
                return Err(ParseError::ColumnOrder { index });
            };

            columns.push(Column {
                hash: field.hash,
                offset: field.offset as usize,
                size: size as usize,
            });
        }

        Ok(Self {
            header,
            head,
            rows,
            columns,
            row_size,
            row_count,
        })
    }

    pub fn column(&self, hash: u32) -> Option<Column> {
        self.columns.iter().find(|c| c.hash == hash).copied()
    }

    pub fn row(&self, index: usize) -> Option<&'a [u8]> {
        if index >= self.row_count {
            return None;
        }

        self.rows.get(index * self.row_size..)?.get(..self.row_size)
    }

    pub fn rows(&self) -> impl Iterator<Item = &'a [u8]> + '_ {
        (0..self.row_count).filter_map(|index| self.row(index))
    }

    pub fn size(&self, rows: usize) -> usize {
        self.row_at(rows).next_multiple_of(4)
    }

    pub fn row_at(&self, index: usize) -> usize {
        Header::SIZE + self.head.len() + index * self.row_size
    }

    /// Writes the table with `rows` rows, at least its own, to `out`, which is [`Self::size`]
    /// bytes. The rows past the table's own are zeros apart from the offset a row starts with
    pub fn write(&self, rows: usize, out: &mut [u8]) -> Result<(), WriteError> {
        let count = self.row_count;

        if rows < count {
            return Err(WriteError::TooFewRows { rows, count });
        }

        let size = self.size(rows);

        if out.len() != size {
            return Err(WriteError::BufferSize {
                len: out.len(),
                size,
            });
        }

        let header = Header {
            row_count: rows as u32,
            ..self.header
        };

        let (out_header, rest) = out.split_at_mut(Header::SIZE);
        let (out_head, rest) = rest.split_at_mut(self.head.len());
        let (out_rows, rest) = rest.split_at_mut(self.rows.len());
        out_header.copy_from_slice(&header.to_bytes());
        out_head.copy_from_slice(self.head);
        out_rows.copy_from_slice(self.rows);
        rest.fill(0);

        // A row's first four bytes hold its offset in the file unless a column starts there
        let row_offsets = self.columns.iter().all(|c| c.offset >= 4) && self.row_size >= 4;

        if row_offsets {
            for index in count..rows {
                let at = self.row_at(index);

                if let Some(offset) = out[at..].first_chunk_mut::<4>() {
                    *offset = (at as u32).to_le_bytes();
                }
            }
        }

        Ok(())
    }

    pub fn to_bytes(&self, rows: usize) -> Result<Vec<u8>, WriteError> {
        let mut out = vec![0; self.size(rows)];
        self.write(rows, &mut out)?;

        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two columns, a u16 at 4 and a u8 padded to 4 at 8, with one row
    pub(crate) fn small() -> Vec<u8> {
        let mut data = Writer::default();
        data.write_u32(1);
        data.write_u32(12);
        data.write_u16(2);
        data.write_bytes(&[0, 1]);

        for (hash, offset) in [(0x1111_1111, 4), (0x2222_2222, 8)] {
            data.write_u32(hash);
            data.write_u32(offset);
        }

        data.write_bytes(&[0x1c, 0, 0, 0, 7, 0, 0, 0, 3, 0, 0, 0]);

        data.into_bytes()
    }

    #[test]
    fn reads_columns_and_rows() {
        let data = small();
        let table = Table::parse(&data).unwrap();

        assert_eq!(
            table.columns,
            vec![
                Column {
                    hash: 0x1111_1111,
                    offset: 4,
                    size: 4
                },
                Column {
                    hash: 0x2222_2222,
                    offset: 8,
                    size: 4
                },
            ]
        );

        assert_eq!(table.row_count, 1);
        assert_eq!(table.row(0), Some(&data[0x1c..]));
        assert_eq!(table.row(1), None);
        assert_eq!(table.to_bytes(1).unwrap(), data);
    }

    #[test]
    fn new_rows_get_their_offset() {
        let data = small();
        let table = Table::parse(&data).unwrap();

        let data = table.to_bytes(2).unwrap();
        assert_eq!(data.len(), 0x34);
        assert_eq!(data[0], 2);
        assert_eq!(&data[0x28..0x2c], &0x28u32.to_le_bytes());
        assert_eq!(&data[0x2c..], &[0; 8]);
    }

    #[test]
    fn checks_what_it_writes_to() {
        let data = small();
        let table = Table::parse(&data).unwrap();

        assert_eq!(
            table.to_bytes(0),
            Err(WriteError::TooFewRows { rows: 0, count: 1 })
        );

        assert_eq!(
            table.write(1, &mut [0; 4]),
            Err(WriteError::BufferSize { len: 4, size: 0x28 })
        );
    }

    #[test]
    fn rejects_truncated_tables() {
        let data = small();
        assert!(Table::parse(&data[..data.len() - 1]).is_err());
        assert!(Table::parse(&data[..8]).is_err());
    }
}

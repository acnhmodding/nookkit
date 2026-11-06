/// A column: its name's CRC32 and where its cell sits in a row. The cell runs to the next column or
/// the end of the row
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Column {
    pub hash: u32,
    pub offset: usize,
    pub size: usize,
}

/// A BCSV table as raw rows, read and written without knowing its column types
#[derive(Debug, Clone)]
pub struct Table {
    /// Everything before the rows, written back as read apart from the row count
    head: Vec<u8>,
    pub columns: Vec<Column>,
    pub row_size: usize,
    pub rows: Vec<Vec<u8>>,
}

const ROW_COUNT: usize = 0x0;
const EXT_HEADER: usize = 0xc;
const COLUMN_INFO: usize = 8;

impl Table {
    pub fn parse(data: &[u8]) -> Option<Self> {
        let u32_at = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?));

        let row_count = u32_at(ROW_COUNT)? as usize;
        let row_size = u32_at(0x4)? as usize;
        let column_count = u16::from_le_bytes(data.get(0x8..0xa)?.try_into().ok()?) as usize;
        let has_ext_header = *data.get(0xa)? != 0;

        let columns_at = if has_ext_header {
            if data.get(0xc..0x10)? != b"VSCB" {
                return None;
            }
            EXT_HEADER + 0x10
        } else {
            EXT_HEADER
        };

        let mut columns = (0..column_count)
            .map(|i| {
                let at = columns_at + i * COLUMN_INFO;
                Some((u32_at(at)?, u32_at(at + 4)? as usize))
            })
            .collect::<Option<Vec<_>>>()?;

        let rows_at = columns_at + column_count * COLUMN_INFO;
        let rows = data
            .get(rows_at..rows_at.checked_add(row_count.checked_mul(row_size)?)?)?
            .chunks(row_size.max(1))
            .take(row_count)
            .map(<[u8]>::to_vec)
            .collect();

        // Cell sizes come from the offsets, which the files list in order
        columns.push((0, row_size));
        let columns = columns
            .windows(2)
            .map(|pair| {
                let ((hash, offset), (_, next)) = (pair[0], pair[1]);
                Some(Column {
                    hash,
                    offset,
                    size: next.checked_sub(offset)?,
                })
            })
            .collect::<Option<Vec<_>>>()?;

        Some(Self {
            head: data[..rows_at].to_vec(),
            columns,
            row_size,
            rows,
        })
    }

    pub fn column(&self, hash: u32) -> Option<Column> {
        self.columns.iter().find(|c| c.hash == hash).copied()
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = self.head.clone();
        out[ROW_COUNT..ROW_COUNT + 4].copy_from_slice(&(self.rows.len() as u32).to_le_bytes());

        // A row's first four bytes hold its offset in the file unless a column starts there
        let row_offsets = self.columns.iter().all(|c| c.offset >= 4) && self.row_size >= 4;

        for row in &self.rows {
            let at = out.len() as u32;
            out.extend_from_slice(row);
            if row_offsets {
                let start = out.len() - row.len();
                out[start..start + 4].copy_from_slice(&at.to_le_bytes());
            }
        }

        out.resize(out.len().next_multiple_of(4), 0);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two columns, a u16 at 4 and a u8 padded to 4 at 8, with one row
    pub(crate) fn small() -> Vec<u8> {
        let mut data = Vec::new();
        data.extend_from_slice(&1u32.to_le_bytes());
        data.extend_from_slice(&12u32.to_le_bytes());
        data.extend_from_slice(&2u16.to_le_bytes());
        data.extend_from_slice(&[0, 1]);
        for (hash, offset) in [(0x1111_1111u32, 4u32), (0x2222_2222, 8)] {
            data.extend_from_slice(&hash.to_le_bytes());
            data.extend_from_slice(&offset.to_le_bytes());
        }
        data.extend_from_slice(&[0x1c, 0, 0, 0, 7, 0, 0, 0, 3, 0, 0, 0]);
        data
    }

    #[test]
    fn reads_columns_and_rows() {
        let table = Table::parse(&small()).unwrap();
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
        assert_eq!(table.rows.len(), 1);
        assert_eq!(table.to_bytes(), small());
    }

    #[test]
    fn new_rows_get_their_offset() {
        let mut table = Table::parse(&small()).unwrap();
        table.rows.push(vec![0; 12]);

        let data = table.to_bytes();
        assert_eq!(data[0], 2);
        assert_eq!(&data[0x28..0x2c], &0x28u32.to_le_bytes());
    }

    #[test]
    fn rejects_truncated_tables() {
        let data = small();
        assert!(Table::parse(&data[..data.len() - 1]).is_none());
        assert!(Table::parse(&data[..8]).is_none());
    }
}

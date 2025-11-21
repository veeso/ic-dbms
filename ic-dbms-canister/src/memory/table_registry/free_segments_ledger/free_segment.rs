use crate::memory::{DataSize, Encode, MSize, MemoryResult, Page, PageOffset};

/// [`Encode`]able representation of a table that keeps track of [`FreeSegment`]s.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct FreeSegmentsTable {
    pub records: Vec<FreeSegment>,
}

/// Represents a free segment's metadata.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct FreeSegment {
    /// The page where the free segment was located.
    pub page: Page,
    /// The offset within the page where the free segment was located.
    pub offset: PageOffset,
    /// The size of the free segment.
    pub size: MSize,
}

/// Represents an adjacent free segment, either before or after a given segment.
#[derive(Debug)]
enum AdjacentSegment {
    Before(FreeSegment),
    After(FreeSegment),
}

impl FreeSegmentsTable {
    /// Inserts a new [`FreeSegment`] into the table.
    pub fn insert_free_segment(&mut self, page: Page, offset: PageOffset, size: MSize) {
        // check for adjacent segments and merge if found
        if let Some(adjacent) = self.has_adjacent_segment(page, offset, size) {
            match adjacent {
                AdjacentSegment::Before(seg) => {
                    // Merge with the segment before
                    let new_size = seg.size.saturating_add(size);
                    self.remove(seg.page, seg.offset, seg.size, seg.size);
                    self.insert_free_segment(page, seg.offset, new_size);
                }
                AdjacentSegment::After(seg) => {
                    // Merge with the segment after
                    let new_size = size.saturating_add(seg.size);
                    self.remove(seg.page, seg.offset, seg.size, seg.size);
                    self.insert_free_segment(page, offset, new_size);
                }
            }
        } else {
            // No adjacent segments found, insert as is
            let record = FreeSegment { page, offset, size };
            self.records.push(record);
        }
    }

    /// Finds a free segment that matches the given predicate.
    pub fn find<F>(&self, predicate: F) -> Option<FreeSegment>
    where
        F: Fn(&&FreeSegment) -> bool,
    {
        self.records.iter().find(predicate).copied()
    }

    /// Removes a free segment that matches the given parameters.
    ///
    /// If `used_size` is less than `size`, the old record is removed, but a new record is added
    /// for the remaining free space.
    pub fn remove(&mut self, page: Page, offset: PageOffset, size: MSize, used_size: MSize) {
        if let Some(pos) = self
            .records
            .iter()
            .position(|r| r.page == page && r.offset == offset && r.size == size)
        {
            self.records.swap_remove(pos);

            // If there is remaining space, add a new record for it.
            if used_size < size {
                let remaining_size = size.saturating_sub(used_size);
                let new_offset = offset.saturating_add(used_size);
                let new_record = FreeSegment {
                    page,
                    offset: new_offset,
                    size: remaining_size,
                };
                self.records.push(new_record);
            }
        }
    }

    /// Checks for adjacent free segments before or after the given segment.
    fn has_adjacent_segment(
        &self,
        page: Page,
        offset: PageOffset,
        size: MSize,
    ) -> Option<AdjacentSegment> {
        self.has_adjacent_segment_before(page, offset)
            .or_else(|| self.has_adjacent_segment_after(page, offset, size))
    }

    /// Checks for an adjacent free segment before the given segment.
    fn has_adjacent_segment_before(
        &self,
        page: Page,
        offset: PageOffset,
    ) -> Option<AdjacentSegment> {
        self.find(|r| r.page == page && r.offset.saturating_add(r.size) == offset)
            .map(AdjacentSegment::Before)
    }

    /// Checks for an adjacent free segment after the given segment.
    fn has_adjacent_segment_after(
        &self,
        page: Page,
        offset: PageOffset,
        size: MSize,
    ) -> Option<AdjacentSegment> {
        self.find(|r| r.page == page && r.offset == offset.saturating_add(size))
            .map(AdjacentSegment::After)
    }
}

impl Encode for FreeSegmentsTable {
    const SIZE: DataSize = DataSize::Variable;

    fn size(&self) -> MSize {
        // 4 bytes for the length + size of each record.
        4 + self.records.iter().map(|r| r.size()).sum::<MSize>()
    }

    fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut buffer = Vec::with_capacity(self.size() as usize);

        // Encode the length of the records vector.
        let length = self.records.len() as u32;
        buffer.extend_from_slice(&length.to_le_bytes());

        // Encode each DeletedRecord.
        for record in &self.records {
            buffer.extend_from_slice(&record.encode());
        }

        std::borrow::Cow::Owned(buffer)
    }

    fn decode(data: std::borrow::Cow<[u8]>) -> MemoryResult<Self>
    where
        Self: Sized,
    {
        let length = u32::from_le_bytes(data[0..4].try_into()?);
        let mut records = Vec::with_capacity(length as usize);
        let record_size = FreeSegment::SIZE.get_fixed_size().expect("Should be fixed");

        let mut offset = 4;
        for _ in 0..length {
            let record_data = data[offset as usize..(offset + record_size) as usize]
                .to_vec()
                .into();
            let record = FreeSegment::decode(record_data)?;
            records.push(record);
            offset += record_size;
        }

        Ok(FreeSegmentsTable { records })
    }
}

impl Encode for FreeSegment {
    const SIZE: DataSize = DataSize::Fixed(8); // page (4) + offset (2) + size (2)

    fn size(&self) -> MSize {
        Self::SIZE.get_fixed_size().expect("Should be fixed")
    }

    fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
        let mut buffer = Vec::with_capacity(self.size() as usize);

        buffer.extend_from_slice(&self.page.to_le_bytes());
        buffer.extend_from_slice(&self.offset.to_le_bytes());
        buffer.extend_from_slice(&self.size.to_le_bytes());
        std::borrow::Cow::Owned(buffer)
    }

    fn decode(data: std::borrow::Cow<[u8]>) -> MemoryResult<Self>
    where
        Self: Sized,
    {
        let page = Page::from_le_bytes(data[0..4].try_into()?);
        let offset = PageOffset::from_le_bytes(data[4..6].try_into()?);
        let size = MSize::from_le_bytes(data[6..8].try_into()?);

        Ok(FreeSegment { page, offset, size })
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_should_encode_and_decode_free_segment() {
        let original_record = FreeSegment {
            page: 42,
            offset: 1000,
            size: 256,
        };

        assert_eq!(original_record.size(), 8);
        let encoded = original_record.encode();
        let decoded = FreeSegment::decode(encoded).expect("Decoding failed");

        assert_eq!(original_record, decoded);
    }

    #[test]
    fn test_should_encode_and_decode_free_segments_table() {
        let original_table = FreeSegmentsTable {
            records: vec![
                FreeSegment {
                    page: 1,
                    offset: 100,
                    size: 50,
                },
                FreeSegment {
                    page: 2,
                    offset: 200,
                    size: 75,
                },
            ],
        };

        let encoded = original_table.encode();
        let decoded = FreeSegmentsTable::decode(encoded).expect("Decoding failed");

        assert_eq!(original_table, decoded);
    }

    #[test]
    fn test_should_insert_free_segment() {
        let mut table = FreeSegmentsTable::default();

        table.insert_free_segment(1, 100, 50);
        table.insert_free_segment(2, 200, 75);

        assert_eq!(table.records.len(), 2);
        assert_eq!(table.records[0].page, 1);
        assert_eq!(table.records[1].page, 2);
    }

    #[test]
    fn test_should_find_free_segment() {
        let mut table = FreeSegmentsTable::default();
        table.insert_free_segment(1, 100, 50);
        table.insert_free_segment(2, 200, 75);

        let record = table.find(|r| r.page == 2);
        assert!(record.is_some());
        assert_eq!(record.unwrap().offset, 200);
    }

    #[test]
    fn test_should_remove_free_segment_with_same_size() {
        let mut table = FreeSegmentsTable::default();
        table.insert_free_segment(1, 100, 50);

        table.remove(1, 100, 50, 50);

        assert!(table.records.is_empty());
    }

    #[test]
    fn test_should_remove_free_segment_and_create_remaining() {
        let mut table = FreeSegmentsTable::default();
        table.insert_free_segment(1, 100, 50);

        table.remove(1, 100, 50, 30);

        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].page, 1);
        assert_eq!(table.records[0].offset, 130);
        assert_eq!(table.records[0].size, 20);
    }

    #[test]
    fn test_should_find_adjacent_segment_before() {
        let mut table = FreeSegmentsTable::default();
        table.insert_free_segment(1, 100, 50);

        let adjacent = table.has_adjacent_segment_before(1, 150);
        assert!(adjacent.is_some());
        match adjacent.unwrap() {
            AdjacentSegment::Before(seg) => {
                assert_eq!(seg.page, 1);
                assert_eq!(seg.offset, 100);
                assert_eq!(seg.size, 50);
            }
            _ => panic!("Expected AdjacentSegment::Before"),
        }
    }

    #[test]
    fn test_should_find_adjacent_segment_after() {
        let mut table = FreeSegmentsTable::default();
        table.insert_free_segment(1, 100, 50);

        let adjacent = table.has_adjacent_segment_after(1, 0, 100);
        assert!(adjacent.is_some());
        match adjacent.unwrap() {
            AdjacentSegment::After(seg) => {
                assert_eq!(seg.page, 1);
                assert_eq!(seg.offset, 100);
                assert_eq!(seg.size, 50);
            }
            _ => panic!("Expected AdjacentSegment::After"),
        }
    }

    #[test]
    fn test_should_insert_adjacent_segment() {
        let mut table = FreeSegmentsTable::default();
        table.insert_free_segment(1, 100, 50);
        table.insert_free_segment(1, 150, 50); // Adjacent to the first

        assert_eq!(table.records.len(), 1);
        assert_eq!(table.records[0].page, 1);
        assert_eq!(table.records[0].offset, 100);
        assert_eq!(table.records[0].size, 100); // Merged size
    }
}

mod free_segment;

pub use self::free_segment::FreeSegment;
use self::free_segment::FreeSegmentsTable;
use crate::memory::{Encode, MEMORY_MANAGER, MemoryResult, Page, PageOffset};

/// The free segments ledger keeps track of free segments in the [`FreeSegmentsTable`] registry.
///
/// Free segments can occur either when a record is deleted or
/// when a record is moved to a different location due to resizing after an update.
///
/// Each record tracks:
///
/// - The page number where the record was located
/// - The offset within that page
/// - The size of the free segment
///
/// The responsibilities of this ledger include:
///
/// - Storing metadata about free segments whenever a record is deleted or moved
/// - Find a suitable location for new records by reusing space from free segments
pub struct FreeSegmentsLedger {
    /// The page where the free segments ledger is stored in memory.
    free_segments_page: Page,
    /// Free segments table that holds metadata about free segments.
    table: FreeSegmentsTable,
}

impl FreeSegmentsLedger {
    /// Loads the deleted records ledger from memory
    pub fn load(deleted_records_page: Page) -> MemoryResult<Self> {
        // read from memory
        let table = MEMORY_MANAGER.with_borrow(|mm| mm.read_at(deleted_records_page, 0))?;

        Ok(Self {
            free_segments_page: deleted_records_page,
            table,
        })
    }

    /// Inserts a new [`FreeSegment`] into the ledger with the specified [`Page`], offset, and size.
    ///
    /// The size is calculated based on the size of the record plus the length prefix.
    ///
    /// The table is then written back to memory.
    pub fn insert_free_segment<E>(
        &mut self,
        page: Page,
        offset: PageOffset,
        record: &E,
    ) -> MemoryResult<()>
    where
        E: Encode,
    {
        self.table.insert_free_segment(page, offset, record.size());
        self.write()
    }

    /// Finds a reusable free segment that can accommodate the size of the given record.
    ///
    /// If a suitable free segment is found, it is returned as [`Some<FreeSegment>`].
    /// If no suitable free segment is found, [`None`] is returned.
    pub fn find_reusable_segment<E>(&self, record: &E) -> Option<FreeSegment>
    where
        E: Encode,
    {
        let required_size = record.size();
        self.table.find(|r| r.size >= required_size)
    }

    /// Commits a reused free segment by removing it from the ledger and updating it based on the used size.
    pub fn commit_reused_space<E>(
        &mut self,
        record: &E,
        FreeSegment { page, offset, size }: FreeSegment,
    ) -> MemoryResult<()>
    where
        E: Encode,
    {
        let used_size = record.size();

        self.table.remove(page, offset, size, used_size);
        self.write()
    }

    /// Writes the current state of the free segments table back to memory.
    fn write(&self) -> MemoryResult<()> {
        MEMORY_MANAGER.with_borrow_mut(|mm| mm.write_at(self.free_segments_page, 0, &self.table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{DataSize, MSize};

    #[test]
    fn test_should_load_free_segments_ledger() {
        // allocate new page
        let page = MEMORY_MANAGER
            .with_borrow_mut(|mm| mm.allocate_page())
            .expect("Failed to allocate page");

        let ledger = FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");
        assert_eq!(ledger.free_segments_page, page);
        assert!(ledger.table.records.is_empty());
    }

    #[test]
    fn test_should_insert_record() {
        // allocate new page
        let page = MEMORY_MANAGER
            .with_borrow_mut(|mm| mm.allocate_page())
            .expect("Failed to allocate page");

        let mut ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");

        let record = TestRecord { data: [0; 100] };

        ledger
            .insert_free_segment(4, 0, &record)
            .expect("Failed to insert deleted record");

        let found_record = ledger
            .table
            .find(|r| r.page == 4 && r.offset == 0 && r.size == record.size());
        assert!(found_record.is_some());

        // verify it's written (reload)
        let reloaded_ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");
        let found_record = reloaded_ledger
            .table
            .find(|r| r.page == 4 && r.offset == 0 && r.size == record.size());
        assert!(found_record.is_some());
    }

    #[test]
    fn test_should_find_suitable_reusable_space() {
        let page = MEMORY_MANAGER
            .with_borrow_mut(|mm| mm.allocate_page())
            .expect("Failed to allocate page");

        let mut ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");

        let record = TestRecord { data: [0; 100] };

        ledger
            .insert_free_segment(4, 0, &record)
            .expect("Failed to insert deleted record");

        let record = TestRecord { data: [0; 100] };
        let reusable_space = ledger.find_reusable_segment(&record);
        assert_eq!(
            reusable_space,
            Some(FreeSegment {
                page: 4,
                offset: 0,
                size: record.size(),
            })
        );
    }

    #[test]
    fn test_should_not_find_suitable_reusable_space() {
        let page = MEMORY_MANAGER
            .with_borrow_mut(|mm| mm.allocate_page())
            .expect("Failed to allocate page");

        let mut ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");

        let record = TestRecord { data: [0; 100] };

        ledger
            .insert_free_segment(4, 0, &record)
            .expect("Failed to insert deleted record");

        let record = BigTestRecord { data: [0; 200] };
        let reusable_space = ledger.find_reusable_segment(&record);
        assert_eq!(reusable_space, None);
    }

    #[test]
    fn test_should_commit_reused_space_without_creating_a_new_record() {
        let page = MEMORY_MANAGER
            .with_borrow_mut(|mm| mm.allocate_page())
            .expect("Failed to allocate page");

        let mut ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");

        let record = TestRecord { data: [0; 100] };

        ledger
            .insert_free_segment(4, 0, &record)
            .expect("Failed to insert deleted record");

        let reusable_space = ledger
            .find_reusable_segment(&record)
            .expect("should find reusable space");

        ledger
            .commit_reused_space(&record, reusable_space)
            .expect("Failed to commit reused space");

        // should be empty
        let record = ledger
            .table
            .find(|r| r.page == 4 && r.offset == 0 && r.size == 100);
        assert!(record.is_none());

        // reload
        let reloaded_ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");
        let record = reloaded_ledger
            .table
            .find(|r| r.page == 4 && r.offset == 0 && r.size == 100);
        assert!(record.is_none());
    }

    #[test]
    fn test_should_commit_reused_space_creating_a_new_record() {
        let page = MEMORY_MANAGER
            .with_borrow_mut(|mm| mm.allocate_page())
            .expect("Failed to allocate page");
        let mut ledger =
            FreeSegmentsLedger::load(page).expect("Failed to load DeletedRecordsLedger");

        let big_record = BigTestRecord { data: [1; 200] };

        ledger
            .insert_free_segment(4, 0, &big_record)
            .expect("Failed to insert deleted record");

        let small_record = TestRecord { data: [0; 100] };
        let reusable_space = ledger
            .find_reusable_segment(&small_record)
            .expect("should find reusable space");

        ledger
            .commit_reused_space(&small_record, reusable_space)
            .expect("Failed to commit reused space");

        // should have a new record for the remaining space
        let record = ledger
            .table
            .find(|r| r.page == 4 && r.offset == 100 && r.size == 100);
        assert!(record.is_some());
    }

    #[derive(Debug, Clone)]
    struct TestRecord {
        data: [u8; 100],
    }

    impl Encode for TestRecord {
        const SIZE: DataSize = DataSize::Fixed(100);

        const ALIGNMENT: MSize = 100;

        fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
            std::borrow::Cow::Borrowed(&self.data)
        }

        fn decode(data: std::borrow::Cow<[u8]>) -> crate::memory::MemoryResult<Self>
        where
            Self: Sized,
        {
            let mut record = TestRecord { data: [0; 100] };
            record.data.copy_from_slice(&data[0..100]);
            Ok(record)
        }

        fn size(&self) -> MSize {
            100
        }
    }

    #[derive(Debug, Clone)]
    struct BigTestRecord {
        data: [u8; 200],
    }

    impl Encode for BigTestRecord {
        const SIZE: DataSize = DataSize::Fixed(200);

        const ALIGNMENT: MSize = 200;

        fn encode(&'_ self) -> std::borrow::Cow<'_, [u8]> {
            std::borrow::Cow::Borrowed(&self.data)
        }

        fn decode(data: std::borrow::Cow<[u8]>) -> crate::memory::MemoryResult<Self>
        where
            Self: Sized,
        {
            let mut record = BigTestRecord { data: [0; 200] };
            record.data.copy_from_slice(&data[0..200]);
            Ok(record)
        }

        fn size(&self) -> MSize {
            200
        }
    }
}

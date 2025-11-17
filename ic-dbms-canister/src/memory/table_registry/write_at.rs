use crate::memory::table_registry::free_segments_ledger::FreeSegment;
use crate::memory::{Page, PageOffset};

/// Indicates where to write a record
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WriteAt {
    /// Write at a previously allocated segment
    ReusedSegment(FreeSegment),
    /// Write at the end of the table
    End(Page, PageOffset),
}

impl WriteAt {
    /// Gets the page where to write the record
    pub fn page(&self) -> Page {
        match self {
            WriteAt::ReusedSegment(segment) => segment.page,
            WriteAt::End(page, _) => *page,
        }
    }

    /// Gets the offset where to write the record
    pub fn offset(&self) -> PageOffset {
        match self {
            WriteAt::ReusedSegment(segment) => segment.offset,
            WriteAt::End(_, offset) => *offset,
        }
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_write_at_free_segment() {
        let reused_segment = FreeSegment {
            page: 1,
            offset: 100,
            size: 50,
        };
        let write_at_reused = WriteAt::ReusedSegment(reused_segment);
        assert_eq!(write_at_reused.page(), 1);
        assert_eq!(write_at_reused.offset(), 100);
    }

    #[test]
    fn test_write_at_end() {
        let write_at_end = WriteAt::End(2, 200);
        assert_eq!(write_at_end.page(), 2);
        assert_eq!(write_at_end.offset(), 200);
    }
}

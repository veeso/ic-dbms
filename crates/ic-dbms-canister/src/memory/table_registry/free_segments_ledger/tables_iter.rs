use ic_dbms_api::prelude::{MemoryResult, Page};

use super::FreeSegmentsTable;

/// An iterator which yields all the [`FreeSegmentsTable`]s.
pub struct TablesIter<'a> {
    /// Tracks the current index.
    index: usize,
    /// The pages to iterate over.
    pages: &'a [Page],
}

impl<'a> TablesIter<'a> {
    /// Creates a new [`TablesIter`].
    pub fn new(pages: &'a [Page]) -> Self {
        Self { index: 0, pages }
    }
}

impl Iterator for TablesIter<'_> {
    type Item = MemoryResult<FreeSegmentsTable>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.pages.len() {
            return None;
        }
        let page = self.pages[self.index];
        self.index += 1;

        // read next page
        Some(FreeSegmentsTable::load(page))
    }
}

#[cfg(test)]
mod tests {
    use ic_dbms_canister::memory::MEMORY_MANAGER;

    use super::*;

    #[test]
    fn test_tables_iter_empty() {
        let pages = vec![];
        let mut iter = TablesIter::new(&pages);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_should_iter_tables() {
        const COUNT: usize = 5;
        let mut pages = Vec::new();
        for _ in 0..COUNT {
            let page = MEMORY_MANAGER
                .with_borrow_mut(|mm| mm.allocate_page().expect("Failed to allocate page"));
            let mut table = FreeSegmentsTable::load(page).expect("Failed to load page");
            // insert a segment
            table
                .insert_free_segment(100 + page as Page, 0, 50)
                .expect("Failed to insert segment");
            pages.push(page);
        }

        let mut iter = TablesIter::new(&pages);
        for expected_page in &pages {
            let table_result = iter.next();
            assert!(table_result.is_some());
            let table = table_result.unwrap().expect("Failed to load table");
            // should have a segment
            let segment = table.find(|_| true).expect("Failed to find segment");
            assert_eq!(segment.page, expected_page + 100);
        }
        assert!(iter.next().is_none());
    }
}

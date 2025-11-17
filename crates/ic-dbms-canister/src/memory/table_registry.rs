mod free_segments_ledger;
mod page_ledger;

use std::marker::PhantomData;

use self::free_segments_ledger::FreeSegmentsLedger;
use self::page_ledger::PageLedger;
use crate::memory::{Encode, MSize, MemoryResult, TableRegistryPage};

/// Each record is prefixed with its length encoded in 2 bytes
const RECORD_LEN_SIZE: MSize = 2;

/// The table registry takes care of storing the records for each table,
/// using the [`FreeSegmentsLedger`] and [`PageLedger`] to derive exactly where to read/write
pub struct TableRegistry<E>
where
    E: Encode,
{
    _marker: PhantomData<E>,
    free_segments_ledger: FreeSegmentsLedger,
    page_ledger: PageLedger,
}

impl<E> TableRegistry<E>
where
    E: Encode,
{
    /// Loads the table registry from memory
    pub fn load(table_pages: TableRegistryPage) -> MemoryResult<Self> {
        Ok(Self {
            _marker: PhantomData,
            free_segments_ledger: FreeSegmentsLedger::load(table_pages.free_segments_page)?,
            page_ledger: PageLedger::load(table_pages.pages_list_page)?,
        })
    }
}

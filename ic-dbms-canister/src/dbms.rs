//! This module exposes all the types related to the DBMS engine.

use crate::dbms::transaction::TransactionId;
use crate::prelude::{Filter, InsertRecord, Query, TableSchema};
use crate::{IcDbmsError, IcDbmsResult};

pub mod query;
pub mod table;
pub mod transaction;
pub mod types;
pub mod value;

/// The main DBMS struct.
///
/// This struct serves as the entry point for interacting with the DBMS engine.
///
/// It provides methods for executing queries.
///
/// - [`Database::select`] - Execute a SELECT query.
/// - [`Database::insert`] - Execute an INSERT query.
/// - [`Database::update`] - Execute an UPDATE query.
/// - [`Database::delete`] - Execute a DELETE query.
pub struct Database {
    /// Id of the loaded transaction, if any.
    transaction: Option<TransactionId>,
}

impl From<TransactionId> for Database {
    fn from(transaction_id: TransactionId) -> Self {
        Self {
            transaction: Some(transaction_id),
        }
    }
}

impl Database {
    /// Load an instance of the [`Database`] for one-shot operations (no transaction).
    pub fn oneshot() -> Self {
        Self { transaction: None }
    }

    /// Load an instance of the [`Database`] within a transaction context.
    pub fn from_transaction(transaction_id: TransactionId) -> Self {
        Self {
            transaction: Some(transaction_id),
        }
    }

    /// Executes a SELECT query and returns the results.
    ///
    /// # Arguments
    ///
    /// - `query` - The SELECT [`Query`] to be executed.
    ///
    /// # Returns
    ///
    /// The returned results are a vector of [`table::TableRecord`] matching the query.
    pub fn select<T>(&self, query: Query<T>) -> IcDbmsResult<Vec<T::Record>>
    where
        T: TableSchema,
    {
        todo!()
    }

    /// Executes an INSERT query.
    ///
    /// # Arguments
    ///
    /// - `record` - The INSERT record to be executed.
    ///
    /// # Returns
    ///
    /// The number of rows inserted.
    pub fn insert<T>(&self, record: T::Insert) -> IcDbmsResult<u64>
    where
        T: TableSchema,
        T::Insert: InsertRecord<Schema = T>,
    {
        todo!()
    }

    /// Executes an UPDATE query.
    ///
    /// # Arguments
    ///
    /// - `record` - The UPDATE record to be executed.
    ///
    /// # Returns
    ///
    /// The number of rows updated.
    pub fn update<T>(&self, record: T::Update) -> IcDbmsResult<u64>
    where
        T: TableSchema,
        T::Update: table::UpdateRecord<Schema = T>,
    {
        todo!()
    }

    /// Executes a DELETE query.
    ///
    /// # Arguments
    ///
    /// - `filter` - An optional [`prelude::Filter`] to specify which records to delete.
    ///
    /// # Returns
    ///
    /// The number of rows deleted.
    pub fn delete<T>(&self, filter: Option<Filter>) -> IcDbmsResult<u64>
    where
        T: TableSchema,
    {
        todo!()
    }

    /// Returns whether the read given record matches the provided filter.
    fn record_matches_filter<T>(&self, record: &T, filter: &Filter) -> IcDbmsResult<bool>
    where
        T: TableSchema,
    {
        let values = record.to_values();
        filter.matches(&values).map_err(IcDbmsError::from)
    }
}

#[cfg(test)]
mod tests {

    use candid::Nat;

    use super::*;

    #[test]
    fn test_should_init_dbms() {
        let dbms = Database::oneshot();
        assert!(dbms.transaction.is_none());

        let tx_dbms = Database::from_transaction(Nat::from(1u64));
        assert!(tx_dbms.transaction.is_some());
    }
}

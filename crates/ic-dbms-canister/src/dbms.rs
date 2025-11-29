//! This module exposes all the types related to the DBMS engine.

use crate::dbms::table::{ColumnDef, TableRecord};
use crate::dbms::transaction::TransactionId;
use crate::dbms::value::Value;
use crate::memory::{NextRecord, SCHEMA_REGISTRY, TableRegistry};
use crate::prelude::{
    Filter, InsertRecord, Query, TRANSACTION_SESSION, TableError, TableSchema, TransactionError,
};
use crate::{IcDbmsError, IcDbmsResult};

pub mod query;
pub mod table;
pub mod transaction;
pub mod types;
pub mod value;

/// Default capacity limit for SELECT queries.
const DEFAULT_SELECT_LIMIT: usize = 128;

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
        // load table registry
        let table_registry = self.load_table_registry::<T>()?;
        // read table
        let mut table_reader = table_registry.read();
        // prepare results vector
        let mut results = Vec::with_capacity(query.limit.unwrap_or(DEFAULT_SELECT_LIMIT));
        // iter and select
        let mut count = 0;

        while let Some(NextRecord { record, .. }) = table_reader.try_next()? {
            // convert record to values
            let values = record.to_values();
            // check whether it matches the filter
            if let Some(filter) = &query.filter {
                if !self.record_matches_filter(&values, filter)? {
                    continue;
                }
            }
            // filter matched, check limit and offset
            count += 1;
            // check whether is before offset
            if query.offset.is_some_and(|offset| count <= offset) {
                continue;
            }
            // get queried fields
            let values = self.select_queried_fields::<T>(values, &query)?;
            // convert to record
            let record = T::Record::from_values(&values);
            // push to results
            results.push(record);
            // check whether reached limit
            if query.limit.is_some_and(|limit| results.len() >= limit) {
                break;
            }
        }

        Ok(results)
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
        // TODO: check whether we are in a transaction context
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
        // TODO: check whether we are in a transaction context
        // TODO: cascade for foreign keys
        todo!()
    }

    /// Commits the current transaction.
    pub fn commit(&self) -> IcDbmsResult<()> {
        todo!();
    }

    /// Rolls back the current transaction.
    pub fn rollback(&self) -> IcDbmsResult<()> {
        let Some(txid) = self.transaction.as_ref() else {
            return Err(IcDbmsError::Transaction(
                TransactionError::NoActiveTransaction,
            ));
        };

        TRANSACTION_SESSION.with_borrow_mut(|ts| ts.close_transaction(txid));
        Ok(())
    }

    /// Returns whether the read given record matches the provided filter.
    fn record_matches_filter(
        &self,
        record_values: &[(ColumnDef, Value)],
        filter: &Filter,
    ) -> IcDbmsResult<bool> {
        filter.matches(record_values).map_err(IcDbmsError::from)
    }

    /// Select only the queried fields from the given record values.
    fn select_queried_fields<T>(
        &self,
        mut record_values: Vec<(ColumnDef, Value)>,
        query: &Query<T>,
    ) -> IcDbmsResult<Vec<(ColumnDef, Value)>>
    where
        T: TableSchema,
    {
        // short-circuit if all selected
        if query.all_selected() {
            return Ok(record_values);
        }
        record_values.retain(|(col_def, _)| query.columns().contains(&col_def.name));
        // TODO: handle eager relations
        Ok(record_values)
    }

    /// Load the table registry for the given table schema.
    fn load_table_registry<T>(&self) -> IcDbmsResult<TableRegistry<T>>
    where
        T: TableSchema,
    {
        // get pages of the table registry from schema registry
        let registry_pages = SCHEMA_REGISTRY
            .with_borrow(|schema| schema.table_registry_page::<T>())
            .ok_or(IcDbmsError::Table(TableError::TableNotFound))?;

        TableRegistry::load(registry_pages).map_err(IcDbmsError::from)
    }
}

#[cfg(test)]
mod tests {

    use candid::Nat;

    use super::*;
    use crate::tests::{USERS_FIXTURES, User, load_fixtures};

    #[test]
    fn test_should_init_dbms() {
        let dbms = Database::oneshot();
        assert!(dbms.transaction.is_none());

        let tx_dbms = Database::from_transaction(Nat::from(1u64));
        assert!(tx_dbms.transaction.is_some());
    }

    #[test]
    fn test_should_select_all_users() {
        load_fixtures();
        let dbms = Database::oneshot();
        let query = Query::<User>::builder().all().build();
        let users = dbms.select(query).expect("failed to select users");

        assert_eq!(users.len(), USERS_FIXTURES.len());
        // check if all users all loaded
        for (i, user) in users.iter().enumerate() {
            assert_eq!(user.id.expect("should have id").0 as usize, i);
            assert_eq!(
                user.name.as_ref().expect("should have name").0,
                USERS_FIXTURES[i]
            );
        }
    }

    #[test]
    fn test_should_select_users_with_offset_and_limit() {
        load_fixtures();
        let dbms = Database::oneshot();
        let query = Query::<User>::builder().offset(2).limit(3).build();
        let users = dbms.select(query).expect("failed to select users");

        assert_eq!(users.len(), 3);
        // check if correct users are loaded
        for (i, user) in users.iter().enumerate() {
            let expected_index = i + 2;
            assert_eq!(user.id.expect("should have id").0 as usize, expected_index);
            assert_eq!(
                user.name.as_ref().expect("should have name").0,
                USERS_FIXTURES[expected_index]
            );
        }
    }

    #[test]
    fn test_should_select_users_with_offset_and_filter() {
        load_fixtures();
        let dbms = Database::oneshot();
        let query = Query::<User>::builder()
            .offset(1)
            .and_where(Filter::gt("id", Value::Uint32(4.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");

        assert_eq!(users.len(), 4);
        // check if correct users are loaded
        for (i, user) in users.iter().enumerate() {
            let expected_index = i + 6;
            assert_eq!(user.id.expect("should have id").0 as usize, expected_index);
            assert_eq!(
                user.name.as_ref().expect("should have name").0,
                USERS_FIXTURES[expected_index]
            );
        }
    }

    #[test]
    fn test_should_select_queried_fields() {
        let dbms = Database::oneshot();

        let record_values = User::columns()
            .iter()
            .cloned()
            .zip(vec![
                Value::Uint32(1.into()),
                Value::Text("Alice".to_string().into()),
            ])
            .collect::<Vec<(ColumnDef, Value)>>();

        let query: Query<User> = Query::builder().field("name").build();
        let selected_fields = dbms
            .select_queried_fields::<User>(record_values, &query)
            .expect("failed to select queried fields");

        assert_eq!(selected_fields.len(), 1);
        assert_eq!(selected_fields[0].0.name, "name");
        assert_eq!(
            selected_fields[0].1,
            Value::Text("Alice".to_string().into())
        );
    }

    #[test]
    fn test_should_get_whether_record_matches_filter() {
        let dbms = Database::oneshot();

        let record_values = User::columns()
            .iter()
            .cloned()
            .zip(vec![
                Value::Uint32(1.into()),
                Value::Text("Alice".to_string().into()),
            ])
            .collect::<Vec<(ColumnDef, Value)>>();
        let filter = Filter::eq("name", Value::Text("Alice".to_string().into()));

        let matches = dbms
            .record_matches_filter(&record_values, &filter)
            .expect("failed to match");
        assert!(matches);

        let non_matching_filter = Filter::eq("name", Value::Text("Bob".to_string().into()));
        let non_matches = dbms
            .record_matches_filter(&record_values, &non_matching_filter)
            .expect("failed to match");
        assert!(!non_matches);
    }

    #[test]
    fn test_should_load_table_registry() {
        init_user_table();

        let dbms = Database::oneshot();
        let table_registry = dbms.load_table_registry::<User>();
        assert!(table_registry.is_ok());
    }

    fn init_user_table() {
        SCHEMA_REGISTRY
            .with_borrow_mut(|sr| sr.register_table::<User>())
            .expect("failed to register `User` table");
    }
}

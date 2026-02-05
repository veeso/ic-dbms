//! This module exposes all the types related to the DBMS engine.

pub mod integrity;
pub mod referenced_tables;
pub mod schema;
pub mod transaction;

use ic_dbms_api::prelude::{
    ColumnDef, DataTypeKind, Database, DeleteBehavior, Filter, ForeignFetcher, ForeignKeyDef,
    IcDbmsError, IcDbmsResult, InsertRecord, OrderDirection, Query, QueryError, TableColumns,
    TableError, TableRecord, TableSchema, TransactionError, TransactionId, UpdateRecord, Value,
    ValuesSource,
};

use crate::dbms::transaction::{DatabaseOverlay, Transaction, TransactionOp};
use crate::memory::{SCHEMA_REGISTRY, TableRegistry};
use crate::prelude::{DatabaseSchema, TRANSACTION_SESSION};
use crate::utils::trap;

/// Default capacity for SELECT queries.
const DEFAULT_SELECT_CAPACITY: usize = 128;

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
/// - [`Database::commit`] - Commit the current transaction.
/// - [`Database::rollback`] - Rollback the current transaction.
///
/// The `transaction` field indicates whether the instance is operating within a transaction context.
/// The [`Database`] can be instantiated for one-shot, with [`Database::oneshot`] operations (no transaction),
/// or within a transaction context with [`Database::from_transaction`].
///
/// If a transaction is active, all operations will be part of that transaction until it is committed or rolled back.
pub struct IcDbmsDatabase {
    /// Database schema to perform generic operations, without knowing the concrete table schema at compile time.
    schema: Box<dyn DatabaseSchema>,
    /// Id of the loaded transaction, if any.
    transaction: Option<TransactionId>,
}

impl IcDbmsDatabase {
    /// Load an instance of the [`Database`] for one-shot operations (no transaction).
    pub fn oneshot(schema: impl DatabaseSchema + 'static) -> Self {
        Self {
            schema: Box::new(schema),
            transaction: None,
        }
    }

    /// Load an instance of the [`Database`] within a transaction context.
    pub fn from_transaction(
        schema: impl DatabaseSchema + 'static,
        transaction_id: TransactionId,
    ) -> Self {
        Self {
            schema: Box::new(schema),
            transaction: Some(transaction_id),
        }
    }

    /// Executes a closure with a mutable reference to the current [`Transaction`].
    fn with_transaction_mut<F, R>(&self, f: F) -> IcDbmsResult<R>
    where
        F: FnOnce(&mut Transaction) -> IcDbmsResult<R>,
    {
        let txid = self.transaction.as_ref().ok_or(IcDbmsError::Transaction(
            TransactionError::NoActiveTransaction,
        ))?;

        TRANSACTION_SESSION.with_borrow_mut(|ts| {
            let tx = ts.get_transaction_mut(txid)?;
            f(tx)
        })
    }

    /// Executes a closure with a reference to the current [`Transaction`].
    fn with_transaction<F, R>(&self, f: F) -> IcDbmsResult<R>
    where
        F: FnOnce(&Transaction) -> IcDbmsResult<R>,
    {
        let txid = self.transaction.as_ref().ok_or(IcDbmsError::Transaction(
            TransactionError::NoActiveTransaction,
        ))?;

        TRANSACTION_SESSION.with_borrow_mut(|ts| {
            let tx = ts.get_transaction_mut(txid)?;
            f(tx)
        })
    }

    /// Executes a closure atomically within the database context.
    ///
    /// If the closure returns an error, the changes are rolled back by trapping the canister.
    fn atomic<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&IcDbmsDatabase) -> IcDbmsResult<R>,
    {
        match f(self) {
            Ok(res) => res,
            Err(err) => trap(err.to_string()),
        }
    }

    /// Deletes foreign key related records recursively if the delete behavior is [`DeleteBehavior::Cascade`].
    fn delete_foreign_keys_cascade<T>(
        &self,
        record_values: &[(ColumnDef, Value)],
    ) -> IcDbmsResult<u64>
    where
        T: TableSchema,
    {
        let mut count = 0;
        // verify referenced tables for foreign key constraints
        for (table, columns) in self.schema.referenced_tables(T::table_name()) {
            for column in columns.iter() {
                // prepare filter
                let pk = record_values
                    .iter()
                    .find(|(col_def, _)| col_def.primary_key)
                    .ok_or(IcDbmsError::Query(QueryError::UnknownColumn(
                        column.to_string(),
                    )))?
                    .1
                    .clone();
                // make filter to find records in the referenced table
                let filter = Filter::eq(column, pk);
                let res = self
                    .schema
                    .delete(self, table, DeleteBehavior::Cascade, Some(filter))?;
                count += res;
            }
        }
        Ok(count)
    }

    /// Retrieves the current [`DatabaseOverlay`].
    fn overlay(&self) -> IcDbmsResult<DatabaseOverlay> {
        self.with_transaction(|tx| Ok(tx.overlay().clone()))
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
    ///
    /// It also loads eager relations if any.
    fn select_queried_fields<T>(
        &self,
        mut record_values: Vec<(ColumnDef, Value)>,
        query: &Query<T>,
    ) -> IcDbmsResult<TableColumns>
    where
        T: TableSchema,
    {
        let mut queried_fields = vec![];

        // handle eager relations
        // FIXME: currently we fetch the FK for each record, which is shit.
        // In the future, we should batch fetch foreign keys for all records in the result set.
        for relation in &query.eager_relations {
            let mut fetched = false;
            // iter all foreign key with that table
            for (fk, fk_value) in record_values
                .iter()
                .filter(|(col_def, _)| {
                    col_def
                        .foreign_key
                        .is_some_and(|fk| fk.foreign_table == *relation)
                })
                .map(|(col, value)| {
                    (
                        col.foreign_key.as_ref().expect("cannot be empty"),
                        value.clone(),
                    )
                })
            {
                // get foreign values
                queried_fields.extend(T::foreign_fetcher().fetch(
                    self,
                    relation,
                    fk.local_column,
                    fk_value,
                )?);
                fetched = true;
            }

            if !fetched {
                return Err(IcDbmsError::Query(QueryError::InvalidQuery(format!(
                    "Cannot load relation '{}' for table '{}': no foreign key found",
                    relation,
                    T::table_name()
                ))));
            }
        }

        // short-circuit if all selected
        if query.all_selected() {
            queried_fields.extend(vec![(ValuesSource::This, record_values)]);
            return Ok(queried_fields);
        }
        record_values.retain(|(col_def, _)| query.columns().contains(&col_def.name.to_string()));
        queried_fields.extend(vec![(ValuesSource::This, record_values)]);
        Ok(queried_fields)
    }

    /// Retrieves existing primary keys for records matching the given filter.
    fn existing_primary_keys_for_filter<T>(
        &self,
        filter: Option<Filter>,
    ) -> IcDbmsResult<Vec<Value>>
    where
        T: TableSchema,
    {
        let pk = T::primary_key();
        let fields = self.select(Query::<T>::builder().filter(filter).build())?;
        let pks = fields
            .into_iter()
            .map(|record| {
                record
                    .to_values()
                    .into_iter()
                    .find(|(col_def, _value)| col_def.name == pk)
                    .expect("primary key not found") // this can't fail.
                    .1
            })
            .collect::<Vec<Value>>();

        Ok(pks)
    }

    /// Load the table registry for the given table schema.
    fn load_table_registry<T>(&self) -> IcDbmsResult<TableRegistry>
    where
        T: TableSchema,
    {
        // get pages of the table registry from schema registry
        let registry_pages = SCHEMA_REGISTRY
            .with_borrow(|schema| schema.table_registry_page::<T>())
            .ok_or(IcDbmsError::Table(TableError::TableNotFound))?;

        TableRegistry::load(registry_pages).map_err(IcDbmsError::from)
    }

    /// Sorts the query results based on the specified column and order direction.
    ///
    /// We only sort values which have [`ValuesSource::This`].
    #[allow(clippy::type_complexity)]
    fn sort_query_results(
        &self,
        results: &mut [Vec<(ValuesSource, Vec<(ColumnDef, Value)>)>],
        column: &str,
        direction: OrderDirection,
    ) {
        results.sort_by(|a, b| {
            fn get_value<'a>(
                values: &'a [(ValuesSource, Vec<(ColumnDef, Value)>)],
                column: &str,
            ) -> Option<&'a Value> {
                values
                    .iter()
                    .find(|(source, _)| *source == ValuesSource::This)
                    .and_then(|(_, cols)| {
                        cols.iter()
                            .find(|(col_def, _)| col_def.name == column)
                            .map(|(_, value)| value)
                    })
            }

            let a_value = get_value(a, column);
            let b_value = get_value(b, column);

            match (a_value, b_value) {
                (Some(a_val), Some(b_val)) => match direction {
                    OrderDirection::Ascending => a_val.cmp(b_val),
                    OrderDirection::Descending => b_val.cmp(a_val),
                },
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, Some(_)) => std::cmp::Ordering::Less,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });
    }

    /// Update the primary key value in the tables referencing the updated table.
    ///
    /// # Arguments
    ///
    /// - `old_pk` - The old primary key value.
    /// - `new_pk` - The new primary key value.
    /// - `data_type` - The data type of the primary key.
    /// - `pk_name` - The name of the primary key column.
    fn update_pk_referencing_updated_table<T>(
        &self,
        old_pk: Value,
        new_pk: Value,
        data_type: DataTypeKind,
        pk_name: &'static str,
    ) -> IcDbmsResult<u64>
    where
        T: TableSchema,
    {
        let mut count = 0;
        // get referencing tables for this table
        // iterate over referencing tables and columns
        for (ref_table, ref_col) in self
            .schema
            .referenced_tables(T::table_name())
            .into_iter()
            .flat_map(|(ref_table, ref_cols)| {
                ref_cols
                    .into_iter()
                    .map(move |ref_col| (ref_table, ref_col))
            })
        {
            let ref_patch_value = (
                ColumnDef {
                    name: ref_col,
                    data_type,
                    nullable: false,
                    primary_key: false,
                    foreign_key: Some(ForeignKeyDef {
                        foreign_table: T::table_name(),
                        foreign_column: pk_name,
                        local_column: ref_col,
                    }),
                },
                new_pk.clone(),
            );
            // make an update patch
            let filter = Filter::eq(ref_col, old_pk.clone());

            count += self
                .schema
                .update(self, ref_table, &[ref_patch_value], Some(filter))?;
        }

        Ok(count)
    }

    /// Given a Vector of [`ColumnDef`] and [`Value`] pairs, sanitize the values using the
    /// sanitizers defined in the table schema.
    fn sanitize_values<T>(
        &self,
        values: Vec<(ColumnDef, Value)>,
    ) -> IcDbmsResult<Vec<(ColumnDef, Value)>>
    where
        T: TableSchema,
    {
        let mut sanitized_values = Vec::with_capacity(values.len());
        for (col_def, value) in values.into_iter() {
            let value = match T::sanitizer(col_def.name) {
                Some(sanitizer) => sanitizer.sanitize(value)?,
                None => value,
            };
            sanitized_values.push((col_def, value));
        }
        Ok(sanitized_values)
    }
}

impl Database for IcDbmsDatabase {
    /// Executes a SELECT query and returns the results.
    ///
    /// # Arguments
    ///
    /// - `query` - The SELECT [`Query`] to be executed.
    ///
    /// # Returns
    ///
    /// The returned results are a vector of [`table::TableRecord`] matching the query.
    fn select<T>(&self, query: Query<T>) -> IcDbmsResult<Vec<T::Record>>
    where
        T: TableSchema,
    {
        // load table registry
        let table_registry = self.load_table_registry::<T>()?;
        // read table
        let table_reader = table_registry.read::<T>();
        // get database overlay
        let mut table_overlay = if self.transaction.is_some() {
            self.overlay()?
        } else {
            DatabaseOverlay::default()
        };
        // overlay table reader
        let mut table_reader = table_overlay.reader(table_reader);

        // prepare results vector
        let mut results = Vec::with_capacity(query.limit.unwrap_or(DEFAULT_SELECT_CAPACITY));
        // iter and select
        let mut count = 0;

        while let Some(values) = table_reader.try_next()? {
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
            // push to results
            results.push(values);
            // check whether reached limit
            if query.limit.is_some_and(|limit| results.len() >= limit) {
                break;
            }
        }

        // Sort results if needed, applying in reverse order so the primary sort key
        // (first in the list) is applied last. Since `sort_by` is a stable sort,
        // this produces correct multi-column ordering.
        for (column, direction) in query.order_by.into_iter().rev() {
            self.sort_query_results(&mut results, &column, direction);
        }

        Ok(results.into_iter().map(T::Record::from_values).collect())
    }

    /// Executes an INSERT query.
    ///
    /// # Arguments
    ///
    /// - `record` - The INSERT record to be executed.
    fn insert<T>(&self, record: T::Insert) -> IcDbmsResult<()>
    where
        T: TableSchema,
        T::Insert: InsertRecord<Schema = T>,
    {
        // check whether the insert is valid
        let record_values = record.clone().into_values();
        let sanitized_values = self.sanitize_values::<T>(record_values)?;
        // validate insert
        self.schema
            .validate_insert(self, T::table_name(), &sanitized_values)?;
        if self.transaction.is_some() {
            // insert a new `insert` into the transaction
            self.with_transaction_mut(|tx| tx.insert::<T>(sanitized_values))?;
        } else {
            // insert directly into the database
            let mut table_registry = self.load_table_registry::<T>()?;
            // convert sanitized values back to record
            let record = T::Insert::from_values(&sanitized_values)?;
            table_registry.insert(record.into_record())?;
        }

        Ok(())
    }

    /// Executes an UPDATE query.
    ///
    /// # Arguments
    ///
    /// - `patch` - The UPDATE patch to be applied.
    /// - `filter` - An optional [`Filter`] to specify which records to update.
    ///
    /// # Returns
    ///
    /// The number of rows updated.
    fn update<T>(&self, patch: T::Update) -> IcDbmsResult<u64>
    where
        T: TableSchema,
        T::Update: UpdateRecord<Schema = T>,
    {
        let filter = patch.where_clause().clone();
        if self.transaction.is_some() {
            let pks = self.existing_primary_keys_for_filter::<T>(filter.clone())?;
            // insert a new `update` into the transaction
            self.with_transaction_mut(|tx| tx.update::<T>(patch, filter.clone(), pks))?;

            // TODO: correctly calculate count here
            // get all records matching the filter
            let query = Query::<T>::builder().all().filter(filter).build();
            let records = self.select::<T>(query)?;

            return Ok(records.len() as u64);
        }

        let patch = patch.update_values();

        // get whether PK is in the patch. If so, store the value to update referencing tables.
        let pk_in_patch = patch.iter().find_map(|(col_def, value)| {
            if col_def.primary_key {
                Some((col_def, value))
            } else {
                None
            }
        });

        let res = self.atomic(|db| {
            let mut count = 0;

            let mut table_registry = db.load_table_registry::<T>()?;
            // we must read directly from the database to get all records
            // this is because we need the page and offset to perform the update
            // this is safe because here we are not in a transaction; so the overlay doesn't matter
            let mut records = vec![];
            // iter all records
            // FIXME: this may be huge, we should do better
            {
                let mut table_reader = table_registry.read::<T>();
                while let Some(values) = table_reader.try_next()? {
                    let record_values = values.record.clone().to_values();
                    if let Some(filter) = &filter {
                        if !db.record_matches_filter(&record_values, filter)? {
                            continue;
                        }
                    }
                    records.push((values, record_values));
                }
            }

            // helper function which converts column-value pairs to a schema entity
            fn values_to_schema_entity<U>(values: Vec<(ColumnDef, Value)>) -> IcDbmsResult<U>
            where
                U: TableSchema,
            {
                let record = U::Insert::from_values(&values)?.into_record();
                Ok(record)
            }

            // update records
            for (record, record_values) in records {
                let current_pk_value = record_values
                    .iter()
                    .find(|(col_def, _)| col_def.primary_key)
                    .expect("primary key not found") // this can't fail.
                    .1
                    .clone();

                let previous_record = values_to_schema_entity::<T>(record_values.clone())?;
                let mut record_values = record_values;

                // apply patch to record values
                for (patch_col_def, patch_value) in &patch {
                    if let Some((_, record_value)) = record_values
                        .iter_mut()
                        .find(|(record_col_def, _)| record_col_def.name == patch_col_def.name)
                    {
                        *record_value = patch_value.clone();
                    }
                }
                // sanitize updated values
                let record_values = db.sanitize_values::<T>(record_values)?;
                // validate updated values
                db.schema.validate_update(
                    db,
                    T::table_name(),
                    &record_values,
                    current_pk_value.clone(),
                )?;
                // build T from values
                let updated_record = values_to_schema_entity::<T>(record_values)?;
                // perform the update in the table registry
                table_registry.update(
                    updated_record,
                    previous_record,
                    record.page,
                    record.offset,
                )?;
                count += 1;

                // update records in tables referencing this table if PK is updated
                if let Some((pk_column, new_pk_value)) = pk_in_patch {
                    count += db.update_pk_referencing_updated_table::<T>(
                        current_pk_value,
                        new_pk_value.clone(),
                        pk_column.data_type,
                        pk_column.name,
                    )?;
                }
            }

            Ok(count)
        });

        Ok(res)
    }

    /// Executes a DELETE query.
    ///
    /// # Arguments
    ///
    /// - `behaviour` - The [`DeleteBehavior`] to apply for foreign key constraints.
    /// - `filter` - An optional [`Filter`] to specify which records to delete.
    ///
    /// # Returns
    ///
    /// The number of rows deleted.
    fn delete<T>(&self, behaviour: DeleteBehavior, filter: Option<Filter>) -> IcDbmsResult<u64>
    where
        T: TableSchema,
    {
        if self.transaction.is_some() {
            let pks = self.existing_primary_keys_for_filter::<T>(filter.clone())?;
            let count = pks.len() as u64;

            self.with_transaction_mut(|tx| tx.delete::<T>(behaviour, filter, pks))?;

            return Ok(count);
        }

        // delete must be atomic
        let res = self.atomic(|db| {
            // delete directly from the database
            // select all records matching the filter
            // read table
            let mut table_registry = db.load_table_registry::<T>()?;
            let mut records = vec![];
            // iter all records
            // FIXME: this may be huge, we should do better
            {
                let mut table_reader = table_registry.read::<T>();
                while let Some(values) = table_reader.try_next()? {
                    let record_values = values.record.clone().to_values();
                    if let Some(filter) = &filter {
                        if !db.record_matches_filter(&record_values, filter)? {
                            continue;
                        }
                    }
                    records.push((values, record_values));
                }
            }
            // deleted records
            let mut count = records.len() as u64;
            for (record, record_values) in records {
                // match delete behaviour
                match behaviour {
                    DeleteBehavior::Cascade => {
                        // delete recursively foreign keys if cascade
                        count += self.delete_foreign_keys_cascade::<T>(&record_values)?;
                    }
                    DeleteBehavior::Restrict => {
                        if self.delete_foreign_keys_cascade::<T>(&record_values)? > 0 {
                            // it's okay; we panic here because we are in an atomic closure
                            return Err(IcDbmsError::Query(
                                QueryError::ForeignKeyConstraintViolation {
                                    referencing_table: T::table_name().to_string(),
                                    field: T::primary_key().to_string(),
                                },
                            ));
                        }
                    }
                }
                // eventually delete the record
                table_registry.delete(record.record, record.page, record.offset)?;
            }

            Ok(count)
        });

        Ok(res)
    }

    /// Commits the current transaction.
    ///
    /// The transaction is consumed.
    ///
    /// Any error during commit will trap the canister to ensure consistency.
    fn commit(&mut self) -> IcDbmsResult<()> {
        // take transaction out of self and get the transaction out of the storage
        // this also invalidates the overlay, so we won't have conflicts during validation
        let Some(txid) = self.transaction.take() else {
            return Err(IcDbmsError::Transaction(
                TransactionError::NoActiveTransaction,
            ));
        };
        let transaction = TRANSACTION_SESSION.with_borrow_mut(|ts| ts.take_transaction(&txid))?;

        // iterate over operations and apply them;
        // for each operation, first validate, then apply
        // using `self.atomic` when applying to ensure consistency
        for op in transaction.operations {
            match op {
                TransactionOp::Insert { table, values } => {
                    // validate
                    self.schema.validate_insert(self, table, &values)?;
                    // insert
                    self.atomic(|db| db.schema.insert(db, table, &values));
                }
                TransactionOp::Delete {
                    table,
                    behaviour,
                    filter,
                } => {
                    self.atomic(|db| db.schema.delete(db, table, behaviour, filter));
                }
                TransactionOp::Update {
                    table,
                    patch,
                    filter,
                } => {
                    self.atomic(|db| db.schema.update(db, table, &patch, filter));
                }
            }
        }

        Ok(())
    }

    /// Rolls back the current transaction.
    ///
    /// The transaction is consumed.
    fn rollback(&mut self) -> IcDbmsResult<()> {
        let Some(txid) = self.transaction.take() else {
            return Err(IcDbmsError::Transaction(
                TransactionError::NoActiveTransaction,
            ));
        };

        TRANSACTION_SESSION.with_borrow_mut(|ts| ts.close_transaction(&txid));
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use candid::{Nat, Principal};
    use ic_dbms_api::prelude::{Text, Uint32};

    use super::*;
    use crate::tests::{
        Message, POSTS_FIXTURES, Post, PostInsertRequest, TestDatabaseSchema, USERS_FIXTURES, User,
        UserInsertRequest, UserUpdateRequest, load_fixtures,
    };

    #[test]
    fn test_should_init_dbms() {
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        assert!(dbms.transaction.is_none());

        let tx_dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, Nat::from(1u64));
        assert!(tx_dbms.transaction.is_some());
    }

    #[test]
    fn test_should_select_all_users() {
        load_fixtures();
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
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
    fn test_should_select_user_in_overlay() {
        load_fixtures();
        // create a transaction
        let transaction_id =
            TRANSACTION_SESSION.with_borrow_mut(|ts| ts.begin_transaction(Principal::anonymous()));
        // insert
        TRANSACTION_SESSION.with_borrow_mut(|ts| {
            let tx = ts
                .get_transaction_mut(&transaction_id)
                .expect("should have tx");
            tx.overlay_mut()
                .insert::<User>(vec![
                    (
                        ColumnDef {
                            name: "id",
                            data_type: ic_dbms_api::prelude::DataTypeKind::Uint32,
                            nullable: false,
                            primary_key: true,
                            foreign_key: None,
                        },
                        Value::Uint32(999.into()),
                    ),
                    (
                        ColumnDef {
                            name: "name",
                            data_type: ic_dbms_api::prelude::DataTypeKind::Text,
                            nullable: false,
                            primary_key: false,
                            foreign_key: None,
                        },
                        Value::Text("OverlayUser".to_string().into()),
                    ),
                ])
                .expect("failed to insert");
        });

        // select by pk
        let dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, transaction_id);
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(999.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");

        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.id.expect("should have id").0, 999);
        assert_eq!(
            user.name.as_ref().expect("should have name").0,
            "OverlayUser"
        );
    }

    #[test]
    fn test_should_select_users_with_offset_and_limit() {
        load_fixtures();
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
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
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
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
    fn test_should_select_post_with_relation() {
        load_fixtures();
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<Post>::builder()
            .all()
            .with(User::table_name())
            .build();
        let posts = dbms.select(query).expect("failed to select posts");
        assert_eq!(posts.len(), POSTS_FIXTURES.len());

        for (id, post) in posts.into_iter().enumerate() {
            let (expected_title, expected_content, expected_user_id) = &POSTS_FIXTURES[id];
            assert_eq!(post.id.expect("should have id").0 as usize, id);
            assert_eq!(
                post.title.as_ref().expect("should have title").0,
                *expected_title
            );
            assert_eq!(
                post.content.as_ref().expect("should have content").0,
                *expected_content
            );
            let user_query = Query::<User>::builder()
                .and_where(Filter::eq("id", Value::Uint32((*expected_user_id).into())))
                .build();
            let author = dbms
                .select(user_query)
                .expect("failed to load user")
                .pop()
                .expect("should have user");
            assert_eq!(
                post.user.expect("should have loaded user"),
                Box::new(author)
            );
        }
    }

    #[test]
    fn test_should_fail_loading_unexisting_column_on_select() {
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder().field("unexisting_column").build();
        let result = dbms.select(query);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_select_queried_fields() {
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

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
        let user_fields = selected_fields
            .into_iter()
            .find(|(table_name, _)| *table_name == ValuesSource::This)
            .map(|(_, cols)| cols)
            .unwrap_or_default();

        assert_eq!(user_fields.len(), 1);
        assert_eq!(user_fields[0].0.name, "name");
        assert_eq!(user_fields[0].1, Value::Text("Alice".to_string().into()));
    }

    #[test]
    fn test_should_select_queried_fields_with_relations() {
        load_fixtures();
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

        let record_values = Post::columns()
            .iter()
            .cloned()
            .zip(vec![
                Value::Uint32(1.into()),
                Value::Text("Title".to_string().into()),
                Value::Text("Content".to_string().into()),
                Value::Uint32(2.into()), // author_id
            ])
            .collect::<Vec<(ColumnDef, Value)>>();

        let query: Query<Post> = Query::builder()
            .field("title")
            .with(User::table_name())
            .build();
        let selected_fields = dbms
            .select_queried_fields::<Post>(record_values, &query)
            .expect("failed to select queried fields");

        // check post fields
        let post_fields = selected_fields
            .iter()
            .find(|(table_name, _)| *table_name == ValuesSource::This)
            .map(|(_, cols)| cols)
            .cloned()
            .unwrap_or_default();
        assert_eq!(post_fields.len(), 1);
        assert_eq!(post_fields[0].0.name, "title");
        assert_eq!(post_fields[0].1, Value::Text("Title".to_string().into()));

        // check user fields
        let user_fields = selected_fields
            .iter()
            .find(|(table_name, _)| {
                *table_name
                    == ValuesSource::Foreign {
                        table: User::table_name().to_string(),
                        column: "user".to_string(),
                    }
            })
            .map(|(_, cols)| cols)
            .cloned()
            .unwrap_or_default();

        let expected_user = USERS_FIXTURES[2]; // author_id = 2

        assert_eq!(user_fields.len(), 4);
        assert_eq!(user_fields[0].0.name, "id");
        assert_eq!(user_fields[0].1, Value::Uint32(2.into()));
        assert_eq!(user_fields[1].0.name, "name");
        assert_eq!(
            user_fields[1].1,
            Value::Text(expected_user.to_string().into())
        );
        assert_eq!(user_fields[2].0.name, "email");
        assert_eq!(
            user_fields[2].1,
            Value::Text(format!("{}@example.com", expected_user.to_lowercase()).into())
        );
        assert_eq!(user_fields[3].0.name, "age");
        assert_eq!(user_fields[3].1, Value::Uint32(22u32.into()));
    }

    #[test]
    fn test_should_select_with_two_fk_on_the_same_table() {
        load_fixtures();

        let query: Query<Message> = Query::builder()
            .all()
            .and_where(Filter::Eq("id".to_string(), Value::Uint32(0.into())))
            .with("users")
            .build();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let messages = dbms.select(query).expect("failed to select messages");
        assert_eq!(messages.len(), 1);
        let message = &messages[0];
        assert_eq!(message.id.expect("should have id").0, 0);
        assert_eq!(
            message
                .sender
                .as_ref()
                .expect("should have sender")
                .name
                .as_ref()
                .unwrap()
                .0,
            "Alice"
        );
        assert_eq!(
            message
                .recipient
                .as_ref()
                .expect("should have recipient")
                .name
                .as_ref()
                .unwrap()
                .0,
            "Bob"
        );
    }

    #[test]
    fn test_should_select_users_sorted_by_name_descending() {
        load_fixtures();
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder().all().order_by_desc("name").build();
        let users = dbms.select(query).expect("failed to select users");

        let mut sorted_usernames = USERS_FIXTURES.to_vec();
        sorted_usernames.sort_by(|a, b| b.cmp(a)); // descending

        assert_eq!(users.len(), USERS_FIXTURES.len());
        // check if all users all loaded in sorted order
        for (i, user) in users.iter().enumerate() {
            assert_eq!(
                user.name.as_ref().expect("should have name").0,
                sorted_usernames[i]
            );
        }
    }

    #[test]
    fn test_should_select_users_sorted_by_multiple_columns() {
        load_fixtures();
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

        // Insert users with duplicate names but different ages to test multi-column sort.
        // The fixture users have unique names, so we add duplicates here.
        for (id, (name, age)) in [("Alice", 50u32), ("Alice", 30), ("Bob", 25), ("Bob", 40)]
            .iter()
            .enumerate()
        {
            let new_user = UserInsertRequest {
                id: Uint32(500 + id as u32),
                name: Text(name.to_string()),
                email: format!("dup_{}@example.com", id).into(),
                age: (*age).into(),
            };
            dbms.insert::<User>(new_user)
                .expect("failed to insert user");
        }

        // Sort by name ASC, age DESC — primary key is name, secondary is age descending.
        let query = Query::<User>::builder()
            .all()
            .and_where(Filter::ge("id", Value::Uint32(500.into())))
            .order_by_asc("name")
            .order_by_desc("age")
            .build();
        let users = dbms.select(query).expect("failed to select users");

        assert_eq!(users.len(), 4);

        // Expected order: Alice(50), Alice(30), Bob(40), Bob(25)
        let expected = [("Alice", 50u32), ("Alice", 30), ("Bob", 40), ("Bob", 25)];
        for (i, user) in users.iter().enumerate() {
            let (expected_name, expected_age) = expected[i];
            assert_eq!(
                user.name.as_ref().expect("should have name").0,
                expected_name,
                "name mismatch at index {i}"
            );
            assert_eq!(
                user.age.expect("should have age").0,
                expected_age,
                "age mismatch at index {i}"
            );
        }
    }

    #[test]
    fn test_should_select_many_entries() {
        const COUNT: u64 = 2_000;
        load_fixtures();

        for i in 1..=COUNT {
            let new_user = UserInsertRequest {
                id: Uint32(1000u32 + i as u32),
                name: Text(format!("User{}", i)),
                email: format!("user_{i}@example.com").into(),
                age: 20.into(),
            };
            assert!(
                IcDbmsDatabase::oneshot(TestDatabaseSchema)
                    .insert::<User>(new_user)
                    .is_ok()
            );
        }

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder()
            .all()
            .and_where(Filter::ge("id", Value::Uint32(1001.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), COUNT as usize);
    }

    #[test]
    fn test_should_fail_loading_unexisting_relation() {
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

        let record_values = Post::columns()
            .iter()
            .cloned()
            .zip(vec![
                Value::Uint32(1.into()),
                Value::Text("Title".to_string().into()),
                Value::Text("Content".to_string().into()),
                Value::Uint32(2.into()), // author_id
            ])
            .collect::<Vec<(ColumnDef, Value)>>();

        let query: Query<Post> = Query::builder()
            .field("title")
            .with("unexisting_relation")
            .build();
        let result = dbms.select_queried_fields::<Post>(record_values, &query);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_get_whether_record_matches_filter() {
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

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

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let table_registry = dbms.load_table_registry::<User>();
        assert!(table_registry.is_ok());
    }

    #[test]
    fn test_should_insert_record_without_transaction() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let new_user = UserInsertRequest {
            id: Uint32(100u32),
            name: Text("NewUser".to_string()),
            email: "new_user@example.com".into(),
            age: 25.into(),
        };

        let result = dbms.insert::<User>(new_user);
        assert!(result.is_ok());

        // find user
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(100u32.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.id.expect("should have id").0, 100);
        assert_eq!(
            user.name.as_ref().expect("should have name").0,
            "NewUser".to_string()
        );
    }

    #[test]
    fn test_should_validate_user_insert_conflict() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let new_user = UserInsertRequest {
            id: Uint32(1u32),
            name: Text("NewUser".to_string()),
            email: "new_user@example.com".into(),
            age: 25.into(),
        };

        let result = dbms.insert::<User>(new_user);
        assert!(result.is_err());
    }

    #[test]
    fn test_should_insert_within_a_transaction() {
        load_fixtures();

        // create a transaction
        let transaction_id =
            TRANSACTION_SESSION.with_borrow_mut(|ts| ts.begin_transaction(Principal::anonymous()));
        let mut dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, transaction_id.clone());

        let new_user = UserInsertRequest {
            id: Uint32(200u32),
            name: Text("TxUser".to_string()),
            email: "new_user@example.com".into(),
            age: 30.into(),
        };

        let result = dbms.insert::<User>(new_user);
        assert!(result.is_ok());

        // user should not be visible outside the transaction
        let oneshot_dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(200u32.into())))
            .build();
        let users = oneshot_dbms
            .select(query.clone())
            .expect("failed to select users");
        assert_eq!(users.len(), 0);

        // commit transaction
        let commit_result = dbms.commit();
        assert!(commit_result.is_ok());

        // now user should be visible
        let users_after_commit = oneshot_dbms.select(query).expect("failed to select users");
        assert_eq!(users_after_commit.len(), 1);

        let user = &users_after_commit[0];
        assert_eq!(user.id.expect("should have id").0, 200);
        assert_eq!(
            user.name.as_ref().expect("should have name").0,
            "TxUser".to_string()
        );

        // transaction should have been removed
        TRANSACTION_SESSION.with_borrow(|ts| {
            let tx_res = ts.get_transaction(&transaction_id);
            assert!(tx_res.is_err());
        });
    }

    #[test]
    fn test_should_rollback_transaction() {
        load_fixtures();

        // create a transaction
        let transaction_id =
            TRANSACTION_SESSION.with_borrow_mut(|ts| ts.begin_transaction(Principal::anonymous()));
        let mut dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, transaction_id.clone());
        let new_user = UserInsertRequest {
            id: Uint32(300u32),
            name: Text("RollbackUser".to_string()),
            email: "new_user@example.com".into(),
            age: 28.into(),
        };
        let result = dbms.insert::<User>(new_user);
        assert!(result.is_ok());

        // rollback transaction
        let rollback_result = dbms.rollback();
        assert!(rollback_result.is_ok());

        // user should not be visible
        let oneshot_dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(300u32.into())))
            .build();
        let users = oneshot_dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 0);

        // transaction should have been removed
        TRANSACTION_SESSION.with_borrow(|ts| {
            let tx_res = ts.get_transaction(&transaction_id);
            assert!(tx_res.is_err());
        });
    }

    #[test]
    fn test_should_sanitize_insert_data() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let new_user = UserInsertRequest {
            id: Uint32(100u32),
            name: Text("NewUser".to_string()),
            email: "new_user@example.com".into(),
            age: 150.into(),
        };

        let result = dbms.insert::<User>(new_user);
        assert!(result.is_ok());

        // find user
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(100u32.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.id.expect("should have id").0, 100);
        assert_eq!(user.age.expect("should have age").0, 120); // sanitized to max 120
    }

    #[test]
    fn test_should_delete_one_shot() {
        load_fixtures();

        // insert user with id 100
        let new_user = UserInsertRequest {
            id: Uint32(100u32),
            name: Text("DeleteUser".to_string()),
            email: "new_user@example.com".into(),
            age: 22.into(),
        };
        assert!(
            IcDbmsDatabase::oneshot(TestDatabaseSchema)
                .insert::<User>(new_user)
                .is_ok()
        );

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(100u32.into())))
            .build();
        let delete_count = dbms
            .delete::<User>(
                DeleteBehavior::Restrict,
                Some(Filter::eq("id", Value::Uint32(100u32.into()))),
            )
            .expect("failed to delete user");
        assert_eq!(delete_count, 1);

        // verify user is deleted
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 0);
    }

    #[test]
    fn test_should_delete_many_entries() {
        const COUNT: u64 = 2_000;
        load_fixtures();

        for i in 1..=COUNT {
            let new_user = UserInsertRequest {
                id: Uint32(1000u32 + i as u32),
                name: Text(format!("User{}", i)),
                email: format!("user_{i}@example.com").into(),
                age: 20.into(),
            };
            assert!(
                IcDbmsDatabase::oneshot(TestDatabaseSchema)
                    .insert::<User>(new_user)
                    .is_ok()
            );
        }

        let mut deleted_total = 0;
        for i in 1..=COUNT {
            let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
            let delete_count = dbms
                .delete::<User>(
                    DeleteBehavior::Restrict,
                    Some(Filter::eq("id", Value::Uint32((1000u32 + i as u32).into()))),
                )
                .expect("failed to delete user");
            assert_eq!(delete_count, 1, "failed to delete user {}", i);
            deleted_total += delete_count;
        }
        assert_eq!(deleted_total, COUNT);
    }

    #[test]
    fn test_should_drop_table() {
        const COUNT: u64 = 5_000;
        load_fixtures();

        for i in 1..=COUNT {
            let new_post = PostInsertRequest {
                id: Uint32(100u32 + i as u32),
                title: Text(format!("Post{}", i)),
                content: Text("Some content".to_string()),
                user: Uint32(1u32),
            };
            assert!(
                IcDbmsDatabase::oneshot(TestDatabaseSchema)
                    .insert::<Post>(new_post)
                    .is_ok()
            );
        }

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let delete_count = dbms
            .delete::<Post>(
                DeleteBehavior::Restrict,
                Some(Filter::ge("id", Value::Uint32(101.into()))),
            )
            .expect("failed to delete post");
        assert_eq!(
            delete_count, COUNT,
            "expected to delete all posts, but deleted {}",
            delete_count
        );
    }

    #[test]
    #[should_panic(expected = "Foreign key constraint violation")]
    fn test_should_not_delete_with_fk_restrict() {
        load_fixtures();

        // user 1 has post and messages for sure.
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

        // this delete will panic
        let _ = dbms.delete::<User>(
            DeleteBehavior::Restrict,
            Some(Filter::eq("id", Value::Uint32(1u32.into()))),
        );
    }

    #[test]
    fn test_should_delete_with_fk_cascade() {
        load_fixtures();

        // user 1 has posts and messages for sure.
        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let delete_count = dbms
            .delete::<User>(
                DeleteBehavior::Cascade,
                Some(Filter::eq("id", Value::Uint32(1u32.into()))),
            )
            .expect("failed to delete user");
        assert!(delete_count > 1); // at least user + posts + messages

        // verify user is deleted
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(1u32.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 0);

        // check posts are deleted (post ID 2)
        let post_query = Query::<Post>::builder()
            .and_where(Filter::eq("user_id", Value::Uint32(1u32.into())))
            .build();
        let posts = dbms.select(post_query).expect("failed to select posts");
        assert_eq!(posts.len(), 0);

        // check messages are deleted (message ID 1)
        let message_query = Query::<Message>::builder()
            .and_where(Filter::eq("sender_id", Value::Uint32(1u32.into())))
            .or_where(Filter::eq("recipient_id", Value::Uint32(1u32.into())))
            .build();
        let messages = dbms
            .select(message_query)
            .expect("failed to select messages");
        assert_eq!(messages.len(), 0);
    }

    #[test]
    fn test_should_delete_within_transaction() {
        load_fixtures();

        // create a transaction
        let transaction_id =
            TRANSACTION_SESSION.with_borrow_mut(|ts| ts.begin_transaction(Principal::anonymous()));
        let mut dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, transaction_id.clone());

        let delete_count = dbms
            .delete::<User>(
                DeleteBehavior::Cascade,
                Some(Filter::eq("id", Value::Uint32(2u32.into()))),
            )
            .expect("failed to delete user");
        assert!(delete_count > 0);

        // user should not be visible outside the transaction
        let oneshot_dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(2u32.into())))
            .build();
        let users = oneshot_dbms
            .select(query.clone())
            .expect("failed to select users");
        assert_eq!(users.len(), 1);

        // commit transaction
        let commit_result = dbms.commit();
        assert!(commit_result.is_ok());

        // now user should be deleted
        let users_after_commit = oneshot_dbms.select(query).expect("failed to select users");
        assert_eq!(users_after_commit.len(), 0);

        // check posts are deleted
        let post_query = Query::<Post>::builder()
            .and_where(Filter::eq("user_id", Value::Uint32(2u32.into())))
            .build();
        let posts = oneshot_dbms
            .select(post_query)
            .expect("failed to select posts");
        assert_eq!(posts.len(), 0);

        // check messages are deleted
        let message_query = Query::<Message>::builder()
            .and_where(Filter::eq("sender_id", Value::Uint32(2u32.into())))
            .or_where(Filter::eq("recipient_id", Value::Uint32(2u32.into())))
            .build();
        let messages = oneshot_dbms
            .select(message_query)
            .expect("failed to select messages");
        assert_eq!(messages.len(), 0);

        // transaction should have been removed
        TRANSACTION_SESSION.with_borrow(|ts| {
            let tx_res = ts.get_transaction(&transaction_id);
            assert!(tx_res.is_err());
        });
    }

    #[test]
    fn test_should_update_one_shot() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let filter = Filter::eq("id", Value::Uint32(3u32.into()));

        let patch = UserUpdateRequest {
            id: None,
            name: Some(Text("UpdatedName".to_string())),
            email: None,
            age: None,
            where_clause: Some(filter.clone()),
        };

        let update_count = dbms.update::<User>(patch).expect("failed to update user");
        assert_eq!(update_count, 1);

        // verify user is updated
        let query = Query::<User>::builder().and_where(filter).build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.id.expect("should have id").0, 3);
        assert_eq!(
            user.name.as_ref().expect("should have name").0,
            "UpdatedName".to_string()
        );
    }

    #[test]
    fn test_should_update_within_transaction() {
        load_fixtures();

        // create a transaction
        let transaction_id =
            TRANSACTION_SESSION.with_borrow_mut(|ts| ts.begin_transaction(Principal::anonymous()));
        let mut dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, transaction_id.clone());

        let filter = Filter::eq("id", Value::Uint32(4u32.into()));
        let patch = UserUpdateRequest {
            id: None,
            name: Some(Text("TxUpdatedName".to_string())),
            email: None,
            age: None,
            where_clause: Some(filter.clone()),
        };

        let update_count = dbms.update::<User>(patch).expect("failed to update user");
        assert_eq!(update_count, 1);

        // user should not be visible outside the transaction
        let oneshot_dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder().and_where(filter.clone()).build();
        let users = oneshot_dbms
            .select(query.clone())
            .expect("failed to select users");
        let user = &users[0];
        assert_eq!(
            user.name.as_ref().expect("should have name").0,
            USERS_FIXTURES[4]
        );

        // commit transaction
        let commit_result = dbms.commit();
        assert!(commit_result.is_ok());

        // now user should be updated
        let users_after_commit = oneshot_dbms.select(query).expect("failed to select users");
        assert_eq!(users_after_commit.len(), 1);
        let user = &users_after_commit[0];
        assert_eq!(
            user.name.as_ref().expect("should have name").0,
            "TxUpdatedName".to_string()
        );

        // transaction should have been removed
        TRANSACTION_SESSION.with_borrow(|ts| {
            let tx_res = ts.get_transaction(&transaction_id);
            assert!(tx_res.is_err());
        });
    }

    #[test]
    #[should_panic(
        expected = "Validation error: Value 'invalid_email' is not a valid email address"
    )]
    fn test_should_fail_to_update_with_invalid_data() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let filter = Filter::eq("id", Value::Uint32(3u32.into()));

        let patch = UserUpdateRequest {
            id: None,
            name: None,
            email: Some("invalid_email".into()), // invalid email format
            age: None,
            where_clause: Some(filter.clone()),
        };

        // this fails due to being inside atomic
        let _ = dbms.update::<User>(patch);
    }

    #[test]
    fn test_should_update_fk_in_table_referencing_another_oneshot() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);

        // update user with PK 0, check whether posts 0 and 1 has updated FK;
        // also check messages 0 and 1
        let filter = Filter::eq("id", Value::Uint32(0u32.into()));

        let patch = UserUpdateRequest {
            id: Some(Uint32(1_000u32)),
            name: None,
            email: None,
            age: None,
            where_clause: Some(filter.clone()),
        };

        let update_count = dbms.update::<User>(patch).expect("failed to update user");
        assert_eq!(update_count, 5); // 2 posts + 1 user + 2 messages

        // verify user is updated
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(1_000u32.into())))
            .build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.id.expect("should have id").0, 1_000);

        // get messages where sender_id or recipient_id is 1_000
        let message_query = Query::<Message>::builder()
            .with("users")
            .and_where(Filter::eq("sender", Value::Uint32(1_000u32.into())))
            .or_where(Filter::eq("recipient", Value::Uint32(1_000u32.into())))
            .build();
        let messages = dbms
            .select(message_query)
            .expect("failed to select messages");
        assert_eq!(messages.len(), 2);
        for message in messages {
            let sender_id = message
                .sender
                .as_ref()
                .expect("should have sender")
                .id
                .expect("should have sender id")
                .0;
            let recipient_id = message
                .recipient
                .as_ref()
                .expect("should have recipient")
                .id
                .expect("should have recipient id")
                .0;
            assert!(sender_id == 1_000 || recipient_id == 1_000);
        }

        // check posts where user_id is 1_000
        let post_query = Query::<Post>::builder()
            .with("users")
            .and_where(Filter::eq("user", Value::Uint32(1_000u32.into())))
            .build();
        let posts = dbms.select(post_query).expect("failed to select posts");
        assert_eq!(posts.len(), 2);
        for post in posts {
            let user_id = post
                .user
                .expect("should have user")
                .id
                .expect("should have user id")
                .0;
            assert_eq!(user_id, 1_000);
        }
    }

    #[test]
    fn test_should_sanitize_update() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let filter = Filter::eq("id", Value::Uint32(3u32.into()));

        let patch = UserUpdateRequest {
            id: None,
            name: None,
            email: None,
            age: Some(200.into()),
            where_clause: Some(filter.clone()),
        };

        let update_count = dbms.update::<User>(patch).expect("failed to update user");
        assert_eq!(update_count, 1);

        // verify user is updated
        let query = Query::<User>::builder().and_where(filter).build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 1);
        let user = &users[0];
        assert_eq!(user.id.expect("should have id").0, 3);
        assert_eq!(user.age.expect("should have age").0, 120); // sanitized to max 120
    }

    #[test]
    fn test_should_update_multiple_records_at_once() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        // update all users with id > 5 (users 6, 7, 8, 9)
        let filter = Filter::gt("id", Value::Uint32(5u32.into()));

        let patch = UserUpdateRequest {
            id: None,
            name: Some(Text("BulkUpdated".to_string())),
            email: None,
            age: None,
            where_clause: Some(filter.clone()),
        };

        let update_count = dbms.update::<User>(patch).expect("failed to update users");
        assert_eq!(update_count, 4); // users 6, 7, 8, 9

        // verify all matched users were updated
        let query = Query::<User>::builder().and_where(filter).build();
        let users = dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 4);
        for user in &users {
            assert_eq!(
                user.name.as_ref().expect("should have name").0,
                "BulkUpdated"
            );
        }

        // verify users with id <= 5 were NOT updated
        let unaffected_query = Query::<User>::builder()
            .and_where(Filter::le("id", Value::Uint32(5u32.into())))
            .build();
        let unaffected_users = dbms
            .select(unaffected_query)
            .expect("failed to select users");
        for user in &unaffected_users {
            assert_ne!(
                user.name.as_ref().expect("should have name").0,
                "BulkUpdated"
            );
        }
    }

    #[test]
    #[should_panic(expected = "Primary key conflict")]
    fn test_should_fail_update_with_pk_conflict_e2e() {
        load_fixtures();

        let dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        // try to update user 3's PK to 2 (which already exists)
        let filter = Filter::eq("id", Value::Uint32(3u32.into()));

        let patch = UserUpdateRequest {
            id: Some(Uint32(2u32)),
            name: None,
            email: None,
            age: None,
            where_clause: Some(filter),
        };

        // this should panic inside atomic because of PK conflict
        let _ = dbms.update::<User>(patch);
    }

    #[test]
    fn test_should_update_pk_with_fk_cascade_in_transaction() {
        load_fixtures();

        // create a transaction
        let transaction_id =
            TRANSACTION_SESSION.with_borrow_mut(|ts| ts.begin_transaction(Principal::anonymous()));
        let mut dbms = IcDbmsDatabase::from_transaction(TestDatabaseSchema, transaction_id.clone());

        // update user 0's PK to 5000 inside the transaction
        let filter = Filter::eq("id", Value::Uint32(0u32.into()));
        let patch = UserUpdateRequest {
            id: Some(Uint32(5000u32)),
            name: None,
            email: None,
            age: None,
            where_clause: Some(filter),
        };

        // NOTE: update_count in transaction path may not reflect cascaded FK changes
        // because the overlay transforms the record, making the original filter not match anymore.
        // The actual count is verified after commit.
        let _update_count = dbms.update::<User>(patch).expect("failed to update user");

        // outside the transaction, user 0 should still exist
        let oneshot_dbms = IcDbmsDatabase::oneshot(TestDatabaseSchema);
        let query = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(0u32.into())))
            .build();
        let users = oneshot_dbms.select(query).expect("failed to select users");
        assert_eq!(users.len(), 1);

        // commit transaction
        let commit_result = dbms.commit();
        assert!(commit_result.is_ok());

        // now user 0 should be gone, user 5000 should exist
        let query_old = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(0u32.into())))
            .build();
        let users_old = oneshot_dbms
            .select(query_old)
            .expect("failed to select users");
        assert_eq!(users_old.len(), 0);

        let query_new = Query::<User>::builder()
            .and_where(Filter::eq("id", Value::Uint32(5000u32.into())))
            .build();
        let users_new = oneshot_dbms
            .select(query_new)
            .expect("failed to select users");
        assert_eq!(users_new.len(), 1);

        // verify FK cascade: posts that referenced user 0 now reference user 5000
        let post_query = Query::<Post>::builder()
            .and_where(Filter::eq("user", Value::Uint32(5000u32.into())))
            .build();
        let posts = oneshot_dbms
            .select(post_query)
            .expect("failed to select posts");
        assert_eq!(posts.len(), 2); // user 0 had 2 posts

        // verify no posts reference user 0 anymore
        let old_post_query = Query::<Post>::builder()
            .and_where(Filter::eq("user", Value::Uint32(0u32.into())))
            .build();
        let old_posts = oneshot_dbms
            .select(old_post_query)
            .expect("failed to select posts");
        assert_eq!(old_posts.len(), 0);
    }

    fn init_user_table() {
        SCHEMA_REGISTRY
            .with_borrow_mut(|sr| sr.register_table::<User>())
            .expect("failed to register `User` table");
    }
}

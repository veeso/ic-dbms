mod ic;
mod types;

use candid::Principal;
use ic_dbms_api::prelude::{
    DeleteBehavior, Filter, InsertRecord, Query, TableSchema, TransactionId, UpdateRecord,
};

pub use self::ic::IcDbmsCanisterClient;
use crate::IcDbmsCanisterClientResult;

/// Trait for implementing a ic-dbms-client.
///
/// This is used so the library can expose also clients for pocket-ic.
///
/// If you're looking for the IC DBMS Canister client, see [`IcDbmsCanisterClient`].
pub trait Client {
    /// Returns the [`Principal`] of the IC DBMS Canister.
    fn principal(&self) -> Principal;

    /// Adds the given principal to the ACL of the canister.
    fn acl_add_principal(
        &self,
        principal: Principal,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<()>>;

    /// Removes the given principal from the ACL of the canister.
    fn acl_remove_principal(
        &self,
        principal: Principal,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<()>>;

    /// Lists all principals in the ACL of the canister.
    fn acl_allowed_principals(
        &self,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<Vec<Principal>>>;

    /// Begins a new transaction and returns its ID.
    fn begin_transaction(&self) -> impl Future<Output = IcDbmsCanisterClientResult<TransactionId>>;

    /// Commits the transaction with the given ID.
    fn commit(
        &self,
        transaction_id: TransactionId,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<()>>;

    /// Executes a `SELECT` query on the IC DBMS Canister.
    fn select<T>(
        &self,
        table: &str,
        query: Query<T>,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<Vec<T::Record>>>
    where
        T: TableSchema;

    /// Executes an `INSERT` query on the IC DBMS Canister.
    fn insert<T>(
        &self,
        table: &str,
        record: T::Insert,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<()>>
    where
        T: TableSchema,
        T::Insert: InsertRecord<Schema = T>;

    /// Executes an `UPDATE` query on the IC DBMS Canister.
    fn update<T>(
        &self,
        table: &str,
        patch: T::Update,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<u64>>
    where
        T: TableSchema,
        T::Update: UpdateRecord<Schema = T>;

    /// Executes a `DELETE` query on the IC DBMS Canister.
    fn delete<T>(
        &self,
        table: &str,
        behaviour: DeleteBehavior,
        filter: Option<Filter>,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<u64>>
    where
        T: TableSchema;
}

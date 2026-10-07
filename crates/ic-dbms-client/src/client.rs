#[cfg(feature = "ic-agent")]
#[cfg_attr(docsrs, doc(cfg(feature = "ic-agent")))]
mod agent;
mod ic;
#[cfg(feature = "pocket-ic")]
#[cfg_attr(docsrs, doc(cfg(feature = "pocket-ic")))]
mod pocket_ic;
mod types;

use candid::{CandidType, Principal};
use ic_dbms_api::prelude::{
    AclEntry, AggregateFunction, AggregatedRow, DeleteBehavior, Filter, IcDbmsResult, InsertRecord,
    JoinColumnDef, MigrationOp, MigrationPolicy, Permission, Query, TableSchema, TransactionId,
    UpdateRecord, Value,
};

#[cfg(feature = "ic-agent")]
#[cfg_attr(docsrs, doc(cfg(feature = "ic-agent")))]
pub use self::agent::IcDbmsAgentClient;
pub use self::ic::IcDbmsCanisterClient;
#[cfg(feature = "pocket-ic")]
#[cfg_attr(docsrs, doc(cfg(feature = "pocket-ic")))]
pub use self::pocket_ic::IcDbmsPocketIcClient;
use crate::prelude::IcDbmsCanisterClientResult;

type RawRecords = Vec<Vec<(JoinColumnDef, Value)>>;

/// Trait for implementing a ic-dbms-client.
///
/// This is used so the library can expose also clients for pocket-ic.
///
/// If you're looking for the IC DBMS Canister client, see [`IcDbmsCanisterClient`].
pub trait Client {
    /// Returns the [`Principal`] of the IC DBMS Canister.
    fn principal(&self) -> Principal;

    /// Grants `permission` to `principal`. The caller must hold
    /// [`Permission::Admin`].
    fn acl_grant(
        &self,
        principal: Principal,
        permission: Permission,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<()>>>;

    /// Revokes `permission` from `principal`. The caller must hold
    /// [`Permission::Admin`]; the last admin cannot be revoked.
    fn acl_revoke(
        &self,
        principal: Principal,
        permission: Permission,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<()>>>;

    /// Lists every principal together with its permissions. The caller must
    /// hold [`Permission::Admin`].
    fn acl_list(
        &self,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<Vec<AclEntry>>>>;

    /// Returns the caller's own permissions.
    fn my_permissions(
        &self,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<Vec<Permission>>>>;

    /// Begins a new transaction and returns its ID. The caller must hold
    /// [`Permission::Admin`].
    fn begin_transaction(
        &self,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<TransactionId>>>;

    /// Commits the transaction with the given ID.
    fn commit(
        &self,
        transaction_id: TransactionId,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<()>>>;

    /// Rolls back the transaction with the given ID.
    fn rollback(
        &self,
        transaction_id: TransactionId,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<()>>>;

    /// Executes a `SELECT` query on the IC DBMS Canister.
    fn select<T>(
        &self,
        table: &str,
        query: Query,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<Vec<T::Record>>>>
    where
        T: TableSchema,
        T::Record: CandidType + for<'de> candid::Deserialize<'de>;

    /// Executes an aggregate query on the IC DBMS Canister.
    ///
    /// The `query` carries `WHERE`, `DISTINCT`, `GROUP BY`, `HAVING`,
    /// `ORDER BY`, `OFFSET`, and `LIMIT` clauses; `aggregates` lists the
    /// [`AggregateFunction`]s to compute per group.
    fn aggregate<T>(
        &self,
        table: &str,
        query: Query,
        aggregates: Vec<AggregateFunction>,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<Vec<AggregatedRow>>>>
    where
        T: TableSchema;

    /// Executes a `SELECT` query on the IC DBMS Canister and returns raw records (without deserialization).
    fn select_raw(
        &self,
        table: &str,
        query: Query,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<RawRecords>>>;

    /// Executes an `INSERT` query on the IC DBMS Canister.
    fn insert<T>(
        &self,
        table: &str,
        record: T::Insert,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<()>>>
    where
        T: TableSchema,
        T::Insert: InsertRecord<Schema = T> + CandidType;

    /// Executes an `UPDATE` query on the IC DBMS Canister.
    fn update<T>(
        &self,
        table: &str,
        patch: T::Update,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<u64>>>
    where
        T: TableSchema,
        T::Update: UpdateRecord<Schema = T> + CandidType;

    /// Executes a `DELETE` query on the IC DBMS Canister.
    fn delete<T>(
        &self,
        table: &str,
        behaviour: DeleteBehavior,
        filter: Option<Filter>,
        transaction_id: Option<TransactionId>,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<u64>>>
    where
        T: TableSchema;

    /// Returns `true` when the canister's persisted schema differs from the
    /// schema compiled into its binary.
    fn has_drift(&self) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<bool>>>;

    /// Returns the migration ops needed to bring the persisted schema in line
    /// with the compiled one, without applying anything.
    fn pending_migrations(
        &self,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<Vec<MigrationOp>>>>;

    /// Applies a planned migration under `policy`.
    fn migrate(
        &self,
        policy: MigrationPolicy,
    ) -> impl Future<Output = IcDbmsCanisterClientResult<IcDbmsResult<()>>>;
}

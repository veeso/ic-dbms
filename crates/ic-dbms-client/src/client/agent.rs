//! This module exposes the ic-dbms canister client which uses the ic-agent [`Agent`] to communicate with the IC.
//!
//! This client is intended to be used from systems that are external to the IC, such as backend services or command-line tools, which
//! need to interact with the IC DBMS canister.

use candid::utils::ArgumentEncoder;
use candid::{CandidType, Decode, Principal};
use ic_agent::Agent;
use ic_dbms_api::prelude::{
    AclGrant, AggregateFunction, AggregatedRow, DeleteBehavior, Filter, IcDbmsResult, InsertRecord,
    MigrationOp, MigrationPolicy, Query, TableSchema, TransactionId, UpdateRecord, Value,
};

use crate::client::{Client, RawRecords};
use crate::errors::{IcAgentError, IcDbmCanisterClientError, IcDbmsCanisterClientResult};

/// Client to interact with an IC DBMS Canister using ic-agent.
#[derive(Clone, Debug)]
pub struct IcDbmsAgentClient<'a> {
    agent: &'a Agent,
    canister_id: Principal,
}

impl<'a> IcDbmsAgentClient<'a> {
    /// Initialize a new [`IcDbmsAgentClient`] with the given reference to an [`Agent`], and the canister ID.
    pub fn new(agent: &'a Agent, canister_id: Principal) -> Self {
        Self { agent, canister_id }
    }
}

impl IcDbmsAgentClient<'_> {
    /// Calls a query method on the IC DBMS Canister with the provided arguments and returns the result.
    async fn query<E, R>(&self, method_name: &str, args: E) -> IcDbmsCanisterClientResult<R>
    where
        E: ArgumentEncoder,
        R: CandidType + for<'de> candid::Deserialize<'de>,
    {
        let args = candid::encode_args(args).map_err(IcAgentError::from)?;
        let result = self
            .agent
            .query(&self.canister_id, method_name)
            .with_arg(args)
            .call()
            .await
            .map_err(IcAgentError::from)?;

        self.decode_result(result)
    }

    /// Calls an update method on the IC DBMS Canister with the provided arguments and returns the result.
    async fn update<E, R>(&self, method_name: &str, args: E) -> IcDbmsCanisterClientResult<R>
    where
        E: ArgumentEncoder,
        R: CandidType + for<'de> candid::Deserialize<'de>,
    {
        let args = candid::encode_args(args).map_err(IcAgentError::from)?;
        let result = self
            .agent
            .update(&self.canister_id, method_name)
            .with_arg(args)
            .call_and_wait()
            .await
            .map_err(IcAgentError::from)?;

        self.decode_result(result)
    }

    /// Helper function to decode the result from a canister call.
    fn decode_result<R>(&self, data: Vec<u8>) -> IcDbmsCanisterClientResult<R>
    where
        R: CandidType + for<'de> candid::Deserialize<'de>,
    {
        Decode!(data.as_slice(), R)
            .map_err(IcAgentError::from)
            .map_err(IcDbmCanisterClientError::from)
    }
}

impl Client for IcDbmsAgentClient<'_> {
    fn principal(&self) -> Principal {
        self.canister_id
    }

    async fn acl_grant(&self, grant: AclGrant) -> IcDbmsCanisterClientResult<IcDbmsResult<()>> {
        self.update("acl_grant", (grant,)).await
    }

    async fn acl_revoke(&self, grant: AclGrant) -> IcDbmsCanisterClientResult<IcDbmsResult<()>> {
        self.update("acl_revoke", (grant,)).await
    }

    async fn acl_list(&self) -> IcDbmsCanisterClientResult<IcDbmsResult<Vec<AclGrant>>> {
        self.query("acl_list", ()).await
    }

    async fn my_permissions(&self) -> IcDbmsCanisterClientResult<IcDbmsResult<Vec<AclGrant>>> {
        self.query("my_permissions", ()).await
    }

    async fn begin_transaction(&self) -> IcDbmsCanisterClientResult<IcDbmsResult<TransactionId>> {
        self.update("begin_transaction", ()).await
    }

    async fn commit(
        &self,
        transaction_id: TransactionId,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<()>> {
        self.update("commit", (transaction_id,)).await
    }

    async fn rollback(
        &self,
        transaction_id: TransactionId,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<()>> {
        self.update("rollback", (transaction_id,)).await
    }

    async fn select<T>(
        &self,
        table: &str,
        query: Query,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<Vec<T::Record>>>
    where
        T: TableSchema,
        T::Record: CandidType + for<'de> candid::Deserialize<'de>,
    {
        self.query(
            &crate::utils::table_method(table, "select"),
            (query, transaction_id),
        )
        .await
    }

    async fn select_raw(
        &self,
        table: &str,
        query: Query,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<RawRecords>> {
        self.query("select", (table, query, transaction_id)).await
    }

    async fn aggregate<T>(
        &self,
        table: &str,
        query: Query,
        aggregates: Vec<AggregateFunction>,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<Vec<AggregatedRow>>>
    where
        T: TableSchema,
    {
        self.query(
            &crate::utils::table_method(table, "aggregate"),
            (query, aggregates, transaction_id),
        )
        .await
    }

    async fn insert<T>(
        &self,
        table: &str,
        record: T::Insert,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<()>>
    where
        T: TableSchema,
        T::Insert: InsertRecord<Schema = T> + CandidType,
    {
        self.update(
            &crate::utils::table_method(table, "insert"),
            (record, transaction_id),
        )
        .await
    }

    async fn update<T>(
        &self,
        table: &str,
        patch: T::Update,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<u64>>
    where
        T: TableSchema,
        T::Update: UpdateRecord<Schema = T> + CandidType,
    {
        self.update(
            &crate::utils::table_method(table, "update"),
            (patch, transaction_id),
        )
        .await
    }

    async fn delete<T>(
        &self,
        table: &str,
        behaviour: DeleteBehavior,
        filter: Option<Filter>,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<u64>>
    where
        T: TableSchema,
    {
        self.update(
            &crate::utils::table_method(table, "delete"),
            (behaviour, filter, transaction_id),
        )
        .await
    }

    async fn has_drift(&self) -> IcDbmsCanisterClientResult<IcDbmsResult<bool>> {
        self.query("has_drift", ()).await
    }

    async fn pending_migrations(
        &self,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<Vec<MigrationOp>>> {
        self.query("pending_migrations", ()).await
    }

    async fn migrate(
        &self,
        policy: MigrationPolicy,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<()>> {
        self.update("migrate", (policy,)).await
    }

    #[cfg(feature = "sql")]
    async fn sql(
        &self,
        query: &str,
        params: Vec<Value>,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<ic_dbms_api::prelude::SqlResult>> {
        self.update("sql", (query, params, transaction_id)).await
    }

    #[cfg(feature = "sql")]
    async fn sql_query(
        &self,
        query: &str,
        params: Vec<Value>,
        transaction_id: Option<TransactionId>,
    ) -> IcDbmsCanisterClientResult<IcDbmsResult<ic_dbms_api::prelude::SqlResult>> {
        self.query("sql_query", (query, params, transaction_id))
            .await
    }
}

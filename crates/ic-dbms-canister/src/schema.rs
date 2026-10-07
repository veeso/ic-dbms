//! Composition of the user schema with the reserved tables.

use ic_dbms_api::prelude::{
    AggregateFunction, AggregatedRow, ColumnDef, DbmsResult, DeleteBehavior, Filter, Query,
    TableSchema as _, TableSchemaSnapshot, Value,
};
use wasm_dbms::prelude::{DatabaseSchema, WasmDbmsDatabase};
use wasm_dbms_memory::prelude::MemoryProvider;

use crate::acl::{AclPrincipal, AclSchema};

/// The schema the engine sees: the user's `#[derive(DatabaseSchema)]`
/// schema plus the reserved tables owned by ic-dbms.
///
/// The engine hashes every persisted table into its drift check, so a
/// reserved table must be part of the compiled schema or every call would
/// report `SchemaDrift` and `migrate` would drop it. Operations on the
/// reserved table name are routed to [`AclSchema`]; everything else goes to
/// the inner schema.
pub struct CanisterSchema<S> {
    inner: S,
}

impl<S> CanisterSchema<S> {
    /// Wraps the user schema `inner`.
    pub fn new(inner: S) -> Self {
        Self { inner }
    }
}

fn is_acl(table: &str) -> bool {
    table == AclPrincipal::table_name()
}

impl<S, M> DatabaseSchema<M> for CanisterSchema<S>
where
    S: DatabaseSchema<M>,
    M: MemoryProvider,
{
    fn select(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &str,
        query: Query,
    ) -> DbmsResult<Vec<Vec<(ColumnDef, Value)>>> {
        if is_acl(table_name) {
            AclSchema.select(dbms, table_name, query)
        } else {
            self.inner.select(dbms, table_name, query)
        }
    }

    fn table_columns(&self, table_name: &str) -> DbmsResult<&'static [ColumnDef]> {
        if is_acl(table_name) {
            <AclSchema as DatabaseSchema<M>>::table_columns(&AclSchema, table_name)
        } else {
            self.inner.table_columns(table_name)
        }
    }

    fn static_table_name(&self, table_name: &str) -> DbmsResult<&'static str> {
        if is_acl(table_name) {
            <AclSchema as DatabaseSchema<M>>::static_table_name(&AclSchema, table_name)
        } else {
            self.inner.static_table_name(table_name)
        }
    }

    fn aggregate(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &str,
        query: Query,
        aggregates: &[AggregateFunction],
    ) -> DbmsResult<Vec<AggregatedRow>> {
        if is_acl(table_name) {
            AclSchema.aggregate(dbms, table_name, query, aggregates)
        } else {
            self.inner.aggregate(dbms, table_name, query, aggregates)
        }
    }

    fn referenced_tables(&self, table: &'static str) -> Vec<(&'static str, Vec<&'static str>)> {
        let mut refs = self.inner.referenced_tables(table);
        refs.extend(<AclSchema as DatabaseSchema<M>>::referenced_tables(
            &AclSchema, table,
        ));
        refs
    }

    fn insert(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
    ) -> DbmsResult<()> {
        if is_acl(table_name) {
            AclSchema.insert(dbms, table_name, record_values)
        } else {
            self.inner.insert(dbms, table_name, record_values)
        }
    }

    fn delete(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        delete_behavior: DeleteBehavior,
        filter: Option<Filter>,
    ) -> DbmsResult<u64> {
        if is_acl(table_name) {
            AclSchema.delete(dbms, table_name, delete_behavior, filter)
        } else {
            self.inner.delete(dbms, table_name, delete_behavior, filter)
        }
    }

    fn update(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        patch_values: &[(ColumnDef, Value)],
        filter: Option<Filter>,
    ) -> DbmsResult<u64> {
        if is_acl(table_name) {
            AclSchema.update(dbms, table_name, patch_values, filter)
        } else {
            self.inner.update(dbms, table_name, patch_values, filter)
        }
    }

    fn validate_insert(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
    ) -> DbmsResult<()> {
        if is_acl(table_name) {
            AclSchema.validate_insert(dbms, table_name, record_values)
        } else {
            self.inner.validate_insert(dbms, table_name, record_values)
        }
    }

    fn validate_update(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
        old_pk: Value,
    ) -> DbmsResult<()> {
        if is_acl(table_name) {
            AclSchema.validate_update(dbms, table_name, record_values, old_pk)
        } else {
            self.inner
                .validate_update(dbms, table_name, record_values, old_pk)
        }
    }

    fn migrate_default(table: &str, column: &str) -> Option<Value>
    where
        Self: Sized,
    {
        if is_acl(table) {
            <AclSchema as DatabaseSchema<M>>::migrate_default(table, column)
        } else {
            S::migrate_default(table, column)
        }
    }

    fn migrate_default_dyn(&self, table: &str, column: &str) -> Option<Value> {
        if is_acl(table) {
            <AclSchema as DatabaseSchema<M>>::migrate_default_dyn(&AclSchema, table, column)
        } else {
            self.inner.migrate_default_dyn(table, column)
        }
    }

    fn migrate_transform(table: &str, column: &str, old: Value) -> DbmsResult<Option<Value>>
    where
        Self: Sized,
    {
        if is_acl(table) {
            <AclSchema as DatabaseSchema<M>>::migrate_transform(table, column, old)
        } else {
            S::migrate_transform(table, column, old)
        }
    }

    fn migrate_transform_dyn(
        &self,
        table: &str,
        column: &str,
        old: Value,
    ) -> DbmsResult<Option<Value>> {
        if is_acl(table) {
            <AclSchema as DatabaseSchema<M>>::migrate_transform_dyn(&AclSchema, table, column, old)
        } else {
            self.inner.migrate_transform_dyn(table, column, old)
        }
    }

    fn compiled_snapshots() -> Vec<TableSchemaSnapshot>
    where
        Self: Sized,
    {
        let mut snapshots = S::compiled_snapshots();
        snapshots.extend(<AclSchema as DatabaseSchema<M>>::compiled_snapshots());
        snapshots
    }

    fn compiled_snapshots_dyn(&self) -> Vec<TableSchemaSnapshot> {
        let mut snapshots = self.inner.compiled_snapshots_dyn();
        snapshots.extend(<AclSchema as DatabaseSchema<M>>::compiled_snapshots_dyn(
            &AclSchema,
        ));
        snapshots
    }

    fn renamed_from_dyn(&self, table: &str, column: &str) -> Vec<&'static str> {
        if is_acl(table) {
            <AclSchema as DatabaseSchema<M>>::renamed_from_dyn(&AclSchema, table, column)
        } else {
            self.inner.renamed_from_dyn(table, column)
        }
    }
}

#[cfg(test)]
mod tests {

    use ic_dbms_api::prelude::{Database as _, MigrationOp};

    use super::*;
    use crate::memory::DBMS_CONTEXT;
    use crate::tests::TestDatabaseSchema;

    fn register_all() {
        DBMS_CONTEXT.with(|ctx| {
            TestDatabaseSchema::register_tables(ctx).expect("user tables");
            crate::acl::register(ctx).expect("acl table");
        });
    }

    #[test]
    fn test_composed_schema_has_no_drift() {
        register_all();
        DBMS_CONTEXT.with(|ctx| {
            let db = WasmDbmsDatabase::oneshot(ctx, CanisterSchema::new(TestDatabaseSchema));
            assert!(!db.has_drift().expect("drift check"));
            assert!(db.pending_migrations().expect("plan").is_empty());
        });
    }

    #[test]
    fn test_bare_user_schema_would_drop_acl_table() {
        register_all();
        DBMS_CONTEXT.with(|ctx| {
            let db = WasmDbmsDatabase::oneshot(ctx, TestDatabaseSchema);
            assert!(db.has_drift().expect("drift check"));
            let ops = db.pending_migrations().expect("plan");
            assert!(ops.iter().any(|op| matches!(
                op,
                MigrationOp::DropTable { name } if name == "ic_dbms_acl"
            )));
        });
    }

    #[test]
    fn test_composed_schema_routes_raw_select_to_both_sides() {
        crate::tests::load_fixtures();
        DBMS_CONTEXT.with(|ctx| {
            crate::acl::register(ctx).expect("acl table");
            let db = WasmDbmsDatabase::oneshot(ctx, CanisterSchema::new(TestDatabaseSchema));
            crate::acl::grant(
                &db,
                crate::utils::caller(),
                ic_dbms_api::prelude::Permission::Admin,
            )
            .expect("grant");
            let users = db
                .select_raw("users", Query::builder().all().build())
                .expect("users");
            assert!(!users.is_empty());
            let acl = db
                .select_raw("ic_dbms_acl", Query::builder().all().build())
                .expect("acl");
            assert_eq!(acl.len(), 1);
            let schema = CanisterSchema::new(TestDatabaseSchema);
            let columns = <CanisterSchema<TestDatabaseSchema> as DatabaseSchema<
                crate::memory::IcMemoryProvider,
            >>::table_columns(&schema, "ic_dbms_acl")
            .expect("columns");
            assert_eq!(columns.len(), 1);
            assert_eq!(columns[0].name, "principal");
        });
    }

    #[test]
    fn test_composed_schema_compiled_snapshots_include_acl() {
        let snapshots = <CanisterSchema<TestDatabaseSchema> as DatabaseSchema<
            crate::memory::IcMemoryProvider,
        >>::compiled_snapshots();
        assert_eq!(snapshots.len(), 4);
        assert!(snapshots.iter().any(|s| s.name == "ic_dbms_acl"));
        let dynamic = CanisterSchema::new(TestDatabaseSchema);
        assert_eq!(
            <CanisterSchema<TestDatabaseSchema> as DatabaseSchema<
                crate::memory::IcMemoryProvider,
            >>::compiled_snapshots_dyn(&dynamic)
            .len(),
            4
        );
    }
}

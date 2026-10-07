# Changelog

All notable changes to this project are documented in this file.

## 0.10.0

Released on 2026-10-07

### Breaking changes

- **canister:** grant per-table permissions in the access control list (#7)

> Permission and AclEntry are removed. acl_grant and acl_revoke take an AclGrant, acl_list and my_permissions return Vec<AclGrant>, and AclError::AccessDenied now carries the required AclPermission and the table.

### Added

- **canister:** track transaction ownership in the canister (#6)

> - feat!: migrate to wasm-dbms 0.10
>
> wasm-dbms 0.10 removes the engine access control list and stops recording who opened a transaction. Adapt the canister runtime, the DbmsCanister macro, the clients, the wrapper canister and the PocketIC tests to the new signatures: DbmsContext, WasmDbmsDatabase and DatabaseSchema lose their access-control type parameter, begin_transaction takes no owner and has_transaction takes no caller. Access control and transaction ownership are re-added in the canister by the next two commits of this branch.

- Breaking: **canister:** grant per-table permissions in the access control list (#7)

> The reserved ic_dbms_acl table now stores one row per grant: a principal, an AclPermission (Admin, Read, Insert, Update, Delete) and an optional table, where no table means every table. Every generated endpoint checks the matching permission: select and aggregate need Read, insert, update and delete need the matching write permission, the untyped select needs Read on every joined table and eager relation, cascade deletes need Delete on every referencing table, and opening a transaction needs at least one grant. ACL management still needs Admin; migrations need Admin or a controller. The clients and the wrapper canister use the new AclGrant type.

- **canister:** expose sql through canister endpoints (#8)

> Add an opt-in sql feature to ic-dbms-canister. With it, the DbmsCanister derive generates an sql update endpoint and an sql_query query endpoint that run statements through wasm-dbms-sql. Both take an optional transaction id. Statements are checked against the access control list with the same rules as the typed endpoints, tables reserved for ic-dbms are hidden from SQL, and BEGIN, COMMIT and ROLLBACK go through the transaction ownership ledger. sql_query refuses every statement that is not a SELECT. ic-dbms-api gains the IcDbmsError::Sql variant and ic-dbms-client gains Client::sql and Client::sql_query behind sql features. A new guide documents the feature.

### Build

- bump pocket-ic-harness to 16.0.0 (#4)

## 0.9.0

Released on 2026-04-28

### Breaking changes

- **acl:** granular per-identity permissions (closes #87)

> granular per-identity permissions (closes #87)

### Added

- **query:** add aggregates, GROUP BY, and HAVING (#86)

> Introduce `Database::aggregate` with COUNT/SUM/AVG/MIN/MAX, GROUP BY,
> HAVING, and aggregate-aware ORDER BY/LIMIT/OFFSET. Expose the new flow
> through the IC canister macro (`aggregate_<table>`) and the
> `ic-dbms-client` Client trait + all three client impls.
>
> - AggregateFunction / AggregatedRow / AggregatedValue types in
>   wasm-dbms-api; HAVING / ORDER BY reference aggregate outputs by
>   synthetic agg{N} names.
> - Plan-time validation: SUM/AVG numeric, unknown col/agg, LIKE/JSON
>   rejected in HAVING, joins/eager relations rejected in aggregate.
> - Reject group_by/having on non-aggregate select paths via new
>   QueryError::AggregateClauseInSelect (mirrors JoinInsideTypedSelect).
> - Fix Query candid serialization order after group_by/having insertion
>   (idl_hash-sorted: eager_relations, distinct_by, joins, offset, limit,
>   filter, group_by, having, order_by, columns).
> - Tests: 29 unit tests, 3 select-guard tests, 6 pocket-ic integration
>   tests including round-trip via wrapper canister.
> - Docs: guides/querying.md aggregations section, reference/query.md
>   Aggregate Types + Errors, ic/reference/schema.md endpoint listing,
>   ic/guides/client-api.md client.aggregate example, errors.md updates.
> - Database trait method docs rewritten with proper Errors sections.

- **migrations:** add schema snapshot, Migrate trait, and macro support (#34)

> Lay the groundwork for schema migrations:
>
> - Snapshot types (`TableSchemaSnapshot`, `ColumnSnapshot`, `IndexSnapshot`,
>   `ForeignKeySnapshot`, `OnDeleteSnapshot`, `DataTypeSnapshot`) with
>   versioned binary `Encode`/`Decode`, `Serialize`/`Deserialize`, and
>   feature-gated `CandidType` derives.
> - `TableSchema::schema_snapshot()` with default impl assembling from
>   `table_name` / `primary_key` / `columns` / `indexes` and
>   `Encode::ALIGNMENT`.
> - `dbms::migration` module: `Migrate` trait, `MigrationOp`, `ColumnChanges`,
>   `MigrationPolicy`, `MigrationError`, plus `DbmsError::Migration` and
>   prelude re-exports.
> - `ColumnDef` gains `default: Option<fn() -> Value>` (kept fn-pointer to
>   preserve `Copy`) and `renamed_from: &'static [&'static str]`.
> - `#[derive(Table)]` parses `#[default = ...]`, `#[renamed_from(...)]`, and
>   `#[migrate]`; emits `impl Migrate for T {}` unless `#[migrate]` is set.
> - `DatabaseSchema` trait and `#[derive(DatabaseSchema)]` gain
>   `migrate_default`, `migrate_transform`, `compiled_snapshots` dispatch
>   methods (object-safe via `Self: Sized`).
> - WIT: new `migration-error(string)` variant; guest maps
>   `DbmsError::Migration`.
> - Docs: schema reference covers the three new attributes; new
>   `docs/reference/migrations.md` and `docs/guides/migrations.md`; errors
>   reference lists every `MigrationError` variant.
> - Tests: 13 new macro tests cover `#[default]`, `#[renamed_from]`,
>   `#[migrate]`, dispatch fall-through, unknown-table behaviour, and
>   multi-table snapshot ordering.
>
> Memory layer integration, the migration engine, `Database` wiring, IC
> endpoints, client surface, integration tests, and full WIT migrate APIs
> are tracked in the issue checklist.

- **migrations:** engine + Database wiring for schema migrations

> Wire the migration engine into the generic `wasm-dbms` crate so callers
> can detect drift, plan migrations, and apply structural ops through the
> existing `Database` trait surface.
>
> Engine layer (`crates/wasm-dbms/wasm-dbms/src/database/migration/`):
>
> - `snapshots`: drift hash via `xxh3` (`xxhash-rust`), seeded with
>   `TableSchemaSnapshot::latest_version()`. `compute_drift` compares
>   persisted snapshots (loaded through `SchemaRegistry::stored_snapshots`)
>   with `S::compiled_snapshots()` reachable through the boxed schema.
> - `diff`: pure stored-vs-compiled diff producing `Vec<MigrationOp>`.
>   Renames resolved via the new `DatabaseSchema::renamed_from_dyn`
>   dispatch, type changes routed through the widening whitelist or
>   `MigrationOp::TransformColumn`.
> - `plan`: deterministic op ordering plus policy-driven validation
>   (destructive-op gate, defense-in-depth missing-default check).
> - `apply`: journaled execution of structural ops (`CreateTable`,
>   `DropTable`, `AlterColumn`, `AddIndex`, `DropIndex`); tightening
>   validation (`nullable: false`, `unique: true`) scans existing rows
>   through schema dispatch. Column-mutating ops (`AddColumn`,
>   `DropColumn`, `RenameColumn`, `WidenColumn`, `TransformColumn`) return
>   the new `MigrationError::DataRewriteUnsupported` until the
>   snapshot-driven (de)serializer (issue #91) lands. `DropTable` leaks
>   pages (issue #90).
>
> Database wiring:
>
> - `DbmsContext` gains `Cell<Option<bool>> drift` (lazy cache) and
>   `Cell<bool> migrating` (apply-in-progress guard so internal reads
>   bypass the gate).
> - `WasmDbmsDatabase::ensure_no_drift` is called as the first line of
>   every `Database` method except `rollback`. ACL methods on
>   `DbmsContext` continue to bypass.
> - `Database` trait gains `has_drift`, `pending_migrations` (renamed
>   from the previous `plan_migration`), and `migrate(policy)`. The IC
>   adapter and any other `Database` implementor must now implement them;
>   the test mock in `wasm-dbms-api` is updated.
>
> API additions (`wasm-dbms-api`):
>
> - `fingerprint_for_name(&str) -> u64` (`xxh3_64`), so the engine can
>   derive a registry key for tables it knows only by name.
> - `MigrationError::DataRewriteUnsupported { op }`.
> - `xxhash-rust` workspace dependency.
>
> Memory layer (`wasm-dbms-memory`):
>
> - `SchemaRegistry::{stored_snapshots, register_table_from_snapshot,
>   unregister_table, table_registry_page_by_name}`.
> - `IndexLedger::init_from_keys` for snapshot-keyed init paths.
> - `test_utils::write_dummy_schema_snapshot` so the table-registry unit
>   tests (which allocate raw pages and call `TableRegistry::load`) can
>   satisfy the snapshot-ledger decode that landed in #34's prior commit.
>
> Macro additions (`wasm-dbms-macros`):
>
> - Emit `compiled_snapshots_dyn`, `migrate_default_dyn`,
>   `migrate_transform_dyn`, and `renamed_from_dyn` as object-safe
>   siblings of the existing `Sized` dispatch methods.
>
> Benches (`ic-dbms-canister`):
>
> - Replace hand-written `DatabaseSchema` impls in `eager_relation` and
>   `read_table` with `#[derive(DatabaseSchema)]`. ~330 LOC of boilerplate
>   removed; the derive macro covers the same dispatch.

- **migrations:** IC layer endpoints for schema migrations

> Wires the migration surface through the IC layer: `has_drift` and
> `pending_migrations` as queries, `migrate` as an update, all admin-gated
> via the existing ACL check. Adds matching `Client` trait methods with
> implementations for `IcDbmsCanisterClient`, `IcDbmsAgentClient`, and
> `IcDbmsPocketIcClient`, plus wrapper-canister bindings and PocketIC
> coverage exercising both the direct client and wrapper paths on a
> fresh canister (no drift, empty plan, migrate is a no-op).

- **migrations:** WIT surface and IC migration docs

> Add has-drift / pending-migrations / migrate to wit/dbms.wit plus the
> supporting snapshot, op, and policy records. Wire them through the
> example guest and host so the round-trip is exercised end-to-end.
>
> Document the IC migration flow: new docs/ic/guides/migrations.md, full
> Candid signatures in docs/ic/reference/schema.md, and a Schema
> Migrations section in docs/ic/guides/client-api.md.

- **migrations:** snapshot-driven record codec for column-mutating ops
- Breaking: **acl:** granular per-identity permissions (closes #87)

> Replaces the flat allow-list ACL with a granular permission model.
> Each identity carries `admin` / `manage_acl` / `migrate` flags plus
> `TablePerms` (READ/INSERT/UPDATE/DELETE) scoped to all tables or to
> specific tables. CRUD endpoints generated by `#[derive(DbmsCanister)]`
> gate per-operation; migration endpoints gate on `migrate`.
>
> Breaking changes:
>
> - ACL page layout bumped to v2 (no migration from 0.8.x).
> - `IcDbmsCanisterInitArgs.allowed_principals` is `Option<Vec<Principal>>`;
>   `None`/empty bootstraps the deployer as full admin.
> - Canister endpoints `acl_add_principal` / `acl_remove_principal` /
>   `acl_allowed_principals` removed; replaced by `grant_admin`,
>   `revoke_admin`, `grant_manage_acl`, `revoke_manage_acl`,
>   `grant_migrate`, `revoke_migrate`, `grant_all_tables_perms`,
>   `revoke_all_tables_perms`, `grant_table_perms`, `revoke_table_perms`,
>   `remove_identity`, `list_identities`, `my_perms`.
> - New `DbmsError::AccessDenied { table, required }` returned in lieu
>   of trapping for unauthorized CRUD/ACL/migrate calls.
> - New `MemoryError::AclLayoutUnsupported`.

### Fixed

- **bench:** add aggregate to DatabaseSchema impls and CI bench-build job

### Build

- **integration-tests:** use pocket-ic-harness crate

> Replace in-tree pocket-ic-tests-macro and PocketIcTestEnv wrapper with
> the external pocket-ic-harness crate.
>
> - Remove pocket-ic-tests-macro crate (replaced by pocket_ic_harness::test).
> - Remove src/pocket_ic.rs, src/actor.rs, src/wasm.rs and the TestEnv trait.
> - Implement Canister for TestCanister and CanisterSetup for TestCanisterSetup.
> - Add TestEnvExt trait providing env.dbms_canister() and
>   env.dbms_canister_client_integration() helpers.
> - Resolve wasm paths via CARGO_MANIFEST_DIR so loading is cwd-independent.

## 0.8.2

Released on 2026-04-21

### Added

- derive Default on generated *UpdateRequest structs

> All fields on update requests are Option<T>, so Default is always
> derivable. This enables the ..Default::default() struct-update pattern
> when constructing partial updates, removing the need to write None
> for every untouched field.
>
> Bumps workspace version to 0.8.2.

## 0.8.1

Released on 2026-04-05

### Added

- add select_join to Database trait and rename CandidColumnDef to JoinColumnDef

> Move select_join from a #[doc(hidden)] method on WasmDbmsDatabase to a
> proper public method on the Database trait, making it the official API
> for join queries. Rename CandidColumnDef to JoinColumnDef to better
> reflect its purpose as the column definition type that carries table
> provenance for join results. Update all docs to reference select_join
> instead of select_raw for join queries.

## 0.7.2

Released on 2026-04-02

### Fixed

- export macros in wasm-dbms-api

> wasm-dbms-macros were not actually exported as the documentation example were showing

## 0.7.0

Released on 2026-03-31

### Breaking changes

- change MemoryProvider::read and MemoryAccess::read_at to take &mut self

> MemoryProvider::read signature changed from &self to &mut self.

### Added

- B+ tree indexes for accelerated queries

> Add a complete B+ tree index system to wasm-dbms. Every table
> automatically gets an index on its primary key, and users can declare
> additional single-column or composite indexes with the `#[index]`
> attribute.
>
> Key changes:
>
> Memory layer (wasm-dbms-memory):
>
> - IndexLedger: per-table registry mapping column sets to B-tree roots
> - IndexTree: page-per-node B+ tree with variable-size keys, doubly-linked
>   leaves for range scans, and automatic node splitting/merging
> - RecordAddress: lightweight (page, offset) pointer stored in leaf entries
> - SchemaRegistry/TableRegistryPage extended with index_registry_page
> - TableRegistry now owns and exposes an IndexLedger
> - INSERT/UPDATE/DELETE maintain all indexes eagerly
>
> DBMS layer (wasm-dbms):
>
> - FilterAnalyzer: extracts index plans (Eq, Range, In) from query filters
> - IndexReader: unified view merging base B-tree results with transaction
>   overlay additions/removals
> - IndexOverlay: in-memory BTreeMap tracking uncommitted index changes per
>   transaction, flushed on commit, discarded on rollback
> - SELECT, UPDATE, DELETE, and JOIN queries use indexes when a suitable
>   plan is found; remaining filter conditions applied as residual checks
>
> Macro layer (wasm-dbms-macros):
>
> - `#[index]` attribute on fields for single-column indexes
> - `#[index(group = "name")]` for composite indexes
> - Automatic primary key index generation in TableSchema
> - Deduplicated shared macro logic from ic-dbms-macros into wasm-dbms-macros
>
> Also includes CI improvements, dependency updates, and documentation
> updates covering the index memory layout, query optimization, and
> architecture changes.

- add #[unique] attribute for table fields

> Add support for the #[unique] field attribute that enforces uniqueness
> constraints on non-primary-key columns. A unique field automatically
> gets a B+ tree index for efficient O(log n) duplicate detection.
>
> - Parse #[unique] in Table derive macro, set ColumnDef::unique and
>   auto-generate an index for the field
> - Add UniqueConstraintViolation error variant to QueryError
> - Enforce uniqueness in InsertIntegrityValidator and
>   UpdateIntegrityValidator (update allows keeping own value)
> - Add comprehensive tests for insert, update, and transaction scenarios
> - Update schema, errors, and IC reference documentation

- add #[autoincrement] attribute for table fields

> Add support for autoincrement columns in table schemas. Fields annotated
> with `#[autoincrement]` automatically generate sequential values on
> insert, starting from zero and incrementing by one.
>
> Implementation across all layers:
>
> **Memory layer (wasm-dbms-memory):**
>
> - AutoincrementLedger: per-table ledger storing current counter values
>   for each autoincrement column, persisted to a dedicated memory page
> - AutoincrementRegistry: HashMap-based registry mapping column names to
>   their current Value, with custom Encode implementation
> - SchemaRegistry: conditionally allocates an autoincrement page when a
>   table has autoincrement columns (Option<Page> in TableRegistryPage)
> - TableRegistry: integrates AutoincrementLedger as an optional field,
>   exposes autoincrement_next() to get the next value for a column
>
> **API layer (wasm-dbms-api):**
>
> - ColumnDef: add auto_increment field to column definitions
> - MemoryError::AutoincrementOverflow: new error variant returned when
>   a column reaches its type's maximum value (uses checked_add)
> - Filter: support autoincrement columns in query filters
>
> **Macro layer (wasm-dbms-macros):**
>
> - Table derive macro: parse #[autoincrement] attribute on fields,
>   propagate auto_increment flag to generated TableSchema impl
>
> **DBMS layer (wasm-dbms):**
>
> - Database: wire autoincrement through insert operations
> - Transaction overlay: support autoincrement in transactional context
>
> **Supported types:** Int8, Int16, Int32, Int64, Uint8, Uint16, Uint32,
> Uint64. Overflow returns AutoincrementOverflow error to prevent
> duplicate key generation.

### Changed

- remove duplicated macros from ic-dbms-macros

> Remove Encode, Table, CustomDataType, and DatabaseSchema derive macros
> from ic-dbms-macros, keeping only DbmsCanister. These macros were
> duplicated from wasm-dbms-macros with the only differences being crate
> path prefixes and Candid/Serde derives on generated types.
>
> IC crates now re-export the wasm-dbms-macros versions through their
> preludes. To support the IC requirement of Candid-serializable generated
> types, a #[candid] attribute is added to wasm-dbms-macros' Table derive:
> when present, generated Record, InsertRequest, and UpdateRequest types
> derive CandidType, Serialize, and Deserialize.

- Breaking: change MemoryProvider::read and MemoryAccess::read_at to take &mut self

> File-backed providers need mutable access to seek before reading.
> Previously this was worked around with try_clone() on every read.
> Making the trait honest about mutation removes that overhead and
> simplifies implementations.

### Build

- update dependencies

## 0.6.0

Released on 2026-03-02

### Breaking changes

- migrate Principal from built-in to CustomDataType

> Value::Principal and DataTypeKind::Principal removed.
> Principal fields in tables must now use #[custom_type] annotation.
> Existing stable memory schemas are incompatible (fingerprint change).

- restructure workspace into wasm-dbms and ic-dbms layers

> restructure workspace into wasm-dbms and ic-dbms layers

### Added

- **ic-dbms-api:** add CustomValue struct with comparison and hashing
- **ic-dbms-api:** add CustomDataType trait
- **ic-dbms-api:** add Value::Custom variant and accessors
- **ic-dbms-api:** add DataTypeKind::Custom variant and CandidDataTypeKind

> Add Custom(&'static str) variant to DataTypeKind for user-defined types.
> Remove CandidType/Serialize/Deserialize derives from DataTypeKind since
> it no longer needs to cross API boundaries directly. Introduce
> CandidDataTypeKind as the Candid-serializable mirror with Custom(String)
> for the canister API layer. Update CandidColumnDef to use the new type.

- **ic-dbms-macros:** add #[derive(CustomDataType)] macro

> Add a proc-macro derive that generates `impl CustomDataType` (with
> TYPE_TAG constant) and `impl From<T> for Value` for user-defined types.
> The attribute `#[type_tag = "..."]` is required and uses the same
> NameValue parsing pattern as the existing `#[table = "..."]` attribute.

- **ic-dbms-macros:** add #[custom_type] support to Table derive macro

> When a field is annotated with #[custom_type], the generated code uses
> Value::Custom(CustomValue { ... }) instead of Value::FieldType(field)
> for to_values/from_values in TableSchema, Record, InsertRequest, and
> UpdateRequest. This allows user-defined types implementing CustomDataType
> to be used as table columns.

- Breaking: migrate Principal from built-in to CustomDataType
- add wasm-dbms dependency to ic-dbms-canister
- add #[derive(DatabaseSchema)] macro for automatic schema dispatch

> Add a DatabaseSchema derive macro that auto-generates the
> DatabaseSchema<M> trait implementation from a #[tables(...)] attribute,
> eliminating ~130+ lines of boilerplate per schema. Two variants exist:
> a generic one in wasm-dbms-macros and an IC-specific one in
> ic-dbms-macros with IC crate paths. Update examples, tests, and docs.

- add AccessControl trait with associated Id type for runtime-agnostic ACL

> Introduce the AccessControl trait in wasm-dbms-memory to abstract access
> control behind a generic interface. Different runtimes can use different
> identity types: Vec<u8> (AccessControlList), Principal (IcAccessControlList),
> or () (NoAccessControl). The A: AccessControl generic parameter is propagated
> through DbmsContext, WasmDbmsDatabase, DatabaseSchema, integrity validators,
> join engine, and both derive macros. Default type parameters preserve backward
> compatibility.

### Changed

- Breaking: restructure workspace into wasm-dbms and ic-dbms layers

> Split the monolithic ic-dbms crates into a two-layer architecture:
>
> - wasm-dbms (generic layer): runtime-agnostic DBMS engine (wasm-dbms-api,
>   wasm-dbms-memory, wasm-dbms, wasm-dbms-macros)
> - ic-dbms (IC layer): thin adapter for Internet Computer canister
>   integration (ic-dbms-api, ic-dbms-canister, ic-dbms-macros,
>   ic-dbms-client, example, integration-tests)
>
> Also fixes integration test wasm paths to account for the new directory
> depth and updates CI, docs, and build scripts accordingly.

- consolidate IC thread-locals into DbmsContext
- remove duplicated IC database engine module
- update ic-dbms-canister prelude to re-export from wasm-dbms
- update IC API layer to use wasm-dbms database engine
- slim down DbmsCanister macro to IC API only
- update IC canister tests to use wasm-dbms engine
- update CHANGELOG, docs, and API for custom data types and AccessControl trait

> Update CHANGELOG with custom data types, AccessControl, and DatabaseSchema entries.
> Remove CallerContext in favor of AccessControl trait. Update IC macros to use
> generic-layer AccessControl. Update example guest, Cargo.toml dependencies, and
> documentation across wasm-dbms and ic-dbms crates.

- make error types runtime-agnostic and replace ACL panic with error

> Rename IC-specific error variants to runtime-agnostic names
> (StableMemoryError → ProviderError, PrincipalError → IdentityDecodeError),
> add ConstraintViolation variant, replace panic in ACL last-identity removal
> with a proper error, simplify get_referenced_tables by removing thread-local
> cache, and add DbmsContext threading documentation.

### Fixed

- **ic-dbms-macros:** fix nullable custom type codegen using inner type

> When a custom type field is declared as Nullable<T>, the macro now
> correctly uses the inner type T (not Nullable<T>) for trait lookups
> like CustomDataType::TYPE_TAG and Encode::decode in all codegen paths.

- address code review findings

> - Replace String::leak() with OnceLock-based static cache in
>   Value::type_name() for Custom variants to prevent unbounded leaks
> - Add compile-time error when #[custom_type] and #[foreign_key] are
>   combined on the same field

- harden custom data types and add CustomValue constructor

> - Add cache size guard (max 64 entries) to Value::type_name() to
>   prevent unbounded memory leaks on IC
> - Replace panicking .expect() with non-panicking if-let-Ok decode
>   in macro codegen for custom types (record, insert, update)
> - Add CustomValue::new<T>() constructor enforcing consistency between
>   type_tag, encoded bytes, and display string
> - Add Project table with #[custom_type] owner field to example canister
> - Add PocketIC integration tests for custom type CRUD and filtering

- update MSRV to 1.91.1, fix ACL persist-before-panic, fix clippy warnings

> - Set rust-version to 1.91.1 (actual MSRV per cargo msrv) across
>   workspace Cargo.toml, CLAUDE.md, and all docs
> - Replace is_multiple_of (Rust 1.87+) with modulo check for MSRV compat
> - Fix ACL remove_identity to check emptiness before persisting, preventing
>   corrupted state on non-IC runtimes
> - Add #[allow(clippy::approx_constant)] to JSON test module
> - Remove unused _name binding in DatabaseSchema metadata parsing

### Style

- apply nightly rustfmt formatting
- formatted code
- formatted code

## 0.5.0

Released on 2026-02-27

### Breaking changes

- Remove generic T from Query, since it's unnecessary

> Remove `T` from `Query` and `QueryBuilder`

### Added

- Breaking: Remove generic T from Query, since it's unnecessary

> The `T: TableSchema` argument from `Query` and `QueryBuilder` was actually unnecessary, because it didn't provide any meaningful information. The T argument has just been moved to the dbms `select` method, in order to bring information to the selected entity.

- add generic select endpoint for untyped table queries (#10)

> Add a `select_raw` method to the Database trait and a `select` canister
> endpoint that returns `Vec<Vec<(CandidColumnDef, Value)>>`, enabling
> table queries by name without compile-time type information. This lays
> the groundwork for future SQL and JOIN support.

- **ic-dbms-client:** add `select_raw` method to allow selecting untyped columns
- implement JOIN support (INNER, LEFT, RIGHT, FULL) (#47)

> Add user-facing join guide content to the querying and relationships
> docs, create a technical deep-dive for the join engine, and update the
> architecture overview and index with join-related entries.
> Add cross-table join queries with nested-loop join engine, qualified
> column resolution, NULL padding for outer joins, and filter support
> on joined rows. Joins are available through the untyped select_raw
> path and the generated select canister endpoint.

- ic-dbms 0.5.0

> updated getrandom to 0.4

### Performance

- batch fetch foreign keys in eager relation loading (#41)

> Replace per-record N+1 foreign key fetching with a batched approach
> using Filter::In queries. Adds ForeignFetcher::fetch_batch trait method,
> HashSet-based FK deduplication, benchmarks, and uses the existing
> TableColumns type alias throughout.

## 0.4.0

Released on 2026-02-06

### Added

- IcDbmsAgentClient for external systems

> Add a new client implementation using ic-agent to allow external systems
> (frontend applications, backend services, CLI tools) to communicate with
> IC DBMS canisters.
>
> - Add IcDbmsAgentClient with full Client trait implementation
> - Add ic-agent feature flag to ic-dbms-client
> - Add IcAgentError for agent-specific error handling
> - Update documentation with usage examples
> - Reorganize integration tests into separate modules
> - Add comprehensive tests for the agent client
> - Update Rust toolchain to 1.93.0

- **api:** implement JSON filtering for queries (#13) (#30)

> - docs: add JSON filter design document
>
> Design for JSON filtering in ic-dbms queries covering:
>
> - JsonFilter enum with Contains, Extract, HasKey operations
> - JsonCmp enum for comparisons on extracted values
> - Dot notation path syntax with bracket array indices
> - Structural containment (PostgreSQL @> style)
> - Module structure and testing strategy

### Changed

- move ic-dbms-* crates into crates/ directory (#38)

> Move the 4 library crates (ic-dbms-api, ic-dbms-canister, ic-dbms-client,
> ic-dbms-macros) into a crates/ subdirectory for better workspace organization.
> Update all path references in workspace members, include paths, and
> dependency paths across the project.

- **canister:** clean up dbms.rs with multiple improvements (#40)

> - refactor(canister): clean up dbms.rs with multiple improvements
>
> * Extract duplicated record collection pattern from update/delete into
>   collect_matching_records helper method
> * Fix with_transaction to use immutable borrow (get_transaction)
>   instead of unnecessarily getting a mutable reference
> * Extract values_to_schema_entity as a standalone module-level function
>   instead of an inner function inside update
> * Wrap insert oneshot path in atomic for consistency with update/delete
> * Hoist PK lookup outside inner loop in delete_foreign_keys_cascade
> * Use pks.len() for update transaction count instead of re-querying
> * Select only PK column in existing_primary_keys_for_filter
> * Re-export NextRecord from memory module for crate-wide use

- **api:** replace like crate with custom LIKE pattern engine (#42)

> - refactor(api): replace `like` crate with custom LIKE pattern engine
>
> Remove the external `like` dependency and implement an in-house SQL LIKE
> pattern matcher with an iterative two-pointer algorithm. The new engine
> runs in O(n*m) worst-case with O(1) space and zero heap allocation,
> replacing the previous recursive approach that had exponential worst-case
> complexity. Includes full Unicode/multi-byte character support.

### Fixed

- **canister:** apply multi-column order_by sorts in correct order (#39)

> The order_by loop was applying each sort column sequentially, which
> meant only the last column's sort survived. Since Rust's sort_by is
> stable, reversing the iteration order (least-significant column first)
> produces correct multi-column ordering.

### Performance

- **canister:** implement in-place update instead of delete+insert (#37)

> Replace the delete-then-insert update strategy with a proper in-place
> update approach in TableRegistry, improving performance by avoiding
> unnecessary memory reallocation when record size is unchanged.
>
> - Add TableRegistry::update with two-path strategy: in-place overwrite
>   for same-size records, delete+reinsert for size changes
> - Add UpdateIntegrityValidator that allows keeping the same PK during
>   updates, unlike InsertIntegrityValidator which rejects any existing PK
> - Add validate_update to DatabaseSchema trait and macro generation
> - Extract shared validation logic (column, FK, non-nullable checks)
>   into integrity::common module to eliminate duplication
> - Cascade PK changes to referencing tables via
>   update_pk_referencing_updated_table
> - Remove DeleteBehavior::Break variant (workaround for old strategy)
> - Extract sanitize_values helper from insert/update flows
> - Add tests for multi-record updates, PK conflict e2e, FK cascade
>   in transactions, and non-nullable field validation

### Build

- PocketIC 12.x (#29)

## 0.3.0

Released on 2025-12-24

### Added

- Sanitizers (#8)

> - feat: Sanitizers
>
> it is now possible to tag fields for sanitization. Sanitizers can be specified in the schema and will be executed before inserting or updating records.

- Int8, Int16, Uint8, Uint16 data types (#17)

> - feat: Int8, Int16, Uint8, Uint16 data types
>
> Added support for smaller integer types to optimise memory usage and improve performance for applications that require precise control over data sizes

- Added `From` implementation for `Value` for inner types (#18)

> e.g. `u8` to `Value::Uint8`, `rust_decimal::Decimal` to `Value::Decimal`

### Changed

- memory align align checks (#16)

> - refactor: read, write and zero methods must check whether we are records with an offset not aligned
> - test: check whether padding bytes are zeroed when writing
> - test: Test to check the free segment has the new offset and size with padding taken into account
> - docs: changelog pr

### Fixed

- FreeSegmentLedger now uses many pages (#21)

> The FreeSegmentLedger has been updated to utilize multiple pages for tracking free segments.

### Performance

- Use aligned records in memory instead of sequential write (#15)

> Changed the previous memory model, which used to store records sequentially in a contiguous block of memory with padded fields, to a more efficient model that aligns fields based on their data types. This change improves memory access speed and reduces fragmentation.

## 0.2.1

Released on 2025-12-23

### Fixed

- table reader never read the next page

## 0.2.0

Released on 2025-12-21

### Added

- field validation (#6)

> - feat: Table field validation
>
> Added `Validate` trait for validation. Added many common used validators into `prelude`

## 0.1.0

Released on 2025-12-11

### Added

- initial commit
- MemoryProvider
- Memory Manager and Schema
- ACL
- Working on Table Registry
- DeletedRecordsLedger
- TableRegistry Insert Op
- TableReader
- Return also nextRecord position when reading from table
- TableRegistry::delete
- TableRegistry::update
- Data types
- Table types for dbms
- Filter
- QueryBuilder
- prelude
- Insert/Update records
- Transaction
- Implementing Database; Implemented Filter::matches
- Select
- Created Encode derive within ic-dbms-macros
- eager relations loader in select
- DatabaseOverlay
- Select using the overlay
- Validate insert record
- Insert oneshot
- IntegrityValidator trait
- Insert with transaction; rollback and commit
- Delete
- Update
- Example canister
- Derive Table
- API module
- Query must be CandidType + Serialize
- ic-dbms-client init
- ic-dbms-client
- ic-dbms-client example
- Pocket IC client for ic-dbms
- IcDbmsCanister derive macro
- Automatically derive referenced tables
- ic_dbms_canister macro rules

### Changed

- Renamed DeletedRecords to FreeSegments
- Dbms modules
- Moved public API types to ic-dbms-api
- Export ic_dbms_api from ic_dbms_canister

### Fixed

- Records must be prefixed with their length in 2 bytes
- Find adjacent free segments to optimize space
- Removed Delegate
- Load relations from values
- do not register same table twice in the register
- Removed Untyped records; no more necessary
- use ValuesSource to allow loading records correctly when the same table has more than one FK on a record
- Made table registry independent from T
- Do not export ic-dbms-api in ic-dbms-canister
- Missing order_by use
- url
- Changed bin name (will be removed later(
- force use of ic_dbms_api for macros
- Made Query exportable
- error
- get_referencing_tables not working with self referencing tables
- removed macro rules
- missing methods and docs for ic-dbms-client
- table method
- Query candid encoding

### Performance

- Reduced memory usage
- Read page only when the offset is zero.
- Cache referenced tables

### Build

- 0.1

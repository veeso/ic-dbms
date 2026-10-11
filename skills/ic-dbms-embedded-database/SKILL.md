---
name: ic-dbms-embedded-database
description: Use when a single Internet Computer canister needs relational storage for its own entities in stable memory, with tables, typed queries, transactions and schema migrations, but without exposing generic database endpoints or an access control list. Covers ic-dbms-canister DBMS_CONTEXT, WasmDbmsDatabase, DatabaseSchema without DbmsCanister, repositories and upgrades.
---

# ic-dbms Embedded Database

## Overview

The canister keeps its own tables in stable memory through ic-dbms and the
wasm-dbms engine. Only `DatabaseSchema` is derived, never `DbmsCanister`: no
generated CRUD endpoints, no ACL, no `ic_dbms_acl` table. The canister exposes
its own domain API and calls the database in-process.

Versions in this skill: `ic-dbms-canister 0.11`, `wasm-dbms* 0.11`,
`ic-cdk 0.20`, `candid 0.10`.

## When to Use

- One canister owns its data: users, posts, settings, queues, audit logs.
- You want relations, indexes, unique constraints, validation and
  transactions instead of hand-rolled `StableBTreeMap`s.
- Callers must go through business logic, never raw table access.

When NOT to use: several canisters or off-chain agents must share one
database. Use the `ic-dbms-dedicated-canister` skill instead.

## Dependencies

```toml
[lib]
crate-type = ["cdylib", "rlib"] # rlib lets `cargo test` run on the host

[dependencies]
candid = "0.10"
ic-cdk = "0.20"
ic-dbms-canister = "0.11"
serde = { version = "1", features = ["derive"] }
thiserror = "2"
wasm-dbms = "0.11" # the derives expand to ::wasm_dbms paths
wasm-dbms-api = "0.11" # Database trait, Query, Filter, DbmsError
wasm-dbms-memory = "0.11"
```

`.cargo/config.toml` at the workspace root (required by `getrandom`):

```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="custom"']
```

## Module Layout

```text
src/
├── lib.rs          # #[init], #[post_upgrade], endpoints, export_candid!
├── schema.rs       # tables and the Schema struct
├── db.rs           # DBMS_CONTEXT access, transactions, migrations
├── repository.rs   # one function per query; the only code that touches tables
└── error.rs        # AppError wrapping DbmsError
```

Endpoints call domain logic; domain logic calls repositories. Never return
`*Record` types from endpoints: map them to your own Candid types.

## Schema

No `#[candid]`: the records never cross the canister boundary.

```rust
use ic_dbms_canister::prelude::*;

#[derive(Debug, Table, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    #[custom_type]
    pub principal: Principal, // ic_dbms_api Principal: wraps candid::Principal
    #[unique]
    #[sanitizer(TrimSanitizer)]
    #[validate(MaxStrlenValidator(30))]
    pub handle: Text,
    pub bio: Nullable<Text>,
    pub created_at: Uint64, // ic_cdk::api::time()
}

#[derive(Debug, Table, Clone, PartialEq, Eq)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint64,
    // `#[custom_type]` columns cannot carry `#[foreign_key]`: index them and
    // enforce the relation in the repository.
    #[index]
    #[custom_type]
    pub author: Principal,
    pub content: Text,
    #[index]
    pub created_at: Uint64,
}

#[derive(DatabaseSchema, Clone, Copy)]
#[tables(User = "users", Post = "posts")]
pub struct Schema;
```

Each `Table` generates `UserRecord`, `UserInsertRequest`, `UserUpdateRequest`.
Attributes, data types, validators, sanitizers and custom types are documented
at <https://wasm-dbms.cc>.

## Database Access

`ic_dbms_canister::prelude::DBMS_CONTEXT` is the thread-local context on IC
stable memory. On the host (`cargo test`) it uses heap memory, fresh per test
thread.

```rust
use ic_dbms_canister::prelude::{DBMS_CONTEXT, IcMemoryProvider, WasmDbmsDatabase};
use wasm_dbms_api::prelude::{Database as _, DbmsError, MigrationPolicy};

use crate::schema::Schema;

pub fn register_tables() -> Result<(), DbmsError> {
    DBMS_CONTEXT.with(|ctx| Schema::register_tables(ctx))
}

pub fn migrate_if_drifted() -> Result<(), DbmsError> {
    DBMS_CONTEXT.with(|ctx| {
        let mut db = WasmDbmsDatabase::oneshot(ctx, Schema);
        if db.has_drift()? {
            for op in db.pending_migrations()? {
                ic_cdk::println!("pending migration: {op:?}");
            }
            db.migrate(MigrationPolicy::default())?;
        }
        Ok(())
    })
}

/// Runs `f` outside a transaction: every write commits on its own.
pub fn with_db<R>(f: impl FnOnce(&WasmDbmsDatabase<'_, IcMemoryProvider>) -> R) -> R {
    DBMS_CONTEXT.with(|ctx| f(&WasmDbmsDatabase::oneshot(ctx, Schema)))
}

/// Runs `f` in a transaction: commits on `Ok`, rolls back on `Err`.
pub fn transaction<T, E>(
    f: impl FnOnce(&WasmDbmsDatabase<'_, IcMemoryProvider>) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<DbmsError>,
{
    DBMS_CONTEXT.with(|ctx| {
        let tx = ctx.begin_transaction();
        let mut db = WasmDbmsDatabase::from_transaction(ctx, Schema, tx);
        match f(&db) {
            Ok(value) => {
                db.commit()?;
                Ok(value)
            }
            Err(err) => {
                let _ = db.rollback();
                Err(err)
            }
        }
    })
}
```

Never hold a transaction across an `.await`: open, use and close it in one
synchronous block. Open transactions are lost on upgrade.

## Repositories

Repositories take the database handle, so the caller decides between
`with_db` and `transaction`.

```rust
use candid::Principal as CandidPrincipal;
use ic_dbms_canister::prelude::{IcMemoryProvider, Principal, WasmDbmsDatabase};
use wasm_dbms_api::prelude::{Database as _, DeleteBehavior, Filter, Nullable, Query, Value};

use crate::error::AppResult;
use crate::schema::{Post, User, UserInsertRequest, UserRecord, UserUpdateRequest};

type Db<'a, 'ctx> = &'a WasmDbmsDatabase<'ctx, IcMemoryProvider>;

pub fn insert_user(db: Db, principal: CandidPrincipal, handle: String, now: u64) -> AppResult<()> {
    db.insert::<User>(UserInsertRequest {
        principal: Principal(principal),
        handle: handle.into(),
        bio: Nullable::Null,
        created_at: now.into(),
    })?;
    Ok(())
}

pub fn get_user(db: Db, principal: CandidPrincipal) -> AppResult<Option<UserRecord>> {
    let rows = db.select::<User>(
        Query::builder()
            .and_where(Filter::eq("principal", Value::from(Principal(principal))))
            .limit(1)
            .build(),
    )?;
    Ok(rows.into_iter().next())
}

pub fn set_bio(db: Db, principal: CandidPrincipal, bio: Option<String>) -> AppResult<u64> {
    let bio = bio.map_or(Nullable::Null, |bio| Nullable::Value(bio.into()));
    Ok(db.update::<User>(UserUpdateRequest {
        bio: Some(bio), // None leaves the column unchanged
        where_clause: Some(Filter::eq("principal", Value::from(Principal(principal)))),
        ..Default::default()
    })?)
}

pub fn delete_user(db: Db, principal: CandidPrincipal) -> AppResult<u64> {
    let principal = Value::from(Principal(principal));
    db.delete::<Post>(
        DeleteBehavior::Restrict,
        Some(Filter::eq("author", principal.clone())),
    )?;
    Ok(db.delete::<User>(
        DeleteBehavior::Restrict,
        Some(Filter::eq("principal", principal)),
    )?)
}
```

Record fields are `Option<_>` because selects can project columns: map them
into domain types and treat `None` on a selected column as a bug.

```rust
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] wasm_dbms_api::prelude::DbmsError),
    #[error("user not found")]
    UserNotFound,
}

pub type AppResult<T> = Result<T, AppError>;
```

## Lifecycle and Endpoints

```rust
#[ic_cdk::init]
fn init() {
    if let Err(err) = db::register_tables() {
        ic_cdk::trap(format!("failed to register tables: {err}"));
    }
}

#[ic_cdk::post_upgrade]
fn post_upgrade() {
    // The table registry reloads from stable memory; do not register again.
    // `migrate` creates new tables and applies column changes.
    if let Err(err) = db::migrate_if_drifted() {
        ic_cdk::trap(format!("failed to migrate database: {err}"));
    }
}

#[ic_cdk::update]
fn sign_up(handle: String) -> Result<(), String> {
    let caller = ic_cdk::api::msg_caller();
    let now = ic_cdk::api::time();
    db::transaction(|tx| repository::insert_user(tx, caller, handle, now))
        .map_err(|e| e.to_string())
}

ic_cdk::export_candid!();
```

`ic_cdk::export_candid!()` resolves endpoint argument and return types at the
crate root: keep the endpoints in `lib.rs` or import their types there.
Authorize callers in your endpoints yourself; there is no ACL.

`MigrationPolicy::default()` refuses destructive ops (dropped tables or
columns). Rename columns with `#[renamed_from("old")]`, give new non-null
columns `#[default = ...]`, and see <https://wasm-dbms.cc> for `#[migrate]`.

## Tests

Unit tests run on the host against heap memory. Register tables at the start
of each test; each test thread gets an empty database.

```rust
#[test]
fn test_should_insert_and_read_user() {
    db::register_tables().expect("register");
    let alice = candid::Principal::from_slice(&[1; 29]);
    db::with_db(|db| repository::insert_user(db, alice, "alice".into(), 1)).expect("insert");
    let user = db::with_db(|db| repository::get_user(db, alice))
        .expect("select")
        .expect("found");
    assert_eq!(user.handle.unwrap().0, "alice");
}
```

## Common Mistakes

| Symptom                                            | Fix                                                                |
| -------------------------------------------------- | ------------------------------------------------------------------ |
| Derived `DbmsCanister` "to get the API"            | Remove it: it exports raw CRUD endpoints and an ACL                |
| Wrapping the schema in `CanisterSchema`            | Not needed without `DbmsCanister`; pass `Schema` directly          |
| `Principal` type mismatch                          | Wrap with `ic_dbms_canister::prelude::Principal(candid_principal)` |
| `#[custom_type]` and `#[foreign_key]` on one field | Not allowed; index the column and delete dependents in a tx        |
| `cannot find type` in `export_candid!`             | Put endpoints in `lib.rs` or import their types at the crate root  |
| Calls fail with `SchemaDrift` after an upgrade     | Call `migrate_if_drifted` in `post_upgrade`                        |
| Tables registered again in `post_upgrade`          | Drop it; let `migrate` create new tables                           |
| Transaction kept open across `.await`              | Finish the transaction before any inter-canister call              |
| Returning `UserRecord` from an endpoint            | Map to a Candid DTO; records are storage types                     |
| Using `DateTime::now()`                            | Does not exist; store `ic_cdk::api::time()` in a `Uint64`          |

## Reference

- Engine guides (queries, joins, aggregates, transactions, migrations):
  <https://wasm-dbms.cc>
- IC adapter documentation: <https://ic.wasm-dbms.cc>

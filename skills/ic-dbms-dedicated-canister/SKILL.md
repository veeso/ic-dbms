---
name: ic-dbms-dedicated-canister
description: Use when building a standalone database canister on the Internet Computer with ic-dbms, where several canisters or off-chain agents read and write shared tables through a generated Candid API guarded by an access control list (ACL). Covers DbmsCanister, ic-dbms-client, acl_grant, IcDbmsCanisterArgs and schema migrations of a database canister.
---

# ic-dbms Dedicated Database Canister

## Overview

ic-dbms turns Rust structs into a complete database canister. Two derives,
`DatabaseSchema` and `DbmsCanister`, generate typed Candid endpoints per table,
transactions, migrations and an ACL keyed by principal. Other canisters and
agents talk to it through `ic-dbms-client`.

Versions in this skill: `ic-dbms-* 0.11`, `wasm-dbms* 0.11`, `ic-cdk 0.20`,
`candid 0.10`.

## When to Use

- A project with several canisters that must persist into one shared database.
- Off-chain services (`ic-agent`) need typed CRUD access to the data.
- Access must be limited per principal and per table.

When NOT to use: one canister owns its data and nobody else reads it. Embed the
database instead, see the `ic-dbms-embedded-database` skill. No generated
endpoints, no ACL, no inter-canister calls.

## Layout

Put the tables in their own crate so the database canister and every client
canister share the same types.

```text
schema/    # tables only: candid, ic-dbms-api, serde, wasm-dbms-api
database/  # the database canister: DatabaseSchema + DbmsCanister
backend/   # any consumer: ic-dbms-client + schema
```

`.cargo/config.toml` at the workspace root (required by `getrandom`):

```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="custom"']
```

## Schema Crate

```toml
[dependencies]
candid = "0.10"
ic-dbms-api = "0.11"
serde = { version = "1", features = ["derive"] }
wasm-dbms-api = "0.11" # the Table derive expands to ::wasm_dbms_api paths
```

```rust
use candid::CandidType;
use ic_dbms_api::prelude::{
    EmailValidator, MaxStrlenValidator, Nullable, Table, Text, TrimSanitizer, Uint32, Uint64,
};
use serde::Deserialize;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[candid] // adds Candid/Serde derives to UserRecord, UserInsertRequest, UserUpdateRequest
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    #[sanitizer(TrimSanitizer)]
    #[validate(MaxStrlenValidator(100))]
    pub name: Text,
    #[unique]
    #[validate(EmailValidator)]
    pub email: Text,
    pub bio: Nullable<Text>,
    pub created_at: Uint64, // ic_cdk::api::time() nanoseconds
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[candid]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,
    pub content: Text,
    #[index]
    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub author_id: Uint32,
}
```

Attributes, data types, validators and sanitizers are documented at
<https://wasm-dbms.cc>.

## Database Canister

```toml
[lib]
crate-type = ["cdylib"]

[dependencies]
candid = "0.10"
ic-cdk = "0.20"
ic-dbms-api = "0.11"
ic-dbms-canister = "0.11" # features = ["sql"] adds the sql and sql_query endpoints
schema = { path = "../schema" }
wasm-dbms = "0.11"
wasm-dbms-api = "0.11"
wasm-dbms-memory = "0.11"
```

```rust
use ic_dbms_canister::prelude::{DatabaseSchema, DbmsCanister};
// DbmsCanister needs every generated type (UserRecord, UserInsertRequest, ...) in scope.
use schema::*;

#[derive(DatabaseSchema, DbmsCanister)]
#[tables(User = "users", Post = "posts")]
pub struct AppDatabase;

ic_cdk::export_candid!();
```

The macro defines `init`, `pre_upgrade` and `inspect_message`. Do not define
them yourself. You may add a `post_upgrade` (see Migrations).

Generated endpoints per table: `insert_<t>`, `select_<t>`, `update_<t>`,
`delete_<t>`, `aggregate_<t>`. Shared: `select` (untyped, joins),
`begin_transaction`, `commit`, `rollback`, `acl_grant`, `acl_revoke`,
`acl_list`, `my_permissions`, `has_drift`, `pending_migrations`, `migrate`.
Table names starting with `ic_dbms_` are reserved.

Build:

```bash
cargo build --target wasm32-unknown-unknown --release -p database
ic-wasm target/wasm32-unknown-unknown/release/database.wasm -o database.wasm shrink
candid-extractor database.wasm > database.did
```

## Deploy and Grant Access

Init argument: `variant { Init = record { allowed_principals = opt vec { ... } } }`.
Each listed principal becomes `Admin`. With `null` or an empty list, the
deployer becomes `Admin`. Upgrades take `(variant { Upgrade })`.

```bash
dfx deploy database --argument '(variant { Init = record { allowed_principals = null } })'
```

Every caller needs a grant, including your backend canisters. Grant the
narrowest permission: `Read`, `Insert`, `Update`, `Delete` or `Admin`, on one
table (`opt "users"`) or on every table (`null`).

```bash
dfx canister call database acl_grant \
  '(record { principal = principal "<backend-id>"; permission = variant { Insert }; table = opt "users" })'
```

From Rust: `AclGrant::table(p, AclPermission::Read, "users")`,
`AclGrant::all_tables(p, AclPermission::Read)`, `AclGrant::admin(p)`.
Reads with `Query::with` (eager relations) or joins need `Read` on every table
touched. `DeleteBehavior::Cascade` needs `Delete` on every referencing table.

## Calling the Database from Another Canister

```toml
[dependencies]
candid = "0.10"
ic-cdk = "0.20"
ic-dbms-api = "0.11"
ic-dbms-client = "0.11" # features: "ic-agent" for off-chain, "pocket-ic" for tests
schema = { path = "../schema" }
```

Every client method returns `IcDbmsCanisterClientResult<IcDbmsResult<T>>`:
the outer error is the call, the inner one is the database. Unwrap both.

```rust
use ic_dbms_api::prelude::{Filter, Nullable, Query, TableSchema as _, Value};
use ic_dbms_client::prelude::{Client as _, IcDbmsCanisterClient};
use schema::{User, UserInsertRequest, UserRecord, UserUpdateRequest};

async fn create_and_rename(client: &IcDbmsCanisterClient) -> Result<Vec<UserRecord>, String> {
    let insert = UserInsertRequest {
        id: 1.into(),
        name: "Alice".into(),
        email: "alice@example.com".into(),
        bio: Nullable::Null,
        created_at: ic_cdk::api::time().into(),
    };
    client
        .insert::<User>(User::table_name(), insert, None)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let patch = UserUpdateRequest {
        name: Some("Alicia".into()),
        where_clause: Some(Filter::eq("id", Value::from(1u32))),
        ..Default::default()
    };
    client
        .update::<User>(User::table_name(), patch, None)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let query = Query::builder()
        .and_where(Filter::eq("id", Value::from(1u32)))
        .limit(1)
        .build();
    client
        .select::<User>(User::table_name(), query, None)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}
```

Build the client with `IcDbmsCanisterClient::new(database_canister_id)`; keep
the id in canister state, set at `init`. Off-chain:
`IcDbmsAgentClient::new(&agent, canister_id)`. Tests:
`IcDbmsPocketIcClient`.

Transactions span calls: `begin_transaction()` returns an id, pass
`Some(tx)` to each call, then `commit(tx)`. On any error call `rollback(tx)`.
Only the principal that opened a transaction may use it. Open transactions are
dropped on upgrade.

## Migrations

After an upgrade that changes a table, every CRUD and ACL call fails with
schema drift until `migrate` runs. Controllers and `Admin`s may call
`has_drift`, `pending_migrations`, then
`migrate(record { allow_destructive = false })`. Review the pending ops first:
an unexpected `DropTable` or `DropColumn` usually means a renamed table or a
missing `#[renamed_from("old")]`.

To migrate automatically on upgrade:

```rust
use ic_dbms_api::prelude::{Database as _, IcDbmsCanisterArgs, MigrationPolicy};
use ic_dbms_canister::prelude::{CanisterSchema, DBMS_CONTEXT, WasmDbmsDatabase};

#[ic_cdk::post_upgrade]
fn post_upgrade(_args: IcDbmsCanisterArgs) {
    DBMS_CONTEXT.with(|ctx| {
        // Always wrap in CanisterSchema: a bare AppDatabase drops the ACL table.
        let mut db = WasmDbmsDatabase::oneshot(ctx, CanisterSchema::new(AppDatabase));
        let drifted = db
            .has_drift()
            .unwrap_or_else(|err| ic_cdk::trap(format!("drift check failed: {err}")));
        if drifted {
            if let Err(err) = db.migrate(MigrationPolicy::default()) {
                ic_cdk::trap(format!("migration failed: {err}"));
            }
        }
    });
}
```

## Common Mistakes

| Symptom                                             | Fix                                                                                        |
| --------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| `cannot find type UserRecord` in the canister crate | `use schema::*;` so every generated type is in scope                                       |
| Unresolved `::wasm_dbms_api` in derive output       | Add `wasm-dbms-api`; add `wasm-dbms`, `wasm-dbms-memory` where `DatabaseSchema` is derived |
| `getrandom` build error on wasm32                   | Add the `getrandom_backend="custom"` rustflag                                              |
| Backend canister gets `access denied`               | `acl_grant` the backend principal; the deployer grant is not shared                        |
| `#[custom_type]` and `#[foreign_key]` on one field  | Not allowed; index the column and check the relation in code                               |
| Calls fail with `SchemaDrift` after upgrade         | Run `migrate`, or add the `post_upgrade` above                                             |
| Defined `init` or `pre_upgrade` next to the macro   | Remove them; `DbmsCanister` already defines both                                           |
| Opened the database with `AppDatabase` directly     | Use `CanisterSchema::new(AppDatabase)`                                                     |
| Using `DateTime::now()`                             | Does not exist; store `ic_cdk::api::time()` in a `Uint64`                                  |
| Record fields are `Option<_>`                       | Records support column projection; unwrap the selected columns                             |

## Reference

- Guides and Candid reference: <https://ic.wasm-dbms.cc>
- Engine (queries, joins, aggregates, transactions, data types):
  <https://wasm-dbms.cc>
- Complete example canister:
  <https://github.com/veeso/ic-dbms/tree/main/crates/example>

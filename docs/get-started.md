---
title: Get Started
description: A guide to get started with ic-dbms canister
---

- [Setup Development Environment](#setup-development-environment)
- [Setup Canisters](#setup-canisters)
    - [Setup the Schema crate](#setup-the-schema-crate)
    - [Setup the DBMS Canister](#setup-the-dbms-canister)
    - [Deploying the Canister](#deploying-the-canister)
- [Interacting with the Canister](#interacting-with-the-canister)
- [Integration tests](#integration-tests)
- [See Also](#see-also)

This guide will help you get started with the `ic-dbms` canister. Follow the steps below to set up your development
environment and deploy the canister on the Internet Computer.

## Setup Development Environment

I strongly suggest you to setup a Cargo workspace including two crates:

1. A canister crate which is an instance of the `ic-dbms-canister` crate.
2. A crate to define your database schema. This will allow you to reuse those types in a canister which interacts with
   the database canister.

Also, it is required to have the following `config.toml` at `.cargo/config.toml` to bypass the issue with get-random,
which is required for the `uuid` crate:

```toml
[target.wasm32-unknown-unknown]
rustflags = ['--cfg', 'getrandom_backend="custom"']
```

## Setup Canisters

### Setup the Schema crate

First, create a new Rust library crate for your database schema with the following dependencies in your `Cargo.toml`:

```toml
[dependencies]
candid = "0.10"
ic-dbms-api = "0.1"
serde = "1"
```

Then inside of `lib.rs`, define your database schema using Rust structs deriving `CandidType`, `Deserialize`, `Table`
and `Clone`. For example:

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    pub email: Text,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,
    pub content: Text,
    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub user: Uint32,
}
```

Mind that you have to follow the following rules when defining your schema:

- Each table must have a primary key, annotated with `#[primary_key]`.
- Foreign keys must be annotated with `#[foreign_key(entity = "...", table = "...", column = "...")]`.
- Supported types for fields are:
    - `Blob`
    - `Boolean`
    - `Date`
    - `DateTime`
    - `Decimal`
    - `Int8`
    - `Int16`
    - `Int32`
    - `Int64`
    - `Nullable<Type>`
    - `Principal`
    - `Text`
    - `Uint8`
    - `Uint16`
    - `Uint32`
    - `Uint64`
    - `Uuid`

And that's it for the schema crate!
This for each table you want to define in your database will create also the following types:

- `{struct_name}Record`
- `{struct_name}InsertRequest`
- `{struct_name}UpdateRequest`
- `{struct_name}ForeignFetcher` (used only by the ic-dbms-canister internally)

### Setup the DBMS Canister

In order to setup the DBMS canister, you need to create a new Rust project and add the following dependencies to your
`Cargo.toml`:

```toml
[dependencies]
candid = "0.10"
ic-cdk = "0.19"
ic-dbms-api = "0.1"
ic-dbms-canister = "0.1"
serde = "1"
```

Then inside your `lib.rs`, you must setup the schema by just doing the following:

```rust
use ic_dbms_canister::prelude::DbmsCanister;

#[derive(DbmsCanister)]
#[tables(User = "users", Post = "posts")]
pub struct IcDbmsCanisterGenerator;

ic_cdk::export_candid!();
```

The canister API will be automatically generated based on the defined tables, with the following methods:

```candid
service : (IcDbmsCanisterArgs) -> {
  acl_add_principal : (principal) -> (Result);
  acl_allowed_principals : () -> (vec principal) query;
  acl_remove_principal : (principal) -> (Result);
  begin_transaction : () -> (nat);
  commit : (nat) -> (Result);
  delete_posts : (DeleteBehavior, opt Filter_1, opt nat) -> (Result_1);
  delete_users : (DeleteBehavior, opt Filter_1, opt nat) -> (Result_1);
  insert_posts : (PostInsertRequest, opt nat) -> (Result);
  insert_users : (UserInsertRequest, opt nat) -> (Result);
  rollback : (nat) -> (Result);
  select_posts : (Query, opt nat) -> (Result_2) query;
  select_users : (Query_1, opt nat) -> (Result_3) query;
  update_posts : (PostUpdateRequest, opt nat) -> (Result_1);
  update_users : (UserUpdateRequest, opt nat) -> (Result_1);
}
```

This is enough to setup the canister with the tables defined in the schema crate.

> [!NOTE]
> If you want you can add custom logic inside of the canister and export additional methods with the `ic_cdk` macros.
> Mind that at the moment it is not possible to add more logic to the `init` method of the canister.
> Anyway I honestly suggest you to keep the canister as simple as possible and just use it as a database canister. If
> you want to add more complex logic, you can create another canister which interacts with the database canister.

At this point you can just build the canister with:

```sh
mkdir -p "${WASM_DIR}"
echo "Building ${canister_name} Canister"
cargo build --target wasm32-unknown-unknown --release --package "${canister_name}"
ic-wasm "target/wasm32-unknown-unknown/release/${canister_name}.wasm" -o "${WASM_DIR}/${wasm_name}.wasm" shrink
candid-extractor "${WASM_DIR}/${wasm_name}.wasm" > "${WASM_DIR}/${wasm_name}.did"
gzip -k "${WASM_DIR}/${wasm_name}.wasm" --force
```

### Deploying the Canister

The canister has currently the following init arguments:

```candid
type IcDbmsCanisterArgs = variant { Upgrade; Init : IcDbmsCanisterInitArgs };
type IcDbmsCanisterInitArgs = record { allowed_principals : vec principal };
```

So you must provide a `Init` variant of `IcDbmsCanisterArgs` with a list of `allowed_principals` which will be able to
interact with the canister.

> [!WARNING]
> Mind that only principals in this list will be able to interact with the canister, so make sure to include all the
> necessary principals!

## Interacting with the Canister

In order to interact with the canister, you can use the `ic-dbms-client` crate which provides a high-level API to
interact with the canister.

You first need to add the following dependency to your `Cargo.toml`:

```toml
[dependencies]
ic-dbms-api = "0.1"
ic-dbms-client = "0.1"
```

Then you can create a client instance and use it to interact with the canister:

```rust
use ic_dbms_client::{IcDbmsCanisterClient, Client as _};

let principal = Principal::from_text("mxzaz-hqaaa-aaaar-qaada-cai") ?;

let client = IcDbmsCanisterClient::new(principal);

// insert a new user
let alice = UserInsertRequest {
id: 1.into(),
name: "Alice".into(),
email: "alice@example.com".into(),
age: Nullable::Value(30.into()),
};

client
.insert::<User>(User::table_name(), alice, None)
.await? ?;

// select users
let query: Query<User> = Query::builder().all().build();
let users = client
.select::<User>(User::table_name(), query, None)
.await? ?;

for user in users {
println!(
    "User: id={:?}, name={:?}, email={:?}, age={:?}",
    user.id, user.name, user.email, user.age
);
}
```

## Integration tests

If you need to add queries in integration tests and you use `pocket-ic`, you can add `ic-dbms-client` with the
`pocket-ic` feature enabled:

```toml
[dependencies]
ic-dbms-client = { version = "0.1", features = ["pocket-ic"] }
```

Then inside your integration tests you can create a client instance using the `PocketIcAgent`:

```rust
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};

let client = IcDbmsPocketIcClient::new(canister_principal, admin_principal, & pic);

let insert_request = UserInsertRequest {
id: Uint32::from(1),
name: "Alice".into(),
email: "alice@example.com".into(),
};
client
.insert::<User>(User::table_name(), insert_request, None)
.await
.expect("failed to call canister")
.expect("failed to insert user");
```

## See Also

- [Schema Definition](./schema.md)
- [Column Validation](./validation.md)

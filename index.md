---
title: "IC-DBMS Canister"
description: "An Internet Computer framework which provides an easy way to implement a database canister by just providing the database schema."
---

![logo](./assets/images/cargo/logo-128.png)

[![license-mit](https://img.shields.io/crates/l/ic-dbms-canister.svg)](https://opensource.org/licenses/MIT)
[![repo-stars](https://img.shields.io/github/stars/veeso/ic-dbms?style=flat)](https://github.com/veeso/ic-dbms/stargazers)
[![downloads](https://img.shields.io/crates/d/ic-dbms-canister.svg)](https://crates.io/crates/ic-dbms-canister)
[![latest-version](https://img.shields.io/crates/v/ic-dbms-canister.svg)](https://crates.io/crates/ic-dbms-canister)
[![ko-fi](https://img.shields.io/badge/donate-ko--fi-red)](https://ko-fi.com/veeso)
[![conventional-commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white)](https://conventionalcommits.org)

[![ci](https://github.com/veeso/ic-dbms/actions/workflows/ci.yml/badge.svg)](https://github.com/veeso/ic-dbms/actions)
[![coveralls](https://coveralls.io/repos/github/veeso/ic-dbms/badge.svg)](https://coveralls.io/github/veeso/ic-dbms)
[![docs](https://docs.rs/ic-dbms-canister/badge.svg)](https://docs.rs/ic-dbms-canister)

---

## Documentation

### Guides

Step-by-step guides for building database canisters:

- [Getting Started](./docs/guides/get-started.md) - Set up your first ic-dbms canister
- [CRUD Operations](./docs/guides/crud-operations.md) - Insert, select, update, and delete records
- [Querying](./docs/guides/querying.md) - Filters, ordering, pagination, and field selection
- [Transactions](./docs/guides/transactions.md) - ACID transactions with commit/rollback
- [Relationships](./docs/guides/relationships.md) - Foreign keys, delete behaviors, and eager loading
- [Access Control](./docs/guides/access-control.md) - Managing the ACL
- [Client API](./docs/guides/client-api.md) - Using the client library

### Reference

API and type reference documentation:

- [Data Types](./docs/reference/data-types.md) - All supported column types
- [Schema Definition](./docs/reference/schema.md) - Table attributes and generated types
- [Validation](./docs/reference/validation.md) - Built-in and custom validators
- [Sanitization](./docs/reference/sanitization.md) - Built-in and custom sanitizers
- [JSON](./docs/reference/json.md) - JSON data type and filtering
- [Errors](./docs/reference/errors.md) - Error types and handling

### Technical Documentation

For advanced users and contributors:

- [Architecture](./docs/technical/architecture.md) - Three-layer system overview
- [Memory Management](./docs/technical/memory.md) - Stable memory internals

---

## Quick Example

Define your schema:

```rust
use candid::{CandidType, Deserialize};
use ic_dbms_api::prelude::*;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    #[sanitizer(TrimSanitizer)]
    #[validate(MaxStrlenValidator(100))]
    pub name: Text,
    #[validate(EmailValidator)]
    pub email: Text,
}
```

Generate the canister:

```rust
use ic_dbms_canister::prelude::DbmsCanister;

#[derive(DbmsCanister)]
#[tables(User = "users")]
pub struct MyDbmsCanister;

ic_cdk::export_candid!();
```

Use the client:

```rust
use ic_dbms_client::{IcDbmsCanisterClient, Client as _};

let client = IcDbmsCanisterClient::new(canister_id);

// Insert
let user = UserInsertRequest { id: 1.into(), name: "Alice".into(), email: "alice@example.com".into() };
client.insert::<User>(User::table_name(), user, None).await??;

// Query
let query = Query::builder()
    .filter(Filter::eq("name", Value::Text("Alice".into())))
    .build();
let users = client.select::<User>(User::table_name(), query, None).await??;
```

---

## Features

- **Schema-driven**: Define tables as Rust structs with derive macros
- **CRUD operations**: Full insert, select, update, delete support
- **ACID transactions**: Commit/rollback with isolation
- **Foreign keys**: Referential integrity with cascade/restrict/break behaviors
- **Validation & Sanitization**: Built-in validators and sanitizers
- **JSON support**: Store and query semi-structured data
- **Access control**: Principal-based ACL
- **Type-safe client**: Compile-time checked operations

# ic-dbms

![logo](https://ic.wasm-dbms.cc/logo-128.png)

[![license-mit](https://img.shields.io/crates/l/ic-dbms-canister.svg?logo=rust)](https://opensource.org/licenses/MIT)
[![repo-stars](https://img.shields.io/github/stars/veeso/ic-dbms?style=flat)](https://github.com/veeso/ic-dbms/stargazers)
[![downloads](https://img.shields.io/crates/d/ic-dbms-canister.svg?logo=rust)](https://crates.io/crates/ic-dbms-canister)
[![latest-version](https://img.shields.io/crates/v/ic-dbms-canister.svg?logo=rust)](https://crates.io/crates/ic-dbms-canister)
[![conventional-commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white)](https://conventionalcommits.org)

[![ci](https://github.com/veeso/ic-dbms/actions/workflows/ci.yml/badge.svg)](https://github.com/veeso/ic-dbms/actions/workflows/ci.yml)
[![coveralls](https://coveralls.io/repos/github/veeso/ic-dbms/badge.svg)](https://coveralls.io/github/veeso/ic-dbms)
[![docs](https://docs.rs/ic-dbms-canister/badge.svg?logo=rust)](https://docs.rs/ic-dbms-canister)

A framework to build relational database canisters on the
[Internet Computer](https://internetcomputer.org/). Define the schema with Rust
structs, derive two macros, and get a complete database canister with a typed
Candid API, ACID transactions, access control, and client libraries.

## Relationship with wasm-dbms

ic-dbms is the Internet Computer adapter for
[wasm-dbms](https://github.com/veeso/wasm-dbms), a runtime-agnostic relational
database engine for WebAssembly.

- **wasm-dbms** provides the engine: tables, queries, joins, indexes,
  transactions, migrations, and memory management. It is documented at
  <https://wasm-dbms.cc>.
- **ic-dbms** adds what a canister needs: Candid serialization, a generated
  canister API, access control based on IC principals, the stable memory
  provider, canister lifecycle management, and client libraries.

The ic-dbms crates were developed inside the wasm-dbms repository up to version
0.9.0 and moved here with their history.

## Crates

| Crate              | Description                                                                  |
| ------------------ | ---------------------------------------------------------------------------- |
| `ic-dbms-api`      | IC-specific types; re-exports the `wasm-dbms-api` types                      |
| `ic-dbms-canister` | Canister runtime: stable memory provider, API helpers, access control        |
| `ic-dbms-macros`   | The `DbmsCanister` derive macro that generates the canister API              |
| `ic-dbms-client`   | Clients for inter-canister calls, `ic-agent`, and PocketIC integration tests |

## Quick start

Define the tables as Rust structs:

```rust
use candid::CandidType;
use ic_dbms_api::prelude::{Text, Uint32};
use ic_dbms_canister::prelude::{
    DatabaseSchema, DbmsCanister, EmailValidator, MaxStrlenValidator, Table, TrimSanitizer,
};
use serde::Deserialize;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[candid]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    #[sanitizer(TrimSanitizer)]
    #[validate(MaxStrlenValidator(20))]
    pub name: Text,
    #[validate(EmailValidator)]
    pub email: Text,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[candid]
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

Generate the database canister from the schema:

```rust
#[derive(DatabaseSchema, DbmsCanister)]
#[tables(User = "users", Post = "posts")]
pub struct IcDbmsCanisterGenerator;

ic_cdk::export_candid!();
```

The [Get Started guide](https://ic.wasm-dbms.cc/guides/get-started.html) covers
the project setup, the build, the deployment, and the client usage. A complete
canister lives in [`crates/example`](./crates/example).

## Generated canister API

The `DbmsCanister` macro generates these endpoints for every table, plus shared
transaction, access control, and migration endpoints:

```candid
service : (IcDbmsCanisterArgs) -> {
  // Per table
  aggregate_users : (Query, vec AggregateFunction, opt nat64) -> (Result) query;
  delete_users : (DeleteBehavior, opt Filter, opt nat64) -> (Result_2);
  insert_users : (UserInsertRequest, opt nat64) -> (Result_1);
  select_users : (Query, opt nat64) -> (Result_9) query;
  update_users : (UserUpdateRequest, opt nat64) -> (Result_2);
  // Untyped select, with join support
  select : (text, Query, opt nat64) -> (Result_6) query;
  // Transactions
  begin_transaction : () -> (Result_3);
  commit : (nat64) -> (Result_1);
  rollback : (nat64) -> (Result_1);
  // Access control
  acl_grant : (principal, Permission) -> (Result_1);
  acl_revoke : (principal, Permission) -> (Result_1);
  acl_list : () -> (Result_4) query;
  my_permissions : () -> (Result_5) query;
  // Migrations
  has_drift : () -> (Result_3) query;
  migrate : (MigrationPolicy) -> (Result_1);
  pending_migrations : () -> (Result_5) query;
}
```

See the [schema reference](https://ic.wasm-dbms.cc/reference/schema.html) for
the complete interface.

## Comparison with other databases on the Internet Computer

This table compares ic-dbms with the other maintained ways to store structured
data in a canister. It reflects each project's own documentation as of October
2026; check the linked projects for their current state.

|                                     | ic-dbms                                            | [ic-rusqlite]                                    | [ic-sqlite-vfs]                               | [IcyDB]                                             | [ic-stable-structures]             |
| ----------------------------------- | -------------------------------------------------- | ------------------------------------------------ | --------------------------------------------- | --------------------------------------------------- | ---------------------------------- |
| What it is                          | Relational DBMS that generates a database canister | SQLite compiled for WASI, converted by `wasi2ic` | SQLite with a VFS on stable memory            | Typed entity store with a single-entity query layer | Low-level stable collections       |
| Latest release                      | 0.9.0 (April 2026)                                 | 0.5.0 (April 2026)                               | 2.1.0 (September 2026)                        | 0.264 (October 2026), pre-1.0                       | 0.7.2 (September 2025)             |
| License                             | MIT                                                | MIT                                              | MIT or Apache-2.0                             | MIT or Apache-2.0                                   | Apache-2.0                         |
| Query interface                     | Typed Rust query builder and generated Candid API  | SQL through `rusqlite`                           | SQL through its own facade                    | Typed Rust API; optional single-entity SQL subset   | Rust collection API                |
| SQL                                 | No (planned)                                       | Full SQLite dialect                              | Full SQLite dialect, with FTS5 and JSON       | Subset: no joins, subqueries, or CTEs               | No                                 |
| Schema definition                   | Rust structs and derive macros                     | SQL DDL                                          | SQL DDL                                       | Rust model macros and `build.rs`                    | Hand-written Rust types            |
| Generated canister API              | Yes, typed endpoints per table                     | No                                               | No                                            | Admin endpoints only                                | No                                 |
| Transactions                        | ACID, commit and rollback                          | SQLite transactions                              | One update call is one transaction            | Atomic batch of up to 64 entities in one store      | None                               |
| Transactions spanning several calls | Yes, through a transaction id                      | Not documented                                   | No                                            | No                                                  | No                                 |
| Joins                               | INNER, LEFT, RIGHT, FULL                           | Any SQL join                                     | Any SQL join                                  | No                                                  | Manual                             |
| Foreign keys                        | Yes, with delete behaviors                         | Yes                                              | Yes, enabled by default                       | Existence and delete-restrict checks, no cascades   | Manual                             |
| Secondary indexes                   | Single, composite, and unique                      | Yes, including partial and expression indexes    | Yes, including partial and expression indexes | Yes                                                 | Manual                             |
| Aggregates                          | COUNT, SUM, AVG, MIN, MAX, GROUP BY, HAVING        | Full SQL, including window functions             | Full SQL, including window functions          | Single-entity aggregates, GROUP BY, HAVING          | Manual                             |
| Subqueries, CTEs, views, triggers   | No                                                 | Yes                                              | Yes                                           | No                                                  | No                                 |
| Schema migrations                   | Built in, with drift detection                     | Through the third-party `ic-sql-migrate`         | Built in, versioned SQL with checksums        | Built in, optional feature                          | Manual                             |
| Validators and sanitizers           | Built in                                           | SQL `CHECK` constraints                          | SQL `CHECK` constraints                       | Schema constraints                                  | Manual                             |
| Access control                      | Built in, admin principals stored in the database  | No                                               | No                                            | No; generated endpoints are controller-gated        | No                                 |
| Storage                             | Stable memory, own page format                     | SQLite file on a WASI virtual filesystem         | SQLite image in one stable virtual memory     | Stable memory, journaled stores                     | Stable memory                      |
| Client library                      | Inter-canister, `ic-agent`, PocketIC               | None                                             | None                                          | None                                                | Not applicable                     |
| Toolchain                           | `wasm32-unknown-unknown`                           | `wasm32-wasip1` and `wasi2ic` post-processing    | `wasm32-unknown-unknown`, precompiled SQLite  | `wasm32-unknown-unknown` and `build.rs`             | `wasm32-unknown-unknown`           |
| Published benchmarks                | Not yet                                            | Yes, tested up to a 71 GB database               | Yes, instruction counts                       | Not found                                           | Not applicable                     |
| Engine maturity                     | New engine (2025)                                  | SQLite, decades of production use                | SQLite; the VFS layer is new (2026)           | New engine (2025)                                   | Maintained by DFINITY, widely used |

[ic-rusqlite]: https://github.com/wasm-forge/ic-rusqlite
[ic-sqlite-vfs]: https://github.com/humandebri/ic-sqlite-vfs
[IcyDB]: https://github.com/dragginzgame/icydb
[ic-stable-structures]: https://github.com/dfinity/stable-structures

Which one to pick:

- **SQLite (ic-rusqlite or ic-sqlite-vfs)** when you need real SQL: ad-hoc
  queries, subqueries, CTEs, window functions, views, triggers, full-text
  search, or an existing SQL schema to port. SQLite is far more battle-tested
  than any engine written for the Internet Computer.
- **ic-stable-structures** when the data is a handful of key-value maps and
  needs no relations.
- **ic-dbms** when you want a database canister generated from Rust types: a
  typed Candid API per table, access control per principal,
  validators and sanitizers, typed clients for canisters and off-chain agents,
  transactions that span several calls and are owned by the principal that
  opened them, and a plain Rust toolchain with no C code or WASI step.

Other projects exist but are no longer maintained or are not relational:
[ZenDB](https://github.com/NatLabs/ZenDB) (Motoko document database, archived),
[CanDB](https://github.com/ORIGYN-SA/CanDB) (Motoko NoSQL store, last commit in
2024), [Juno](https://github.com/junobuild/juno) datastore (maintenance mode),
[Sudograph](https://github.com/sudograph/sudograph) (abandoned in 2021), and
[froghub ic-sqlite](https://github.com/froghub-io/ic-sqlite) (abandoned in 2023).

## Features

- [x] Tables defined as Rust structs
- [x] CRUD operations
- [x] Queries with filtering, ordering, and pagination
- [x] Foreign keys and eager loading of relations
- [x] JOIN operations between tables
- [x] Aggregates with GROUP BY and HAVING
- [x] Transactions with commit and rollback
- [x] Indexes
- [x] Validators, sanitizers, and custom data types
- [x] Schema migrations
- [x] Access control per principal (admin list), ready for per-table permissions
- [x] Clients for canisters, `ic-agent`, and PocketIC
- [ ] SQL query support

## Documentation

Read the documentation at <https://ic.wasm-dbms.cc>. The engine documentation
(querying, transactions, relationships, data types, validation) is at
<https://wasm-dbms.cc>.

## Development

Every task runs through a [just](https://just.systems) recipe. Run `just` to
list them all.

```sh
just build_all        # build the crates for wasm32 and the canister artifacts
just test             # run unit and documentation tests
just integration_test # run the PocketIC integration tests
just check            # run the local quality gate
just docs_build       # build the documentation site
```

Building the canisters requires
[`ic-wasm`](https://github.com/dfinity/ic-wasm) and
[`candid-extractor`](https://crates.io/crates/candid-extractor). See
[CONTRIBUTING.md](./CONTRIBUTING.md) for the complete setup.

## License

This project is licensed under the MIT License. See the [LICENSE](./LICENSE)
file for details.

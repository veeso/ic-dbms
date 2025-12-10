# ic-dbms-client

![logo](../assets/images/cargo/logo-128.png)

[![license-mit](https://img.shields.io/crates/l/ic-dbms-canister.svg)](https://opensource.org/licenses/MIT)
[![repo-stars](https://img.shields.io/github/stars/veeso/ic-dbms?style=flat)](https://github.com/veeso/ic-dbms/stargazers)
[![downloads](https://img.shields.io/crates/d/ic-dbms-client.svg)](https://crates.io/crates/ic-dbms-client)
[![latest-version](https://img.shields.io/crates/v/ic-dbms-client.svg)](https://crates.io/crates/ic-dbms-client)
[![ko-fi](https://img.shields.io/badge/donate-ko--fi-red)](https://ko-fi.com/veeso)
[![conventional-commits](https://img.shields.io/badge/Conventional%20Commits-1.0.0-%23FE5196?logo=conventionalcommits&logoColor=white)](https://conventionalcommits.org)

[![ci](https://github.com/veeso/ic-dbms/actions/workflows/ci.yml/badge.svg)](https://github.com/veeso/ic-dbms/actions)
[![coveralls](https://coveralls.io/repos/github/veeso/ic-dbms/badge.svg)](https://coveralls.io/github/veeso/ic-dbms)
[![docs](https://docs.rs/ic-dbms-client/badge.svg)](https://docs.rs/ic-dbms-client)

This crate exposes all the types which may be used by an external canister to interact with an IC DBMS Canister instance.

## Usage

### Add the dependency

```toml
[dependencies]
ic-dbms-api = "0.1"
ic-dbms-client = "0.1"
```

### Implement the record types

## Available Types

You can import all the useful types and traits by using the prelude module:

```rust
use ic_dbms_client::prelude::*;
```

### Query

- [`DeleteBehavior`](crate::prelude::DeleteBehavior)
- [`Filter`](crate::prelude::Filter)
- [`Query`](crate::prelude::Query)
- [`QueryBuilder`](crate::prelude::QueryBuilder)
- [`OrderDirection`](crate::prelude::OrderDirection)
- [`Select`](crate::prelude::Select)

### Table

- [`ColumnDef`](crate::prelude::ColumnDef)
- [`ForeignKeyDef`](crate::prelude::ForeignKeyDef)
- [`InsertRecord`](crate::prelude::InsertRecord)
- [`TableColumns`](crate::prelude::TableColumns)
- [`TableError`](crate::prelude::TableError)
- [`TableName`](crate::prelude::TableName)
- [`TableRecord`](crate::prelude::TableRecord)
- [`UpdateRecord`](crate::prelude::UpdateRecord)
- [`ValuesSource`](crate::prelude::ValuesSource)

### Types

- [`Blob`](crate::prelude::Blob)
- [`Boolean`](crate::prelude::Boolean)
- [`Date`](crate::prelude::Date)
- [`DateTime`](crate::prelude::DateTime)
- [`Decimal`](crate::prelude::Decimal)
- [`Int32`](crate::prelude::Int32)
- [`Int64`](crate::prelude::Int64)
- [`Nullable`](crate::prelude::Nullable)
- [`Principal`](crate::prelude::Principal)
- [`Text`](crate::prelude::Text)
- [`Uint32`](crate::prelude::Uint32)
- [`Uint64`](crate::prelude::Uint64)
- [`Uuid`](crate::prelude::Uuid)

### Value

- [`Value`](crate::prelude::Value)

## License

This project is licensed under the MIT License. See the [LICENSE](../LICENSE) file for details.

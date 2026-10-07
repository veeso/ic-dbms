# SQL

ic-dbms canisters can run SQL statements alongside the typed CRUD
endpoints. SQL support is opt-in: without the `sql` feature the canister
does not contain the SQL engine and does not expose the SQL endpoints.

The supported dialect (`SELECT` with joins and aggregates, `INSERT`,
`UPDATE`, `DELETE`, `BEGIN`, `COMMIT`, `ROLLBACK` and `?` parameters) is
described in the [wasm-dbms SQL reference](https://wasm-dbms.cc/reference/sql.html).
The schema is still defined in Rust with `#[derive(Table)]`; SQL only reads
and writes data.

## Enabling SQL on a canister

Enable the `sql` feature of `ic-dbms-canister` in the canister crate:

```toml
[dependencies]
ic-dbms-canister = { version = "0.10", features = ["sql"] }
```

`#[derive(DbmsCanister)]` then generates two more endpoints:

```candid
sql : (text, vec Value, opt nat64) -> (Result_SqlResult);
sql_query : (text, vec Value, opt nat64) -> (Result_SqlResult) query;
```

| Endpoint    | Call type | Runs                                                         |
| ----------- | --------- | ------------------------------------------------------------ |
| `sql`       | update    | Any supported statement                                      |
| `sql_query` | query     | `SELECT` only; anything else returns `SqlError::Unsupported` |

Both take the statement, one `Value` per `?` placeholder in order, and an
optional transaction ID.

## Results

A successful statement returns a `SqlResult`:

| Variant           | Returned by                                                       |
| ----------------- | ----------------------------------------------------------------- |
| `Rows(rows)`      | `SELECT`; each row is a list of column definition and value pairs |
| `RowsAffected(n)` | `INSERT`, `UPDATE`, `DELETE`                                      |
| `TxBegin(id)`     | `BEGIN`                                                           |
| `TxCommit`        | `COMMIT`                                                          |
| `TxRollback`      | `ROLLBACK`                                                        |

For a join, each column definition carries the name of its table.

## Transactions

`BEGIN` returns a transaction ID and makes the caller its owner, exactly
like `begin_transaction`. Pass the ID as the third argument to run later
statements inside the transaction, then send `COMMIT` or `ROLLBACK` with the
same ID. Transactions opened through SQL and through `begin_transaction`
are interchangeable. Using another principal's transaction ID traps.

```rust
use ic_dbms_api::prelude::{SqlResult, Value};
use ic_dbms_client::prelude::Client as _;

let SqlResult::TxBegin(tx) = client.sql("BEGIN", vec![], None).await?? else {
    unreachable!("BEGIN returns TxBegin");
};
client
    .sql(
        "UPDATE users SET name = ? WHERE id = 1",
        vec![Value::from("Alicia")],
        Some(tx),
    )
    .await??;
client.sql("COMMIT", vec![], Some(tx)).await??;
```

## Access control

SQL statements are checked against the [access control list](./access-control.md)
with the same rules as the typed endpoints:

| Statement            | Required permission                                     |
| -------------------- | ------------------------------------------------------- |
| `SELECT`             | `Read` on the `FROM` table and on every joined table    |
| `INSERT`             | `Insert` on the table                                   |
| `UPDATE`             | `Update` on the table                                   |
| `DELETE`             | `Delete` on the table                                   |
| `DELETE ... CASCADE` | `Delete` on the table and on every table referencing it |
| `BEGIN`              | Any grant                                               |
| `COMMIT`, `ROLLBACK` | Ownership of the transaction                            |

The reserved `ic_dbms_acl` table does not exist for SQL, even for admins:
any statement naming it fails with `SqlError::UnknownTable`. Use
`acl_list`, `acl_grant` and `acl_revoke` to manage grants.

## Calling SQL from a client

Enable the `sql` feature of `ic-dbms-client` to get `Client::sql` and
`Client::sql_query`:

```toml
[dependencies]
ic-dbms-client = { version = "0.10", features = ["sql"] }
```

```rust
use ic_dbms_api::prelude::{SqlResult, Value};
use ic_dbms_client::prelude::Client as _;

let result = client
    .sql_query("SELECT name FROM users WHERE id = ?", vec![Value::from(1u32)], None)
    .await??;
if let SqlResult::Rows(rows) = result {
    for row in rows {
        for (column, value) in row {
            println!("{} = {value:?}", column.name);
        }
    }
}
```

## Errors

SQL failures are returned as `IcDbmsError::Sql(SqlError)`. Access control
failures stay `IcDbmsError::Acl`. See the [errors reference](../reference/errors.md#sqlerror)
and the [wasm-dbms SQL reference](https://wasm-dbms.cc/reference/sql.html)
for every `SqlError` variant. `UPDATE` and `DELETE` without a `WHERE` clause
are refused with `SqlError::MissingWhereClause`.

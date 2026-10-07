# Errors Reference (IC)

> **Note:** This is the IC-specific error handling reference. For the complete error hierarchy, all error variants, and their causes, see the [generic errors reference](https://wasm-dbms.cc/reference/errors.html).

- [Overview](#overview)
- [IcDbmsError](#icdbmserror)
- [AclError](#aclerror)
- [SqlError](#sqlerror)
- [Double Result Pattern](#double-result-pattern)
  - [Why Two Results?](#why-two-results)
  - [Using the `??` Operator](#using-the--operator)
  - [Explicit Error Handling](#explicit-error-handling)
- [Client Error Handling Examples](#client-error-handling-examples)
  - [Basic Pattern](#basic-pattern)
  - [Detailed Matching](#detailed-matching)
  - [Helper Function Pattern](#helper-function-pattern)
  - [Retry Pattern for Transient Errors](#retry-pattern-for-transient-errors)

---

## Overview

When using ic-dbms through the `ic-dbms-client` crate, error handling has an additional layer compared to direct wasm-dbms usage. The IC's inter-canister call model introduces network-level errors alongside database-level errors, resulting in the **double Result pattern**.

---

## IcDbmsError

`IcDbmsError` is the error type returned by every endpoint of an ic-dbms
canister. It wraps the engine's `DbmsError` and adds the access control
errors, which the engine knows nothing about:

```rust
use ic_dbms_api::prelude::IcDbmsError;

pub enum IcDbmsError {
    /// The canister access control list refused the call.
    Acl(AclError),
    /// The database engine reported an error.
    Dbms(DbmsError),
    /// The SQL front-end rejected or failed to run a statement
    /// (only with the `sql` feature).
    Sql(SqlError),
}

pub enum AclError {
    AccessDenied {
        required: AclRequirement,
        table: Option<String>,
    },
    AdminGrantWithTable,
    AnonymousPrincipal,
    InvalidTable(String),
    LastAdmin,
}
```

`DbmsError` (`Memory`, `Migration`, `Query`, `Sanitize`, `Table`,
`Transaction`, `Validation`) is documented in the
[generic errors reference](https://wasm-dbms.cc/reference/errors.html).
`IcDbmsError` implements `From` for `DbmsError`, `AclError`, `SqlError` and
every engine error enum, so `?` works in code that mixes them.

---

## AclError

| Variant                            | Meaning                                                                                  |
| ---------------------------------- | ---------------------------------------------------------------------------------------- |
| `AccessDenied { required, table }` | The caller lacks `required`; `table` names the table for permission-specific operations. |
| `AdminGrantWithTable`              | An `Admin` grant named a table; `Admin` always covers every table.                       |
| `AnonymousPrincipal`               | Grants cannot be given to the anonymous principal.                                       |
| `InvalidTable(name)`               | The grant named an unknown or reserved table.                                            |
| `LastAdmin`                        | The operation would remove the last `Admin` grant.                                       |

`required` is `AclRequirement::Permission(permission)` for operations needing
a specific permission. It is `AclRequirement::AnyGrant` when
`begin_transaction` is called by a principal holding no grants.

```rust
use ic_dbms_api::prelude::{AclError, IcDbmsError};

match res {
    Ok(()) => {}
    Err(IcDbmsError::Acl(AclError::AccessDenied { required, table })) => {
        eprintln!("missing access: {required} on {table:?}");
    }
    Err(IcDbmsError::Acl(AclError::LastAdmin)) => {
        eprintln!("grant another admin first");
    }
    Err(other) => return Err(other),
}
```

---

## SqlError

With the `sql` feature, the `sql` and `sql_query` endpoints return
`IcDbmsError::Sql(SqlError)` when a statement cannot be parsed, planned or
run. The variants are defined by wasm-dbms and documented in the
[wasm-dbms SQL reference](https://wasm-dbms.cc/reference/sql.html). Two
cases are specific to canisters:

- `SqlError::Unsupported` with the message `sql_query only runs SELECT
  statements; use sql for writes and transactions`, returned by `sql_query`
  for any other statement.
- `SqlError::UnknownTable("ic_dbms_acl")` for any statement that names the
  reserved access control table.

Engine errors raised while running a statement, such as a primary key
conflict, arrive as `SqlError::Runtime(DbmsError)`.

## Double Result Pattern

### Why Two Results?

Client operations return `Result<Result<T, IcDbmsError>, CallError>`:

```
Result<                          -- Outer: IC call result
    Result<T, IcDbmsError>,      -- Inner: Database operation result
    CallError                    -- Network/canister call error
>
```

- **Outer `Result`** (`CallError`): The inter-canister call itself failed. This happens when:
  - The canister is unreachable or stopped
  - The canister ran out of cycles
  - The message was rejected (e.g., unauthorized caller)
  - Network timeout on agent calls

- **Inner `Result`** (`IcDbmsError`): The call succeeded but the database operation failed. This happens when:
  - Primary key conflict
  - Foreign key constraint violation
  - Validation failure
  - Transaction not found
  - Any other database logic error

### Using the `??` Operator

The simplest approach is to use `??` to unwrap both layers:

```rust
// Propagates both CallError and IcDbmsError
let users = client.select::<User>(User::table_name(), query, None).await??;
```

This requires your function to return an error type that both `CallError` and `IcDbmsError` can convert into (e.g., `Box<dyn std::error::Error>`, `anyhow::Error`, or a custom enum).

### Explicit Error Handling

```rust
match client.insert::<User>(User::table_name(), user, None).await {
    Ok(Ok(())) => {
        // Success: call succeeded AND database operation succeeded
        println!("Insert successful");
    }
    Ok(Err(db_error)) => {
        // Call succeeded but database operation failed
        println!("Database error: {:?}", db_error);
    }
    Err(call_error) => {
        // Inter-canister call itself failed
        println!("Call failed: {:?}", call_error);
    }
}
```

---

## Client Error Handling Examples

### Basic Pattern

```rust
use ic_dbms_api::prelude::{DbmsError, IcDbmsError, QueryError};

let result = client.insert::<User>(User::table_name(), user, None).await;

match result {
    Ok(Ok(())) => println!("Insert successful"),
    Ok(Err(e)) => println!("Database error: {:?}", e),
    Err(e) => println!("Call failed: {:?}", e),
}
```

### Detailed Matching

```rust
match client.insert::<User>(User::table_name(), user, None).await {
    Ok(Ok(())) => {
        println!("Insert successful");
    }
    Ok(Err(db_error)) => {
        match db_error {
            IcDbmsError::Dbms(DbmsError::Query(QueryError::PrimaryKeyConflict)) => {
                println!("User already exists");
            }
            IcDbmsError::Dbms(DbmsError::Query(QueryError::BrokenForeignKeyReference)) => {
                println!("Referenced record doesn't exist");
            }
            IcDbmsError::Dbms(DbmsError::Validation(msg)) => {
                println!("Validation error: {}", msg);
            }
            _ => {
                println!("Database error: {:?}", db_error);
            }
        }
    }
    Err(call_error) => {
        println!("Failed to call canister: {:?}", call_error);
    }
}
```

### Helper Function Pattern

```rust
fn handle_db_error(error: IcDbmsError) -> String {
    match error {
        IcDbmsError::Dbms(DbmsError::Query(QueryError::PrimaryKeyConflict)) =>
            "Record with this ID already exists".to_string(),
        IcDbmsError::Dbms(DbmsError::Query(QueryError::BrokenForeignKeyReference)) =>
            "Referenced record not found".to_string(),
        IcDbmsError::Dbms(DbmsError::Query(QueryError::ForeignKeyConstraintViolation)) =>
            "Cannot delete: record has dependencies".to_string(),
        IcDbmsError::Dbms(DbmsError::Validation(msg)) =>
            format!("Invalid data: {}", msg),
        _ =>
            format!("Unexpected error: {:?}", error),
    }
}

// Usage
let result = client.insert::<User>(User::table_name(), user, None).await;
match result {
    Ok(Ok(())) => Ok(()),
    Ok(Err(e)) => Err(handle_db_error(e)),
    Err(e) => Err(format!("Call failed: {:?}", e)),
}
```

### Retry Pattern for Transient Errors

Network-level errors (outer `Result`) may be transient. Database errors (inner `Result`) are deterministic and should not be retried.

```rust
async fn insert_with_retry<T: Table>(
    client: &impl Client,
    table: &str,
    record: T::InsertRequest,
    max_retries: u32,
) -> Result<(), String> {
    for attempt in 0..max_retries {
        match client.insert::<T>(table, record.clone(), None).await {
            Ok(Ok(())) => return Ok(()),
            Ok(Err(e)) => {
                // Database errors are deterministic - don't retry
                return Err(format!("Database error: {:?}", e));
            }
            Err(call_err) => {
                // Call errors might be transient - retry
                if attempt < max_retries - 1 {
                    println!("Attempt {} failed, retrying...", attempt + 1);
                    continue;
                }
                return Err(format!(
                    "Call failed after {} attempts: {:?}",
                    max_retries, call_err
                ));
            }
        }
    }
    unreachable!()
}
```

# Errors Reference

- [Overview](#overview)
- [Error Hierarchy](#error-hierarchy)
- [IcDbmsError](#icdbmserror)
- [Query Errors](#query-errors)
  - [PrimaryKeyConflict](#primarykeyconflict)
  - [BrokenForeignKeyReference](#brokenforeignkeyreference)
  - [ForeignKeyConstraintViolation](#foreignkeyconstraintviolation)
  - [UnknownColumn](#unknowncolumn)
  - [MissingNonNullableField](#missingnonnullablefield)
  - [RecordNotFound](#recordnotfound)
  - [InvalidQuery](#invalidquery)
- [Transaction Errors](#transaction-errors)
  - [TransactionNotFound](#transactionnotfound)
- [Validation Errors](#validation-errors)
- [Sanitization Errors](#sanitization-errors)
- [Memory Errors](#memory-errors)
- [Client Error Handling](#client-error-handling)
  - [Double Result Pattern](#double-result-pattern)
  - [Error Handling Examples](#error-handling-examples)

---

## Overview

ic-dbms uses a structured error system to provide clear information about what went wrong. Errors are categorized by their source:

| Category | Description |
|----------|-------------|
| Query | Database operation errors (constraints, missing data) |
| Transaction | Transaction state errors |
| Validation | Data validation failures |
| Sanitization | Data sanitization failures |
| Memory | Low-level memory errors |
| Table | Schema/table definition errors |

---

## Error Hierarchy

```
IcDbmsError
├── Query(QueryError)
│   ├── PrimaryKeyConflict
│   ├── BrokenForeignKeyReference
│   ├── ForeignKeyConstraintViolation
│   ├── UnknownColumn
│   ├── MissingNonNullableField
│   ├── RecordNotFound
│   └── InvalidQuery
├── Transaction(TransactionError)
│   └── NotFound
├── Validation(String)
├── Sanitize(String)
├── Memory(MemoryError)
└── Table(TableError)
```

---

## IcDbmsError

The top-level error enum:

```rust
use ic_dbms_api::prelude::IcDbmsError;

pub enum IcDbmsError {
    Memory(MemoryError),
    Query(QueryError),
    Table(TableError),
    Transaction(TransactionError),
    Sanitize(String),
    Validation(String),
}
```

**Matching on error types:**

```rust
match error {
    IcDbmsError::Query(query_err) => {
        // Handle query errors
    }
    IcDbmsError::Transaction(tx_err) => {
        // Handle transaction errors
    }
    IcDbmsError::Validation(msg) => {
        // Handle validation errors
        println!("Validation failed: {}", msg);
    }
    IcDbmsError::Sanitize(msg) => {
        // Handle sanitization errors
        println!("Sanitization failed: {}", msg);
    }
    IcDbmsError::Memory(mem_err) => {
        // Handle memory errors (rare)
    }
    IcDbmsError::Table(table_err) => {
        // Handle table errors (rare)
    }
}
```

---

## Query Errors

Query errors occur during database operations.

### PrimaryKeyConflict

**Cause:** Attempting to insert a record with a primary key that already exists.

```rust
// Insert first user
client.insert::<User>(User::table_name(), UserInsertRequest {
    id: 1.into(),
    name: "Alice".into(),
    ..
}, None).await??;

// Insert second user with same ID - FAILS
let result = client.insert::<User>(User::table_name(), UserInsertRequest {
    id: 1.into(),  // Same ID!
    name: "Bob".into(),
    ..
}, None).await?;

match result {
    Err(IcDbmsError::Query(QueryError::PrimaryKeyConflict)) => {
        println!("A user with this ID already exists");
    }
    _ => {}
}
```

**Solutions:**
- Use a unique primary key (e.g., UUID)
- Check if record exists before inserting
- Use upsert pattern (check, then insert or update)

### BrokenForeignKeyReference

**Cause:** Foreign key references a record that doesn't exist.

```rust
// Insert post with non-existent author
let result = client.insert::<Post>(Post::table_name(), PostInsertRequest {
    id: 1.into(),
    title: "My Post".into(),
    author_id: 999.into(),  // User 999 doesn't exist!
    ..
}, None).await?;

match result {
    Err(IcDbmsError::Query(QueryError::BrokenForeignKeyReference)) => {
        println!("Referenced user does not exist");
    }
    _ => {}
}
```

**Solutions:**
- Ensure referenced record exists before inserting
- Create referenced record first in a transaction

### ForeignKeyConstraintViolation

**Cause:** Attempting to delete a record that is referenced by other records (with `Restrict` behavior).

```rust
// User has posts - cannot delete with Restrict
let result = client.delete::<User>(
    User::table_name(),
    DeleteBehavior::Restrict,
    Some(Filter::eq("id", Value::Uint32(1.into()))),
    None
).await?;

match result {
    Err(IcDbmsError::Query(QueryError::ForeignKeyConstraintViolation)) => {
        println!("Cannot delete: user has related records");
    }
    _ => {}
}
```

**Solutions:**
- Delete related records first
- Use `DeleteBehavior::Cascade` to delete related records automatically
- Use `DeleteBehavior::Break` to break the references

### UnknownColumn

**Cause:** Referencing a column that doesn't exist in the table.

```rust
// Filter with wrong column name
let filter = Filter::eq("username", Value::Text("alice".into()));  // Column is "name", not "username"

let result = client.select::<User>(User::table_name(),
    Query::builder().filter(filter).build(),
    None
).await?;

match result {
    Err(IcDbmsError::Query(QueryError::UnknownColumn)) => {
        println!("Column does not exist in table");
    }
    _ => {}
}
```

**Solutions:**
- Check column names in your schema
- Use IDE autocompletion with typed column names

### MissingNonNullableField

**Cause:** Required field not provided in insert/update.

```rust
// This typically happens at compile time with the generated types,
// but can occur if manually constructing requests or using dynamic queries
```

**Solutions:**
- Provide all required fields
- Use `Nullable<T>` for optional fields

### RecordNotFound

**Cause:** Operation targets a record that doesn't exist.

```rust
// Update non-existent record
let update = UserUpdateRequest::builder()
    .set_name("New Name".into())
    .filter(Filter::eq("id", Value::Uint32(999.into())))  // Doesn't exist
    .build();

let affected = client.update::<User>(User::table_name(), update, None).await??;

// affected == 0 indicates no records matched
if affected == 0 {
    println!("No records found to update");
}
```

**Note:** Update and delete operations return the count of affected rows. A count of 0 isn't necessarily an error but indicates no matches.

### InvalidQuery

**Cause:** Malformed query (invalid JSON path, bad filter syntax, etc.).

```rust
// Invalid JSON path
let filter = Filter::json("metadata", JsonFilter::has_key("user."));  // Trailing dot

let result = client.select::<User>(User::table_name(),
    Query::builder().filter(filter).build(),
    None
).await?;

match result {
    Err(IcDbmsError::Query(QueryError::InvalidQuery)) => {
        println!("Query is malformed");
    }
    _ => {}
}
```

**Common causes:**
- Invalid JSON paths (trailing dots, unclosed brackets)
- Applying JSON filter to non-JSON column
- Type mismatches in comparisons

---

## Transaction Errors

### TransactionNotFound

**Cause:** Invalid transaction ID or transaction already completed.

```rust
// Use invalid transaction ID
let result = client.commit(99999).await?;

match result {
    Err(IcDbmsError::Transaction(TransactionError::NotFound)) => {
        println!("Transaction not found or already completed");
    }
    _ => {}
}
```

**Causes:**
- Transaction ID never existed
- Transaction was already committed
- Transaction was already rolled back
- Caller doesn't own the transaction

---

## Validation Errors

**Cause:** Data fails validation rules.

```rust
#[derive(Table, ...)]
#[table = "users"]
pub struct User {
    #[validate(EmailValidator)]
    pub email: Text,
}

// Insert with invalid email
let result = client.insert::<User>(User::table_name(), UserInsertRequest {
    id: 1.into(),
    email: "not-an-email".into(),  // Invalid!
    ..
}, None).await?;

match result {
    Err(IcDbmsError::Validation(msg)) => {
        println!("Validation failed: {}", msg);
        // msg might be: "Invalid email format"
    }
    _ => {}
}
```

**Common validation errors:**
- String too long (`MaxStrlenValidator`)
- String too short (`MinStrlenValidator`)
- Invalid email format (`EmailValidator`)
- Invalid URL format (`UrlValidator`)
- Invalid phone format (`PhoneNumberValidator`)

---

## Sanitization Errors

**Cause:** Sanitizer fails to process the data.

```rust
// Sanitization errors are rare but can occur with malformed data
match result {
    Err(IcDbmsError::Sanitize(msg)) => {
        println!("Sanitization failed: {}", msg);
    }
    _ => {}
}
```

Sanitization errors are less common than validation errors since sanitizers typically transform data rather than reject it.

---

## Memory Errors

**Cause:** Low-level stable memory errors.

```rust
pub enum MemoryError {
    OutOfBounds,           // Read/write outside allocated memory
    StableMemoryError(String),  // IC stable memory API error
    InsufficientSpace,     // Not enough space to allocate
}
```

**Memory errors are rare** and usually indicate:
- Canister running out of stable memory
- Corrupted memory state
- Bug in ic-dbms (please report!)

---

## Client Error Handling

### Double Result Pattern

Client operations return `Result<Result<T, IcDbmsError>, CallError>`:

- **Outer Result:** Network/call errors (canister unreachable, cycles exhausted)
- **Inner Result:** Database errors (validation, constraints, etc.)

### Error Handling Examples

**Basic with `??`:**

```rust
// Propagate both error types
let users = client.select::<User>(User::table_name(), query, None).await??;
```

**Detailed error handling:**

```rust
match client.insert::<User>(User::table_name(), user, None).await {
    Ok(Ok(())) => {
        println!("Insert successful");
    }
    Ok(Err(db_error)) => {
        // Handle database errors
        match db_error {
            IcDbmsError::Query(QueryError::PrimaryKeyConflict) => {
                println!("User already exists");
            }
            IcDbmsError::Query(QueryError::BrokenForeignKeyReference) => {
                println!("Referenced record doesn't exist");
            }
            IcDbmsError::Validation(msg) => {
                println!("Validation error: {}", msg);
            }
            _ => {
                println!("Database error: {:?}", db_error);
            }
        }
    }
    Err(call_error) => {
        // Handle network/call errors
        println!("Failed to call canister: {:?}", call_error);
    }
}
```

**Helper function pattern:**

```rust
fn handle_db_error(error: IcDbmsError) -> String {
    match error {
        IcDbmsError::Query(QueryError::PrimaryKeyConflict) =>
            "Record with this ID already exists".to_string(),
        IcDbmsError::Query(QueryError::BrokenForeignKeyReference) =>
            "Referenced record not found".to_string(),
        IcDbmsError::Query(QueryError::ForeignKeyConstraintViolation) =>
            "Cannot delete: record has dependencies".to_string(),
        IcDbmsError::Validation(msg) =>
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

**Retry pattern for transient errors:**

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
                // Database errors - don't retry
                return Err(format!("Database error: {:?}", e));
            }
            Err(call_err) => {
                // Call errors - might be transient, retry
                if attempt < max_retries - 1 {
                    println!("Attempt {} failed, retrying...", attempt + 1);
                    continue;
                }
                return Err(format!("Call failed after {} attempts: {:?}", max_retries, call_err));
            }
        }
    }
    unreachable!()
}
```

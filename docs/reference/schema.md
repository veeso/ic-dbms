# Schema Definition

- [Schema Definition](#schema-definition)
  - [Overview](#overview)
  - [Table Definition](#table-definition)
    - [Required Derives](#required-derives)
    - [Table Attribute](#table-attribute)
  - [Column Attributes](#column-attributes)
    - [Primary Key](#primary-key)
    - [Foreign Key](#foreign-key)
    - [Sanitizer](#sanitizer)
    - [Validate](#validate)
    - [Alignment](#alignment)
  - [Generated Types](#generated-types)
    - [Record Type](#record-type)
    - [InsertRequest Type](#insertrequest-type)
    - [UpdateRequest Type](#updaterequest-type)
    - [ForeignFetcher Type](#foreignfetcher-type)
  - [DbmsCanister Macro](#dbmscanister-macro)
    - [Basic Usage](#basic-usage)
    - [Generated API](#generated-api)
  - [Complete Example](#complete-example)
  - [Best Practices](#best-practices)

---

## Overview

ic-dbms schemas are defined entirely in Rust using derive macros and attributes. Each struct represents a database table, and each field represents a column.

**Key concepts:**

- Structs with `#[derive(Table)]` become database tables
- Fields become columns with their types
- Attributes configure primary keys, foreign keys, validation, and more
- The `DbmsCanister` macro generates the complete canister API

---

## Table Definition

### Required Derives

Every table struct must have these derives:

```rust
use candid::{CandidType, Deserialize};
use ic_dbms_api::prelude::*;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
}
```

| Derive | Required | Purpose |
|--------|----------|---------|
| `Table` | Yes | Generates table schema and related types |
| `CandidType` | Yes | Enables Candid serialization |
| `Deserialize` | Yes | Enables deserialization from Candid |
| `Clone` | Yes | Required by the macro system |
| `Debug` | Recommended | Useful for debugging |
| `PartialEq`, `Eq` | Recommended | Useful for comparisons in tests |

### Table Attribute

The `#[table = "name"]` attribute specifies the table name in the database:

```rust
#[derive(Table, ...)]
#[table = "user_accounts"]  // Table name in database
pub struct UserAccount {    // Rust struct name (can differ)
    // ...
}
```

**Naming conventions:**

- Use `snake_case` for table names
- Table names should be plural (e.g., `users`, `posts`, `order_items`)
- Keep names short but descriptive

---

## Column Attributes

### Primary Key

Every table must have exactly one primary key:

```rust
#[derive(Table, ...)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,  // Primary key
    pub name: Text,
}
```

**Primary key rules:**

- Exactly one field must be marked with `#[primary_key]`
- Primary keys must be unique across all records
- Primary keys cannot be null
- Common types: `Uint32`, `Uint64`, `Uuid`, `Text`

**UUID as primary key:**

```rust
#[derive(Table, ...)]
#[table = "orders"]
pub struct Order {
    #[primary_key]
    pub id: Uuid,  // UUID primary key
    pub total: Decimal,
}
```

### Foreign Key

Define relationships between tables:

```rust
#[derive(Table, ...)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,

    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub author_id: Uint32,
}
```

**Attribute parameters:**

| Parameter | Description |
|-----------|-------------|
| `entity` | Rust struct name of the referenced table |
| `table` | Table name (from `#[table = "..."]`) |
| `column` | Column name in the referenced table |

**Nullable foreign key:**

```rust
#[foreign_key(entity = "User", table = "users", column = "id")]
pub manager_id: Nullable<Uint32>,  // Can be null
```

**Self-referential foreign key:**

```rust
#[derive(Table, ...)]
#[table = "categories"]
pub struct Category {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,

    #[foreign_key(entity = "Category", table = "categories", column = "id")]
    pub parent_id: Nullable<Uint32>,
}
```

### Sanitizer

Apply data transformations before storage:

```rust
#[derive(Table, ...)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,

    #[sanitizer(TrimSanitizer)]
    pub name: Text,

    #[sanitizer(LowerCaseSanitizer)]
    #[sanitizer(TrimSanitizer)]
    pub email: Text,

    #[sanitizer(RoundToScaleSanitizer(2))]
    pub balance: Decimal,

    #[sanitizer(ClampSanitizer, min = 0, max = 120)]
    pub age: Uint8,
}
```

**Syntax variants:**

```rust
// Unit struct (no parameters)
#[sanitizer(TrimSanitizer)]

// Tuple struct (positional parameter)
#[sanitizer(RoundToScaleSanitizer(2))]

// Named fields struct
#[sanitizer(ClampSanitizer, min = 0, max = 100)]
```

See [Sanitization Reference](./sanitization.md) for all available sanitizers.

### Validate

Add validation rules:

```rust
#[derive(Table, ...)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,

    #[validate(MaxStrlenValidator(100))]
    pub name: Text,

    #[validate(EmailValidator)]
    pub email: Text,

    #[validate(UrlValidator)]
    pub website: Nullable<Text>,
}
```

**Validation happens after sanitization:**

```rust
#[sanitizer(TrimSanitizer)]           // 1. First: trim whitespace
#[validate(MaxStrlenValidator(100))]  // 2. Then: check length
pub name: Text,
```

See [Validation Reference](./validation.md) for all available validators.

### Alignment

Advanced: Configure memory alignment for dynamic-size tables:

```rust
#[derive(Table, ...)]
#[table = "large_records"]
#[alignment = 64]  // 64-byte alignment
pub struct LargeRecord {
    #[primary_key]
    pub id: Uint32,
    pub data: Text,  // Variable-size field
}
```

**When to use:**

- Performance tuning for specific access patterns
- Optimizing memory layout for large records

**Rules:**

- Minimum alignment is 8 bytes for dynamic types
- Default alignment is 32 bytes
- Fixed-size tables ignore this attribute (alignment equals record size)

> **Caution:** Only change alignment if you understand the performance implications.

---

## Generated Types

The `Table` macro generates several types for each table.

### Record Type

`{StructName}Record` - The full record type returned from queries:

```rust
// Generated from User struct
pub struct UserRecord {
    pub id: Uint32,
    pub name: Text,
    pub email: Text,
}

// Usage
let users: Vec<UserRecord> = client
    .select::<User>(User::table_name(), query, None)
    .await??;

for user in users {
    println!("{}: {}", user.id, user.name);
}
```

### InsertRequest Type

`{StructName}InsertRequest` - Request type for inserting records:

```rust
// Generated from User struct
pub struct UserInsertRequest {
    pub id: Uint32,
    pub name: Text,
    pub email: Text,
}

// Usage
let user = UserInsertRequest {
    id: 1.into(),
    name: "Alice".into(),
    email: "alice@example.com".into(),
};

client.insert::<User>(User::table_name(), user, None).await??;
```

### UpdateRequest Type

`{StructName}UpdateRequest` - Request type for updating records:

```rust
// Generated from User struct (with builder pattern)
let update = UserUpdateRequest::builder()
    .set_name("New Name".into())
    .set_email("new@example.com".into())
    .filter(Filter::eq("id", Value::Uint32(1.into())))
    .build();

// Usage
client.update::<User>(User::table_name(), update, None).await??;
```

**Builder methods:**

- `set_{field_name}(value)` - Set a field value
- `filter(Filter)` - WHERE clause (required)
- `build()` - Build the update request

### ForeignFetcher Type

`{StructName}ForeignFetcher` - Internal type for eager loading:

```rust
// Generated automatically, used internally
// You typically don't interact with this directly
```

---

## DbmsCanister Macro

### Basic Usage

Generate a complete canister API from your tables:

```rust
use ic_dbms_canister::prelude::DbmsCanister;
use my_schema::{User, Post, Comment};

#[derive(DbmsCanister)]
#[tables(User = "users", Post = "posts", Comment = "comments")]
pub struct MyDbmsCanister;

ic_cdk::export_candid!();
```

**Format:** `#[tables(StructName = "table_name", ...)]`

### Generated API

For each table, the macro generates:

```candid
service : (IcDbmsCanisterArgs) -> {
  // For "users" table
  insert_users : (UserInsertRequest, opt nat) -> (Result);
  select_users : (Query, opt nat) -> (Result_Vec_UserRecord) query;
  update_users : (UserUpdateRequest, opt nat) -> (Result_u64);
  delete_users : (DeleteBehavior, opt Filter, opt nat) -> (Result_u64);

  // For "posts" table
  insert_posts : (PostInsertRequest, opt nat) -> (Result);
  select_posts : (Query, opt nat) -> (Result_Vec_PostRecord) query;
  update_posts : (PostUpdateRequest, opt nat) -> (Result_u64);
  delete_posts : (DeleteBehavior, opt Filter, opt nat) -> (Result_u64);

  // Transaction methods
  begin_transaction : () -> (nat);
  commit : (nat) -> (Result);
  rollback : (nat) -> (Result);

  // ACL methods
  acl_add_principal : (principal) -> (Result);
  acl_remove_principal : (principal) -> (Result);
  acl_allowed_principals : () -> (vec principal) query;
}
```

---

## Complete Example

```rust
// schema/src/lib.rs
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

    #[sanitizer(TrimSanitizer)]
    #[sanitizer(LowerCaseSanitizer)]
    #[validate(EmailValidator)]
    pub email: Text,

    pub created_at: DateTime,

    pub is_active: Boolean,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uuid,

    #[validate(MaxStrlenValidator(200))]
    pub title: Text,

    pub content: Text,

    pub published: Boolean,

    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub author_id: Uint32,

    pub metadata: Nullable<Json>,

    pub created_at: DateTime,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "comments"]
pub struct Comment {
    #[primary_key]
    pub id: Uuid,

    #[validate(MaxStrlenValidator(1000))]
    pub content: Text,

    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub author_id: Uint32,

    #[foreign_key(entity = "Post", table = "posts", column = "id")]
    pub post_id: Uuid,

    pub created_at: DateTime,
}
```

```rust
// canister/src/lib.rs
use ic_dbms_canister::prelude::DbmsCanister;
use my_schema::{User, Post, Comment};

#[derive(DbmsCanister)]
#[tables(User = "users", Post = "posts", Comment = "comments")]
pub struct BlogDbmsCanister;

ic_cdk::export_candid!();
```

---

## Best Practices

**1. Keep schema in a separate crate**

```
my-project/
├── schema/           # Reusable types
│   ├── Cargo.toml
│   └── src/lib.rs
└── canister/         # Canister implementation
    ├── Cargo.toml
    └── src/lib.rs
```

**2. Use appropriate primary key types**

```rust
// Sequential IDs - simple, good for internal use
pub id: Uint32,

// UUIDs - better for distributed systems, no guessing
pub id: Uuid,
```

**3. Always validate user input**

```rust
#[validate(MaxStrlenValidator(1000))]  // Prevent huge strings
pub content: Text,

#[validate(EmailValidator)]  // Validate format
pub email: Text,
```

**4. Use nullable for optional fields**

```rust
pub phone: Nullable<Text>,  // Clearly optional
pub bio: Nullable<Text>,
```

**5. Consider sanitization for consistency**

```rust
#[sanitizer(TrimSanitizer)]
#[sanitizer(LowerCaseSanitizer)]
pub email: Text,  // Always lowercase, no whitespace
```

**6. Document your schema**

```rust
/// User account information
#[derive(Table, ...)]
#[table = "users"]
pub struct User {
    /// Unique user identifier
    #[primary_key]
    pub id: Uint32,

    /// User's display name (max 100 chars)
    #[validate(MaxStrlenValidator(100))]
    pub name: Text,
}
```

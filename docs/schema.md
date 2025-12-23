# Schema Definition

This section describes how to define your database schema when using **ic-dbms-canister**, focusing on user-defined
entities and their relationships.

The schema is defined entirely in Rust by annotating structs with derive macros and attributes. Each struct represents a
database table, and each field represents a column.

---

## General Rules

When defining your schema, you **must** follow these rules:

- Every table **must** define a primary key using the `#[primary_key]` attribute.
- Foreign keys **must** be explicitly declared using  
  `#[foreign_key(entity = "...", table = "...", column = "...")]`.
- Only supported data types can be used for fields.
- Each table **must** be annotated with `#[table = "..."]`, which specifies the table name in the database.

---

## Supported Field Types

The following types are supported in schema definitions:

- `Blob`
- `Boolean`
- `Date`
- `DateTime`
- `Decimal`
- `Int32`
- `Int64`
- `Nullable<Type>`
- `Principal`
- `Text`
- `Uint32`
- `Uint64`
- `Uuid`

Using unsupported types will result in a compile-time error.

---

## Examples

### Defining a simple table

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    pub email: Text,
}
```

### Defining a table with foreign keys

```rust
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

### Use a specific memory alignment

It is possible to specify a different memory alignment for a table using the `#[alignment = "..."]` attribute:

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "aligned_table"]
#[alignment = 48]
pub struct AlignedTable {
    #[primary_key]
    pub id: Uint32,
    pub data: Text,
}
```

> [!CAUTION]
>
> Specifying a custom alignment is an advanced feature and should be used with caution. Incorrect alignment can lead to
> performance degradation. Ensure you understand the implications before using this attribute.

> [!NOTE]
>
> If the table has a fixed size (e.g. a table with just `Int32` and `Boolean` fields), the alignment attribute will be
> ignored
> since fixed-size tables have the alignment equal to their size.

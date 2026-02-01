# JSON Data Type

- [JSON Data Type](#json-data-type)
  - [Defining JSON Columns](#defining-json-columns)
  - [Creating JSON Values](#creating-json-values)
  - [JSON Filtering](#json-filtering)
    - [Path Syntax](#path-syntax)
  - [Filter Operations](#filter-operations)
    - [Contains (Structural Containment)](#contains-structural-containment)
    - [Extract (Path Extraction + Comparison)](#extract-path-extraction--comparison)
      - [Available comparison operations](#available-comparison-operations)
      - [Type Conversion](#type-conversion)
    - [HasKey (Path Existence)](#haskey-path-existence)
  - [Combining JSON Filters](#combining-json-filters)
  - [Complete Example](#complete-example)
  - [Error Handling](#error-handling)

The `Json` data type allows you to store and query semi-structured JSON data within your database tables. This is useful
for flexible schemas, metadata storage, or any scenario where the structure of data may vary between records.

## Defining JSON Columns

To use JSON in your schema, simply use the `Json` type for your field:

```rust
use ic_dbms_api::prelude::*;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    pub metadata: Json,  // Flexible JSON storage
}
```

## Creating JSON Values

You can create JSON values from strings using `FromStr`:

```rust
use std::str::FromStr;
use ic_dbms_api::prelude::Json;

fn main() {
    let json = Json::from_str(r#"{"name": "Alice", "age": 30}"#).unwrap();
}
```

Or from `serde_json::Value`:

```rust
use serde_json::json;
use ic_dbms_api::prelude::Json;

fn main() {
    let json = Json::from(json!({"name": "Alice", "age": 30}));
}
```

## JSON Filtering

ic-dbms provides powerful JSON filtering capabilities through the `JsonFilter` enum. You can filter records based on
JSON column contents using three operations:

- **Contains**: Structural containment check
- **Extract**: Extract value at path and compare
- **HasKey**: Check if a path exists

### Path Syntax

Paths use dot notation with bracket array indices:

| Path              | Meaning                        |
|-------------------|--------------------------------|
| `"name"`          | Root-level field `name`        |
| `"user.name"`     | Nested field `user.name`       |
| `"items[0]"`      | First element of `items` array |
| `"users[0].name"` | `name` field of first user     |
| `"data[0][1]"`    | Nested array access            |
| `"[0]"`           | First element of root array    |

---

## Filter Operations

### Contains (Structural Containment)

Checks if the JSON column contains a specified pattern. This implements PostgreSQL `@>` style containment:

- **Objects**: All key-value pairs in the pattern must exist in the target (recursive)
- **Arrays**: All elements in the pattern must exist in the target (order-independent)
- **Primitives**: Must be equal

```rust
use ic_dbms_api::prelude::*;
use std::str::FromStr;

fn main() {
    // Filter where metadata contains {"active": true}
    let pattern = Json::from_str(r#"{"active": true}"#).unwrap();
    let filter = Filter::json("metadata", JsonFilter::contains(pattern));
}
```

**Containment Examples:**

| Target                                   | Pattern                       | Result                      |
|------------------------------------------|-------------------------------|-----------------------------|
| `{"a": 1, "b": 2}`                       | `{"a": 1}`                    | ✓ Match                     |
| `{"a": 1}`                               | `{"a": 1, "b": 2}`            | ✗ No match                  |
| `{"user": {"name": "Alice", "age": 30}}` | `{"user": {"name": "Alice"}}` | ✓ Match                     |
| `[1, 2, 3]`                              | `[3, 1]`                      | ✓ Match (order-independent) |
| `[1, 2]`                                 | `[1, 2, 3]`                   | ✗ No match                  |

### Extract (Path Extraction + Comparison)

Extracts a value at the specified path and applies a comparison operation:

```rust
use ic_dbms_api::prelude::*;

fn main() {
    // Filter where metadata.user.name = "Alice"
    let filter = Filter::json(
        "metadata",
        JsonFilter::extract_eq("user.name", Value::Text("Alice".into()))
    );


    // Filter where metadata.user.age > 18
    let filter = Filter::json(
        "metadata",
        JsonFilter::extract_gt("user.age", Value::Int64(18.into()))
    );

    // Filter where metadata.status is in ["active", "pending"]
    let filter = Filter::json(
        "metadata",
        JsonFilter::extract_in("status", vec![
            Value::Text("active".into()),
            Value::Text("pending".into()),
        ])
    );
}
```

#### Available comparison operations

| Method                     | Description                         |
|----------------------------|-------------------------------------|
| `extract_eq(path, value)`  | Equal                               |
| `extract_ne(path, value)`  | Not equal                           |
| `extract_gt(path, value)`  | Greater than                        |
| `extract_lt(path, value)`  | Less than                           |
| `extract_ge(path, value)`  | Greater than or equal               |
| `extract_le(path, value)`  | Less than or equal                  |
| `extract_in(path, values)` | Value in list                       |
| `extract_is_null(path)`    | Path doesn't exist or value is null |
| `extract_not_null(path)`   | Path exists and value is not null   |

#### Type Conversion

When extracting values, JSON types are converted to DBMS types:

| JSON Type      | DBMS Value       |
|----------------|------------------|
| `null`         | `Value::Null`    |
| `true`/`false` | `Value::Boolean` |
| Integer number | `Value::Int64`   |
| Float number   | `Value::Decimal` |
| String         | `Value::Text`    |
| Array          | `Value::Json`    |
| Object         | `Value::Json`    |

### HasKey (Path Existence)

Checks if a path exists in the JSON structure:

```rust
use ic_dbms_api::prelude::*;

fn main() {
    // Filter where metadata has "email" key
    let filter = Filter::json("metadata", JsonFilter::has_key("email"));

    // Filter where metadata has nested path "user.address.city"
    let filter = Filter::json("metadata", JsonFilter::has_key("user.address.city"));

    // Filter where metadata has array element at index 0
    let filter = Filter::json("metadata", JsonFilter::has_key("items[0]"));
}
```

> **Note:** `HasKey` returns `true` even if the value at the path is `null`. It only checks for path existence.

---

## Combining JSON Filters

JSON filters can be combined with other filters using `and()`, `or()`, and `not()`:

```rust
use ic_dbms_api::prelude::*;
use std::str::FromStr;

fn main() {
    // has email AND user.age > 18
    let filter = Filter::json("metadata", JsonFilter::has_key("email"))
        .and(Filter::json("metadata", JsonFilter::extract_gt("user.age", Value::Int64(18.into()))));

    // role = "admin" OR role = "moderator"
    let filter = Filter::json("metadata", JsonFilter::extract_eq("role", Value::Text("admin".into())))
        .or(Filter::json("metadata", JsonFilter::extract_eq("role", Value::Text("moderator".into()))));

    // Combine with regular filters: id = 1 AND metadata contains {"active": true}
    let pattern = Json::from_str(r#"{"active": true}"#).unwrap();
    let filter = Filter::eq("id", Value::Int32(1.into()))
        .and(Filter::json("metadata", JsonFilter::contains(pattern)));
}
```

## Complete Example

```rust
use ic_dbms_api::prelude::*;
use std::str::FromStr;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "products"]
pub struct Product {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    pub attributes: Json,  // {"color": "red", "size": "M", "tags": ["sale", "new"]}
}

fn main() {
    // Query: Find all red products
    let filter = Filter::json(
        "attributes",
        JsonFilter::extract_eq("color", Value::Text("red".into()))
    );

    // Query: Find products with "sale" tag
    let pattern = Json::from_str(r#"{"tags": ["sale"]}"#).unwrap();
    let filter = Filter::json("attributes", JsonFilter::contains(pattern));

    // Query: Find products that have a size attribute
    let filter = Filter::json("attributes", JsonFilter::has_key("size"));

    // Query: Find red products with price > 100
    let filter = Filter::json("attributes", JsonFilter::extract_eq("color", Value::Text("red".into())))
        .and(Filter::json("attributes", JsonFilter::extract_gt("price", Value::Int64(100.into()))));
}
```

## Error Handling

JSON filter operations may return errors in the following cases:

- **Invalid path syntax**: Empty paths, trailing dots, unclosed brackets, negative indices, or non-numeric array indices
- **Non-JSON column**: Attempting to apply a JSON filter to a column that is not of type `Json`

```rust
use ic_dbms_api::prelude::*;

fn main() {
    // This will return QueryError::InvalidQuery for invalid path
    let filter = Filter::json("metadata", JsonFilter::has_key("user.")); // Trailing dot

    // This will return QueryError::InvalidQuery if "name" column is not Json type
    let filter = Filter::json("name", JsonFilter::has_key("field"));
}
```

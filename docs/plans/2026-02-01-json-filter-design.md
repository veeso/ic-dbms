# JSON Filter Design for ic-dbms

**Date:** 2026-02-01
**Status:** Approved

## Overview

This document describes the design for JSON filtering in ic-dbms queries. The implementation enables filtering records based on JSON column contents using field extraction, structural containment, and key existence checks.

## Goals

1. **Field extraction + comparison**: Query by fields inside JSON (e.g., filter where `json_col.user.name = 'Alice'`)
2. **Containment checks**: Check if JSON contains a pattern (e.g., filter where `json_col` contains `{"active": true}`)
3. **Key existence**: Check if keys/paths exist (e.g., filter where `json_col` has key `email`)

## Data Structures

### JsonCmp

Comparison operations for extracted JSON values:

```rust
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub enum JsonCmp {
    Eq(Value),
    Ne(Value),
    Gt(Value),
    Lt(Value),
    Ge(Value),
    Le(Value),
    In(Vec<Value>),
    IsNull,
    NotNull,
}
```

### JsonFilter

JSON-specific filter operations:

```rust
#[derive(Debug, Clone, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub enum JsonFilter {
    /// Structural containment: column's JSON contains the pattern.
    Contains(Json),
    /// Extract value at path, then compare.
    /// Path uses dot notation with bracket array indices: "user.items[0].name"
    Extract(String, JsonCmp),
    /// Check if path/key exists in the JSON.
    HasKey(String),
}
```

### Filter Extension

Add JSON variant to existing Filter enum:

```rust
pub enum Filter {
    // ... existing variants ...
    /// JSON-specific filter on a column.
    Json(String, JsonFilter),
}
```

## Path Syntax

Paths use dot notation with bracket array indices:

| Path | Meaning |
|------|---------|
| `"name"` | Root-level field `name` |
| `"user.name"` | Nested field `user.name` |
| `"items[0]"` | First element of `items` array |
| `"users[0].name"` | `name` field of first user |
| `"data[0][1]"` | Nested array access |

### Path Segment Representation

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
enum PathSegment {
    Key(String),    // object field
    Index(usize),   // array index
}
```

### Parsing Rules

1. Split on `.` (but not inside brackets)
2. For each segment, check for `[n]` suffix
3. `users[0]` becomes `Key("users")` + `Index(0)`
4. `[0]` alone at start becomes `Index(0)` (for root-level arrays)

### Invalid Paths (Return Error)

- Empty path: `""`
- Trailing dot: `"user."`
- Unclosed bracket: `"items[0"`
- Empty brackets: `"items[]"`
- Negative index: `"items[-1]"`
- Non-numeric index: `"items[abc]"`

## Operations

### Contains (Structural Containment)

PostgreSQL `@>` style containment. The left JSON "contains" the right JSON if:

- **Objects**: All key-value pairs in needle exist in haystack (recursive)
- **Arrays**: All elements in needle exist in haystack (order-independent)
- **Primitives**: Must be equal

**Examples:**

| Haystack | Needle | Result |
|----------|--------|--------|
| `{"a": 1, "b": 2}` | `{"a": 1}` | true |
| `{"a": 1}` | `{"a": 1, "b": 2}` | false |
| `{"user": {"name": "Alice", "age": 30}}` | `{"user": {"name": "Alice"}}` | true |
| `[1, 2, 3]` | `[3, 1]` | true |
| `[1, 2]` | `[1, 2, 3]` | false |

### Extract

Extracts a value at the given path and applies a comparison.

- If path doesn't exist, returns `None`
- `IsNull` matches when path doesn't exist or value is JSON null
- All other comparisons fail when path doesn't exist
- Extracted nested objects/arrays become `Value::Json`
- Extracted primitives convert to corresponding `Value` types

### HasKey

Checks if a path exists in the JSON. Returns `true` if the path can be traversed successfully, `false` otherwise.

## Value Extraction

### Type Conversion

When extracting values for comparison, JSON types map to DBMS types:

| JSON Type | DBMS Value |
|-----------|------------|
| `null` | `Value::Null` |
| `true`/`false` | `Value::Boolean` |
| Integer number | `Value::Int64` |
| Float number | `Value::Decimal` |
| String | `Value::Text` |
| Array | `Value::Json` |
| Object | `Value::Json` |

## Module Structure

```
ic-dbms-api/src/dbms/query/
├── filter.rs              # Filter enum, delegates to json_filter
├── json_filter.rs         # JsonFilter, JsonCmp enums + matches method
└── json_filter/
    ├── path.rs            # PathSegment, parse_path()
    ├── extract.rs         # extract_at_path(), json_value_to_dbms_value()
    └── contains.rs        # json_contains()
```

### JsonFilter::matches

```rust
impl JsonFilter {
    pub fn matches(&self, json: &Json) -> QueryResult<bool> {
        let res = match self {
            JsonFilter::Contains(pattern) => {
                json_contains(&json.value, &pattern.value)
            }
            JsonFilter::Extract(path, cmp) => {
                let segments = parse_path(path)?;
                let extracted = extract_at_path(json, &segments);
                cmp.matches(extracted)
            }
            JsonFilter::HasKey(path) => {
                let segments = parse_path(path)?;
                extract_at_path(json, &segments).is_some()
            }
        };
        Ok(res)
    }
}
```

### JsonCmp::matches

```rust
impl JsonCmp {
    pub fn matches(&self, value: Option<Value>) -> bool {
        match (value, self) {
            (None, JsonCmp::IsNull) => true,
            (None, _) => false,
            (Some(_), JsonCmp::IsNull) => false,
            (Some(v), JsonCmp::NotNull) => !v.is_null(),
            (Some(v), JsonCmp::Eq(target)) => v == *target,
            (Some(v), JsonCmp::Ne(target)) => v != *target,
            (Some(v), JsonCmp::Gt(target)) => v > *target,
            (Some(v), JsonCmp::Lt(target)) => v < *target,
            (Some(v), JsonCmp::Ge(target)) => v >= *target,
            (Some(v), JsonCmp::Le(target)) => v <= *target,
            (Some(v), JsonCmp::In(list)) => list.contains(&v),
        }
    }
}
```

### Filter Integration

```rust
impl Filter {
    pub fn matches(&self, values: &[(ColumnDef, Value)]) -> QueryResult<bool> {
        match self {
            // ... existing variants ...
            Filter::Json(field, json_filter) => {
                let json = values
                    .iter()
                    .find(|(col, _)| col.name == *field)
                    .and_then(|(_, val)| val.as_json())
                    .ok_or_else(|| QueryError::InvalidQuery(
                        format!("Column '{field}' is not a Json type")
                    ))?;
                json_filter.matches(json)
            }
        }
    }
}
```

## Testing Strategy

### path.rs

- Valid paths: `"name"`, `"user.name"`, `"items[0]"`, `"users[0].name"`, `"data[0][1]"`
- Invalid paths: `""`, `"user."`, `"items[0"`, `"items[]"`, `"items[-1]"`, `"items[abc]"`

### extract.rs

- Extract from object: nested keys, missing keys
- Extract from array: valid index, out of bounds
- Type mismatch: key on array, index on object
- Conversion: null, bool, number (int/float), string, nested object/array

### contains.rs

- Object containment: subset, superset, nested, missing keys
- Array containment: subset, order-independent, duplicates
- Primitive equality: all JSON types
- Mixed nesting: objects with arrays, arrays with objects

### json_filter.rs

- `JsonFilter::Contains` with various patterns
- `JsonFilter::Extract` with all `JsonCmp` variants
- `JsonFilter::HasKey` with existing/missing paths
- Error cases: invalid paths, non-JSON columns

### filter.rs

- `Filter::Json` combined with `Filter::And`/`Or`
- JSON filters in realistic query scenarios

## Usage Examples

```rust
// Filter where data.user.name = "Alice"
Filter::Json(
    "data".to_string(),
    JsonFilter::Extract("user.name".to_string(), JsonCmp::Eq(Value::Text("Alice".into())))
)

// Filter where data contains {"active": true}
Filter::Json(
    "data".to_string(),
    JsonFilter::Contains(Json::from_str(r#"{"active": true}"#).unwrap())
)

// Filter where data has "email" key
Filter::Json(
    "data".to_string(),
    JsonFilter::HasKey("email".to_string())
)

// Combined: has email AND user.age > 18
Filter::Json("data".to_string(), JsonFilter::HasKey("email".to_string()))
    .and(Filter::Json(
        "data".to_string(),
        JsonFilter::Extract("user.age".to_string(), JsonCmp::Gt(Value::Int64(18.into())))
    ))
```

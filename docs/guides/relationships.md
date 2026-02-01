# Relationships

- [Overview](#overview)
- [Defining Foreign Keys](#defining-foreign-keys)
  - [Foreign Key Syntax](#foreign-key-syntax)
  - [Foreign Key Constraints](#foreign-key-constraints)
- [Referential Integrity](#referential-integrity)
  - [Insert Validation](#insert-validation)
  - [Update Validation](#update-validation)
- [Delete Behaviors](#delete-behaviors)
  - [Restrict](#restrict)
  - [Cascade](#cascade)
  - [Break](#break)
  - [Choosing a Delete Behavior](#choosing-a-delete-behavior)
- [Eager Loading](#eager-loading)
  - [Basic Eager Loading](#basic-eager-loading)
  - [Multiple Relations](#multiple-relations)
  - [Eager Loading with Filters](#eager-loading-with-filters)
- [Common Patterns](#common-patterns)
  - [One-to-Many](#one-to-many)
  - [Many-to-Many](#many-to-many)
  - [Self-Referential](#self-referential)

---

## Overview

ic-dbms supports foreign key relationships between tables, providing:

- **Referential integrity**: Ensures foreign keys point to valid records
- **Delete behaviors**: Control what happens when referenced records are deleted
- **Eager loading**: Load related records in a single query

---

## Defining Foreign Keys

### Foreign Key Syntax

Use the `#[foreign_key]` attribute to define relationships:

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,
    pub content: Text,

    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub author_id: Uint32,
}
```

**Attribute parameters:**

| Parameter | Description |
|-----------|-------------|
| `entity` | The Rust struct name of the referenced table |
| `table` | The table name (as specified in `#[table = "..."]`) |
| `column` | The column in the referenced table (usually the primary key) |

### Foreign Key Constraints

When you define a foreign key:

1. The field type must match the referenced column type
2. The referenced table must be included in your `DbmsCanister` definition
3. Foreign key values must reference existing records (enforced on insert/update)

```rust
// Both tables must be in the canister definition
#[derive(DbmsCanister)]
#[tables(User = "users", Post = "posts")]
pub struct MyDbmsCanister;
```

---

## Referential Integrity

ic-dbms enforces referential integrity automatically.

### Insert Validation

When inserting a record with a foreign key, the referenced record must exist:

```rust
// This user exists
client.insert::<User>(User::table_name(), UserInsertRequest {
    id: 1.into(),
    name: "Alice".into(),
    ..
}, None).await??;

// Insert post referencing existing user - OK
client.insert::<Post>(Post::table_name(), PostInsertRequest {
    id: 1.into(),
    title: "My Post".into(),
    author_id: 1.into(),  // User 1 exists
    ..
}, None).await??;

// Insert post referencing non-existent user - FAILS
let result = client.insert::<Post>(Post::table_name(), PostInsertRequest {
    id: 2.into(),
    title: "Another Post".into(),
    author_id: 999.into(),  // User 999 doesn't exist
    ..
}, None).await?;

assert!(matches!(
    result,
    Err(IcDbmsError::Query(QueryError::BrokenForeignKeyReference))
));
```

### Update Validation

Updates are also validated:

```rust
// Changing author_id to non-existent user fails
let update = PostUpdateRequest::builder()
    .set_author_id(999.into())  // User 999 doesn't exist
    .filter(Filter::eq("id", Value::Uint32(1.into())))
    .build();

let result = client.update::<Post>(Post::table_name(), update, None).await?;
assert!(matches!(
    result,
    Err(IcDbmsError::Query(QueryError::BrokenForeignKeyReference))
));
```

---

## Delete Behaviors

When deleting a record that is referenced by other records, you must specify how to handle the references.

### Restrict

**Behavior**: Fail if any records reference this one.

```rust
use ic_dbms_api::prelude::DeleteBehavior;

// User has posts - delete fails
let result = client.delete::<User>(
    User::table_name(),
    DeleteBehavior::Restrict,
    Some(Filter::eq("id", Value::Uint32(1.into()))),
    None
).await?;

match result {
    Err(IcDbmsError::Query(QueryError::ForeignKeyConstraintViolation)) => {
        println!("Cannot delete: user has posts");
    }
    _ => {}
}

// Delete posts first, then user
client.delete::<Post>(
    Post::table_name(),
    DeleteBehavior::Restrict,
    Some(Filter::eq("author_id", Value::Uint32(1.into()))),
    None
).await??;

// Now user can be deleted
client.delete::<User>(
    User::table_name(),
    DeleteBehavior::Restrict,
    Some(Filter::eq("id", Value::Uint32(1.into()))),
    None
).await??;
```

**Use when**: You want to prevent accidental data loss. The caller must explicitly handle related records.

### Cascade

**Behavior**: Delete all records that reference this one (recursively).

```rust
// Deletes user AND all their posts
client.delete::<User>(
    User::table_name(),
    DeleteBehavior::Cascade,
    Some(Filter::eq("id", Value::Uint32(1.into()))),
    None
).await??;
```

**Cascade is recursive:**

```rust
// Schema:
// User -> Posts -> Comments
// Deleting a user cascades to posts, which cascades to comments

client.delete::<User>(
    User::table_name(),
    DeleteBehavior::Cascade,
    Some(Filter::eq("id", Value::Uint32(1.into()))),
    None
).await??;
// User deleted
// All user's posts deleted
// All comments on those posts deleted
```

**Use when**: Related records have no meaning without the parent (e.g., comments on a deleted post).

### Break

**Behavior**: Break the foreign key reference. If the foreign key is nullable, set it to null. Otherwise, the reference becomes invalid.

```rust
// Posts' author_id references will be broken
client.delete::<User>(
    User::table_name(),
    DeleteBehavior::Break,
    Some(Filter::eq("id", Value::Uint32(1.into()))),
    None
).await??;

// Posts still exist but author_id points to non-existent user
```

**Use when**: Related records should be preserved but can exist without the parent.

> **Warning**: Using `Break` with non-nullable foreign keys results in orphaned records with invalid references. Consider using `Nullable<Uint32>` for foreign keys where `Break` behavior is desired.

### Choosing a Delete Behavior

| Scenario | Recommended Behavior |
|----------|---------------------|
| User account deletion (keep posts) | `Break` with nullable FK |
| User account deletion (remove everything) | `Cascade` |
| Prevent accidental deletion | `Restrict` |
| Soft delete pattern | Don't delete; use status field |
| Comments on posts | `Cascade` (comments meaningless without post) |
| Products in orders | `Restrict` (orders are historical records) |

---

## Eager Loading

Eager loading fetches related records in a single query, avoiding N+1 query problems.

### Basic Eager Loading

Use `.with()` to eager load a related table:

```rust
// Load posts with their authors
let query = Query::builder()
    .all()
    .with("users")  // Name of the related table
    .build();

let posts = client.select::<Post>(Post::table_name(), query, None).await??;

// Each post now has author data available
for post in posts {
    println!("Post '{}' by author_id {}", post.title, post.author_id);
}
```

### Multiple Relations

Load multiple related tables:

```rust
// Schema:
// Post -> User (author)
// Post -> Category

let query = Query::builder()
    .all()
    .with("users")
    .with("categories")
    .build();

let posts = client.select::<Post>(Post::table_name(), query, None).await??;
```

### Eager Loading with Filters

Combine eager loading with filters:

```rust
// Load published posts with their authors
let query = Query::builder()
    .filter(Filter::eq("published", Value::Boolean(true)))
    .order_by("created_at", OrderDirection::Descending)
    .limit(10)
    .with("users")
    .build();

let posts = client.select::<Post>(Post::table_name(), query, None).await??;
```

---

## Common Patterns

### One-to-Many

A user has many posts:

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "posts"]
pub struct Post {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,
    #[foreign_key(entity = "User", table = "users", column = "id")]
    pub author_id: Uint32,
}

// Query all posts by a user
let query = Query::builder()
    .filter(Filter::eq("author_id", Value::Uint32(user_id.into())))
    .build();
let user_posts = client.select::<Post>(Post::table_name(), query, None).await??;
```

### Many-to-Many

Use a junction table for many-to-many relationships:

```rust
// Students and Courses (many-to-many)

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "students"]
pub struct Student {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "courses"]
pub struct Course {
    #[primary_key]
    pub id: Uint32,
    pub title: Text,
}

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "enrollments"]
pub struct Enrollment {
    #[primary_key]
    pub id: Uint32,
    #[foreign_key(entity = "Student", table = "students", column = "id")]
    pub student_id: Uint32,
    #[foreign_key(entity = "Course", table = "courses", column = "id")]
    pub course_id: Uint32,
    pub enrolled_at: DateTime,
}

// Find all courses for a student
let query = Query::builder()
    .filter(Filter::eq("student_id", Value::Uint32(student_id.into())))
    .with("courses")
    .build();
let enrollments = client.select::<Enrollment>(Enrollment::table_name(), query, None).await??;
```

### Self-Referential

A table can reference itself (e.g., categories with parent categories, employees with managers):

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "employees"]
pub struct Employee {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    #[foreign_key(entity = "Employee", table = "employees", column = "id")]
    pub manager_id: Nullable<Uint32>,  // Nullable for top-level employees
}

// Find all employees under a manager
let query = Query::builder()
    .filter(Filter::eq("manager_id", Value::Uint32(manager_id.into())))
    .build();
let direct_reports = client.select::<Employee>(Employee::table_name(), query, None).await??;
```

```rust
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "categories"]
pub struct Category {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    #[foreign_key(entity = "Category", table = "categories", column = "id")]
    pub parent_id: Nullable<Uint32>,  // Nullable for root categories
}

// Find root categories
let query = Query::builder()
    .filter(Filter::is_null("parent_id"))
    .build();
let root_categories = client.select::<Category>(Category::table_name(), query, None).await??;

// Find children of a category
let query = Query::builder()
    .filter(Filter::eq("parent_id", Value::Uint32(parent_id.into())))
    .build();
let children = client.select::<Category>(Category::table_name(), query, None).await??;
```

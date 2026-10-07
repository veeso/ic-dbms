# Access Control (IC)

> **Note:** This is the IC-specific access control guide. Access control is an
> IC-only feature; the wasm-dbms engine has none and leaves it to the embedder.

ic-dbms keeps an Access Control List (ACL) inside the database itself, in a
reserved table called `ic_dbms_acl` that `#[derive(DbmsCanister)]` always
registers next to your tables. Each row is one **grant**: a principal, a
permission and an optional table. A principal may hold many grants. A
principal with no grants may call only `my_permissions`.

## Permissions and grants

```rust
pub enum AclPermission {
    /// Everything: every operation on every table, transactions, ACL
    /// management and schema migrations.
    Admin,
    /// Select and aggregate on the granted table.
    Read,
    /// Insert into the granted table.
    Insert,
    /// Update rows of the granted table.
    Update,
    /// Delete rows of the granted table.
    Delete,
}

pub struct AclGrant {
    pub principal: Principal,
    pub permission: AclPermission,
    /// `None` means every table, including tables added later.
    pub table: Option<String>,
}
```

| Grant                                 | Meaning                                               |
| ------------------------------------- | ----------------------------------------------------- |
| `AclGrant::admin(p)`                  | `p` may do everything. Always has `table: None`.      |
| `AclGrant::table(p, Read, "users")`   | `p` may select and aggregate `users`.                 |
| `AclGrant::all_tables(p, Read)`       | `p` may read every table, including ones added later. |
| `AclGrant::table(p, Insert, "users")` | `p` may insert into `users`.                          |

Rules:

- Granting the same grant twice stores one row.
- An `Admin` grant naming a table is refused (`AdminGrantWithTable`).
- The table must be one of the canister's tables (`InvalidTable`); the
  reserved `ic_dbms_acl` table cannot be granted.
- The anonymous principal cannot receive grants (`AnonymousPrincipal`).
- Revoking a grant that is not stored is a no-op.
- At least one `Admin` grant must remain (`LastAdmin`).

## Initialization

```rust
use ic_dbms_api::prelude::IcDbmsCanisterInitArgs;

let args = IcDbmsCanisterInitArgs {
    allowed_principals: Some(vec![operator_principal]),
};
```

| `allowed_principals` | Result                                      |
| -------------------- | ------------------------------------------- |
| `None`               | The deployer principal becomes an admin.    |
| `Some(vec![])`       | Same as `None`: the deployer becomes admin. |
| `Some(vec![p, q])`   | Each listed principal becomes an admin.     |

`init` registers your tables, then the `ic_dbms_acl` table, then stores an
`Admin` grant for each principal. The anonymous principal makes `init` trap.

## Endpoints

| Endpoint         | Kind   | Required permission | Effect                                  |
| ---------------- | ------ | ------------------- | --------------------------------------- |
| `acl_grant`      | update | `Admin`             | Store a grant. Idempotent.              |
| `acl_revoke`     | update | `Admin`             | Remove a grant. Refuses the last admin. |
| `acl_list`       | query  | `Admin`             | List every grant.                       |
| `my_permissions` | query  | none                | Return exactly the caller's grants.     |

```candid
type AclPermission = variant { Admin; Read; Insert; Update; Delete };
type AclGrant = record {
  principal  : principal;
  permission : AclPermission;
  table      : opt text;
};

acl_grant      : (AclGrant) -> (variant { Ok; Err : IcDbmsError });
acl_revoke     : (AclGrant) -> (variant { Ok; Err : IcDbmsError });
acl_list       : () -> (variant { Ok : vec AclGrant; Err : IcDbmsError }) query;
my_permissions : () -> (variant { Ok : vec AclGrant; Err : IcDbmsError }) query;
```

### Enforcement

Every generated endpoint except `my_permissions` checks the caller before
touching data. A grant on a named table matches only that table; a grant with
no table matches every user table; `Admin` matches everything.

| Endpoint                                     | Required                                                                  |
| -------------------------------------------- | ------------------------------------------------------------------------- |
| `select_<t>`, `aggregate_<t>`                | `Read` on `t` and on every relation loaded eagerly (`QueryBuilder::with`) |
| `select` (untyped)                           | `Read` on the root table, every joined table and every eager relation     |
| `insert_<t>`                                 | `Insert` on `t`                                                           |
| `update_<t>`                                 | `Update` on `t`                                                           |
| `delete_<t>` with `Restrict`                 | `Delete` on `t`                                                           |
| `delete_<t>` with `Cascade`                  | `Delete` on `t` and on every table that references `t`, transitively      |
| `begin_transaction`                          | At least one grant                                                        |
| `commit`, `rollback`                         | The principal that opened the transaction                                 |
| `acl_grant`, `acl_revoke`, `acl_list`        | `Admin`                                                                   |
| `has_drift`, `pending_migrations`, `migrate` | `Admin` or a canister controller                                          |

Operations inside a transaction are checked one by one, exactly as outside
it: opening a transaction does not widen what the caller may do.

SQL statements sent to `sql` or `sql_query` (with the `sql` feature) are
checked with the same rules, per statement; see the
[SQL guide](./sql.md#access-control).

The reserved `ic_dbms_acl` table is never matched by a table-less grant, so
only `Admin` can read it through the untyped `select`, directly or through a
join.

The check reads the caller's grants through the same database view as the
operation, using the index on the `principal` column.

Transaction ownership is tracked in the canister heap, not in the engine:
`begin_transaction` records the caller next to the new id, every call that
names the id compares the caller with that record, and commit or rollback
removes it. A mismatch and an unknown id trap with the same message.

### Controllers and schema drift

The ACL lives in the database, so while the schema has drifted after an
upgrade every read of the ACL table fails with `SchemaDrift`, exactly like a
CRUD call would. To avoid locking the canister, its controllers may always
call `has_drift`, `pending_migrations` and `migrate`. Controllers can already
reinstall or delete the canister, so this grants them nothing new. ACL
management endpoints are unavailable during drift: run `migrate` first.

### Reserved table names

Table names starting with `ic_dbms_` are reserved. Declaring one in
`#[tables(...)]` is a compile-time error:

```text
error: table name `ic_dbms_acl` is reserved: names starting with `ic_dbms_` belong to ic-dbms
```

## Errors

| Error                                                          | Cause                                                   |
| -------------------------------------------------------------- | ------------------------------------------------------- |
| `IcDbmsError::Acl(AclError::AccessDenied { required, table })` | The caller lacks the specified permission or any grant. |
| `IcDbmsError::Acl(AclError::AdminGrantWithTable)`              | `acl_grant` of `Admin` naming a table.                  |
| `IcDbmsError::Acl(AclError::InvalidTable(name))`               | `acl_grant` naming an unknown or reserved table.        |
| `IcDbmsError::Acl(AclError::AnonymousPrincipal)`               | `acl_grant` or `init` targeted the anonymous principal. |
| `IcDbmsError::Acl(AclError::LastAdmin)`                        | `acl_revoke` would remove the last `Admin` grant.       |

`AccessDenied.required` is `AclRequirement::Permission(permission)` for a
specific operation and `AclRequirement::AnyGrant` when an unlisted principal
tries to open a transaction. `table` is present only for a table-specific
permission check.

See the [errors reference](../reference/errors.md) for the full `IcDbmsError`
type.

## Recipes

### Read-only frontend

```rust
use ic_dbms_api::prelude::{AclGrant, AclPermission};

client
    .acl_grant(AclGrant::table(frontend, AclPermission::Read, "users"))
    .await??;
client
    .acl_grant(AclGrant::table(frontend, AclPermission::Read, "posts"))
    .await??;
```

### Auditor that reads everything

```rust
client
    .acl_grant(AclGrant::all_tables(auditor, AclPermission::Read))
    .await??;
```

### Rotate an operator

```rust
client.acl_grant(AclGrant::admin(new_operator)).await??;
client.acl_revoke(AclGrant::admin(old_operator)).await??;
```

### Check your own access

```rust
let grants = client.my_permissions().await??;
let can_insert_users = grants.iter().any(|g| {
    g.permission == AclPermission::Admin
        || (g.permission == AclPermission::Insert
            && g.table.as_deref().is_none_or(|t| t == "users"))
});
```

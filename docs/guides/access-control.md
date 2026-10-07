# Access Control (IC)

> **Note:** This is the IC-specific access control guide. Access control is an
> IC-only feature; the wasm-dbms engine has none and leaves it to the embedder.

ic-dbms keeps an Access Control List (ACL) of principals inside the database
itself, in a reserved table called `ic_dbms_acl` that `#[derive(DbmsCanister)]`
always registers next to your tables. A principal listed in that table holds
the `Admin` permission and may call every endpoint. A principal that is not
listed may call only `my_permissions`.

## Permissions

```rust
pub enum Permission {
    /// Full access: every table operation, transactions, ACL management and
    /// schema migrations.
    Admin,
}
```

`Admin` is the only permission today. The model is prepared for per-table
grants, such as "read tables `users` and `posts`, insert into `users`": they
will arrive as new `Permission` variants and, on the storage side, as new
columns or a child table of `ic_dbms_acl` added through the engine's own
migration machinery. Every endpoint already takes or returns `Permission`
values, so adding variants does not change their signatures.

## Initialization

```rust
use ic_dbms_api::prelude::IcDbmsCanisterInitArgs;

let args = IcDbmsCanisterInitArgs {
    allowed_principals: Some(vec![operator_principal]),
};
```

Bootstrap rules:

| `allowed_principals` | Result                                      |
| -------------------- | ------------------------------------------- |
| `None`               | The deployer principal becomes an admin.    |
| `Some(vec![])`       | Same as `None`: the deployer becomes admin. |
| `Some(vec![p, q])`   | Each listed principal becomes an admin.     |

`init` registers your tables, then the `ic_dbms_acl` table, then grants
`Admin` to each principal. The anonymous principal is rejected and makes
`init` trap.

## Endpoints

| Endpoint         | Kind   | Required permission | Effect                                         |
| ---------------- | ------ | ------------------- | ---------------------------------------------- |
| `acl_grant`      | update | `Admin`             | Grant a permission to a principal. Idempotent. |
| `acl_revoke`     | update | `Admin`             | Revoke a permission. Refuses the last admin.   |
| `acl_list`       | query  | `Admin`             | List every principal with its permissions.     |
| `my_permissions` | query  | none                | Return the caller's own permissions.           |

```candid
type Permission = variant { Admin };
type AclEntry = record { principal : principal; permissions : vec Permission };

acl_grant      : (principal, Permission) -> (variant { Ok; Err : IcDbmsError });
acl_revoke     : (principal, Permission) -> (variant { Ok; Err : IcDbmsError });
acl_list       : () -> (variant { Ok : vec AclEntry; Err : IcDbmsError }) query;
my_permissions : () -> (variant { Ok : vec Permission; Err : IcDbmsError }) query;
```

### Enforcement

Every generated endpoint except `my_permissions` checks the caller before
touching data:

| Endpoint kind                                                           | Who may call                              |
| ----------------------------------------------------------------------- | ----------------------------------------- |
| `select_*`, `aggregate_*`, `select`, `insert_*`, `update_*`, `delete_*` | `Admin`                                   |
| `begin_transaction`                                                     | `Admin`                                   |
| `commit`, `rollback`                                                    | The principal that opened the transaction |
| `acl_grant`, `acl_revoke`, `acl_list`                                   | `Admin`                                   |
| `has_drift`, `pending_migrations`, `migrate`                            | `Admin` or a canister controller          |

The check reads the `ic_dbms_acl` table through the same database view as
the operation, so it costs one primary-key lookup per call.

Transaction ownership is tracked in the canister heap, not in the engine:
`begin_transaction` records the caller next to the new id, every call that
names the id compares the caller with that record, and commit or rollback
removes it. A mismatch and an unknown id trap with the same message.

The untyped `select` endpoint can read `ic_dbms_acl` like any other table;
because it requires `Admin`, only admins can list admins that way.

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

## Last-admin guard

`acl_revoke(principal, Admin)` refuses the operation when `principal` is the
only admin left:

```text
IcDbmsError::Acl(AclError::LastAdmin)
```

## Errors

| Error                                                   | Cause                                                   |
| ------------------------------------------------------- | ------------------------------------------------------- |
| `IcDbmsError::Acl(AclError::AccessDenied { required })` | The caller lacks `required` (today always `Admin`).     |
| `IcDbmsError::Acl(AclError::AnonymousPrincipal)`        | `acl_grant` or `init` targeted the anonymous principal. |
| `IcDbmsError::Acl(AclError::LastAdmin)`                 | `acl_revoke` would remove the last admin.               |

See the [errors reference](../reference/errors.md) for the full `IcDbmsError`
type.

## Recipes

### Add an operator

```rust
use ic_dbms_api::prelude::Permission;

client.acl_grant(operator, Permission::Admin).await??;
```

### Rotate an operator

```rust
client.acl_grant(new_operator, Permission::Admin).await??;
client.acl_revoke(old_operator, Permission::Admin).await??;
```

### Check your own access

```rust
let perms = client.my_permissions().await??;
if perms.contains(&Permission::Admin) {
    // full access
}
```

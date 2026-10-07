//! Access control list stored in the reserved `ic_dbms_acl` table.
//!
//! The table has one row per principal; a row means the principal holds
//! [`Permission::Admin`]. `#[derive(DbmsCanister)]` registers the table at
//! `init` and composes it with the user schema through
//! [`CanisterSchema`](crate::schema::CanisterSchema), so the engine's drift
//! detection and migrations treat it like any other table.
//!
//! Every function takes the [`WasmDbmsDatabase`] of the current call so the
//! read goes through the same composed schema (and the same drift check) as
//! the operation it guards. No endpoint can stage writes to this table
//! inside a transaction, so reading it through a transactional database is
//! equivalent to reading committed state.

use candid::CandidType;
use ic_dbms_api::prelude::{
    AclEntry, AclError, Database as _, DeleteBehavior, Filter, IcDbmsError, IcDbmsResult,
    Permission, Principal, Query,
};
use serde::Deserialize;
use wasm_dbms::prelude::{DbmsContext, WasmDbmsDatabase};
use wasm_dbms_macros::{DatabaseSchema, Table};

use crate::memory::IcMemoryProvider;

/// Table names starting with this prefix are reserved for ic-dbms.
pub const RESERVED_TABLE_PREFIX: &str = "ic_dbms_";

/// A principal holding [`Permission::Admin`].
///
/// Stored in the reserved `ic_dbms_acl` table.
#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[candid]
#[table = "ic_dbms_acl"]
pub struct AclPrincipal {
    /// The principal that holds the permission.
    #[primary_key]
    #[custom_type]
    pub principal: Principal,
}

/// Schema dispatch for the reserved ACL table alone.
#[derive(DatabaseSchema)]
#[tables(AclPrincipal = "ic_dbms_acl")]
pub struct AclSchema;

type Db<'a> = WasmDbmsDatabase<'a, IcMemoryProvider>;

/// Registers the reserved ACL table in `ctx`. Idempotent.
pub fn register(ctx: &DbmsContext<IcMemoryProvider>) -> IcDbmsResult<()> {
    AclSchema::register_tables(ctx).map_err(IcDbmsError::from)
}

/// Returns whether `principal` holds [`Permission::Admin`].
pub fn is_admin(db: &Db<'_>, principal: candid::Principal) -> IcDbmsResult<bool> {
    let rows = db.select::<AclPrincipal>(by_principal(principal))?;
    Ok(!rows.is_empty())
}

/// Returns [`AclError::AccessDenied`] unless `caller` holds
/// [`Permission::Admin`].
pub fn require_admin(db: &Db<'_>, caller: candid::Principal) -> IcDbmsResult<()> {
    if is_admin(db, caller)? {
        Ok(())
    } else {
        Err(AclError::AccessDenied {
            required: Permission::Admin,
        }
        .into())
    }
}

/// Grants `permission` to `target`. Granting an already held permission is a
/// no-op. The anonymous principal is rejected.
pub fn grant(db: &Db<'_>, target: candid::Principal, permission: Permission) -> IcDbmsResult<()> {
    if target == candid::Principal::anonymous() {
        return Err(AclError::AnonymousPrincipal.into());
    }
    match permission {
        Permission::Admin => {
            if is_admin(db, target)? {
                return Ok(());
            }
            db.insert::<AclPrincipal>(AclPrincipalInsertRequest {
                principal: Principal(target),
            })?;
            Ok(())
        }
    }
}

/// Revokes `permission` from `target`. Revoking a permission that is not
/// held is a no-op. Refuses to remove the last admin.
pub fn revoke(db: &Db<'_>, target: candid::Principal, permission: Permission) -> IcDbmsResult<()> {
    match permission {
        Permission::Admin => {
            if !is_admin(db, target)? {
                return Ok(());
            }
            if admin_count(db)? == 1 {
                return Err(AclError::LastAdmin.into());
            }
            db.delete::<AclPrincipal>(
                DeleteBehavior::Restrict,
                Some(Filter::eq("principal", Principal(target).into())),
            )?;
            Ok(())
        }
    }
}

/// Lists every principal with its permissions.
pub fn list(db: &Db<'_>) -> IcDbmsResult<Vec<AclEntry>> {
    let rows = db.select::<AclPrincipal>(Query::builder().all().build())?;
    Ok(rows
        .into_iter()
        .filter_map(|row| row.principal)
        .map(|principal| AclEntry {
            principal: principal.0,
            permissions: vec![Permission::Admin],
        })
        .collect())
}

/// Returns the permissions held by `principal` (empty when unlisted).
pub fn permissions_of(db: &Db<'_>, principal: candid::Principal) -> IcDbmsResult<Vec<Permission>> {
    if is_admin(db, principal)? {
        Ok(vec![Permission::Admin])
    } else {
        Ok(Vec::new())
    }
}

fn admin_count(db: &Db<'_>) -> IcDbmsResult<usize> {
    Ok(db
        .select::<AclPrincipal>(Query::builder().all().build())?
        .len())
}

fn by_principal(principal: candid::Principal) -> Query {
    Query::builder()
        .all()
        .and_where(Filter::eq("principal", Principal(principal).into()))
        .build()
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::memory::DBMS_CONTEXT;

    fn alice() -> candid::Principal {
        crate::utils::caller()
    }

    fn bob() -> candid::Principal {
        candid::Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap()
    }

    /// Runs `f` against a database over a fresh context that holds only the
    /// ACL table, so the compiled schema matches the registry.
    fn with_acl_db<R>(f: impl FnOnce(&Db<'_>) -> R) -> R {
        DBMS_CONTEXT.with(|ctx| {
            register(ctx).expect("register acl table");
            let db = WasmDbmsDatabase::oneshot(ctx, AclSchema);
            f(&db)
        })
    }

    #[test]
    fn test_should_register_table_twice() {
        DBMS_CONTEXT.with(|ctx| {
            register(ctx).expect("first registration");
            register(ctx).expect("second registration is idempotent");
            assert!(ctx.has_table("ic_dbms_acl"));
        });
    }

    #[test]
    fn test_should_grant_admin() {
        with_acl_db(|db| {
            assert!(!is_admin(db, alice()).unwrap());
            grant(db, alice(), Permission::Admin).unwrap();
            assert!(is_admin(db, alice()).unwrap());
            assert_eq!(
                permissions_of(db, alice()).unwrap(),
                vec![Permission::Admin]
            );
        });
    }

    #[test]
    fn test_should_grant_idempotently() {
        with_acl_db(|db| {
            grant(db, alice(), Permission::Admin).unwrap();
            grant(db, alice(), Permission::Admin).unwrap();
            assert_eq!(list(db).unwrap().len(), 1);
        });
    }

    #[test]
    fn test_should_reject_anonymous_principal() {
        with_acl_db(|db| {
            let err = grant(db, candid::Principal::anonymous(), Permission::Admin)
                .expect_err("anonymous must be rejected");
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::AnonymousPrincipal)
            ));
            assert!(list(db).unwrap().is_empty());
        });
    }

    #[test]
    fn test_should_require_admin() {
        with_acl_db(|db| {
            let err = require_admin(db, bob()).expect_err("bob is not admin");
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::AccessDenied {
                    required: Permission::Admin
                })
            ));
            grant(db, bob(), Permission::Admin).unwrap();
            require_admin(db, bob()).expect("bob is admin now");
        });
    }

    #[test]
    fn test_should_revoke_admin_when_another_remains() {
        with_acl_db(|db| {
            grant(db, alice(), Permission::Admin).unwrap();
            grant(db, bob(), Permission::Admin).unwrap();
            revoke(db, bob(), Permission::Admin).unwrap();
            assert!(!is_admin(db, bob()).unwrap());
            assert!(is_admin(db, alice()).unwrap());
            assert!(permissions_of(db, bob()).unwrap().is_empty());
        });
    }

    #[test]
    fn test_should_refuse_to_revoke_last_admin() {
        with_acl_db(|db| {
            grant(db, alice(), Permission::Admin).unwrap();
            let err = revoke(db, alice(), Permission::Admin).expect_err("last admin");
            assert!(matches!(err, IcDbmsError::Acl(AclError::LastAdmin)));
            assert!(is_admin(db, alice()).unwrap());
        });
    }

    #[test]
    fn test_should_ignore_revoke_of_unlisted_principal() {
        with_acl_db(|db| {
            grant(db, alice(), Permission::Admin).unwrap();
            revoke(db, bob(), Permission::Admin).expect("no-op");
            assert_eq!(list(db).unwrap().len(), 1);
        });
    }

    #[test]
    fn test_should_list_entries() {
        with_acl_db(|db| {
            grant(db, alice(), Permission::Admin).unwrap();
            grant(db, bob(), Permission::Admin).unwrap();
            let entries = list(db).unwrap();
            assert_eq!(entries.len(), 2);
            assert!(
                entries
                    .iter()
                    .all(|e| e.permissions == vec![Permission::Admin])
            );
            assert!(entries.iter().any(|e| e.principal == alice()));
            assert!(entries.iter().any(|e| e.principal == bob()));
        });
    }
}

//! Access control list stored in the reserved `ic_dbms_acl` table.
//!
//! The table holds one row per grant: a principal, an [`AclPermission`]
//! and an optional table name, where null means every table.
//! `#[derive(DbmsCanister)]` registers the table at `init` and composes it
//! with the user schema through
//! [`CanisterSchema`](crate::schema::CanisterSchema), so the engine's schema
//! drift handling treats it like any other table.
//!
//! Every function takes the [`WasmDbmsDatabase`] of the current call so the
//! read goes through the same composed schema (and the same drift check) as
//! the operation it guards. No endpoint can stage writes to this table
//! inside a transaction, so reading it through a transactional database is
//! equivalent to reading committed state.

use ic_dbms_api::prelude::{
    AclError, AclGrant, AclPermission, AclRequirement, Autoincrement, Database as _,
    DeleteBehavior, Filter, IcDbmsError, IcDbmsResult, Nullable, Principal, Query, Text, Uint64,
};
use wasm_dbms::prelude::{DbmsContext, WasmDbmsDatabase};
use wasm_dbms_macros::{DatabaseSchema, Table};

use crate::memory::IcMemoryProvider;

/// Table names starting with this prefix are reserved for ic-dbms.
pub const RESERVED_TABLE_PREFIX: &str = "ic_dbms_";

/// One grant, stored in the reserved `ic_dbms_acl` table.
///
/// The table is never exposed through typed endpoints, so it has no Candid
/// derives (`#[autoincrement]` keys do not implement Candid in wasm-dbms).
#[derive(Debug, Table, Clone, PartialEq, Eq)]
#[table = "ic_dbms_acl"]
pub struct AclGrantRow {
    /// Row identifier.
    #[primary_key]
    #[autoincrement]
    pub id: Uint64,
    /// The principal that receives the grant.
    #[custom_type]
    #[index]
    pub principal: Principal,
    /// The operation allowed.
    #[custom_type]
    pub permission: AclPermission,
    /// The table the grant applies to; null means every table.
    pub table: Nullable<Text>,
}

/// Schema dispatch for the reserved ACL table alone.
#[derive(DatabaseSchema)]
#[tables(AclGrantRow = "ic_dbms_acl")]
pub struct AclSchema;

type Db<'a> = WasmDbmsDatabase<'a, IcMemoryProvider>;

/// Registers the reserved ACL table in `ctx`. Idempotent.
pub fn register(ctx: &DbmsContext<IcMemoryProvider>) -> IcDbmsResult<()> {
    AclSchema::register_tables(ctx).map_err(IcDbmsError::from)
}

/// Returns every grant held by `principal`, in insertion order (empty when
/// unlisted).
pub fn grants_of(db: &Db<'_>, principal: candid::Principal) -> IcDbmsResult<Vec<AclGrant>> {
    select_grants(
        db,
        Some(Filter::eq("principal", Principal(principal).into())),
    )
}

/// Lists every grant, in insertion order.
pub fn list(db: &Db<'_>) -> IcDbmsResult<Vec<AclGrant>> {
    select_grants(db, None)
}

/// Returns whether `principal` holds [`AclPermission::Admin`].
pub fn is_admin(db: &Db<'_>, principal: candid::Principal) -> IcDbmsResult<bool> {
    Ok(grants_of(db, principal)?
        .iter()
        .any(|grant| grant.permission == AclPermission::Admin))
}

/// Returns [`AclError::AccessDenied`] unless `caller` holds
/// [`AclPermission::Admin`].
pub fn require_admin(db: &Db<'_>, caller: candid::Principal) -> IcDbmsResult<()> {
    if is_admin(db, caller)? {
        Ok(())
    } else {
        Err(AclError::AccessDenied {
            required: AclPermission::Admin.into(),
            table: None,
        }
        .into())
    }
}

/// Stores `grant`; storing a grant that already exists is a no-op.
///
/// Refuses the anonymous principal, an [`AclPermission::Admin`] grant that
/// names a table, a reserved table, and any table for which
/// `is_user_table` returns `false`.
pub fn grant(
    db: &Db<'_>,
    grant: &AclGrant,
    is_user_table: impl Fn(&str) -> bool,
) -> IcDbmsResult<()> {
    if grant.principal == candid::Principal::anonymous() {
        return Err(AclError::AnonymousPrincipal.into());
    }
    if let Some(table) = &grant.table {
        if grant.permission == AclPermission::Admin {
            return Err(AclError::AdminGrantWithTable.into());
        }
        if table.starts_with(RESERVED_TABLE_PREFIX) || !is_user_table(table) {
            return Err(AclError::InvalidTable(table.clone()).into());
        }
    }
    if find_row_id(db, grant)?.is_some() {
        return Ok(());
    }
    db.insert::<AclGrantRow>(AclGrantRowInsertRequest {
        id: Autoincrement::Auto,
        principal: Principal(grant.principal),
        permission: grant.permission,
        table: match &grant.table {
            Some(table) => Nullable::Value(Text::from(table.as_str())),
            None => Nullable::Null,
        },
    })?;
    Ok(())
}

/// Removes `grant`; removing a grant that is not stored is a no-op.
///
/// Refuses to remove the last [`AclPermission::Admin`] grant.
pub fn revoke(db: &Db<'_>, grant: &AclGrant) -> IcDbmsResult<()> {
    let Some(id) = find_row_id(db, grant)? else {
        return Ok(());
    };
    if grant.permission == AclPermission::Admin && admin_count(db)? == 1 {
        return Err(AclError::LastAdmin.into());
    }
    db.delete::<AclGrantRow>(DeleteBehavior::Restrict, Some(Filter::eq("id", id.into())))?;
    Ok(())
}

fn admin_count(db: &Db<'_>) -> IcDbmsResult<usize> {
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("permission", AclPermission::Admin.into()))
        .build();
    Ok(db.select::<AclGrantRow>(query)?.len())
}

/// Returns the id of the row storing exactly `grant`.
fn find_row_id(db: &Db<'_>, grant: &AclGrant) -> IcDbmsResult<Option<Uint64>> {
    let table_filter = match &grant.table {
        Some(table) => Filter::eq("table", Text::from(table.as_str()).into()),
        None => Filter::is_null("table"),
    };
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("principal", Principal(grant.principal).into()))
        .and_where(Filter::eq("permission", grant.permission.into()))
        .and_where(table_filter)
        .build();
    Ok(db
        .select::<AclGrantRow>(query)?
        .into_iter()
        .find_map(|row| row.id))
}

fn select_grants(db: &Db<'_>, filter: Option<Filter>) -> IcDbmsResult<Vec<AclGrant>> {
    let mut query = Query::builder().all();
    if let Some(filter) = filter {
        query = query.and_where(filter);
    }
    Ok(db
        .select::<AclGrantRow>(query.build())?
        .into_iter()
        .filter_map(into_grant)
        .collect())
}

fn into_grant(row: AclGrantRowRecord) -> Option<AclGrant> {
    Some(AclGrant {
        principal: row.principal?.0,
        permission: row.permission?,
        table: Option::<Text>::from(row.table?).map(|table| table.0),
    })
}

/// Returns whether `grants` allow `permission` on `table`.
///
/// [`AclPermission::Admin`] allows everything. Any other grant matches its
/// own permission on its own table, or on every table when its table is
/// null. Reserved tables are reachable only through `Admin`.
pub fn grants_allow(grants: &[AclGrant], permission: AclPermission, table: &str) -> bool {
    grants.iter().any(|grant| {
        grant.permission == AclPermission::Admin
            || (!table.starts_with(RESERVED_TABLE_PREFIX)
                && grant.permission == permission
                && grant
                    .table
                    .as_deref()
                    .is_none_or(|granted| granted == table))
    })
}

/// Returns [`AclError::AccessDenied`] for the first table in `tables` on
/// which `caller` lacks `permission`. A reserved table reports
/// [`AclPermission::Admin`] as the missing permission.
pub fn require(
    db: &Db<'_>,
    caller: candid::Principal,
    permission: AclPermission,
    tables: &[&str],
) -> IcDbmsResult<()> {
    let grants = grants_of(db, caller)?;
    match tables
        .iter()
        .copied()
        .find(|table| !grants_allow(&grants, permission, table))
    {
        None => Ok(()),
        Some(table) => Err(AclError::AccessDenied {
            required: if table.starts_with(RESERVED_TABLE_PREFIX) {
                AclPermission::Admin
            } else {
                permission
            }
            .into(),
            table: Some(table.to_string()),
        }
        .into()),
    }
}

/// Returns [`AclError::AccessDenied`] when `caller` holds no grant at all.
pub fn require_any_grant(db: &Db<'_>, caller: candid::Principal) -> IcDbmsResult<()> {
    if grants_of(db, caller)?.is_empty() {
        Err(AclError::AccessDenied {
            required: AclRequirement::AnyGrant,
            table: None,
        }
        .into())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use ic_dbms_api::prelude::AclPermission;

    use super::*;
    use crate::memory::DBMS_CONTEXT;

    fn alice() -> candid::Principal {
        crate::utils::caller()
    }

    fn bob() -> candid::Principal {
        candid::Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap()
    }

    /// Only `users` and `posts` are user tables in these tests.
    fn is_user_table(table: &str) -> bool {
        matches!(table, "users" | "posts")
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
    fn test_should_store_and_list_grants() {
        with_acl_db(|db| {
            let admin = AclGrant::admin(alice());
            let read_users = AclGrant::table(bob(), AclPermission::Read, "users");
            let insert_all = AclGrant::all_tables(bob(), AclPermission::Insert);
            for g in [&admin, &read_users, &insert_all] {
                grant(db, g, is_user_table).expect("grant");
            }
            assert_eq!(
                list(db).unwrap(),
                vec![admin.clone(), read_users.clone(), insert_all.clone()]
            );
            assert_eq!(grants_of(db, bob()).unwrap(), vec![read_users, insert_all]);
            assert_eq!(grants_of(db, alice()).unwrap(), vec![admin]);
            assert!(is_admin(db, alice()).unwrap());
            assert!(!is_admin(db, bob()).unwrap());
        });
    }

    #[test]
    fn test_should_not_store_duplicate_grants() {
        with_acl_db(|db| {
            let read_users = AclGrant::table(bob(), AclPermission::Read, "users");
            let read_all = AclGrant::all_tables(bob(), AclPermission::Read);
            for g in [&read_users, &read_users, &read_all, &read_all] {
                grant(db, g, is_user_table).expect("grant");
            }
            assert_eq!(list(db).unwrap(), vec![read_users, read_all]);
        });
    }

    #[test]
    fn test_should_reject_invalid_grants() {
        with_acl_db(|db| {
            let cases = [
                (
                    AclGrant::admin(candid::Principal::anonymous()),
                    AclError::AnonymousPrincipal,
                ),
                (
                    AclGrant::table(candid::Principal::anonymous(), AclPermission::Read, "users"),
                    AclError::AnonymousPrincipal,
                ),
                (
                    AclGrant::table(bob(), AclPermission::Admin, "users"),
                    AclError::AdminGrantWithTable,
                ),
                (
                    AclGrant::table(bob(), AclPermission::Read, "nope"),
                    AclError::InvalidTable("nope".to_string()),
                ),
            ];
            for (g, expected) in cases {
                let err = grant(db, &g, is_user_table).expect_err("invalid grant");
                assert!(
                    matches!(err, IcDbmsError::Acl(ref e) if *e == expected),
                    "{g:?}: {err:?}"
                );
            }
            // A reserved table is refused even when the predicate accepts it.
            let reserved = AclGrant::table(bob(), AclPermission::Read, "ic_dbms_acl");
            let err = grant(db, &reserved, |_| true).expect_err("reserved table");
            assert!(list(db).unwrap().is_empty());
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::InvalidTable(ref t)) if t == "ic_dbms_acl"
            ));
        });
    }

    #[test]
    fn test_should_revoke_only_the_matching_row() {
        with_acl_db(|db| {
            let read_users = AclGrant::table(bob(), AclPermission::Read, "users");
            let read_all = AclGrant::all_tables(bob(), AclPermission::Read);
            grant(db, &read_users, is_user_table).unwrap();
            grant(db, &read_all, is_user_table).unwrap();
            revoke(db, &read_users).expect("revoke");
            assert_eq!(grants_of(db, bob()).unwrap(), vec![read_all]);
        });
    }

    #[test]
    fn test_should_ignore_revoke_of_missing_grant() {
        with_acl_db(|db| {
            grant(db, &AclGrant::admin(alice()), is_user_table).unwrap();
            revoke(db, &AclGrant::admin(bob())).expect("no-op");
            revoke(db, &AclGrant::table(alice(), AclPermission::Read, "users")).expect("no-op");
            assert_eq!(list(db).unwrap(), vec![AclGrant::admin(alice())]);
        });
    }

    #[test]
    fn test_should_refuse_to_revoke_last_admin() {
        with_acl_db(|db| {
            grant(db, &AclGrant::admin(alice()), is_user_table).unwrap();
            let err = revoke(db, &AclGrant::admin(alice())).expect_err("last admin");
            assert!(matches!(err, IcDbmsError::Acl(AclError::LastAdmin)));
            grant(db, &AclGrant::admin(bob()), is_user_table).unwrap();
            revoke(db, &AclGrant::admin(alice())).expect("another admin remains");
            assert!(!is_admin(db, alice()).unwrap());
            assert!(is_admin(db, bob()).unwrap());
        });
    }

    #[test]
    fn test_should_revoke_non_admin_grant_of_last_admin() {
        with_acl_db(|db| {
            let read_users = AclGrant::table(alice(), AclPermission::Read, "users");
            grant(db, &AclGrant::admin(alice()), is_user_table).unwrap();
            grant(db, &read_users, is_user_table).unwrap();
            revoke(db, &read_users).expect("not an admin grant");
            assert_eq!(list(db).unwrap(), vec![AclGrant::admin(alice())]);
        });
    }

    #[test]
    fn test_should_require_admin() {
        with_acl_db(|db| {
            grant(
                db,
                &AclGrant::all_tables(bob(), AclPermission::Read),
                is_user_table,
            )
            .unwrap();
            let err = require_admin(db, bob()).expect_err("read is not admin");
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::AccessDenied {
                    required: AclRequirement::Permission(AclPermission::Admin),
                    table: None,
                })
            ));
            grant(db, &AclGrant::admin(bob()), is_user_table).unwrap();
            require_admin(db, bob()).expect("bob is admin now");
        });
    }

    #[test]
    fn test_grants_allow_matches_permission_and_table() {
        let read_users = [AclGrant::table(bob(), AclPermission::Read, "users")];
        assert!(grants_allow(&read_users, AclPermission::Read, "users"));
        assert!(!grants_allow(&read_users, AclPermission::Read, "posts"));
        assert!(!grants_allow(&read_users, AclPermission::Insert, "users"));
        assert!(!grants_allow(&[], AclPermission::Read, "users"));
    }

    #[test]
    fn test_grants_allow_null_table_covers_every_user_table() {
        let read_all = [AclGrant::all_tables(bob(), AclPermission::Read)];
        assert!(grants_allow(&read_all, AclPermission::Read, "users"));
        assert!(grants_allow(
            &read_all,
            AclPermission::Read,
            "table_added_later"
        ));
        assert!(!grants_allow(&read_all, AclPermission::Delete, "users"));
        assert!(!grants_allow(&read_all, AclPermission::Read, "ic_dbms_acl"));
    }

    #[test]
    fn test_grants_allow_admin_covers_everything() {
        let admin = [AclGrant::admin(bob())];
        for permission in [
            AclPermission::Read,
            AclPermission::Insert,
            AclPermission::Update,
            AclPermission::Delete,
        ] {
            assert!(grants_allow(&admin, permission, "users"));
            assert!(grants_allow(&admin, permission, "ic_dbms_acl"));
        }
    }

    #[test]
    fn test_require_names_the_first_denied_table() {
        with_acl_db(|db| {
            grant(
                db,
                &AclGrant::table(bob(), AclPermission::Read, "users"),
                is_user_table,
            )
            .unwrap();
            require(db, bob(), AclPermission::Read, &["users"]).expect("held");
            let err = require(
                db,
                bob(),
                AclPermission::Read,
                &["users", "posts", "messages"],
            )
            .expect_err("posts not held");
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::AccessDenied {
                    required: AclRequirement::Permission(AclPermission::Read),
                    table: Some(ref table),
                }) if table == "posts"
            ));
            let err =
                require(db, bob(), AclPermission::Read, &["ic_dbms_acl"]).expect_err("reserved");
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::AccessDenied {
                    required: AclRequirement::Permission(AclPermission::Admin),
                    table: Some(ref table),
                }) if table == "ic_dbms_acl"
            ));
        });
    }

    #[test]
    fn test_require_any_grant() {
        with_acl_db(|db| {
            let err = require_any_grant(db, bob()).expect_err("no grants");
            assert!(matches!(
                err,
                IcDbmsError::Acl(AclError::AccessDenied {
                    required: AclRequirement::AnyGrant,
                    table: None,
                })
            ));
            grant(
                db,
                &AclGrant::table(bob(), AclPermission::Read, "users"),
                is_user_table,
            )
            .unwrap();
            require_any_grant(db, bob()).expect("one grant is enough");
        });
    }
}

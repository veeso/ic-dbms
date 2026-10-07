use candid::{Encode, Principal};
use ic_dbms_api::prelude::{
    AclError, AclGrant, AclPermission, AclRequirement, IcDbmsCanisterArgs, IcDbmsCanisterInitArgs,
    IcDbmsError, IcDbmsResult, JoinColumnDef, MigrationPolicy, Query, TableSchema, Text, Uint32,
    Value,
};
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};
use pocket_ic_harness::{Canister, PocketIcTestEnv};
use pocket_ic_tests::table::{User, UserInsertRequest};
use pocket_ic_tests::{TestCanister, TestEnvExt, admin, alice, assert_access_denied, bob};

type RawRows = Vec<Vec<(JoinColumnDef, Value)>>;

fn user_record(id: u32, name: &str) -> UserInsertRequest {
    UserInsertRequest {
        id: Uint32::from(id),
        name: Text::from(name),
        email: Text::from(format!("{name}@example.com")),
    }
}

fn admin_client(env: &PocketIcTestEnv<TestCanister>) -> IcDbmsPocketIcClient<'_> {
    IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic)
}

fn bob_client(env: &PocketIcTestEnv<TestCanister>) -> IcDbmsPocketIcClient<'_> {
    IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic)
}

async fn grant(env: &PocketIcTestEnv<TestCanister>, grant: AclGrant) {
    admin_client(env)
        .acl_grant(grant)
        .await
        .expect("call")
        .expect("grant");
}

#[pocket_ic_harness::test]
async fn test_admin_can_run_crud(env: PocketIcTestEnv<TestCanister>) {
    admin_client(&env)
        .insert::<User>(User::table_name(), user_record(1, "ada"), None)
        .await
        .expect("call ok")
        .expect("insert ok");
}

#[pocket_ic_harness::test]
async fn test_grant_admin_allows_crud_and_shows_in_my_permissions(
    env: PocketIcTestEnv<TestCanister>,
) {
    grant(&env, AclGrant::admin(bob())).await;

    let bob_client = bob_client(&env);
    bob_client
        .insert::<User>(User::table_name(), user_record(2, "bob"), None)
        .await
        .expect("call")
        .expect("insert as admin");
    let grants = bob_client
        .my_permissions()
        .await
        .expect("call")
        .expect("my_permissions");
    assert_eq!(grants, vec![AclGrant::admin(bob())]);
}

#[pocket_ic_harness::test]
async fn test_revoke_admin_denies_crud(env: PocketIcTestEnv<TestCanister>) {
    grant(&env, AclGrant::admin(bob())).await;
    admin_client(&env)
        .acl_revoke(AclGrant::admin(bob()))
        .await
        .expect("call")
        .expect("revoke");

    let res = bob_client(&env)
        .select::<User>(User::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Read, Some("users"));
}

#[pocket_ic_harness::test]
async fn test_principal_can_hold_several_grants(env: PocketIcTestEnv<TestCanister>) {
    let held = [
        AclGrant::table(bob(), AclPermission::Read, "users"),
        AclGrant::table(bob(), AclPermission::Read, "posts"),
        AclGrant::table(bob(), AclPermission::Insert, "users"),
    ];
    for g in &held {
        grant(&env, g.clone()).await;
    }

    let mine = bob_client(&env)
        .my_permissions()
        .await
        .expect("call")
        .expect("my_permissions");
    assert_eq!(mine, held.to_vec());

    let all = admin_client(&env)
        .acl_list()
        .await
        .expect("call")
        .expect("list");
    for g in &held {
        assert!(all.contains(g), "{g:?} missing from acl_list");
    }
}

#[pocket_ic_harness::test]
async fn test_grant_is_idempotent(env: PocketIcTestEnv<TestCanister>) {
    let read_users = AclGrant::table(bob(), AclPermission::Read, "users");
    grant(&env, read_users.clone()).await;
    grant(&env, read_users.clone()).await;
    let entries = admin_client(&env)
        .acl_list()
        .await
        .expect("call")
        .expect("list");
    assert_eq!(entries.iter().filter(|g| **g == read_users).count(), 1);
}

#[pocket_ic_harness::test]
async fn test_admin_grant_naming_a_table_is_refused(env: PocketIcTestEnv<TestCanister>) {
    let res = admin_client(&env)
        .acl_grant(AclGrant::table(bob(), AclPermission::Admin, "users"))
        .await
        .expect("call");
    assert!(matches!(
        res,
        Err(IcDbmsError::Acl(AclError::AdminGrantWithTable))
    ));
}

#[pocket_ic_harness::test]
async fn test_grant_on_unknown_or_reserved_table_is_refused(env: PocketIcTestEnv<TestCanister>) {
    for table in ["nonexistent", "ic_dbms_acl", ""] {
        let res = admin_client(&env)
            .acl_grant(AclGrant::table(bob(), AclPermission::Read, table))
            .await
            .expect("call");
        assert!(
            matches!(&res, Err(IcDbmsError::Acl(AclError::InvalidTable(t))) if t == table),
            "{table:?}: {res:?}"
        );
    }
    let mine = bob_client(&env)
        .my_permissions()
        .await
        .expect("call")
        .expect("own");
    assert!(mine.is_empty());
}

#[pocket_ic_harness::test]
async fn test_anonymous_cannot_be_granted(env: PocketIcTestEnv<TestCanister>) {
    for g in [
        AclGrant::admin(Principal::anonymous()),
        AclGrant::table(Principal::anonymous(), AclPermission::Read, "users"),
    ] {
        let res = admin_client(&env).acl_grant(g).await.expect("call");
        assert!(matches!(
            res,
            Err(IcDbmsError::Acl(AclError::AnonymousPrincipal))
        ));
    }
}

#[pocket_ic_harness::test]
async fn test_revoking_a_grant_not_held_is_a_noop(env: PocketIcTestEnv<TestCanister>) {
    let read_users = AclGrant::table(bob(), AclPermission::Read, "users");
    grant(&env, read_users.clone()).await;
    for missing in [
        AclGrant::table(bob(), AclPermission::Insert, "users"),
        AclGrant::table(bob(), AclPermission::Read, "posts"),
        AclGrant::all_tables(bob(), AclPermission::Read),
        AclGrant::admin(alice()),
    ] {
        admin_client(&env)
            .acl_revoke(missing)
            .await
            .expect("call")
            .expect("no-op revoke");
    }
    let mine = bob_client(&env)
        .my_permissions()
        .await
        .expect("call")
        .expect("own");
    assert_eq!(mine, vec![read_users]);
}

#[pocket_ic_harness::test]
async fn test_revoke_removes_only_the_named_grant(env: PocketIcTestEnv<TestCanister>) {
    let read_users = AclGrant::table(bob(), AclPermission::Read, "users");
    let read_posts = AclGrant::table(bob(), AclPermission::Read, "posts");
    grant(&env, read_users.clone()).await;
    grant(&env, read_posts.clone()).await;
    admin_client(&env)
        .acl_revoke(read_users)
        .await
        .expect("call")
        .expect("revoke");
    let mine = bob_client(&env)
        .my_permissions()
        .await
        .expect("call")
        .expect("own");
    assert_eq!(mine, vec![read_posts]);
}

#[pocket_ic_harness::test]
async fn test_cannot_revoke_last_admin(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = admin_client(&env);
    admin_client
        .acl_revoke(AclGrant::admin(env.dbms_canister_client_integration()))
        .await
        .expect("call")
        .expect("revoke wrapper");
    let res = admin_client
        .acl_revoke(AclGrant::admin(admin()))
        .await
        .expect("call");
    assert!(matches!(res, Err(IcDbmsError::Acl(AclError::LastAdmin))));
    admin_client
        .acl_list()
        .await
        .expect("call")
        .expect("list still allowed");
}

#[pocket_ic_harness::test]
async fn test_acl_list_contains_bootstrap_admins(env: PocketIcTestEnv<TestCanister>) {
    let entries = admin_client(&env)
        .acl_list()
        .await
        .expect("call")
        .expect("list");
    assert!(entries.contains(&AclGrant::admin(admin())));
    assert!(entries.contains(&AclGrant::admin(env.dbms_canister_client_integration())));
}

#[pocket_ic_harness::test]
async fn test_non_admin_cannot_manage_acl_or_see_permissions(env: PocketIcTestEnv<TestCanister>) {
    let bob_client = bob_client(&env);
    let res = bob_client
        .acl_grant(AclGrant::admin(alice()))
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let res = bob_client
        .acl_revoke(AclGrant::admin(admin()))
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let res = bob_client.acl_list().await.expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let mine = bob_client
        .my_permissions()
        .await
        .expect("call")
        .expect("own perms");
    assert!(mine.is_empty());
}

#[pocket_ic_harness::test]
async fn test_transaction_requires_at_least_one_grant(env: PocketIcTestEnv<TestCanister>) {
    let res = bob_client(&env).begin_transaction().await.expect("call");
    assert!(matches!(
        res,
        Err(IcDbmsError::Acl(AclError::AccessDenied {
            required: AclRequirement::AnyGrant,
            table: None,
        }))
    ));
}

#[pocket_ic_harness::test]
async fn test_non_admin_non_controller_cannot_migrate(env: PocketIcTestEnv<TestCanister>) {
    let bob_client = bob_client(&env);
    let res = bob_client.has_drift().await.expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let res = bob_client.pending_migrations().await.expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let res = bob_client
        .migrate(MigrationPolicy::default())
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
}

#[pocket_ic_harness::test]
async fn test_controller_without_admin_can_still_migrate(env: PocketIcTestEnv<TestCanister>) {
    // `admin()` created the canisters, so it is their controller. The
    // wrapper stays admin, so revoking `admin()`'s own grant is allowed.
    let admin_client = admin_client(&env);
    admin_client
        .acl_revoke(AclGrant::admin(admin()))
        .await
        .expect("call")
        .expect("revoke own admin");

    admin_client
        .has_drift()
        .await
        .expect("call")
        .expect("controller");
    admin_client
        .pending_migrations()
        .await
        .expect("call")
        .expect("controller");
    admin_client
        .migrate(MigrationPolicy::default())
        .await
        .expect("call")
        .expect("controller");
    let res = admin_client.acl_list().await.expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
}

#[pocket_ic_harness::test]
async fn test_reserved_acl_table_is_readable_only_by_admin(env: PocketIcTestEnv<TestCanister>) {
    let payload = Encode!(
        &"ic_dbms_acl".to_string(),
        &Query::builder().all().build(),
        &None::<u64>
    )
    .expect("encode payload");

    let res: Result<IcDbmsResult<RawRows>, _> = env
        .query(env.dbms_canister(), bob(), "select", payload.clone())
        .await;
    assert_access_denied(
        &res.expect("call"),
        AclPermission::Admin,
        Some("ic_dbms_acl"),
    );

    let res: Result<IcDbmsResult<RawRows>, _> = env
        .query(env.dbms_canister(), admin(), "select", payload)
        .await;
    let rows = res.expect("call").expect("admin reads the acl table");
    assert_eq!(rows.len(), 2);
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum EmptyTestCanister {
    DbmsCanister,
    DbmsCanisterClientIntegration,
}

impl Canister for EmptyTestCanister {
    fn as_path(&self) -> &'static std::path::Path {
        match self {
            EmptyTestCanister::DbmsCanister => std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../.artifact/example.wasm.gz"
            )),
            EmptyTestCanister::DbmsCanisterClientIntegration => std::path::Path::new(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../../.artifact/dbms_canister_client_integration.wasm.gz"
            )),
        }
    }

    fn all_canisters() -> &'static [Self] {
        &[Self::DbmsCanister, Self::DbmsCanisterClientIntegration]
    }

    fn init_arg(&self, env: &PocketIcTestEnv<Self>) -> Vec<u8> {
        match self {
            EmptyTestCanister::DbmsCanister => {
                Encode!(&IcDbmsCanisterArgs::Init(IcDbmsCanisterInitArgs {
                    allowed_principals: None,
                }))
                .expect("failed to encode dbms canister init args")
            }
            EmptyTestCanister::DbmsCanisterClientIntegration => {
                let dbms_canister = env.canister_id(&EmptyTestCanister::DbmsCanister);
                Encode!(&dbms_canister).expect("Failed to encode init arg")
            }
        }
    }
}

#[pocket_ic_harness::test]
async fn test_empty_init_bootstraps_deployer_as_admin(env: PocketIcTestEnv<EmptyTestCanister>) {
    let dbms_canister = env.canister_id(&EmptyTestCanister::DbmsCanister);
    let admin_client = IcDbmsPocketIcClient::new(dbms_canister, admin(), &env.pic);
    let entries = admin_client.acl_list().await.expect("call").expect("list");
    assert_eq!(entries, vec![AclGrant::admin(admin())]);
}

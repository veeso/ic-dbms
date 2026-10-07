use candid::{Encode, Principal};
use ic_dbms_api::prelude::{
    AclError, IcDbmsCanisterArgs, IcDbmsCanisterInitArgs, IcDbmsError, IcDbmsResult, JoinColumnDef,
    Permission, Query, TableSchema, Text, Uint32, Value,
};
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};
use pocket_ic_harness::{Canister, PocketIcTestEnv};
use pocket_ic_tests::table::{User, UserInsertRequest};
use pocket_ic_tests::{TestCanister, TestEnvExt, admin, alice, bob};

type RawRows = Vec<Vec<(JoinColumnDef, Value)>>;

fn user_record(id: u32, name: &str) -> UserInsertRequest {
    UserInsertRequest {
        id: Uint32::from(id),
        name: Text::from(name),
        email: Text::from(format!("{name}@example.com")),
    }
}

fn is_access_denied<T>(res: &IcDbmsResult<T>) -> bool {
    matches!(
        res,
        Err(IcDbmsError::Acl(AclError::AccessDenied {
            required: Permission::Admin
        }))
    )
}

#[pocket_ic_harness::test]
async fn test_admin_can_run_crud(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    client
        .insert::<User>(User::table_name(), user_record(1, "ada"), None)
        .await
        .expect("call ok")
        .expect("insert ok");
}

#[pocket_ic_harness::test]
async fn test_grant_admin_allows_crud_and_shows_in_my_permissions(
    env: PocketIcTestEnv<TestCanister>,
) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .acl_grant(bob(), Permission::Admin)
        .await
        .expect("call")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    bob_client
        .insert::<User>(User::table_name(), user_record(2, "bob"), None)
        .await
        .expect("call")
        .expect("insert as admin");
    let perms = bob_client
        .my_permissions()
        .await
        .expect("call")
        .expect("my_permissions");
    assert_eq!(perms, vec![Permission::Admin]);
}

#[pocket_ic_harness::test]
async fn test_revoke_admin_denies_crud(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .acl_grant(bob(), Permission::Admin)
        .await
        .expect("call")
        .expect("grant");
    admin_client
        .acl_revoke(bob(), Permission::Admin)
        .await
        .expect("call")
        .expect("revoke");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let res = bob_client
        .select::<User>(User::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call");
    assert!(is_access_denied(&res));
}

#[pocket_ic_harness::test]
async fn test_grant_is_idempotent(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    for _ in 0..2 {
        admin_client
            .acl_grant(bob(), Permission::Admin)
            .await
            .expect("call")
            .expect("grant");
    }
    let entries = admin_client.acl_list().await.expect("call").expect("list");
    assert_eq!(entries.iter().filter(|e| e.principal == bob()).count(), 1);
}

#[pocket_ic_harness::test]
async fn test_cannot_revoke_last_admin(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .acl_revoke(env.dbms_canister_client_integration(), Permission::Admin)
        .await
        .expect("call")
        .expect("revoke wrapper");
    let res = admin_client
        .acl_revoke(admin(), Permission::Admin)
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
async fn test_anonymous_cannot_be_granted(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    let res = admin_client
        .acl_grant(Principal::anonymous(), Permission::Admin)
        .await
        .expect("call");
    assert!(matches!(
        res,
        Err(IcDbmsError::Acl(AclError::AnonymousPrincipal))
    ));
}

#[pocket_ic_harness::test]
async fn test_acl_list_contains_bootstrap_admins(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    let entries = admin_client.acl_list().await.expect("call").expect("list");
    assert!(
        entries
            .iter()
            .any(|e| e.principal == admin() && e.permissions == vec![Permission::Admin])
    );
    assert!(
        entries
            .iter()
            .any(|e| e.principal == env.dbms_canister_client_integration())
    );
}

#[pocket_ic_harness::test]
async fn test_non_admin_cannot_manage_acl_or_see_permissions(env: PocketIcTestEnv<TestCanister>) {
    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let res = bob_client
        .acl_grant(alice(), Permission::Admin)
        .await
        .expect("call");
    assert!(is_access_denied(&res));
    let res = bob_client
        .acl_revoke(admin(), Permission::Admin)
        .await
        .expect("call");
    assert!(is_access_denied(&res));
    let res = bob_client.acl_list().await.expect("call");
    assert!(is_access_denied(&res));
    let perms = bob_client
        .my_permissions()
        .await
        .expect("call")
        .expect("own perms");
    assert!(perms.is_empty());
}

#[pocket_ic_harness::test]
async fn test_non_admin_cannot_begin_transaction(env: PocketIcTestEnv<TestCanister>) {
    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let res = bob_client.begin_transaction().await.expect("call");
    assert!(is_access_denied(&res));
}

#[pocket_ic_harness::test]
async fn test_non_admin_non_controller_cannot_migrate(env: PocketIcTestEnv<TestCanister>) {
    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let res = bob_client.has_drift().await.expect("call");
    assert!(is_access_denied(&res));
    let res = bob_client.pending_migrations().await.expect("call");
    assert!(is_access_denied(&res));
    let res = bob_client
        .migrate(ic_dbms_api::prelude::MigrationPolicy::default())
        .await
        .expect("call");
    assert!(is_access_denied(&res));
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
    assert!(is_access_denied(&res.expect("call")));

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
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].principal, admin());
    assert_eq!(entries[0].permissions, vec![Permission::Admin]);
}

use candid::Encode;
use ic_dbms_api::prelude::{
    DbmsError, DeleteBehavior, Filter, IcDbmsCanisterArgs, IcDbmsCanisterInitArgs, JoinColumnDef,
    Query, RequiredPerm, TablePerms, TableSchema, Text, Uint32, Value,
};
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};
use pocket_ic_harness::{Canister, PocketIcTestEnv};
use pocket_ic_tests::table::{Post, User, UserInsertRequest};
use pocket_ic_tests::{TestCanister, TestEnvExt, admin, bob};

fn user_record(id: u32, name: &str) -> UserInsertRequest {
    UserInsertRequest {
        id: Uint32::from(id),
        name: Text::from(name),
        email: Text::from(format!("{name}@example.com")),
    }
}

#[pocket_ic_harness::test]
async fn test_no_perms_identity_is_denied(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let res = client
        .insert::<User>(User::table_name(), user_record(1, "bob"), None)
        .await
        .expect("call ok");
    assert!(matches!(
        res,
        Err(DbmsError::AccessDenied {
            required: RequiredPerm::Table(p),
            ..
        }) if p.contains(TablePerms::INSERT)
    ));
}

#[pocket_ic_harness::test]
async fn test_admin_can_run_any_crud(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    client
        .insert::<User>(User::table_name(), user_record(1, "ada"), None)
        .await
        .expect("call ok")
        .expect("insert ok");
}

#[pocket_ic_harness::test]
async fn test_per_table_grant_isolates(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .grant_table_perms(bob(), "users", TablePerms::READ)
        .await
        .expect("call")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    // Reading users works.
    bob_client
        .select::<User>(User::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call")
        .expect("select users");
    // Reading posts denied.
    let res = bob_client
        .select::<Post>(Post::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call");
    assert!(matches!(res, Err(DbmsError::AccessDenied { .. })));
}

#[pocket_ic_harness::test]
async fn test_all_tables_grant_unions_with_per_table(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .grant_all_tables_perms(bob(), TablePerms::READ)
        .await
        .expect("call")
        .expect("grant");
    admin_client
        .grant_table_perms(bob(), "users", TablePerms::INSERT)
        .await
        .expect("call")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    bob_client
        .select::<User>(User::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call")
        .expect("select via all_tables");
    bob_client
        .insert::<User>(User::table_name(), user_record(50, "bob"), None)
        .await
        .expect("call")
        .expect("insert via per_table");
}

#[pocket_ic_harness::test]
async fn test_migration_bot_only_has_migrate(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .grant_migrate(bob())
        .await
        .expect("call")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    // CRUD denied.
    let res = bob_client
        .select::<User>(User::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call");
    assert!(matches!(res, Err(DbmsError::AccessDenied { .. })));

    // has_drift allowed.
    bob_client
        .has_drift()
        .await
        .expect("call")
        .expect("has_drift");
}

#[pocket_ic_harness::test]
async fn test_acl_manager_can_grant(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .grant_manage_acl(bob())
        .await
        .expect("call")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    bob_client
        .grant_admin(pocket_ic_tests::alice())
        .await
        .expect("call")
        .expect("grant");
}

#[pocket_ic_harness::test]
async fn test_init_with_explicit_admins_grants_full(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    let identities = admin_client
        .list_identities()
        .await
        .expect("call")
        .expect("list");
    assert!(
        identities
            .iter()
            .any(|(p, perms)| *p == admin() && perms.admin && perms.manage_acl && perms.migrate)
    );
    assert!(identities.iter().any(|(p, perms)| {
        *p == env.dbms_canister_client_integration()
            && perms.admin
            && perms.manage_acl
            && perms.migrate
    }));
}

#[pocket_ic_harness::test]
async fn test_my_perms_for_unprivileged_caller(env: PocketIcTestEnv<TestCanister>) {
    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let perms = bob_client.my_perms().await.expect("call");
    assert!(!perms.admin);
    assert!(!perms.manage_acl);
    assert!(!perms.migrate);
    assert!(perms.all_tables.is_empty());
    assert!(perms.per_table.is_empty());
}

#[pocket_ic_harness::test]
async fn test_transaction_with_no_perms_can_open_but_crud_fails(
    env: PocketIcTestEnv<TestCanister>,
) {
    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let tx = bob_client.begin_transaction().await.expect("call");
    let res = bob_client
        .insert::<User>(User::table_name(), user_record(60, "bob"), Some(tx))
        .await
        .expect("call");
    assert!(matches!(res, Err(DbmsError::AccessDenied { .. })));
    bob_client
        .commit(tx)
        .await
        .expect("call")
        .expect("commit empty tx");
}

#[pocket_ic_harness::test]
async fn test_revoke_table_perms_removes_only_specified_bits(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .grant_table_perms(bob(), "users", TablePerms::READ | TablePerms::INSERT)
        .await
        .expect("call")
        .expect("grant");
    admin_client
        .revoke_table_perms(bob(), "users", TablePerms::INSERT)
        .await
        .expect("call")
        .expect("revoke");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    bob_client
        .select::<User>(User::table_name(), Query::builder().all().build(), None)
        .await
        .expect("call")
        .expect("read still works");
    let res = bob_client
        .insert::<User>(User::table_name(), user_record(70, "bob"), None)
        .await
        .expect("call");
    assert!(matches!(res, Err(DbmsError::AccessDenied { .. })));
}

#[pocket_ic_harness::test]
async fn test_unknown_table_acl_grant_is_rejected(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    let res = admin_client
        .grant_table_perms(bob(), "missing", TablePerms::READ)
        .await
        .expect("call");
    assert!(matches!(
        res,
        Err(DbmsError::Query(ic_dbms_api::prelude::QueryError::TableNotFound(table)))
            if table == "missing"
    ));
}

#[pocket_ic_harness::test]
async fn test_join_requires_read_on_joined_table(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .insert::<User>(User::table_name(), user_record(80, "joiner"), None)
        .await
        .expect("call")
        .expect("insert user");
    admin_client
        .grant_table_perms(bob(), "users", TablePerms::READ)
        .await
        .expect("call")
        .expect("grant");

    let query = Query::builder()
        .all()
        .inner_join("posts", "users.id", "posts.user")
        .build();
    let payload =
        Encode!(&"users".to_string(), &query, &None::<u64>).expect("failed to encode payload");
    let res: Result<ic_dbms_api::prelude::IcDbmsResult<Vec<Vec<(JoinColumnDef, Value)>>>, _> = env
        .query(env.dbms_canister(), bob(), "select", payload)
        .await;
    let res = res.expect("failed to call canister");
    assert!(matches!(
        res,
        Err(DbmsError::AccessDenied {
            required: RequiredPerm::Table(perms),
            table: Some(table),
        }) if perms == TablePerms::READ && table == Post::fingerprint()
    ));
}

#[pocket_ic_harness::test]
async fn test_delete_perm_check(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    admin_client
        .grant_table_perms(bob(), "users", TablePerms::READ)
        .await
        .expect("call")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    let res = bob_client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Restrict,
            Some(Filter::eq("id", Value::Uint32(99u32.into()))),
            None,
        )
        .await
        .expect("call");
    assert!(matches!(
        res,
        Err(DbmsError::AccessDenied {
            required: RequiredPerm::Table(p),
            ..
        }) if p.contains(TablePerms::DELETE)
    ));
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
async fn test_empty_init_bootstraps_deployer_as_full_admin(
    env: PocketIcTestEnv<EmptyTestCanister>,
) {
    let dbms_canister = env.canister_id(&EmptyTestCanister::DbmsCanister);
    let admin_client = IcDbmsPocketIcClient::new(dbms_canister, admin(), &env.pic);
    let identities = admin_client
        .list_identities()
        .await
        .expect("call")
        .expect("list");
    assert!(
        identities
            .iter()
            .any(|(p, perms)| *p == admin() && perms.admin && perms.manage_acl && perms.migrate)
    );
}

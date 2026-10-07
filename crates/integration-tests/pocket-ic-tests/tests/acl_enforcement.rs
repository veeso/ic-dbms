use ic_dbms_api::prelude::{
    AclGrant, AclPermission, AggregateFunction, DeleteBehavior, Filter, IcDbmsResult,
    MigrationPolicy, Query, TableSchema, Text, Uint32, Value,
};
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};
use pocket_ic_harness::PocketIcTestEnv;
use pocket_ic_tests::table::{
    Post, PostInsertRequest, PostUpdateRequest, User, UserInsertRequest, UserUpdateRequest,
};
use pocket_ic_tests::{TestCanister, TestEnvExt as _, admin, assert_access_denied, bob};

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

fn user(id: u32) -> UserInsertRequest {
    UserInsertRequest {
        id: Uint32::from(id),
        name: Text::from(format!("user{id}")),
        email: Text::from(format!("user{id}@example.com")),
    }
}

fn post(id: u32, user: u32) -> PostInsertRequest {
    PostInsertRequest {
        id: Uint32::from(id),
        title: Text::from("title"),
        content: Text::from("content"),
        user: Uint32::from(user),
    }
}

fn by_id(id: u32) -> Filter {
    Filter::eq("id", Value::Uint32(id.into()))
}

fn rename(id: u32) -> UserUpdateRequest {
    UserUpdateRequest {
        id: None,
        name: Some(Text::from("renamed")),
        email: None,
        where_clause: Some(by_id(id)),
    }
}

fn rename_post(id: u32) -> PostUpdateRequest {
    PostUpdateRequest {
        id: None,
        title: Some(Text::from("renamed")),
        content: None,
        user: None,
        where_clause: Some(by_id(id)),
    }
}

fn all() -> Query {
    Query::builder().all().build()
}

fn count() -> Vec<AggregateFunction> {
    vec![AggregateFunction::Count(None)]
}

/// Inserts user `id` and, when `with_post`, post `id` written by that user.
async fn seed(env: &PocketIcTestEnv<TestCanister>, id: u32, with_post: bool) {
    let admin_client = admin_client(env);
    admin_client
        .insert::<User>(User::table_name(), user(id), None)
        .await
        .expect("call")
        .expect("insert user");
    if with_post {
        admin_client
            .insert::<Post>(Post::table_name(), post(id, id), None)
            .await
            .expect("call")
            .expect("insert post");
    }
}

#[pocket_ic_harness::test]
async fn test_read_grant_allows_select_and_aggregate_on_its_table_only(
    env: PocketIcTestEnv<TestCanister>,
) {
    grant(&env, AclGrant::table(bob(), AclPermission::Read, "users")).await;
    let bob_client = bob_client(&env);

    bob_client
        .select::<User>(User::table_name(), all(), None)
        .await
        .expect("call")
        .expect("select users");
    bob_client
        .aggregate::<User>(User::table_name(), all(), count(), None)
        .await
        .expect("call")
        .expect("aggregate users");

    let res = bob_client
        .select::<Post>(Post::table_name(), all(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Read, Some("posts"));
    let res = bob_client
        .aggregate::<Post>(Post::table_name(), all(), count(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Read, Some("posts"));
    let res = bob_client
        .select_raw("projects", all(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Read, Some("projects"));
}

#[pocket_ic_harness::test]
async fn test_read_only_principal_is_refused_writes_acl_management_and_migrations(
    env: PocketIcTestEnv<TestCanister>,
) {
    grant(&env, AclGrant::table(bob(), AclPermission::Read, "users")).await;
    let bob_client = bob_client(&env);

    let res = bob_client
        .insert::<User>(User::table_name(), user(10), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Insert, Some("users"));
    let res = bob_client
        .update::<User>(User::table_name(), rename(10), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Update, Some("users"));
    let res = bob_client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Restrict,
            Some(by_id(10)),
            None,
        )
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Delete, Some("users"));

    let insert_users = AclGrant::table(bob(), AclPermission::Insert, "users");
    let res = bob_client
        .acl_grant(insert_users.clone())
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let res = bob_client.acl_revoke(insert_users).await.expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);
    let res = bob_client.acl_list().await.expect("call");
    assert_access_denied(&res, AclPermission::Admin, None);

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
async fn test_each_write_grant_allows_only_its_own_operation(env: PocketIcTestEnv<TestCanister>) {
    seed(&env, 20, false).await;
    let admin_client = admin_client(&env);
    let bob_client = bob_client(&env);

    for (index, held) in [
        AclPermission::Insert,
        AclPermission::Update,
        AclPermission::Delete,
    ]
    .into_iter()
    .enumerate()
    {
        let held_grant = AclGrant::table(bob(), held, "users");
        admin_client
            .acl_grant(held_grant.clone())
            .await
            .expect("call")
            .expect("grant");

        let insert: IcDbmsResult<u64> = bob_client
            .insert::<User>(User::table_name(), user(21 + index as u32), None)
            .await
            .expect("call")
            .map(|()| 1);
        let update = bob_client
            .update::<User>(User::table_name(), rename(20), None)
            .await
            .expect("call");
        // id 999 matches no row, so an allowed delete removes nothing.
        let delete = bob_client
            .delete::<User>(
                User::table_name(),
                DeleteBehavior::Restrict,
                Some(by_id(999)),
                None,
            )
            .await
            .expect("call");

        for (permission, res) in [
            (AclPermission::Insert, insert),
            (AclPermission::Update, update),
            (AclPermission::Delete, delete),
        ] {
            if permission == held {
                res.unwrap_or_else(|err| panic!("{held} must allow {permission}: {err}"));
            } else {
                assert_access_denied(&res, permission, Some("users"));
            }
        }

        admin_client
            .acl_revoke(held_grant)
            .await
            .expect("call")
            .expect("revoke");
    }
}

#[pocket_ic_harness::test]
async fn test_all_tables_grant_applies_to_every_user_table(env: PocketIcTestEnv<TestCanister>) {
    grant(&env, AclGrant::all_tables(bob(), AclPermission::Read)).await;
    let bob_client = bob_client(&env);

    bob_client
        .select::<User>(User::table_name(), all(), None)
        .await
        .expect("call")
        .expect("users");
    bob_client
        .select::<Post>(Post::table_name(), all(), None)
        .await
        .expect("call")
        .expect("posts");
    bob_client
        .select_raw("projects", all(), None)
        .await
        .expect("call")
        .expect("projects");

    let res = bob_client
        .select_raw("ic_dbms_acl", all(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Admin, Some("ic_dbms_acl"));
    let join_acl = Query::builder()
        .all()
        .inner_join("ic_dbms_acl", "users.id", "ic_dbms_acl.id")
        .build();
    let res = bob_client
        .select_raw("users", join_acl, None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Admin, Some("ic_dbms_acl"));

    let res = bob_client
        .insert::<User>(User::table_name(), user(30), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Insert, Some("users"));
}

#[pocket_ic_harness::test]
async fn test_all_tables_insert_grant_applies_across_tables_until_revoked(
    env: PocketIcTestEnv<TestCanister>,
) {
    let insert_all = AclGrant::all_tables(bob(), AclPermission::Insert);
    grant(&env, insert_all.clone()).await;
    let bob_client = bob_client(&env);

    bob_client
        .insert::<User>(User::table_name(), user(31), None)
        .await
        .expect("call")
        .expect("insert user");
    bob_client
        .insert::<Post>(Post::table_name(), post(31, 31), None)
        .await
        .expect("call")
        .expect("insert post");

    admin_client(&env)
        .acl_revoke(insert_all)
        .await
        .expect("call")
        .expect("revoke");
    let res = bob_client
        .insert::<User>(User::table_name(), user(32), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Insert, Some("users"));
}

#[pocket_ic_harness::test]
async fn test_all_tables_update_grant_applies_across_tables_until_revoked(
    env: PocketIcTestEnv<TestCanister>,
) {
    seed(&env, 33, true).await;
    let update_all = AclGrant::all_tables(bob(), AclPermission::Update);
    grant(&env, update_all.clone()).await;
    let bob_client = bob_client(&env);

    assert_eq!(
        bob_client
            .update::<User>(User::table_name(), rename(33), None)
            .await
            .expect("call")
            .expect("update user"),
        1
    );
    assert_eq!(
        bob_client
            .update::<Post>(Post::table_name(), rename_post(33), None)
            .await
            .expect("call")
            .expect("update post"),
        1
    );

    admin_client(&env)
        .acl_revoke(update_all)
        .await
        .expect("call")
        .expect("revoke");
    let res = bob_client
        .update::<User>(User::table_name(), rename(33), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Update, Some("users"));
}

#[pocket_ic_harness::test]
async fn test_all_tables_delete_grant_applies_across_tables_until_revoked(
    env: PocketIcTestEnv<TestCanister>,
) {
    seed(&env, 34, true).await;
    seed(&env, 35, false).await;
    let delete_all = AclGrant::all_tables(bob(), AclPermission::Delete);
    grant(&env, delete_all.clone()).await;
    let bob_client = bob_client(&env);

    assert_eq!(
        bob_client
            .delete::<Post>(
                Post::table_name(),
                DeleteBehavior::Restrict,
                Some(by_id(34)),
                None,
            )
            .await
            .expect("call")
            .expect("delete post"),
        1
    );
    assert_eq!(
        bob_client
            .delete::<User>(
                User::table_name(),
                DeleteBehavior::Restrict,
                Some(by_id(35)),
                None,
            )
            .await
            .expect("call")
            .expect("delete user"),
        1
    );

    admin_client(&env)
        .acl_revoke(delete_all)
        .await
        .expect("call")
        .expect("revoke");
    let res = bob_client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Restrict,
            Some(by_id(34)),
            None,
        )
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Delete, Some("users"));
}

#[pocket_ic_harness::test]
async fn test_join_requires_read_on_every_joined_table(env: PocketIcTestEnv<TestCanister>) {
    seed(&env, 40, true).await;
    grant(&env, AclGrant::table(bob(), AclPermission::Read, "users")).await;
    let bob_client = bob_client(&env);
    let join = Query::builder()
        .all()
        .inner_join("posts", "users.id", "posts.user")
        .build();

    let res = bob_client
        .select_raw("users", join.clone(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Read, Some("posts"));

    grant(&env, AclGrant::table(bob(), AclPermission::Read, "posts")).await;
    let rows = bob_client
        .select_raw("users", join, None)
        .await
        .expect("call")
        .expect("join with both reads");
    assert!(!rows.is_empty());
}

#[pocket_ic_harness::test]
async fn test_eager_relation_requires_read_on_the_related_table(
    env: PocketIcTestEnv<TestCanister>,
) {
    seed(&env, 50, true).await;
    grant(&env, AclGrant::table(bob(), AclPermission::Read, "posts")).await;
    let bob_client = bob_client(&env);
    let eager = Query::builder().all().with("users").build();

    let res = bob_client
        .select::<Post>(Post::table_name(), eager.clone(), None)
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Read, Some("users"));

    grant(&env, AclGrant::table(bob(), AclPermission::Read, "users")).await;
    bob_client
        .select::<Post>(Post::table_name(), eager, None)
        .await
        .expect("call")
        .expect("eager load with both reads");
}

#[pocket_ic_harness::test]
async fn test_cascade_delete_requires_delete_on_referencing_tables(
    env: PocketIcTestEnv<TestCanister>,
) {
    seed(&env, 60, true).await;
    seed(&env, 61, false).await;
    grant(&env, AclGrant::table(bob(), AclPermission::Delete, "users")).await;
    let bob_client = bob_client(&env);

    let res = bob_client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Cascade,
            Some(by_id(60)),
            None,
        )
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Delete, Some("posts"));

    let deleted = bob_client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Restrict,
            Some(by_id(61)),
            None,
        )
        .await
        .expect("call")
        .expect("restrict delete needs only users");
    assert_eq!(deleted, 1);

    grant(&env, AclGrant::table(bob(), AclPermission::Delete, "posts")).await;
    let deleted = bob_client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Cascade,
            Some(by_id(60)),
            None,
        )
        .await
        .expect("call")
        .expect("cascade with both deletes");
    assert_eq!(deleted, 2, "the user and its post");
}

#[pocket_ic_harness::test]
async fn test_operations_inside_a_transaction_are_checked_per_table(
    env: PocketIcTestEnv<TestCanister>,
) {
    grant(&env, AclGrant::table(bob(), AclPermission::Read, "users")).await;
    let bob_client = bob_client(&env);

    let tx = bob_client
        .begin_transaction()
        .await
        .expect("call")
        .expect("one grant opens a transaction");
    bob_client
        .select::<User>(User::table_name(), all(), Some(tx))
        .await
        .expect("call")
        .expect("read inside the transaction");
    let res = bob_client
        .insert::<User>(User::table_name(), user(70), Some(tx))
        .await
        .expect("call");
    assert_access_denied(&res, AclPermission::Insert, Some("users"));
    bob_client
        .rollback(tx)
        .await
        .expect("call")
        .expect("owner rolls back");
}

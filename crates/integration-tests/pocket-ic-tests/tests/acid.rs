use ic_dbms_api::prelude::{Filter, Permission, Query, TableSchema, Text, Uint32, Value};
use ic_dbms_client::prelude::{
    Client as _, IcDbmCanisterClientError, IcDbmsPocketIcClient, PocketIcError,
};
use pocket_ic_harness::PocketIcTestEnv;
use pocket_ic_tests::table::{Post, PostInsertRequest, User, UserInsertRequest};
use pocket_ic_tests::{TestCanister, TestEnvExt as _, admin, bob};

fn assert_transaction_trap<T: std::fmt::Debug>(result: Result<T, IcDbmCanisterClientError>) {
    match result {
        Err(IcDbmCanisterClientError::PocketIc(PocketIcError::Reject(reject))) => {
            assert!(
                reject.reject_message.contains("does not own transaction"),
                "unexpected rejection message: {}",
                reject.reject_message
            );
        }
        other => panic!("expected transaction ownership trap, got {other:?}"),
    }
}

#[pocket_ic_harness::test]
async fn test_should_operate_on_a_transaction(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);

    let transaction_id = client
        .begin_transaction()
        .await
        .expect("failed to call canister")
        .expect("failed to begin transaction");

    // insert user
    let insert_request = UserInsertRequest {
        id: Uint32::from(5),
        name: "Frank".into(),
        email: "frankmetano@example.com".into(),
    };
    client
        .insert::<User>(User::table_name(), insert_request, Some(transaction_id))
        .await
        .expect("failed to call canister")
        .expect("failed to insert user");
    // insert a post for user
    let insert_request = PostInsertRequest {
        id: Uint32::from(1),
        user: Uint32::from(5),
        title: "Hello World".into(),
        content: "This is my first post.".into(),
    };
    client
        .insert::<Post>(Post::table_name(), insert_request, Some(transaction_id))
        .await
        .expect("failed to call canister")
        .expect("failed to insert post");

    // commit
    client
        .commit(transaction_id)
        .await
        .expect("failed to call canister")
        .expect("failed to commit transaction");

    // search
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("id", Value::Uint32(5.into())))
        .build();
    let users = client
        .select::<User>(User::table_name(), query, None)
        .await
        .expect("failed to call canister")
        .expect("failed to query user");
    assert_eq!(users.len(), 1);
    let user = &users[0];
    assert_eq!(user.id.unwrap(), Uint32::from(5));
    assert_eq!(user.name.as_ref().unwrap(), &Text::from("Frank"));

    // verify post
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("user", Value::Uint32(5.into())))
        .build();
    let posts = client
        .select::<Post>(Post::table_name(), query, None)
        .await
        .expect("failed to call canister")
        .expect("failed to query post");
    assert_eq!(posts.len(), 1);
    let post = &posts[0];
    assert_eq!(post.id.unwrap(), Uint32::from(1));
    assert_eq!(post.title.as_ref().unwrap(), &Text::from("Hello World"));
}

#[pocket_ic_harness::test]
async fn test_should_rollback_transaction(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);

    let transaction_id = client
        .begin_transaction()
        .await
        .expect("failed to call canister")
        .expect("failed to begin transaction");

    // insert user
    let insert_request = UserInsertRequest {
        id: Uint32::from(6),
        name: "Julia".into(),
        email: "julia.scoreza@example.com".into(),
    };
    client
        .insert::<User>(User::table_name(), insert_request, Some(transaction_id))
        .await
        .expect("failed to call canister")
        .expect("failed to insert user");
    // insert a post for user
    let insert_request = PostInsertRequest {
        id: Uint32::from(2),
        user: Uint32::from(6),
        title: "Hello World".into(),
        content: "I've just started blogging.".into(),
    };
    client
        .insert::<Post>(Post::table_name(), insert_request, Some(transaction_id))
        .await
        .expect("failed to call canister")
        .expect("failed to insert post");

    // rollback
    client
        .rollback(transaction_id)
        .await
        .expect("failed to call canister")
        .expect("failed to rollback transaction");

    // search
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("id", Value::Uint32(6.into())))
        .build();
    let users = client
        .select::<User>(User::table_name(), query, None)
        .await
        .expect("failed to call canister")
        .expect("failed to query user");
    assert!(users.is_empty());

    // verify post
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("user", Value::Uint32(6.into())))
        .build();
    let posts = client
        .select::<Post>(Post::table_name(), query, None)
        .await
        .expect("failed to call canister")
        .expect("failed to query post");
    assert!(posts.is_empty());
}

#[pocket_ic_harness::test]
async fn test_should_not_perform_transaction_not_owned(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);

    let transaction_id = Some(1111u64);

    // insert user
    let insert_request = UserInsertRequest {
        id: Uint32::from(7),
        name: "Kevin".into(),
        email: "kevin.scoreza@example.com".into(),
    };
    let result = client
        .insert::<User>(User::table_name(), insert_request, transaction_id)
        .await;
    assert!(result.is_err());
}

#[pocket_ic_harness::test]
async fn test_should_not_commit_transaction_of_another_principal(
    env: PocketIcTestEnv<TestCanister>,
) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    // bob is an admin too, so only ownership can stop him.
    admin_client
        .acl_grant(bob(), Permission::Admin)
        .await
        .expect("failed to call canister")
        .expect("failed to grant admin");

    let transaction_id = admin_client
        .begin_transaction()
        .await
        .expect("failed to call canister")
        .expect("failed to begin transaction");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);

    // bob cannot write inside admin's transaction: the canister traps.
    let insert_request = UserInsertRequest {
        id: Uint32::from(8),
        name: "Mallory".into(),
        email: "mallory@example.com".into(),
    };
    let result = bob_client
        .insert::<User>(User::table_name(), insert_request, Some(transaction_id))
        .await;
    assert_transaction_trap(result);

    // bob cannot commit or roll back admin's transaction either.
    assert_transaction_trap(bob_client.commit(transaction_id).await);
    assert_transaction_trap(bob_client.rollback(transaction_id).await);

    // The owner still can.
    admin_client
        .commit(transaction_id)
        .await
        .expect("failed to call canister")
        .expect("owner commits");

    // A closed id traps for everyone, including the former owner.
    assert_transaction_trap(admin_client.commit(transaction_id).await);
}

use candid::Nat;
use ic_dbms_api::prelude::{DeleteBehavior, Filter, Query, TableSchema, Text, Uint32, Value};
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};
use pocket_ic_tests::TestEnv;
use pocket_ic_tests::table::{Post, PostInsertRequest, User, UserInsertRequest, UserUpdateRequest};

#[pocket_ic_tests_macro::test]
async fn test_should_init_dbms_canister(env: PocketIcTestEnv) {}

#[pocket_ic_tests_macro::test]
async fn test_should_insert_and_query_data(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    let insert_request = UserInsertRequest {
        id: Uint32::from(1),
        name: "Alice".into(),
        email: "alice@example.com".into(),
    };
    client
        .insert::<User>(User::table_name(), insert_request, None)
        .await
        .expect("failed to call canister")
        .expect("failed to insert user");

    // query user
    let query = Query::<User>::builder()
        .all()
        .and_where(Filter::eq("id", Value::Uint32(1.into())))
        .build();
    let users = client
        .select::<User>(User::table_name(), query, None)
        .await
        .expect("failed to call canister")
        .expect("failed to query user");

    assert_eq!(users.len(), 1);
    let user = &users[0];
    assert_eq!(user.id.unwrap(), Uint32::from(1));
    assert_eq!(user.name.as_ref().unwrap(), &Text::from("Alice"));
    assert_eq!(
        user.email.as_ref().unwrap(),
        &Text::from("alice@example.com")
    );
}

#[pocket_ic_tests_macro::test]
async fn test_should_delete_a_user(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    let insert_request = UserInsertRequest {
        id: Uint32::from(2),
        name: "Bob".into(),
        email: "bob@example.com".into(),
    };
    client
        .insert::<User>(User::table_name(), insert_request, None)
        .await
        .expect("failed to call canister")
        .expect("failed to insert user");

    // delete user
    client
        .delete::<User>(
            User::table_name(),
            DeleteBehavior::Restrict,
            Some(Filter::eq("id", Value::Uint32(2.into()))),
            None,
        )
        .await
        .expect("failed to call canister")
        .expect("failed to delete user");
}

#[pocket_ic_tests_macro::test]
async fn test_should_update_a_user(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    let insert_request = UserInsertRequest {
        id: Uint32::from(3),
        name: "Charlie".into(),
        email: "charlie@example.com".into(),
    };
    client
        .insert::<User>(User::table_name(), insert_request, None)
        .await
        .expect("failed to call canister")
        .expect("failed to insert user");

    let patch = UserUpdateRequest {
        id: None,
        name: Some("Charles".into()),
        email: None,
        where_clause: Some(Filter::eq("id", Value::Uint32(3.into()))),
    };

    client
        .update::<User>(User::table_name(), patch, None)
        .await
        .expect("failed to call canister")
        .expect("failed to update user");

    // select
    let query = Query::<User>::builder()
        .all()
        .and_where(Filter::eq("id", Value::Uint32(3.into())))
        .build();
    let users = client
        .select::<User>(User::table_name(), query, None)
        .await
        .expect("failed to call canister")
        .expect("failed to query user");

    assert_eq!(users.len(), 1);
    let user = &users[0];
    assert_eq!(user.id.unwrap(), Uint32::from(3));
    assert_eq!(user.name.as_ref().unwrap(), &Text::from("Charles"));
    assert_eq!(
        user.email.as_ref().unwrap(),
        &Text::from("charlie@example.com")
    );
}

#[pocket_ic_tests_macro::test]
async fn test_should_not_allow_unauthorized_call(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.alice(), &env.pic);

    let insert_request = UserInsertRequest {
        id: Uint32::from(4),
        name: "Eve".into(),
        email: "eve@example.com".into(),
    };
    let result = client
        .insert::<User>(User::table_name(), insert_request, None)
        .await;
    assert!(result.is_err());
}

#[pocket_ic_tests_macro::test]
async fn test_should_add_and_remove_principal_to_acl(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    // add alice to ACL
    client
        .acl_add_principal(env.bob())
        .await
        .expect("failed to call canister")
        .expect("failed to add principal to ACL");

    // get
    let acl = client
        .acl_allowed_principals()
        .await
        .expect("failed to call canister");
    assert!(acl.contains(&env.bob()));
    assert!(acl.contains(&env.admin()));

    // remove alice from ACL
    client
        .acl_remove_principal(env.bob())
        .await
        .expect("failed to call canister")
        .expect("failed to remove principal from ACL");

    // get
    let acl = client
        .acl_allowed_principals()
        .await
        .expect("failed to call canister");
    assert!(!acl.contains(&env.bob()));
    assert!(acl.contains(&env.admin()));
}

#[pocket_ic_tests_macro::test]
async fn test_should_operate_on_a_transaction(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    let transaction_id = client
        .begin_transaction()
        .await
        .expect("failed to call canister");

    // insert user
    let insert_request = UserInsertRequest {
        id: Uint32::from(5),
        name: "Frank".into(),
        email: "frankmetano@example.com".into(),
    };
    client
        .insert::<User>(
            User::table_name(),
            insert_request,
            Some(transaction_id.clone()),
        )
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
        .insert::<Post>(
            Post::table_name(),
            insert_request,
            Some(transaction_id.clone()),
        )
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
    let query = Query::<User>::builder()
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
    let query = Query::<Post>::builder()
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

#[pocket_ic_tests_macro::test]
async fn test_should_rollback_transaction(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    let transaction_id = client
        .begin_transaction()
        .await
        .expect("failed to call canister");

    // insert user
    let insert_request = UserInsertRequest {
        id: Uint32::from(6),
        name: "Julia".into(),
        email: "julia.scoreza@example.com".into(),
    };
    client
        .insert::<User>(
            User::table_name(),
            insert_request,
            Some(transaction_id.clone()),
        )
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
        .insert::<Post>(
            Post::table_name(),
            insert_request,
            Some(transaction_id.clone()),
        )
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
    let query = Query::<User>::builder()
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
    let query = Query::<Post>::builder()
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

#[pocket_ic_tests_macro::test]
async fn test_should_not_perform_transaction_not_owned(env: PocketIcTestEnv) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), env.admin(), &env.pic);

    let transaction_id = Some(Nat::from(1111u64));

    // insert user
    let insert_request = UserInsertRequest {
        id: Uint32::from(7),
        name: "Kevin".into(),
        email: "kevin.scoreza@example.com".into(),
    };
    let result = client
        .insert::<User>(User::table_name(), insert_request, transaction_id.clone())
        .await;
    assert!(result.is_err());
}

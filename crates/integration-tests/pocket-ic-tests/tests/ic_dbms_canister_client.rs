use candid::Encode;
use ic_dbms_api::prelude::{
    DeleteBehavior, Filter, IcDbmsResult, JoinColumnDef, Query, SqlResult, TransactionId, Value,
};
use pocket_ic_harness::PocketIcTestEnv;
use pocket_ic_tests::table::{UserInsertRequest, UserRecord, UserUpdateRequest};
use pocket_ic_tests::{PocketIcClient, TestCanister, TestEnvExt as _, admin, bob};

type TestResult<T> = Result<IcDbmsResult<T>, String>;

#[pocket_ic_harness::test]
async fn test_should_grant_and_revoke_through_wrapper(env: PocketIcTestEnv<TestCanister>) {
    use ic_dbms_api::prelude::{AclGrant, AclPermission};

    let client = PocketIcClient::new(env.dbms_canister_client_integration(), admin(), &env.pic);
    let read_users = AclGrant::table(bob(), AclPermission::Read, "users");

    let res: Result<IcDbmsResult<()>, String> = client
        .update("acl_grant", Encode!(&read_users).expect("Failed to encode"))
        .await
        .expect("Can't update");
    res.expect("Client error").expect("Failed to grant");

    let entries: Result<IcDbmsResult<Vec<AclGrant>>, String> = client
        .update("acl_list", Encode!().expect("Failed to encode"))
        .await
        .expect("Can't query");
    let entries = entries.expect("Client error").expect("list ok");
    assert!(entries.contains(&read_users));

    let res: Result<IcDbmsResult<()>, String> = client
        .update(
            "acl_revoke",
            Encode!(&read_users).expect("Failed to encode"),
        )
        .await
        .expect("Can't update");
    res.expect("Client error").expect("Failed to revoke");

    let grants: Result<IcDbmsResult<Vec<AclGrant>>, String> = client
        .update("my_permissions", Encode!().expect("Failed to encode"))
        .await
        .expect("Can't query");
    assert_eq!(
        grants.expect("Client error").expect("my_permissions ok"),
        vec![AclGrant::admin(env.dbms_canister_client_integration())]
    );
}

#[pocket_ic_harness::test]
async fn test_should_begin_commit_transaction(env: PocketIcTestEnv<TestCanister>) {
    let client = PocketIcClient::new(env.dbms_canister_client_integration(), admin(), &env.pic);

    // Begin transaction
    let res: Result<IcDbmsResult<TransactionId>, String> = client
        .update("begin_transaction", Encode!().expect("Failed to encode"))
        .await
        .expect("Can't update");

    let transaction_id = res
        .expect("Client error")
        .expect("Failed to begin transaction");

    // Commit transaction
    let res: Result<IcDbmsResult<()>, String> = client
        .update(
            "commit",
            Encode!(&transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't update");

    res.expect("Client error")
        .expect("Failed to commit transaction");
}

#[pocket_ic_harness::test]
async fn test_should_begin_rollback_transaction(env: PocketIcTestEnv<TestCanister>) {
    let client = PocketIcClient::new(env.dbms_canister_client_integration(), admin(), &env.pic);

    // Begin transaction
    let res: Result<IcDbmsResult<TransactionId>, String> = client
        .update("begin_transaction", Encode!().expect("Failed to encode"))
        .await
        .expect("Can't update");

    let transaction_id = res
        .expect("Client error")
        .expect("Failed to begin transaction");

    // Rollback transaction
    let res: Result<IcDbmsResult<()>, String> = client
        .update(
            "rollback",
            Encode!(&transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't update");

    res.expect("Client error")
        .expect("Failed to rollback transaction");
}

#[pocket_ic_harness::test]
async fn test_should_insert_select_update_delete(env: PocketIcTestEnv<TestCanister>) {
    let client = PocketIcClient::new(env.dbms_canister_client_integration(), admin(), &env.pic);

    // Insert a record
    let insert_request = UserInsertRequest {
        id: 1u32.into(),
        name: "Alice".into(),
        email: "alice@example.com".into(),
    };
    let transaction_id: Option<TransactionId> = None;

    let res: Result<IcDbmsResult<()>, String> = client
        .update(
            "insert",
            Encode!(&insert_request, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't update");

    res.expect("Client error").expect("Failed to insert record");

    // Select the record
    let query = Query::builder()
        .all()
        .and_where(Filter::eq("id", Value::Uint32(1.into())))
        .build();

    let res: Result<IcDbmsResult<Vec<UserRecord>>, String> = client
        .update(
            "select",
            Encode!(&query, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't query");

    let records = res
        .expect("Client error")
        .expect("Failed to select records");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].id.unwrap(), 1u32.into());
    assert_eq!(records[0].name.as_ref().unwrap(), &"Alice".into());
    assert_eq!(
        records[0].email.as_ref().unwrap(),
        &"alice@example.com".into()
    );

    // select raw
    let res: TestResult<Vec<Vec<(JoinColumnDef, Value)>>> = client
        .update(
            "select_raw",
            Encode!(&query, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't query");
    assert!(res.is_ok());

    // Update the record
    let update_request = UserUpdateRequest {
        id: None,
        name: Some("Alice Updated".into()),
        email: None,
        where_clause: Some(Filter::eq("id", Value::Uint32(1.into()))),
    };

    let res: Result<IcDbmsResult<u64>, String> = client
        .update(
            "update",
            Encode!(&update_request, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't update");

    let updated_count = res.expect("Client error").expect("Failed to update record");
    assert_eq!(updated_count, 1);

    // Select again to verify update
    let res: Result<IcDbmsResult<Vec<UserRecord>>, String> = client
        .update(
            "select",
            Encode!(&query, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't query");

    let records = res
        .expect("Client error")
        .expect("Failed to select records");
    assert_eq!(records[0].name.as_ref().unwrap(), &"Alice Updated".into());

    // Delete the record
    let behaviour = DeleteBehavior::Restrict;
    let filter: Option<Filter> = None;

    let res: Result<IcDbmsResult<u64>, String> = client
        .update(
            "delete",
            Encode!(&behaviour, &filter, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't update");

    let deleted_count = res.expect("Client error").expect("Failed to delete record");
    assert_eq!(deleted_count, 1);

    // Verify deletion
    let res: Result<IcDbmsResult<Vec<UserRecord>>, String> = client
        .update(
            "select",
            Encode!(&query, &transaction_id).expect("Failed to encode"),
        )
        .await
        .expect("Can't query");

    let records = res
        .expect("Client error")
        .expect("Failed to select records");
    assert!(records.is_empty());
}

#[pocket_ic_harness::test]
async fn test_should_run_sql_through_wrapper(env: PocketIcTestEnv<TestCanister>) {
    let client = PocketIcClient::new(env.dbms_canister_client_integration(), admin(), &env.pic);

    let res: TestResult<SqlResult> = client
        .update(
            "sql",
            Encode!(
                &"INSERT INTO users (id, name, email) VALUES (8, 'Heidi', 'heidi@example.com')",
                &Vec::<Value>::new(),
                &None::<TransactionId>
            )
            .expect("Failed to encode"),
        )
        .await
        .expect("Can't update");
    assert_eq!(
        res.expect("Client error").expect("insert"),
        SqlResult::RowsAffected(1)
    );

    let res: TestResult<SqlResult> = client
        .update(
            "sql_query",
            Encode!(
                &"SELECT name FROM users WHERE id = ?",
                &vec![Value::from(8u32)],
                &None::<TransactionId>
            )
            .expect("Failed to encode"),
        )
        .await
        .expect("Can't update");
    let SqlResult::Rows(rows) = res.expect("Client error").expect("select") else {
        panic!("expected rows");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0][0].1, Value::from("Heidi"));
}

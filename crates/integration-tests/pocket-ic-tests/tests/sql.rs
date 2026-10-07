use ic_dbms_api::prelude::{
    AclGrant, AclPermission, IcDbmsError, IcDbmsResult, SqlError, SqlResult, TransactionId, Value,
};
use ic_dbms_client::prelude::{Client as _, IcDbmsPocketIcClient};
use pocket_ic_harness::PocketIcTestEnv;
use pocket_ic_tests::{TestCanister, TestEnvExt as _, admin, assert_access_denied, bob};

async fn sql(
    client: &IcDbmsPocketIcClient<'_>,
    query: &str,
    params: Vec<Value>,
    transaction_id: Option<TransactionId>,
) -> IcDbmsResult<SqlResult> {
    client
        .sql(query, params, transaction_id)
        .await
        .expect("failed to call canister")
}

async fn sql_query(
    client: &IcDbmsPocketIcClient<'_>,
    query: &str,
    params: Vec<Value>,
    transaction_id: Option<TransactionId>,
) -> IcDbmsResult<SqlResult> {
    client
        .sql_query(query, params, transaction_id)
        .await
        .expect("failed to call canister")
}

/// Returns the values of every row, dropping the column definitions.
fn values(result: IcDbmsResult<SqlResult>) -> Vec<Vec<Value>> {
    match result.expect("sql failed") {
        SqlResult::Rows(rows) => rows
            .into_iter()
            .map(|row| row.into_iter().map(|(_, value)| value).collect())
            .collect(),
        other => panic!("expected rows, got {other:?}"),
    }
}

async fn insert_user(client: &IcDbmsPocketIcClient<'_>, id: u32, name: &str) {
    let result = sql(
        client,
        "INSERT INTO users (id, name, email) VALUES (?, ?, ?)",
        vec![
            Value::from(id),
            Value::from(name),
            Value::from(format!("{}@example.com", name.to_lowercase())),
        ],
        None,
    )
    .await;
    assert_eq!(result.expect("insert user"), SqlResult::RowsAffected(1));
}

/// Users 1 (Alice) and 2 (Bob), and post 1 by Alice.
async fn seed(client: &IcDbmsPocketIcClient<'_>) {
    insert_user(client, 1, "Alice").await;
    insert_user(client, 2, "Bob").await;
    let result = sql(
        client,
        "INSERT INTO posts (id, title, content, user) VALUES (1, 'Hello', 'First post', 1)",
        vec![],
        None,
    )
    .await;
    assert_eq!(result.expect("insert post"), SqlResult::RowsAffected(1));
}

async fn user_names(client: &IcDbmsPocketIcClient<'_>) -> Vec<Vec<Value>> {
    values(
        sql_query(
            client,
            "SELECT id, name FROM users ORDER BY id",
            vec![],
            None,
        )
        .await,
    )
}

#[pocket_ic_harness::test]
async fn test_sql_should_insert_and_select(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&client).await;

    let result = sql_query(
        &client,
        "SELECT id, name FROM users WHERE id = ?",
        vec![Value::from(1u32)],
        None,
    )
    .await;
    let SqlResult::Rows(rows) = result.expect("select") else {
        panic!("expected rows");
    };
    assert_eq!(rows.len(), 1);
    let names: Vec<&str> = rows[0]
        .iter()
        .map(|(column, _)| column.name.as_str())
        .collect();
    assert_eq!(names, vec!["id", "name"]);
    assert_eq!(rows[0][0].1, Value::from(1u32));
    assert_eq!(rows[0][1].1, Value::from("Alice"));
}

#[pocket_ic_harness::test]
async fn test_sql_should_join_tables(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&client).await;

    let result = sql_query(
        &client,
        "SELECT users.name, posts.title FROM posts JOIN users ON posts.user = users.id",
        vec![],
        None,
    )
    .await;
    let SqlResult::Rows(rows) = result.expect("join") else {
        panic!("expected rows");
    };
    assert_eq!(rows.len(), 1);
    let tables: Vec<Option<&str>> = rows[0]
        .iter()
        .map(|(column, _)| column.table.as_deref())
        .collect();
    assert_eq!(tables, vec![Some("users"), Some("posts")]);
    assert_eq!(rows[0][0].1, Value::from("Alice"));
    assert_eq!(rows[0][1].1, Value::from("Hello"));
}

#[pocket_ic_harness::test]
async fn test_sql_should_update_with_filter(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&client).await;

    let result = sql(
        &client,
        "UPDATE users SET name = ? WHERE id = 1",
        vec![Value::from("Alicia")],
        None,
    )
    .await;
    assert_eq!(result.expect("update"), SqlResult::RowsAffected(1));
    assert_eq!(
        user_names(&client).await,
        vec![
            vec![Value::from(1u32), Value::from("Alicia")],
            vec![Value::from(2u32), Value::from("Bob")],
        ]
    );
}

#[pocket_ic_harness::test]
async fn test_sql_should_delete_with_filter(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&client).await;

    let result = sql(&client, "DELETE FROM users WHERE id = 2", vec![], None).await;
    assert_eq!(result.expect("delete"), SqlResult::RowsAffected(1));
    assert_eq!(
        user_names(&client).await,
        vec![vec![Value::from(1u32), Value::from("Alice")]]
    );
}

#[pocket_ic_harness::test]
async fn test_sql_should_begin_and_commit(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);

    let SqlResult::TxBegin(tx) = sql(&client, "BEGIN", vec![], None).await.expect("begin") else {
        panic!("BEGIN must return TxBegin");
    };
    let result = sql(
        &client,
        "INSERT INTO users (id, name, email) VALUES (5, 'Frank', 'frank@example.com')",
        vec![],
        Some(tx),
    )
    .await;
    assert_eq!(result.expect("insert in tx"), SqlResult::RowsAffected(1));

    assert!(user_names(&client).await.is_empty());
    let in_tx = sql_query(&client, "SELECT name FROM users", vec![], Some(tx)).await;
    assert_eq!(values(in_tx), vec![vec![Value::from("Frank")]]);

    let result = sql(&client, "COMMIT", vec![], Some(tx)).await;
    assert_eq!(result.expect("commit"), SqlResult::TxCommit);
    assert_eq!(
        user_names(&client).await,
        vec![vec![Value::from(5u32), Value::from("Frank")]]
    );
}

#[pocket_ic_harness::test]
async fn test_sql_should_require_where_clause(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&client).await;

    for statement in ["UPDATE users SET name = 'X'", "DELETE FROM users"] {
        let result = sql(&client, statement, vec![], None).await;
        assert!(
            matches!(result, Err(IcDbmsError::Sql(SqlError::MissingWhereClause))),
            "`{statement}` returned {result:?}"
        );
    }
    assert_eq!(user_names(&client).await.len(), 2);
}

#[pocket_ic_harness::test]
async fn test_sql_query_should_only_run_selects(env: PocketIcTestEnv<TestCanister>) {
    let client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&client).await;

    for statement in [
        "INSERT INTO users (id, name, email) VALUES (9, 'Ivan', 'ivan@example.com')",
        "UPDATE users SET name = 'X' WHERE id = 1",
        "DELETE FROM users WHERE id = 2",
        "BEGIN",
    ] {
        let result = sql_query(&client, statement, vec![], None).await;
        assert!(
            matches!(result, Err(IcDbmsError::Sql(SqlError::Unsupported(_)))),
            "`{statement}` returned {result:?}"
        );
    }
    assert_eq!(
        user_names(&client).await,
        vec![
            vec![Value::from(1u32), Value::from("Alice")],
            vec![Value::from(2u32), Value::from("Bob")],
        ]
    );
    // The update endpoint runs reads too.
    let read = sql(&client, "SELECT name FROM users WHERE id = 2", vec![], None).await;
    assert_eq!(values(read), vec![vec![Value::from("Bob")]]);
}

#[pocket_ic_harness::test]
async fn test_sql_should_enforce_acl(env: PocketIcTestEnv<TestCanister>) {
    let admin_client = IcDbmsPocketIcClient::new(env.dbms_canister(), admin(), &env.pic);
    seed(&admin_client).await;
    admin_client
        .acl_grant(AclGrant::table(bob(), AclPermission::Read, "users"))
        .await
        .expect("failed to call canister")
        .expect("grant");

    let bob_client = IcDbmsPocketIcClient::new(env.dbms_canister(), bob(), &env.pic);
    assert_eq!(user_names(&bob_client).await.len(), 2);
    assert_access_denied(
        &sql(&bob_client, "DELETE FROM users WHERE id = 2", vec![], None).await,
        AclPermission::Delete,
        Some("users"),
    );
    assert_access_denied(
        &sql_query(
            &bob_client,
            "SELECT users.name FROM users JOIN posts ON users.id = posts.user",
            vec![],
            None,
        )
        .await,
        AclPermission::Read,
        Some("posts"),
    );
    let reserved = sql_query(&admin_client, "SELECT * FROM ic_dbms_acl", vec![], None).await;
    assert!(matches!(
        reserved,
        Err(IcDbmsError::Sql(SqlError::UnknownTable(table))) if table == "ic_dbms_acl"
    ));
}

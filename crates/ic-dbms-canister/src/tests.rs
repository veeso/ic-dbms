//! Test types, fixtures and mocks.

mod message;
mod post;
mod user;

use ic_dbms_api::prelude::{
    ColumnDef, Database as _, DeleteBehavior, Filter, InsertRecord as _, Query, QueryError,
    TableSchema as _, UpdateRecord as _, Value, flatten_table_columns,
};
use wasm_dbms::prelude::{DatabaseSchema, WasmDbmsDatabase};
use wasm_dbms_memory::prelude::MemoryProvider;

#[allow(unused_imports)]
pub use self::message::{
    MESSAGES_FIXTURES, Message, MessageInsertRequest, MessageRecord, MessageUpdateRequest,
};
#[allow(unused_imports)]
pub use self::post::{POSTS_FIXTURES, Post, PostInsertRequest, PostRecord, PostUpdateRequest};
#[allow(unused_imports)]
pub use self::user::{USERS_FIXTURES, User, UserInsertRequest, UserRecord, UserUpdateRequest};
use crate::memory::DBMS_CONTEXT;
use crate::prelude::{InsertIntegrityValidator, UpdateIntegrityValidator, get_referenced_tables};

/// Loads fixtures into the database for testing purposes.
///
/// Registers all test tables and inserts fixture data via [`WasmDbmsDatabase`].
///
/// # Panics
///
/// Panics if any operation fails.
pub fn load_fixtures() {
    DBMS_CONTEXT.with(|ctx| {
        ctx.register_table::<User>()
            .expect("failed to register `User` table");
        ctx.register_table::<Post>()
            .expect("failed to register `Post` table");
        ctx.register_table::<Message>()
            .expect("failed to register `Message` table");

        let db = WasmDbmsDatabase::oneshot(ctx, TestDatabaseSchema);

        // Insert users
        for (id, name) in USERS_FIXTURES.iter().enumerate() {
            let record = UserInsertRequest {
                id: (id as u32).into(),
                name: name.to_string().into(),
                email: format!("{}@example.com", name.to_lowercase()).into(),
                age: (20 + id as u32).into(),
            };
            db.insert::<User>(record).expect("failed to insert user");
        }

        // Insert posts
        for (id, (title, content, user_id)) in POSTS_FIXTURES.iter().enumerate() {
            let record = PostInsertRequest {
                id: (id as u32).into(),
                title: title.to_string().into(),
                content: content.to_string().into(),
                user: (*user_id).into(),
            };
            db.insert::<Post>(record).expect("failed to insert post");
        }

        // Insert messages
        for (id, (text, sender_id, recipient_id)) in MESSAGES_FIXTURES.iter().enumerate() {
            let record = MessageInsertRequest {
                id: (id as u32).into(),
                text: text.to_string().into(),
                sender: (*sender_id).into(),
                recipient: (*recipient_id).into(),
                read_at: ic_dbms_api::prelude::Nullable::Null,
            };
            db.insert::<Message>(record)
                .expect("failed to insert message");
        }
    });
}

pub struct TestDatabaseSchema;

impl<M> DatabaseSchema<M> for TestDatabaseSchema
where
    M: MemoryProvider,
{
    fn select(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &str,
        query: Query,
    ) -> ic_dbms_api::prelude::IcDbmsResult<Vec<Vec<(ColumnDef, Value)>>> {
        match table_name {
            name if name == User::table_name() => {
                let results = dbms.select_columns::<User>(query)?;
                Ok(flatten_table_columns(results))
            }
            name if name == Post::table_name() => {
                let results = dbms.select_columns::<Post>(query)?;
                Ok(flatten_table_columns(results))
            }
            name if name == Message::table_name() => {
                let results = dbms.select_columns::<Message>(query)?;
                Ok(flatten_table_columns(results))
            }
            _ => Err(ic_dbms_api::prelude::IcDbmsError::Query(
                QueryError::TableNotFound(table_name.to_string()),
            )),
        }
    }

    fn referenced_tables(&self, table: &'static str) -> Vec<(&'static str, Vec<&'static str>)> {
        let tables = &[
            (User::table_name(), User::columns()),
            (Post::table_name(), Post::columns()),
            (Message::table_name(), Message::columns()),
        ];
        get_referenced_tables(table, tables)
    }

    fn insert(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
    ) -> ic_dbms_api::prelude::IcDbmsResult<()> {
        match table_name {
            name if name == User::table_name() => {
                let insert_request = UserInsertRequest::from_values(record_values)?;
                dbms.insert::<User>(insert_request)
            }
            name if name == Post::table_name() => {
                let insert_request = PostInsertRequest::from_values(record_values)?;
                dbms.insert::<Post>(insert_request)
            }
            name if name == Message::table_name() => {
                let insert_request = MessageInsertRequest::from_values(record_values)?;
                dbms.insert::<Message>(insert_request)
            }
            _ => Err(ic_dbms_api::prelude::IcDbmsError::Query(
                QueryError::TableNotFound(table_name.to_string()),
            )),
        }
    }

    fn delete(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        delete_behavior: DeleteBehavior,
        filter: Option<Filter>,
    ) -> ic_dbms_api::prelude::IcDbmsResult<u64> {
        match table_name {
            name if name == User::table_name() => dbms.delete::<User>(delete_behavior, filter),
            name if name == Post::table_name() => dbms.delete::<Post>(delete_behavior, filter),
            name if name == Message::table_name() => {
                dbms.delete::<Message>(delete_behavior, filter)
            }
            _ => Err(ic_dbms_api::prelude::IcDbmsError::Query(
                QueryError::TableNotFound(table_name.to_string()),
            )),
        }
    }

    fn update(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        patch_values: &[(ColumnDef, Value)],
        filter: Option<Filter>,
    ) -> ic_dbms_api::prelude::IcDbmsResult<u64> {
        match table_name {
            name if name == User::table_name() => {
                let update_request = UserUpdateRequest::from_values(patch_values, filter);
                dbms.update::<User>(update_request)
            }
            name if name == Post::table_name() => {
                let update_request = PostUpdateRequest::from_values(patch_values, filter);
                dbms.update::<Post>(update_request)
            }
            name if name == Message::table_name() => {
                let update_request = MessageUpdateRequest::from_values(patch_values, filter);
                dbms.update::<Message>(update_request)
            }
            _ => Err(ic_dbms_api::prelude::IcDbmsError::Query(
                QueryError::TableNotFound(table_name.to_string()),
            )),
        }
    }

    fn validate_insert(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
    ) -> ic_dbms_api::prelude::IcDbmsResult<()> {
        match table_name {
            name if name == User::table_name() => {
                InsertIntegrityValidator::<User, M>::new(dbms).validate(record_values)
            }
            name if name == Post::table_name() => {
                InsertIntegrityValidator::<Post, M>::new(dbms).validate(record_values)
            }
            name if name == Message::table_name() => {
                InsertIntegrityValidator::<Message, M>::new(dbms).validate(record_values)
            }
            _ => Err(ic_dbms_api::prelude::IcDbmsError::Query(
                QueryError::TableNotFound(table_name.to_string()),
            )),
        }
    }

    fn validate_update(
        &self,
        dbms: &WasmDbmsDatabase<'_, M>,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
        old_pk: Value,
    ) -> ic_dbms_api::prelude::IcDbmsResult<()> {
        match table_name {
            name if name == User::table_name() => {
                UpdateIntegrityValidator::<User, M>::new(dbms, old_pk).validate(record_values)
            }
            name if name == Post::table_name() => {
                UpdateIntegrityValidator::<Post, M>::new(dbms, old_pk).validate(record_values)
            }
            name if name == Message::table_name() => {
                UpdateIntegrityValidator::<Message, M>::new(dbms, old_pk).validate(record_values)
            }
            _ => Err(ic_dbms_api::prelude::IcDbmsError::Query(
                QueryError::TableNotFound(table_name.to_string()),
            )),
        }
    }
}

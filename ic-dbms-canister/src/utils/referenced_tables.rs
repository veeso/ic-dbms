use ic_dbms_api::prelude::ColumnDef;

/// Given a list of tables with their column definitions,
/// returns the list of tables that reference the target table.
///
/// The returned list contains tuples of table names and the columns
/// that reference the target table.
///
/// Example:
///
/// If we have the following tables:
/// - users (id)
/// - posts (id, user_id) where user_id references users.id
/// - comments (id, user_id) where user_id references user.id
///
/// Calling `get_referenced_tables("users", ...)` would return:
/// `[("posts", &["user_id"]), ("comments", &["user_id"])]`
pub fn get_referenced_tables(
    target: &'static str,
    tables: &[(&'static str, &'static [ColumnDef])],
) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut referenced_tables = vec![];
    // iterate over tables different from target
    for (table_name, columns) in tables.iter().filter(|(name, _)| *name != target) {
        let mut referenced_tables_columns = vec![];
        // iterate over fks
        for fk in columns.iter().filter_map(|col| col.foreign_key.as_ref()) {
            if fk.foreign_table == target {
                referenced_tables_columns.push(fk.local_column);
            }
        }
        if !referenced_tables_columns.is_empty() {
            referenced_tables.push((*table_name, referenced_tables_columns));
        }
    }

    referenced_tables
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::prelude::TableSchema as _;
    use crate::tests::{Message, Post, User};

    #[test]
    fn test_should_get_referenced_tables() {
        let tables = &[
            (User::table_name(), User::columns()),
            (Post::table_name(), Post::columns()),
            (Message::table_name(), Message::columns()),
        ];
        let references = get_referenced_tables(User::table_name(), tables);
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].0, "posts");
        assert_eq!(references[0].1, vec!["user"]);
        assert_eq!(references[1].0, "messages");
        assert_eq!(references[1].1, vec!["sender", "recipient"]);
    }
}

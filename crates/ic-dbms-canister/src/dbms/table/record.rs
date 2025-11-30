use crate::dbms::query::QueryResult;
use crate::dbms::table::{ColumnDef, TableSchema};
use crate::dbms::value::Value;
use crate::prelude::Filter;

/// This trait represents a record returned by a [`crate::dbms::query::Query`] for a table.
pub trait TableRecord {
    /// The table schema associated with this record.
    type Schema: TableSchema<Record = Self>;

    /// Constructs [`TableRecord`] from a list of column values.
    fn from_values(values: &[(ColumnDef, Value)]) -> Self;

    /// Converts the record into a list of column [`Value`]s.
    fn to_values(&self) -> Vec<Value>;
}

/// This trait represents a record for inserting into a table.
pub trait InsertRecord: Sized {
    /// The [`TableRecord`] type associated with this table schema.
    type Record: TableRecord;
    /// The table schema associated with this record.
    type Schema: TableSchema<Record = Self::Record>;

    /// Converts the record into a list of column [`Value`]s for insertion.
    fn into_values(self) -> Vec<(ColumnDef, Value)>;

    /// Constructs the [`InsertRecord`] from an untyped [`UntypedInsertRecord`] representation.
    fn from_untyped(untyped: UntypedInsertRecord) -> QueryResult<Self>;

    /// Converts the record into an untyped [`UntypedInsertRecord`] representation.
    fn into_untyped(self) -> UntypedInsertRecord {
        UntypedInsertRecord {
            fields: self
                .into_values()
                .into_iter()
                .map(|(col_def, value)| (col_def.name.to_string(), value))
                .collect(),
        }
    }
}

/// This trait represents a record for updating a table.
pub trait UpdateRecord: Sized {
    /// The [`TableRecord`] type associated with this table schema.
    type Record: TableRecord;
    /// The table schema associated with this record.
    type Schema: TableSchema<Record = Self::Record>;

    /// Get the list of column [`Value`]s to be updated.
    fn update_values(&self) -> Vec<(ColumnDef, Value)>;

    /// Get the [`Filter`] condition for the update operation.
    fn where_clause(&self) -> Option<Filter>;

    /// Constructs the [`UpdateRecord`] from an untyped [`UntypedUpdateRecord`] representation.
    fn from_untyped(untyped: UntypedUpdateRecord) -> QueryResult<Self>;

    /// Converts the record into an untyped [`UntypedUpdateRecord`] representation.
    fn into_untyped(self) -> UntypedUpdateRecord {
        UntypedUpdateRecord {
            update_fields: self
                .update_values()
                .into_iter()
                .map(|(col_def, value)| (col_def.name.to_string(), value))
                .collect(),
            where_clause: self.where_clause(),
        }
    }
}

/// Untyped insert record for dynamic operations.
#[derive(Debug, Clone)]
pub struct UntypedInsertRecord {
    pub fields: Vec<(String, Value)>,
}

/// Untyped update record for dynamic operations.
#[derive(Debug, Clone)]
pub struct UntypedUpdateRecord {
    pub update_fields: Vec<(String, Value)>,
    pub where_clause: Option<Filter>,
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::tests::{UserInsertRequest, UserUpdateRequest};

    #[test]
    fn test_should_insert_make_into_untyped() {
        let typed = UserInsertRequest {
            id: 1.into(),
            name: "Alice".to_string().into(),
        };

        let untyped = typed.into_untyped();
        assert_eq!(
            untyped.fields,
            vec![
                ("id".to_string(), Value::Uint32(1.into())),
                ("name".to_string(), Value::Text("Alice".to_string().into()))
            ]
        );

        let from_untyped = UserInsertRequest::from_untyped(untyped).unwrap();
        assert_eq!(from_untyped.id.0, 1);
        assert_eq!(from_untyped.name.0, "Alice".to_string());
    }

    #[test]
    fn test_should_insert_make_from_untyped() {
        let typed = UserUpdateRequest {
            id: Some(2.into()),
            name: Some("Bob".to_string().into()),
            where_clause: Some(Filter::Eq("id", Value::Uint32(1.into()))),
        };
        let untyped = typed.into_untyped();

        assert_eq!(
            untyped.update_fields,
            vec![
                ("id".to_string(), Value::Uint32(2.into())),
                ("name".to_string(), Value::Text("Bob".to_string().into()))
            ]
        );
        assert_eq!(
            untyped.where_clause,
            Some(Filter::Eq("id", Value::Uint32(1.into())))
        );

        let from_untyped = UserUpdateRequest::from_untyped(untyped).unwrap();
        assert_eq!(from_untyped.id, Some(2.into()));
        assert_eq!(from_untyped.name, Some("Bob".to_string().into()));
        assert_eq!(
            from_untyped.where_clause,
            Some(Filter::Eq("id", Value::Uint32(1.into())))
        );
    }
}

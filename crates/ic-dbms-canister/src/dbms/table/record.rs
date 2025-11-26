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

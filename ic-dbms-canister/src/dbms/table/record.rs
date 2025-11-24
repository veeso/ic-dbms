use crate::dbms::table::{ColumnDef, TableSchema};
use crate::dbms::value::Value;

/// This trait represents a record returned by a [`crate::dbms::query::Query`] for a table.
pub trait TableRecord {
    /// The table schema associated with this record.
    type Schema: TableSchema<Record = Self>;

    /// Constructs [`TableRecord`] from a list of column values.
    fn from_values(values: &[(ColumnDef, Value)]) -> Self;

    /// Converts the record into a list of column [`Value`]s.
    fn to_values(&self) -> Vec<Value>;
}

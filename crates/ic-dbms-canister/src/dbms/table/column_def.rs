use crate::dbms::types::DataTypeKind;

/// Defines a column in a database table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnDef {
    /// The name of the column.
    pub name: &'static str,
    /// The data type of the column.
    pub data_type: DataTypeKind,
    /// Indicates if this column can contain NULL values.
    pub nullable: bool,
    /// Indicates if this column is part of the primary key.
    pub primary_key: bool,
    /// Foreign key definition, if any.
    pub foreign_keys: Option<ForeignKeyDef>,
}

/// Defines a foreign key relationship for a column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForeignKeyDef {
    pub table: &'static str,
    pub column: &'static str,
}

/// Defines the behavior for delete operations regarding foreign key constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeleteBehavior {
    /// Delete only the records matching the filter.
    Restrict,
    /// Cascade delete to related records.
    Cascade,
    /// Set foreign key fields to null in related records.
    /// Note: This requires the foreign key fields to be nullable. If they are not nullable,
    /// a [`crate::prelude::QueryError::ForeignKeyConstraintViolation`] error will be raised.
    SetNull,
}

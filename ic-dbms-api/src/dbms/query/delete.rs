use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Defines the behavior for delete operations regarding foreign key constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub enum DeleteBehavior {
    /// Delete only the records matching the filter.
    /// If there are foreign key constraints that would be violated, the operation will fail.
    Restrict,
    /// Cascade delete to related records.
    /// Any records that reference the deleted records via foreign keys will also be deleted.
    Cascade,
    /// Break the foreign key references.
    /// If there are foreign key constraints, the references will be broken.
    /// Don't use this option unless you are sure what you're doing!
    Break,
}

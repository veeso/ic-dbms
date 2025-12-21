use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// IcDbms Error type
#[derive(Debug, Error, CandidType, Serialize, Deserialize)]
pub enum IcDbmsError {
    #[error("Memory error: {0}")]
    Memory(#[from] crate::memory::MemoryError),
    #[error("Query error: {0}")]
    Query(#[from] crate::dbms::query::QueryError),
    #[error("Table error: {0}")]
    Table(#[from] crate::dbms::table::TableError),
    #[error("Transaction error: {0}")]
    Transaction(#[from] crate::dbms::transaction::TransactionError),
    #[error("Validation error: {0}")]
    Validation(String),
}

/// IcDbms Result type
pub type IcDbmsResult<T> = Result<T, IcDbmsError>;

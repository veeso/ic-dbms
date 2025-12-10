use candid::CandidType;
use serde::{Deserialize, Serialize};

/// Type alias for Transaction ID
pub type TransactionId = candid::Nat;

/// An enum representing possible errors that can occur during transaction operations.
#[derive(Debug, thiserror::Error, CandidType, Serialize, Deserialize)]
pub enum TransactionError {
    #[error("No active transaction")]
    NoActiveTransaction,
}

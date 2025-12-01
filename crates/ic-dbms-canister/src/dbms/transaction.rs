//! This module contains the implementation of transactions within the DBMS engine.

mod overlay;
mod session;

pub use self::overlay::DatabaseOverlay;
pub use self::session::{TRANSACTION_SESSION, TransactionId, TransactionSession};

/// A transaction represents a sequence of operations performed as a single logical unit of work.
#[derive(Debug, Default, Clone)]
pub struct Transaction(DatabaseOverlay);

impl Transaction {
    /// Get a reference to the [`DatabaseOverlay`] associated with this transaction.
    pub fn overlay(&self) -> &DatabaseOverlay {
        &self.0
    }

    /// Get a mutable reference to the [`DatabaseOverlay`] associated with this transaction.
    pub fn overlay_mut(&mut self) -> &mut DatabaseOverlay {
        &mut self.0
    }
}

/// An enum representing possible errors that can occur during transaction operations.
#[derive(Debug, thiserror::Error)]
pub enum TransactionError {
    #[error("No active transaction")]
    NoActiveTransaction,
}

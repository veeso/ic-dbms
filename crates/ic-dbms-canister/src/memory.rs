//! Memory module provides stable memory management for the IC DBMS Canister.

mod delegate;
mod encode;
mod provider;

use thiserror::Error;

pub use self::delegate::MemoryDelegate;
pub use self::encode::{DataSize, Encode};
use self::provider::MemoryProvider;

// instantiate a static memory manager with the stable memory provider
thread_local! {
    #[cfg(target_family = "wasm")]
    pub static MEMORY_MANAGER: MemoryManager<provider::IcMemoryProvider> = MemoryManager {
        provider: provider::IcMemoryProvider::default(),
    };

    #[cfg(not(target_family = "wasm"))]
    pub static MEMORY_MANAGER: MemoryManager<provider::HeapMemoryProvider> = MemoryManager {
        provider: provider::HeapMemoryProvider::default(),
    };
}

/// The result type for memory operations.
pub type MemoryResult<T> = Result<T, MemoryError>;

/// An enum representing possible memory-related errors.
#[derive(Debug, Error)]
pub enum MemoryError {
    /// Error when attempting to access stable memory out of bounds.
    #[error("Stable memory access out of bounds")]
    OutOfBounds,
    /// Error when failing to grow stable memory.
    #[error("Failed to grow stable memory: {0}")]
    StableMemoryError(#[from] ic_cdk::stable::StableMemoryError),
}

/// The memory manager is the main struct responsible for handling the stable memory operations.
///
/// It takes advantage of [`MemoryDelegate`]s to know how to allocate and write memory for different kind of data.
pub struct MemoryManager<P: MemoryProvider> {
    provider: P,
}

//! Prelude exposes all the types for the `ic-dbms-api` crate.

// Re-export everything from wasm-dbms-api prelude (includes derive macros).
pub use wasm_dbms_api::prelude::*;

// IC-specific types.
pub use crate::error::{IcDbmsError, IcDbmsResult};
pub use crate::init::{IcDbmsCanisterArgs, IcDbmsCanisterInitArgs, IcDbmsCanisterUpgradeArgs};
pub use crate::principal::Principal;

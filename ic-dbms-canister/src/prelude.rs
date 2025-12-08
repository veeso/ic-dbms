//! Re-exports all the most commonly used items from this crate.

pub use ic_dbms_api::prelude::*;
pub use ic_dbms_macros::{Encode, Table};

pub use crate::dbms::IcDbmsDatabase;
pub use crate::dbms::integrity::InsertIntegrityValidator;
pub use crate::dbms::schema::DatabaseSchema;
pub use crate::dbms::transaction::TRANSACTION_SESSION;
pub use crate::memory::{SCHEMA_REGISTRY, SchemaRegistry};
pub use crate::utils::self_reference_values;

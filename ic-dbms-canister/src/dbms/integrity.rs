//! This module exposes all the integrity validators for the DBMS.

mod insert;

pub use self::insert::InsertIntegrityValidator;
use crate::IcDbmsResult;
use crate::dbms::Database;
use crate::dbms::table::ColumnDef;
use crate::dbms::value::Value;

/// Trait for integrity validators.
///
/// The integrity validator is responsible for validating the integrity of
/// database operations such as insert, update, and delete based on the table schema.
///
/// It must be globally implemented by the DBMS to ensure consistent integrity checks
/// across all tables and operations.
///
/// It is provided to the [`Database`] to allow it to perform integrity validation before running transactions.
pub trait IntegrityValidator {
    fn validate_insert(
        &self,
        dbms: &Database,
        table_name: &'static str,
        record_values: &[(ColumnDef, Value)],
    ) -> IcDbmsResult<()>;
}

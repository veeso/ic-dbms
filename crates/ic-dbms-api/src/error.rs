//! Error type returned by every endpoint of an ic-dbms canister.

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasm_dbms_api::prelude::{
    DbmsError, MemoryError, MigrationError, QueryError, TableError, TransactionError,
};

use crate::acl::AclError;

/// Error returned by every endpoint of an ic-dbms canister.
///
/// Engine errors are wrapped in [`IcDbmsError::Dbms`]; access control
/// failures, which the engine knows nothing about, are
/// [`IcDbmsError::Acl`].
#[derive(Debug, Error, CandidType, Serialize, Deserialize)]
pub enum IcDbmsError {
    /// The canister access control list refused the call.
    #[error("Access control error: {0}")]
    Acl(#[from] AclError),
    /// The database engine reported an error.
    #[error("{0}")]
    Dbms(#[from] DbmsError),
}

impl From<MemoryError> for IcDbmsError {
    fn from(error: MemoryError) -> Self {
        Self::Dbms(DbmsError::from(error))
    }
}

impl From<MigrationError> for IcDbmsError {
    fn from(error: MigrationError) -> Self {
        Self::Dbms(DbmsError::from(error))
    }
}

impl From<QueryError> for IcDbmsError {
    fn from(error: QueryError) -> Self {
        Self::Dbms(DbmsError::from(error))
    }
}

impl From<TableError> for IcDbmsError {
    fn from(error: TableError) -> Self {
        Self::Dbms(DbmsError::from(error))
    }
}

impl From<TransactionError> for IcDbmsError {
    fn from(error: TransactionError) -> Self {
        Self::Dbms(DbmsError::from(error))
    }
}

/// Result type returned by every endpoint of an ic-dbms canister.
pub type IcDbmsResult<T> = Result<T, IcDbmsError>;

#[cfg(test)]
mod test {

    use super::*;
    use crate::acl::{AclError, Permission};

    #[test]
    fn test_should_display_dbms_error_transparently() {
        let error = IcDbmsError::Dbms(DbmsError::Memory(MemoryError::OutOfBounds));
        assert_eq!(
            error.to_string(),
            "Memory error: Stable memory access out of bounds"
        );
        let error = IcDbmsError::Dbms(DbmsError::Query(QueryError::UnknownColumn(
            "foo".to_string(),
        )));
        assert_eq!(error.to_string(), "Query error: Unknown column: foo");
        let error = IcDbmsError::Dbms(DbmsError::Validation("invalid email".to_string()));
        assert_eq!(error.to_string(), "Validation error: invalid email");
    }

    #[test]
    fn test_should_display_acl_error() {
        let error = IcDbmsError::Acl(AclError::AccessDenied {
            required: Permission::Admin,
        });
        assert_eq!(
            error.to_string(),
            "Access control error: access denied: the caller lacks the Admin permission"
        );
    }

    #[test]
    fn test_should_convert_from_engine_errors() {
        let error: IcDbmsError = MemoryError::OutOfBounds.into();
        assert!(matches!(
            error,
            IcDbmsError::Dbms(DbmsError::Memory(MemoryError::OutOfBounds))
        ));
        let error: IcDbmsError = QueryError::UnknownColumn("col".to_string()).into();
        assert!(matches!(error, IcDbmsError::Dbms(DbmsError::Query(_))));
        let error: IcDbmsError = TableError::TableNotFound.into();
        assert!(matches!(error, IcDbmsError::Dbms(DbmsError::Table(_))));
        let error: IcDbmsError = TransactionError::NoActiveTransaction.into();
        assert!(matches!(
            error,
            IcDbmsError::Dbms(DbmsError::Transaction(_))
        ));
        let error: IcDbmsError = DbmsError::Sanitize("x".to_string()).into();
        assert!(matches!(error, IcDbmsError::Dbms(DbmsError::Sanitize(_))));
    }

    #[test]
    fn test_should_convert_from_acl_error() {
        let error: IcDbmsError = AclError::LastAdmin.into();
        assert!(matches!(error, IcDbmsError::Acl(AclError::LastAdmin)));
    }

    #[test]
    fn test_should_candid_roundtrip() {
        let error = IcDbmsError::Acl(AclError::AnonymousPrincipal);
        let bytes = candid::encode_one(&error).expect("encode");
        let decoded: IcDbmsError = candid::decode_one(&bytes).expect("decode");
        assert!(matches!(
            decoded,
            IcDbmsError::Acl(AclError::AnonymousPrincipal)
        ));
    }
}

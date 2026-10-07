//! Access control types shared by the canister and the clients.
//!
//! Today the model has a single permission, [`Permission::Admin`], which
//! allows every operation. The types are shaped so that per-table grants
//! (for example "read tables X and Y, insert into X") can be added as new
//! [`Permission`] variants without changing the endpoints that take or
//! return them.

use std::fmt;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// A permission that can be granted to a principal.
///
/// The endpoints already carry this enum, so future per-table grants can be
/// represented by adding variants; clients that need to use new variants
/// must update their type definitions accordingly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CandidType, Serialize, Deserialize)]
pub enum Permission {
    /// Full access: every table operation, transactions, ACL management and
    /// schema migrations.
    Admin,
}

impl fmt::Display for Permission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Admin => write!(f, "Admin"),
        }
    }
}

/// A principal together with the permissions it holds.
#[derive(Clone, Debug, PartialEq, Eq, CandidType, Serialize, Deserialize)]
pub struct AclEntry {
    /// The principal the permissions belong to.
    pub principal: candid::Principal,
    /// The permissions held by `principal`.
    pub permissions: Vec<Permission>,
}

/// Errors raised by the canister access control list.
#[derive(Clone, Debug, PartialEq, Eq, Error, CandidType, Serialize, Deserialize)]
pub enum AclError {
    /// The caller does not hold the permission the operation requires.
    #[error("access denied: the caller lacks the {required} permission")]
    AccessDenied {
        /// The permission the operation requires.
        required: Permission,
    },
    /// Permissions cannot be granted to the anonymous principal.
    #[error("the anonymous principal cannot hold permissions")]
    AnonymousPrincipal,
    /// The operation would leave the canister without any admin.
    #[error("at least one principal must retain the Admin permission")]
    LastAdmin,
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_permission_display() {
        assert_eq!(Permission::Admin.to_string(), "Admin");
    }

    #[test]
    fn test_acl_entry_candid_roundtrip() {
        let entry = AclEntry {
            principal: candid::Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap(),
            permissions: vec![Permission::Admin],
        };
        let bytes = candid::encode_one(&entry).expect("encode");
        let decoded: AclEntry = candid::decode_one(&bytes).expect("decode");
        assert_eq!(decoded, entry);
    }

    #[test]
    fn test_acl_error_candid_roundtrip() {
        for error in [
            AclError::AccessDenied {
                required: Permission::Admin,
            },
            AclError::AnonymousPrincipal,
            AclError::LastAdmin,
        ] {
            let bytes = candid::encode_one(&error).expect("encode");
            let decoded: AclError = candid::decode_one(&bytes).expect("decode");
            assert_eq!(decoded, error);
        }
    }

    #[test]
    fn test_acl_error_display() {
        assert_eq!(
            AclError::AccessDenied {
                required: Permission::Admin
            }
            .to_string(),
            "access denied: the caller lacks the Admin permission"
        );
        assert_eq!(
            AclError::AnonymousPrincipal.to_string(),
            "the anonymous principal cannot hold permissions"
        );
        assert_eq!(
            AclError::LastAdmin.to_string(),
            "at least one principal must retain the Admin permission"
        );
    }
}

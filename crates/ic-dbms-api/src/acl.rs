//! Access control types shared by the canister and the clients.
//!
//! The canister stores one [`AclGrant`] per row of its reserved
//! `ic_dbms_acl` table: a principal, an [`AclPermission`] and an optional
//! table name, where no table means every table.

use std::borrow::Cow;
use std::fmt;

use candid::CandidType;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wasm_dbms_api::prelude::{
    DataSize, DataType, DecodeError, Encode, MSize, MemoryError, MemoryResult, PageOffset,
};
use wasm_dbms_macros::CustomDataType;

/// An operation a grant allows.
///
/// Stored in the reserved `ic_dbms_acl` table as a one-byte custom data
/// type and exposed through Candid as a variant.
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Hash,
    CustomDataType,
    CandidType,
    Serialize,
    Deserialize,
)]
#[type_tag = "acl_permission"]
pub enum AclPermission {
    /// Everything: every operation on every table, transactions, ACL
    /// management and schema migrations. Always granted on every table.
    Admin,
    /// Select and aggregate on the granted table. The default, because it
    /// is the narrowest permission.
    #[default]
    Read,
    /// Insert into the granted table.
    Insert,
    /// Update rows of the granted table.
    Update,
    /// Delete rows of the granted table.
    Delete,
}

impl AclPermission {
    const ALL: [Self; 5] = [
        Self::Admin,
        Self::Read,
        Self::Insert,
        Self::Update,
        Self::Delete,
    ];

    /// The byte stored in stable memory. Never reorder: persisted rows
    /// depend on it.
    fn tag(self) -> u8 {
        match self {
            Self::Admin => 0,
            Self::Read => 1,
            Self::Insert => 2,
            Self::Update => 3,
            Self::Delete => 4,
        }
    }
}

impl fmt::Display for AclPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Admin => "Admin",
            Self::Read => "Read",
            Self::Insert => "Insert",
            Self::Update => "Update",
            Self::Delete => "Delete",
        };
        f.write_str(name)
    }
}

/// The access required by a protected operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, CandidType, Serialize, Deserialize)]
pub enum AclRequirement {
    /// One specific permission, optionally on a table.
    Permission(AclPermission),
    /// At least one grant of any kind.
    AnyGrant,
}

impl From<AclPermission> for AclRequirement {
    fn from(permission: AclPermission) -> Self {
        Self::Permission(permission)
    }
}

impl fmt::Display for AclRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Permission(permission) => write!(f, "the {permission} permission"),
            Self::AnyGrant => f.write_str("any grant"),
        }
    }
}

impl Encode for AclPermission {
    const SIZE: DataSize = DataSize::Fixed(1);

    const ALIGNMENT: PageOffset = 1;

    fn encode(&'_ self) -> Cow<'_, [u8]> {
        Cow::Owned(vec![self.tag()])
    }

    fn decode(data: Cow<[u8]>) -> MemoryResult<Self>
    where
        Self: Sized,
    {
        let byte = *data
            .first()
            .ok_or(MemoryError::DecodeError(DecodeError::TooShort))?;
        Self::ALL
            .into_iter()
            .find(|permission| permission.tag() == byte)
            .ok_or(MemoryError::DecodeError(DecodeError::InvalidDiscriminant(
                byte,
            )))
    }

    fn size(&self) -> MSize {
        1
    }
}

impl DataType for AclPermission {}

/// One grant: `principal` may perform `permission` on `table`, or on every
/// table when `table` is `None`.
///
/// [`AclPermission::Admin`] grants always have `table: None`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, CandidType, Serialize, Deserialize)]
pub struct AclGrant {
    /// The principal that receives the grant.
    pub principal: candid::Principal,
    /// The operation allowed.
    pub permission: AclPermission,
    /// The table the grant applies to; `None` means every table, including
    /// tables added later.
    pub table: Option<String>,
}

impl AclGrant {
    /// Full access for `principal`.
    pub fn admin(principal: candid::Principal) -> Self {
        Self {
            principal,
            permission: AclPermission::Admin,
            table: None,
        }
    }

    /// `permission` on the table called `table`.
    pub fn table(
        principal: candid::Principal,
        permission: AclPermission,
        table: impl Into<String>,
    ) -> Self {
        Self {
            principal,
            permission,
            table: Some(table.into()),
        }
    }

    /// `permission` on every table, including tables added later.
    pub fn all_tables(principal: candid::Principal, permission: AclPermission) -> Self {
        Self {
            principal,
            permission,
            table: None,
        }
    }
}

/// Errors raised by the canister access control list.
#[derive(Clone, Debug, PartialEq, Eq, Error, CandidType, Serialize, Deserialize)]
pub enum AclError {
    /// The caller does not satisfy the operation's access requirement.
    ///
    /// `table` names the table where access was required, or is `None` for
    /// operations that are not tied to a table (transactions, ACL management
    /// and migrations).
    #[error("access denied: the caller lacks {required}{}", on_table(.table))]
    AccessDenied {
        /// The access the operation requires.
        required: AclRequirement,
        /// The table the access is required on.
        table: Option<String>,
    },
    /// Permissions cannot be granted to the anonymous principal.
    #[error("the anonymous principal cannot hold permissions")]
    AnonymousPrincipal,
    /// The operation would leave the canister without any admin.
    #[error("at least one principal must retain the Admin permission")]
    LastAdmin,
    /// An `Admin` grant named a table; `Admin` always covers every table.
    #[error("the Admin permission applies to every table and cannot name one")]
    AdminGrantWithTable,
    /// The grant named a table that is not one of the canister's tables.
    #[error("table `{0}` does not exist or cannot be granted")]
    InvalidTable(String),
}

fn on_table(table: &Option<String>) -> String {
    table
        .as_ref()
        .map(|table| format!(" on table `{table}`"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {

    use std::borrow::Cow;

    use wasm_dbms_api::prelude::{CustomDataType as _, DecodeError, MemoryError, Value};

    use super::*;

    const ALL_PERMISSIONS: [AclPermission; 5] = [
        AclPermission::Admin,
        AclPermission::Read,
        AclPermission::Insert,
        AclPermission::Update,
        AclPermission::Delete,
    ];

    fn principal() -> candid::Principal {
        candid::Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap()
    }

    #[test]
    fn test_acl_permission_encode_decode_roundtrip() {
        for permission in ALL_PERMISSIONS {
            let encoded = permission.encode();
            assert_eq!(encoded.len(), 1);
            assert_eq!(permission.size(), 1);
            assert_eq!(AclPermission::decode(encoded).unwrap(), permission);
        }
    }

    #[test]
    fn test_acl_permission_decode_rejects_bad_bytes() {
        assert!(matches!(
            AclPermission::decode(Cow::Borrowed(&[])),
            Err(MemoryError::DecodeError(DecodeError::TooShort))
        ));
        assert!(matches!(
            AclPermission::decode(Cow::Borrowed(&[9])),
            Err(MemoryError::DecodeError(DecodeError::InvalidDiscriminant(
                9
            )))
        ));
    }

    #[test]
    fn test_acl_permission_display() {
        let names: Vec<String> = ALL_PERMISSIONS.iter().map(ToString::to_string).collect();
        assert_eq!(names, ["Admin", "Read", "Insert", "Update", "Delete"]);
    }

    #[test]
    fn test_acl_permission_candid_roundtrip() {
        for permission in ALL_PERMISSIONS {
            let bytes = candid::encode_one(permission).expect("encode");
            let decoded: AclPermission = candid::decode_one(&bytes).expect("decode");
            assert_eq!(decoded, permission);
        }
    }

    #[test]
    fn test_acl_permission_converts_to_tagged_custom_value() {
        assert_eq!(AclPermission::TYPE_TAG, "acl_permission");
        let value: Value = AclPermission::Insert.into();
        assert!(matches!(
            value,
            Value::Custom(ref custom)
                if custom.type_tag == "acl_permission" && custom.display == "Insert"
        ));
    }

    #[test]
    fn test_acl_grant_constructors() {
        assert_eq!(
            AclGrant::admin(principal()),
            AclGrant {
                principal: principal(),
                permission: AclPermission::Admin,
                table: None,
            }
        );
        assert_eq!(
            AclGrant::table(principal(), AclPermission::Read, "users"),
            AclGrant {
                principal: principal(),
                permission: AclPermission::Read,
                table: Some("users".to_string()),
            }
        );
        assert_eq!(
            AclGrant::all_tables(principal(), AclPermission::Delete),
            AclGrant {
                principal: principal(),
                permission: AclPermission::Delete,
                table: None,
            }
        );
    }

    #[test]
    fn test_acl_grant_candid_roundtrip() {
        for grant in [
            AclGrant::admin(principal()),
            AclGrant::table(principal(), AclPermission::Update, "posts"),
        ] {
            let bytes = candid::encode_one(&grant).expect("encode");
            let decoded: AclGrant = candid::decode_one(&bytes).expect("decode");
            assert_eq!(decoded, grant);
        }
    }

    #[test]
    fn test_acl_error_candid_roundtrip() {
        for error in [
            AclError::AccessDenied {
                required: AclPermission::Admin.into(),
                table: None,
            },
            AclError::AccessDenied {
                required: AclPermission::Read.into(),
                table: Some("users".to_string()),
            },
            AclError::AccessDenied {
                required: AclRequirement::AnyGrant,
                table: None,
            },
            AclError::AnonymousPrincipal,
            AclError::LastAdmin,
            AclError::AdminGrantWithTable,
            AclError::InvalidTable("nope".to_string()),
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
                required: AclPermission::Admin.into(),
                table: None,
            }
            .to_string(),
            "access denied: the caller lacks the Admin permission"
        );
        assert_eq!(
            AclError::AccessDenied {
                required: AclPermission::Delete.into(),
                table: Some("posts".to_string()),
            }
            .to_string(),
            "access denied: the caller lacks the Delete permission on table `posts`"
        );
        assert_eq!(
            AclError::AnonymousPrincipal.to_string(),
            "the anonymous principal cannot hold permissions"
        );
        assert_eq!(
            AclError::LastAdmin.to_string(),
            "at least one principal must retain the Admin permission"
        );
        assert_eq!(
            AclError::AdminGrantWithTable.to_string(),
            "the Admin permission applies to every table and cannot name one"
        );
        assert_eq!(
            AclError::InvalidTable("nope".to_string()).to_string(),
            "table `nope` does not exist or cannot be granted"
        );
        assert_eq!(
            AclError::AccessDenied {
                required: AclRequirement::AnyGrant,
                table: None,
            }
            .to_string(),
            "access denied: the caller lacks any grant"
        );
    }
}

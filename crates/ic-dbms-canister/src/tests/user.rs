use ic_dbms_api::prelude::{Encode, TableSchema, Text, Uint32};
use ic_dbms_macros::Table;

use crate::memory::{SCHEMA_REGISTRY, TableRegistry};

/// A simple user struct for testing purposes.
#[derive(Debug, Table, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
}

pub const USERS_FIXTURES: &[&str] = &[
    "Alice", "Bob", "Charlie", "Diana", "Eve", "Frank", "Grace", "Heidi", "Ivan", "Judy",
];

/// Loads fixtures into the database for testing purposes.
///
/// # Panics
///
/// Panics if any operation fails.
pub fn load_fixtures() {
    // register tables
    let user_pages = SCHEMA_REGISTRY
        .with_borrow_mut(|sr| sr.register_table::<User>())
        .expect("failed to register `User` table");

    let mut user_table: TableRegistry =
        TableRegistry::load(user_pages).expect("failed to load `User` table registry");

    // insert users
    for (id, user) in USERS_FIXTURES.iter().enumerate() {
        let user = User {
            id: Uint32(id as u32),
            name: Text(user.to_string()),
        };
        user_table.insert(user).expect("failed to insert user");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_encode_decode() {
        let user = User {
            id: 42u32.into(),
            name: "Alice".to_string().into(),
        };
        let encoded = user.encode();
        let decoded = User::decode(encoded).unwrap();
        assert_eq!(user, decoded);
    }

    #[test]
    fn test_should_have_fingerprint() {
        let fingerprint = User::fingerprint();
        assert_ne!(fingerprint, 0);
    }
}

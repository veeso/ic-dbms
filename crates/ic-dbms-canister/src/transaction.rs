//! Heap-only ledger from transaction ID to the principal that opened it.
//!
//! The engine identifies a transaction by its ID alone. The canister records
//! the caller next to each new ID here and checks it before every
//! transactional call, so one principal cannot read, write, commit or roll
//! back another principal's transaction.
//!
//! The ledger lives on the heap and is never written to stable memory. The
//! engine keeps its open transactions on the heap as well and transaction IDs
//! restart from zero with every new context, so after an upgrade the open
//! transactions and their owners disappear together and stay consistent.

use std::cell::RefCell;
use std::collections::HashMap;

use candid::Principal;
use ic_dbms_api::prelude::TransactionId;

thread_local! {
    static TRANSACTION_OWNERS: RefCell<HashMap<TransactionId, Principal>> =
        RefCell::new(HashMap::new());
}

/// Records `owner` as the principal that opened `transaction_id`.
pub fn record(transaction_id: TransactionId, owner: Principal) {
    TRANSACTION_OWNERS.with_borrow_mut(|owners| {
        owners.insert(transaction_id, owner);
    });
}

/// Returns the principal that opened `transaction_id`, if the ledger knows it.
pub fn owner(transaction_id: &TransactionId) -> Option<Principal> {
    TRANSACTION_OWNERS.with_borrow(|owners| owners.get(transaction_id).copied())
}

/// Removes the entry for `transaction_id`. A missing entry is not an error.
pub fn forget(transaction_id: &TransactionId) {
    TRANSACTION_OWNERS.with_borrow_mut(|owners| {
        owners.remove(transaction_id);
    });
}

/// Removes every entry.
pub fn clear() {
    TRANSACTION_OWNERS.with_borrow_mut(HashMap::clear);
}

/// Returns the number of recorded transactions.
pub fn len() -> usize {
    TRANSACTION_OWNERS.with_borrow(HashMap::len)
}

#[cfg(test)]
mod tests {

    use candid::Principal;

    use super::*;

    fn alice() -> Principal {
        Principal::from_text("ghsi2-tqaaa-aaaan-aaaca-cai").unwrap()
    }

    fn bob() -> Principal {
        Principal::from_text("ryjl3-tyaaa-aaaaa-aaaba-cai").unwrap()
    }

    #[test]
    fn test_should_record_and_lookup_owner() {
        record(1, alice());
        record(2, bob());
        assert_eq!(owner(&1), Some(alice()));
        assert_eq!(owner(&2), Some(bob()));
        assert_eq!(owner(&3), None);
        assert_eq!(len(), 2);
    }

    #[test]
    fn test_should_overwrite_owner_of_reused_id() {
        record(7, alice());
        record(7, bob());
        assert_eq!(owner(&7), Some(bob()));
        assert_eq!(len(), 1);
    }

    #[test]
    fn test_should_forget_entry() {
        record(1, alice());
        forget(&1);
        assert_eq!(owner(&1), None);
        forget(&1);
        assert_eq!(len(), 0);
    }

    #[test]
    fn test_should_clear_all_entries() {
        record(1, alice());
        record(2, bob());
        clear();
        assert_eq!(len(), 0);
    }
}

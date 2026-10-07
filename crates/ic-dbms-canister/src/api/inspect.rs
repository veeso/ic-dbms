//! Inspect implementation for the IC DBMS canister.
//!
//! Every call is accepted here; the endpoint bodies perform the real checks
//! and return errors as values.

/// Handles an inspect call to the canister.
pub fn inspect() {
    ic_cdk::api::accept_message();
}

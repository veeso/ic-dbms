#![crate_name = "ic_dbms_macros"]
#![crate_type = "lib"]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(clippy::print_stdout)]
#![deny(clippy::print_stderr)]

//! Macros and derive for ic-dbms-canister
//!
//! This crate provides procedural macros to automatically implement traits
//! required by the `ic-dbms-canister`.
//!
//! ## Provided Derive Macros
//!
//! - `DbmsCanister`: Automatically implements the API for the ic-dbms-canister.
//!
//! All other derive macros (`Encode`, `Table`, `DatabaseSchema`, `CustomDataType`)
//! are provided by `wasm-dbms-macros` and re-exported through the
//! `ic-dbms-api` and `ic-dbms-canister` preludes.
//!
//! ## Feature flags
//!
//! | name  | description                                           | default |
//! | ----- | ----------------------------------------------------- | ------- |
//! | `sql` | Generate the `sql` and `sql_query` canister endpoints. |         |

#![doc(html_playground_url = "https://play.rust-lang.org")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/veeso/wasm-dbms/main/assets/images/cargo/logo-128.png"
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/veeso/wasm-dbms/main/assets/images/cargo/logo-512.png"
)]

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

mod dbms_canister;

/// Automatically implements the api for the ic-dbms-canister: the lifecycle
/// hooks, the access control endpoints, the transaction endpoints and the
/// CRUD endpoints for the defined tables.
///
/// The derive always registers the reserved `ic_dbms_acl` table; table names
/// starting with `ic_dbms_` are rejected at compile time.
///
/// With the `sql` feature of `ic-dbms-canister` enabled, the derive also
/// emits the `sql` update endpoint and the `sql_query` query endpoint,
/// which run SQL statements against the database.
///
/// The derive also defines the canister's `pre_upgrade` hook, which clears
/// the transaction ownership ledger. Define `post_upgrade` yourself when you
/// need one; do not define another `pre_upgrade`.
#[proc_macro_derive(DbmsCanister, attributes(tables))]
pub fn derive_dbms_canister(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    match self::dbms_canister::dbms_canister(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

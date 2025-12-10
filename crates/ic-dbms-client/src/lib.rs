//! # IC DBMS Client
//!
//! This crate exposes all the types which may be used by an external canister to interact with
//! an IC DBMS Canister instance.
//!
//! You can import all the useful types and traits by using the prelude module:
//!
//! ```rust
//! use ic_dbms_client::prelude::*;
//! ```
//!
//! ### Query
//!
//! - [`DeleteBehavior`](crate::prelude::DeleteBehavior)
//! - [`Filter`](crate::prelude::Filter)
//! - [`Query`](crate::prelude::Query)
//! - [`QueryBuilder`](crate::prelude::QueryBuilder)
//! - [`OrderDirection`](crate::prelude::OrderDirection)
//! - [`Select`](crate::prelude::Select)
//!
//! ### Table
//!
//! - [`ColumnDef`](crate::prelude::ColumnDef)
//! - [`ForeignKeyDef`](crate::prelude::ForeignKeyDef)
//! - [`InsertRecord`](crate::prelude::InsertRecord)
//! - [`TableColumns`](crate::prelude::TableColumns)
//! - [`TableError`](crate::prelude::TableError)
//! - [`TableRecord`](crate::prelude::TableRecord)
//! - [`UpdateRecord`](crate::prelude::UpdateRecord)
//! - [`ValuesSource`](crate::prelude::ValuesSource)
//!
//! ### Types
//!
//! - [`Blob`](crate::prelude::Blob)
//! - [`Boolean`](crate::prelude::Boolean)
//! - [`Date`](crate::prelude::Date)
//! - [`DateTime`](crate::prelude::DateTime)
//! - [`Decimal`](crate::prelude::Decimal)
//! - [`Int32`](crate::prelude::Int32)
//! - [`Int64`](crate::prelude::Int64)
//! - [`Nullable`](crate::prelude::Nullable)
//! - [`Principal`](crate::prelude::Principal)
//! - [`Text`](crate::prelude::Text)
//! - [`Uint32`](crate::prelude::Uint32)
//! - [`Uint64`](crate::prelude::Uint64)
//! - [`Uuid`](crate::prelude::Uuid)
//!
//! ### Value
//!
//! - [`Value`](crate::prelude::Value)
//!
//! ## Interact with an IC DBMS Canister
//!
//! ## Client
//!
//! All the client methods can be accessed  through the [`Client`](crate::prelude::Client) trait.
//!
//! The crate provides an implementation of the client for IC DBMS Canister, called [`IcDbmsCanisterClient`](crate::prelude::IcDbmsCanisterClient),
//! which can be used on ic canisters.
//!
//! If you want to use the client in integration tests with `pocket-ic`, you can use the
//! [`IcDbmsPocketIcClient`](crate::prelude::IcDbmsPocketIcClient) implementation, but first you need to enable the `pocket-ic` feature.
//!
//! ## Usage
//!
//! ### Add the dependencies
//!
//! ```toml
//! [dependencies]
//! candid = "0.10"
//! ic-dbms-api = "0.1"
//! ic-dbms-client = "0.1"
//! serde = "1"
//! ```
//!
//! ### Implement the record types
//!
//! You can define your table as you did for the database, or use a common crate to share the types between the canisters.
//!
//! ```rust,ignore
//! use candid::{CandidType, Principal};
//! use ic_dbms_api::prelude::{Nullable, Query, Table, TableSchema, Text, Uint32, Uint64};
//! use ic_dbms_client::prelude::{Client as _, IcDbmsCanisterClient};
//! use serde::Deserialize;
//!
//! #[derive(Table, CandidType, Clone, Deserialize)]
//! #[table = "users"]
//! pub struct User {
//!     #[primary_key]
//!     id: Uint64,
//!     name: Text,
//!     email: Text,
//!     age: Nullable<Uint32>,
//! }
//! ```
//!
//! ### Use the client
//!
//! ```rust,ignore
//! let principal = Principal::from_text("...")?;
//! let client = IcDbmsCanisterClient::new(principal);
//!
//! let alice = UserInsertRequest {
//!     id: 1.into(),
//!     name: "Alice".into(),
//!     email: "alice@example.com".into(),
//!     age: Nullable::Value(30.into()),
//! };
//!
//! client
//!     .insert::<User>(User::table_name(), alice, None)
//!     .await?;
//! ```
//!

#![doc(html_playground_url = "https://play.rust-lang.org")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/veeso/ic-dbms/main/assets/images/cargo/logo-128.png"
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/veeso/ic-dbms/main/assets/images/cargo/logo-512.png"
)]

mod client;
mod errors;
pub mod prelude;

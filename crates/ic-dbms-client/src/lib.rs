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
//! - [`TableName`](crate::prelude::TableName)
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
//! TODO:
//!

#![doc(html_playground_url = "https://play.rust-lang.org")]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/veeso/ic-dbms/main/assets/images/cargo/logo-128.png"
)]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/veeso/ic-dbms/main/assets/images/cargo/logo-512.png"
)]

mod client;
pub mod prelude;

pub type IcDbmsCanisterClientResult<T> = Result<T, IcDbmCanisterClientError>;

#[derive(thiserror::Error, Debug)]
pub enum IcDbmCanisterClientError {
    #[error("IC DBMS Canister error: {0}")]
    Canister(#[from] ic_dbms_api::prelude::IcDbmsError),
}

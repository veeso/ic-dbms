//! This module exposes the data types used in the DBMS canister.

use candid::CandidType;
use serde::{Deserialize, Serialize};

use crate::dbms::value::Value;
use crate::memory::Encode;

mod blob;
mod boolean;
mod date;
mod datetime;
mod decimal;
mod integers;
mod nullable;
mod principal;
mod text;
mod uuid;

pub use self::blob::Blob;
pub use self::boolean::Boolean;
pub use self::date::Date;
pub use self::datetime::DateTime;
pub use self::decimal::Decimal;
pub use self::integers::{Int8, Int16, Int32, Int64, Uint8, Uint16, Uint32, Uint64};
pub use self::nullable::Nullable;
pub use self::principal::Principal;
pub use self::text::Text;
pub use self::uuid::Uuid;

/// A trait representing a data type that can be stored in the DBMS.
///
/// This is an umbrella trait that combines several other traits to ensure that
/// any type implementing [`DataType`] can be cloned, compared, hashed, encoded,
/// and serialized/deserialized using both Candid and Serde.
///
/// Also it is used by the DBMS to compare and sort values of different data types.
pub trait DataType:
    Clone
    + std::fmt::Debug
    + std::fmt::Display
    + PartialEq
    + Eq
    + Default
    + PartialOrd
    + Ord
    + std::hash::Hash
    + Encode
    + CandidType
    + Serialize
    + Into<Value>
    + for<'de> Deserialize<'de>
{
}

/// An enumeration of all supported data type kinds in the DBMS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DataTypeKind {
    Blob,
    Boolean,
    Date,
    DateTime,
    Decimal,
    Int32,
    Int64,
    Principal,
    Text,
    Uint32,
    Uint64,
    Uuid,
}

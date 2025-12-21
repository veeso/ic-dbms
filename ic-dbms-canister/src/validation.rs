//! This module contains all the built-in validations which can be applied to columns.
//!
//! Each validation function takes a [`&crate::prelude::Value`] as input and returns a `IcDbmsResult<()>` indicating
//! whether the value passes the validation or not.

use ic_dbms_api::prelude::IcDbmsResult;

mod case;
mod color;
mod email;
mod locale;
mod phone;
mod strlen;
mod web;

pub use self::case::{CamelCaseValidator, KebabCaseValidator, SnakeCaseValidator};
pub use self::color::RgbColorValidator;
pub use self::email::EmailValidator;
pub use self::locale::{CountryIso639Validator, CountryIso3166Validator};
pub use self::phone::PhoneNumberValidator;
pub use self::strlen::{MaxStrlenValidator, MinStrlenValidator, RangeStrlenValidator};
pub use self::web::{MimeTypeValidator, UrlValidator};

/// Trait for validating values.
pub trait Validate {
    /// Validates the given value.
    ///
    /// In case of error it should return a [`crate::prelude::IcDbmsError::Validation`] error.
    fn validate(&self, value: &crate::prelude::Value) -> IcDbmsResult<()>;
}

/// A validator that performs no validation.
pub struct NoValidation;

impl Validate for NoValidation {
    fn validate(&self, _value: &crate::prelude::Value) -> IcDbmsResult<()> {
        Ok(())
    }
}

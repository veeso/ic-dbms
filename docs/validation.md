# Column Validation

It is possible to add validation rules to table columns by using the `#[validate(ValidateImpl)]` attribute. This allows you to enforce constraints on the data being inserted or updated in the database.

## Syntax

The `#[validate(...)]` attribute can be added to any field in a struct that represents a database table. You can specify one validator by providing its name and any required parameters.

Such as:

- `#[validate(MaxStrlenValidator(255))]` - Validates that the string length does not exceed 255 characters.
- `#[validate(EmailValidator)]` - Validates that the string is a valid email address.

## Supported Validations

By default all these validators are available in `ic-dbms-api` prelude:

- `CamelCaseValidator`
- `CountryIso639Validator`
- `CountryIso3166Validator`
- `EmailValidator`
- `KebabCaseValidator`
- `MaxStrlenValidator`
- `MimeTypeValidator`
- `MinStrlenValidator`
- `PhoneNumberValidator`
- `RangeStrlenValidator`
- `RgbColorValidator`
- `SnakeCaseValidator`
- `UrlValidator`

## Example Usage

```rust
use ic_dbms_api::prelude::*;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    #[validate(MaxStrlenValidator(20))]
    pub name: Text,
    #[validate(EmailValidator)]
    pub email: Text,
}
```

## Implementing Custom Validators

In order to implement custom validators it is enough to create a struct that implements the `ic_dbms_canister::prelude::Validate` trait, with the following methods:

```rust
pub trait Validate {
    /// Validates the given value.
    ///
    /// In case of error it should return a [`crate::prelude::IcDbmsError::Validation`] error.
    fn validate(&self, value: &crate::prelude::Value) -> IcDbmsResult<()>;
}
```

Then just provide your validator struct in the `#[validate(...)]` attribute.

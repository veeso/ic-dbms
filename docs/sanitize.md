# Column Sanitization

It is possible to add sanitization rules to table columns by using the `#[sanitizer(SanitizeImpl)]` attribute. This allows you to enforce constraints on the data being inserted or updated in the database.

Note that sanitizers are always applied before validators. This means that the data will first be sanitized and then validated according to the specified validation rules.

> Unfortunately, I could not use `sanitize` as attribute name because of a conflict with nightly reserved keywords.

## Syntax

The `#[sanitizer(...)]` attribute can be added to any field in a struct that represents a database table. You can specify one sanitizer by providing its name and any required parameters.
Such as:

- `#[sanitizer(TrimSanitizer)]` - For **unit structs** sanitizers.
- `#[sanitizer(RoundToScaleSanitizer(2))]` - For **tuple structs** sanitizers.
- `#[sanitizer(ClampSanitizer, min = 0, max = 120)]` - For **named fields structs** sanitizers.

## Supported Validations

By default all these validators are available in `ic-dbms-api` prelude:

- `ClampSanitizer`
  - `ClampUnsignedSanitizer`
- `CollapseWhitespaceSanitizer`
- `LowerCaseSanitizer`
- `NullIfEmptySanitizer`
- `RoundToScaleSanitizer`
- `SlugSanitizer`
- `TimezoneSanitizer`
  - `UtcSanitizer`
- `TrimSanitizer`
- `UpperCaseSanitizer`
- `UrlEncodingSanitizer`

## Example Usage

```rust
use ic_dbms_api::prelude::*;

#[derive(Debug, Table, CandidType, Deserialize, Clone, PartialEq, Eq)]
#[table = "users"]
pub struct User {
    #[primary_key]
    pub id: Uint32,
    pub name: Text,
    #[sanitizer(TrimSanitizer)]
    pub email: Text,
    #[sanitizer(RoundToScaleSanitizer(2))]
    pub balance: Decimal,
}
```

## Implementing Custom Sanitizers

In order to implement custom sanitizers it is enough to create a struct that implements the `ic_dbms_canister::prelude::Sanitize` trait, with the following methods:

```rust
pub trait Sanitize {
    /// Sanitizes the given value.
    ///
    /// In case of error it should return a [`crate::prelude::IcDbmsError::Sanitize`] error.
    fn sanitize(&self, value: crate::prelude::Value) -> IcDbmsResult<crate::prelude::Value>;
}
```

Then just provide your sanitizer struct in the `#[sanitize(...)]` attribute.

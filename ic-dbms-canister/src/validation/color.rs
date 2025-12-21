use ic_dbms_api::prelude::Value;

use crate::prelude::Validate;

/// A validator for RGB color strings.
///
/// An RGB color string must be in the format `#RRGGBB`, where `RR`, `GG`, and `BB` are
/// two-digit hexadecimal numbers representing the red, green, and blue components of the color.
///
/// # Examples
///
/// ```rust
/// use ic_dbms_canister::prelude::{RgbColorValidator, Value, Validate};
///
/// let validator = RgbColorValidator;
/// let valid_color = Value::Text(ic_dbms_api::prelude::Text("#1A2B3C".into()));
/// assert!(validator.validate(&valid_color).is_ok());
/// ```
pub struct RgbColorValidator;

impl Validate for RgbColorValidator {
    fn validate(&self, value: &Value) -> ic_dbms_api::prelude::IcDbmsResult<()> {
        let Value::Text(text) = value else {
            return Err(ic_dbms_api::prelude::IcDbmsError::Validation(
                "RGB color validation requires a text value".to_string(),
            ));
        };

        let s = &text.0;
        if s.len() != 7 || !s.starts_with('#') {
            return Err(ic_dbms_api::prelude::IcDbmsError::Validation(
                "Invalid RGB color format".to_string(),
            ));
        }
        for c in s.chars().skip(1) {
            if !c.is_ascii_hexdigit() {
                return Err(ic_dbms_api::prelude::IcDbmsError::Validation(
                    "Invalid RGB color format".to_string(),
                ));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use ic_dbms_api::prelude::Value;

    use super::*;

    #[test]
    fn test_rgb_color_validator() {
        let validator = RgbColorValidator;

        // Valid RGB color
        let value = Value::Text(ic_dbms_api::prelude::Text("#1A2B3C".to_string()));
        assert!(validator.validate(&value).is_ok());

        // Invalid RGB color (wrong length)
        let value = Value::Text(ic_dbms_api::prelude::Text("#1A2B3".to_string()));
        assert!(validator.validate(&value).is_err());

        // Invalid RGB color (missing #)
        let value = Value::Text(ic_dbms_api::prelude::Text("1A2B3C".to_string()));
        assert!(validator.validate(&value).is_err());

        // Invalid RGB color (non-hex character)
        let value = Value::Text(ic_dbms_api::prelude::Text("#1A2B3G".to_string()));
        assert!(validator.validate(&value).is_err());

        // Invalid type
        let value = Value::Int32(123i32.into());
        assert!(validator.validate(&value).is_err());
    }
}

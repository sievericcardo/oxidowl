//! XSD Datatype Validation
//!
//! This module implements validation for XML Schema Datatypes (XSD) as specified
//! in the OWL 2 specification. It handles datatype checking and facet restrictions.

use crate::ontology::{IRI, Literal};
use crate::{Error, Result};
use std::str::FromStr;

/// XSD namespace
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

/// Validator for XSD datatypes
#[derive(Debug, Default)]
pub struct DatatypeValidator {
    /// Enable strict validation
    strict: bool,
}

impl DatatypeValidator {
    /// Create a new datatype validator
    #[must_use] 
    pub fn new() -> Self {
        Self { strict: true }
    }

    /// Create a validator with custom strictness
    #[must_use] 
    pub fn with_strict(strict: bool) -> Self {
        Self { strict }
    }

    /// Validate that a literal conforms to its declared datatype
    pub fn validate_literal(&self, literal: &Literal) -> Result<bool> {
        let datatype_url = match &literal.datatype {
            Some(url) => url,
            None => {
                // No datatype means xsd:string
                return Ok(true);
            }
        };

        let datatype_iri_str = datatype_url.as_str();
        let value = &literal.value;

        // Check if it's an XSD datatype
        if !datatype_iri_str.starts_with(XSD_NS) {
            // Non-XSD datatypes are accepted in non-strict mode
            return Ok(!self.strict);
        }

        let datatype_local = &datatype_iri_str[XSD_NS.len()..];

        match datatype_local {
            // String and related types
            "string" => Ok(true), // Any value is valid
            "normalizedString" => Self::validate_normalized_string(value),
            "token" => Self::validate_token(value),
            "language" => Self::validate_language(value),
            "Name" => Self::validate_name(value),
            "NCName" => Self::validate_ncname(value),
            "NMTOKEN" => Self::validate_nmtoken(value),

            // Boolean
            "boolean" => Self::validate_boolean(value),

            // Numeric types - Decimal
            "decimal" => Self::validate_decimal(value),
            "integer" => Self::validate_integer(value),
            "long" => Self::validate_long(value),
            "int" => Self::validate_int(value),
            "short" => Self::validate_short(value),
            "byte" => Self::validate_byte(value),
            "nonNegativeInteger" => Self::validate_non_negative_integer(value),
            "positiveInteger" => Self::validate_positive_integer(value),
            "nonPositiveInteger" => Self::validate_non_positive_integer(value),
            "negativeInteger" => Self::validate_negative_integer(value),
            "unsignedLong" => Self::validate_unsigned_long(value),
            "unsignedInt" => Self::validate_unsigned_int(value),
            "unsignedShort" => Self::validate_unsigned_short(value),
            "unsignedByte" => Self::validate_unsigned_byte(value),

            // Numeric types - Floating point
            "float" => Self::validate_float(value),
            "double" => Self::validate_double(value),

            // Date and time types
            "dateTime" => Self::validate_datetime(value),
            "dateTimeStamp" => Self::validate_datetime_stamp(value),
            "date" => Self::validate_date(value),
            "time" => Self::validate_time(value),
            "gYear" => Self::validate_gyear(value),
            "gYearMonth" => Self::validate_gyear_month(value),
            "gMonth" => Self::validate_gmonth(value),
            "gMonthDay" => Self::validate_gmonth_day(value),
            "gDay" => Self::validate_gday(value),
            "duration" => Self::validate_duration(value),

            // Binary types
            "hexBinary" => Self::validate_hex_binary(value),
            "base64Binary" => Self::validate_base64_binary(value),

            // URI type
            "anyURI" => Self::validate_any_uri(value),

            _ => {
                // Unknown datatype
                if self.strict {
                    Err(Error::invalid_input(format!(
                        "Unknown XSD datatype: {datatype_local}"
                    )))
                } else {
                    Ok(true)
                }
            }
        }
    }

    /// Validate a datatype match between two IRIs
    #[must_use] 
    pub fn datatypes_compatible(datatype1: &IRI, datatype2: &IRI) -> bool {
        if datatype1 == datatype2 {
            return true;
        }

        // Check for derived types
        Self::is_derived_type(datatype1, datatype2) || Self::is_derived_type(datatype2, datatype1)
    }

    /// Check if type1 is derived from type2
    fn is_derived_type(type1: &IRI, type2: &IRI) -> bool {
        let t1_str = type1.as_str();
        let t2_str = type2.as_str();

        if !t1_str.starts_with(XSD_NS) || !t2_str.starts_with(XSD_NS) {
            return false;
        }

        let local1 = &t1_str[XSD_NS.len()..];
        let local2 = &t2_str[XSD_NS.len()..];

        // Check derivation hierarchy
        match (local1, local2) {
            // Integer hierarchy
            (_, "decimal") if Self::is_integer_type(local1) => true,
            (_, "integer")
                if matches!(
                    local1,
                    "long"
                        | "int"
                        | "short"
                        | "byte"
                        | "nonNegativeInteger"
                        | "positiveInteger"
                        | "nonPositiveInteger"
                        | "negativeInteger"
                        | "unsignedLong"
                        | "unsignedInt"
                        | "unsignedShort"
                        | "unsignedByte"
                ) =>
            {
                true
            }
            ("int" | "short" | "byte", "long") => true,
            ("short" | "byte", "int") => true,
            ("byte", "short") => true,
            ("positiveInteger", "nonNegativeInteger") => true,
            ("negativeInteger", "nonPositiveInteger") => true,
            ("unsignedInt" | "unsignedShort" | "unsignedByte", "unsignedLong") => true,
            ("unsignedShort" | "unsignedByte", "unsignedInt") => true,
            ("unsignedByte", "unsignedShort") => true,

            // String hierarchy
            (
                "normalizedString" | "token" | "language" | "Name" | "NCName" | "NMTOKEN",
                "string",
            ) => true,
            ("token" | "language" | "Name" | "NCName" | "NMTOKEN", "normalizedString") => true,
            ("Name" | "NCName", "token") => true,
            ("NCName", "Name") => true,

            _ => false,
        }
    }

    fn is_integer_type(local_name: &str) -> bool {
        matches!(
            local_name,
            "integer"
                | "long"
                | "int"
                | "short"
                | "byte"
                | "nonNegativeInteger"
                | "positiveInteger"
                | "nonPositiveInteger"
                | "negativeInteger"
                | "unsignedLong"
                | "unsignedInt"
                | "unsignedShort"
                | "unsignedByte"
        )
    }

    // String type validators
    fn validate_normalized_string(value: &str) -> Result<bool> {
        // No carriage returns, line feeds, or tabs
        Ok(!value.chars().any(|c| c == '\r' || c == '\n' || c == '\t'))
    }

    fn validate_token(value: &str) -> Result<bool> {
        // Normalized, no leading/trailing spaces, no consecutive spaces
        if !Self::validate_normalized_string(value)? {
            return Ok(false);
        }
        Ok(!value.starts_with(' ') && !value.ends_with(' ') && !value.contains("  "))
    }

    fn validate_language(value: &str) -> Result<bool> {
        // RFC 3066 language tag pattern: [a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*
        let parts: Vec<&str> = value.split('-').collect();
        if parts.is_empty() {
            return Ok(false);
        }

        // First part must be letters only, 1-8 characters
        if !parts[0].chars().all(|c| c.is_ascii_alphabetic())
            || parts[0].len() > 8
            || parts[0].is_empty()
        {
            return Ok(false);
        }

        // Subsequent parts must be alphanumeric, 1-8 characters
        for part in &parts[1..] {
            if !part.chars().all(|c| c.is_ascii_alphanumeric()) || part.len() > 8 || part.is_empty()
            {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn validate_name(value: &str) -> Result<bool> {
        // XML Name: starts with letter/underscore/colon, then NameChar*
        if value.is_empty() {
            return Ok(false);
        }
        let first = value
            .chars()
            .next()
            .ok_or_else(|| Error::internal("Empty string after is_empty check"))?;
        Ok((first.is_alphabetic() || first == '_' || first == ':')
            && value
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.'))
    }

    fn validate_ncname(value: &str) -> Result<bool> {
        // Like Name but no colons
        if value.is_empty() {
            return Ok(false);
        }
        let first = value
            .chars()
            .next()
            .ok_or_else(|| Error::internal("Empty string after is_empty check"))?;
        Ok((first.is_alphabetic() || first == '_')
            && value
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
            && !value.contains(':'))
    }

    fn validate_nmtoken(value: &str) -> Result<bool> {
        // NameChar+
        Ok(!value.is_empty()
            && value
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.'))
    }

    // Boolean validator
    fn validate_boolean(value: &str) -> Result<bool> {
        Ok(matches!(value, "true" | "false" | "1" | "0"))
    }

    // Numeric validators
    fn validate_decimal(value: &str) -> Result<bool> {
        // Optional sign, digits, optional decimal point and more digits
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }

        let without_sign = trimmed
            .strip_prefix('+')
            .or_else(|| trimmed.strip_prefix('-'))
            .unwrap_or(trimmed);

        // Must have at least one digit
        if !without_sign.chars().any(|c| c.is_ascii_digit()) {
            return Ok(false);
        }

        // Can have at most one decimal point
        let decimal_count = without_sign.matches('.').count();
        if decimal_count > 1 {
            return Ok(false);
        }

        Ok(without_sign.chars().all(|c| c.is_ascii_digit() || c == '.'))
    }

    fn validate_integer(value: &str) -> Result<bool> {
        Ok(i128::from_str(value.trim()).is_ok())
    }

    fn validate_long(value: &str) -> Result<bool> {
        Ok(i64::from_str(value.trim()).is_ok())
    }

    fn validate_int(value: &str) -> Result<bool> {
        Ok(i32::from_str(value.trim()).is_ok())
    }

    fn validate_short(value: &str) -> Result<bool> {
        Ok(i16::from_str(value.trim()).is_ok())
    }

    fn validate_byte(value: &str) -> Result<bool> {
        Ok(i8::from_str(value.trim()).is_ok())
    }

    fn validate_non_negative_integer(value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n >= 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_positive_integer(value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n > 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_non_positive_integer(value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n <= 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_negative_integer(value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n < 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_unsigned_long(value: &str) -> Result<bool> {
        Ok(u64::from_str(value.trim()).is_ok())
    }

    fn validate_unsigned_int(value: &str) -> Result<bool> {
        Ok(u32::from_str(value.trim()).is_ok())
    }

    fn validate_unsigned_short(value: &str) -> Result<bool> {
        Ok(u16::from_str(value.trim()).is_ok())
    }

    fn validate_unsigned_byte(value: &str) -> Result<bool> {
        Ok(u8::from_str(value.trim()).is_ok())
    }

    fn validate_float(value: &str) -> Result<bool> {
        let trimmed = value.trim();
        Ok(f32::from_str(trimmed).is_ok() || matches!(trimmed, "INF" | "-INF" | "NaN"))
    }

    fn validate_double(value: &str) -> Result<bool> {
        let trimmed = value.trim();
        Ok(f64::from_str(trimmed).is_ok() || matches!(trimmed, "INF" | "-INF" | "NaN"))
    }

    // Date/Time validators (simplified - full ISO 8601 is complex)
    fn validate_datetime(value: &str) -> Result<bool> {
        // Basic format: YYYY-MM-DDTHH:MM:SS(.sss)?(Z|[+-]HH:MM)?
        let datetime_pattern = regex::Regex::new(
            r"^-?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$",
        )
        .expect("Valid hardcoded regex pattern for datetime");
        Ok(datetime_pattern.is_match(value))
    }

    fn validate_datetime_stamp(value: &str) -> Result<bool> {
        // Like dateTime but timezone is required
        let datetime_pattern = regex::Regex::new(
            r"^-?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$",
        )
        .expect("Valid hardcoded regex pattern for datetime stamp");
        Ok(datetime_pattern.is_match(value))
    }

    fn validate_date(value: &str) -> Result<bool> {
        // Format: YYYY-MM-DD(Z|[+-]HH:MM)?
        let date_pattern = regex::Regex::new(r"^-?\d{4}-\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for date");
        Ok(date_pattern.is_match(value))
    }

    fn validate_time(value: &str) -> Result<bool> {
        // Format: HH:MM:SS(.sss)?(Z|[+-]HH:MM)?
        let time_pattern = regex::Regex::new(r"^\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for time");
        Ok(time_pattern.is_match(value))
    }

    fn validate_gyear(value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^-?\d{4}(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for gYear");
        Ok(pattern.is_match(value))
    }

    fn validate_gyear_month(value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^-?\d{4}-\d{2}(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for gYearMonth");
        Ok(pattern.is_match(value))
    }

    fn validate_gmonth(value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^--\d{2}(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for gMonth");
        Ok(pattern.is_match(value))
    }

    fn validate_gmonth_day(value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^--\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for gMonthDay");
        Ok(pattern.is_match(value))
    }

    fn validate_gday(value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^---\d{2}(Z|[+-]\d{2}:\d{2})?$")
            .expect("Valid hardcoded regex pattern for gDay");
        Ok(pattern.is_match(value))
    }

    fn validate_duration(value: &str) -> Result<bool> {
        // Format: P(nY)?(nM)?(nD)?(T(nH)?(nM)?(nS)?)?
        let pattern =
            regex::Regex::new(r"^-?P(\d+Y)?(\d+M)?(\d+D)?(T(\d+H)?(\d+M)?(\d+(\.\d+)?S)?)?$")
                .expect("Valid hardcoded regex pattern for duration");
        Ok(pattern.is_match(value))
    }

    // Binary validators
    fn validate_hex_binary(value: &str) -> Result<bool> {
        Ok(value.chars().all(|c| c.is_ascii_hexdigit()) && value.len().is_multiple_of(2))
    }

    fn validate_base64_binary(value: &str) -> Result<bool> {
        // Remove whitespace
        let clean: String = value.chars().filter(|c| !c.is_whitespace()).collect();

        // Must be valid base64 characters
        let valid_chars = clean
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');

        // Padding must be correct
        let padding_ok = match clean.matches('=').count() {
            0 => true,
            1 => clean.ends_with('='),
            2 => clean.ends_with("=="),
            _ => false,
        };

        Ok(valid_chars && padding_ok && clean.len().is_multiple_of(4))
    }

    // URI validator
    fn validate_any_uri(value: &str) -> Result<bool> {
        // Basic URI validation - should be more comprehensive
        Ok(!value.is_empty() && !value.contains(char::is_whitespace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_boolean() {
        assert!(
            DatatypeValidator::validate_boolean("true")
                .expect("Failed to validate boolean datatype value")
        );
        assert!(
            DatatypeValidator::validate_boolean("false")
                .expect("Failed to validate boolean datatype value")
        );        assert!(
            DatatypeValidator::validate_boolean("1")
                .expect("Failed to validate boolean datatype value")
        );
        assert!(
            DatatypeValidator::validate_boolean("0")
                .expect("Failed to validate boolean datatype value")
        );
        assert!(
            !DatatypeValidator::validate_boolean("yes")
                .expect("Failed to validate boolean datatype value")
        );
    }

    #[test]
    fn test_validate_integer() {
        assert!(
            DatatypeValidator::validate_integer("123")
                .expect("Failed to validate integer datatype value")
        );
        assert!(
            DatatypeValidator::validate_integer("-456")
                .expect("Failed to validate integer datatype value")
        );
        assert!(
            DatatypeValidator::validate_integer("0")
                .expect("Failed to validate integer datatype value")
        );
        assert!(
            !DatatypeValidator::validate_integer("12.34")
                .expect("Failed to validate integer datatype value")
        );
        assert!(
            !DatatypeValidator::validate_integer("abc")
                .expect("Failed to validate integer datatype value")
        );
    }

    #[test]
    fn test_validate_decimal() {
        assert!(
            DatatypeValidator::validate_decimal("123.45")
                .expect("Failed to validate decimal datatype value")
        );
        assert!(
            DatatypeValidator::validate_decimal("-67.89")
                .expect("Failed to validate decimal datatype value")
        );
        assert!(
            DatatypeValidator::validate_decimal("100")
                .expect("Failed to validate decimal datatype value")
        );
        assert!(
            DatatypeValidator::validate_decimal(".5")
                .expect("Failed to validate decimal datatype value")
        );
        assert!(
            !DatatypeValidator::validate_decimal("12.34.56")
                .expect("Failed to validate decimal datatype value")
        );
    }

    #[test]
    fn test_datatype_hierarchy() {
        let int_iri = IRI::new("http://www.w3.org/2001/XMLSchema#int");
        let long_iri = IRI::new("http://www.w3.org/2001/XMLSchema#long");
        let integer_iri = IRI::new("http://www.w3.org/2001/XMLSchema#integer");
        let decimal_iri = IRI::new("http://www.w3.org/2001/XMLSchema#decimal");

        assert!(DatatypeValidator::datatypes_compatible(&int_iri, &long_iri));
        assert!(DatatypeValidator::datatypes_compatible(&int_iri, &integer_iri));
        assert!(DatatypeValidator::datatypes_compatible(&int_iri, &decimal_iri));
    }
}

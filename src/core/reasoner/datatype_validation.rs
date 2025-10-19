//! XSD Datatype Validation
//!
//! This module implements validation for XML Schema Datatypes (XSD) as specified
//! in the OWL 2 specification. It handles datatype checking and facet restrictions.

use crate::{Error, Result};
use crate::ontology::{Literal, IRI};
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
    pub fn new() -> Self {
        Self { strict: true }
    }

    /// Create a validator with custom strictness
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
            "normalizedString" => self.validate_normalized_string(value),
            "token" => self.validate_token(value),
            "language" => self.validate_language(value),
            "Name" => self.validate_name(value),
            "NCName" => self.validate_ncname(value),
            "NMTOKEN" => self.validate_nmtoken(value),

            // Boolean
            "boolean" => self.validate_boolean(value),

            // Numeric types - Decimal
            "decimal" => self.validate_decimal(value),
            "integer" => self.validate_integer(value),
            "long" => self.validate_long(value),
            "int" => self.validate_int(value),
            "short" => self.validate_short(value),
            "byte" => self.validate_byte(value),
            "nonNegativeInteger" => self.validate_non_negative_integer(value),
            "positiveInteger" => self.validate_positive_integer(value),
            "nonPositiveInteger" => self.validate_non_positive_integer(value),
            "negativeInteger" => self.validate_negative_integer(value),
            "unsignedLong" => self.validate_unsigned_long(value),
            "unsignedInt" => self.validate_unsigned_int(value),
            "unsignedShort" => self.validate_unsigned_short(value),
            "unsignedByte" => self.validate_unsigned_byte(value),

            // Numeric types - Floating point
            "float" => self.validate_float(value),
            "double" => self.validate_double(value),

            // Date and time types
            "dateTime" => self.validate_datetime(value),
            "dateTimeStamp" => self.validate_datetime_stamp(value),
            "date" => self.validate_date(value),
            "time" => self.validate_time(value),
            "gYear" => self.validate_gyear(value),
            "gYearMonth" => self.validate_gyear_month(value),
            "gMonth" => self.validate_gmonth(value),
            "gMonthDay" => self.validate_gmonth_day(value),
            "gDay" => self.validate_gday(value),
            "duration" => self.validate_duration(value),

            // Binary types
            "hexBinary" => self.validate_hex_binary(value),
            "base64Binary" => self.validate_base64_binary(value),

            // URI type
            "anyURI" => self.validate_any_uri(value),

            _ => {
                // Unknown datatype
                if self.strict {
                    Err(Error::invalid_input(format!("Unknown XSD datatype: {}", datatype_local)))
                } else {
                    Ok(true)
                }
            }
        }
    }

    /// Validate a datatype match between two IRIs
    pub fn datatypes_compatible(&self, datatype1: &IRI, datatype2: &IRI) -> bool {
        if datatype1 == datatype2 {
            return true;
        }

        // Check for derived types
        self.is_derived_type(datatype1, datatype2) || self.is_derived_type(datatype2, datatype1)
    }

    /// Check if type1 is derived from type2
    fn is_derived_type(&self, type1: &IRI, type2: &IRI) -> bool {
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
            (_, "decimal") if self.is_integer_type(local1) => true,
            (_, "integer") if matches!(local1,
                "long" | "int" | "short" | "byte" |
                "nonNegativeInteger" | "positiveInteger" |
                "nonPositiveInteger" | "negativeInteger" |
                "unsignedLong" | "unsignedInt" | "unsignedShort" | "unsignedByte") => true,
            ("int" | "short" | "byte", "long") => true,
            ("short" | "byte", "int") => true,
            ("byte", "short") => true,
            ("positiveInteger", "nonNegativeInteger") => true,
            ("negativeInteger", "nonPositiveInteger") => true,
            ("unsignedInt" | "unsignedShort" | "unsignedByte", "unsignedLong") => true,
            ("unsignedShort" | "unsignedByte", "unsignedInt") => true,
            ("unsignedByte", "unsignedShort") => true,

            // String hierarchy
            ("normalizedString" | "token" | "language" | "Name" | "NCName" | "NMTOKEN", "string") => true,
            ("token" | "language" | "Name" | "NCName" | "NMTOKEN", "normalizedString") => true,
            ("Name" | "NCName", "token") => true,
            ("NCName", "Name") => true,

            _ => false,
        }
    }

    fn is_integer_type(&self, local_name: &str) -> bool {
        matches!(local_name,
            "integer" | "long" | "int" | "short" | "byte" |
            "nonNegativeInteger" | "positiveInteger" |
            "nonPositiveInteger" | "negativeInteger" |
            "unsignedLong" | "unsignedInt" | "unsignedShort" | "unsignedByte")
    }

    // String type validators
    fn validate_normalized_string(&self, value: &str) -> Result<bool> {
        // No carriage returns, line feeds, or tabs
        Ok(!value.chars().any(|c| c == '\r' || c == '\n' || c == '\t'))
    }

    fn validate_token(&self, value: &str) -> Result<bool> {
        // Normalized, no leading/trailing spaces, no consecutive spaces
        if !self.validate_normalized_string(value)? {
            return Ok(false);
        }
        Ok(!value.starts_with(' ') &&
           !value.ends_with(' ') &&
           !value.contains("  "))
    }

    fn validate_language(&self, value: &str) -> Result<bool> {
        // RFC 3066 language tag pattern: [a-zA-Z]{1,8}(-[a-zA-Z0-9]{1,8})*
        let parts: Vec<&str> = value.split('-').collect();
        if parts.is_empty() {
            return Ok(false);
        }

        // First part must be letters only, 1-8 characters
        if !parts[0].chars().all(|c| c.is_ascii_alphabetic()) ||
           parts[0].len() > 8 || parts[0].is_empty() {
            return Ok(false);
        }

        // Subsequent parts must be alphanumeric, 1-8 characters
        for part in &parts[1..] {
            if !part.chars().all(|c| c.is_ascii_alphanumeric()) ||
               part.len() > 8 || part.is_empty() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    fn validate_name(&self, value: &str) -> Result<bool> {
        // XML Name: starts with letter/underscore/colon, then NameChar*
        if value.is_empty() {
            return Ok(false);
        }
        let first = value.chars().next().unwrap();
        Ok((first.is_alphabetic() || first == '_' || first == ':') &&
           value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.'))
    }

    fn validate_ncname(&self, value: &str) -> Result<bool> {
        // Like Name but no colons
        if value.is_empty() {
            return Ok(false);
        }
        let first = value.chars().next().unwrap();
        Ok((first.is_alphabetic() || first == '_') &&
           value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.') &&
           !value.contains(':'))
    }

    fn validate_nmtoken(&self, value: &str) -> Result<bool> {
        // NameChar+
        Ok(!value.is_empty() &&
           value.chars().all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.'))
    }

    // Boolean validator
    fn validate_boolean(&self, value: &str) -> Result<bool> {
        Ok(matches!(value, "true" | "false" | "1" | "0"))
    }

    // Numeric validators
    fn validate_decimal(&self, value: &str) -> Result<bool> {
        // Optional sign, digits, optional decimal point and more digits
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Ok(false);
        }

        let without_sign = trimmed.strip_prefix('+').or_else(|| trimmed.strip_prefix('-')).unwrap_or(trimmed);
        
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

    fn validate_integer(&self, value: &str) -> Result<bool> {
        Ok(i128::from_str(value.trim()).is_ok())
    }

    fn validate_long(&self, value: &str) -> Result<bool> {
        Ok(i64::from_str(value.trim()).is_ok())
    }

    fn validate_int(&self, value: &str) -> Result<bool> {
        Ok(i32::from_str(value.trim()).is_ok())
    }

    fn validate_short(&self, value: &str) -> Result<bool> {
        Ok(i16::from_str(value.trim()).is_ok())
    }

    fn validate_byte(&self, value: &str) -> Result<bool> {
        Ok(i8::from_str(value.trim()).is_ok())
    }

    fn validate_non_negative_integer(&self, value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n >= 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_positive_integer(&self, value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n > 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_non_positive_integer(&self, value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n <= 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_negative_integer(&self, value: &str) -> Result<bool> {
        match i128::from_str(value.trim()) {
            Ok(n) => Ok(n < 0),
            Err(_) => Ok(false),
        }
    }

    fn validate_unsigned_long(&self, value: &str) -> Result<bool> {
        Ok(u64::from_str(value.trim()).is_ok())
    }

    fn validate_unsigned_int(&self, value: &str) -> Result<bool> {
        Ok(u32::from_str(value.trim()).is_ok())
    }

    fn validate_unsigned_short(&self, value: &str) -> Result<bool> {
        Ok(u16::from_str(value.trim()).is_ok())
    }

    fn validate_unsigned_byte(&self, value: &str) -> Result<bool> {
        Ok(u8::from_str(value.trim()).is_ok())
    }

    fn validate_float(&self, value: &str) -> Result<bool> {
        let trimmed = value.trim();
        Ok(f32::from_str(trimmed).is_ok() ||
           matches!(trimmed, "INF" | "-INF" | "NaN"))
    }

    fn validate_double(&self, value: &str) -> Result<bool> {
        let trimmed = value.trim();
        Ok(f64::from_str(trimmed).is_ok() ||
           matches!(trimmed, "INF" | "-INF" | "NaN"))
    }

    // Date/Time validators (simplified - full ISO 8601 is complex)
    fn validate_datetime(&self, value: &str) -> Result<bool> {
        // Basic format: YYYY-MM-DDTHH:MM:SS(.sss)?(Z|[+-]HH:MM)?
        let datetime_pattern = regex::Regex::new(
            r"^-?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$"
        ).unwrap();
        Ok(datetime_pattern.is_match(value))
    }

    fn validate_datetime_stamp(&self, value: &str) -> Result<bool> {
        // Like dateTime but timezone is required
        let datetime_pattern = regex::Regex::new(
            r"^-?\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})$"
        ).unwrap();
        Ok(datetime_pattern.is_match(value))
    }

    fn validate_date(&self, value: &str) -> Result<bool> {
        // Format: YYYY-MM-DD(Z|[+-]HH:MM)?
        let date_pattern = regex::Regex::new(
            r"^-?\d{4}-\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$"
        ).unwrap();
        Ok(date_pattern.is_match(value))
    }

    fn validate_time(&self, value: &str) -> Result<bool> {
        // Format: HH:MM:SS(.sss)?(Z|[+-]HH:MM)?
        let time_pattern = regex::Regex::new(
            r"^\d{2}:\d{2}:\d{2}(\.\d+)?(Z|[+-]\d{2}:\d{2})?$"
        ).unwrap();
        Ok(time_pattern.is_match(value))
    }

    fn validate_gyear(&self, value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^-?\d{4}(Z|[+-]\d{2}:\d{2})?$").unwrap();
        Ok(pattern.is_match(value))
    }

    fn validate_gyear_month(&self, value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^-?\d{4}-\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
        Ok(pattern.is_match(value))
    }

    fn validate_gmonth(&self, value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^--\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
        Ok(pattern.is_match(value))
    }

    fn validate_gmonth_day(&self, value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^--\d{2}-\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
        Ok(pattern.is_match(value))
    }

    fn validate_gday(&self, value: &str) -> Result<bool> {
        let pattern = regex::Regex::new(r"^---\d{2}(Z|[+-]\d{2}:\d{2})?$").unwrap();
        Ok(pattern.is_match(value))
    }

    fn validate_duration(&self, value: &str) -> Result<bool> {
        // Format: P(nY)?(nM)?(nD)?(T(nH)?(nM)?(nS)?)?
        let pattern = regex::Regex::new(
            r"^-?P(\d+Y)?(\d+M)?(\d+D)?(T(\d+H)?(\d+M)?(\d+(\.\d+)?S)?)?$"
        ).unwrap();
        Ok(pattern.is_match(value))
    }

    // Binary validators
    fn validate_hex_binary(&self, value: &str) -> Result<bool> {
        Ok(value.chars().all(|c| c.is_ascii_hexdigit()) && value.len() % 2 == 0)
    }

    fn validate_base64_binary(&self, value: &str) -> Result<bool> {
        // Remove whitespace
        let clean: String = value.chars().filter(|c| !c.is_whitespace()).collect();
        
        // Must be valid base64 characters
        let valid_chars = clean.chars().all(|c|
            c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=');
        
        // Padding must be correct
        let padding_ok = match clean.matches('=').count() {
            0 => true,
            1 => clean.ends_with('='),
            2 => clean.ends_with("=="),
            _ => false,
        };
        
        Ok(valid_chars && padding_ok && clean.len() % 4 == 0)
    }

    // URI validator
    fn validate_any_uri(&self, value: &str) -> Result<bool> {
        // Basic URI validation - should be more comprehensive
        Ok(!value.is_empty() && !value.contains(char::is_whitespace))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_boolean() {
        let validator = DatatypeValidator::new();
        assert!(validator.validate_boolean("true").unwrap());
        assert!(validator.validate_boolean("false").unwrap());
        assert!(validator.validate_boolean("1").unwrap());
        assert!(validator.validate_boolean("0").unwrap());
        assert!(!validator.validate_boolean("yes").unwrap());
    }

    #[test]
    fn test_validate_integer() {
        let validator = DatatypeValidator::new();
        assert!(validator.validate_integer("123").unwrap());
        assert!(validator.validate_integer("-456").unwrap());
        assert!(validator.validate_integer("0").unwrap());
        assert!(!validator.validate_integer("12.34").unwrap());
        assert!(!validator.validate_integer("abc").unwrap());
    }

    #[test]
    fn test_validate_decimal() {
        let validator = DatatypeValidator::new();
        assert!(validator.validate_decimal("123.45").unwrap());
        assert!(validator.validate_decimal("-67.89").unwrap());
        assert!(validator.validate_decimal("100").unwrap());
        assert!(validator.validate_decimal(".5").unwrap());
        assert!(!validator.validate_decimal("12.34.56").unwrap());
    }

    #[test]
    fn test_datatype_hierarchy() {
        let validator = DatatypeValidator::new();
        let int_iri = IRI::new("http://www.w3.org/2001/XMLSchema#int");
        let long_iri = IRI::new("http://www.w3.org/2001/XMLSchema#long");
        let integer_iri = IRI::new("http://www.w3.org/2001/XMLSchema#integer");
        let decimal_iri = IRI::new("http://www.w3.org/2001/XMLSchema#decimal");
        
        assert!(validator.datatypes_compatible(&int_iri, &long_iri));
        assert!(validator.datatypes_compatible(&int_iri, &integer_iri));
        assert!(validator.datatypes_compatible(&int_iri, &decimal_iri));
    }
}

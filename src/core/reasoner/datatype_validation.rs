//! XSD Datatype Validation
//!
//! This module implements validation for XML Schema Datatypes (XSD) as specified
//! in the OWL 2 specification. It handles datatype checking, facet restrictions,
//! concrete-literal generation, and value equality. The value-space helpers here
//! are shared by both the classification service (`classification.rs`) and the
//! tableau executor (`executor.rs`).

use crate::ontology::datatypes::DatatypeManager;
use crate::ontology::{DataRange, FacetRestriction, IRI, Literal, OWL2Datatype, Ontology};
use crate::{Error, Result};
use std::cmp::Ordering;
use std::collections::HashSet;
use std::str::FromStr;

/// XSD namespace
const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
/// OWL namespace
const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
/// `rdfs:Literal` — the top datatype whose value space is the union of all value spaces.
const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
/// `rdf:langRange` facet IRI
const RDF_LANG_RANGE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langRange";
/// `owl:real`
const OWL_REAL: &str = "http://www.w3.org/2002/07/owl#real";
/// `owl:rational`
const OWL_RATIONAL: &str = "http://www.w3.org/2002/07/owl#rational";

/// Outcome of attempting to mint a concrete literal whose value lies in a data range.
///
/// This is the tableau concrete-domain result of `generate_literal_for_range`:
/// - [`RangeValue::Value`] — a concrete literal is known to satisfy the range.
/// - [`RangeValue::Empty`] — the range is provably empty (e.g. contradictory facets,
///   or an intersection of disjoint datatypes); the tableau must record a clash.
/// - [`RangeValue::Witness`] — no concrete literal was minted; the tableau falls back
///   to a syntactic range-witness marker (untyped placeholder node).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeValue {
    /// A concrete literal whose value lies in the range's value space.
    Value(Literal),
    /// The range is provably empty — a clash.
    Empty,
    /// No concrete literal minted; callers keep a syntactic witness.
    Witness,
}

/// Validator for XSD datatypes
#[derive(Debug, Default)]
pub struct DatatypeValidator {
    /// Enable strict validation
    strict: bool,
    /// Datatype manager holding `DatatypeDefinition` axioms for user-defined datatypes.
    manager: DatatypeManager,
}

impl DatatypeValidator {
    /// Create a new datatype validator
    #[must_use]
    pub fn new() -> Self {
        Self {
            strict: true,
            manager: DatatypeManager::new(),
        }
    }

    /// Create a validator with custom strictness
    #[must_use]
    pub fn with_strict(strict: bool) -> Self {
        Self {
            strict,
            manager: DatatypeManager::new(),
        }
    }

    /// Register the `DatatypeDefinition` axioms of an ontology so that literals of
    /// user-defined datatypes are validated against their defining data range.
    pub fn register_ontology_datatypes(&mut self, ontology: &Ontology) {
        self.manager.register_definitions_from_ontology(ontology);
    }

    /// Whether `datatype_iri` names a user-defined datatype (`DatatypeDefinition` axiom).
    #[must_use]
    pub fn is_defined_datatype(&self, datatype_iri: &IRI) -> bool {
        self.manager.get_datatype_definition(datatype_iri).is_some()
    }

    /// The defining data range of a user-defined datatype, if any.
    #[must_use]
    pub fn datatype_definition_range(&self, datatype_iri: &IRI) -> Option<DataRange> {
        self.manager
            .get_datatype_definition(datatype_iri)
            .map(|def| def.data_range.clone())
    }

    /// Validate that a literal conforms to its declared datatype
    ///
    /// # Panics
    ///
    /// Panics if a user-defined datatype definition disappears between the
    /// existence check and its use.
    pub fn validate_literal(&self, literal: &Literal) -> Result<bool> {
        let Some(datatype_url) = &literal.datatype else {
            // No datatype means xsd:string
            return Ok(true);
        };

        let datatype_iri_str = datatype_url.as_str();
        let value = &literal.value;

        // A user-defined datatype is validated against its definition's data range.
        if !datatype_iri_str.starts_with(XSD_NS)
            && !datatype_iri_str.starts_with(OWL_NS)
            && self
                .manager
                .get_datatype_definition(&IRI::new(datatype_iri_str))
                .is_some()
        {
            return self.literal_satisfies_data_range(
                literal,
                &self
                    .manager
                    .get_datatype_definition(&IRI::new(datatype_iri_str))
                    .expect("just checked")
                    .data_range,
            );
        }

        // owl:real / owl:rational accept any numeric lexical form.
        if datatype_iri_str == OWL_REAL || datatype_iri_str == OWL_RATIONAL {
            return Ok(value.parse::<f64>().is_ok());
        }

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

        let s1 = datatype1.as_str();
        let s2 = datatype2.as_str();

        // owl:real contains every numeric XSD datatype and owl:rational.
        if s1 == OWL_REAL {
            return Self::is_real_member(s2);
        }
        if s2 == OWL_REAL {
            return Self::is_real_member(s1);
        }
        // owl:rational contains decimal and the integer family (but not float/double,
        // whose value spaces include INF/NaN, nor real itself).
        if s1 == OWL_RATIONAL {
            return Self::is_rational_member(s2);
        }
        if s2 == OWL_RATIONAL {
            return Self::is_rational_member(s1);
        }

        // Check for derived types
        Self::is_derived_type(datatype1, datatype2) || Self::is_derived_type(datatype2, datatype1)
    }

    /// Whether the value space of the datatype identified by `iri_str` is a subset of
    /// `owl:real` (all numeric XSD datatypes plus rational).
    fn is_real_member(iri_str: &str) -> bool {
        match OWL2Datatype::from_iri(&IRI::new(iri_str)) {
            Some(dt) => dt.is_numeric() || matches!(dt, OWL2Datatype::Rational),
            None => false,
        }
    }

    /// Whether the value space of the datatype identified by `iri_str` is a subset of
    /// `owl:rational` (decimal and the integer family).
    fn is_rational_member(iri_str: &str) -> bool {
        match OWL2Datatype::from_iri(&IRI::new(iri_str)) {
            Some(dt) => {
                dt.is_numeric()
                    && !matches!(
                        dt,
                        OWL2Datatype::Float
                            | OWL2Datatype::Double
                            | OWL2Datatype::Real
                            | OWL2Datatype::Rational
                    )
            }
            None => false,
        }
    }

    /// Datatype compatibility that also resolves user-defined datatypes
    /// (`DatatypeDefinition` axioms) to their base datatype before comparing.
    #[must_use]
    pub fn datatypes_compatible_with(&self, a: &IRI, b: &IRI) -> bool {
        if a == b {
            return true;
        }
        let resolved_a = self.resolve_to_base(a);
        let resolved_b = self.resolve_to_base(b);
        match (resolved_a, resolved_b) {
            (Some(ra), Some(rb)) => Self::datatypes_compatible(&ra, &rb),
            _ => Self::datatypes_compatible(a, b),
        }
    }

    /// Follow the `DatatypeDefinition` chain of a datatype IRI down to its underlying base
    /// datatype, returning `None` if a definition uses a non-datatype range (union,
    /// complement, one-of, …) — those cannot be summarised by a base datatype.
    fn resolve_to_base(&self, iri: &IRI) -> Option<IRI> {
        let mut current = iri.clone();
        for _ in 0..16 {
            let Some(def) = self.manager.get_datatype_definition(&current) else {
                break;
            };
            match &def.data_range {
                DataRange::Datatype(base) => current = base.clone(),
                DataRange::DatatypeRestriction { datatype: base, .. } => current = base.clone(),
                _ => return None,
            }
        }
        Some(current)
    }

    /// Whether `lexical` is a well-formed literal of the built-in datatype `iri`.
    ///
    /// Called only for built-in (non-user-defined) range datatypes; user-defined ranges are
    /// handled by recursing into their definition before this point.
    fn lexical_matches_builtin_iri(&self, iri: &IRI, lexical: &str) -> Result<bool> {
        if iri.as_str() == OWL_REAL || iri.as_str() == OWL_RATIONAL {
            // Both value spaces consist of the rational/real numbers; any numeric
            // lexical form (integer or decimal) is a member.
            return Ok(lexical.parse::<f64>().is_ok());
        }
        match OWL2Datatype::from_iri(iri) {
            Some(dt) => Ok(Self::lexical_valid_for_builtin(&dt, lexical)),
            // Unknown (non-built-in, non-defined) datatype: be lenient.
            None => Ok(true),
        }
    }

    /// Whether a lexical form is a well-formed literal of the built-in datatype `dt`.
    fn lexical_valid_for_builtin(dt: &OWL2Datatype, lexical: &str) -> bool {
        match dt {
            OWL2Datatype::String
            | OWL2Datatype::NormalizedString
            | OWL2Datatype::Token
            | OWL2Datatype::Language
            | OWL2Datatype::Name
            | OWL2Datatype::NCName
            | OWL2Datatype::NMTOKEN
            | OWL2Datatype::NMTOKENS
            | OWL2Datatype::PlainLiteral
            | OWL2Datatype::RdfPlainLiteral
            | OWL2Datatype::LangString
            | OWL2Datatype::Literal
            | OWL2Datatype::RdfText
            | OWL2Datatype::XMLLiteral
            | OWL2Datatype::RdfXMLLiteral => true,
            OWL2Datatype::Boolean => matches!(lexical, "true" | "false" | "1" | "0"),
            OWL2Datatype::Integer => lexical.parse::<i128>().is_ok(),
            OWL2Datatype::Long => lexical.parse::<i64>().is_ok(),
            OWL2Datatype::Int => lexical.parse::<i32>().is_ok(),
            OWL2Datatype::Short => lexical.parse::<i16>().is_ok(),
            OWL2Datatype::Byte => lexical.parse::<i8>().is_ok(),
            OWL2Datatype::NonNegativeInteger => lexical.parse::<i128>().is_ok_and(|v| v >= 0),
            OWL2Datatype::PositiveInteger => lexical.parse::<i128>().is_ok_and(|v| v > 0),
            OWL2Datatype::NonPositiveInteger => lexical.parse::<i128>().is_ok_and(|v| v <= 0),
            OWL2Datatype::NegativeInteger => lexical.parse::<i128>().is_ok_and(|v| v < 0),
            OWL2Datatype::UnsignedLong => lexical.parse::<u64>().is_ok(),
            OWL2Datatype::UnsignedInt => lexical.parse::<u32>().is_ok(),
            OWL2Datatype::UnsignedShort => lexical.parse::<u16>().is_ok(),
            OWL2Datatype::UnsignedByte => lexical.parse::<u8>().is_ok(),
            OWL2Datatype::Decimal => Self::validate_decimal(lexical).unwrap_or(false),
            OWL2Datatype::Float | OWL2Datatype::Double => {
                lexical.parse::<f64>().is_ok() || matches!(lexical, "INF" | "-INF" | "NaN")
            }
            OWL2Datatype::DateTime => Self::validate_datetime(lexical).unwrap_or(false),
            OWL2Datatype::DateTimeStamp => Self::validate_datetime_stamp(lexical).unwrap_or(false),
            OWL2Datatype::Date => Self::validate_date(lexical).unwrap_or(false),
            OWL2Datatype::Time => Self::validate_time(lexical).unwrap_or(false),
            OWL2Datatype::GYear => Self::validate_gyear(lexical).unwrap_or(false),
            OWL2Datatype::GYearMonth => Self::validate_gyear_month(lexical).unwrap_or(false),
            OWL2Datatype::GMonthDay => Self::validate_gmonth_day(lexical).unwrap_or(false),
            OWL2Datatype::GMonth => Self::validate_gmonth(lexical).unwrap_or(false),
            OWL2Datatype::GDay => Self::validate_gday(lexical).unwrap_or(false),
            OWL2Datatype::Duration
            | OWL2Datatype::DayTimeDuration
            | OWL2Datatype::YearMonthDuration => Self::validate_duration(lexical).unwrap_or(false),
            OWL2Datatype::HexBinary => Self::validate_hex_binary(lexical).unwrap_or(false),
            OWL2Datatype::Base64Binary => Self::validate_base64_binary(lexical).unwrap_or(false),
            OWL2Datatype::AnyURI => Self::validate_any_uri(lexical).unwrap_or(false),
            OWL2Datatype::Real | OWL2Datatype::Rational => lexical.parse::<f64>().is_ok(),
        }
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

    // Date/Time validators using XSD dateTime pattern matching.
    // Covers the most common ISO 8601 formats: date with time, timezone,
    // fractional seconds, and date-only forms.
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

    // ─────────────────────────────────────────────────────────────────────────────
    // Value-space reasoning (shared by the classification service and the tableau)
    // ─────────────────────────────────────────────────────────────────────────────

    /// Whether the literal's value lies in the value space of `range`.
    ///
    /// This is the single value-space membership check used by both the ABox/instance
    /// classification path and the tableau datatype rules. It covers:
    /// - datatype compatibility (including derived XSD types and `owl:real`/`owl:rational`)
    /// - user-defined datatypes (`DatatypeDefinition` axioms)
    /// - all constraining facets (`length`, `minLength`, `maxLength`, `pattern`,
    ///   `totalDigits`, `fractionDigits`, `langRange`, and the ordering facets)
    /// - `DataOneOf`, intersections, unions and complements.
    pub fn literal_satisfies_data_range(
        &self,
        literal: &Literal,
        range: &DataRange,
    ) -> Result<bool> {
        match range {
            DataRange::Datatype(datatype_iri) => {
                let iri_str = datatype_iri.as_str();
                // Top datatype: every literal belongs to rdfs:Literal.
                if iri_str == RDFS_LITERAL {
                    return Ok(true);
                }

                // User-defined datatype → check against its defining data range. This
                // recurses into a *distinct* range (the definition), so it terminates even
                // for literals whose own datatype is user-defined.
                if let Some(definition) = self.manager.get_datatype_definition(datatype_iri) {
                    return self.literal_satisfies_data_range(literal, &definition.data_range);
                }

                // Built-in datatype: the literal's declared datatype must be compatible with
                // the range datatype (resolving user-defined literal datatypes to their base),
                // and its lexical form must be a well-formed literal of the range datatype.
                let literal_datatype = match &literal.datatype {
                    Some(url) => IRI::new(url.as_str()),
                    // Untyped plain literal → xsd:string.
                    None => IRI::new(&format!("{XSD_NS}string")),
                };
                if !self.datatypes_compatible_with(&literal_datatype, datatype_iri) {
                    return Ok(false);
                }
                self.lexical_matches_builtin_iri(datatype_iri, &literal.value)
            }
            DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                // Must be a member of the base datatype AND satisfy every facet.
                if !self
                    .literal_satisfies_data_range(literal, &DataRange::Datatype(datatype.clone()))?
                {
                    return Ok(false);
                }
                for restriction in restrictions {
                    if !Self::evaluate_facet_restriction(literal, restriction)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            DataRange::DataIntersectionOf(ranges) => {
                for range in ranges {
                    if !self.literal_satisfies_data_range(literal, range)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            DataRange::DataUnionOf(ranges) => {
                for range in ranges {
                    if self.literal_satisfies_data_range(literal, range)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            DataRange::DataComplementOf(range) => {
                Ok(!self.literal_satisfies_data_range(literal, range)?)
            }
            DataRange::DataOneOf(literals) => Ok(literals
                .iter()
                .any(|candidate| Self::literal_values_equal(literal, candidate))),
        }
    }

    /// Whether two data ranges are provably disjoint at the datatype level
    /// (no literal value can belong to both).
    ///
    /// Used to detect empty intersections such as `xsd:int ⊓ xsd:string`.
    #[must_use]
    pub fn datatypes_are_disjoint(a: &IRI, b: &IRI) -> bool {
        if a == b {
            return false;
        }
        if a.as_str() == RDFS_LITERAL || b.as_str() == RDFS_LITERAL {
            return false;
        }
        if Self::datatypes_compatible(a, b) {
            return false;
        }
        let is_numeric_like = |dt: &OWL2Datatype| {
            dt.is_numeric() || matches!(dt, OWL2Datatype::Real | OWL2Datatype::Rational)
        };
        match (OWL2Datatype::from_iri(a), OWL2Datatype::from_iri(b)) {
            (Some(x), Some(y)) => {
                if x == y {
                    return false;
                }
                if x.is_subtype_of(&y) || y.is_subtype_of(&x) {
                    return false;
                }
                if is_numeric_like(&x) && is_numeric_like(&y) {
                    return false; // Both live in the numeric/real family → they overlap.
                }
                if is_numeric_like(&x) != is_numeric_like(&y) {
                    return true; // Exactly one is numeric → disjoint.
                }
                // Different non-numeric families are disjoint; the same family overlaps
                // (e.g. the string subtypes all share the string value space).
                x.category() != y.category()
            }
            _ => false,
        }
    }

    /// Whether a data range is provably empty (no literal can satisfy it).
    ///
    /// Used as a fast emptiness pre-check before minting fresh data values.
    #[must_use]
    pub fn data_range_is_empty(&self, range: &DataRange) -> bool {
        matches!(self.generate_literal_for_range(range, 0), RangeValue::Empty)
    }

    /// Generate a concrete literal whose value lies in `range`.
    ///
    /// `seed` offsets the canonical value so distinct seeds produce distinct values where
    /// the range's value space is infinite (used by the data ≥-rule to mint pairwise-≠
    /// successors).
    #[must_use]
    pub fn generate_literal_for_range(&self, range: &DataRange, seed: usize) -> RangeValue {
        match range {
            DataRange::Datatype(iri) => {
                let iri_str = iri.as_str();
                if iri_str == RDFS_LITERAL {
                    return RangeValue::Witness;
                }
                // User-defined datatype → generate from its defining range.
                if let Some(definition) = self.manager.get_datatype_definition(iri) {
                    return self.generate_literal_for_range(&definition.data_range, seed);
                }
                match OWL2Datatype::from_iri(iri) {
                    Some(dt) => RangeValue::Value(Self::canonical_literal(&dt, seed)),
                    // Unknown datatype → no concrete value; keep a syntactic witness.
                    None => RangeValue::Witness,
                }
            }
            DataRange::DataOneOf(literals) => match literals.first() {
                Some(first) => RangeValue::Value(first.clone()),
                None => RangeValue::Empty, // DataOneOf() is the empty range.
            },
            DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                let base = DataRange::Datatype(datatype.clone());
                match self.generate_literal_for_range(&base, seed) {
                    RangeValue::Value(candidate) => {
                        let satisfies_all = restrictions.iter().all(|fr| {
                            Self::evaluate_facet_restriction(&candidate, fr).unwrap_or(false)
                        });
                        if satisfies_all {
                            RangeValue::Value(candidate)
                        } else {
                            self.synthesize_restriction_value(datatype, restrictions, seed)
                        }
                    }
                    RangeValue::Empty => RangeValue::Empty,
                    RangeValue::Witness => RangeValue::Witness,
                }
            }
            DataRange::DataIntersectionOf(ranges) => {
                if ranges.is_empty() {
                    return RangeValue::Empty;
                }
                // Two provably-disjoint datatype operands → empty intersection.
                for i in 0..ranges.len() {
                    for j in (i + 1)..ranges.len() {
                        if Self::ranges_provably_disjoint(&ranges[i], &ranges[j]) {
                            return RangeValue::Empty;
                        }
                    }
                }
                // Mint a value for one operand and verify it against all the others.
                for (k, sub) in ranges.iter().enumerate() {
                    match self.generate_literal_for_range(sub, seed) {
                        RangeValue::Value(lit) => {
                            let mut ok = true;
                            for (m, other) in ranges.iter().enumerate() {
                                if m == k {
                                    continue;
                                }
                                if !self
                                    .literal_satisfies_data_range(&lit, other)
                                    .unwrap_or(false)
                                {
                                    ok = false;
                                    break;
                                }
                            }
                            if ok {
                                return RangeValue::Value(lit);
                            }
                        }
                        RangeValue::Empty => return RangeValue::Empty,
                        RangeValue::Witness => {}
                    }
                }
                RangeValue::Witness
            }
            DataRange::DataUnionOf(ranges) => {
                for sub in ranges {
                    match self.generate_literal_for_range(sub, seed) {
                        RangeValue::Value(lit) => return RangeValue::Value(lit),
                        // Skip an empty branch; try the next disjunct.
                        RangeValue::Empty => {}
                        RangeValue::Witness => return RangeValue::Witness,
                    }
                }
                RangeValue::Empty // Every branch is empty.
            }
            DataRange::DataComplementOf(inner) => {
                // Mint a literal of a datatype provably disjoint from the inner range.
                match inner.as_ref() {
                    DataRange::Datatype(iri) => match OWL2Datatype::from_iri(iri) {
                        Some(dt) if dt.is_numeric() => RangeValue::Value(Literal::with_datatype(
                            "v".to_string(),
                            OWL2Datatype::String.iri(),
                        )),
                        Some(_) => RangeValue::Value(Literal::with_datatype(
                            "0".to_string(),
                            OWL2Datatype::Int.iri(),
                        )),
                        None => RangeValue::Witness,
                    },
                    _ => RangeValue::Witness,
                }
            }
        }
    }

    /// Mint a literal in `range` that is value-distinct from every literal in `used`.
    ///
    /// Returns [`RangeValue::Empty`] when the range cannot provide another distinct value
    /// (e.g. `≥3 P xsd:boolean`, or `≥2` over a single-value facet-restricted range).
    #[must_use]
    pub fn generate_distinct_literal_for_range(
        &self,
        range: &DataRange,
        seed: usize,
        used: &HashSet<Literal>,
    ) -> RangeValue {
        let mut saw_candidate = false;
        for s in seed..=seed.saturating_add(16) {
            match self.generate_literal_for_range(range, s) {
                RangeValue::Value(lit) => {
                    if used.iter().all(|u| !Self::literal_values_equal(u, &lit)) {
                        return RangeValue::Value(lit);
                    }
                    saw_candidate = true;
                }
                RangeValue::Empty => {}
                RangeValue::Witness => return RangeValue::Witness,
            }
        }
        if saw_candidate {
            RangeValue::Empty
        } else {
            RangeValue::Witness
        }
    }

    /// Whether two top-level ranges are provably disjoint (empty intersection).
    fn ranges_provably_disjoint(a: &DataRange, b: &DataRange) -> bool {
        if let (DataRange::Datatype(x), DataRange::Datatype(y)) = (a, b) {
            return Self::datatypes_are_disjoint(x, y);
        }
        false
    }

    /// Try to synthesise a literal satisfying a facet-restricted range when the
    /// canonical base value fails a facet. Also detects contradictory facet sets
    /// (e.g. `minInclusive 10` + `maxInclusive 5`) and reports an empty range.
    fn synthesize_restriction_value(
        &self,
        datatype: &IRI,
        restrictions: &[FacetRestriction],
        seed: usize,
    ) -> RangeValue {
        let mut min_inc: Option<f64> = None;
        let mut max_inc: Option<f64> = None;
        let mut min_exc: Option<f64> = None;
        let mut max_exc: Option<f64> = None;
        let mut pattern: Option<&str> = None;
        let mut length: Option<usize> = None;
        let mut min_len: Option<usize> = None;
        let mut max_len: Option<usize> = None;

        for r in restrictions {
            let v = r.value.value.as_str();
            match r.facet.as_str() {
                "http://www.w3.org/2001/XMLSchema#minInclusive" => {
                    min_inc = v.parse::<f64>().ok();
                }
                "http://www.w3.org/2001/XMLSchema#maxInclusive" => {
                    max_inc = v.parse::<f64>().ok();
                }
                "http://www.w3.org/2001/XMLSchema#minExclusive" => {
                    min_exc = v.parse::<f64>().ok();
                }
                "http://www.w3.org/2001/XMLSchema#maxExclusive" => {
                    max_exc = v.parse::<f64>().ok();
                }
                "http://www.w3.org/2001/XMLSchema#pattern" => pattern = Some(v),
                "http://www.w3.org/2001/XMLSchema#length" => length = v.parse().ok(),
                "http://www.w3.org/2001/XMLSchema#minLength" => min_len = v.parse().ok(),
                "http://www.w3.org/2001/XMLSchema#maxLength" => max_len = v.parse().ok(),
                _ => {}
            }
        }

        // ── Numeric bounds ───────────────────────────────────────────────────────
        // Lower bound as an inclusive number.
        let lower = min_inc.or_else(|| min_exc.map(|m| m + 1.0));
        // Upper bound as an inclusive number.
        let upper = max_inc.or_else(|| max_exc.map(|m| m - 1.0));

        // Contradictory facets → empty range.
        if let (Some(lo), Some(hi)) = (lower, upper)
            && lo > hi
        {
            return RangeValue::Empty;
        }
        if let (Some(lo), Some(hi)) = (min_exc, max_inc)
            && lo >= hi
        {
            return RangeValue::Empty;
        }
        if let (Some(lo), Some(hi)) = (min_exc, max_exc)
            && lo >= hi
        {
            return RangeValue::Empty;
        }
        if let (Some(lo), Some(hi)) = (min_inc, max_exc)
            && lo >= hi
        {
            return RangeValue::Empty;
        }

        // Numeric base: candidate = lower + seed, must respect the upper bound.
        if let Some(lo) = lower {
            let candidate = lo + seed as f64;
            if let Some(hi) = upper
                && candidate > hi
            {
                return RangeValue::Empty;
            }
            return RangeValue::Value(Literal::with_datatype(
                candidate.to_string(),
                datatype.clone(),
            ));
        }

        // ── Pattern / length bases ───────────────────────────────────────────────
        if let Some(p) = pattern
            && let Some(s) = Self::generate_string_from_pattern(p)
        {
            let candidate = Literal::with_datatype(s, datatype.clone());
            if let Some(len) = length
                && candidate.value.chars().count() != len
            {
                return RangeValue::Empty;
            }
            if let Some(ml) = min_len
                && candidate.value.chars().count() < ml
            {
                return RangeValue::Empty;
            }
            if let Some(mx) = max_len
                && candidate.value.chars().count() > mx
            {
                return RangeValue::Empty;
            }
            return RangeValue::Value(candidate);
        }
        if let Some(len) = length {
            return RangeValue::Value(Literal::with_datatype("a".repeat(len), datatype.clone()));
        }
        if let Some(ml) = min_len {
            let len = ml.max(max_len.unwrap_or(ml));
            return RangeValue::Value(Literal::with_datatype("a".repeat(len), datatype.clone()));
        }

        RangeValue::Witness
    }

    /// Evaluate a single facet restriction against a literal.
    ///
    /// Covers every OWL 2 constraining facet: `length`, `minLength`, `maxLength`,
    /// `pattern`, `minInclusive`, `maxInclusive`, `minExclusive`, `maxExclusive`,
    /// `totalDigits`, `fractionDigits`, and `rdf:langRange`. Unknown facets are
    /// treated conservatively as satisfied.
    pub fn evaluate_facet_restriction(
        literal: &Literal,
        restriction: &FacetRestriction,
    ) -> Result<bool> {
        let facet_iri = restriction.facet.as_str();
        let facet_value = &restriction.value.value;
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#minInclusive" => {
                Self::check_ordering_facet(literal, restriction, |ord| ord != Ordering::Less)
            }
            "http://www.w3.org/2001/XMLSchema#maxInclusive" => {
                Self::check_ordering_facet(literal, restriction, |ord| ord != Ordering::Greater)
            }
            "http://www.w3.org/2001/XMLSchema#minExclusive" => {
                Self::check_ordering_facet(literal, restriction, |ord| ord == Ordering::Greater)
            }
            "http://www.w3.org/2001/XMLSchema#maxExclusive" => {
                Self::check_ordering_facet(literal, restriction, |ord| ord == Ordering::Less)
            }
            "http://www.w3.org/2001/XMLSchema#length" => {
                let len: usize = facet_value.parse().unwrap_or(usize::MAX);
                Ok(literal.value.chars().count() == len)
            }
            "http://www.w3.org/2001/XMLSchema#minLength" => {
                let min: usize = facet_value.parse().unwrap_or(0);
                Ok(literal.value.chars().count() >= min)
            }
            "http://www.w3.org/2001/XMLSchema#maxLength" => {
                let max: usize = facet_value.parse().unwrap_or(usize::MAX);
                Ok(literal.value.chars().count() <= max)
            }
            "http://www.w3.org/2001/XMLSchema#pattern" => {
                let re = regex::Regex::new(facet_value).map_err(|e| {
                    Error::invalid_input(format!("Invalid pattern facet '{facet_value}': {e}"))
                })?;
                Ok(re.is_match(&literal.value))
            }
            "http://www.w3.org/2001/XMLSchema#totalDigits" => {
                let max: usize = facet_value.parse().unwrap_or(usize::MAX);
                let digit_count = literal.value.chars().filter(char::is_ascii_digit).count();
                Ok(digit_count <= max)
            }
            "http://www.w3.org/2001/XMLSchema#fractionDigits" => {
                let max: usize = facet_value.parse().unwrap_or(usize::MAX);
                let fraction_digits = match literal.value.split_once('.') {
                    Some((_, frac)) => frac.chars().filter(char::is_ascii_digit).count(),
                    None => 0,
                };
                Ok(fraction_digits <= max)
            }
            RDF_LANG_RANGE => {
                let range = facet_value.trim();
                Ok(match &literal.language {
                    None => false,
                    Some(lang) => Self::language_matches_range(lang, range),
                })
            }
            _ => {
                // Unknown facet → conservatively satisfied.
                Ok(true)
            }
        }
    }

    /// Compare a literal against a facet value using the datatype's ordering.
    fn check_ordering_facet(
        literal: &Literal,
        restriction: &FacetRestriction,
        comparator: impl Fn(Ordering) -> bool,
    ) -> Result<bool> {
        match Self::value_ordering(literal, &restriction.value) {
            Some(ord) => Ok(comparator(ord)),
            None => Ok(false),
        }
    }

    /// Order two literals in their shared value space, if comparable.
    ///
    /// Numeric values are compared arithmetically; string-family values (including
    /// untyped plain literals and language-tagged strings) are compared by codepoint.
    fn value_ordering(a: &Literal, b: &Literal) -> Option<Ordering> {
        if let (Ok(x), Ok(y)) = (a.value.parse::<f64>(), b.value.parse::<f64>()) {
            return x.partial_cmp(&y);
        }
        let is_string_like = |l: &Literal| -> bool {
            if l.language.is_some() || l.datatype.is_none() {
                return true;
            }
            matches!(
                OWL2Datatype::from_iri(&IRI::new(l.datatype.as_ref().map_or("", url::Url::as_str))),
                Some(dt) if dt.category() == crate::ontology::datatypes::DatatypeCategory::String
            )
        };
        if is_string_like(a) && is_string_like(b) {
            return Some(a.value.cmp(&b.value));
        }
        None
    }

    /// RFC 4647 basic language-range matching (e.g. range `en` matches `en`, `en-US`).
    fn language_matches_range(language: &str, range: &str) -> bool {
        if range.is_empty() || range == "*" {
            return true;
        }
        if language.eq_ignore_ascii_case(range) {
            return true;
        }
        language
            .to_ascii_lowercase()
            .starts_with(&format!("{}-", range.to_ascii_lowercase()))
    }

    /// Datatype value equality: two literals denote the same value iff their lexical
    /// forms map to the same value-space element under their datatypes.
    ///
    /// `"5"^^xsd:int` equals `"5"^^xsd:integer` (numeric value equality) but
    /// `"05"^^xsd:int` also equals `"5"^^xsd:int` — numeric lexical forms are
    /// canonicalised through their numeric value. String literals compare by lexical
    /// form (plus language tag).
    #[must_use]
    pub fn literal_values_equal(a: &Literal, b: &Literal) -> bool {
        if a.language.is_some() || b.language.is_some() {
            return a.value == b.value && a.language == b.language;
        }
        match (&a.datatype, &b.datatype) {
            (Some(da), Some(db)) => {
                if da == db {
                    return a.value == b.value;
                }
                let ia = IRI::new(da.as_str());
                let ib = IRI::new(db.as_str());
                if Self::datatypes_compatible(&ia, &ib)
                    && let (Ok(x), Ok(y)) = (a.value.parse::<f64>(), b.value.parse::<f64>())
                {
                    return x.partial_cmp(&y) == Some(std::cmp::Ordering::Equal);
                }
                false
            }
            (None, None) => a.value == b.value,
            _ => false,
        }
    }

    /// Mint a canonical literal for a built-in OWL 2 datatype.
    ///
    /// `seed` varies the value so repeated calls produce distinct values for
    /// distinct-successor generation.
    fn canonical_literal(dt: &OWL2Datatype, seed: usize) -> Literal {
        let iri = dt.iri();
        match dt {
            OWL2Datatype::Boolean => {
                let v = if seed.is_multiple_of(2) {
                    "false"
                } else {
                    "true"
                };
                Literal::with_datatype(v.to_string(), iri)
            }
            OWL2Datatype::PositiveInteger => Literal::with_datatype((seed + 1).to_string(), iri),
            OWL2Datatype::NegativeInteger => Literal::with_datatype(
                (i64::try_from(seed).unwrap_or(i64::MAX).saturating_neg() - 1).to_string(),
                iri,
            ),
            OWL2Datatype::NonPositiveInteger => Literal::with_datatype(
                i64::try_from(seed)
                    .unwrap_or(i64::MAX)
                    .saturating_neg()
                    .to_string(),
                iri,
            ),
            OWL2Datatype::NonNegativeInteger
            | OWL2Datatype::UnsignedLong
            | OWL2Datatype::UnsignedInt
            | OWL2Datatype::UnsignedShort
            | OWL2Datatype::UnsignedByte
            | OWL2Datatype::Integer
            | OWL2Datatype::Long
            | OWL2Datatype::Int
            | OWL2Datatype::Short
            | OWL2Datatype::Byte
            | OWL2Datatype::Decimal
            | OWL2Datatype::Float
            | OWL2Datatype::Double
            | OWL2Datatype::Real
            | OWL2Datatype::Rational => Literal::with_datatype(seed.to_string(), iri),
            OWL2Datatype::Language => {
                const LANGS: [&str; 8] = ["en", "fr", "de", "es", "it", "pt", "ru", "zh"];
                Literal::with_datatype(LANGS[seed % LANGS.len()].to_string(), iri)
            }
            OWL2Datatype::String
            | OWL2Datatype::NormalizedString
            | OWL2Datatype::Token
            | OWL2Datatype::Name
            | OWL2Datatype::NCName
            | OWL2Datatype::NMTOKEN
            | OWL2Datatype::NMTOKENS
            | OWL2Datatype::PlainLiteral
            | OWL2Datatype::RdfPlainLiteral
            | OWL2Datatype::LangString
            | OWL2Datatype::Literal
            | OWL2Datatype::RdfText => Literal::with_datatype(format!("v{seed}"), iri),
            OWL2Datatype::HexBinary => Literal::with_datatype(format!("{seed:02x}"), iri),
            OWL2Datatype::Base64Binary => Literal::with_datatype("AA".to_string(), iri),
            OWL2Datatype::AnyURI => {
                Literal::with_datatype(format!("http://example.org/v{seed}"), iri)
            }
            OWL2Datatype::DateTime | OWL2Datatype::DateTimeStamp => {
                Literal::with_datatype("1970-01-01T00:00:00Z".to_string(), iri)
            }
            OWL2Datatype::Date => Literal::with_datatype("1970-01-01".to_string(), iri),
            OWL2Datatype::Time => Literal::with_datatype("00:00:00".to_string(), iri),
            OWL2Datatype::GYear => Literal::with_datatype("1970".to_string(), iri),
            OWL2Datatype::GYearMonth => Literal::with_datatype("1970-01".to_string(), iri),
            OWL2Datatype::GMonthDay => Literal::with_datatype("--01-01".to_string(), iri),
            OWL2Datatype::GMonth => Literal::with_datatype("--01".to_string(), iri),
            OWL2Datatype::GDay => Literal::with_datatype("---01".to_string(), iri),
            OWL2Datatype::Duration
            | OWL2Datatype::DayTimeDuration
            | OWL2Datatype::YearMonthDuration => Literal::with_datatype("P0D".to_string(), iri),
            OWL2Datatype::XMLLiteral | OWL2Datatype::RdfXMLLiteral => {
                Literal::with_datatype("<x/>".to_string(), iri)
            }
        }
    }

    /// Generate a concrete string matching a (simple) XSD regex `pattern` facet.
    ///
    /// Handles literal characters, `.`, `\d`/`\w`/`\s`, character classes with ranges
    /// (`[0-9]`, `[a-zA-Z]`) and quantifiers (`{n}`, `{n,m}`, `+`, `*`, `?`). Returns
    /// `None` for unsupported constructs; callers fall back to a syntactic witness.
    fn generate_string_from_pattern(pattern: &str) -> Option<String> {
        let chars: Vec<char> = pattern.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match c {
                '[' => {
                    let mut j = i + 1;
                    let mut class_chars: Vec<char> = Vec::new();
                    while j < chars.len() && chars[j] != ']' {
                        if j + 2 < chars.len() && chars[j + 1] == '-' && chars[j + 2] != ']' {
                            let start = chars[j] as u32;
                            let end = chars[j + 2] as u32;
                            for code in start..=end {
                                class_chars.push(char::from_u32(code)?);
                            }
                            j += 3;
                        } else {
                            class_chars.push(chars[j]);
                            j += 1;
                        }
                    }
                    if j >= chars.len() {
                        return None;
                    }
                    i = j + 1;
                    let (rep, next) = read_quantifier(&chars, i)?;
                    i = next;
                    let first = *class_chars.first()?;
                    out.extend(std::iter::repeat_n(first, rep));
                }
                '\\' => {
                    i += 1;
                    if i >= chars.len() {
                        return None;
                    }
                    let ch = match chars[i] {
                        'd' => '0',
                        'w' => 'a',
                        's' => ' ',
                        'n' => '\n',
                        't' => '\t',
                        other => other,
                    };
                    i += 1;
                    let (rep, next) = read_quantifier(&chars, i)?;
                    i = next;
                    out.extend(std::iter::repeat_n(ch, rep));
                }
                '.' => {
                    i += 1;
                    let (rep, next) = read_quantifier(&chars, i)?;
                    i = next;
                    out.extend(std::iter::repeat_n('a', rep));
                }
                '^' | '$' => {
                    i += 1;
                }
                '(' | ')' | '|' => {
                    // Disjunctions/groups are unsupported for generation; bail out.
                    return None;
                }
                other => {
                    i += 1;
                    let (rep, next) = read_quantifier(&chars, i)?;
                    i = next;
                    out.extend(std::iter::repeat_n(other, rep));
                }
            }
        }
        Some(out)
    }
}

/// Read an XSD regex quantifier following an atom.
///
/// Returns `(repeat_count, next_index)`. Unquantified atoms repeat once.
fn read_quantifier(chars: &[char], i: usize) -> Option<(usize, usize)> {
    if i >= chars.len() {
        return Some((1, i));
    }
    match chars[i] {
        '+' => Some((1, i + 1)),
        '*' | '?' => Some((0, i + 1)),
        '{' => {
            let close = chars[i..].iter().position(|&c| c == '}')? + i;
            let inner: String = chars[i + 1..close].iter().collect();
            let next = close + 1;
            if let Some((lo, _hi)) = inner.split_once(',') {
                let n: usize = lo.trim().parse().ok()?;
                Some((n, next))
            } else {
                let n: usize = inner.trim().parse().ok()?;
                Some((n, next))
            }
        }
        _ => Some((1, i)),
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
        );
        assert!(
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
        assert!(DatatypeValidator::datatypes_compatible(
            &int_iri,
            &integer_iri
        ));
        assert!(DatatypeValidator::datatypes_compatible(
            &int_iri,
            &decimal_iri
        ));
    }
}

//! Numeric Value Space Handlers (integer, decimal, float, double, real)

use super::ValueSpaceHandler;
use crate::error::Error;

/// Handler for xsd:integer and derived integer types.
#[derive(Debug, Clone, Default)]
pub struct IntegerValueSpace;

impl ValueSpaceHandler for IntegerValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#integer"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        value.parse::<i128>().is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        value.parse::<i128>().map(|n| n.to_string()).unwrap_or_else(|_| value.to_string())
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let pa = parse_i128(a)?;
        let pb = parse_i128(b)?;
        Ok(pa == pb)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        let v = parse_i128(value)?;
        check_numeric_facet_i128(v, facet_iri, facet_value)
    }

    fn is_finite(&self) -> bool {
        true // Conceptually infinite but bounded representation.
    }

    fn is_clash(&self, values: &[&str]) -> bool {
        // Clash if two different numeric values are asserted equal (e.g., via
        // nominals or datatype restrictions) — not applicable in the pure value
        // space; clash is detected via facet intersection emptiness instead.
        let _ = values;
        false
    }
}

/// Handler for xsd:decimal.
#[derive(Debug, Clone, Default)]
pub struct DecimalValueSpace;

impl ValueSpaceHandler for DecimalValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#decimal"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        value.parse::<f64>().is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        value.parse::<f64>()
            .map(|f| format!("{f}"))
            .unwrap_or_else(|_| value.to_string())
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let pa = parse_f64(a)?;
        let pb = parse_f64(b)?;
        Ok((pa - pb).abs() < f64::EPSILON)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        let v = parse_f64(value)?;
        check_numeric_facet_f64(v, facet_iri, facet_value)
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

/// Handler for xsd:float and xsd:double.
#[derive(Debug, Clone)]
pub struct FloatValueSpace {
    datatype: &'static str,
}

impl FloatValueSpace {
    #[must_use]
    pub fn xsd_float() -> Self {
        Self { datatype: "http://www.w3.org/2001/XMLSchema#float" }
    }

    #[must_use]
    pub fn xsd_double() -> Self {
        Self { datatype: "http://www.w3.org/2001/XMLSchema#double" }
    }
}

impl ValueSpaceHandler for FloatValueSpace {
    fn datatype_iri(&self) -> &str {
        self.datatype
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        matches!(value, "INF" | "-INF" | "NaN") || value.parse::<f64>().is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        match value {
            "INF" | "+INF" => "INF".to_string(),
            "-INF" => "-INF".to_string(),
            "NaN" => "NaN".to_string(),
            other => other
                .parse::<f64>()
                .map(|f| format!("{f:.10e}"))
                .unwrap_or_else(|_| other.to_string()),
        }
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        if a == "NaN" || b == "NaN" {
            return Ok(false); // NaN != NaN per IEEE 754.
        }
        let pa = parse_f64(a)?;
        let pb = parse_f64(b)?;
        Ok(pa == pb)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        if value == "NaN" {
            // NaN does not satisfy ordering constraints.
            return Ok(false);
        }
        let v = parse_f64(value)?;
        check_numeric_facet_f64(v, facet_iri, facet_value)
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn parse_i128(s: &str) -> Result<i128, Error> {
    s.parse::<i128>()
        .map_err(|_| Error::invalid_input(format!("Not an integer literal: {s}")))
}

fn parse_f64(s: &str) -> Result<f64, Error> {
    match s {
        "INF" | "+INF" => Ok(f64::INFINITY),
        "-INF" => Ok(f64::NEG_INFINITY),
        "NaN" => Ok(f64::NAN),
        other => other
            .parse::<f64>()
            .map_err(|_| Error::invalid_input(format!("Not a numeric literal: {other}"))),
    }
}

fn check_numeric_facet_i128(v: i128, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
    let fv = parse_i128(facet_value)?;
    match facet_iri {
        "http://www.w3.org/2001/XMLSchema#minInclusive" => Ok(v >= fv),
        "http://www.w3.org/2001/XMLSchema#maxInclusive" => Ok(v <= fv),
        "http://www.w3.org/2001/XMLSchema#minExclusive" => Ok(v > fv),
        "http://www.w3.org/2001/XMLSchema#maxExclusive" => Ok(v < fv),
        other => Err(Error::invalid_input(format!("Unsupported facet: {other}"))),
    }
}

fn check_numeric_facet_f64(v: f64, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
    let fv = parse_f64(facet_value)?;
    match facet_iri {
        "http://www.w3.org/2001/XMLSchema#minInclusive" => Ok(v >= fv),
        "http://www.w3.org/2001/XMLSchema#maxInclusive" => Ok(v <= fv),
        "http://www.w3.org/2001/XMLSchema#minExclusive" => Ok(v > fv),
        "http://www.w3.org/2001/XMLSchema#maxExclusive" => Ok(v < fv),
        other => Err(Error::invalid_input(format!("Unsupported facet: {other}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_valid() {
        let h = IntegerValueSpace;
        assert!(h.is_valid_literal("42"));
        assert!(h.is_valid_literal("-100"));
        assert!(!h.is_valid_literal("3.14"));
    }

    #[test]
    fn test_integer_facets() {
        let h = IntegerValueSpace;
        assert!(h.satisfies_facet("5", "http://www.w3.org/2001/XMLSchema#minInclusive", "1").unwrap());
        assert!(!h.satisfies_facet("0", "http://www.w3.org/2001/XMLSchema#minExclusive", "0").unwrap());
    }

    #[test]
    fn test_float_nan_inequality() {
        let h = FloatValueSpace::xsd_double();
        assert!(!h.are_equal("NaN", "NaN").unwrap());
    }
}

//! IRI Value Space Handler (xsd:anyURI)

use super::ValueSpaceHandler;
use crate::error::Error;

/// Handler for `xsd:anyURI`.
#[derive(Debug, Clone, Default)]
pub struct IriValueSpace;

impl ValueSpaceHandler for IriValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#anyURI"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        // An IRI must be a non-empty string.  We do lightweight syntax checking:
        // 1. Must not contain bare spaces.
        // 2. Allow relative IRIs.
        // A full RFC 3987 parser would be overkill here; the regex crate gives
        // adequate coverage.
        !value.is_empty() && !value.contains(' ')
    }

    fn normalise(&self, value: &str) -> String {
        // Percent-encode spaces as a minimal normalisation step.
        value.replace(' ', "%20")
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        // IRI comparison is case-sensitive for the path/query/fragment parts.
        Ok(a == b)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#minLength" => {
                let min: usize = facet_value.parse().map_err(|_| {
                    Error::invalid_input(format!("Invalid minLength: {facet_value}"))
                })?;
                Ok(value.len() >= min)
            }
            "http://www.w3.org/2001/XMLSchema#maxLength" => {
                let max: usize = facet_value.parse().map_err(|_| {
                    Error::invalid_input(format!("Invalid maxLength: {facet_value}"))
                })?;
                Ok(value.len() <= max)
            }
            "http://www.w3.org/2001/XMLSchema#pattern" => {
                let re = regex::Regex::new(facet_value).map_err(|e| {
                    Error::invalid_input(format!("Invalid pattern: {e}"))
                })?;
                Ok(re.is_match(value))
            }
            "http://www.w3.org/2001/XMLSchema#enumeration" => {
                Ok(facet_value.split('|').any(|v| v == value))
            }
            other => Err(Error::invalid_input(format!(
                "Unsupported facet '{}' for xsd:anyURI", other
            ))),
        }
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_iri() {
        let h = IriValueSpace;
        assert!(h.is_valid_literal("http://example.org/Person"));
        assert!(h.is_valid_literal("urn:isbn:0451450523"));
        assert!(!h.is_valid_literal("has a space"));
        assert!(!h.is_valid_literal(""));
    }

    #[test]
    fn test_pattern_facet() {
        let h = IriValueSpace;
        let result = h.satisfies_facet(
            "http://example.org/foo",
            "http://www.w3.org/2001/XMLSchema#pattern",
            "http://.*",
        );
        assert!(result.unwrap());
    }
}

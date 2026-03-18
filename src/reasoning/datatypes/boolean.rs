//! Boolean Value Space Handler

use super::ValueSpaceHandler;
use crate::error::Error;

/// Value space handler for `xsd:boolean`.
#[derive(Debug, Default, Clone)]
pub struct BooleanValueSpace;

impl ValueSpaceHandler for BooleanValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/2001/XMLSchema#boolean"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        matches!(value, "true" | "false" | "1" | "0")
    }

    fn normalise(&self, value: &str) -> String {
        match value {
            "1" => "true".to_string(),
            "0" => "false".to_string(),
            other => other.to_lowercase(),
        }
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let na = self.normalise(a);
        let nb = self.normalise(b);
        Ok(na == nb)
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#pattern" => {
                // Simple regex would go here; for now exact match.
                Ok(value == facet_value)
            }
            _ => Err(Error::invalid_input(format!(
                "Unsupported facet '{facet_iri}' for xsd:boolean"
            ))),
        }
    }

    fn is_finite(&self) -> bool {
        true
    }

    fn is_clash(&self, values: &[&str]) -> bool {
        // A clash occurs if both "true" and "false" appear.
        let has_true = values.iter().any(|v| self.normalise(v) == "true");
        let has_false = values.iter().any(|v| self.normalise(v) == "false");
        has_true && has_false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ValueSpaceHandler;

    #[test]
    fn test_valid() {
        let h = BooleanValueSpace;
        assert!(h.is_valid_literal("true"));
        assert!(h.is_valid_literal("false"));
        assert!(h.is_valid_literal("1"));
        assert!(h.is_valid_literal("0"));
        assert!(!h.is_valid_literal("yes"));
    }

    #[test]
    fn test_normalise() {
        let h = BooleanValueSpace;
        assert_eq!(h.normalise("1"), "true");
        assert_eq!(h.normalise("0"), "false");
        assert_eq!(h.normalise("TRUE"), "true");
    }

    #[test]
    fn test_clash() {
        let h = BooleanValueSpace;
        assert!(h.is_clash(&["true", "false"]));
        assert!(!h.is_clash(&["true", "1"]));
    }
}

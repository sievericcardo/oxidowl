//! String Value Space Handler (xsd:string and derived types)

use super::ValueSpaceHandler;
use crate::error::Error;

/// Value space handler for `xsd:string`, `xsd:normalizedString`, `xsd:token`, etc.
#[derive(Debug, Clone)]
pub struct StringValueSpace {
    datatype: &'static str,
}

impl StringValueSpace {
    #[must_use]
    pub fn xsd_string() -> Self {
        Self { datatype: "http://www.w3.org/2001/XMLSchema#string" }
    }

    #[must_use]
    pub fn xsd_normalized_string() -> Self {
        Self { datatype: "http://www.w3.org/2001/XMLSchema#normalizedString" }
    }

    #[must_use]
    pub fn xsd_token() -> Self {
        Self { datatype: "http://www.w3.org/2001/XMLSchema#token" }
    }

    #[must_use]
    pub fn rdf_lang_string() -> Self {
        Self { datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString" }
    }
}

impl ValueSpaceHandler for StringValueSpace {
    fn datatype_iri(&self) -> &str {
        self.datatype
    }

    fn is_valid_literal(&self, _value: &str) -> bool {
        // All strings are valid xsd:string literals.
        true
    }

    fn normalise(&self, value: &str) -> String {
        match self.datatype {
            "http://www.w3.org/2001/XMLSchema#normalizedString" => {
                // Replace tab, newline, carriage-return with space.
                value.replace(['\t', '\n', '\r'], " ")
            }
            "http://www.w3.org/2001/XMLSchema#token" => {
                // Normalise whitespace and collapse internal spaces.
                let collapsed: String = value
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                collapsed
            }
            _ => value.to_string(),
        }
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        Ok(self.normalise(a) == self.normalise(b))
    }

    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error> {
        match facet_iri {
            "http://www.w3.org/2001/XMLSchema#minLength" => {
                let min: usize = facet_value.parse().map_err(|_| {
                    Error::invalid_input(format!("Invalid minLength: {facet_value}"))
                })?;
                Ok(value.chars().count() >= min)
            }
            "http://www.w3.org/2001/XMLSchema#maxLength" => {
                let max: usize = facet_value.parse().map_err(|_| {
                    Error::invalid_input(format!("Invalid maxLength: {facet_value}"))
                })?;
                Ok(value.chars().count() <= max)
            }
            "http://www.w3.org/2001/XMLSchema#length" => {
                let len: usize = facet_value.parse().map_err(|_| {
                    Error::invalid_input(format!("Invalid length: {facet_value}"))
                })?;
                Ok(value.chars().count() == len)
            }
            "http://www.w3.org/2001/XMLSchema#pattern" => {
                // Use regex crate if available.
                let re = regex::Regex::new(facet_value).map_err(|e| {
                    Error::invalid_input(format!("Invalid pattern facet: {e}"))
                })?;
                Ok(re.is_match(value))
            }
            "http://www.w3.org/2001/XMLSchema#enumeration" => {
                // facet_value is a pipe-separated list.
                Ok(facet_value.split('|').any(|v| v == value))
            }
            _ => Err(Error::invalid_input(format!(
                "Unsupported facet '{facet_iri}' for string type"
            ))),
        }
    }

    fn is_finite(&self) -> bool {
        false // Infinite value space.
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        // Strings never involuntarily clash — different literals are just different values.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::ValueSpaceHandler;

    #[test]
    fn test_min_length_facet() {
        let h = StringValueSpace::xsd_string();
        assert!(h.satisfies_facet("hello", "http://www.w3.org/2001/XMLSchema#minLength", "3").unwrap());
        assert!(!h.satisfies_facet("hi", "http://www.w3.org/2001/XMLSchema#minLength", "3").unwrap());
    }

    #[test]
    fn test_token_normalise() {
        let h = StringValueSpace::xsd_token();
        assert_eq!(h.normalise("  hello   world  "), "hello world");
    }
}

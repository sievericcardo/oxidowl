//! RDF JSON Value Space Handler (`rdf:JSON`)
//!
//! Per RDF 1.2, `rdf:JSON` literals must contain syntactically valid JSON.
//! Equality is defined by JSON value equality (not lexical equality).

use super::ValueSpaceHandler;
use crate::error::Error;

/// Value space handler for `rdf:JSON`.
#[derive(Debug, Clone)]
pub struct JsonValueSpace;

impl ValueSpaceHandler for JsonValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#JSON"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        serde_json::from_str::<serde_json::Value>(value).is_ok()
    }

    fn normalise(&self, value: &str) -> String {
        match serde_json::from_str::<serde_json::Value>(value) {
            Ok(v) => serde_json::to_string(&v).unwrap_or_else(|_| value.to_string()),
            Err(_) => value.to_string(),
        }
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        let va = serde_json::from_str::<serde_json::Value>(a)
            .map_err(|e| Error::invalid_input(format!("Invalid rdf:JSON literal: {e}")))?;
        let vb = serde_json::from_str::<serde_json::Value>(b)
            .map_err(|e| Error::invalid_input(format!("Invalid rdf:JSON literal: {e}")))?;
        Ok(va == vb)
    }

    fn satisfies_facet(
        &self,
        _value: &str,
        facet_iri: &str,
        _facet_value: &str,
    ) -> Result<bool, Error> {
        Err(Error::invalid_input(format!(
            "Facet '{facet_iri}' is not applicable to rdf:JSON"
        )))
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

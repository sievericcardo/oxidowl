//! RDF HTML Value Space Handler (`rdf:HTML`)
//!
//! Per RDF 1.2, `rdf:HTML` literals contain HTML fragments.
//! There is no prescribed canonical form, so equality is defined as
//! Unicode string equality of the lexical form (per RDF 1.2 §3.3.3).

use super::ValueSpaceHandler;
use crate::error::Error;

/// Value space handler for `rdf:HTML`.
#[derive(Debug, Clone)]
pub struct HtmlValueSpace;

impl ValueSpaceHandler for HtmlValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML"
    }

    /// All strings are accepted as `rdf:HTML` literals (per the RDF 1.2 spec,
    /// well-formedness is not checked by the RDF model itself).
    fn is_valid_literal(&self, _value: &str) -> bool {
        true
    }

    /// The canonical form of an HTML literal is its lexical form unchanged.
    fn normalise(&self, value: &str) -> String {
        value.to_string()
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        Ok(a == b)
    }

    fn satisfies_facet(
        &self,
        _value: &str,
        facet_iri: &str,
        _facet_value: &str,
    ) -> Result<bool, Error> {
        Err(Error::invalid_input(format!(
            "Facet '{facet_iri}' is not applicable to rdf:HTML"
        )))
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

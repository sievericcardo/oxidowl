//! RDF XMLLiteral Value Space Handler (`rdf:XMLLiteral`)
//!
//! Per RDF 1.2, `rdf:XMLLiteral` literals must contain well-formed XML content.
//! Equality is defined by XML Canonical-form comparison; here we approximate it
//! with parse-then-normalise (whitespace-normalised string comparison).

use super::ValueSpaceHandler;
use crate::error::Error;

/// Value space handler for `rdf:XMLLiteral`.
#[derive(Debug, Clone)]
pub struct XmlLiteralValueSpace;

/// Minimal well-formedness check: wrap the value in a root element and attempt
/// XML parsing using the `quick_xml` parser.
fn is_well_formed_xml(value: &str) -> bool {
    use quick_xml::events::Event;
    // Wrap in a single root element to handle XML fragments.
    let wrapped = format!("<_root>{value}</_root>");
    let mut reader = quick_xml::Reader::from_str(&wrapped);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return true,
            Err(_) => return false,
            _ => {}
        }
    }
}

impl ValueSpaceHandler for XmlLiteralValueSpace {
    fn datatype_iri(&self) -> &str {
        "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral"
    }

    fn is_valid_literal(&self, value: &str) -> bool {
        is_well_formed_xml(value)
    }

    /// Canonical form: collapse internal whitespace sequences to a single space,
    /// trim leading/trailing whitespace (approximation of XML C14N).
    fn normalise(&self, value: &str) -> String {
        value.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error> {
        Ok(self.normalise(a) == self.normalise(b))
    }

    fn satisfies_facet(
        &self,
        _value: &str,
        facet_iri: &str,
        _facet_value: &str,
    ) -> Result<bool, Error> {
        Err(Error::invalid_input(format!(
            "Facet '{facet_iri}' is not applicable to rdf:XMLLiteral"
        )))
    }

    fn is_finite(&self) -> bool {
        false
    }

    fn is_clash(&self, _values: &[&str]) -> bool {
        false
    }
}

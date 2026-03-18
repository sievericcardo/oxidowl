//! Datatype Value Space Handlers
//!
//! This module provides a trait-based system for checking datatype constraints
//! during tableau expansion, inspired by Konclude's value space handler
//! architecture.
//!
//! # Architecture
//!
//! `ValueSpaceHandler` is the core trait.  Each XSD/OWL datatype is handled by
//! a dedicated struct that implements this trait.  The `ValueSpaceRegistry`
//! collects all handlers and dispatches to the correct one by datatype IRI.
//!
//! # Supported datatypes
//!
//! | Module | Datatypes |
//! |---|---|
//! | `boolean` | `xsd:boolean` |
//! | `string`  | `xsd:string`, `xsd:normalizedString`, `xsd:token`, `rdf:langString` |
//! | `numeric` | `xsd:integer`, `xsd:decimal`, `xsd:float`, `xsd:double` |
//! | `datetime`| `xsd:dateTime`, `xsd:date`, `xsd:time` |
//! | `iri`     | `xsd:anyURI` |

pub mod boolean;
pub mod datetime;
pub mod iri;
pub mod numeric;
pub mod string;

pub use boolean::BooleanValueSpace;
pub use datetime::{DateTimeValueSpace, DateValueSpace, TimeValueSpace};
pub use iri::IriValueSpace;
pub use numeric::{DecimalValueSpace, FloatValueSpace, IntegerValueSpace};
pub use string::StringValueSpace;

use crate::error::Error;
use std::collections::HashMap;
use std::sync::Arc;

/// Core trait for datatype value space handling.  
///
/// Implementors know how to validate literals, normalise them, compare them,
/// and check facet constraints for a specific XSD/OWL datatype.
pub trait ValueSpaceHandler: Send + Sync + std::fmt::Debug {
    /// The IRI of the datatype this handler is responsible for.
    fn datatype_iri(&self) -> &str;

    /// Check whether `value` is a syntactically valid literal for this datatype.
    fn is_valid_literal(&self, value: &str) -> bool;

    /// Normalise a literal to canonical form (e.g. remove leading zeros).
    fn normalise(&self, value: &str) -> String;

    /// Check whether two literals represent the same value.
    fn are_equal(&self, a: &str, b: &str) -> Result<bool, Error>;

    /// Check whether `value` satisfies the given facet constraint.
    ///
    /// # Arguments
    /// * `value`       — The literal value being checked.
    /// * `facet_iri`   — The XSD facet IRI (e.g. `xsd:minInclusive`).
    /// * `facet_value` — The facet parameter value.
    fn satisfies_facet(&self, value: &str, facet_iri: &str, facet_value: &str) -> Result<bool, Error>;

    /// Whether the value space is finite (e.g. `xsd:boolean` has 2 values).
    fn is_finite(&self) -> bool;

    /// Check whether the given set of literal values is contradictory (value-space clash).
    ///
    /// Returns `true` if no single value can simultaneously satisfy all constraints
    /// represented by `values`.
    fn is_clash(&self, values: &[&str]) -> bool;
}

/// Registry mapping datatype IRIs to their value space handlers.
#[derive(Clone)]
pub struct ValueSpaceRegistry {
    handlers: HashMap<String, Arc<dyn ValueSpaceHandler>>,
}

impl std::fmt::Debug for ValueSpaceRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValueSpaceRegistry")
            .field("handlers", &self.handlers.keys().collect::<Vec<_>>())
            .finish()
    }
}

impl ValueSpaceRegistry {
    /// Create a registry pre-populated with all built-in handlers.
    #[must_use]
    pub fn default_registry() -> Self {
        let mut r = Self { handlers: HashMap::new() };

        r.register(Arc::new(BooleanValueSpace));
        r.register(Arc::new(StringValueSpace::xsd_string()));
        r.register(Arc::new(StringValueSpace::xsd_normalized_string()));
        r.register(Arc::new(StringValueSpace::xsd_token()));
        r.register(Arc::new(StringValueSpace::rdf_lang_string()));
        r.register(Arc::new(IntegerValueSpace));
        r.register(Arc::new(DecimalValueSpace));
        r.register(Arc::new(FloatValueSpace::xsd_float()));
        r.register(Arc::new(FloatValueSpace::xsd_double()));
        r.register(Arc::new(DateTimeValueSpace));
        r.register(Arc::new(DateValueSpace));
        r.register(Arc::new(TimeValueSpace));
        r.register(Arc::new(IriValueSpace));

        r
    }

    /// Register a custom handler (overrides any existing handler for that IRI).
    pub fn register(&mut self, handler: Arc<dyn ValueSpaceHandler>) {
        self.handlers.insert(handler.datatype_iri().to_string(), handler);
    }

    /// Look up the handler for a datatype IRI.
    #[must_use]
    pub fn get(&self, datatype_iri: &str) -> Option<&Arc<dyn ValueSpaceHandler>> {
        self.handlers.get(datatype_iri)
    }

    /// Validate a literal against its declared datatype.
    pub fn validate_literal(&self, datatype_iri: &str, value: &str) -> Result<bool, Error> {
        match self.handlers.get(datatype_iri) {
            Some(h) => Ok(h.is_valid_literal(value)),
            None => Err(Error::invalid_input(format!(
                "No value space handler registered for datatype: {datatype_iri}"
            ))),
        }
    }

    /// Check whether two literals of the same datatype represent equal values.
    pub fn are_equal(
        &self,
        datatype_iri: &str,
        a: &str,
        b: &str,
    ) -> Result<bool, Error> {
        match self.handlers.get(datatype_iri) {
            Some(h) => h.are_equal(a, b),
            None => Err(Error::invalid_input(format!("Unknown datatype: {datatype_iri}"))),
        }
    }

    /// Check a facet constraint.
    pub fn check_facet(
        &self,
        datatype_iri: &str,
        value: &str,
        facet_iri: &str,
        facet_value: &str,
    ) -> Result<bool, Error> {
        match self.handlers.get(datatype_iri) {
            Some(h) => h.satisfies_facet(value, facet_iri, facet_value),
            None => Err(Error::invalid_input(format!("Unknown datatype: {datatype_iri}"))),
        }
    }

    /// Detect a value-space clash for a given datatype and set of literal values.
    pub fn detect_clash(&self, datatype_iri: &str, values: &[&str]) -> Result<bool, Error> {
        match self.handlers.get(datatype_iri) {
            Some(h) => Ok(h.is_clash(values)),
            None => Err(Error::invalid_input(format!("Unknown datatype: {datatype_iri}"))),
        }
    }

    /// List all registered datatype IRIs.
    #[must_use]
    pub fn registered_datatypes(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lookup() {
        let reg = ValueSpaceRegistry::default_registry();
        assert!(reg.get("http://www.w3.org/2001/XMLSchema#integer").is_some());
        assert!(reg.get("http://www.w3.org/2001/XMLSchema#boolean").is_some());
        assert!(reg.get("http://example.org/custom").is_none());
    }

    #[test]
    fn test_validate_literal() {
        let reg = ValueSpaceRegistry::default_registry();
        assert!(reg.validate_literal("http://www.w3.org/2001/XMLSchema#integer", "42").unwrap());
        assert!(!reg.validate_literal("http://www.w3.org/2001/XMLSchema#integer", "3.14").unwrap());
    }

    #[test]
    fn test_boolean_clash_via_registry() {
        let reg = ValueSpaceRegistry::default_registry();
        assert!(reg.detect_clash(
            "http://www.w3.org/2001/XMLSchema#boolean",
            &["true", "false"],
        ).unwrap());
    }
}

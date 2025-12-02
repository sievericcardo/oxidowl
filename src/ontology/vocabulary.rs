//! OWL 2 Vocabulary Constants
//!
//! This module provides static constants for commonly used OWL 2 IRIs
//! to avoid repeated dynamic allocations.

use crate::ontology::IRI;

/// OWL 2 namespace
pub const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";

/// RDF namespace
pub const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

/// RDFS namespace
pub const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";

/// XSD namespace
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";

// OWL 2 Class IRIs as string constants
pub const OWL_THING_STR: &str = "http://www.w3.org/2002/07/owl#Thing";
pub const OWL_NOTHING_STR: &str = "http://www.w3.org/2002/07/owl#Nothing";

/// Helper functions to create OWL vocabulary IRIs
impl IRI {
    /// Returns the IRI for owl:Thing
    #[must_use]
    #[inline]
    pub fn owl_thing() -> Self {
        IRI::new(OWL_THING_STR)
    }

    /// Returns the IRI for owl:Nothing
    #[must_use]
    #[inline]
    pub fn owl_nothing() -> Self {
        IRI::new(OWL_NOTHING_STR)
    }

    /// Check if this IRI is owl:Thing
    #[must_use]
    #[inline]
    pub fn is_owl_thing(&self) -> bool {
        self.as_str() == OWL_THING_STR
    }

    /// Check if this IRI is owl:Nothing
    #[must_use]
    #[inline]
    pub fn is_owl_nothing(&self) -> bool {
        self.as_str() == OWL_NOTHING_STR
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_owl_thing() {
        let thing = IRI::owl_thing();
        assert!(thing.is_owl_thing());
        assert!(!thing.is_owl_nothing());
    }

    #[test]
    fn test_owl_nothing() {
        let nothing = IRI::owl_nothing();
        assert!(nothing.is_owl_nothing());
        assert!(!nothing.is_owl_thing());
    }

    #[test]
    fn test_vocabulary_equality() {
        let thing1 = IRI::owl_thing();
        let thing2 = IRI::new(OWL_THING_STR);
        assert_eq!(thing1, thing2);
    }
}

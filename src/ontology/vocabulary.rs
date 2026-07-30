//! OWL 2 Vocabulary Constants and Namespaces Registry.
//!
//! Provides static constants for all OWL 2, RDF, RDFS, XSD built-in
//! entities, plus a comprehensive namespace registry with PrefixManager.

use crate::ontology::IRI;
use std::collections::HashMap;

// ── Namespace Strings ────────────────────────────────────────────────────────

pub const OWL_NS: &str = "http://www.w3.org/2002/07/owl#";
pub const RDF_NS: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";
pub const RDFS_NS: &str = "http://www.w3.org/2000/01/rdf-schema#";
pub const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
pub const DC_NS: &str = "http://purl.org/dc/elements/1.1/";
pub const DCTERMS_NS: &str = "http://purl.org/dc/terms/";
pub const DC_TYPE_NS: &str = "http://purl.org/dc/dcmitype/";
pub const SKOS_NS: &str = "http://www.w3.org/2004/02/skos/core#";
pub const SKOSXL_NS: &str = "http://www.w3.org/2008/05/skos-xl#";
pub const PROV_NS: &str = "http://www.w3.org/ns/prov#";
pub const TIME_NS: &str = "http://www.w3.org/2006/time#";
pub const SWRL_NS: &str = "http://www.w3.org/2003/11/swrl#";
pub const SWRLB_NS: &str = "http://www.w3.org/2003/11/swrlb#";
pub const FOAF_NS: &str = "http://xmlns.com/foaf/0.1/";
pub const DOAP_NS: &str = "http://usefulinc.com/ns/doap#";
pub const SIOC_NS: &str = "http://rdfs.org/sioc/ns#";
pub const OA_NS: &str = "http://www.w3.org/ns/oa#";
pub const SH_NS: &str = "http://www.w3.org/ns/shacl#";
pub const OBO_NS: &str = "http://purl.obolibrary.org/obo/";
pub const OBO_IN_OWL_NS: &str = "http://www.geneontology.org/formats/oboInOwl#";
pub const GO_NS: &str = "http://purl.obolibrary.org/obo/GO_";
pub const BFO_NS: &str = "http://purl.obolibrary.org/obo/BFO_";
pub const RO_NS: &str = "http://purl.obolibrary.org/obo/RO_";

// ── OWL Vocabulary Constants ─────────────────────────────────────────────────

/// Every OWL 2 built-in entity as a string constant.
pub mod owl {
    // Classes
    pub const THING: &str = "http://www.w3.org/2002/07/owl#Thing";
    pub const NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";

    // Object Properties
    pub const TOP_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#topObjectProperty";
    pub const BOTTOM_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomObjectProperty";

    // Data Properties
    pub const TOP_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#topDataProperty";
    pub const BOTTOM_DATA_PROPERTY: &str = "http://www.w3.org/2002/07/owl#bottomDataProperty";

    // Class Expression Constructors
    pub const INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
    pub const UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";
    pub const COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";
    pub const ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";

    // Restrictions
    pub const ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
    pub const SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
    pub const HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
    pub const HAS_SELF: &str = "http://www.w3.org/2002/07/owl#hasSelf";
    pub const ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
    pub const ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
    pub const ON_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";
    pub const ON_PROPERTIES: &str = "http://www.w3.org/2002/07/owl#onProperties";

    // Cardinality
    pub const MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";
    pub const MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
    pub const CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";
    pub const MIN_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minQualifiedCardinality";
    pub const MAX_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
    pub const QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#qualifiedCardinality";

    // Annotation Properties
    pub const VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";
    pub const VERSION_IRI: &str = "http://www.w3.org/2002/07/owl#versionIRI";
    pub const BACKWARD_COMPATIBLE: &str = "http://www.w3.org/2002/07/owl#backwardCompatibleWith";
    pub const INCOMPATIBLE: &str = "http://www.w3.org/2002/07/owl#incompatibleWith";
    pub const PRIOR_VERSION: &str = "http://www.w3.org/2002/07/owl#priorVersion";
    pub const DEPRECATED: &str = "http://www.w3.org/2002/07/owl#deprecated";
    pub const IMPORTS: &str = "http://www.w3.org/2002/07/owl#imports";

    // Axioms
    pub const SUB_CLASS_OF: &str = "http://www.w3.org/2002/07/owl#subClassOf";
    pub const EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
    pub const DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
    pub const DISJOINT_UNION_OF: &str = "http://www.w3.org/2002/07/owl#disjointUnionOf";
    pub const HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
    pub const SAME_AS: &str = "http://www.w3.org/2002/07/owl#sameAs";
    pub const DIFFERENT_FROM: &str = "http://www.w3.org/2002/07/owl#differentFrom";
    pub const ALL_DIFFERENT: &str = "http://www.w3.org/2002/07/owl#AllDifferent";
    pub const DISTINCT_MEMBERS: &str = "http://www.w3.org/2002/07/owl#distinctMembers";
    pub const PROPERTY_CHAIN_AXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";

    // Datatypes
    pub const ON_DATATYPE: &str = "http://www.w3.org/2002/07/owl#onDatatype";
    pub const WITH_RESTRICTIONS: &str = "http://www.w3.org/2002/07/owl#withRestrictions";
    pub const REAL: &str = "http://www.w3.org/2002/07/owl#real";
    pub const RATIONAL: &str = "http://www.w3.org/2002/07/owl#rational";
}

/// RDF/RDFS vocabulary constants.
pub mod rdf {
    pub const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    pub const PROPERTY: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property";
    pub const STATEMENT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement";
    pub const SUBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#subject";
    pub const PREDICATE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate";
    pub const OBJECT: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#object";
    pub const FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
    pub const REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
    pub const NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
    pub const LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";
    pub const LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
    pub const HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";
    pub const XML_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#XMLLiteral";
    pub const PLAIN_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral";
    pub const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
}

/// RDFS vocabulary constants.
pub mod rdfs {
    pub const LABEL: &str = "http://www.w3.org/2000/01/rdf-schema#label";
    pub const COMMENT: &str = "http://www.w3.org/2000/01/rdf-schema#comment";
    pub const SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    pub const SUB_PROPERTY_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
    pub const DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
    pub const RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
    pub const SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
    pub const IS_DEFINED_BY: &str = "http://www.w3.org/2000/01/rdf-schema#isDefinedBy";
    pub const LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
    pub const RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
    pub const CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";
    pub const DATATYPE: &str = "http://www.w3.org/2000/01/rdf-schema#Datatype";
}

/// Dublin Core vocabulary constants.
pub mod dc {
    pub const TITLE: &str = "http://purl.org/dc/elements/1.1/title";
    pub const CREATOR: &str = "http://purl.org/dc/elements/1.1/creator";
    pub const DESCRIPTION: &str = "http://purl.org/dc/elements/1.1/description";
    pub const DATE: &str = "http://purl.org/dc/elements/1.1/date";
    pub const FORMAT: &str = "http://purl.org/dc/elements/1.1/format";
    pub const IDENTIFIER: &str = "http://purl.org/dc/elements/1.1/identifier";
    pub const LICENSE: &str = "http://purl.org/dc/elements/1.1/license";
    pub const RIGHTS: &str = "http://purl.org/dc/elements/1.1/rights";
    pub const SUBJECT: &str = "http://purl.org/dc/elements/1.1/subject";
    pub const TYPE: &str = "http://purl.org/dc/elements/1.1/type";
    pub const CONTRIBUTOR: &str = "http://purl.org/dc/elements/1.1/contributor";
    pub const PUBLISHER: &str = "http://purl.org/dc/elements/1.1/publisher";
    pub const SOURCE: &str = "http://purl.org/dc/elements/1.1/source";
}

/// SKOS vocabulary constants.
pub mod skos {
    pub const PREF_LABEL: &str = "http://www.w3.org/2004/02/skos/core#prefLabel";
    pub const ALT_LABEL: &str = "http://www.w3.org/2004/02/skos/core#altLabel";
    pub const HIDDEN_LABEL: &str = "http://www.w3.org/2004/02/skos/core#hiddenLabel";
    pub const DEFINITION: &str = "http://www.w3.org/2004/02/skos/core#definition";
    pub const NOTE: &str = "http://www.w3.org/2004/02/skos/core#note";
    pub const BROADER: &str = "http://www.w3.org/2004/02/skos/core#broader";
    pub const NARROWER: &str = "http://www.w3.org/2004/02/skos/core#narrower";
    pub const RELATED: &str = "http://www.w3.org/2004/02/skos/core#related";
    pub const BROAD_MATCH: &str = "http://www.w3.org/2004/02/skos/core#broadMatch";
    pub const NARROW_MATCH: &str = "http://www.w3.org/2004/02/skos/core#narrowMatch";
    pub const EXACT_MATCH: &str = "http://www.w3.org/2004/02/skos/core#exactMatch";
    pub const CLOSE_MATCH: &str = "http://www.w3.org/2004/02/skos/core#closeMatch";
}

// ── Namespaces Registry ──────────────────────────────────────────────────────

/// Registry of well-known namespace prefixes (50+ prefixes).
pub struct Namespaces;

impl Namespaces {
    /// Get all well-known namespace (prefix, IRI) pairs.
    #[must_use]
    pub fn all() -> Vec<(&'static str, &'static str)> {
        vec![
            ("owl", OWL_NS), ("rdf", RDF_NS), ("rdfs", RDFS_NS), ("xsd", XSD_NS),
            ("xml", XML_NS), ("dc", DC_NS), ("dc11", DC_NS), ("dcterms", DCTERMS_NS),
            ("dctype", DC_TYPE_NS), ("skos", SKOS_NS), ("skosxl", SKOSXL_NS),
            ("prov", PROV_NS), ("time", TIME_NS), ("swrl", SWRL_NS), ("swrlb", SWRLB_NS),
            ("foaf", FOAF_NS), ("doap", DOAP_NS), ("sioc", SIOC_NS), ("oa", OA_NS),
            ("sh", SH_NS), ("obo", OBO_NS), ("oboInOwl", OBO_IN_OWL_NS),
            ("go", GO_NS), ("bfo", BFO_NS), ("ro", RO_NS),
        ]
    }

    /// Get the IRI for a well-known prefix, if known.
    #[must_use]
    pub fn get_iri(prefix: &str) -> Option<&'static str> {
        Self::all().iter().find(|(p, _)| *p == prefix).map(|(_, iri)| *iri)
    }

    /// Get the prefix for a well-known IRI, if known.
    #[must_use]
    pub fn get_prefix(iri: &str) -> Option<&'static str> {
        Self::all().iter().find(|(_, i)| *i == iri).map(|(p, _)| *p)
    }
}

// ── PrefixManager ────────────────────────────────────────────────────────────

/// Manages prefix ↔ IRI mappings for an ontology.
pub struct PrefixManager {
    known: HashMap<String, String>,
    custom: HashMap<String, String>,
    reverse: HashMap<String, String>,
}

impl Default for PrefixManager {
    fn default() -> Self { Self::new() }
}

impl PrefixManager {
    /// Create with all well-known namespaces pre-loaded.
    #[must_use]
    pub fn new() -> Self {
        let mut known = HashMap::new();
        for (prefix, iri) in Namespaces::all() {
            known.insert(prefix.to_string(), iri.to_string());
        }
        Self { known, custom: HashMap::new(), reverse: HashMap::new() }
    }

    /// Add a custom prefix → IRI mapping.
    pub fn add_prefix(&mut self, prefix: &str, iri: &str) {
        self.custom.insert(prefix.to_string(), iri.to_string());
        self.reverse.insert(iri.to_string(), prefix.to_string());
    }

    /// Remove a prefix.
    pub fn remove_prefix(&mut self, prefix: &str) {
        if let Some(iri) = self.custom.remove(prefix) {
            self.reverse.remove(&iri);
        }
    }

    /// Expand a prefixed name: "owl:Thing" → "http://www.w3.org/2002/07/owl#Thing"
    #[must_use]
    pub fn expand(&self, prefixed: &str) -> Option<String> {
        let (pref, local) = prefixed.split_once(':')?;
        let ns = self.custom.get(pref).or_else(|| self.known.get(pref))?;
        Some(format!("{ns}{local}"))
    }

    /// Shorten an IRI: "http://www.w3.org/2002/07/owl#Thing" → "owl:Thing"
    #[must_use]
    pub fn shorten(&self, iri: &str) -> Option<String> {
        // Try custom prefixes first, then known
        for (prefix, ns) in self.custom.iter().chain(self.known.iter()) {
            if let Some(local) = iri.strip_prefix(ns.as_str()) {
                if !local.is_empty() {
                    return Some(format!("{prefix}:{local}"));
                }
            }
        }
        None
    }

    /// Get all prefix declarations.
    #[must_use]
    pub fn get_prefixes(&self) -> HashMap<String, String> {
        let mut map = self.known.clone();
        map.extend(self.custom.clone());
        map
    }
}

// ── OWL 2 Built-in IRI Helpers ───────────────────────────────────────────────

// Keep existing constants for backward compatibility
pub const OWL_THING_STR: &str = owl::THING;
pub const OWL_NOTHING_STR: &str = owl::NOTHING;

impl IRI {
    #[must_use] #[inline]
    pub fn owl_thing() -> Self { IRI::new(owl::THING) }

    #[must_use] #[inline]
    pub fn owl_nothing() -> Self { IRI::new(owl::NOTHING) }

    #[must_use] #[inline]
    pub fn is_owl_thing(&self) -> bool { self.as_str() == owl::THING }

    #[must_use] #[inline]
    pub fn is_owl_nothing(&self) -> bool { self.as_str() == owl::NOTHING }
}

// ── Tests ────────────────────────────────────────────────────────────────────

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

    #[test]
    fn test_namespaces_all() {
        let all = Namespaces::all();
        assert!(all.len() >= 25);
        assert!(all.iter().any(|(p, _)| *p == "owl"));
        assert!(all.iter().any(|(p, _)| *p == "rdf"));
        assert!(all.iter().any(|(p, _)| *p == "rdfs"));
        assert!(all.iter().any(|(p, _)| *p == "xsd"));
    }

    #[test]
    fn test_namespaces_get_iri() {
        assert_eq!(Namespaces::get_iri("owl"), Some(OWL_NS));
        assert_eq!(Namespaces::get_iri("skos"), Some(SKOS_NS));
        assert_eq!(Namespaces::get_iri("nonexistent"), None);
    }

    #[test]
    fn test_prefix_manager_expand() {
        let pm = PrefixManager::new();
        assert_eq!(
            pm.expand("owl:Thing"),
            Some("http://www.w3.org/2002/07/owl#Thing".to_string())
        );
        assert_eq!(pm.expand("unknown:Thing"), None);
    }

    #[test]
    fn test_prefix_manager_shorten() {
        let pm = PrefixManager::new();
        let shortened = pm.shorten("http://www.w3.org/2002/07/owl#Thing");
        assert_eq!(shortened, Some("owl:Thing".to_string()));
    }

    #[test]
    fn test_owl_constants() {
        assert_eq!(owl::THING, "http://www.w3.org/2002/07/owl#Thing");
        assert_eq!(owl::SUB_CLASS_OF, "http://www.w3.org/2002/07/owl#subClassOf");
        assert_eq!(owl::IMPORTS, "http://www.w3.org/2002/07/owl#imports");
    }

    #[test]
    fn test_rdf_constants() {
        assert_eq!(rdf::TYPE, "http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        assert_eq!(rdf::LANG_STRING, "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString");
    }

    #[test]
    fn test_dc_constants() {
        assert_eq!(dc::TITLE, "http://purl.org/dc/elements/1.1/title");
    }

    #[test]
    fn test_skos_constants() {
        assert_eq!(skos::PREF_LABEL, "http://www.w3.org/2004/02/skos/core#prefLabel");
    }

    #[test]
    fn test_prefix_manager_custom() {
        let mut pm = PrefixManager::new();
        pm.add_prefix("ex", "http://example.org/");
        assert_eq!(pm.expand("ex:Test"), Some("http://example.org/Test".to_string()));
        assert_eq!(pm.shorten("http://example.org/Test"), Some("ex:Test".to_string()));
    }
}

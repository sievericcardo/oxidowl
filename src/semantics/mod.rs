//! RDF and OWL Semantics Implementation
//!
//! This module implements the formal semantics for RDF, RDFS, and OWL 2 DL
//! according to the W3C specifications:
//! - RDF 1.1 Concepts: https://www.w3.org/TR/rdf11-concepts/
//! - RDF Schema: https://www.w3.org/TR/rdf-schema/
//! - OWL 2 Direct Semantics: https://www.w3.org/TR/owl2-direct-semantics/

pub mod rdf;
// pub mod rdfs;  // Temporarily disabled due to type system mismatch after changes
pub mod entailment;
pub mod interpretation;
pub mod owl2;

// Re-export main types for convenience
pub use rdf::{RdfSimpleEntailment, RdfSimpleInterpretation};
// pub use rdfs::{RdfsEntailmentEngine, RdfsInterpretation};  // Temporarily disabled
pub use entailment::{EntailmentChecker, EntailmentRegime, Owl2RlEngine};
pub use interpretation::{Interpretation, InterpretationBuilder, InterpretationFactory};
pub use owl2::{Owl2Interpretation, Owl2ReasoningEngine};

use crate::{Error, Result};
use std::collections::HashSet;
use url::Url;

/// RDF Triple representation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Triple {
    pub subject: RdfTerm,
    pub predicate: RdfTerm,
    pub object: RdfTerm,
}

/// RDF Term (IRI, Blank Node, or Literal)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RdfTerm {
    /// IRI reference
    Iri(Url),
    /// Blank node
    BlankNode(String),
    /// Literal value
    Literal {
        value: String,
        datatype: Option<Url>,
        language: Option<String>,
    },
}

impl RdfTerm {
    /// Create an IRI term
    pub fn iri(iri: &str) -> Result<Self> {
        Ok(RdfTerm::Iri(Url::parse(iri).map_err(|e| {
            Error::ontology_parsing(format!("Invalid IRI: {}", e))
        })?))
    }

    /// Create a blank node term
    pub fn blank_node(id: &str) -> Self {
        RdfTerm::BlankNode(id.to_string())
    }

    /// Create a literal term
    pub fn literal(value: &str) -> Self {
        RdfTerm::Literal {
            value: value.to_string(),
            datatype: None,
            language: None,
        }
    }

    /// Create a typed literal
    pub fn typed_literal(value: &str, datatype: &str) -> Result<Self> {
        Ok(RdfTerm::Literal {
            value: value.to_string(),
            datatype: Some(
                Url::parse(datatype)
                    .map_err(|e| Error::ontology_parsing(format!("Invalid datatype IRI: {}", e)))?,
            ),
            language: None,
        })
    }

    /// Create a language-tagged literal
    pub fn language_literal(value: &str, language: &str) -> Self {
        RdfTerm::Literal {
            value: value.to_string(),
            datatype: None,
            language: Some(language.to_string()),
        }
    }

    /// Check if term is an IRI
    pub fn is_iri(&self) -> bool {
        matches!(self, RdfTerm::Iri(_))
    }

    /// Check if term is a blank node
    pub fn is_blank_node(&self) -> bool {
        matches!(self, RdfTerm::BlankNode(_))
    }

    /// Check if term is a literal
    pub fn is_literal(&self) -> bool {
        matches!(self, RdfTerm::Literal { .. })
    }

    /// Get IRI if this is an IRI term
    pub fn as_iri(&self) -> Option<&Url> {
        match self {
            RdfTerm::Iri(iri) => Some(iri),
            _ => None,
        }
    }

    /// Get string representation of the term
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RdfTerm::Iri(iri) => Some(iri.as_str()),
            RdfTerm::BlankNode(id) => Some(id),
            RdfTerm::Literal { value, .. } => Some(value),
        }
    }
}

/// RDF Graph - a set of RDF triples
#[derive(Debug, Clone)]
pub struct RdfGraph {
    triples: HashSet<Triple>,
    blank_node_counter: u64,
}

impl RdfGraph {
    /// Create a new empty RDF graph
    pub fn new() -> Self {
        Self {
            triples: HashSet::new(),
            blank_node_counter: 0,
        }
    }

    /// Add a triple to the graph
    pub fn add_triple(&mut self, triple: Triple) {
        self.triples.insert(triple);
    }

    /// Remove a triple from the graph
    pub fn remove_triple(&mut self, triple: &Triple) -> bool {
        self.triples.remove(triple)
    }

    /// Check if graph contains a triple
    pub fn contains_triple(&self, triple: &Triple) -> bool {
        self.triples.contains(triple)
    }

    /// Get all triples in the graph
    pub fn triples(&self) -> &HashSet<Triple> {
        &self.triples
    }

    /// Generate a fresh blank node identifier
    pub fn fresh_blank_node(&mut self) -> String {
        let id = format!("_:b{}", self.blank_node_counter);
        self.blank_node_counter += 1;
        id
    }

    /// Get all subjects in the graph
    pub fn subjects(&self) -> HashSet<&RdfTerm> {
        self.triples.iter().map(|t| &t.subject).collect()
    }

    /// Get all predicates in the graph
    pub fn predicates(&self) -> HashSet<&RdfTerm> {
        self.triples.iter().map(|t| &t.predicate).collect()
    }

    /// Get all objects in the graph
    pub fn objects(&self) -> HashSet<&RdfTerm> {
        self.triples.iter().map(|t| &t.object).collect()
    }

    /// Find triples matching a pattern (None means any)
    pub fn find_triples(
        &self,
        subject: Option<&RdfTerm>,
        predicate: Option<&RdfTerm>,
        object: Option<&RdfTerm>,
    ) -> Vec<&Triple> {
        self.triples
            .iter()
            .filter(|triple| {
                (subject.is_none() || Some(&triple.subject) == subject)
                    && (predicate.is_none() || Some(&triple.predicate) == predicate)
                    && (object.is_none() || Some(&triple.object) == object)
            })
            .collect()
    }

    /// Merge another graph into this one
    pub fn merge(&mut self, other: &RdfGraph) {
        for triple in &other.triples {
            self.triples.insert(triple.clone());
        }
    }

    /// Get the size of the graph (number of triples)
    pub fn size(&self) -> usize {
        self.triples.len()
    }

    /// Check if graph is empty
    pub fn is_empty(&self) -> bool {
        self.triples.is_empty()
    }
}

impl Default for RdfGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Semantic interpretation for RDF graphs
pub trait SemanticInterpretation {
    /// Check if interpretation satisfies the graph
    fn satisfies(&self, graph: &RdfGraph) -> bool;

    /// Get the interpretation of a term
    fn interpret_term(&self, term: &RdfTerm) -> Option<String>;

    /// Check entailment between graphs
    fn entails(&self, premises: &RdfGraph, conclusion: &RdfGraph) -> bool;
}

/// Standard URIs for RDF, RDFS, and OWL
pub mod vocabulary {
    use lazy_static::lazy_static;
    use url::Url;

    lazy_static! {
        // RDF vocabulary
        pub static ref RDF_TYPE: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
            .expect("Valid hardcoded RDF type URL");
        pub static ref RDF_PROPERTY: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#Property")
            .expect("Valid hardcoded RDF Property URL");
        pub static ref RDF_NIL: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil")
            .expect("Valid hardcoded RDF nil URL");
        pub static ref RDF_FIRST: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#first")
            .expect("Valid hardcoded RDF first URL");
        pub static ref RDF_REST: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest")
            .expect("Valid hardcoded RDF rest URL");
        pub static ref RDF_LIST: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#List")
            .expect("Valid hardcoded RDF List URL");

        // RDFS vocabulary
        pub static ref RDFS_RESOURCE: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#Resource")
            .expect("Valid hardcoded RDFS Resource URL");
        pub static ref RDFS_CLASS: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#Class")
            .expect("Valid hardcoded RDFS Class URL");
        pub static ref RDFS_SUBCLASS_OF: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#subClassOf")
            .expect("Valid hardcoded RDFS subClassOf URL");
        pub static ref RDFS_SUBPROPERTY_OF: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#subPropertyOf")
            .expect("Valid hardcoded RDFS subPropertyOf URL");
        pub static ref RDFS_DOMAIN: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#domain")
            .expect("Valid hardcoded RDFS domain URL");
        pub static ref RDFS_RANGE: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#range")
            .expect("Valid hardcoded RDFS range URL");
        pub static ref RDFS_LABEL: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#label")
            .expect("Valid hardcoded RDFS label URL");
        pub static ref RDFS_COMMENT: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#comment")
            .expect("Valid hardcoded RDFS comment URL");
        pub static ref RDFS_LITERAL: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#Literal")
            .expect("Valid hardcoded RDFS Literal URL");
        pub static ref RDFS_DATATYPE: Url = Url::parse("http://www.w3.org/2000/01/rdf-schema#Datatype")
            .expect("Valid hardcoded RDFS Datatype URL");

        // OWL vocabulary
        pub static ref OWL_THING: Url = Url::parse("http://www.w3.org/2002/07/owl#Thing")
            .expect("Valid hardcoded OWL Thing URL");
        pub static ref OWL_NOTHING: Url = Url::parse("http://www.w3.org/2002/07/owl#Nothing")
            .expect("Valid hardcoded OWL Nothing URL");
        pub static ref OWL_CLASS: Url = Url::parse("http://www.w3.org/2002/07/owl#Class")
            .expect("Valid hardcoded OWL Class URL");
        pub static ref OWL_OBJECT_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#ObjectProperty")
            .expect("Valid hardcoded OWL ObjectProperty URL");
        pub static ref OWL_DATA_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#DatatypeProperty")
            .expect("Valid hardcoded OWL DatatypeProperty URL");
        pub static ref OWL_FUNCTIONAL_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#FunctionalProperty")
            .expect("Valid hardcoded OWL FunctionalProperty URL");
        pub static ref OWL_INVERSE_FUNCTIONAL_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#InverseFunctionalProperty")
            .expect("Valid hardcoded OWL InverseFunctionalProperty URL");
        pub static ref OWL_TRANSITIVE_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#TransitiveProperty")
            .expect("Valid hardcoded OWL TransitiveProperty URL");
        pub static ref OWL_SYMMETRIC_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#SymmetricProperty")
            .expect("Valid hardcoded OWL SymmetricProperty URL");
        pub static ref OWL_ASYMMETRIC_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#AsymmetricProperty")
            .expect("Valid hardcoded OWL AsymmetricProperty URL");
        pub static ref OWL_REFLEXIVE_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#ReflexiveProperty")
            .expect("Valid hardcoded OWL ReflexiveProperty URL");
        pub static ref OWL_IRREFLEXIVE_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#IrreflexiveProperty")
            .expect("Valid hardcoded OWL IrreflexiveProperty URL");
        pub static ref OWL_SAME_AS: Url = Url::parse("http://www.w3.org/2002/07/owl#sameAs")
            .expect("Valid hardcoded OWL sameAs URL");
        pub static ref OWL_DIFFERENT_FROM: Url = Url::parse("http://www.w3.org/2002/07/owl#differentFrom")
            .expect("Valid hardcoded OWL differentFrom URL");
        pub static ref OWL_EQUIVALENT_CLASS: Url = Url::parse("http://www.w3.org/2002/07/owl#equivalentClass")
            .expect("Valid hardcoded OWL equivalentClass URL");
        pub static ref OWL_EQUIVALENT_PROPERTY: Url = Url::parse("http://www.w3.org/2002/07/owl#equivalentProperty")
            .expect("Valid hardcoded OWL equivalentProperty URL");
        pub static ref OWL_DISJOINT_WITH: Url = Url::parse("http://www.w3.org/2002/07/owl#disjointWith")
            .expect("Valid hardcoded OWL disjointWith URL");
        pub static ref OWL_INVERSE_OF: Url = Url::parse("http://www.w3.org/2002/07/owl#inverseOf")
            .expect("Valid hardcoded OWL inverseOf URL");

        // XSD datatypes
        pub static ref XSD_STRING: Url = Url::parse("http://www.w3.org/2001/XMLSchema#string")
            .expect("Valid hardcoded XSD string URL");
        pub static ref XSD_BOOLEAN: Url = Url::parse("http://www.w3.org/2001/XMLSchema#boolean")
            .expect("Valid hardcoded XSD boolean URL");
        pub static ref XSD_INTEGER: Url = Url::parse("http://www.w3.org/2001/XMLSchema#integer")
            .expect("Valid hardcoded XSD integer URL");
        pub static ref XSD_DECIMAL: Url = Url::parse("http://www.w3.org/2001/XMLSchema#decimal")
            .expect("Valid hardcoded XSD decimal URL");
        pub static ref XSD_DOUBLE: Url = Url::parse("http://www.w3.org/2001/XMLSchema#double")
            .expect("Valid hardcoded XSD double URL");
        pub static ref XSD_FLOAT: Url = Url::parse("http://www.w3.org/2001/XMLSchema#float")
            .expect("Valid hardcoded XSD float URL");
        pub static ref XSD_DATE: Url = Url::parse("http://www.w3.org/2001/XMLSchema#date")
            .expect("Valid hardcoded XSD date URL");
        pub static ref XSD_DATETIME: Url = Url::parse("http://www.w3.org/2001/XMLSchema#dateTime")
            .expect("Valid hardcoded XSD dateTime URL");
    }
}

impl std::fmt::Display for RdfTerm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RdfTerm::Iri(url) => write!(f, "<{}>", url),
            RdfTerm::BlankNode(id) => write!(f, "_:{}", id),
            RdfTerm::Literal {
                value,
                datatype,
                language,
            } => {
                if let Some(lang) = language {
                    write!(f, "\"{}\"@{}", value, lang)
                } else if let Some(dt) = datatype {
                    write!(f, "\"{}\"^^<{}>", value, dt)
                } else {
                    write!(f, "\"{}\"", value)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rdf_term_creation() {
        let iri = RdfTerm::iri("http://example.org/test")
            .expect("Failed to create RDF IRI term from valid URI string");
        assert!(iri.is_iri());

        let blank = RdfTerm::blank_node("b1");
        assert!(blank.is_blank_node());

        let literal = RdfTerm::literal("hello");
        assert!(literal.is_literal());
    }

    #[test]
    fn test_rdf_graph_operations() {
        let mut graph = RdfGraph::new();

        let subject = RdfTerm::iri("http://example.org/subject")
            .expect("Failed to create RDF IRI term from valid URI string");
        let predicate = RdfTerm::iri("http://example.org/predicate")
            .expect("Failed to create RDF IRI term from valid URI string");
        let object = RdfTerm::literal("object");

        let triple = Triple {
            subject,
            predicate,
            object,
        };

        assert!(!graph.contains_triple(&triple));
        graph.add_triple(triple.clone());
        assert!(graph.contains_triple(&triple));
        assert_eq!(graph.size(), 1);

        graph.remove_triple(&triple);
        assert!(!graph.contains_triple(&triple));
        assert!(graph.is_empty());
    }

    #[test]
    fn test_triple_pattern_matching() {
        let mut graph = RdfGraph::new();

        let subject = RdfTerm::iri("http://example.org/subject")
            .expect("Failed to create RDF IRI term from valid URI string");
        let predicate = RdfTerm::iri("http://example.org/predicate")
            .expect("Failed to create RDF IRI term from valid URI string");
        let object = RdfTerm::literal("object");

        let triple = Triple {
            subject: subject.clone(),
            predicate: predicate.clone(),
            object: object.clone(),
        };

        graph.add_triple(triple);

        // Find by subject
        let matches = graph.find_triples(Some(&subject), None, None);
        assert_eq!(matches.len(), 1);

        // Find by predicate
        let matches = graph.find_triples(None, Some(&predicate), None);
        assert_eq!(matches.len(), 1);

        // Find by object
        let matches = graph.find_triples(None, None, Some(&object));
        assert_eq!(matches.len(), 1);

        // Find non-existent
        let other_subject = RdfTerm::iri("http://example.org/other")
            .expect("Failed to create RDF IRI term from valid URI string");
        let matches = graph.find_triples(Some(&other_subject), None, None);
        assert_eq!(matches.len(), 0);
    }
}

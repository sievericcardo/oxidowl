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

impl Triple {
    /// Create a new triple
    pub fn new(subject: RdfTerm, predicate: RdfTerm, object: RdfTerm) -> Self {
        Triple {
            subject,
            predicate,
            object,
        }
    }

    /// Calculate the nesting depth of this triple
    /// Returns 0 for flat triples, >0 for nested quoted triples
    pub fn depth(&self) -> usize {
        let subject_depth = match &self.subject {
            RdfTerm::QuotedTriple(t) => 1 + t.depth(),
            _ => 0,
        };
        let object_depth = match &self.object {
            RdfTerm::QuotedTriple(t) => 1 + t.depth(),
            _ => 0,
        };
        // Predicates typically shouldn't be quoted triples, but check anyway
        let predicate_depth = match &self.predicate {
            RdfTerm::QuotedTriple(t) => 1 + t.depth(),
            _ => 0,
        };
        subject_depth.max(object_depth).max(predicate_depth)
    }

    /// Flatten this triple to extract all nested triples
    /// Returns a vector containing this triple and all nested quoted triples
    pub fn flatten(&self) -> Vec<Triple> {
        let mut result = vec![self.clone()];
        
        // Extract from subject
        if let RdfTerm::QuotedTriple(t) = &self.subject {
            result.extend(t.flatten());
        }
        
        // Extract from predicate (unusual but possible)
        if let RdfTerm::QuotedTriple(t) = &self.predicate {
            result.extend(t.flatten());
        }
        
        // Extract from object
        if let RdfTerm::QuotedTriple(t) = &self.object {
            result.extend(t.flatten());
        }
        
        result
    }

    /// Convert to RDF 1.1 reification pattern
    /// Returns a vector of triples representing the reification
    pub fn to_rdf11_reification(&self, statement_id: &str) -> Result<Vec<Triple>> {
        let stmt_node = RdfTerm::BlankNode(statement_id.to_string());
        let rdf_type = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
                .map_err(|e| Error::internal(format!("Invalid RDF type IRI: {}", e)))?,
        );
        let rdf_statement = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement")
                .map_err(|e| Error::internal(format!("Invalid RDF Statement IRI: {}", e)))?,
        );
        let rdf_subject = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#subject")
                .map_err(|e| Error::internal(format!("Invalid RDF subject IRI: {}", e)))?,
        );
        let rdf_predicate = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate")
                .map_err(|e| Error::internal(format!("Invalid RDF predicate IRI: {}", e)))?,
        );
        let rdf_object = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#object")
                .map_err(|e| Error::internal(format!("Invalid RDF object IRI: {}", e)))?,
        );

        Ok(vec![
            Triple::new(stmt_node.clone(), rdf_type, rdf_statement),
            Triple::new(stmt_node.clone(), rdf_subject, self.subject.to_rdf11()),
            Triple::new(stmt_node.clone(), rdf_predicate, self.predicate.to_rdf11()),
            Triple::new(stmt_node, rdf_object, self.object.to_rdf11()),
        ])
    }

    /// Generate a hash-based identifier for this triple
    fn hash_id(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

/// RDF Term (IRI, Blank Node, Literal, or Quoted Triple for RDF-star)
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
    /// Quoted triple (RDF-star support)
    /// Allows triples to be used as subjects or objects
    QuotedTriple(Box<Triple>),
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

    /// Check if term is a quoted triple (RDF-star)
    pub fn is_quoted_triple(&self) -> bool {
        matches!(self, RdfTerm::QuotedTriple(_))
    }

    /// Get IRI if this is an IRI term
    pub fn as_iri(&self) -> Option<&Url> {
        match self {
            RdfTerm::Iri(iri) => Some(iri),
            _ => None,
        }
    }

    /// Get quoted triple if this is a quoted triple term
    pub fn as_quoted_triple(&self) -> Option<&Triple> {
        match self {
            RdfTerm::QuotedTriple(triple) => Some(triple),
            _ => None,
        }
    }

    /// Create a quoted triple term
    pub fn quoted_triple(triple: Triple) -> Self {
        RdfTerm::QuotedTriple(Box::new(triple))
    }

    /// Get string representation of the term
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RdfTerm::Iri(iri) => Some(iri.as_str()),
            RdfTerm::BlankNode(id) => Some(id),
            RdfTerm::Literal { value, .. } => Some(value),
            RdfTerm::QuotedTriple(_) => None, // Quoted triples don't have simple string representation
        }
    }

    /// Convert to RDF 1.1 compatible term by stripping RDF-star features
    /// Quoted triples are converted to blank nodes
    pub fn to_rdf11(&self) -> Self {
        match self {
            RdfTerm::QuotedTriple(triple) => {
                // Convert quoted triple to a blank node for RDF 1.1 compatibility
                // The actual reification should be done at the graph level
                RdfTerm::BlankNode(format!("_:qt_{}", triple.hash_id()))
            }
            other => other.clone(),
        }
    }
}

/// RDF Graph - a set of RDF triples with RDF-star support
#[derive(Debug, Clone)]
pub struct RdfGraph {
    triples: HashSet<Triple>,
    blank_node_counter: u64,
    /// Track RDF version for compatibility
    rdf_version: RdfVersion,
}

/// RDF Version for compatibility tracking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RdfVersion {
    /// RDF 1.1 (no RDF-star features)
    RDF11,
    /// RDF 1.2 (includes directionality, rdf:reifies)
    RDF12,
    /// RDF-star (includes quoted triples)
    RDFStar,
}

impl Default for RdfVersion {
    fn default() -> Self {
        RdfVersion::RDFStar // Default to most feature-complete version
    }
}

impl RdfGraph {
    /// Create a new empty RDF graph with RDF-star support
    pub fn new() -> Self {
        Self {
            triples: HashSet::new(),
            blank_node_counter: 0,
            rdf_version: RdfVersion::default(),
        }
    }

    /// Create a new RDF graph with specified version
    pub fn with_version(version: RdfVersion) -> Self {
        Self {
            triples: HashSet::new(),
            blank_node_counter: 0,
            rdf_version: version,
        }
    }

    /// Get the RDF version of this graph
    pub fn rdf_version(&self) -> RdfVersion {
        self.rdf_version
    }

    /// Set the RDF version of this graph
    pub fn set_rdf_version(&mut self, version: RdfVersion) {
        self.rdf_version = version;
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

    /// Extract all quoted triples from the graph (including nested ones)
    pub fn extract_quoted_triples(&self) -> Vec<Triple> {
        let mut result = Vec::new();
        for triple in &self.triples {
            // Check subject
            if let RdfTerm::QuotedTriple(qt) = &triple.subject {
                result.extend(qt.flatten());
            }
            // Check predicate (unusual but possible)
            if let RdfTerm::QuotedTriple(qt) = &triple.predicate {
                result.extend(qt.flatten());
            }
            // Check object
            if let RdfTerm::QuotedTriple(qt) = &triple.object {
                result.extend(qt.flatten());
            }
        }
        result
    }

    /// Replace a quoted triple with another
    pub fn replace_quoted_triple(&mut self, old: &Triple, new: Triple) -> bool {
        let mut replaced = false;
        let mut triples_to_remove = Vec::new();
        let mut triples_to_add = Vec::new();

        for t in &self.triples {
            let mut updated = t.clone();
            let mut needs_update = false;

            if let RdfTerm::QuotedTriple(qt) = &updated.subject {
                if qt.as_ref() == old {
                    updated.subject = RdfTerm::quoted_triple(new.clone());
                    needs_update = true;
                }
            }
            if let RdfTerm::QuotedTriple(qt) = &updated.object {
                if qt.as_ref() == old {
                    updated.object = RdfTerm::quoted_triple(new.clone());
                    needs_update = true;
                }
            }

            if needs_update {
                replaced = true;
                triples_to_remove.push(t.clone());
                triples_to_add.push(updated);
            }
        }

        for old_triple in triples_to_remove {
            self.triples.remove(&old_triple);
        }
        for new_triple in triples_to_add {
            self.triples.insert(new_triple);
        }

        replaced
    }

    /// Count quoted triples in the graph
    pub fn quoted_triple_count(&self) -> usize {
        self.extract_quoted_triples().len()
    }

    /// Convert this graph to RDF 1.1 by stripping RDF-star features
    /// Returns a new graph with quoted triples converted to reification
    pub fn to_rdf11(&self) -> Result<RdfGraph> {
        let mut result = RdfGraph::with_version(RdfVersion::RDF11);
        let mut reification_counter = 0u64;

        for triple in &self.triples {
            // Check if triple contains quoted triples
            let has_quoted = triple.subject.is_quoted_triple()
                || triple.predicate.is_quoted_triple()
                || triple.object.is_quoted_triple();

            if !has_quoted {
                // Simple triple, just add it
                result.add_triple(triple.clone());
            } else {
                // Complex triple with quoted components
                // Need to reify the quoted triples
                let new_subject = if let RdfTerm::QuotedTriple(qt) = &triple.subject {
                    let stmt_id = format!("_:stmt{}", reification_counter);
                    reification_counter += 1;
                    let reified = qt.to_rdf11_reification(&stmt_id)?;
                    for t in reified {
                        result.add_triple(t);
                    }
                    RdfTerm::BlankNode(stmt_id)
                } else {
                    triple.subject.to_rdf11()
                };

                let new_object = if let RdfTerm::QuotedTriple(qt) = &triple.object {
                    let stmt_id = format!("_:stmt{}", reification_counter);
                    reification_counter += 1;
                    let reified = qt.to_rdf11_reification(&stmt_id)?;
                    for t in reified {
                        result.add_triple(t);
                    }
                    RdfTerm::BlankNode(stmt_id)
                } else {
                    triple.object.to_rdf11()
                };

                result.add_triple(Triple::new(
                    new_subject,
                    triple.predicate.to_rdf11(),
                    new_object,
                ));
            }
        }

        Ok(result)
    }

    /// Reify all quoted triples in this graph (in-place conversion toward RDF 1.1)
    pub fn reify_quoted_triples(&mut self) -> Result<()> {
        let rdf11_graph = self.to_rdf11()?;
        self.triples = rdf11_graph.triples;
        self.rdf_version = RdfVersion::RDF11;
        Ok(())
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
            RdfTerm::QuotedTriple(triple) => {
                // Turtle-star syntax: << subject predicate object >>
                write!(f, "<< {} {} {} >>", triple.subject, triple.predicate, triple.object)
            }
        }
    }
}

impl std::fmt::Display for Triple {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.subject, self.predicate, self.object)
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

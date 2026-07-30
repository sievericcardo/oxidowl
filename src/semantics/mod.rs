//! RDF and OWL Semantics Implementation
//!
//! This module implements the formal semantics for RDF, RDFS, and OWL 2 DL
//! according to the W3C specifications:
//! - RDF 1.1 Concepts: <https://www.w3.org/TR/rdf11-concepts>/
//! - RDF Schema: <https://www.w3.org/TR/rdf-schema>/
//! - OWL 2 Direct Semantics: <https://www.w3.org/TR/owl2-direct-semantics>/

pub mod entailment;
pub mod graph_isomorphism;
pub mod interpretation;
pub mod iri_validation;
pub mod owl_rdf_mapping;
pub mod owl2;
pub mod quoted_triple_optimizer;
pub mod rdf;
pub mod rdfs; // Re-enabled after fixing type system issues
pub mod skolemization;

// Re-export main types for convenience
pub use entailment::{EntailmentChecker, EntailmentRegime, Owl2RlEngine};
pub use interpretation::{Interpretation, InterpretationBuilder, InterpretationFactory};
pub use iri_validation::{IriValidationMode, IriValidator};
pub use owl2::{Owl2Interpretation, Owl2ReasoningEngine};
pub use quoted_triple_optimizer::{
    OptimizerStats, QuotedTripleCache, QuotedTripleInternPool, QuotedTripleOptimizer,
    QuotedTripleOptimizerConfig,
};
pub use rdf::{RdfSimpleEntailment, RdfSimpleInterpretation};
pub use rdfs::{RdfsEntailmentEngine, RdfsInterpretation}; // Re-enabled

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

/// RDF Triple representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Triple {
    pub subject: RdfTerm,
    pub predicate: RdfTerm,
    pub object: RdfTerm,
}

impl Triple {
    /// Create a new triple
    #[must_use]
    pub fn new(subject: RdfTerm, predicate: RdfTerm, object: RdfTerm) -> Self {
        Triple {
            subject,
            predicate,
            object,
        }
    }

    /// Calculate the nesting depth of this triple
    /// Returns 0 for flat triples, >0 for nested quoted/triple-term triples
    #[must_use]
    pub fn depth(&self) -> usize {
        let subject_depth = match &self.subject {
            RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) => 1 + t.depth(),
            _ => 0,
        };
        let object_depth = match &self.object {
            RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) => 1 + t.depth(),
            _ => 0,
        };
        // Predicates typically shouldn't be quoted/triple-term triples, but check anyway
        let predicate_depth = match &self.predicate {
            RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) => 1 + t.depth(),
            _ => 0,
        };
        subject_depth.max(object_depth).max(predicate_depth)
    }

    /// Flatten this triple to extract all nested triples
    /// Returns a vector containing this triple and all nested quoted triples
    #[must_use]
    pub fn flatten(&self) -> Vec<Triple> {
        let mut result = vec![self.clone()];

        // Extract from subject
        if let RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) = &self.subject {
            result.extend(t.flatten());
        }

        // Extract from predicate (unusual but possible)
        if let RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) = &self.predicate {
            result.extend(t.flatten());
        }

        // Extract from object
        if let RdfTerm::QuotedTriple(t) | RdfTerm::TripleTerm(t) = &self.object {
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
                .map_err(|e| Error::internal(format!("Invalid RDF type IRI: {e}")))?,
        );
        let rdf_statement = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement")
                .map_err(|e| Error::internal(format!("Invalid RDF Statement IRI: {e}")))?,
        );
        let rdf_subject = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#subject")
                .map_err(|e| Error::internal(format!("Invalid RDF subject IRI: {e}")))?,
        );
        let rdf_predicate = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate")
                .map_err(|e| Error::internal(format!("Invalid RDF predicate IRI: {e}")))?,
        );
        let rdf_object = RdfTerm::Iri(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#object")
                .map_err(|e| Error::internal(format!("Invalid RDF object IRI: {e}")))?,
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

/// RDF Term (IRI, Blank Node, Literal, Quoted Triple for RDF-star, or Triple Term for RDF 1.2)
#[derive(Debug, Clone, Serialize, Deserialize)]
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
        /// Base direction for dirLangString (RDF 1.2)
        /// Valid values: "ltr" (left-to-right) or "rtl" (right-to-left)
        direction: Option<String>,
    },
    /// Quoted triple (RDF-star support)
    /// Allows triples to be used as subjects *or* objects
    QuotedTriple(Box<Triple>),
    /// Triple term (RDF 1.2) — can ONLY appear in object position
    TripleTerm(Box<Triple>),
}

// Manual PartialEq: language tags are compared case-insensitively per RDF 1.2 §3.4
impl PartialEq for RdfTerm {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RdfTerm::Iri(a), RdfTerm::Iri(b)) => a == b,
            (RdfTerm::BlankNode(a), RdfTerm::BlankNode(b)) => a == b,
            (
                RdfTerm::Literal {
                    value: v1,
                    datatype: dt1,
                    language: lang1,
                    direction: dir1,
                },
                RdfTerm::Literal {
                    value: v2,
                    datatype: dt2,
                    language: lang2,
                    direction: dir2,
                },
            ) => {
                v1 == v2
                    && dt1 == dt2
                    && lang1.as_deref().map(str::to_ascii_lowercase)
                        == lang2.as_deref().map(str::to_ascii_lowercase)
                    && dir1 == dir2
            }
            (RdfTerm::QuotedTriple(a), RdfTerm::QuotedTriple(b)) => a == b,
            (RdfTerm::TripleTerm(a), RdfTerm::TripleTerm(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for RdfTerm {}

impl std::hash::Hash for RdfTerm {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            RdfTerm::Iri(url) => url.hash(state),
            RdfTerm::BlankNode(id) => id.hash(state),
            RdfTerm::Literal {
                value,
                datatype,
                language,
                direction,
            } => {
                value.hash(state);
                datatype.as_ref().map(Url::as_str).hash(state);
                // Hash the lowercased language tag for case-insensitive equality
                language.as_deref().map(str::to_ascii_lowercase).hash(state);
                direction.hash(state);
            }
            RdfTerm::QuotedTriple(triple) => triple.hash(state),
            RdfTerm::TripleTerm(triple) => triple.hash(state),
        }
    }
}

impl RdfTerm {
    /// Create an IRI term
    pub fn iri(iri: &str) -> Result<Self> {
        Ok(RdfTerm::Iri(Url::parse(iri).map_err(|e| {
            Error::ontology_parsing(format!("Invalid IRI: {e}"))
        })?))
    }

    /// Create a blank node term (lenient - accepts any string)
    #[must_use]
    pub fn blank_node(id: &str) -> Self {
        RdfTerm::BlankNode(id.to_string())
    }

    /// Create a blank node term with validation (RDF 1.2 well-formedness)
    /// Blank node labels must match: _:[A-Za-z0-9]+
    pub fn blank_node_validated(id: &str) -> Result<Self> {
        if let Some(label) = id.strip_prefix("_:") {
            if Self::is_valid_blank_node_label(label) {
                Ok(RdfTerm::BlankNode(id.to_string()))
            } else {
                Err(Error::ontology_parsing(format!(
                    "Invalid blank node label: '{id}'. Must contain only [A-Za-z0-9] characters"
                )))
            }
        } else {
            Err(Error::ontology_parsing(format!(
                "Invalid blank node format: '{id}'. Must start with '_:'"
            )))
        }
    }

    /// Validate a blank node label (without the _: prefix)
    /// RDF 1.2 requires labels to contain only [A-Za-z0-9]
    #[must_use]
    pub fn is_valid_blank_node_label(label: &str) -> bool {
        !label.is_empty() && label.chars().all(|c| c.is_ascii_alphanumeric())
    }

    /// Validate a full blank node identifier (with _: prefix)
    #[must_use]
    pub fn is_valid_blank_node(id: &str) -> bool {
        if let Some(label) = id.strip_prefix("_:") {
            Self::is_valid_blank_node_label(label)
        } else {
            false
        }
    }

    /// Create a literal term
    #[must_use]
    pub fn literal(value: &str) -> Self {
        RdfTerm::Literal {
            value: value.to_string(),
            datatype: None,
            language: None,
            direction: None,
        }
    }

    /// Create a typed literal
    pub fn typed_literal(value: &str, datatype: &str) -> Result<Self> {
        Ok(RdfTerm::Literal {
            value: value.to_string(),
            datatype: Some(
                Url::parse(datatype)
                    .map_err(|e| Error::ontology_parsing(format!("Invalid datatype IRI: {e}")))?,
            ),
            language: None,
            direction: None,
        })
    }

    /// Create a language-tagged literal
    #[must_use]
    pub fn language_literal(value: &str, language: &str) -> Self {
        RdfTerm::Literal {
            value: value.to_string(),
            datatype: None,
            language: Some(language.to_string()),
            direction: None,
        }
    }

    /// Create a dirLangString literal (RDF 1.2)
    /// Direction must be "ltr" or "rtl"
    pub fn dir_lang_string(value: &str, language: &str, direction: &str) -> Result<Self> {
        // Validate direction
        if direction != "ltr" && direction != "rtl" {
            return Err(Error::ontology_parsing(format!(
                "Invalid direction for dirLangString: '{direction}'. Must be 'ltr' or 'rtl'"
            )));
        }

        Ok(RdfTerm::Literal {
            value: value.to_string(),
            datatype: Some(
                Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString")
                    .expect("Valid RDF dirLangString IRI"),
            ),
            language: Some(language.to_string()),
            direction: Some(direction.to_string()),
        })
    }

    /// Check if term is an IRI
    #[must_use]
    pub fn is_iri(&self) -> bool {
        matches!(self, RdfTerm::Iri(_))
    }

    /// Check if term is a blank node
    #[must_use]
    pub fn is_blank_node(&self) -> bool {
        matches!(self, RdfTerm::BlankNode(_))
    }

    /// Check if term is a literal
    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(self, RdfTerm::Literal { .. })
    }

    /// Check if term is a quoted triple (RDF-star)
    #[must_use]
    pub fn is_quoted_triple(&self) -> bool {
        matches!(self, RdfTerm::QuotedTriple(_))
    }

    /// Check if term is a triple term (RDF 1.2 object-only embedded triple)
    #[must_use]
    pub fn is_triple_term(&self) -> bool {
        matches!(self, RdfTerm::TripleTerm(_))
    }

    /// Check if term is any embedded triple (QuotedTriple or TripleTerm)
    #[must_use]
    pub fn is_embedded_triple(&self) -> bool {
        matches!(self, RdfTerm::QuotedTriple(_) | RdfTerm::TripleTerm(_))
    }

    /// Get IRI if this is an IRI term
    #[must_use]
    pub fn as_iri(&self) -> Option<&Url> {
        match self {
            RdfTerm::Iri(iri) => Some(iri),
            _ => None,
        }
    }

    /// Get quoted triple if this is a quoted triple term
    #[must_use]
    pub fn as_quoted_triple(&self) -> Option<&Triple> {
        match self {
            RdfTerm::QuotedTriple(triple) => Some(triple),
            _ => None,
        }
    }

    /// Get inner triple if this is a triple term (RDF 1.2)
    #[must_use]
    pub fn as_triple_term(&self) -> Option<&Triple> {
        match self {
            RdfTerm::TripleTerm(triple) => Some(triple),
            _ => None,
        }
    }

    /// Get inner triple from either QuotedTriple or TripleTerm
    #[must_use]
    pub fn as_embedded_triple(&self) -> Option<&Triple> {
        match self {
            RdfTerm::QuotedTriple(triple) | RdfTerm::TripleTerm(triple) => Some(triple),
            _ => None,
        }
    }

    /// Create a triple term (RDF 1.2 — object position only)
    #[must_use]
    pub fn triple_term(triple: Triple) -> Self {
        RdfTerm::TripleTerm(Box::new(triple))
    }

    /// Create a quoted triple term
    #[must_use]
    pub fn quoted_triple(triple: Triple) -> Self {
        RdfTerm::QuotedTriple(Box::new(triple))
    }

    /// Get string representation of the term
    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        match self {
            RdfTerm::Iri(iri) => Some(iri.as_str()),
            RdfTerm::BlankNode(id) => Some(id),
            RdfTerm::Literal { value, .. } => Some(value),
            // Embedded triples don't have simple string representations
            RdfTerm::QuotedTriple(_) | RdfTerm::TripleTerm(_) => None,
        }
    }

    /// Convert to RDF 1.1 compatible term by stripping RDF-star features
    /// Quoted triples and triple terms are converted to blank nodes
    #[must_use]
    pub fn to_rdf11(&self) -> Self {
        match self {
            RdfTerm::QuotedTriple(triple) => {
                RdfTerm::BlankNode(format!("_:qt_{}", triple.hash_id()))
            }
            RdfTerm::TripleTerm(triple) => RdfTerm::BlankNode(format!("_:tt_{}", triple.hash_id())),
            other => other.clone(),
        }
    }
}

/// A reifying triple: a resource (`reifier`) that asserts something about an embedded triple.
///
/// Per RDF 1.2, this is represented as: `reifier rdf:reifies <<subject predicate object>>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReifyingTriple {
    /// The reifier — an IRI or blank node that reifies the triple term.
    pub reifier: RdfTerm,
    /// The triple being reified.
    pub triple_term: Triple,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RdfVersion {
    /// RDF 1.1 (no RDF-star features)
    RDF11,
    /// RDF 1.2 Basic (dirLangString + rdf:reifies, but no triple terms)
    RDF12Basic,
    /// RDF 1.2 (full — includes triple terms in object position)
    RDF12,
    /// RDF-star (includes quoted triples in subject+object)
    #[default]
    RDFStar,
}

impl std::fmt::Display for RdfVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RdfVersion::RDF11 => write!(f, "1.1"),
            RdfVersion::RDF12Basic => write!(f, "1.2-basic"),
            RdfVersion::RDF12 => write!(f, "1.2"),
            RdfVersion::RDFStar => write!(f, "rdf-star"),
        }
    }
}

impl std::str::FromStr for RdfVersion {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "1.1" => Ok(RdfVersion::RDF11),
            "1.2-basic" => Ok(RdfVersion::RDF12Basic),
            "1.2" => Ok(RdfVersion::RDF12),
            "rdf-star" | "rdf*" => Ok(RdfVersion::RDFStar),
            other => Err(Error::ontology_parsing(format!(
                "Unknown RDF version: '{other}'. Expected '1.1', '1.2-basic', '1.2', or 'rdf-star'"
            ))),
        }
    }
}

impl RdfGraph {
    /// Create a new empty RDF graph with RDF-star support
    #[must_use]
    pub fn new() -> Self {
        Self {
            triples: HashSet::new(),
            blank_node_counter: 0,
            rdf_version: RdfVersion::default(),
        }
    }

    /// Create a new RDF graph with specified version
    #[must_use]
    pub fn with_version(version: RdfVersion) -> Self {
        Self {
            triples: HashSet::new(),
            blank_node_counter: 0,
            rdf_version: version,
        }
    }

    /// Get the RDF version of this graph
    #[must_use]
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
    #[must_use]
    pub fn contains_triple(&self, triple: &Triple) -> bool {
        self.triples.contains(triple)
    }

    /// Get all triples in the graph
    #[must_use]
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
    #[must_use]
    pub fn subjects(&self) -> HashSet<&RdfTerm> {
        self.triples.iter().map(|t| &t.subject).collect()
    }

    /// Get all predicates in the graph
    #[must_use]
    pub fn predicates(&self) -> HashSet<&RdfTerm> {
        self.triples.iter().map(|t| &t.predicate).collect()
    }

    /// Get all objects in the graph
    #[must_use]
    pub fn objects(&self) -> HashSet<&RdfTerm> {
        self.triples.iter().map(|t| &t.object).collect()
    }

    /// Find triples matching a pattern (None means any)
    #[must_use]
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

    /// Extract all quoted triples and triple terms from the graph (including nested ones)
    #[must_use]
    pub fn extract_quoted_triples(&self) -> Vec<Triple> {
        let mut result = Vec::new();
        for triple in &self.triples {
            // Check subject
            if let RdfTerm::QuotedTriple(qt) | RdfTerm::TripleTerm(qt) = &triple.subject {
                result.extend(qt.flatten());
            }
            // Check predicate (unusual but possible)
            if let RdfTerm::QuotedTriple(qt) | RdfTerm::TripleTerm(qt) = &triple.predicate {
                result.extend(qt.flatten());
            }
            // Check object
            if let RdfTerm::QuotedTriple(qt) | RdfTerm::TripleTerm(qt) = &triple.object {
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

            if let RdfTerm::QuotedTriple(qt) = &updated.subject
                && qt.as_ref() == old
            {
                updated.subject = RdfTerm::quoted_triple(new.clone());
                needs_update = true;
            }
            if let RdfTerm::TripleTerm(qt) = &updated.subject
                && qt.as_ref() == old
            {
                updated.subject = RdfTerm::triple_term(new.clone());
                needs_update = true;
            }
            if let RdfTerm::QuotedTriple(qt) = &updated.object
                && qt.as_ref() == old
            {
                updated.object = RdfTerm::quoted_triple(new.clone());
                needs_update = true;
            }
            if let RdfTerm::TripleTerm(qt) = &updated.object
                && qt.as_ref() == old
            {
                updated.object = RdfTerm::triple_term(new.clone());
                needs_update = true;
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
    #[must_use]
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
                || triple.subject.is_triple_term()
                || triple.predicate.is_quoted_triple()
                || triple.predicate.is_triple_term()
                || triple.object.is_quoted_triple()
                || triple.object.is_triple_term();

            if !has_quoted {
                // Simple triple, just add it
                result.add_triple(triple.clone());
            } else {
                // Complex triple with quoted/triple-term components
                // Need to reify the quoted triples
                let new_subject =
                    if let RdfTerm::QuotedTriple(qt) | RdfTerm::TripleTerm(qt) = &triple.subject {
                        let stmt_id = format!("_:stmt{reification_counter}");
                        reification_counter += 1;
                        let reified = qt.to_rdf11_reification(&stmt_id)?;
                        for t in reified {
                            result.add_triple(t);
                        }
                        RdfTerm::BlankNode(stmt_id)
                    } else {
                        triple.subject.to_rdf11()
                    };

                let new_object =
                    if let RdfTerm::QuotedTriple(qt) | RdfTerm::TripleTerm(qt) = &triple.object {
                        let stmt_id = format!("_:stmt{reification_counter}");
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

    /// Detect the minimum RDF version required to represent this graph's contents.
    ///
    /// - Returns `RDF12` if any triple term (`TripleTerm` in object position) is present.
    /// - Returns `RDF12` if any dirLangString (literal with `direction`) is present.
    /// - Returns `RDFStar` if any quoted triple (`QuotedTriple`) is present.
    /// - Returns `RDF11` otherwise.
    #[must_use]
    pub fn detect_version(&self) -> RdfVersion {
        let mut has_quoted = false;
        let mut has_rdf12 = false;

        for triple in &self.triples {
            for term in [&triple.subject, &triple.predicate, &triple.object] {
                match term {
                    RdfTerm::TripleTerm(_) => has_rdf12 = true,
                    RdfTerm::QuotedTriple(_) => has_quoted = true,
                    RdfTerm::Literal { direction, .. } if direction.is_some() => {
                        has_rdf12 = true;
                    }
                    _ => {}
                }
            }
        }

        if has_rdf12 {
            RdfVersion::RDF12
        } else if has_quoted {
            RdfVersion::RDFStar
        } else {
            RdfVersion::RDF11
        }
    }

    /// Return all reifying triples — triples where predicate is `rdf:reifies`
    /// and the object is an embedded triple (triple term or quoted triple).
    #[must_use]
    pub fn reifying_triples(&self) -> Vec<ReifyingTriple> {
        use vocabulary::RDF_REIFIES;
        let reifies_term = RdfTerm::Iri(RDF_REIFIES.clone());
        self.triples
            .iter()
            .filter(|t| t.predicate == reifies_term)
            .filter_map(|t| {
                t.object.as_embedded_triple().map(|inner| ReifyingTriple {
                    reifier: t.subject.clone(),
                    triple_term: inner.clone(),
                })
            })
            .collect()
    }

    /// Add a reifying triple: `reifier rdf:reifies <<s p o>>`.
    ///
    /// `reifier` should be an IRI or blank node.
    /// The embedded triple is stored as a `TripleTerm`.
    pub fn add_reifying_triple(&mut self, reifier: RdfTerm, triple_term: Triple) {
        use vocabulary::RDF_REIFIES;
        let reifies_pred = RdfTerm::Iri(RDF_REIFIES.clone());
        let object = RdfTerm::TripleTerm(Box::new(triple_term));
        self.add_triple(Triple::new(reifier, reifies_pred, object));
    }

    /// Merge another graph into this one
    pub fn merge(&mut self, other: &RdfGraph) {
        for triple in &other.triples {
            self.triples.insert(triple.clone());
        }
    }

    /// Get the size of the graph (number of triples)
    #[must_use]
    pub fn size(&self) -> usize {
        self.triples.len()
    }

    /// Check if graph is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.triples.is_empty()
    }

    /// Serialize graph to N-Triples format
    /// Supports N-Triples-star for quoted triples (RDF-star)
    #[must_use]
    pub fn to_ntriples(&self) -> String {
        let mut result = String::new();
        for triple in &self.triples {
            result.push_str(&format!(
                "{} {} {} .\n",
                triple.subject, triple.predicate, triple.object
            ));
        }
        result
    }

    /// Serialize graph to Turtle format
    /// Supports Turtle-star for quoted triples (RDF-star)
    #[must_use]
    pub fn to_turtle(&self) -> String {
        let mut result = String::new();

        // Add standard prefixes
        result.push_str("@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .\n");
        result.push_str("@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n");
        result.push_str("@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .\n");
        result.push('\n');

        // Serialize triples
        for triple in &self.triples {
            result.push_str(&format!(
                "{} {} {} .\n",
                triple.subject, triple.predicate, triple.object
            ));
        }
        result
    }

    /// Serialize graph to Turtle-star format (explicit RDF-star)
    /// Alias for `to_turtle()` since RDF-star syntax is automatically used when needed
    #[must_use]
    pub fn to_turtle_star(&self) -> String {
        self.to_turtle()
    }

    /// Serialize graph to N-Triples-star format (explicit RDF-star)
    /// Alias for `to_ntriples()` since RDF-star syntax is automatically used when needed
    #[must_use]
    pub fn to_ntriples_star(&self) -> String {
        self.to_ntriples()
    }

    /// Serialize graph to RDF/XML format
    /// Supports RDF 1.2 features like rdf:reifies and dirLangString
    #[must_use]
    pub fn to_rdf_xml(&self) -> String {
        let mut result = String::new();

        // XML header
        result.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        result.push_str("<rdf:RDF\n");
        result.push_str("    xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"\n");
        result.push_str("    xmlns:rdfs=\"http://www.w3.org/2000/01/rdf-schema#\">\n");
        result.push('\n');

        // Group triples by subject for better RDF/XML structure
        use std::collections::HashMap;
        let mut subject_map: HashMap<String, Vec<(&RdfTerm, &RdfTerm)>> = HashMap::new();

        for triple in &self.triples {
            let subject_str = format!("{}", triple.subject);
            subject_map
                .entry(subject_str)
                .or_default()
                .push((&triple.predicate, &triple.object));
        }

        // Serialize each subject's description
        for (subject, pred_obj_pairs) in subject_map {
            // Handle quoted triples specially - convert to reification
            if subject.starts_with("<<") {
                // For RDF-star in RDF/XML, use reification
                result.push_str("  <!-- RDF-star quoted triple represented as reification -->\n");
                result.push_str(&format!(
                    "  <rdf:Description rdf:nodeID=\"{}\">\n",
                    self.hash_string(&subject)
                ));
                for (pred, obj) in pred_obj_pairs {
                    result.push_str(&self.serialize_predicate_object_xml(pred, obj));
                }
                result.push_str("  </rdf:Description>\n\n");
            } else if let Some(node_id) = subject.strip_prefix("_:") {
                // Blank node
                // Remove "_:" prefix
                result.push_str(&format!("  <rdf:Description rdf:nodeID=\"{node_id}\">\n"));
                for (pred, obj) in pred_obj_pairs {
                    result.push_str(&self.serialize_predicate_object_xml(pred, obj));
                }
                result.push_str("  </rdf:Description>\n\n");
            } else {
                // IRI resource
                let iri = subject.trim_start_matches('<').trim_end_matches('>');
                result.push_str(&format!("  <rdf:Description rdf:about=\"{iri}\">\n"));
                for (pred, obj) in pred_obj_pairs {
                    result.push_str(&self.serialize_predicate_object_xml(pred, obj));
                }
                result.push_str("  </rdf:Description>\n\n");
            }
        }

        result.push_str("</rdf:RDF>\n");
        result
    }

    /// Helper to serialize predicate-object pair in RDF/XML
    fn serialize_predicate_object_xml(&self, predicate: &RdfTerm, object: &RdfTerm) -> String {
        let pred_iri = match predicate {
            RdfTerm::Iri(url) => url.as_str(),
            _ => return String::new(), // Invalid predicate
        };

        // Extract namespace and local name
        let (_ns, local) = self.split_iri(pred_iri);

        match object {
            RdfTerm::Iri(url) => {
                format!("    <{local} rdf:resource=\"{url}\" />\n")
            }
            RdfTerm::BlankNode(id) => {
                let node_id = id.strip_prefix("_:").unwrap_or(id);
                format!("    <{local} rdf:nodeID=\"{node_id}\" />\n")
            }
            RdfTerm::Literal {
                value,
                datatype,
                language,
                direction,
            } => {
                if let Some(dir) = direction {
                    // RDF 1.2 dirLangString
                    if let Some(lang) = language {
                        format!(
                            "    <{}>{}</{}>  <!-- lang: {}, dir: {} -->\n",
                            local,
                            Self::xml_escape(value),
                            local,
                            lang,
                            dir
                        )
                    } else {
                        format!("    <{}>{}</{}>\n", local, Self::xml_escape(value), local)
                    }
                } else if let Some(lang) = language {
                    format!(
                        "    <{} xml:lang=\"{}\">{}</{}>\n",
                        local,
                        lang,
                        Self::xml_escape(value),
                        local
                    )
                } else if let Some(dt) = datatype {
                    format!(
                        "    <{} rdf:datatype=\"{}\">{}</{}>\n",
                        local,
                        dt,
                        Self::xml_escape(value),
                        local
                    )
                } else {
                    format!("    <{}>{}</{}>\n", local, Self::xml_escape(value), local)
                }
            }
            RdfTerm::QuotedTriple(_) | RdfTerm::TripleTerm(_) => {
                // RDF-star/RDF-1.2 embedded triple — represent as blank node reference
                format!(
                    "    <{} rdf:nodeID=\"qt_{}\" />\n",
                    local,
                    self.hash_term(object)
                )
            }
        }
    }

    /// Helper to split IRI into namespace and local name
    fn split_iri(&self, iri: &str) -> (String, String) {
        if let Some(pos) = iri.rfind('#') {
            (iri[..=pos].to_string(), iri[pos + 1..].to_string())
        } else if let Some(pos) = iri.rfind('/') {
            (iri[..=pos].to_string(), iri[pos + 1..].to_string())
        } else {
            (String::new(), iri.to_string())
        }
    }

    /// Helper to escape XML special characters
    fn xml_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&apos;")
    }

    /// Helper to hash a string for generating unique IDs
    fn hash_string(&self, s: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        format!("n{}", hasher.finish())
    }

    /// Helper to hash a term for generating unique IDs
    fn hash_term(&self, term: &RdfTerm) -> String {
        self.hash_string(&format!("{term}"))
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
        pub static ref RDF_STATEMENT: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement")
            .expect("Valid hardcoded RDF Statement URL");
        pub static ref RDF_SUBJECT: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#subject")
            .expect("Valid hardcoded RDF subject URL");
        pub static ref RDF_PREDICATE: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate")
            .expect("Valid hardcoded RDF predicate URL");
        pub static ref RDF_OBJECT: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#object")
            .expect("Valid hardcoded RDF object URL");
        pub static ref RDF_REIFIES: Url = Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies")
            .expect("Valid hardcoded RDF reifies URL");

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
            RdfTerm::Iri(url) => write!(f, "<{url}>"),
            RdfTerm::BlankNode(id) => write!(f, "_:{id}"),
            RdfTerm::Literal {
                value,
                datatype,
                language,
                direction,
            } => {
                if let Some(dir) = direction {
                    // RDF 1.2 dirLangString: "value"@lang--dir
                    if let Some(lang) = language {
                        write!(f, "\"{value}\"@{lang}--{dir}")
                    } else {
                        // dirLangString requires language tag
                        write!(f, "\"{value}\"")
                    }
                } else if let Some(lang) = language {
                    write!(f, "\"{value}\"@{lang}")
                } else if let Some(dt) = datatype {
                    write!(f, "\"{value}\"^^<{dt}>")
                } else {
                    write!(f, "\"{value}\"")
                }
            }
            RdfTerm::QuotedTriple(triple) => {
                // Turtle-star syntax: << subject predicate object >>
                write!(
                    f,
                    "<< {} {} {} >>",
                    triple.subject, triple.predicate, triple.object
                )
            }
            RdfTerm::TripleTerm(triple) => {
                // RDF 1.2 triple term (object-only): << subject predicate object >>
                write!(
                    f,
                    "<< {} {} {} >>",
                    triple.subject, triple.predicate, triple.object
                )
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

    // ── RDF 1.2 compliance tests ──────────────────────────────────────────────

    #[test]
    fn test_language_tag_case_insensitive_equality() {
        // RDF 1.2 §3.4: language tags are case-insensitive
        let a = RdfTerm::language_literal("chat", "fr");
        let b = RdfTerm::language_literal("chat", "FR");
        let c = RdfTerm::language_literal("chat", "Fr");
        assert_eq!(a, b, "\"chat\"@fr must equal \"chat\"@FR");
        assert_eq!(a, c, "\"chat\"@fr must equal \"chat\"@Fr");
        assert_eq!(b, c, "\"chat\"@FR must equal \"chat\"@Fr");
    }

    #[test]
    fn test_language_tag_case_insensitive_hash() {
        use std::collections::HashSet;
        let a = RdfTerm::language_literal("hello", "en");
        let b = RdfTerm::language_literal("hello", "EN");
        let mut set = HashSet::new();
        set.insert(a);
        // Inserting an equal term should not grow the set
        assert!(
            !set.insert(b),
            "Hash must be consistent with case-insensitive equality"
        );
    }

    #[test]
    fn test_triple_term_variant() {
        let s = RdfTerm::iri("http://example.org/s").unwrap();
        let p = RdfTerm::iri("http://example.org/p").unwrap();
        let o = RdfTerm::iri("http://example.org/o").unwrap();
        let inner = Triple::new(s, p, o);

        let tt = RdfTerm::triple_term(inner.clone());
        assert!(tt.is_triple_term());
        assert!(!tt.is_quoted_triple());
        assert!(tt.is_embedded_triple());
        assert_eq!(tt.as_triple_term(), Some(&inner));
    }

    #[test]
    fn test_reifying_triple_helpers() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/s").unwrap();
        let p = RdfTerm::iri("http://example.org/p").unwrap();
        let o = RdfTerm::iri("http://example.org/o").unwrap();
        let inner = Triple::new(s, p, o);

        let reifier = RdfTerm::iri("http://example.org/reifier").unwrap();
        graph.add_reifying_triple(reifier.clone(), inner.clone());

        let reifying = graph.reifying_triples();
        assert_eq!(reifying.len(), 1);
        assert_eq!(reifying[0].reifier, reifier);
        assert_eq!(reifying[0].triple_term, inner);
    }

    #[test]
    fn test_rdf_version_display() {
        assert_eq!(RdfVersion::RDF11.to_string(), "1.1");
        assert_eq!(RdfVersion::RDF12Basic.to_string(), "1.2-basic");
        assert_eq!(RdfVersion::RDF12.to_string(), "1.2");
        assert_eq!(RdfVersion::RDFStar.to_string(), "rdf-star");
    }

    #[test]
    fn test_rdf_version_from_str() {
        use std::str::FromStr;
        assert_eq!(RdfVersion::from_str("1.1").unwrap(), RdfVersion::RDF11);
        assert_eq!(
            RdfVersion::from_str("1.2-basic").unwrap(),
            RdfVersion::RDF12Basic
        );
        assert_eq!(RdfVersion::from_str("1.2").unwrap(), RdfVersion::RDF12);
        assert_eq!(
            RdfVersion::from_str("rdf-star").unwrap(),
            RdfVersion::RDFStar
        );
        assert!(RdfVersion::from_str("2.0").is_err());
    }

    #[test]
    fn test_detect_version_rdf11() {
        let mut graph = RdfGraph::new();
        let s = RdfTerm::iri("http://example.org/s").unwrap();
        let p = RdfTerm::iri("http://example.org/p").unwrap();
        let o = RdfTerm::literal("hello");
        graph.add_triple(Triple::new(s, p, o));
        assert_eq!(graph.detect_version(), RdfVersion::RDF11);
    }

    #[test]
    fn test_detect_version_rdfstar() {
        let mut graph = RdfGraph::new();
        let s = RdfTerm::iri("http://example.org/s").unwrap();
        let p = RdfTerm::iri("http://example.org/p").unwrap();
        let o = RdfTerm::iri("http://example.org/o").unwrap();
        let inner = Triple::new(s.clone(), p.clone(), o.clone());
        let qt = RdfTerm::quoted_triple(inner);
        graph.add_triple(Triple::new(qt, p, o));
        assert_eq!(graph.detect_version(), RdfVersion::RDFStar);
    }

    #[test]
    fn test_detect_version_rdf12_triple_term() {
        let mut graph = RdfGraph::new();
        let s = RdfTerm::iri("http://example.org/s").unwrap();
        let p = RdfTerm::iri("http://example.org/p").unwrap();
        let o = RdfTerm::iri("http://example.org/o").unwrap();
        let inner = Triple::new(s, p.clone(), o.clone());
        let tt = RdfTerm::triple_term(inner);
        graph.add_triple(Triple::new(tt, p, o));
        assert_eq!(graph.detect_version(), RdfVersion::RDF12);
    }

    #[test]
    fn test_detect_version_rdf12_dir_lang_string() {
        let mut graph = RdfGraph::new();
        let s = RdfTerm::iri("http://example.org/s").unwrap();
        let p = RdfTerm::iri("http://example.org/p").unwrap();
        let o = RdfTerm::dir_lang_string("مرحبا", "ar", "rtl").unwrap();
        graph.add_triple(Triple::new(s, p, o));
        assert_eq!(graph.detect_version(), RdfVersion::RDF12);
    }

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

    #[test]
    fn test_dir_lang_string_creation() {
        // Test creating dirLangString with valid direction
        let term_ltr = RdfTerm::dir_lang_string("Hello", "en", "ltr");
        assert!(term_ltr.is_ok());

        let term_rtl = RdfTerm::dir_lang_string("مرحبا", "ar", "rtl");
        assert!(term_rtl.is_ok());

        // Test invalid direction
        let term_invalid = RdfTerm::dir_lang_string("Hello", "en", "invalid");
        assert!(term_invalid.is_err());
    }

    #[test]
    fn test_dir_lang_string_display() {
        // Test display format for dirLangString: "value"@lang--dir
        let term = RdfTerm::dir_lang_string("Hello", "en", "ltr").expect("Valid dirLangString");
        let display = format!("{}", term);
        assert_eq!(display, "\"Hello\"@en--ltr");

        let term_rtl = RdfTerm::dir_lang_string("مرحبا", "ar", "rtl").expect("Valid dirLangString");
        let display_rtl = format!("{}", term_rtl);
        assert_eq!(display_rtl, "\"مرحبا\"@ar--rtl");
    }

    #[test]
    fn test_blank_node_validation_valid() {
        // Test valid blank node labels (RDF 1.2 well-formedness)
        assert!(RdfTerm::is_valid_blank_node("_:node1"));
        assert!(RdfTerm::is_valid_blank_node("_:a"));
        assert!(RdfTerm::is_valid_blank_node("_:Z"));
        assert!(RdfTerm::is_valid_blank_node("_:node123"));
        assert!(RdfTerm::is_valid_blank_node("_:ABC123xyz"));

        // Test label validation (without _: prefix)
        assert!(RdfTerm::is_valid_blank_node_label("node1"));
        assert!(RdfTerm::is_valid_blank_node_label("a"));
        assert!(RdfTerm::is_valid_blank_node_label("123"));

        // Test validated constructor
        assert!(RdfTerm::blank_node_validated("_:valid123").is_ok());
    }

    #[test]
    fn test_blank_node_validation_invalid() {
        // Test invalid blank node labels (RDF 1.2 well-formedness)
        assert!(!RdfTerm::is_valid_blank_node("_:node-1")); // Hyphen not allowed
        assert!(!RdfTerm::is_valid_blank_node("_:node_1")); // Underscore not allowed
        assert!(!RdfTerm::is_valid_blank_node("_:node.1")); // Dot not allowed
        assert!(!RdfTerm::is_valid_blank_node("_:node:1")); // Colon not allowed
        assert!(!RdfTerm::is_valid_blank_node("_:")); // Empty label
        assert!(!RdfTerm::is_valid_blank_node("node1")); // Missing _: prefix
        assert!(!RdfTerm::is_valid_blank_node("_:café")); // Non-ASCII

        // Test label validation
        assert!(!RdfTerm::is_valid_blank_node_label("")); // Empty
        assert!(!RdfTerm::is_valid_blank_node_label("node-1")); // Hyphen
        assert!(!RdfTerm::is_valid_blank_node_label("node_1")); // Underscore

        // Test validated constructor
        assert!(RdfTerm::blank_node_validated("_:invalid-node").is_err());
        assert!(RdfTerm::blank_node_validated("_:").is_err());
        assert!(RdfTerm::blank_node_validated("invalid").is_err());
    }

    #[test]
    fn test_blank_node_lenient_constructor() {
        // The lenient blank_node() constructor should accept any string
        // This is for RDF 1.1 backward compatibility
        let node1 = RdfTerm::blank_node("_:node-with-hyphens");
        assert!(node1.is_blank_node());

        let node2 = RdfTerm::blank_node("_:node_with_underscores");
        assert!(node2.is_blank_node());
    }

    #[test]
    fn test_blank_node_validated_error_messages() {
        // Test that error messages are informative
        let result = RdfTerm::blank_node_validated("_:invalid-node");
        assert!(result.is_err());
        let err_msg = format!("{}", result.unwrap_err());
        assert!(err_msg.contains("Invalid blank node label"));
        assert!(err_msg.contains("A-Za-z0-9"));

        let result2 = RdfTerm::blank_node_validated("invalid");
        assert!(result2.is_err());
        let err_msg2 = format!("{}", result2.unwrap_err());
        assert!(err_msg2.contains("Must start with '_:'"));
    }

    #[test]
    fn test_dir_lang_string_has_correct_datatype() {
        let term = RdfTerm::dir_lang_string("Hello", "en", "ltr").expect("Valid dirLangString");

        match term {
            RdfTerm::Literal {
                datatype,
                language,
                direction,
                ..
            } => {
                assert_eq!(
                    datatype.as_ref().map(|u| u.as_str()),
                    Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString")
                );
                assert_eq!(language.as_deref(), Some("en"));
                assert_eq!(direction.as_deref(), Some("ltr"));
            }
            _ => panic!("Expected Literal term"),
        }
    }

    #[test]
    fn test_regular_language_literal_vs_dir_lang_string() {
        // Regular language tag without direction
        let regular = RdfTerm::language_literal("Hello", "en");
        let regular_display = format!("{}", regular);
        assert_eq!(regular_display, "\"Hello\"@en");

        // dirLangString with direction
        let dir_lang = RdfTerm::dir_lang_string("Hello", "en", "ltr").expect("Valid dirLangString");
        let dir_lang_display = format!("{}", dir_lang);
        assert_eq!(dir_lang_display, "\"Hello\"@en--ltr");

        // These should be different
        assert_ne!(regular_display, dir_lang_display);
    }

    #[test]
    fn test_ntriples_serialization_simple() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/alice").unwrap();
        let p = RdfTerm::iri("http://example.org/knows").unwrap();
        let o = RdfTerm::iri("http://example.org/bob").unwrap();

        graph.add_triple(Triple::new(s, p, o));

        let ntriples = graph.to_ntriples();
        assert!(ntriples.contains("<http://example.org/alice>"));
        assert!(ntriples.contains("<http://example.org/knows>"));
        assert!(ntriples.contains("<http://example.org/bob>"));
        assert!(ntriples.ends_with(" .\n"));
    }

    #[test]
    fn test_ntriples_serialization_rdfstar() {
        let mut graph = RdfGraph::new();

        // Create a quoted triple: << :alice :knows :bob >>
        let inner_s = RdfTerm::iri("http://example.org/alice").unwrap();
        let inner_p = RdfTerm::iri("http://example.org/knows").unwrap();
        let inner_o = RdfTerm::iri("http://example.org/bob").unwrap();
        let inner_triple = Triple::new(inner_s, inner_p, inner_o);

        // Use quoted triple as subject
        let qt = RdfTerm::quoted_triple(inner_triple);
        let p = RdfTerm::iri("http://example.org/certainty").unwrap();
        let o = RdfTerm::literal("high");

        graph.add_triple(Triple::new(qt, p, o));

        let ntriples = graph.to_ntriples();
        assert!(ntriples.contains("<<"));
        assert!(ntriples.contains(">>"));
        assert!(ntriples.contains("certainty"));
        assert!(ntriples.contains("\"high\""));
    }

    #[test]
    fn test_turtle_serialization_simple() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/alice").unwrap();
        let p = RdfTerm::iri("http://example.org/knows").unwrap();
        let o = RdfTerm::iri("http://example.org/bob").unwrap();

        graph.add_triple(Triple::new(s, p, o));

        let turtle = graph.to_turtle();
        assert!(turtle.contains("@prefix"));
        assert!(turtle.contains("rdf:"));
        assert!(turtle.contains("<http://example.org/alice>"));
        assert!(turtle.contains("<http://example.org/knows>"));
        assert!(turtle.contains("<http://example.org/bob>"));
    }

    #[test]
    fn test_turtle_serialization_with_literals() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/alice").unwrap();
        let p1 = RdfTerm::iri("http://example.org/name").unwrap();
        let o1 = RdfTerm::literal("Alice");

        let p2 = RdfTerm::iri("http://example.org/age").unwrap();
        let o2 = RdfTerm::typed_literal("30", "http://www.w3.org/2001/XMLSchema#integer").unwrap();

        graph.add_triple(Triple::new(s.clone(), p1, o1));
        graph.add_triple(Triple::new(s, p2, o2));

        let turtle = graph.to_turtle();
        assert!(turtle.contains("\"Alice\""));
        assert!(turtle.contains("\"30\""));
        assert!(turtle.contains("integer"));
    }

    #[test]
    fn test_turtle_serialization_dirlangstring() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/greeting").unwrap();
        let p = RdfTerm::iri("http://example.org/text").unwrap();
        let o = RdfTerm::dir_lang_string("مرحبا", "ar", "rtl").unwrap();

        graph.add_triple(Triple::new(s, p, o));

        let turtle = graph.to_turtle();
        assert!(turtle.contains("\"مرحبا\"@ar--rtl"));
    }

    #[test]
    fn test_rdf_xml_serialization_simple() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/alice").unwrap();
        let p = RdfTerm::iri("http://example.org/knows").unwrap();
        let o = RdfTerm::iri("http://example.org/bob").unwrap();

        graph.add_triple(Triple::new(s, p, o));

        let rdf_xml = graph.to_rdf_xml();
        assert!(rdf_xml.contains("<?xml"));
        assert!(rdf_xml.contains("<rdf:RDF"));
        assert!(rdf_xml.contains("xmlns:rdf"));
        assert!(rdf_xml.contains("rdf:Description"));
        assert!(rdf_xml.contains("http://example.org/alice"));
        assert!(rdf_xml.contains("</rdf:RDF>"));
    }

    #[test]
    fn test_rdf_xml_serialization_blank_nodes() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::blank_node("_:b1");
        let p = RdfTerm::iri("http://example.org/label").unwrap();
        let o = RdfTerm::literal("test");

        graph.add_triple(Triple::new(s, p, o));

        let rdf_xml = graph.to_rdf_xml();
        assert!(rdf_xml.contains("rdf:nodeID"));
        assert!(rdf_xml.contains("b1"));
    }

    #[test]
    fn test_rdf_xml_serialization_literals_with_language() {
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/doc").unwrap();
        let p = RdfTerm::iri("http://example.org/title").unwrap();
        let o = RdfTerm::language_literal("Hello", "en");

        graph.add_triple(Triple::new(s, p, o));

        let rdf_xml = graph.to_rdf_xml();
        assert!(rdf_xml.contains("xml:lang=\"en\""));
        assert!(rdf_xml.contains("Hello"));
    }

    #[test]
    fn test_serialization_round_trip_ntriples() {
        // Create a graph with various RDF features
        let mut graph = RdfGraph::new();

        let s = RdfTerm::iri("http://example.org/subject").unwrap();
        let p = RdfTerm::iri("http://example.org/predicate").unwrap();
        let o = RdfTerm::literal("object");

        graph.add_triple(Triple::new(s, p, o));

        let ntriples = graph.to_ntriples();

        // Verify serialization contains expected elements
        assert!(!ntriples.is_empty());
        assert!(ntriples.contains("<http://example.org/subject>"));
        assert!(ntriples.contains("<http://example.org/predicate>"));
        assert!(ntriples.contains("\"object\""));
        assert!(ntriples.ends_with(" .\n"));
    }

    #[test]
    fn test_serialization_preserves_rdfstar() {
        let mut graph = RdfGraph::new();

        // Create nested quoted triple
        let inner_s = RdfTerm::iri("http://example.org/a").unwrap();
        let inner_p = RdfTerm::iri("http://example.org/b").unwrap();
        let inner_o = RdfTerm::iri("http://example.org/c").unwrap();
        let inner = Triple::new(inner_s, inner_p, inner_o);

        let qt = RdfTerm::quoted_triple(inner);
        let p = RdfTerm::iri("http://example.org/certainty").unwrap();
        let o = RdfTerm::literal("0.9");

        graph.add_triple(Triple::new(qt, p, o));

        // Both formats should preserve RDF-star syntax
        let ntriples = graph.to_ntriples();
        assert!(ntriples.contains("<<"));
        assert!(ntriples.contains(">>"));

        let turtle = graph.to_turtle();
        assert!(turtle.contains("<<"));
        assert!(turtle.contains(">>"));
    }
}

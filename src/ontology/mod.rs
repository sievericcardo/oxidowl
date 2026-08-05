//! OWL 2 DL Ontology module
//!
//! This module provides the core ontology types and structures for OWL 2 DL reasoning.

use crate::{Error, Result};
use url::Url;

// ─── Global IRI intern pool (Phase 2.1) ─────────────────────────────────────

/// Global IRI intern pool — deduplicates `Arc<str>` allocations across all ontologies.
/// Maps IRI string → `Weak<str>` so interned IRIs are dropped when no longer referenced.
///
/// Guarded by the `cache` feature because it depends on `dashmap`.
#[cfg(feature = "cache")]
static IRI_INTERN_POOL: std::sync::OnceLock<dashmap::DashMap<Box<str>, std::sync::Weak<str>>> =
    std::sync::OnceLock::new();

#[cfg(feature = "cache")]
fn get_intern_pool() -> &'static dashmap::DashMap<Box<str>, std::sync::Weak<str>> {
    IRI_INTERN_POOL.get_or_init(dashmap::DashMap::new)
}

/// Intern an IRI string, returning a shared `Arc<str>`.
///
/// If an equal IRI already exists in the pool the existing `Arc` is returned,
/// avoiding a new heap allocation.  Stale `Weak` entries are evicted lazily
/// whenever a new value is inserted for the same key.
#[cfg(feature = "cache")]
#[must_use]
pub fn intern_iri(s: &str) -> std::sync::Arc<str> {
    let pool = get_intern_pool();
    // Fast path: check if already interned and still alive.
    if let Some(weak) = pool.get(s)
        && let Some(strong) = weak.upgrade()
    {
        return strong;
    }
    // Slow path: create new interned value and store the weak reference.
    let arc: std::sync::Arc<str> = std::sync::Arc::from(s);
    let weak = std::sync::Arc::downgrade(&arc);
    pool.insert(s.into(), weak);
    arc
}

pub mod axioms;
pub mod concepts;
pub mod datatypes;
pub mod indexes;
pub mod individuals;
pub mod owl_xml_vocabulary;
pub mod properties;
pub mod shortform;
pub mod vocabulary;

// Re-export main types
pub use axioms::*;
pub use concepts::*;
pub use datatypes::*;
pub use indexes::AxiomIndex;
pub use individuals::*;
pub use properties::*;

use std::sync::{Arc, RwLock};

/// Type alias for a thread-safe, shared ontology reference
///
/// This type represents an ontology that can be safely shared across threads
/// and allows for both read and write access through the `RwLock`.
pub type OntologyRef = Arc<RwLock<Ontology>>;

/// IRI (Internationalized Resource Identifier) wrapper
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct IRI {
    value: std::sync::Arc<str>,
}

impl IRI {
    /// Create a new IRI from a string.
    ///
    /// When the `cache` feature is enabled the string is deduplicated through
    /// the global IRI intern pool so that equal IRIs share a single `Arc`
    /// allocation.  Without the feature a fresh `Arc` is allocated each time.
    #[must_use]
    pub fn new(value: &str) -> Self {
        #[cfg(feature = "cache")]
        {
            Self {
                value: intern_iri(value),
            }
        }
        #[cfg(not(feature = "cache"))]
        {
            Self {
                value: std::sync::Arc::from(value),
            }
        }
    }

    /// Convert to URL
    pub fn to_url(&self) -> Result<Url> {
        Url::parse(&self.value)
            .map_err(|e| crate::Error::ontology_parsing(format!("Invalid IRI: {e}")))
    }

    /// Get the string value
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }

    /// Create IRI from URL
    #[must_use]
    pub fn from_url(url: Url) -> Self {
        Self {
            value: std::sync::Arc::from(url.to_string().as_str()),
        }
    }
}

impl From<String> for IRI {
    fn from(value: String) -> Self {
        Self {
            value: std::sync::Arc::from(value.as_str()),
        }
    }
}

impl From<Url> for IRI {
    fn from(url: Url) -> Self {
        Self {
            value: std::sync::Arc::from(url.to_string().as_str()),
        }
    }
}

impl std::fmt::Display for IRI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.value)
    }
}

impl std::fmt::Display for ObjectPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjectPropertyExpression::ObjectProperty(prop) => write!(f, "{}", prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => write!(f, "{}⁻", prop.iri),
            ObjectPropertyExpression::PropertyChain(chain) => {
                write!(f, "PropertyChain(")?;
                for (i, prop) in chain.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ∘ ")?;
                    }
                    write!(f, "{prop}")?;
                }
                write!(f, ")")
            }
        }
    }
}

impl ObjectPropertyExpression {
    /// Get the IRI if this is a simple object property
    #[must_use]
    pub fn iri(&self) -> Option<&IRI> {
        match self {
            ObjectPropertyExpression::ObjectProperty(prop) => Some(&prop.iri),
            ObjectPropertyExpression::InverseObjectProperty(prop) => Some(&prop.iri),
            ObjectPropertyExpression::PropertyChain(_) => None,
        }
    }
}

/// Object Property
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct ObjectProperty {
    pub iri: IRI,
}

/// Object property expressions (simple or complex)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ObjectPropertyExpression {
    /// Named object property
    ObjectProperty(ObjectProperty),

    /// Inverse object property
    InverseObjectProperty(ObjectProperty),

    /// Property chain (OWL 2 property composition)
    PropertyChain(Vec<ObjectPropertyExpression>),
}

/// Data Property
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct DataProperty {
    pub iri: IRI,
}

/// Named Datatype
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Datatype {
    pub iri: IRI,
}

/// Data property expressions (simple or complex)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DataPropertyExpression {
    /// Named data property
    DataProperty(DataProperty),
}

impl std::fmt::Display for DataPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataPropertyExpression::DataProperty(prop) => write!(f, "{}", prop.iri),
        }
    }
}

/// Annotation Property
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AnnotationProperty {
    pub iri: IRI,
}

/// Annotation property expressions
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationPropertyExpression {
    /// Named annotation property
    AnnotationProperty(AnnotationProperty),
}

impl std::fmt::Display for AnnotationPropertyExpression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnnotationPropertyExpression::AnnotationProperty(prop) => write!(f, "{}", prop.iri),
        }
    }
}

/// Literal value
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct Literal {
    /// Lexical value
    pub value: String,
    /// Language tag (if present)
    pub language: Option<String>,
    /// Datatype IRI
    pub datatype: Option<url::Url>,
}

impl Literal {
    /// Create a new literal with just a value
    #[must_use]
    pub fn new(value: String) -> Self {
        Self {
            value,
            language: None,
            datatype: None,
        }
    }

    /// Create a literal with a language tag
    #[must_use]
    pub fn with_language(value: String, language: String) -> Self {
        Self {
            value,
            language: Some(language),
            datatype: None,
        }
    }

    /// Create a literal with a datatype
    #[must_use]
    pub fn with_datatype(value: String, datatype: IRI) -> Self {
        Self {
            value,
            language: None,
            datatype: datatype.to_url().ok(),
        }
    }
}

impl std::fmt::Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "\"{}\"", self.value)?;
        if let Some(lang) = &self.language {
            write!(f, "@{lang}")?;
        } else if let Some(dt) = &self.datatype {
            write!(f, "^^<{dt}>")?;
        }
        Ok(())
    }
}

/// Data Range (OWL 2 Datatype expression)
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DataRange {
    /// Named datatype
    Datatype(IRI),
    /// Intersection of data ranges
    DataIntersectionOf(Vec<DataRange>),
    /// Union of data ranges
    DataUnionOf(Vec<DataRange>),
    /// Complement of a data range
    DataComplementOf(Box<DataRange>),
    /// Enumeration of literals
    DataOneOf(Vec<Literal>),
    /// Datatype restriction
    DatatypeRestriction {
        datatype: IRI,
        restrictions: Vec<FacetRestriction>,
    },
}

impl std::fmt::Display for DataRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataRange::Datatype(iri) => write!(f, "{iri}"),
            DataRange::DataIntersectionOf(ranges) => {
                write!(f, "(")?;
                for (i, range) in ranges.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊓ ")?;
                    }
                    write!(f, "{range}")?;
                }
                write!(f, ")")
            }
            DataRange::DataUnionOf(ranges) => {
                write!(f, "(")?;
                for (i, range) in ranges.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊔ ")?;
                    }
                    write!(f, "{range}")?;
                }
                write!(f, ")")
            }
            DataRange::DataComplementOf(range) => write!(f, "¬{range}"),
            DataRange::DataOneOf(literals) => {
                write!(f, "{{")?;
                for (i, literal) in literals.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{literal:?}")?;
                }
                write!(f, "}}")
            }
            DataRange::DatatypeRestriction {
                datatype,
                restrictions: _,
            } => {
                write!(f, "{datatype}[restrictions]")
            }
        }
    }
}

/// Facet restriction for datatype restrictions
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct FacetRestriction {
    pub facet: IRI,
    pub value: Literal,
}

/// Annotation subject
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationSubject {
    /// IRI
    IRI(IRI),
    /// Anonymous individual
    AnonymousIndividual(AnonymousIndividual),
}

/// Annotation value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AnnotationValue {
    /// IRI
    IRI(IRI),
    /// Anonymous individual
    AnonymousIndividual(AnonymousIndividual),
    /// Literal
    Literal(Literal),
}

/// Annotation
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Annotation {
    pub property: AnnotationProperty,
    pub value: AnnotationValue,
    pub annotations: Vec<Annotation>,
}

impl Annotation {
    /// Create a new annotation with annotations.
    #[must_use]
    pub fn new(
        property: AnnotationProperty,
        value: AnnotationValue,
        annotations: Vec<Annotation>,
    ) -> Self {
        Self {
            property,
            value,
            annotations,
        }
    }
}

/// Ontology signature containing all entities
#[derive(Debug, Clone, Default)]
pub struct Signature {
    /// All classes in the ontology
    pub classes: Vec<concepts::Class>,
    /// All object properties in the ontology
    pub object_properties: Vec<ObjectProperty>,
    /// All data properties in the ontology
    pub data_properties: Vec<DataProperty>,
    /// All individuals in the ontology
    pub individuals: Vec<individuals::Individual>,
}

impl Signature {
    /// Create a new empty signature
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

/// OWL Imports Declaration
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImportsDeclaration {
    pub imported_ontology_iri: IRI,
}

/// Ontology identity (OWL 2 OWLOntologyID)
#[derive(Debug, Clone)]
pub struct OntologyID {
    pub ontology_iri: Option<IRI>,
    pub version_iri: Option<IRI>,
    /// Unique identifier for distinguishing anonymous ontologies.
    /// Set to 0 for ontologies with an explicit IRI.
    internal_id: u64,
}

use std::sync::atomic::{AtomicU64, Ordering};

static ANON_COUNTER: AtomicU64 = AtomicU64::new(1);

impl OntologyID {
    /// Create a new anonymous OntologyID with a unique internal identifier.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ontology_iri: None,
            version_iri: None,
            internal_id: ANON_COUNTER.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Create an OntologyID with explicit IRIs.
    #[must_use]
    pub fn new_with_iris(ontology_iri: IRI, version_iri: Option<IRI>) -> Self {
        Self {
            ontology_iri: Some(ontology_iri),
            version_iri,
            internal_id: 0,
        }
    }

    /// Get the ontology IRI.
    #[must_use]
    pub fn get_ontology_iri(&self) -> Option<&IRI> {
        self.ontology_iri.as_ref()
    }

    /// Get the version IRI.
    #[must_use]
    pub fn get_version_iri(&self) -> Option<&IRI> {
        self.version_iri.as_ref()
    }

    /// Check if this is an anonymous ontology (no ontology IRI).
    #[must_use]
    pub fn is_anonymous(&self) -> bool {
        self.ontology_iri.is_none()
    }
}

impl PartialEq for OntologyID {
    fn eq(&self, other: &Self) -> bool {
        self.ontology_iri == other.ontology_iri
            && self.version_iri == other.version_iri
            && match (&self.ontology_iri, &other.ontology_iri) {
                (None, None) => self.internal_id == other.internal_id,
                (Some(_), Some(_)) => true,
                _ => false,
            }
    }
}

impl Eq for OntologyID {}

impl std::hash::Hash for OntologyID {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.ontology_iri.hash(state);
        self.version_iri.hash(state);
        if self.ontology_iri.is_none() {
            self.internal_id.hash(state);
        }
    }
}

impl Default for OntologyID {
    fn default() -> Self {
        Self::new()
    }
}

/// Main ontology structure containing all axioms and metadata
#[derive(Debug, Clone)]
pub struct Ontology {
    /// All axioms
    pub axioms: Vec<axioms::Axiom>,
    /// Ontology annotations
    pub annotations: Vec<Annotation>,
    /// Ontology identity
    pub id: OntologyID,
    /// Imports
    pub imports: Vec<ImportsDeclaration>,
    /// Prefix mappings (prefix name -> namespace IRI)
    pub prefixes: std::collections::HashMap<String, IRI>,
    /// Next axiom ID
    next_id: u64,
    /// Monotonically increasing version counter.
    /// Bumped on every mutation (`add_axiom`, `remove_axiom`).
    /// Cache layers compare their recorded version against this value
    /// to detect staleness instantly — no TTL wait required.
    version: u64,
    /// RDF graph for RDF-star and RDF 1.2 support
    /// Contains triples that may include quoted triples (RDF-star) or RDF 1.2 features
    pub rdf_graph: Option<crate::semantics::RdfGraph>,
}

impl Ontology {
    /// Create a new empty ontology
    #[must_use]
    pub fn new() -> Self {
        Self {
            axioms: Vec::new(),
            annotations: Vec::new(),
            id: OntologyID::new(),
            imports: Vec::new(),
            next_id: 1,
            version: 1,
            prefixes: std::collections::HashMap::new(),
            rdf_graph: None,
        }
    }

    /// Generate next axiom ID
    pub fn next_axiom_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Return the current version stamp.
    ///
    /// This counter starts at `1` and is incremented by every call to
    /// [`add_axiom`](Self::add_axiom) or [`remove_axiom`](Self::remove_axiom).
    /// Cache layers record the version at which they computed a result and
    /// treat their entry as stale whenever the ontology version has advanced.
    #[must_use]
    pub fn version(&self) -> u64 {
        self.version
    }

    /// Bump the version counter — call after **any** mutation.
    ///
    /// Using saturating arithmetic prevents overflow: after `u64::MAX` mutations
    /// the counter stays at `u64::MAX`, which will cause all caches to remain
    /// permanently stale (safe, just mildly wasteful for extreme edge cases).
    pub fn bump_version(&mut self) {
        self.version = self.version.saturating_add(1);
    }

    /// Set the ontology IRI
    pub fn set_iri(&mut self, iri: IRI) {
        self.id.ontology_iri = Some(iri);
    }

    /// Set the ontology IRI (alternative method name used by adapter)
    pub fn set_ontology_iri(&mut self, iri: Option<IRI>) {
        self.id.ontology_iri = iri;
    }

    /// Set the version IRI  
    pub fn set_version_iri(&mut self, iri: Option<IRI>) {
        self.id.version_iri = iri;
    }

    /// Get the ontology IRI
    #[must_use]
    pub fn get_iri(&self) -> Option<&IRI> {
        self.id.ontology_iri.as_ref()
    }

    /// Set all prefix mappings at once
    pub fn set_prefixes(&mut self, prefixes: std::collections::HashMap<String, IRI>) {
        self.prefixes = prefixes;
    }

    /// Get all prefix mappings
    #[must_use]
    pub fn get_prefixes(&self) -> &std::collections::HashMap<String, IRI> {
        &self.prefixes
    }

    /// Add a single prefix mapping
    pub fn add_prefix(&mut self, prefix: String, iri: IRI) {
        self.prefixes.insert(prefix, iri);
    }

    /// Get a single prefix mapping
    #[must_use]
    pub fn get_prefix(&self, prefix: &str) -> Option<&IRI> {
        self.prefixes.get(prefix)
    }

    /// Get the RDF graph (if present)
    #[must_use]
    pub fn get_rdf_graph(&self) -> Option<&crate::semantics::RdfGraph> {
        self.rdf_graph.as_ref()
    }

    /// Get mutable reference to the RDF graph (if present)
    #[must_use]
    pub fn get_rdf_graph_mut(&mut self) -> Option<&mut crate::semantics::RdfGraph> {
        self.rdf_graph.as_mut()
    }

    /// Set the RDF graph
    pub fn set_rdf_graph(&mut self, graph: crate::semantics::RdfGraph) {
        self.rdf_graph = Some(graph);
    }

    /// Get or create the RDF graph
    pub fn get_or_create_rdf_graph(&mut self) -> &mut crate::semantics::RdfGraph {
        self.rdf_graph
            .get_or_insert_with(crate::semantics::RdfGraph::new)
    }

    /// Add an RDF triple to the ontology's RDF graph
    pub fn add_rdf_triple(&mut self, triple: crate::semantics::Triple) {
        self.get_or_create_rdf_graph().add_triple(triple);
    }

    /// Check if ontology contains RDF-star features (quoted triples)
    #[must_use]
    pub fn has_rdf_star_features(&self) -> bool {
        if let Some(graph) = &self.rdf_graph {
            graph.quoted_triple_count() > 0
        } else {
            false
        }
    }

    /// Convert RDF graph to RDF 1.1 by reifying quoted triples
    pub fn reify_rdf_star(&mut self) -> crate::Result<()> {
        if let Some(graph) = &mut self.rdf_graph {
            graph.reify_quoted_triples()?;
        }
        Ok(())
    }

    /// Add an axiom to the ontology
    pub fn add_axiom(&mut self, axiom: axioms::Axiom) {
        self.axioms.push(axiom);
        self.bump_version();
    }

    /// Remove an axiom from the ontology
    pub fn remove_axiom(&mut self, axiom: &axioms::Axiom) {
        self.axioms.retain(|a| a != axiom);
        self.bump_version();
    }

    /// Get all axioms
    #[must_use]
    pub fn axioms(&self) -> &[axioms::Axiom] {
        &self.axioms
    }

    /// Build a bidirectional axiom index from the current axiom set.
    ///
    /// The caller should build the index once and reuse it across multiple
    /// queries rather than rebuilding it on every call.
    #[must_use]
    pub fn build_index(&self) -> indexes::AxiomIndex {
        indexes::AxiomIndex::build(&self.axioms)
    }

    /// Check if the ontology is empty (no axioms and no annotations)
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.axioms.is_empty() && self.annotations.is_empty()
    }

    /// Check if an entity has a declaration axiom in this ontology
    #[must_use]
    pub fn is_declared(&self, entity: &axioms::Entity) -> bool {
        self.axioms
            .iter()
            .any(|a| matches!(a, axioms::Axiom::Declaration(d) if &d.entity == entity))
    }

    /// Get all axioms of a specific type
    #[must_use]
    pub fn get_axioms_by_type(&self, axiom_type: &axioms::AxiomType) -> Vec<&axioms::Axiom> {
        self.axioms
            .iter()
            .filter(|a| &a.axiom_type() == axiom_type)
            .collect()
    }

    /// Get the count of a specific axiom type
    #[must_use]
    pub fn get_axiom_count_by_type(&self, axiom_type: &axioms::AxiomType) -> usize {
        self.axioms
            .iter()
            .filter(|a| &a.axiom_type() == axiom_type)
            .count()
    }

    /// Get all TBox (class-level) axioms
    #[must_use]
    pub fn get_tbox_axioms(&self) -> Vec<&axioms::Axiom> {
        self.axioms
            .iter()
            .filter(|a| {
                matches!(
                    a.axiom_type(),
                    axioms::AxiomType::SubClassOf
                        | axioms::AxiomType::EquivalentClasses
                        | axioms::AxiomType::DisjointClasses
                        | axioms::AxiomType::DisjointUnion
                )
            })
            .collect()
    }

    /// Get all RBox (property-level) axioms
    #[must_use]
    pub fn get_rbox_axioms(&self) -> Vec<&axioms::Axiom> {
        self.axioms
            .iter()
            .filter(|a| {
                let t = a.axiom_type();
                matches!(
                    t,
                    axioms::AxiomType::SubObjectPropertyOf
                        | axioms::AxiomType::EquivalentObjectProperties
                        | axioms::AxiomType::DisjointObjectProperties
                        | axioms::AxiomType::InverseObjectProperties
                        | axioms::AxiomType::ObjectPropertyDomain
                        | axioms::AxiomType::ObjectPropertyRange
                        | axioms::AxiomType::FunctionalObjectProperty
                        | axioms::AxiomType::InverseFunctionalObjectProperty
                        | axioms::AxiomType::ReflexiveObjectProperty
                        | axioms::AxiomType::IrreflexiveObjectProperty
                        | axioms::AxiomType::SymmetricObjectProperty
                        | axioms::AxiomType::AsymmetricObjectProperty
                        | axioms::AxiomType::TransitiveObjectProperty
                        | axioms::AxiomType::SubDataPropertyOf
                        | axioms::AxiomType::EquivalentDataProperties
                        | axioms::AxiomType::DisjointDataProperties
                        | axioms::AxiomType::DataPropertyDomain
                        | axioms::AxiomType::DataPropertyRange
                        | axioms::AxiomType::FunctionalDataProperty
                )
            })
            .collect()
    }

    /// Get all ABox (individual-level) axioms
    #[must_use]
    pub fn get_abox_axioms(&self) -> Vec<&axioms::Axiom> {
        self.axioms
            .iter()
            .filter(|a| {
                matches!(
                    a.axiom_type(),
                    axioms::AxiomType::ClassAssertion
                        | axioms::AxiomType::SameIndividual
                        | axioms::AxiomType::DifferentIndividuals
                        | axioms::AxiomType::ObjectPropertyAssertion
                        | axioms::AxiomType::DataPropertyAssertion
                        | axioms::AxiomType::NegativeObjectPropertyAssertion
                        | axioms::AxiomType::NegativeDataPropertyAssertion
                )
            })
            .collect()
    }

    /// Count logical (non-annotation) axioms
    #[must_use]
    pub fn get_logical_axiom_count(&self) -> usize {
        self.axioms.iter().filter(|a| a.is_logical()).count()
    }

    /// Get all classes from the signature (Declaration axioms)
    #[must_use]
    pub fn get_classes_in_signature(&self) -> Vec<concepts::Class> {
        let mut classes = Vec::new();
        for a in &self.axioms {
            if let axioms::Axiom::Declaration(d) = a
                && let axioms::Entity::Class(iri) = &d.entity
            {
                classes.push(concepts::Class { iri: iri.clone() });
            }
        }
        classes
    }

    /// Get all object properties from the signature
    ///
    /// Includes `owl:topObjectProperty` if the ontology uses any object properties,
    /// matching OWL API v5 behaviour.
    #[must_use]
    pub fn get_object_properties_in_signature(&self) -> Vec<ObjectProperty> {
        let mut props = Vec::new();
        for a in &self.axioms {
            if let axioms::Axiom::Declaration(d) = a
                && let axioms::Entity::ObjectProperty(iri) = &d.entity
            {
                props.push(ObjectProperty { iri: iri.clone() });
            }
        }
        // OWL API v5 always includes owl:topObjectProperty when there are object properties
        if !props.is_empty() {
            let top_iri = IRI::new("http://www.w3.org/2002/07/owl#topObjectProperty");
            if !props.iter().any(|p| p.iri == top_iri) {
                props.push(ObjectProperty { iri: top_iri });
            }
        }
        props
    }

    /// Get all data properties from the signature
    #[must_use]
    pub fn get_data_properties_in_signature(&self) -> Vec<DataProperty> {
        let mut props = Vec::new();
        for a in &self.axioms {
            if let axioms::Axiom::Declaration(d) = a
                && let axioms::Entity::DataProperty(iri) = &d.entity
            {
                props.push(DataProperty { iri: iri.clone() });
            }
        }

        // OWL API v5 includes owl:topDataProperty in the signature whenever it is
        // referenced as the super-property in a SubDataPropertyOf axiom, even if it
        // was never explicitly declared.  Mirror that behaviour.
        const TOP_DATA_PROP: &str = "http://www.w3.org/2002/07/owl#topDataProperty";
        let references_top = self.axioms.iter().any(|a| {
            if let axioms::Axiom::SubDataPropertyOf(ax) = a {
                let DataPropertyExpression::DataProperty(p) = &ax.super_property;
                p.iri.as_str() == TOP_DATA_PROP
            } else {
                false
            }
        });
        if references_top {
            let top_iri = IRI::new(TOP_DATA_PROP);
            if !props.iter().any(|p| p.iri == top_iri) {
                props.push(DataProperty { iri: top_iri });
            }
        }

        props
    }

    /// Get all named individuals from the signature
    #[must_use]
    pub fn get_individuals_in_signature(&self) -> Vec<individuals::NamedIndividual> {
        let mut inds = Vec::new();
        for a in &self.axioms {
            if let axioms::Axiom::Declaration(d) = a
                && let axioms::Entity::NamedIndividual(iri) = &d.entity
            {
                inds.push(individuals::NamedIndividual { iri: iri.clone() });
            }
        }
        inds
    }

    /// Get all datatypes from the signature.
    ///
    /// Includes explicitly declared datatypes AND OWL 2 built-in XSD datatypes
    /// that are actually *used* in the ontology (in `DataPropertyRange` axioms or
    /// typed literals in `DataPropertyAssertion` axioms), matching OWL API v5.
    #[must_use]
    pub fn get_datatypes_in_signature(&self) -> Vec<Datatype> {
        let mut dts = Vec::new();

        // Step 1: collect explicitly declared datatypes
        for a in &self.axioms {
            if let axioms::Axiom::Declaration(d) = a
                && let axioms::Entity::Datatype(iri) = &d.entity
            {
                dts.push(Datatype { iri: iri.clone() });
            }
        }

        // Known OWL 2 / XSD built-in datatypes that should appear in the signature
        // when actually used.
        const BUILTIN_DATATYPES: &[&str] = &[
            "http://www.w3.org/2001/XMLSchema#string",
            "http://www.w3.org/2001/XMLSchema#integer",
            "http://www.w3.org/2001/XMLSchema#decimal",
            "http://www.w3.org/2001/XMLSchema#double",
            "http://www.w3.org/2001/XMLSchema#float",
            "http://www.w3.org/2001/XMLSchema#boolean",
            "http://www.w3.org/2001/XMLSchema#dateTime",
            "http://www.w3.org/2001/XMLSchema#int",
            "http://www.w3.org/2001/XMLSchema#long",
            "http://www.w3.org/2001/XMLSchema#short",
            "http://www.w3.org/2001/XMLSchema#date",
            "http://www.w3.org/2001/XMLSchema#anyURI",
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#PlainLiteral",
        ];
        let builtin_set: std::collections::HashSet<&str> =
            BUILTIN_DATATYPES.iter().copied().collect();

        // Step 2: collect IRI strings of all used datatypes from axioms
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();

        /// Recursively gather datatype IRIs from a DataRange.
        fn collect_from_range(range: &DataRange, out: &mut std::collections::HashSet<String>) {
            match range {
                DataRange::Datatype(iri) => {
                    out.insert(iri.as_str().to_owned());
                }
                DataRange::DataIntersectionOf(rs) => {
                    for r in rs {
                        collect_from_range(r, out);
                    }
                }
                DataRange::DataUnionOf(rs) => {
                    for r in rs {
                        collect_from_range(r, out);
                    }
                }
                DataRange::DataComplementOf(inner) => collect_from_range(inner, out),
                DataRange::DatatypeRestriction { datatype, .. } => {
                    out.insert(datatype.as_str().to_owned());
                }
                DataRange::DataOneOf(_) => {}
            }
        }

        for a in &self.axioms {
            match a {
                // DataPropertyRange axioms reference datatypes directly
                axioms::Axiom::DataPropertyRange(ax) => {
                    collect_from_range(&ax.range, &mut used);
                }
                // Typed literals in DataPropertyAssertion
                axioms::Axiom::DataPropertyAssertion(ax) => {
                    if let Some(url) = &ax.value.datatype {
                        used.insert(url.to_string());
                    }
                }
                // Typed literals in NegativeDataPropertyAssertion
                axioms::Axiom::NegativeDataPropertyAssertion(ax) => {
                    if let Some(url) = &ax.value.datatype {
                        used.insert(url.to_string());
                    }
                }
                // Typed literals in AnnotationAssertion — OWL API v5 includes any
                // explicitly-typed literal's datatype in the ontology signature.
                // Also include built-in datatype IRIs used as IRI-valued annotations
                // (e.g. dcam:rangeIncludes xsd:date).
                axioms::Axiom::AnnotationAssertion(ax) => match &ax.value {
                    AnnotationValue::Literal(lit) => {
                        if let Some(url) = &lit.datatype {
                            used.insert(url.to_string());
                        }
                    }
                    AnnotationValue::IRI(iri) => {
                        used.insert(iri.as_str().to_owned());
                    }
                    AnnotationValue::AnonymousIndividual(_) => {}
                },
                _ => {}
            }
        }

        // Step 3: add built-in datatypes that are actually used and not yet listed
        for iri_str in &used {
            if builtin_set.contains(iri_str.as_str()) {
                let iri = IRI::new(iri_str);
                if !dts.iter().any(|d| d.iri == iri) {
                    dts.push(Datatype { iri });
                }
            }
        }

        dts
    }

    /// Get all annotation properties from the signature.
    ///
    /// Includes explicitly declared annotation properties AND built-in RDFS/OWL
    /// annotation properties that are actually *used* in `AnnotationAssertion`
    /// axioms, matching OWL API v5 behaviour.
    #[must_use]
    pub fn get_annotation_properties_in_signature(&self) -> Vec<AnnotationProperty> {
        let mut props = Vec::new();
        for a in &self.axioms {
            if let axioms::Axiom::Declaration(d) = a
                && let axioms::Entity::AnnotationProperty(iri) = &d.entity
            {
                props.push(AnnotationProperty { iri: iri.clone() });
            }
        }

        // Built-in annotation properties that should appear in the signature when used
        const BUILTIN_ANNOTATION_PROPS: &[&str] = &[
            "http://www.w3.org/2000/01/rdf-schema#label",
            "http://www.w3.org/2000/01/rdf-schema#comment",
            "http://www.w3.org/2000/01/rdf-schema#isDefinedBy",
            "http://www.w3.org/2000/01/rdf-schema#seeAlso",
            "http://www.w3.org/2002/07/owl#deprecated",
            "http://www.w3.org/2002/07/owl#versionInfo",
            "http://www.w3.org/2002/07/owl#priorVersion",
        ];
        let builtin_set: std::collections::HashSet<&str> =
            BUILTIN_ANNOTATION_PROPS.iter().copied().collect();

        // Scan AnnotationAssertion axioms and add built-in properties that are used
        for a in &self.axioms {
            if let axioms::Axiom::AnnotationAssertion(ax) = a {
                let iri_str = ax.property.iri.as_str();
                if builtin_set.contains(iri_str) {
                    let iri = ax.property.iri.clone();
                    if !props.iter().any(|p| p.iri == iri) {
                        props.push(AnnotationProperty { iri });
                    }
                }
            }
        }

        props
    }

    /// Count axioms by type
    #[must_use]
    pub fn count_axioms_by_type(&self) -> std::collections::HashMap<axioms::AxiomType, usize> {
        let mut counts = std::collections::HashMap::new();
        for axiom in &self.axioms {
            *counts.entry(axiom.axiom_type()).or_insert(0) += 1;
        }
        counts
    }

    /// Get the signature of the ontology
    pub fn signature(&self) -> Result<Signature> {
        let mut signature = Signature::new();

        // Helper function to extract classes from class expressions
        fn extract_classes_from_expression(
            expr: &concepts::ClassExpression,
            classes: &mut Vec<concepts::Class>,
        ) {
            match expr {
                concepts::ClassExpression::Class(class) => {
                    if !classes.iter().any(|c| c.iri == class.iri) {
                        classes.push(class.clone());
                    }
                }
                concepts::ClassExpression::ObjectIntersectionOf(exprs)
                | concepts::ClassExpression::ObjectUnionOf(exprs) => {
                    for expr in exprs {
                        extract_classes_from_expression(expr, classes);
                    }
                }
                concepts::ClassExpression::ObjectComplementOf(expr) => {
                    extract_classes_from_expression(expr, classes);
                }
                concepts::ClassExpression::ObjectSomeValuesFrom { filler, .. }
                | concepts::ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                    extract_classes_from_expression(filler, classes);
                }
                concepts::ClassExpression::ObjectMinCardinality { filler, .. }
                | concepts::ClassExpression::ObjectMaxCardinality { filler, .. }
                | concepts::ClassExpression::ObjectExactCardinality { filler, .. } => {
                    extract_classes_from_expression(filler, classes);
                }
                concepts::ClassExpression::ObjectHasValue { .. }
                | concepts::ClassExpression::ObjectHasSelf { .. }
                | concepts::ClassExpression::ObjectOneOf(..)
                | concepts::ClassExpression::DataSomeValuesFrom { .. }
                | concepts::ClassExpression::DataAllValuesFrom { .. }
                | concepts::ClassExpression::DataMinCardinality { .. }
                | concepts::ClassExpression::DataMaxCardinality { .. }
                | concepts::ClassExpression::DataExactCardinality { .. }
                | concepts::ClassExpression::DataHasValue { .. } => {
                    // These don't contain nested class expressions
                }
            }
        }

        log::debug!("Computing signature from {} axioms", self.axioms.len());

        // Extract entities from axioms
        for axiom in &self.axioms {
            let discriminant = std::mem::discriminant(axiom);
            log::debug!("Processing axiom discriminant: {discriminant:?}");
            match axiom {
                axioms::Axiom::Declaration(decl) => {
                    match &decl.entity {
                        axioms::Entity::Class(iri) => {
                            signature.classes.push(concepts::Class { iri: iri.clone() });
                            log::debug!("Added class from declaration: {iri}");
                        }
                        axioms::Entity::ObjectProperty(iri) => {
                            signature
                                .object_properties
                                .push(ObjectProperty { iri: iri.clone() });
                        }
                        axioms::Entity::DataProperty(iri) => {
                            signature
                                .data_properties
                                .push(DataProperty { iri: iri.clone() });
                        }
                        axioms::Entity::NamedIndividual(iri) => {
                            signature.individuals.push(individuals::Individual::Named(
                                individuals::NamedIndividual { iri: iri.clone() },
                            ));
                        }
                        axioms::Entity::AnnotationProperty(_prop) => {
                            // Handle annotation property
                        }
                        axioms::Entity::Datatype(_datatype) => {
                            // Handle datatype
                        }
                    }
                }
                axioms::Axiom::SubClassOf(axiom) => {
                    log::debug!("Processing SubClassOf axiom");
                    extract_classes_from_expression(&axiom.subclass, &mut signature.classes);
                    extract_classes_from_expression(&axiom.superclass, &mut signature.classes);
                }
                axioms::Axiom::EquivalentClasses(axiom) => {
                    log::debug!(
                        "Processing EquivalentClasses axiom with {} classes",
                        axiom.classes.len()
                    );
                    for class_expr in &axiom.classes {
                        extract_classes_from_expression(class_expr, &mut signature.classes);
                    }
                }
                axioms::Axiom::ClassAssertion(axiom) => {
                    log::debug!("Processing ClassAssertion axiom");
                    extract_classes_from_expression(&axiom.class, &mut signature.classes);
                    // Also add the individual
                    if !signature.individuals.iter().any(|i| match i {
                        individuals::Individual::Named(named) => {
                            named.iri
                                == match &axiom.individual {
                                    individuals::Individual::Named(named) => named.iri.clone(),
                                    _ => return false,
                                }
                        }
                        _ => false,
                    }) {
                        signature.individuals.push(axiom.individual.clone());
                    }
                }
                axioms::Axiom::DisjointUnion(axiom) => {
                    log::debug!("Processing DisjointUnion axiom");
                    extract_classes_from_expression(&axiom.class, &mut signature.classes);
                    for disjoint_class in &axiom.disjoint_classes {
                        extract_classes_from_expression(disjoint_class, &mut signature.classes);
                    }
                }
                axioms::Axiom::DisjointClasses(axiom) => {
                    log::debug!("Processing DisjointClasses axiom");
                    for class_expr in &axiom.classes {
                        extract_classes_from_expression(class_expr, &mut signature.classes);
                    }
                }
                axioms::Axiom::ObjectPropertyAssertion(axiom) => {
                    log::debug!("Processing ObjectPropertyAssertion axiom");
                    // Add individuals but these don't typically contain classes
                    if !signature.individuals.contains(&axiom.source) {
                        signature.individuals.push(axiom.source.clone());
                    }
                    if !signature.individuals.contains(&axiom.target) {
                        signature.individuals.push(axiom.target.clone());
                    }
                }
                axioms::Axiom::DataPropertyAssertion(axiom) => {
                    log::debug!("Processing DataPropertyAssertion axiom");
                    if !signature.individuals.contains(&axiom.individual) {
                        signature.individuals.push(axiom.individual.clone());
                    }
                }
                // Handle other axiom types as needed
                axiom => {
                    log::debug!(
                        "Processing other axiom type: {:?}",
                        std::mem::discriminant(axiom)
                    );
                }
            }
        }

        log::debug!(
            "Final signature: {} classes, {} individuals",
            signature.classes.len(),
            signature.individuals.len()
        );
        for class in &signature.classes {
            log::debug!("  Class: {}", class.iri);
        }

        Ok(signature)
    }

    /// Load an ontology from a file using horned-owl for robust parsing
    pub fn from_file_with_horned_owl<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        use horned_owl::io::ParserConfiguration;
        use std::fs::File;
        use std::io::{BufReader, Read};

        let file = File::open(path.as_ref()).map_err(|e| Error::io(e.to_string()))?;
        let mut reader = BufReader::new(file);
        let config = ParserConfiguration::default();

        // Use horned-owl's RDF parser for all file types (most compatible)
        let result = horned_owl::io::rdf::reader::read(&mut reader, config)
            .map_err(|e| Error::ontology_parsing(format!("Horned-owl parsing error: {e}")))?;

        // Convert the horned-owl ontology to oxidowl ontology using enhanced adapter
        let mut adapter = crate::adapter::HornedOwlAdapter::new();
        let mut ontology =
            adapter.convert_basic_ontology::<std::rc::Rc<str>, std::rc::Rc<str>, _>(&result.0)?;

        // Try to extract ontology IRI from the file by re-reading it
        // This is a workaround since horned-owl's API is complex
        let file = File::open(path.as_ref()).map_err(|e| Error::io(e.to_string()))?;
        let mut file_reader = BufReader::new(file);
        let mut file_contents = String::new();
        file_reader
            .read_to_string(&mut file_contents)
            .map_err(|e| Error::io(e.to_string()))?;

        // Look for patterns like: <http://...> rdf:type owl:Ontology
        if let Some(iri) = Self::extract_ontology_iri_from_content(&file_contents) {
            ontology.set_ontology_iri(Some(iri));
        }

        Ok(ontology)
    }

    /// Extract ontology IRI from file content (Turtle/RDF format)
    fn extract_ontology_iri_from_content(content: &str) -> Option<IRI> {
        // Match pattern: <http://...> rdf:type owl:Ontology
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.contains("rdf:type") && trimmed.contains("owl:Ontology") {
                // Extract IRI between < and >
                if let Some(start) = trimmed.find('<')
                    && let Some(end) = trimmed[start..].find('>')
                {
                    let iri_str = &trimmed[start + 1..start + end];
                    if iri_str.starts_with("http") {
                        return Some(IRI::new(iri_str));
                    }
                }
            }
        }
        None
    }

    /// Convert a horned-owl ontology to oxidowl ontology with full SWRL support
    pub fn from_horned_owl_with_swrl<A>(
        horned_ontology: horned_owl::ontology::set::SetOntology<A>,
        _prefix_mapping: curie::PrefixMapping,
    ) -> Result<Self>
    where
        A: horned_owl::model::ForIRI + Clone + std::fmt::Display + std::hash::Hash + Eq,
    {
        let mut adapter = crate::adapter::HornedOwlAdapter::new();
        adapter.convert_ontology_with_swrl::<A, A, _>(&horned_ontology)
    }

    /// Load an ontology from a file
    pub fn from_file<P: AsRef<std::path::Path>>(path: P, format: Option<String>) -> Result<Self> {
        use std::fs::File;
        use std::io::Read;

        let mut file = File::open(path.as_ref()).map_err(|e| Error::io(e.to_string()))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|e| Error::io(e.to_string()))?;

        // Parse based on format or file extension
        let format = format.unwrap_or_else(|| {
            path.as_ref()
                .extension()
                .and_then(|ext| ext.to_str())
                .map_or_else(|| "owl".to_string(), str::to_lowercase)
        });

        match format.as_str() {
            "owl" | "xml" => {
                // Use OWL XML parser
                crate::parsers::owl_xml::parse(&contents)
            }
            "ttl" | "turtle" => {
                // Use Turtle parser
                crate::parsers::turtle::parse(&contents)
            }
            "rdf" | "rdfxml" => {
                // Use RDF/XML parser
                crate::parsers::rdf_xml::parse(&contents)
            }
            "nt" | "ntriples" => {
                // Use N-Triples parser
                crate::parsers::ntriples::parse(&contents)
            }
            "functional" | "func" | "ofn" => {
                // Use Functional syntax parser
                crate::parsers::functional::parse(&contents)
            }
            _ => {
                // Default to OWL XML
                crate::parsers::owl_xml::parse(&contents)
            }
        }
    }

    /// Add a class (placeholder for compatibility)
    pub fn add_class(&mut self, class: concepts::Class) {
        // This creates a declaration axiom for the class
        let axiom = axioms::Axiom::Declaration(axioms::DeclarationAxiom {
            id: self.next_axiom_id(),
            entity: axioms::Entity::Class(class.iri),
        });
        self.add_axiom(axiom);
    }

    /// Add an object property (placeholder for compatibility)
    pub fn add_object_property(&mut self, property: ObjectProperty) {
        // This creates a declaration axiom for the property
        let axiom = axioms::Axiom::Declaration(axioms::DeclarationAxiom {
            id: self.next_axiom_id(),
            entity: axioms::Entity::ObjectProperty(property.iri),
        });
        self.add_axiom(axiom);
    }

    /// Add an individual and its declaration axiom
    pub fn add_individual(&mut self, _subject: IRI, individual: individuals::Individual) {
        // Add a declaration axiom for the individual
        let declaration = axioms::DeclarationAxiom {
            id: self.next_axiom_id(),
            entity: match individual {
                individuals::Individual::Named(ref named) => {
                    axioms::Entity::NamedIndividual(named.iri.clone())
                }
                individuals::Individual::Anonymous(_) => {
                    // Anonymous individuals are not typically declared
                    return;
                }
            },
        };

        self.add_axiom(axioms::Axiom::Declaration(declaration));

        // Also store in internal tracking if needed
        // For now, the axiom storage is sufficient
    }

    /// Get classes by extracting them from declaration axioms
    #[must_use]
    pub fn classes(&self) -> Vec<(IRI, concepts::Class)> {
        let mut classes = Vec::with_capacity(self.axioms.len());

        for axiom in &self.axioms {
            if let axioms::Axiom::Declaration(decl) = axiom
                && let axioms::Entity::Class(iri) = &decl.entity
            {
                let class = concepts::Class { iri: iri.clone() };
                classes.push((iri.clone(), class));
            }
        }

        classes
    }

    /// Extract individuals from the axioms
    #[must_use]
    pub fn individuals(&self) -> Vec<(IRI, individuals::Individual)> {
        let mut individuals = Vec::with_capacity(self.axioms.len());

        for axiom in &self.axioms {
            match axiom {
                // Extract from declaration axioms
                axioms::Axiom::Declaration(decl) => {
                    if let axioms::Entity::NamedIndividual(iri) = &decl.entity {
                        let individual = individuals::Individual::named(iri.clone());
                        individuals.push((iri.clone(), individual));
                    }
                }
                // Extract from class assertion axioms
                axioms::Axiom::ClassAssertion(assertion) => {
                    let iri = match &assertion.individual {
                        individuals::Individual::Named(named) => &named.iri,
                        individuals::Individual::Anonymous(_) => continue, // Skip anonymous
                    };

                    // Only add if not already present
                    if !individuals
                        .iter()
                        .any(|(existing_iri, _)| existing_iri == iri)
                    {
                        individuals.push((iri.clone(), assertion.individual.clone()));
                    }
                }
                // Extract from object property assertion axioms
                axioms::Axiom::ObjectPropertyAssertion(assertion) => {
                    // Extract source
                    if let individuals::Individual::Named(named) = &assertion.source
                        && !individuals
                            .iter()
                            .any(|(existing_iri, _)| existing_iri == &named.iri)
                    {
                        individuals.push((named.iri.clone(), assertion.source.clone()));
                    }

                    // Extract target
                    if let individuals::Individual::Named(named) = &assertion.target
                        && !individuals
                            .iter()
                            .any(|(existing_iri, _)| existing_iri == &named.iri)
                    {
                        individuals.push((named.iri.clone(), assertion.target.clone()));
                    }
                }
                axioms::Axiom::DataPropertyAssertion(data_assertion) => {
                    // Extract individual from data property assertion
                    if let Individual::Named(named) = &data_assertion.individual {
                        individuals.push((named.iri.clone(), data_assertion.individual.clone()));
                    }
                }
                axioms::Axiom::NegativeObjectPropertyAssertion(neg_obj_assertion) => {
                    // Extract individuals from negative object property assertion
                    if let Individual::Named(named) = &neg_obj_assertion.source {
                        individuals.push((named.iri.clone(), neg_obj_assertion.source.clone()));
                    }
                    if let Individual::Named(named) = &neg_obj_assertion.target {
                        individuals.push((named.iri.clone(), neg_obj_assertion.target.clone()));
                    }
                }
                axioms::Axiom::NegativeDataPropertyAssertion(neg_data_assertion) => {
                    // Extract individual from negative data property assertion
                    if let Individual::Named(named) = &neg_data_assertion.individual {
                        individuals
                            .push((named.iri.clone(), neg_data_assertion.individual.clone()));
                    }
                }
                axioms::Axiom::SameIndividual(same_individuals) => {
                    // Extract individuals from same individual axiom
                    for individual in &same_individuals.individuals {
                        if let Individual::Named(named) = individual {
                            individuals.push((named.iri.clone(), individual.clone()));
                        }
                    }
                }
                axioms::Axiom::DifferentIndividuals(diff_individuals) => {
                    // Extract individuals from different individuals axiom
                    for individual in &diff_individuals.individuals {
                        if let Individual::Named(named) = individual {
                            individuals.push((named.iri.clone(), individual.clone()));
                        }
                    }
                }
                _ => {
                    // Other axiom types don't typically contain individuals
                }
            }
        }

        individuals
    }

    /// Get object properties by extracting them from declaration axioms
    #[must_use]
    pub fn object_properties(&self) -> Vec<ObjectProperty> {
        let mut properties = Vec::with_capacity(self.axioms.len());

        for axiom in &self.axioms {
            if let axioms::Axiom::Declaration(decl) = axiom
                && let axioms::Entity::ObjectProperty(iri) = &decl.entity
            {
                let property = ObjectProperty { iri: iri.clone() };
                properties.push(property);
            }
        }

        properties
    }

    /// Query for property chain axioms and return super property if chain matches
    ///
    /// This method searches for `SubObjectPropertyOf` axioms with property chains
    /// that match the given first and second roles. If found, it returns the
    /// super property that can be inferred from the chain.
    ///
    /// Example: If we have R ∘ S ⊑ T and we query with (R, S), returns Some(T)
    #[must_use]
    pub fn get_property_chain_super(&self, first_role: &str, second_role: &str) -> Option<String> {
        for axiom in &self.axioms {
            if let axioms::Axiom::SubObjectPropertyOf(sub_prop_axiom) = axiom {
                // Check if the sub_property is a property chain
                if let ObjectPropertyExpression::PropertyChain(chain) = &sub_prop_axiom.sub_property
                {
                    // Check if this chain matches our (first_role, second_role) pattern
                    if chain.len() == 2 {
                        let first_matches = match &chain[0] {
                            ObjectPropertyExpression::ObjectProperty(prop) => {
                                prop.iri.as_str().ends_with(first_role)
                                    || prop.iri.as_str() == first_role
                            }
                            _ => false,
                        };

                        let second_matches = match &chain[1] {
                            ObjectPropertyExpression::ObjectProperty(prop) => {
                                prop.iri.as_str().ends_with(second_role)
                                    || prop.iri.as_str() == second_role
                            }
                            _ => false,
                        };

                        if first_matches && second_matches {
                            // Extract the super property name
                            if let ObjectPropertyExpression::ObjectProperty(super_prop) =
                                &sub_prop_axiom.super_property
                            {
                                return Some(super_prop.iri.to_string());
                            }
                        }
                    }
                }
            }
        }

        None
    }

    /// Get concept definition from `EquivalentClasses` axioms
    ///
    /// This method searches for `EquivalentClasses` axioms containing the given
    /// named class and returns an equivalent definition if found.
    ///
    /// Example: If Person ≡ Human ⊓ ∃hasParent.Person, returns Some(Human ⊓ ∃hasParent.Person)
    #[must_use]
    pub fn get_concept_definition(&self, named_class: &concepts::Class) -> Option<ClassExpression> {
        for axiom in &self.axioms {
            if let axioms::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                // Check if this equivalence contains our target class
                let contains_target = equiv_axiom
                    .classes
                    .iter()
                    .any(|ce| matches!(ce, ClassExpression::Class(c) if c.iri == named_class.iri));

                if contains_target {
                    // Return the first non-trivial equivalent definition
                    for ce in &equiv_axiom.classes {
                        // Skip the trivial self-reference
                        if matches!(ce, ClassExpression::Class(c) if c.iri == named_class.iri) {
                            continue;
                        }
                        // Return the first complex definition found
                        return Some(ce.clone());
                    }
                }
            }
        }

        None
    }

    /// Get all equivalent classes for a given class
    ///
    /// Returns all class expressions that are declared equivalent to the given class.
    #[must_use]
    pub fn get_equivalent_classes(&self, named_class: &concepts::Class) -> Vec<ClassExpression> {
        let mut equivalents = Vec::new();

        for axiom in &self.axioms {
            if let axioms::Axiom::EquivalentClasses(equiv_axiom) = axiom {
                // Check if this equivalence contains our target class
                let contains_target = equiv_axiom
                    .classes
                    .iter()
                    .any(|ce| matches!(ce, ClassExpression::Class(c) if c.iri == named_class.iri));

                if contains_target {
                    // Collect all non-self equivalent expressions
                    for ce in &equiv_axiom.classes {
                        if !matches!(ce, ClassExpression::Class(c) if c.iri == named_class.iri) {
                            equivalents.push(ce.clone());
                        }
                    }
                }
            }
        }

        equivalents
    }
}

impl Default for Ontology {
    fn default() -> Self {
        Self::new()
    }
}

/// Supported ontology formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OntologyFormat {
    /// Auto-detect format
    Auto,
    /// OWL Functional Syntax
    Functional,
    /// OWL/XML
    OwlXml,
    /// RDF/XML
    RdfXml,
    /// Turtle
    Turtle,
    /// N-Triples
    NTriples,
    /// Manchester Syntax
    Manchester,
    /// LaTeX (write-only)
    Latex,
    /// DL Syntax
    DL,
    /// KRSS
    Krss,
    /// KRSS2
    Krss2,
    /// OBO Format
    Obo,
    /// N-Quads
    NQuads,
    /// Notation3
    N3,
    /// TriG
    TriG,
    /// TriX
    TriX,
    /// JSON-LD
    JsonLd,
    /// RDF/JSON
    RdfJson,
    /// RDFa
    Rdfa,
    /// Binary RDF
    BinaryRdf,
    /// HDT
    Hdt,
}

impl OntologyFormat {
    /// Get the file extension for this format
    #[must_use]
    pub fn extension(&self) -> &'static str {
        match self {
            OntologyFormat::Auto => "",
            OntologyFormat::Functional => "owx",
            OntologyFormat::OwlXml => "owl",
            OntologyFormat::RdfXml => "rdf",
            OntologyFormat::Turtle => "ttl",
            OntologyFormat::NTriples => "nt",
            OntologyFormat::Manchester => "omn",
            OntologyFormat::Latex => "tex",
            OntologyFormat::DL => "dl",
            OntologyFormat::Krss => "krss",
            OntologyFormat::Krss2 => "krss2",
            OntologyFormat::Obo => "obo",
            OntologyFormat::NQuads => "nq",
            OntologyFormat::N3 => "n3",
            OntologyFormat::TriG => "trig",
            OntologyFormat::TriX => "xml",
            OntologyFormat::JsonLd => "jsonld",
            OntologyFormat::RdfJson => "rj",
            OntologyFormat::Rdfa => "html",
            OntologyFormat::BinaryRdf => "brdf",
            OntologyFormat::Hdt => "hdt",
        }
    }

    /// Get the media type for this format
    #[must_use]
    pub fn media_type(&self) -> &'static str {
        match self {
            OntologyFormat::Auto => "",
            OntologyFormat::Functional => "text/owl-functional",
            OntologyFormat::OwlXml => "application/owl+xml",
            OntologyFormat::RdfXml => "application/rdf+xml",
            OntologyFormat::Turtle => "text/turtle",
            OntologyFormat::NTriples => "application/n-triples",
            OntologyFormat::Manchester => "text/owl-manchester",
            OntologyFormat::Latex => "application/x-latex",
            OntologyFormat::DL => "text/owl-dl",
            OntologyFormat::Krss => "text/krss",
            OntologyFormat::Krss2 => "text/krss2",
            OntologyFormat::Obo => "text/obo",
            OntologyFormat::NQuads => "application/n-quads",
            OntologyFormat::N3 => "text/n3",
            OntologyFormat::TriG => "application/trig",
            OntologyFormat::TriX => "application/trix",
            OntologyFormat::JsonLd => "application/ld+json",
            OntologyFormat::RdfJson => "application/rdf+json",
            OntologyFormat::Rdfa => "text/html",
            OntologyFormat::BinaryRdf => "application/x-binary-rdf",
            OntologyFormat::Hdt => "application/x-hdt",
        }
    }

    /// Get the format string for parsing
    #[must_use]
    pub fn format_string(&self) -> &'static str {
        match self {
            OntologyFormat::Auto => "auto",
            OntologyFormat::Functional => "functional",
            OntologyFormat::OwlXml => "owl",
            OntologyFormat::RdfXml => "rdf",
            OntologyFormat::Turtle => "ttl",
            OntologyFormat::NTriples => "nt",
            OntologyFormat::Manchester => "omn",
            OntologyFormat::Latex => "latex",
            OntologyFormat::DL => "dl",
            OntologyFormat::Krss => "krss",
            OntologyFormat::Krss2 => "krss2",
            OntologyFormat::Obo => "obo",
            OntologyFormat::NQuads => "nquads",
            OntologyFormat::N3 => "n3",
            OntologyFormat::TriG => "trig",
            OntologyFormat::TriX => "trix",
            OntologyFormat::JsonLd => "jsonld",
            OntologyFormat::RdfJson => "rdfjson",
            OntologyFormat::Rdfa => "rdfa",
            OntologyFormat::BinaryRdf => "binary",
            OntologyFormat::Hdt => "hdt",
        }
    }

    /// Try to detect format from file extension
    #[must_use]
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "owx" => Some(OntologyFormat::OwlXml),
            "owl" | "ofn" => Some(OntologyFormat::Functional),
            "rdf" => Some(OntologyFormat::RdfXml),
            "ttl" => Some(OntologyFormat::Turtle),
            "nt" => Some(OntologyFormat::NTriples),
            "omn" => Some(OntologyFormat::Manchester),
            "tex" | "latex" => Some(OntologyFormat::Latex),
            "dl" => Some(OntologyFormat::DL),
            "krss" => Some(OntologyFormat::Krss),
            "krss2" => Some(OntologyFormat::Krss2),
            "obo" => Some(OntologyFormat::Obo),
            "nq" | "nquads" => Some(OntologyFormat::NQuads),
            "n3" => Some(OntologyFormat::N3),
            "trig" => Some(OntologyFormat::TriG),
            "trix" => Some(OntologyFormat::TriX),
            "jsonld" | "json-ld" => Some(OntologyFormat::JsonLd),
            "rj" | "rjson" => Some(OntologyFormat::RdfJson),
            "html" | "xhtml" => Some(OntologyFormat::Rdfa),
            "brdf" => Some(OntologyFormat::BinaryRdf),
            "hdt" => Some(OntologyFormat::Hdt),
            _ => None,
        }
    }
}

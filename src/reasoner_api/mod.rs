//! Standard OWL 2 Reasoning Interface
//!
//! Provides the [`OWLReasoner`] trait matching the OWL API's reasoner
//! interface, along with [`Node`]/[`NodeSet`] types for representing
//! equivalence classes in hierarchies.
//!
//! Two implementations are provided:
//! - [`TableauOWLReasoner`] wraps the existing `Reasoner` struct
//! - [`StructuralReasoner`] provides fast non-logical reasoning

pub mod structural;

#[cfg(test)]
mod tests;

use crate::Result;
use crate::ontology::axioms::Axiom;
use crate::ontology::{
    ClassExpression, DataPropertyExpression, DataRange, Individual, ObjectPropertyExpression,
    OntologyRef,
};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Node / NodeSet ───────────────────────────────────────────────────────────

/// A set of equivalent entities (e.g., equivalent classes).
/// Represents a single node in the classification hierarchy.
#[derive(Debug, Clone)]
pub struct Node<T: Clone + Eq + Hash> {
    entities: HashSet<T>,
    is_top_node: bool,
    is_bottom_node: bool,
}

impl<T: Clone + Eq + Hash> Node<T> {
    /// Create a node with a single entity.
    #[must_use]
    pub fn singleton(entity: T) -> Self {
        let mut entities = HashSet::new();
        entities.insert(entity);
        Self {
            entities,
            is_top_node: false,
            is_bottom_node: false,
        }
    }

    /// Create a node from a set of equivalent entities.
    #[must_use]
    pub fn new(entities: HashSet<T>) -> Self {
        Self {
            entities,
            is_top_node: false,
            is_bottom_node: false,
        }
    }

    /// Create the TOP node (owl:Thing).
    #[must_use]
    pub fn top_node(entity: T) -> Self {
        let mut entities = HashSet::new();
        entities.insert(entity);
        Self {
            entities,
            is_top_node: true,
            is_bottom_node: false,
        }
    }

    /// Create the BOTTOM node (owl:Nothing).
    #[must_use]
    pub fn bottom_node(entity: T) -> Self {
        let mut entities = HashSet::new();
        entities.insert(entity);
        Self {
            entities,
            is_top_node: false,
            is_bottom_node: true,
        }
    }

    /// Get all entities in this equivalence class.
    #[must_use]
    pub fn get_entities(&self) -> &HashSet<T> {
        &self.entities
    }

    /// Number of entities in this node.
    #[must_use]
    pub fn get_size(&self) -> usize {
        self.entities.len()
    }

    /// Whether this node contains exactly one entity.
    #[must_use]
    pub fn is_singleton(&self) -> bool {
        self.entities.len() == 1
    }

    /// Get a representative element (first in iteration order).
    #[must_use]
    pub fn get_representative_element(&self) -> T
    where
        T: Clone,
    {
        self.entities
            .iter()
            .next()
            .cloned()
            .expect("Node must have at least one entity")
    }

    /// Whether this is the TOP node (owl:Thing equivalence class).
    #[must_use]
    pub fn is_top_node(&self) -> bool {
        self.is_top_node
    }

    /// Whether this is the BOTTOM node (owl:Nothing equivalence class).
    #[must_use]
    pub fn is_bottom_node(&self) -> bool {
        self.is_bottom_node
    }

    /// Check if the node contains a specific entity.
    #[must_use]
    pub fn contains(&self, entity: &T) -> bool {
        self.entities.contains(entity)
    }
}

impl<T: Clone + Eq + Hash> PartialEq for Node<T> {
    fn eq(&self, other: &Self) -> bool {
        self.entities == other.entities
    }
}

impl<T: Clone + Eq + Hash> Eq for Node<T> {}

impl<T: Clone + Eq + Hash> Hash for Node<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        for entity in &self.entities {
            entity.hash(state);
        }
    }
}

/// A set of nodes — represents results of hierarchical queries.
#[derive(Debug, Clone)]
pub struct NodeSet<T: Clone + Eq + Hash> {
    nodes: HashSet<Node<T>>,
}

impl<T: Clone + Eq + Hash> NodeSet<T> {
    /// Create an empty node set.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            nodes: HashSet::new(),
        }
    }

    /// Create a node set from an existing set of nodes.
    #[must_use]
    pub fn new(nodes: HashSet<Node<T>>) -> Self {
        Self { nodes }
    }

    /// Get all nodes.
    #[must_use]
    pub fn get_nodes(&self) -> &HashSet<Node<T>> {
        &self.nodes
    }

    /// Whether the set is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Whether the set contains only the TOP singleton.
    #[must_use]
    pub fn is_top_singleton(&self) -> bool {
        self.nodes.len() == 1
            && self
                .nodes
                .iter()
                .next()
                .is_some_and(|n| n.is_top_node() && n.is_singleton())
    }

    /// Whether the set contains only the BOTTOM singleton.
    #[must_use]
    pub fn is_bottom_singleton(&self) -> bool {
        self.nodes.len() == 1
            && self
                .nodes
                .iter()
                .next()
                .is_some_and(|n| n.is_bottom_node() && n.is_singleton())
    }

    /// Get all entities flattened across all nodes.
    #[must_use]
    pub fn get_flattened(&self) -> HashSet<T> {
        let mut result = HashSet::new();
        for node in &self.nodes {
            result.extend(node.entities.iter().cloned());
        }
        result
    }

    /// Check if any node contains the given entity.
    #[must_use]
    pub fn contains_entity(&self, entity: &T) -> bool {
        self.nodes.iter().any(|n| n.contains(entity))
    }
}

impl<T: Clone + Eq + Hash> FromIterator<Node<T>> for NodeSet<T> {
    fn from_iter<I: IntoIterator<Item = Node<T>>>(iter: I) -> Self {
        Self {
            nodes: iter.into_iter().collect(),
        }
    }
}

// ── Inference Types ──────────────────────────────────────────────────────────

/// Types of inferences that can be precomputed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InferenceType {
    ClassAssertions,
    ClassHierarchy,
    DataPropertyHierarchy,
    DisjointClasses,
    ObjectPropertyHierarchy,
    PropertyAssertions,
    SameIndividual,
    DifferentIndividuals,
}

/// Whether to return only direct or all results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferenceDepth {
    Direct,
    All,
}

// ── Buffering / Policy Enums ─────────────────────────────────────────────────

/// Controls whether reasoner buffers changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferingMode {
    /// Buffer changes until flush() is called.
    Buffering,
    /// Apply changes immediately.
    NonBuffering,
}

/// Controls whether the reasoner may introduce fresh entities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshEntityPolicy {
    Allowed,
    Disallowed,
}

/// Controls how individual nodes are grouped in results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndividualNodeSetPolicy {
    /// Group by sameAs relation.
    BySameAs,
    /// Each individual is a separate node.
    ByName,
}

// ── OWL Reasoner Configuration ──────────────────────────────────────────────

/// Configuration for an OWL reasoner instance.
#[derive(Clone)]
pub struct OWLReasonerConfiguration {
    pub buffering_mode: BufferingMode,
    pub fresh_entity_policy: FreshEntityPolicy,
    pub individual_node_set_policy: IndividualNodeSetPolicy,
    pub timeout: Option<Duration>,
    pub progress_monitor: Option<Arc<dyn ReasonerProgressMonitor>>,
}

impl std::fmt::Debug for OWLReasonerConfiguration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OWLReasonerConfiguration")
            .field("buffering_mode", &self.buffering_mode)
            .field("fresh_entity_policy", &self.fresh_entity_policy)
            .field(
                "individual_node_set_policy",
                &self.individual_node_set_policy,
            )
            .field("timeout", &self.timeout)
            .field(
                "progress_monitor",
                &self.progress_monitor.as_ref().map(|_| "Monitor"),
            )
            .finish()
    }
}

impl Default for OWLReasonerConfiguration {
    fn default() -> Self {
        Self {
            buffering_mode: BufferingMode::NonBuffering,
            fresh_entity_policy: FreshEntityPolicy::Allowed,
            individual_node_set_policy: IndividualNodeSetPolicy::ByName,
            timeout: None,
            progress_monitor: None,
        }
    }
}

/// Callback for tracking long-running reasoning operations.
pub trait ReasonerProgressMonitor: Send + Sync {
    /// Called periodically with progress info.
    fn on_progress(&self, message: &str, percentage: f64);

    /// Whether the operation has been cancelled.
    fn is_cancelled(&self) -> bool;
}

// ── OWL Reasoner Trait ───────────────────────────────────────────────────────

/// A standard OWL 2 reasoning interface.
///
/// Implementors provide consistency checking, classification, instance
/// checking, property reasoning, entailment verification, and lifecycle
/// management.
///
/// # Thread Safety
///
/// All methods take `&self` — implementors must use interior mutability
/// (e.g., `Mutex` or `RwLock`) for any mutable state.
pub trait OWLReasoner: Send + Sync {
    // ── Metadata ─────────────────────────────────────────────────────────

    /// Human-readable name of this reasoner.
    fn get_reasoner_name(&self) -> &'static str {
        "Oxidowl Reasoner"
    }

    /// Version of the reasoner.
    fn get_reasoner_version(&self) -> (u32, u32, u32) {
        (1, 0, 0)
    }

    // ── Configuration Accessors ───────────────────────────────────────────

    /// Get the current buffering mode.
    fn get_buffering_mode(&self) -> BufferingMode {
        BufferingMode::NonBuffering
    }

    /// Get the timeout for reasoning operations, if set.
    fn get_timeout(&self) -> Option<Duration> {
        None
    }

    /// Get the fresh entity policy.
    fn get_fresh_entity_policy(&self) -> FreshEntityPolicy {
        FreshEntityPolicy::Allowed
    }

    /// Get the individual node set policy.
    fn get_individual_node_set_policy(&self) -> IndividualNodeSetPolicy {
        IndividualNodeSetPolicy::ByName
    }

    // ── Ontology ─────────────────────────────────────────────────────────

    /// Get the root ontology this reasoner operates on.
    fn get_root_ontology(&self) -> OntologyRef;

    // ── Consistency ──────────────────────────────────────────────────────

    /// Check if the ontology is logically consistent.
    fn is_consistent(&self) -> Result<bool>;

    // ── Satisfiability ───────────────────────────────────────────────────

    /// Check if a class expression is satisfiable (can have instances).
    fn is_satisfiable(&self, class: &ClassExpression) -> Result<bool>;

    /// Get all unsatisfiable classes.
    fn get_unsatisfiable_classes(&self) -> Result<Node<ClassExpression>>;

    // ── Class Hierarchy ──────────────────────────────────────────────────

    /// Get sub-classes of the given class expression.
    fn get_sub_classes(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>>;

    /// Get super-classes of the given class expression.
    fn get_super_classes(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>>;

    /// Get classes equivalent to the given class expression.
    fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Node<ClassExpression>>;

    /// Get classes disjoint with the given class expression.
    fn get_disjoint_classes(&self, class: &ClassExpression) -> Result<NodeSet<ClassExpression>>;

    // ── Instance Checking ────────────────────────────────────────────────

    /// Get instances of the given class expression.
    fn get_instances(&self, class: &ClassExpression, direct: bool) -> Result<NodeSet<Individual>>;

    /// Get types (classes) of the given individual.
    fn get_types(&self, individual: &Individual, direct: bool) -> Result<NodeSet<ClassExpression>>;

    // ── Individual Relations ─────────────────────────────────────────────

    /// Get individuals that are the same as the given individual.
    fn get_same_individuals(&self, individual: &Individual) -> Result<Node<Individual>>;

    /// Get individuals that are different from the given individual.
    fn get_different_individuals(&self, individual: &Individual) -> Result<NodeSet<Individual>>;

    // ── Object Properties ────────────────────────────────────────────────

    /// Get the TOP object property (owl:topObjectProperty).
    fn get_top_object_property(&self) -> ObjectPropertyExpression;

    /// Get the BOTTOM object property (owl:bottomObjectProperty).
    fn get_bottom_object_property(&self) -> ObjectPropertyExpression;

    /// Get sub-properties of the given object property expression.
    fn get_sub_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ObjectPropertyExpression>>;

    /// Get super-properties of the given object property expression.
    fn get_super_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ObjectPropertyExpression>>;

    /// Get object properties equivalent to the given one.
    fn get_equivalent_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Result<Node<ObjectPropertyExpression>>;

    /// Get object properties disjoint with the given one.
    fn get_disjoint_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Result<NodeSet<ObjectPropertyExpression>>;

    /// Get inverse object properties for the given property.
    fn get_inverse_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Result<Node<ObjectPropertyExpression>>;

    /// Get domains of the given object property.
    fn get_object_property_domains(
        &self,
        prop: &ObjectPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>>;

    /// Get ranges of the given object property.
    fn get_object_property_ranges(
        &self,
        prop: &ObjectPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>>;

    // ── Data Properties ──────────────────────────────────────────────────

    /// Get the TOP data property (owl:topDataProperty).
    fn get_top_data_property(&self) -> DataPropertyExpression;

    /// Get the BOTTOM data property (owl:bottomDataProperty).
    fn get_bottom_data_property(&self) -> DataPropertyExpression;

    /// Get sub-properties of the given data property expression.
    fn get_sub_data_properties(
        &self,
        prop: &DataPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<DataPropertyExpression>>;

    /// Get super-properties of the given data property expression.
    fn get_super_data_properties(
        &self,
        prop: &DataPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<DataPropertyExpression>>;

    /// Get data properties equivalent to the given one.
    fn get_equivalent_data_properties(
        &self,
        prop: &DataPropertyExpression,
    ) -> Result<Node<DataPropertyExpression>>;

    /// Get data properties disjoint with the given one.
    fn get_disjoint_data_properties(
        &self,
        prop: &DataPropertyExpression,
    ) -> Result<NodeSet<DataPropertyExpression>>;

    /// Get domains of the given data property.
    fn get_data_property_domains(
        &self,
        prop: &DataPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>>;

    /// Get ranges of the given data property.
    fn get_data_property_ranges(
        &self,
        prop: &DataPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<DataRange>>;

    // ── Individual Property Values ───────────────────────────────────────

    /// Get object property values for the given individual and property.
    fn get_object_property_values(
        &self,
        individual: &crate::ontology::NamedIndividual,
        property: &ObjectPropertyExpression,
    ) -> Result<NodeSet<crate::ontology::NamedIndividual>> {
        let _ = (individual, property);
        Ok(NodeSet::empty())
    }

    /// Get data property values for the given individual and property.
    fn get_data_property_values(
        &self,
        individual: &crate::ontology::NamedIndividual,
        property: &DataPropertyExpression,
    ) -> Result<NodeSet<crate::ontology::Literal>> {
        let _ = (individual, property);
        Ok(NodeSet::empty())
    }

    // ── Top / Bottom Nodes ────────────────────────────────────────────────

    /// Get the TOP class node (owl:Thing equivalence class).
    fn get_top_class_node(&self) -> Result<Node<ClassExpression>> {
        Ok(Node::top_node(ClassExpression::Class(
            crate::ontology::Class {
                iri: crate::ontology::IRI::owl_thing(),
            },
        )))
    }

    /// Get the BOTTOM class node (owl:Nothing equivalence class).
    fn get_bottom_class_node(&self) -> Result<Node<ClassExpression>> {
        Ok(Node::bottom_node(ClassExpression::Class(
            crate::ontology::Class {
                iri: crate::ontology::IRI::owl_nothing(),
            },
        )))
    }

    /// Get the TOP object property node.
    fn get_top_object_property_node(&self) -> Result<Node<ObjectPropertyExpression>> {
        Ok(Node::top_node(ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty {
                iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#topObjectProperty"),
            },
        )))
    }

    /// Get the BOTTOM object property node.
    fn get_bottom_object_property_node(&self) -> Result<Node<ObjectPropertyExpression>> {
        Ok(Node::bottom_node(ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty {
                iri: crate::ontology::IRI::new(
                    "http://www.w3.org/2002/07/owl#bottomObjectProperty",
                ),
            },
        )))
    }

    /// Get the TOP data property node.
    fn get_top_data_property_node(&self) -> Result<Node<DataPropertyExpression>> {
        Ok(Node::top_node(DataPropertyExpression::DataProperty(
            crate::ontology::DataProperty {
                iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#topDataProperty"),
            },
        )))
    }

    /// Get the BOTTOM data property node.
    fn get_bottom_data_property_node(&self) -> Result<Node<DataPropertyExpression>> {
        Ok(Node::bottom_node(DataPropertyExpression::DataProperty(
            crate::ontology::DataProperty {
                iri: crate::ontology::IRI::new(
                    "http://www.w3.org/2002/07/owl#bottomDataProperty",
                ),
            },
        )))
    }

    // ── Pending Axiom Changes ────────────────────────────────────────────

    /// Get pending axiom additions in the buffering mode.
    fn get_pending_axiom_additions(&self) -> Vec<Axiom> {
        vec![]
    }

    /// Get pending axiom removals in the buffering mode.
    fn get_pending_axiom_removals(&self) -> Vec<Axiom> {
        vec![]
    }

    // ── Entailment ───────────────────────────────────────────────────────

    /// Check if a single axiom is entailed by the ontology.
    fn is_entailed(&self, axiom: &Axiom) -> Result<bool>;

    /// Check if all given axioms are entailed.
    fn is_entailed_axioms(&self, axioms: &[Axiom]) -> Result<bool> {
        for axiom in axioms {
            if !self.is_entailed(axiom)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    // ── Lifecycle ────────────────────────────────────────────────────────

    /// Precompute specified inference types (warm the cache).
    fn precompute_inferences(&self, _inference_types: &[InferenceType]) -> Result<()> {
        Ok(())
    }

    /// Check if an inference type has been precomputed.
    fn is_precomputed(&self, _inference_type: InferenceType) -> bool {
        false
    }

    /// Get the set of inference types that this reasoner can precompute.
    fn get_precomputable_inference_types(&self) -> Vec<InferenceType> {
        vec![
            InferenceType::ClassAssertions,
            InferenceType::ClassHierarchy,
            InferenceType::DataPropertyHierarchy,
            InferenceType::DisjointClasses,
            InferenceType::ObjectPropertyHierarchy,
            InferenceType::PropertyAssertions,
            InferenceType::SameIndividual,
            InferenceType::DifferentIndividuals,
        ]
    }

    /// Check whether entailment checking is supported for a given axiom type.
    fn is_entailment_checking_supported(&self, axiom_type: &crate::ontology::axioms::AxiomType) -> bool {
        matches!(
            axiom_type,
            crate::ontology::axioms::AxiomType::SubClassOf
                | crate::ontology::axioms::AxiomType::EquivalentClasses
                | crate::ontology::axioms::AxiomType::ClassAssertion
        )
    }

    /// Get pending ontology changes since last flush.
    fn get_pending_changes(&self) -> Vec<crate::manager::changes::OntologyChange> {
        vec![]
    }

    /// Flush buffered changes and recompute inferences.
    fn flush(&self) -> Result<()> {
        Ok(())
    }

    /// Release resources held by the reasoner.
    fn dispose(&self) {}

    /// Request that the reasoner interrupt its current computation.
    /// The result of the interrupted computation is undefined;
    /// typically the reasoner throws an error or returns a partial result.
    fn interrupt(&self) {
        // Default: no-op
    }
}

// ── Reasoner Factory Trait ───────────────────────────────────────────────────

/// Factory for creating reasoner instances.
pub trait ReasonerFactory: Send + Sync {
    /// Create a reasoner for the given ontology.
    fn create_reasoner(
        &self,
        ontology: &OntologyRef,
        config: &OWLReasonerConfiguration,
    ) -> Result<Box<dyn OWLReasoner>>;

    /// Human-readable name of this reasoner.
    fn get_reasoner_name(&self) -> &'static str;
}

// ── Tableau-based OWL Reasoner ───────────────────────────────────────────────

/// A full tableau-based OWL reasoner wrapping the existing `Reasoner` struct.
/// Uses `Mutex` for interior mutability to satisfy `OWLReasoner: Send + Sync`.
pub struct TableauOWLReasoner {
    ontology: OntologyRef,
    reasoner: Mutex<crate::core::reasoner::Reasoner>,
    #[allow(dead_code)]
    config: OWLReasonerConfiguration,
    /// Cached classification hierarchy (lazily computed).
    cached_hierarchy: Mutex<Option<ClassificationCache>>,
    /// Cached property hierarchy (lazily computed).
    cached_property_hierarchy: Mutex<Option<PropertyHierarchyCache>>,
}

#[derive(Debug, Clone, Default)]
struct ClassificationCache {
    sub_class_map: HashMap<ClassExpression, HashSet<ClassExpression>>,
    super_class_map: HashMap<ClassExpression, HashSet<ClassExpression>>,
    equivalent_map: HashMap<ClassExpression, HashSet<ClassExpression>>,
    instance_map: HashMap<ClassExpression, HashSet<Individual>>,
    type_map: HashMap<Individual, HashSet<ClassExpression>>,
    unsatisfiable: HashSet<ClassExpression>,
}

#[derive(Debug, Clone, Default)]
struct PropertyHierarchyCache {
    sub_object_properties: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    super_object_properties: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    equivalent_object_properties:
        HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    disjoint_object_properties:
        HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    inverse_object_properties:
        HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    object_property_domains: HashMap<ObjectPropertyExpression, HashSet<ClassExpression>>,
    object_property_ranges: HashMap<ObjectPropertyExpression, HashSet<ClassExpression>>,
    sub_data_properties: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    super_data_properties: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    equivalent_data_properties:
        HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    disjoint_data_properties: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    data_property_domains: HashMap<DataPropertyExpression, HashSet<ClassExpression>>,
    data_property_ranges: HashMap<DataPropertyExpression, HashSet<DataRange>>,
}

impl TableauOWLReasoner {
    /// Create a new tableau-based reasoner for the given ontology.
    pub fn new(ontology: OntologyRef, config: OWLReasonerConfiguration) -> Result<Self> {
        let reasoner_config = crate::config::ReasonerConfig::default();
        let mut reasoner = crate::core::reasoner::Reasoner::new(reasoner_config)?;
        let ont_clone = ontology
            .read()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?
            .clone();
        reasoner.load_ontology(ont_clone)?;

        Ok(Self {
            ontology,
            reasoner: Mutex::new(reasoner),
            config,
            cached_hierarchy: Mutex::new(None),
            cached_property_hierarchy: Mutex::new(None),
        })
    }

    /// Ensure classification has been performed, populating the cache.
    fn ensure_classified(&self) -> Result<()> {
        let mut cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        if cache.is_some() {
            return Ok(());
        }

        let mut reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;

        let classification = reasoner.classify()?;
        let mut scm: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        let mut supcm: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        let mut eqm: HashMap<ClassExpression, HashSet<ClassExpression>> = HashMap::new();
        let mut unsat: HashSet<ClassExpression> = HashSet::new();

        for (sub, supers) in &classification.hierarchy {
            if supers.is_empty() {
                // subclass with no superclasses = unsatisfiable (owl:Nothing)
                unsat.insert(sub.clone());
            }
            for sup in supers {
                scm.entry(sub.clone()).or_default().insert(sup.clone());
                supcm.entry(sup.clone()).or_default().insert(sub.clone());
            }
        }

        // Build equivalence map: classes that are mutual sub/super
        for (sub, supers) in &classification.hierarchy {
            for sup in supers {
                if let Some(sub_supers) = classification.hierarchy.get(sup) {
                    if sub_supers.contains(sub) {
                        eqm.entry(sub.clone()).or_default().insert(sup.clone());
                        eqm.entry(sup.clone()).or_default().insert(sub.clone());
                    }
                }
            }
        }

        // Instance and type maps require realisation
        let realisation = reasoner.realize()?;
        let mut im: HashMap<ClassExpression, HashSet<Individual>> = HashMap::new();
        let mut tm: HashMap<Individual, HashSet<ClassExpression>> = HashMap::new();
        for (ind, types) in &realisation.types {
            for ce in types {
                im.entry(ce.clone()).or_default().insert(ind.clone());
                tm.entry(ind.clone()).or_default().insert(ce.clone());
            }
        }

        *cache = Some(ClassificationCache {
            sub_class_map: scm,
            super_class_map: supcm,
            equivalent_map: eqm,
            instance_map: im,
            type_map: tm,
            unsatisfiable: unsat,
        });

        Ok(())
    }

    #[allow(dead_code)]
    fn with_cache<F, T>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&ClassificationCache) -> T,
    {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        Ok(f(cache.as_ref().unwrap()))
    }

    /// Ensure property hierarchy has been computed, populating the cache.
    fn ensure_property_hierarchy(&self) -> Result<()> {
        let mut cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        if cache.is_some() {
            return Ok(());
        }

        let guard = self.ontology.read().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let mut phc = PropertyHierarchyCache::default();

        // Index all property axioms
        for axiom in guard.axioms() {
            match axiom {
                Axiom::SubObjectPropertyOf(a) => {
                    phc.sub_object_properties
                        .entry(a.sub_property.clone())
                        .or_default()
                        .insert(a.super_property.clone());
                    phc.super_object_properties
                        .entry(a.super_property.clone())
                        .or_default()
                        .insert(a.sub_property.clone());
                }
                Axiom::EquivalentObjectProperties(a) => {
                    for p1 in &a.properties {
                        for p2 in &a.properties {
                            if p1 != p2 {
                                phc.equivalent_object_properties
                                    .entry(p1.clone())
                                    .or_default()
                                    .insert(p2.clone());
                            }
                        }
                    }
                }
                Axiom::DisjointObjectProperties(a) => {
                    for p1 in &a.properties {
                        for p2 in &a.properties {
                            if p1 != p2 {
                                phc.disjoint_object_properties
                                    .entry(p1.clone())
                                    .or_default()
                                    .insert(p2.clone());
                            }
                        }
                    }
                }
                Axiom::InverseObjectProperties(a) => {
                    phc.inverse_object_properties
                        .entry(a.property1.clone())
                        .or_default()
                        .insert(a.property2.clone());
                    phc.inverse_object_properties
                        .entry(a.property2.clone())
                        .or_default()
                        .insert(a.property1.clone());
                }
                Axiom::ObjectPropertyDomain(a) => {
                    phc.object_property_domains
                        .entry(a.property.clone())
                        .or_default()
                        .insert(a.domain.clone());
                }
                Axiom::ObjectPropertyRange(a) => {
                    phc.object_property_ranges
                        .entry(a.property.clone())
                        .or_default()
                        .insert(a.range.clone());
                }
                Axiom::SubDataPropertyOf(a) => {
                    phc.sub_data_properties
                        .entry(a.sub_property.clone())
                        .or_default()
                        .insert(a.super_property.clone());
                    phc.super_data_properties
                        .entry(a.super_property.clone())
                        .or_default()
                        .insert(a.sub_property.clone());
                }
                Axiom::EquivalentDataProperties(a) => {
                    for p1 in &a.properties {
                        for p2 in &a.properties {
                            if p1 != p2 {
                                phc.equivalent_data_properties
                                    .entry(p1.clone())
                                    .or_default()
                                    .insert(p2.clone());
                            }
                        }
                    }
                }
                Axiom::DisjointDataProperties(a) => {
                    for p1 in &a.properties {
                        for p2 in &a.properties {
                            if p1 != p2 {
                                phc.disjoint_data_properties
                                    .entry(p1.clone())
                                    .or_default()
                                    .insert(p2.clone());
                            }
                        }
                    }
                }
                Axiom::DataPropertyDomain(a) => {
                    phc.data_property_domains
                        .entry(a.property.clone())
                        .or_default()
                        .insert(a.domain.clone());
                }
                Axiom::DataPropertyRange(a) => {
                    phc.data_property_ranges
                        .entry(a.property.clone())
                        .or_default()
                        .insert(a.range.clone());
                }
                _ => {}
            }
        }

        // Compute transitive closure for sub-property hierarchies
        // Object properties
        {
            let op_keys: Vec<_> = phc.sub_object_properties.keys().cloned().collect();
            for k in &op_keys {
                let mut to_add = Vec::new();
                if let Some(direct) = phc.sub_object_properties.get(k).cloned() {
                    for target in &direct {
                        if let Some(transitive) = phc.sub_object_properties.get(target) {
                            for t in transitive {
                                to_add.push(t.clone());
                            }
                        }
                    }
                }
                for t in to_add {
                    phc.sub_object_properties.entry(k.clone()).or_default().insert(t);
                }
            }
            let op_keys2: Vec<_> = phc.super_object_properties.keys().cloned().collect();
            for k in &op_keys2 {
                let mut to_add = Vec::new();
                if let Some(direct) = phc.super_object_properties.get(k).cloned() {
                    for target in &direct {
                        if let Some(transitive) = phc.super_object_properties.get(target) {
                            for t in transitive {
                                to_add.push(t.clone());
                            }
                        }
                    }
                }
                for t in to_add {
                    phc.super_object_properties.entry(k.clone()).or_default().insert(t);
                }
            }
            // Data properties
            let dp_keys: Vec<_> = phc.sub_data_properties.keys().cloned().collect();
            for k in &dp_keys {
                let mut to_add = Vec::new();
                if let Some(direct) = phc.sub_data_properties.get(k).cloned() {
                    for target in &direct {
                        if let Some(transitive) = phc.sub_data_properties.get(target) {
                            for t in transitive {
                                to_add.push(t.clone());
                            }
                        }
                    }
                }
                for t in to_add {
                    phc.sub_data_properties.entry(k.clone()).or_default().insert(t);
                }
            }
            let dp_keys2: Vec<_> = phc.super_data_properties.keys().cloned().collect();
            for k in &dp_keys2 {
                let mut to_add = Vec::new();
                if let Some(direct) = phc.super_data_properties.get(k).cloned() {
                    for target in &direct {
                        if let Some(transitive) = phc.super_data_properties.get(target) {
                            for t in transitive {
                                to_add.push(t.clone());
                            }
                        }
                    }
                }
                for t in to_add {
                    phc.super_data_properties.entry(k.clone()).or_default().insert(t);
                }
            }
        }

        // Propagate inverses through sub-property chains
        let op_keys: Vec<_> = phc.inverse_object_properties.keys().cloned().collect();
        for prop in &op_keys {
            if let Some(inverses) = phc.inverse_object_properties.get(prop).cloned() {
                for inv in &inverses {
                    if let Some(subs) = phc.sub_object_properties.get(prop).cloned() {
                        for sub in &subs {
                            phc.inverse_object_properties
                                .entry(sub.clone())
                                .or_default()
                                .insert(inv.clone());
                        }
                    }
                }
            }
        }

        drop(guard);
        *cache = Some(phc);
        Ok(())
    }
}

impl OWLReasoner for TableauOWLReasoner {
    fn get_root_ontology(&self) -> OntologyRef {
        self.ontology.clone()
    }

    fn get_reasoner_name(&self) -> &'static str {
        "Oxidowl Tableau Reasoner"
    }

    fn get_buffering_mode(&self) -> BufferingMode {
        self.config.buffering_mode
    }

    fn get_timeout(&self) -> Option<Duration> {
        self.config.timeout
    }

    fn get_fresh_entity_policy(&self) -> FreshEntityPolicy {
        self.config.fresh_entity_policy
    }

    fn get_individual_node_set_policy(&self) -> IndividualNodeSetPolicy {
        self.config.individual_node_set_policy
    }

    fn is_consistent(&self) -> Result<bool> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        reasoner.is_consistent()
    }

    fn is_satisfiable(&self, class: &ClassExpression) -> Result<bool> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        reasoner.is_class_satisfiable(class)
    }

    fn get_unsatisfiable_classes(&self) -> Result<Node<ClassExpression>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if cache.unsatisfiable.is_empty() {
            Ok(Node::bottom_node(ClassExpression::Class(
                crate::ontology::Class {
                    iri: crate::ontology::IRI::owl_nothing(),
                },
            )))
        } else {
            Ok(Node::new(cache.unsatisfiable.clone()))
        }
    }

    fn get_sub_classes(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        let mut result: HashSet<Node<ClassExpression>> = HashSet::new();
        if let Some(subs) = cache.super_class_map.get(class) {
            for sub in subs {
                if direct {
                    // Direct subclass: no intermediate class
                    let is_direct = !cache.super_class_map.get(class).is_some_and(|all_subs| {
                        all_subs.iter().any(|other_sub| {
                            other_sub != sub
                                && cache
                                    .super_class_map
                                    .get(other_sub)
                                    .is_some_and(|transitive| transitive.contains(sub))
                        })
                    });
                    if is_direct {
                        result.insert(Node::singleton(sub.clone()));
                    }
                } else {
                    result.insert(Node::singleton(sub.clone()));
                }
            }
        }
        Ok(NodeSet::new(result))
    }

    fn get_super_classes(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<NodeSet<ClassExpression>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        let mut result: HashSet<Node<ClassExpression>> = HashSet::new();
        if let Some(supers) = cache.sub_class_map.get(class) {
            for sup in supers {
                if direct {
                    let is_direct = !cache.sub_class_map.get(class).is_some_and(|all_sups| {
                        all_sups.iter().any(|other_sup| {
                            other_sup != sup
                                && cache
                                    .sub_class_map
                                    .get(other_sup)
                                    .is_some_and(|transitive| transitive.contains(sup))
                        })
                    });
                    if is_direct {
                        result.insert(Node::singleton(sup.clone()));
                    }
                } else {
                    result.insert(Node::singleton(sup.clone()));
                }
            }
        }
        Ok(NodeSet::new(result))
    }

    fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Node<ClassExpression>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        if let Some(eq_set) = cache.equivalent_map.get(class) {
            Ok(Node::new(eq_set.clone()))
        } else {
            Ok(Node::singleton(class.clone()))
        }
    }

    fn get_disjoint_classes(&self, class: &ClassExpression) -> Result<NodeSet<ClassExpression>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        let mut disjoint_set = HashSet::new();
        let _bottom = ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::owl_nothing(),
        });
        if let Some(subs) = cache.super_class_map.get(class) {
            for sub in subs {
                if cache.unsatisfiable.contains(sub) {
                    disjoint_set.insert(sub.clone());
                }
            }
        }
        if disjoint_set.is_empty() {
            Ok(NodeSet::empty())
        } else {
            let mut nodes = HashSet::new();
            for d in disjoint_set {
                nodes.insert(Node::singleton(d));
            }
            Ok(NodeSet::new(nodes))
        }
    }

    fn get_instances(&self, class: &ClassExpression, _direct: bool) -> Result<NodeSet<Individual>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        let mut result: HashSet<Node<Individual>> = HashSet::new();
        if let Some(instances) = cache.instance_map.get(class) {
            for ind in instances {
                result.insert(Node::singleton(ind.clone()));
            }
        }
        Ok(NodeSet::new(result))
    }

    fn get_types(&self, individual: &Individual, direct: bool) -> Result<NodeSet<ClassExpression>> {
        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        let mut result: HashSet<Node<ClassExpression>> = HashSet::new();
        if let Some(types) = cache.type_map.get(individual) {
            let mut all_types = types.clone();
            if direct {
                // Remove types that have proper subtypes in the result set
                let to_remove: HashSet<_> = all_types
                    .iter()
                    .filter(|t| {
                        all_types.iter().any(|other| {
                            other != *t
                                && cache
                                    .super_class_map
                                    .get(other)
                                    .is_some_and(|subs| subs.contains(t))
                        })
                    })
                    .cloned()
                    .collect();
                all_types.retain(|t| !to_remove.contains(t));
            }
            for t in all_types {
                result.insert(Node::singleton(t));
            }
        }
        Ok(NodeSet::new(result))
    }

    fn get_same_individuals(&self, individual: &Individual) -> Result<Node<Individual>> {
        Ok(Node::singleton(individual.clone()))
    }

    fn get_different_individuals(&self, _individual: &Individual) -> Result<NodeSet<Individual>> {
        Ok(NodeSet::empty())
    }

    fn get_top_object_property(&self) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#topObjectProperty"),
        })
    }

    fn get_bottom_object_property(&self) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#bottomObjectProperty"),
        })
    }

    fn get_sub_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ObjectPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(subs) = cache.super_object_properties.get(prop) {
            if direct {
                Ok(NodeSet::new(subs.iter().map(|s| Node::singleton(s.clone())).collect()))
            } else {
                let mut all = subs.clone();
                let mut stack: Vec<_> = subs.iter().cloned().collect();
                while let Some(current) = stack.pop() {
                    if let Some(trans) = cache.super_object_properties.get(&current) {
                        for t in trans {
                            if all.insert(t.clone()) {
                                stack.push(t.clone());
                            }
                        }
                    }
                }
                Ok(NodeSet::new(all.into_iter().map(Node::singleton).collect()))
            }
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_super_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<ObjectPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(sups) = cache.sub_object_properties.get(prop) {
            if direct {
                Ok(NodeSet::new(sups.iter().map(|s| Node::singleton(s.clone())).collect()))
            } else {
                let mut all = sups.clone();
                let mut stack: Vec<_> = sups.iter().cloned().collect();
                while let Some(current) = stack.pop() {
                    if let Some(trans) = cache.sub_object_properties.get(&current) {
                        for t in trans {
                            if all.insert(t.clone()) {
                                stack.push(t.clone());
                            }
                        }
                    }
                }
                Ok(NodeSet::new(all.into_iter().map(Node::singleton).collect()))
            }
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_equivalent_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Result<Node<ObjectPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(eq) = cache.equivalent_object_properties.get(prop) {
            let mut set = eq.clone();
            set.insert(prop.clone());
            Ok(Node::new(set))
        } else {
            Ok(Node::singleton(prop.clone()))
        }
    }

    fn get_disjoint_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Result<NodeSet<ObjectPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(disj) = cache.disjoint_object_properties.get(prop) {
            Ok(NodeSet::new(disj.iter().map(|d| Node::singleton(d.clone())).collect()))
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_inverse_object_properties(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Result<Node<ObjectPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        let mut inverses = HashSet::new();
        // Always include structural inverse
        match prop {
            ObjectPropertyExpression::ObjectProperty(p) => {
                inverses.insert(ObjectPropertyExpression::InverseObjectProperty(p.clone()));
            }
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                inverses.insert(ObjectPropertyExpression::ObjectProperty(p.clone()));
            }
            ObjectPropertyExpression::PropertyChain(_) => {
                inverses.insert(prop.clone());
            }
        }
        // Add declared inverses
        if let Some(declared) = cache.inverse_object_properties.get(prop) {
            inverses.extend(declared.iter().cloned());
        }
        Ok(Node::new(inverses))
    }

    fn get_object_property_domains(
        &self,
        prop: &ObjectPropertyExpression,
        _direct: bool,
    ) -> Result<NodeSet<ClassExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(domains) = cache.object_property_domains.get(prop) {
            Ok(NodeSet::new(domains.iter().map(|d| Node::singleton(d.clone())).collect()))
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_object_property_ranges(
        &self,
        prop: &ObjectPropertyExpression,
        _direct: bool,
    ) -> Result<NodeSet<ClassExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(ranges) = cache.object_property_ranges.get(prop) {
            Ok(NodeSet::new(ranges.iter().map(|r| Node::singleton(r.clone())).collect()))
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_top_data_property(&self) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(crate::ontology::DataProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#topDataProperty"),
        })
    }

    fn get_bottom_data_property(&self) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(crate::ontology::DataProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#bottomDataProperty"),
        })
    }

    fn get_sub_data_properties(
        &self,
        prop: &DataPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<DataPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(subs) = cache.super_data_properties.get(prop) {
            if direct {
                Ok(NodeSet::new(subs.iter().map(|s| Node::singleton(s.clone())).collect()))
            } else {
                let mut all = subs.clone();
                let mut stack: Vec<_> = subs.iter().cloned().collect();
                while let Some(current) = stack.pop() {
                    if let Some(trans) = cache.super_data_properties.get(&current) {
                        for t in trans {
                            if all.insert(t.clone()) {
                                stack.push(t.clone());
                            }
                        }
                    }
                }
                Ok(NodeSet::new(all.into_iter().map(Node::singleton).collect()))
            }
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_super_data_properties(
        &self,
        prop: &DataPropertyExpression,
        direct: bool,
    ) -> Result<NodeSet<DataPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(sups) = cache.sub_data_properties.get(prop) {
            if direct {
                Ok(NodeSet::new(sups.iter().map(|s| Node::singleton(s.clone())).collect()))
            } else {
                let mut all = sups.clone();
                let mut stack: Vec<_> = sups.iter().cloned().collect();
                while let Some(current) = stack.pop() {
                    if let Some(trans) = cache.sub_data_properties.get(&current) {
                        for t in trans {
                            if all.insert(t.clone()) {
                                stack.push(t.clone());
                            }
                        }
                    }
                }
                Ok(NodeSet::new(all.into_iter().map(Node::singleton).collect()))
            }
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_equivalent_data_properties(
        &self,
        prop: &DataPropertyExpression,
    ) -> Result<Node<DataPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(eq) = cache.equivalent_data_properties.get(prop) {
            let mut set = eq.clone();
            set.insert(prop.clone());
            Ok(Node::new(set))
        } else {
            Ok(Node::singleton(prop.clone()))
        }
    }

    fn get_disjoint_data_properties(
        &self,
        prop: &DataPropertyExpression,
    ) -> Result<NodeSet<DataPropertyExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(disj) = cache.disjoint_data_properties.get(prop) {
            Ok(NodeSet::new(disj.iter().map(|d| Node::singleton(d.clone())).collect()))
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_data_property_domains(
        &self,
        prop: &DataPropertyExpression,
        _direct: bool,
    ) -> Result<NodeSet<ClassExpression>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(domains) = cache.data_property_domains.get(prop) {
            Ok(NodeSet::new(domains.iter().map(|d| Node::singleton(d.clone())).collect()))
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_data_property_ranges(
        &self,
        prop: &DataPropertyExpression,
        _direct: bool,
    ) -> Result<NodeSet<DataRange>> {
        self.ensure_property_hierarchy()?;
        let cache = self
            .cached_property_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();
        if let Some(ranges) = cache.data_property_ranges.get(prop) {
            Ok(NodeSet::new(ranges.iter().map(|r| Node::singleton(r.clone())).collect()))
        } else {
            Ok(NodeSet::empty())
        }
    }

    fn get_object_property_values(
        &self,
        individual: &crate::ontology::NamedIndividual,
        property: &ObjectPropertyExpression,
    ) -> Result<NodeSet<crate::ontology::NamedIndividual>> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let ind = Individual::Named(individual.clone());
        let values = reasoner.get_object_property_values(&ind, property)?;
        let nodes: HashSet<Node<crate::ontology::NamedIndividual>> = values
            .into_iter()
            .filter_map(|v| {
                if let Individual::Named(ni) = v {
                    Some(Node::singleton(ni))
                } else {
                    None
                }
            })
            .collect();
        Ok(NodeSet::new(nodes))
    }

    fn get_data_property_values(
        &self,
        individual: &crate::ontology::NamedIndividual,
        property: &DataPropertyExpression,
    ) -> Result<NodeSet<crate::ontology::Literal>> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let ind = Individual::Named(individual.clone());
        let values = reasoner.get_data_property_values(&ind, property)?;
        Ok(NodeSet::new(
            values.into_iter().map(Node::singleton).collect(),
        ))
    }

    fn is_entailed(&self, axiom: &Axiom) -> Result<bool> {
        let mut reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let mut stats = crate::core::reasoner::ReasoningStatistics::default();
        reasoner.check_entailment(axiom, &self.ontology, &mut stats)
    }

    fn precompute_inferences(&self, inference_types: &[InferenceType]) -> Result<()> {
        for it in inference_types {
            match it {
                InferenceType::ClassHierarchy => {
                    self.ensure_classified()?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn is_precomputed(&self, inference_type: InferenceType) -> bool {
        match inference_type {
            InferenceType::ClassHierarchy => self
                .cached_hierarchy
                .lock()
                .map(|c| c.is_some())
                .unwrap_or(false),
            _ => false,
        }
    }
}

// ── Tableau Reasoner Factory ─────────────────────────────────────────────────

/// Factory for creating tableau-based OWL reasoners.
#[derive(Debug, Clone, Copy, Default)]
pub struct TableauReasonerFactory;

impl ReasonerFactory for TableauReasonerFactory {
    fn create_reasoner(
        &self,
        ontology: &OntologyRef,
        config: &OWLReasonerConfiguration,
    ) -> Result<Box<dyn OWLReasoner>> {
        Ok(Box::new(TableauOWLReasoner::new(
            ontology.clone(),
            config.clone(),
        )?))
    }

    fn get_reasoner_name(&self) -> &'static str {
        "Oxidowl Tableau Reasoner"
    }
}

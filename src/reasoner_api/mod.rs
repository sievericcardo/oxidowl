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
    Ontology, OntologyRef,
};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::time::Duration;

// ── Top / Bottom Entity Markers ──────────────────────────────────────────────

/// Identifies entities that are the built-in TOP or BOTTOM marker of their
/// hierarchy (e.g. `owl:Thing` / `owl:Nothing`, `owl:topObjectProperty` /
/// `owl:bottomObjectProperty`, `owl:topDataProperty` / `owl:bottomDataProperty`).
///
/// Used by [`Node::get_flattened_representative`] and the `get_entities_minus_*`
/// helpers to mirror OWLAPI's behaviour of preferring non-marker entities.
pub trait IsTopBottomEntity {
    /// Whether this entity is a built-in TOP or BOTTOM marker.
    fn is_top_bottom_entity(&self) -> bool;
}

impl IsTopBottomEntity for ClassExpression {
    fn is_top_bottom_entity(&self) -> bool {
        matches!(self, ClassExpression::Class(c) if c.is_thing() || c.is_nothing())
    }
}

impl IsTopBottomEntity for ObjectPropertyExpression {
    fn is_top_bottom_entity(&self) -> bool {
        matches!(self, ObjectPropertyExpression::ObjectProperty(p) if is_top_bottom_object_property_iri(&p.iri))
    }
}

impl IsTopBottomEntity for DataPropertyExpression {
    fn is_top_bottom_entity(&self) -> bool {
        matches!(self, DataPropertyExpression::DataProperty(p) if is_top_bottom_data_property_iri(&p.iri))
    }
}

impl IsTopBottomEntity for Individual {
    fn is_top_bottom_entity(&self) -> bool {
        false
    }
}

impl IsTopBottomEntity for crate::ontology::NamedIndividual {
    fn is_top_bottom_entity(&self) -> bool {
        false
    }
}

impl IsTopBottomEntity for crate::ontology::Literal {
    fn is_top_bottom_entity(&self) -> bool {
        false
    }
}

fn is_top_bottom_object_property_iri(iri: &crate::ontology::IRI) -> bool {
    let s = iri.as_str();
    s == "http://www.w3.org/2002/07/owl#topObjectProperty"
        || s == "http://www.w3.org/2002/07/owl#bottomObjectProperty"
}

fn is_top_bottom_data_property_iri(iri: &crate::ontology::IRI) -> bool {
    let s = iri.as_str();
    s == "http://www.w3.org/2002/07/owl#topDataProperty"
        || s == "http://www.w3.org/2002/07/owl#bottomDataProperty"
}

fn is_top_or_bottom_class(ce: &ClassExpression) -> bool {
    matches!(ce, ClassExpression::Class(c) if c.is_thing() || c.is_nothing())
}

/// Reconstruct an [`Individual`] from its Display form (the key used by the
/// tableau's nominal labels / `individual_map`). Named individuals display as
/// their IRI; anonymous individuals display as their stable `_:id` key, which
/// round-trips through [`crate::ontology::AnonymousIndividual`]'s `id` field
/// (no ids are minted here), so `get_same_individuals` / `get_different_individuals`
/// can return anonymous members of a merged node.
fn individual_from_display(display: &str) -> Option<Individual> {
    if let Some(id) = display.strip_prefix("_:") {
        return Some(Individual::Anonymous(
            crate::ontology::AnonymousIndividual { id: id.to_string() },
        ));
    }
    Some(Individual::named(crate::ontology::IRI::new(display)))
}

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
    ///
    /// # Panics
    ///
    /// Panics if the equivalence node has no entities.
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

    /// Get all entities in this node minus the built-in TOP marker
    /// (e.g. `owl:Thing`).
    #[must_use]
    pub fn get_entities_minus_top(&self) -> HashSet<T>
    where
        T: IsTopBottomEntity,
    {
        self.entities
            .iter()
            .filter(|e| !e.is_top_bottom_entity())
            .cloned()
            .collect()
    }

    /// Get all entities in this node minus the built-in BOTTOM marker
    /// (e.g. `owl:Nothing`).
    #[must_use]
    pub fn get_entities_minus_bottom(&self) -> HashSet<T>
    where
        T: IsTopBottomEntity,
    {
        self.entities
            .iter()
            .filter(|e| !e.is_top_bottom_entity())
            .cloned()
            .collect()
    }

    /// Get all entities in this node minus both the TOP and BOTTOM markers.
    #[must_use]
    pub fn get_entities_minus_top_bottom(&self) -> HashSet<T>
    where
        T: IsTopBottomEntity,
    {
        self.entities
            .iter()
            .filter(|e| !e.is_top_bottom_entity())
            .cloned()
            .collect()
    }

    /// Get a representative element, preferring an entity that is not a built-in
    /// TOP or BOTTOM marker. For a bottom node such as `{owl:Nothing, K}` this
    /// returns `K`; for a plain node it behaves like [`get_representative_element`].
    #[must_use]
    pub fn get_flattened_representative(&self) -> T
    where
        T: Clone + IsTopBottomEntity,
    {
        self.entities
            .iter()
            .find(|e| !e.is_top_bottom_entity())
            .cloned()
            .unwrap_or_else(|| self.get_representative_element())
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

    /// Get all entities flattened across all nodes, excluding the TOP node.
    #[must_use]
    pub fn get_entities_minus_top(&self) -> HashSet<T> {
        let mut result = HashSet::new();
        for node in &self.nodes {
            if node.is_top_node() {
                continue;
            }
            result.extend(node.entities.iter().cloned());
        }
        result
    }

    /// Get all entities flattened across all nodes, excluding the BOTTOM node.
    #[must_use]
    pub fn get_entities_minus_bottom(&self) -> HashSet<T> {
        let mut result = HashSet::new();
        for node in &self.nodes {
            if node.is_bottom_node() {
                continue;
            }
            result.extend(node.entities.iter().cloned());
        }
        result
    }

    /// Get all entities flattened across all nodes, excluding both the TOP and
    /// BOTTOM nodes. Mirrors OWLAPI's `getEntitiesMinusTopMinusBottom`.
    #[must_use]
    pub fn get_entities_minus_top_bottom(&self) -> HashSet<T> {
        let mut result = HashSet::new();
        for node in &self.nodes {
            if node.is_top_node() || node.is_bottom_node() {
                continue;
            }
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

// ── Individual Equality (SameAs / DifferentAs) ──────────────────────────────

/// Precomputed `SameIndividual` / `DifferentIndividuals` reasoning for an
/// ontology.
///
/// Same-as components are built as the connected components of the graph whose
/// edges are the asserted `SameIndividual` axioms, additionally closed over
/// equality forced by functional / inverse-functional object properties (a
/// syntactic, sound approximation). Different individuals are tracked at the
/// level of same-as components: any member of one component is different from
/// every member of a component it is pairwise-declared different from.
#[derive(Debug, Clone, Default)]
pub struct IndividualEquality {
    /// Same-as component representative -> set of members (includes the
    /// representative).
    components: HashMap<Individual, HashSet<Individual>>,
    /// Individual -> representative of its same-as component.
    rep_of: HashMap<Individual, Individual>,
    /// Component representative -> set of different-component representatives.
    different: HashMap<Individual, HashSet<Individual>>,
    /// Whether a `SameIndividual`/`DifferentIndividuals` contradiction was
    /// detected (the ontology is then inconsistent).
    pub contradiction: bool,
}

impl IndividualEquality {
    /// Build the equality structure from an ontology.
    ///
    /// # Panics
    ///
    /// Panics if an equivalence component is unexpectedly empty.
    #[must_use]
    pub fn from_ontology(ontology: &Ontology) -> Self {
        let mut adjacency: HashMap<Individual, Vec<Individual>> = HashMap::new();
        let mut all: HashSet<Individual> = HashSet::new();
        let mut add_edge = |a: Individual, b: Individual| {
            adjacency.entry(a.clone()).or_default().push(b.clone());
            adjacency.entry(b).or_default().push(a);
        };

        // 1. SameIndividual axioms → connected components.
        for axiom in ontology.axioms() {
            if let Axiom::SameIndividual(sa) = axiom {
                for i in 0..sa.individuals.len() {
                    all.insert(sa.individuals[i].clone());
                    for j in (i + 1)..sa.individuals.len() {
                        add_edge(sa.individuals[i].clone(), sa.individuals[j].clone());
                    }
                }
            }
        }

        // 2. Functional / inverse-functional property-driven equality (syntactic).
        Self::collect_functional_equality(ontology, &mut add_edge, &mut all);

        // 3. BFS to build same-as components.
        let mut components: HashMap<Individual, HashSet<Individual>> = HashMap::new();
        let mut rep_of: HashMap<Individual, Individual> = HashMap::new();
        let mut visited: HashSet<Individual> = HashSet::new();
        for start in &all {
            if visited.contains(start) {
                continue;
            }
            let mut component = HashSet::new();
            let mut stack = vec![start.clone()];
            visited.insert(start.clone());
            while let Some(node) = stack.pop() {
                component.insert(node.clone());
                if let Some(neighbors) = adjacency.get(&node) {
                    for n in neighbors {
                        if visited.insert(n.clone()) {
                            stack.push(n.clone());
                        }
                    }
                }
            }
            let rep = component
                .iter()
                .next()
                .cloned()
                .expect("component is never empty");
            for m in &component {
                rep_of.insert(m.clone(), rep.clone());
            }
            components.insert(rep, component);
        }

        // 4. DifferentIndividuals → edges between component representatives.
        let mut different: HashMap<Individual, HashSet<Individual>> = HashMap::new();
        let mut contradiction = false;
        for axiom in ontology.axioms() {
            if let Axiom::DifferentIndividuals(da) = axiom {
                let reps: HashSet<Individual> = da
                    .individuals
                    .iter()
                    .map(|i| rep_of.get(i).cloned().unwrap_or_else(|| i.clone()))
                    .collect();
                for r1 in &reps {
                    for r2 in &reps {
                        if r1 != r2 {
                            different.entry(r1.clone()).or_default().insert(r2.clone());
                        }
                    }
                }
                // Two members of a DifferentIndividuals axiom in the SAME same-as
                // component means SameIndividual ⊓ DifferentIndividuals → inconsistent.
                let mut seen_comps: HashSet<Individual> = HashSet::new();
                for i in &da.individuals {
                    let r = rep_of.get(i).cloned().unwrap_or_else(|| i.clone());
                    if !seen_comps.insert(r) {
                        contradiction = true;
                    }
                }
            }
        }

        Self {
            components,
            rep_of,
            different,
            contradiction,
        }
    }

    /// Syntactic (sound) equality derivation from functional and
    /// inverse-functional object properties:
    /// - `p` functional ∧ `p(s,t1)` ∧ `p(s,t2)`  ⇒  `t1 ≡ t2`
    /// - `p` inverse-functional ∧ `p(s1,t)` ∧ `p(s2,t)`  ⇒  `s1 ≡ s2`
    fn collect_functional_equality(
        ontology: &Ontology,
        add_edge: &mut impl FnMut(Individual, Individual),
        all: &mut HashSet<Individual>,
    ) {
        let mut functional: HashSet<ObjectPropertyExpression> = HashSet::new();
        let mut inverse_functional: HashSet<ObjectPropertyExpression> = HashSet::new();
        let mut assertions: Vec<&crate::ontology::axioms::ObjectPropertyAssertionAxiom> =
            Vec::new();
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::FunctionalObjectProperty(a) => {
                    functional.insert(a.property.clone());
                }
                Axiom::InverseFunctionalObjectProperty(a) => {
                    inverse_functional.insert(a.property.clone());
                }
                Axiom::ObjectPropertyAssertion(a) => assertions.push(a),
                _ => {}
            }
        }
        if functional.is_empty() && inverse_functional.is_empty() {
            return;
        }

        let mut by_src: HashMap<(ObjectPropertyExpression, Individual), Vec<Individual>> =
            HashMap::new();
        let mut by_tgt: HashMap<(ObjectPropertyExpression, Individual), Vec<Individual>> =
            HashMap::new();
        for a in assertions {
            if functional.contains(&a.property) {
                by_src
                    .entry((a.property.clone(), a.source.clone()))
                    .or_default()
                    .push(a.target.clone());
            }
            if inverse_functional.contains(&a.property) {
                by_tgt
                    .entry((a.property.clone(), a.target.clone()))
                    .or_default()
                    .push(a.source.clone());
            }
        }
        for ((_, _), targets) in by_src {
            for i in 0..targets.len() {
                all.insert(targets[i].clone());
                for j in (i + 1)..targets.len() {
                    add_edge(targets[i].clone(), targets[j].clone());
                }
            }
        }
        for ((_, _), sources) in by_tgt {
            for i in 0..sources.len() {
                all.insert(sources[i].clone());
                for j in (i + 1)..sources.len() {
                    add_edge(sources[i].clone(), sources[j].clone());
                }
            }
        }
    }

    /// Get the `Node` of all individuals equivalent to `individual`.
    #[must_use]
    pub fn same_as(&self, individual: &Individual) -> Node<Individual> {
        if let Some(rep) = self.rep_of.get(individual)
            && let Some(members) = self.components.get(rep)
        {
            return Node::new(members.clone());
        }
        Node::singleton(individual.clone())
    }

    /// Get the `NodeSet` of individuals different from `individual`, grouped by
    /// same-as component.
    #[must_use]
    pub fn different(&self, individual: &Individual) -> NodeSet<Individual> {
        let rep = self
            .rep_of
            .get(individual)
            .cloned()
            .unwrap_or_else(|| individual.clone());
        let mut nodes = HashSet::new();
        if let Some(diffs) = self.different.get(&rep) {
            for d in diffs {
                let members = self
                    .components
                    .get(d)
                    .cloned()
                    .unwrap_or_else(|| HashSet::from([d.clone()]));
                nodes.insert(Node::new(members));
            }
        }
        NodeSet::new(nodes)
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
    /// Buffer changes until `flush()` is called.
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

    // ── OWLAPI / HermiT parity helpers ───────────────────────────────────────

    /// Whether `individual` is an instance of `class` (entailed by the
    /// ontology) — the OWL API's `hasType`.
    ///
    /// Default: `class` is among [`OWLReasoner::get_types`] of the individual,
    /// or a *superclass* of one of them (computed through
    /// [`OWLReasoner::get_super_classes`], so upward inference works even for
    /// implementors whose `get_types` only reports asserted memberships).
    /// Implementors with a dedicated instance check override this with the
    /// semantically complete answer (the tableau reasoner routes it through
    /// `Reasoner::is_instance_of`, which also covers derived memberships).
    fn has_type(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        let types = self.get_types(individual, false)?;
        if types.contains_entity(class) {
            return Ok(true);
        }
        for type_expression in types.get_flattened() {
            if self
                .get_super_classes(&type_expression, false)?
                .contains_entity(class)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Whether `a` and `b` are the same individual (entailed by the ontology)
    /// — the OWL API's `isSameIndividual`.
    ///
    /// Default: membership in [`OWLReasoner::get_same_individuals`], which
    /// merges explicit `SameIndividual` axioms with any equality the reasoner
    /// materialises.
    fn is_same_individual(&self, a: &Individual, b: &Individual) -> Result<bool> {
        Ok(self.get_same_individuals(a)?.contains(b))
    }

    /// Whether `subject --property--> object` is entailed — the OWL API's
    /// `hasObjectPropertyRelationship`.
    ///
    /// Default: membership in [`OWLReasoner::get_object_property_values`].
    fn has_object_property_relationship(
        &self,
        subject: &crate::ontology::NamedIndividual,
        property: &ObjectPropertyExpression,
        object: &crate::ontology::NamedIndividual,
    ) -> Result<bool> {
        Ok(self
            .get_object_property_values(subject, property)?
            .contains_entity(object))
    }

    /// Whether `subject --property--> value` is entailed — the OWL API's
    /// `hasDataPropertyRelationship`.
    ///
    /// Default: membership in [`OWLReasoner::get_data_property_values`].
    fn has_data_property_relationship(
        &self,
        subject: &crate::ontology::NamedIndividual,
        property: &DataPropertyExpression,
        value: &crate::ontology::Literal,
    ) -> Result<bool> {
        Ok(self
            .get_data_property_values(subject, property)?
            .contains_entity(value))
    }

    /// Every materialised `(source, target)` pair for `property`, including
    /// pairs that only follow from an inverse-labelled edge
    /// (`b --inv(R)--> a` yields `(a, b)` for `R`). Only named individuals are
    /// returned. Output is sorted by `(source, target)` and deduplicated, so it
    /// is deterministic across runs.
    ///
    /// Default: derive from [`OWLReasoner::get_object_property_values`] over
    /// every named individual in the signature.
    fn get_object_property_instances(
        &self,
        property: &ObjectPropertyExpression,
    ) -> Result<
        Vec<(
            crate::ontology::NamedIndividual,
            crate::ontology::NamedIndividual,
        )>,
    > {
        let ontology = self.get_root_ontology();
        let sources: Vec<crate::ontology::NamedIndividual> = {
            let guard = ontology.read().map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
            guard
                .signature()?
                .individuals
                .iter()
                .filter_map(|individual| match individual {
                    Individual::Named(named) => Some(named.clone()),
                    Individual::Anonymous(_) => None,
                })
                .collect()
        };

        let mut pairs: Vec<(
            crate::ontology::NamedIndividual,
            crate::ontology::NamedIndividual,
        )> = Vec::new();
        for source in sources {
            for target in self
                .get_object_property_values(&source, property)?
                .get_flattened()
            {
                pairs.push((source.clone(), target));
            }
        }
        pairs.sort_by(|a, b| {
            a.0.iri
                .to_string()
                .cmp(&b.0.iri.to_string())
                .then_with(|| a.1.iri.to_string().cmp(&b.1.iri.to_string()))
        });
        pairs.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        Ok(pairs)
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
                iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#bottomDataProperty"),
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
    ///
    /// The default is conservative (class axioms only); the tableau-backed
    /// [`TableauOWLReasoner`] overrides this to report the full workstream-A7
    /// coverage.
    fn is_entailment_checking_supported(
        &self,
        axiom_type: &crate::ontology::axioms::AxiomType,
    ) -> bool {
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
    /// Cached `SameAs` / `DifferentAs` components (lazily computed).
    cached_individual_equality: Mutex<Option<IndividualEquality>>,
    /// Inference types explicitly warmed by [`OWLReasoner::precompute_inferences`].
    ///
    /// Separate from the caches above because `is_precomputed` must report
    /// *what was asked for*, not merely "some cache happens to be warm"; the
    /// lazy caches are additionally consulted so an inference computed lazily
    /// still reports as computed.
    precomputed: Mutex<HashSet<InferenceType>>,
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
    /// Property → its full **strict** super set: the transitive closure of
    /// `SubObjectPropertyOf` / `EquivalentObjectProperties` (reachability
    /// only) / `InverseObjectProperties`, plus `owl:topObjectProperty`.
    /// Equivalents are excluded — see `core::reasoner::property_hierarchy`.
    object_supers: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    /// Property → its full **strict** sub set (plus `owl:bottomObjectProperty`).
    object_subs: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    equivalent_object_properties:
        HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    disjoint_object_properties:
        HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    inverse_object_properties: HashMap<ObjectPropertyExpression, HashSet<ObjectPropertyExpression>>,
    /// Property → its domains, including the domains **inherited from its
    /// super properties** (and equivalents).
    object_property_domains: HashMap<ObjectPropertyExpression, HashSet<ClassExpression>>,
    /// Property → its ranges, including ranges inherited from super properties.
    object_property_ranges: HashMap<ObjectPropertyExpression, HashSet<ClassExpression>>,
    /// Data property → its full **strict** super set (plus `owl:topDataProperty`).
    data_supers: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    /// Data property → its full **strict** sub set (plus `owl:bottomDataProperty`).
    data_subs: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    equivalent_data_properties: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    disjoint_data_properties: HashMap<DataPropertyExpression, HashSet<DataPropertyExpression>>,
    /// Data property → its domains, super-property inheritance included.
    data_property_domains: HashMap<DataPropertyExpression, HashSet<ClassExpression>>,
    /// Data property → its ranges, super-property inheritance included.
    data_property_ranges: HashMap<DataPropertyExpression, HashSet<DataRange>>,
}

impl PropertyHierarchyCache {
    /// `higher` is a strict super of `lower` in this cache's closure.
    fn object_strictly_above(
        &self,
        higher: &ObjectPropertyExpression,
        lower: &ObjectPropertyExpression,
    ) -> bool {
        self.object_supers
            .get(lower)
            .is_some_and(|up| up.contains(higher))
            && !self
                .object_supers
                .get(higher)
                .is_some_and(|down| down.contains(lower))
    }

    /// `higher` is a strict super of `lower` (data side).
    fn data_strictly_above(
        &self,
        higher: &DataPropertyExpression,
        lower: &DataPropertyExpression,
    ) -> bool {
        self.data_supers
            .get(lower)
            .is_some_and(|up| up.contains(higher))
            && !self
                .data_supers
                .get(higher)
                .is_some_and(|down| down.contains(lower))
    }

    /// OWL API `DirectSubObjectPropertyOf` filter over a transitive super set:
    /// drop `s` when a named property `t` of the set sits strictly between the
    /// queried property and `s`.
    fn direct_object_supers(
        &self,
        supers: &HashSet<ObjectPropertyExpression>,
    ) -> HashSet<ObjectPropertyExpression> {
        supers
            .iter()
            .filter(|s| {
                !supers.iter().any(|t| {
                    t != *s
                        && matches!(t, ObjectPropertyExpression::ObjectProperty(_))
                        && self.object_strictly_above(s, t)
                })
            })
            .cloned()
            .collect()
    }

    /// Mirror of [`Self::direct_object_supers`] for the sub direction.
    fn direct_object_subs(
        &self,
        subs: &HashSet<ObjectPropertyExpression>,
    ) -> HashSet<ObjectPropertyExpression> {
        subs.iter()
            .filter(|s| {
                !subs.iter().any(|t| {
                    t != *s
                        && matches!(t, ObjectPropertyExpression::ObjectProperty(_))
                        && self.object_strictly_above(t, s)
                })
            })
            .cloned()
            .collect()
    }

    /// [`Self::direct_object_supers`] for data properties (every data
    /// property expression is a name, so no `is_named` check is needed).
    fn direct_data_supers(
        &self,
        supers: &HashSet<DataPropertyExpression>,
    ) -> HashSet<DataPropertyExpression> {
        supers
            .iter()
            .filter(|s| {
                !supers
                    .iter()
                    .any(|t| t != *s && self.data_strictly_above(s, t))
            })
            .cloned()
            .collect()
    }

    /// Mirror of [`Self::direct_data_supers`].
    fn direct_data_subs(
        &self,
        subs: &HashSet<DataPropertyExpression>,
    ) -> HashSet<DataPropertyExpression> {
        subs.iter()
            .filter(|s| {
                !subs
                    .iter()
                    .any(|t| t != *s && self.data_strictly_above(t, s))
            })
            .cloned()
            .collect()
    }

    /// Group a flat member set into OWL API equivalence [`Node`]s (a `Node`
    /// contains exactly the pairwise-equivalent members of the set).
    fn group_object_nodes(
        &self,
        members: HashSet<ObjectPropertyExpression>,
    ) -> HashSet<Node<ObjectPropertyExpression>> {
        let mut remaining: Vec<ObjectPropertyExpression> = members.into_iter().collect();
        let mut nodes: HashSet<Node<ObjectPropertyExpression>> = HashSet::new();
        while let Some(first) = remaining.pop() {
            let mut component: HashSet<ObjectPropertyExpression> = HashSet::new();
            component.insert(first.clone());
            let mut i = 0;
            while i < remaining.len() {
                let candidate = &remaining[i];
                let equivalent = self
                    .equivalent_object_properties
                    .get(&first)
                    .is_some_and(|eq| eq.contains(candidate))
                    || self
                        .equivalent_object_properties
                        .get(candidate)
                        .is_some_and(|eq| eq.contains(&first));
                if equivalent {
                    component.insert(remaining.remove(i));
                } else {
                    i += 1;
                }
            }
            nodes.insert(Node::new(component));
        }
        nodes
    }

    /// [`Self::group_object_nodes`] for data properties.
    fn group_data_nodes(
        &self,
        members: HashSet<DataPropertyExpression>,
    ) -> HashSet<Node<DataPropertyExpression>> {
        let mut remaining: Vec<DataPropertyExpression> = members.into_iter().collect();
        let mut nodes: HashSet<Node<DataPropertyExpression>> = HashSet::new();
        while let Some(first) = remaining.pop() {
            let mut component: HashSet<DataPropertyExpression> = HashSet::new();
            component.insert(first.clone());
            let mut i = 0;
            while i < remaining.len() {
                let candidate = &remaining[i];
                let equivalent = self
                    .equivalent_data_properties
                    .get(&first)
                    .is_some_and(|eq| eq.contains(candidate))
                    || self
                        .equivalent_data_properties
                        .get(candidate)
                        .is_some_and(|eq| eq.contains(&first));
                if equivalent {
                    component.insert(remaining.remove(i));
                } else {
                    i += 1;
                }
            }
            nodes.insert(Node::new(component));
        }
        nodes
    }
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
            cached_individual_equality: Mutex::new(None),
            precomputed: Mutex::new(HashSet::new()),
        })
    }

    /// Group a set of instances into result [`Node`]s honouring the
    /// configured [`IndividualNodeSetPolicy`] — shared by the `owl:Thing`
    /// fast path and the realisation-backed answer of
    /// [`OWLReasoner::get_instances`].
    fn instance_nodes(&self, instances: HashSet<Individual>) -> Result<HashSet<Node<Individual>>> {
        if matches!(
            self.config.individual_node_set_policy,
            IndividualNodeSetPolicy::BySameAs
        ) {
            let equality = self.ensure_individual_equality()?;
            let mut nodes: HashSet<Node<Individual>> = HashSet::new();
            let mut covered: HashSet<Individual> = HashSet::new();
            for ind in &instances {
                if !covered.insert(ind.clone()) {
                    continue;
                }
                let node = equality.same_as(ind);
                covered.extend(node.get_entities().iter().cloned());
                nodes.insert(node);
            }
            Ok(nodes)
        } else {
            Ok(instances.into_iter().map(Node::singleton).collect())
        }
    }

    /// Ensure the `SameAs` / `DifferentAs` components have been computed, populating
    /// the cache.
    fn ensure_individual_equality(&self) -> Result<IndividualEquality> {
        let mut cache =
            self.cached_individual_equality
                .lock()
                .map_err(|e| crate::Error::Internal {
                    message: format!("Lock poisoned: {e}"),
                })?;
        if let Some(eq) = cache.as_ref() {
            return Ok(eq.clone());
        }
        let guard = self.ontology.read().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let equality = IndividualEquality::from_ontology(&guard);
        drop(guard);
        *cache = Some(equality.clone());
        Ok(equality)
    }

    /// Warm the one-shot realisation snapshot (`TableauSnapshot`) that the
    /// property-value, same-as and instance queries merge their answers from.
    ///
    /// Uses the core reasoner's own ontology ref so the snapshot cache stays
    /// single-source (see `Reasoner::tableau_individual_components`). Returns
    /// `Err` when the tableau ends `Unknown` — an interrupted run proves
    /// nothing, so the caller must not report the inference as precomputed.
    fn warm_realization_snapshot(&self) -> Result<()> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let Some(ontology) = reasoner.get_ontology().cloned() else {
            return Ok(());
        };
        reasoner.realization_snapshot(&ontology)?;
        Ok(())
    }

    /// Compute the transitive closure of the subclass relation for `class` from
    /// the classification cache, excluding equivalent classes.
    fn collect_strict_subclasses(
        cache: &ClassificationCache,
        class: &ClassExpression,
    ) -> Vec<ClassExpression> {
        let equivalents = cache.equivalent_map.get(class).cloned().unwrap_or_default();
        let mut result: Vec<ClassExpression> = Vec::new();
        let mut seen: HashSet<ClassExpression> = HashSet::new();
        let mut stack: Vec<ClassExpression> = cache
            .super_class_map
            .get(class)
            .into_iter()
            .flat_map(|s| s.iter().cloned())
            .collect();
        while let Some(c) = stack.pop() {
            if !seen.insert(c.clone()) {
                continue;
            }
            if c == *class || equivalents.contains(&c) {
                continue;
            }
            result.push(c.clone());
            if let Some(subs) = cache.super_class_map.get(&c) {
                stack.extend(subs.iter().cloned());
            }
        }
        result
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
            for sup in supers {
                scm.entry(sub.clone()).or_default().insert(sup.clone());
                supcm.entry(sup.clone()).or_default().insert(sub.clone());
            }
        }

        // Determine unsatisfiability via satisfiability checks rather than the
        // (unsound) "no superclasses ⇒ unsatisfiable" heuristic: a class with no
        // discovered subsumers is satisfiable unless the tableau proves otherwise.
        let hierarchy_keys: Vec<ClassExpression> =
            classification.hierarchy.keys().cloned().collect();
        for sub in hierarchy_keys {
            if matches!(&sub, ClassExpression::Class(c) if c.is_nothing())
                || !reasoner.is_class_satisfiable(&sub)?
            {
                unsat.insert(sub.clone());
            }
        }

        // Build equivalence map: classes that are mutual sub/super
        for (sub, supers) in &classification.hierarchy {
            for sup in supers {
                if let Some(sub_supers) = classification.hierarchy.get(sup)
                    && sub_supers.contains(sub)
                {
                    eqm.entry(sub.clone()).or_default().insert(sup.clone());
                    eqm.entry(sup.clone()).or_default().insert(sub.clone());
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
        let mut cache =
            self.cached_property_hierarchy
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

        // Index the non-hierarchy property axioms (disjointness, declared
        // inverses, domains/ranges). The sub/super/equivalence maps are
        // filled from the shared closure below.
        for axiom in guard.axioms() {
            match axiom {
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

        // ── Inferred hierarchies (OWL API / HermiT parity) ─────────────────
        // The shared closure computes the transitive strict super/sub sets,
        // the equivalence classes, the inverse-expression nodes and the
        // `top`/`bottom` edges in one place
        // (`core::reasoner::property_hierarchy`).
        let object_hierarchy =
            crate::core::reasoner::property_hierarchy::ObjectPropertyHierarchy::compute(&guard);
        for node in object_hierarchy.nodes() {
            phc.object_supers
                .insert(node.clone(), object_hierarchy.supers(node));
            phc.object_subs
                .insert(node.clone(), object_hierarchy.subs(node));
            phc.equivalent_object_properties
                .insert(node.clone(), object_hierarchy.equivalents(node));
        }

        // Domain/range inheritance: a property inherits the domains/ranges
        // asserted for its super properties *and* equivalents — the OWL API
        // `getObjectPropertyDomains(pe, direct = false)` semantics.
        let asserted_domains = std::mem::take(&mut phc.object_property_domains);
        for (property, domains) in &asserted_domains {
            for node in object_hierarchy.nodes() {
                if node == property
                    || object_hierarchy.supers(node).contains(property)
                    || object_hierarchy.equivalents(node).contains(property)
                {
                    phc.object_property_domains
                        .entry(node.clone())
                        .or_default()
                        .extend(domains.iter().cloned());
                }
            }
        }
        let asserted_ranges = std::mem::take(&mut phc.object_property_ranges);
        for (property, ranges) in &asserted_ranges {
            for node in object_hierarchy.nodes() {
                if node == property
                    || object_hierarchy.supers(node).contains(property)
                    || object_hierarchy.equivalents(node).contains(property)
                {
                    phc.object_property_ranges
                        .entry(node.clone())
                        .or_default()
                        .extend(ranges.iter().cloned());
                }
            }
        }

        let data_hierarchy =
            crate::core::reasoner::property_hierarchy::DataPropertyHierarchy::compute(&guard);
        for node in data_hierarchy.nodes() {
            phc.data_supers
                .insert(node.clone(), data_hierarchy.supers(node));
            phc.data_subs
                .insert(node.clone(), data_hierarchy.subs(node));
            phc.equivalent_data_properties
                .insert(node.clone(), data_hierarchy.equivalents(node));
        }

        let asserted_data_domains = std::mem::take(&mut phc.data_property_domains);
        for (property, domains) in &asserted_data_domains {
            for node in data_hierarchy.nodes() {
                if node == property
                    || data_hierarchy.supers(node).contains(property)
                    || data_hierarchy.equivalents(node).contains(property)
                {
                    phc.data_property_domains
                        .entry(node.clone())
                        .or_default()
                        .extend(domains.iter().cloned());
                }
            }
        }
        let asserted_data_ranges = std::mem::take(&mut phc.data_property_ranges);
        for (property, ranges) in &asserted_data_ranges {
            for node in data_hierarchy.nodes() {
                if node == property
                    || data_hierarchy.supers(node).contains(property)
                    || data_hierarchy.equivalents(node).contains(property)
                {
                    phc.data_property_ranges
                        .entry(node.clone())
                        .or_default()
                        .extend(ranges.iter().cloned());
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

        // Collect candidate named classes from the ontology signature.
        let guard = self.ontology.read().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let signature = guard.signature()?;
        drop(guard);
        let candidates: Vec<ClassExpression> = signature
            .classes
            .iter()
            .map(|c| ClassExpression::Class(c.clone()))
            .filter(|c| !is_top_or_bottom_class(c))
            .collect();

        let unsat = cache.unsatisfiable.clone();
        let equivalents = cache.equivalent_map.get(class).cloned().unwrap_or_default();

        // `owl:Thing` case: `D` is disjoint with `⊤` iff `D` is unsatisfiable
        // (`⊤ ⊓ D ≡ D ⊑ ⊥`). Return the bottom-equivalent classes.
        if matches!(class, ClassExpression::Class(c) if c.is_thing()) {
            let nodes: HashSet<Node<ClassExpression>> =
                unsat.into_iter().map(Node::singleton).collect();
            return Ok(NodeSet::new(nodes));
        }

        // Unsatisfiable `class` case: `¬class ≡ ⊤`, so every satisfiable named
        // class is entailed disjoint (`D ⊑ ¬class`). Following OWLAPI semantics
        // we return the named classes in the signature (excluding top/bottom
        // markers and other bottom-equivalent classes).
        if unsat.contains(class) || matches!(class, ClassExpression::Class(c) if c.is_nothing()) {
            let nodes: HashSet<Node<ClassExpression>> = candidates
                .into_iter()
                .filter(|c| !unsat.contains(c))
                .map(Node::singleton)
                .collect();
            return Ok(NodeSet::new(nodes));
        }

        // General case: `D` is disjoint with `ce` iff `ce ⊓ D` is unsatisfiable.
        let mut disjoint: Vec<ClassExpression> = Vec::new();
        for d in candidates {
            if unsat.contains(&d) || equivalents.contains(&d) {
                continue;
            }
            let intersection =
                ClassExpression::ObjectIntersectionOf(vec![class.clone(), d.clone()]);
            let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
            let sat = reasoner.is_class_satisfiable(&intersection)?;
            drop(reasoner);
            if !sat {
                disjoint.push(d);
            }
        }
        let nodes: HashSet<Node<ClassExpression>> =
            disjoint.into_iter().map(Node::singleton).collect();
        Ok(NodeSet::new(nodes))
    }

    fn get_instances(&self, class: &ClassExpression, direct: bool) -> Result<NodeSet<Individual>> {
        // ── owl:Thing / owl:Nothing fast paths (OWL API parity) ────────────
        // Every individual of the signature — named **and** anonymous,
        // including those that only occur in property assertions — is an
        // instance of `owl:Thing`; nothing is an instance of `owl:Nothing`.
        // The realisation never stores `owl:Thing` as a realised type, so
        // without this fast path both answers came back empty.
        if let ClassExpression::Class(cls) = class {
            if cls.is_nothing() {
                return Ok(NodeSet::empty());
            }
            if cls.is_thing() {
                let individuals: HashSet<Individual> = {
                    let guard = self.ontology.read().map_err(|e| crate::Error::Internal {
                        message: format!("Lock poisoned: {e}"),
                    })?;
                    guard.signature()?.individuals.into_iter().collect()
                };
                let instances: HashSet<Individual> = if direct {
                    // Direct instances of `owl:Thing` are the individuals with
                    // no more specific realised type (`DirectClassAssertion`).
                    self.ensure_classified()?;
                    let cache =
                        self.cached_hierarchy
                            .lock()
                            .map_err(|e| crate::Error::Internal {
                                message: format!("Lock poisoned: {e}"),
                            })?;
                    let cache = cache.as_ref().unwrap();
                    individuals
                        .into_iter()
                        .filter(|i| {
                            cache.type_map.get(i).is_none_or(|types| {
                                types
                                    .iter()
                                    .all(|t| matches!(t, ClassExpression::Class(c) if c.is_thing()))
                            })
                        })
                        .collect()
                } else {
                    individuals
                };
                return Ok(NodeSet::new(self.instance_nodes(instances)?));
            }
        }

        self.ensure_classified()?;
        let cache = self
            .cached_hierarchy
            .lock()
            .map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
        let cache = cache.as_ref().unwrap();

        let mut instances: HashSet<Individual> =
            cache.instance_map.get(class).cloned().unwrap_or_default();

        // ── Tableau fallback for classes the realisation never reached ──────
        // `instance_map` covers the named-class hierarchy and *defined*
        // classes; a named class that is only reachable through a complex GCI
        // (`A ⊓ B ⊑ C` with `A(a)`, `B(a)`) never enters it, so an empty
        // answer is re-checked against the completed tableau through
        // `Reasoner::is_instance_of` — the OWL API contract is the *complete*
        // instance set. Only runs when the answer would otherwise be empty,
        // so classes with a realisation answer keep their O(1) lookup.
        if instances.is_empty() {
            let individuals: Vec<Individual> = {
                let guard = self.ontology.read().map_err(|e| crate::Error::Internal {
                    message: format!("Lock poisoned: {e}"),
                })?;
                guard.signature()?.individuals
            };
            if !individuals.is_empty() {
                let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
                    message: format!("Lock poisoned: {e}"),
                })?;
                for individual in &individuals {
                    if reasoner.is_instance_of(individual, class)? {
                        instances.insert(individual.clone());
                    }
                }
            }
        }

        if direct {
            // Direct instances: individuals that are instances of `class` but not
            // of any strict subclass of `class` in the computed class hierarchy.
            let subclasses = Self::collect_strict_subclasses(cache, class);
            let mut to_remove: HashSet<Individual> = HashSet::new();
            for sub in &subclasses {
                if let Some(insts) = cache.instance_map.get(sub) {
                    to_remove.extend(insts.iter().cloned());
                }
            }
            instances.retain(|i| !to_remove.contains(i));
        }

        // Same-as expansion: `a ≡ _:x` and `a ∈ C` entail `_:x ∈ C`, but the
        // realisation map keys the two individuals separately — HermiT returns
        // the whole equivalence class from `getInstances`. Syntactic equality
        // only (the same structure `getSameIndividuals` starts from), so an
        // over-merge is impossible.
        if !instances.is_empty() {
            let equality = self.ensure_individual_equality()?;
            let mut expanded: HashSet<Individual> = HashSet::with_capacity(instances.len());
            for individual in &instances {
                for member in equality.same_as(individual).get_entities() {
                    expanded.insert(member.clone());
                }
            }
            instances = expanded;
        }

        // Group same-as individuals into a single node when the configured
        // policy requests it; otherwise each individual is its own node.
        Ok(NodeSet::new(self.instance_nodes(instances)?))
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
                // Direct types: keep a type `t` only if the individual is not an
                // instance of any *proper subclass* of `t`. Equivalently, remove
                // `t` when another type `other` in the set is a strict subclass
                // of `t` (in the transitive subclass closure).
                let to_remove: HashSet<_> = all_types
                    .iter()
                    .filter(|t| {
                        all_types.iter().any(|other| {
                            other != *t && Self::collect_strict_subclasses(cache, t).contains(other)
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

        // ── owl:Thing is a type of every individual (OWL API parity) ───────
        // For `direct = false` it is always present; for `direct = true` it
        // only survives when the individual has no more specific type (a
        // singleton `owl:Thing` node is then the whole answer).
        let thing = ClassExpression::Class(crate::ontology::Class::thing());
        let has_thing = result.iter().any(|n| n.contains(&thing));
        if direct {
            let has_other = result
                .iter()
                .any(|n| n.get_entities().iter().any(|c| *c != thing));
            if has_other {
                // Drop the pure `owl:Thing` node; a node that also carries an
                // equivalent of `owl:Thing` keeps its other members.
                result.retain(|n| !(n.contains(&thing) && n.get_entities().len() == 1));
            } else if !has_thing {
                result.insert(Node::singleton(thing));
            }
        } else if !has_thing {
            result.insert(Node::singleton(thing));
        }

        Ok(NodeSet::new(result))
    }

    fn get_same_individuals(&self, individual: &Individual) -> Result<Node<Individual>> {
        // Syntactic SameIndividual/functional equality first (always available).
        let equality = self.ensure_individual_equality()?;
        let mut members: HashSet<Individual> = equality.same_as(individual).get_entities().clone();

        // Back the result with the completed tableau's merge map
        // (`TableauSnapshot::individual_nodes`) when it is available:
        // SameIndividual, functional/inverse-functional and HasKey equalities
        // materialised by the tableau collapse two individuals onto the same
        // node. Prefer the shared one-shot snapshot of the reasoner's own
        // ontology (one tableau run for the whole query session) and fall back
        // to a fresh run over the live ontology ref — merge (not replace) so
        // individuals the tableau never saw still come from the syntactic
        // structure. An inconsistent or interrupted tableau keeps the syntactic
        // answer, as before.
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let merge_map = reasoner
            .get_ontology()
            .and_then(|own| reasoner.realization_snapshot(own).ok())
            .filter(|snapshot| snapshot.satisfiable)
            .map(|snapshot| snapshot.individual_nodes.clone())
            .or_else(|| {
                reasoner
                    .tableau_individual_components(&self.ontology)
                    .ok()
                    .flatten()
            });
        drop(reasoner);

        if let Some(map) = merge_map
            && let Some(node) = map.get(&individual.to_string()).copied()
        {
            for (name, &nid) in &map {
                if nid == node
                    && let Some(ind) = individual_from_display(name)
                {
                    members.insert(ind);
                }
            }
        }

        Ok(Node::new(members))
    }

    fn get_different_individuals(&self, individual: &Individual) -> Result<NodeSet<Individual>> {
        let equality = self.ensure_individual_equality()?;
        let mut nodes: HashSet<Node<Individual>> =
            equality.different(individual).get_nodes().clone();

        // ── Disjoint-type entailment ─────────────────────────────────────────
        // Two individuals are also different when their most-specific realised
        // types contain a pair of disjoint classes, even without an explicit
        // `DifferentIndividuals` axiom: `A ⊑ ¬B`, `A(a)`, `B(b)` ⇒ `a` ≠ `b`,
        // because `A ⊓ B` is unsatisfiable.
        //
        // Bounded on purpose — every candidate is a fresh pair of type queries
        // and every unseen type pair costs up to three decisions (two
        // classification-backed subsumption tests and, only when both miss,
        // one tableau satisfiability run): at most
        // `MAX_DISJOINT_CANDIDATES` candidates and
        // `MAX_DISJOINT_PAIR_CHECKS` type pairs per call, with a per-call memo
        // on the type pairs. Reaching a cap only *limits recall* (the explicit
        // `DifferentIndividuals` answer is always returned unchanged); it never
        // invents a difference, so there are no false positives.
        const MAX_DISJOINT_CANDIDATES: usize = 64;
        const MAX_DISJOINT_PAIR_CHECKS: usize = 512;

        // An individual is never different from itself or from its same-as
        // component.
        let same = equality.same_as(individual).get_entities().clone();

        let mut candidates: Vec<Individual> = {
            let guard = self.ontology.read().map_err(|e| crate::Error::Internal {
                message: format!("Lock poisoned: {e}"),
            })?;
            // Named **and** anonymous individuals are candidates: the
            // disjoint-type entailment is equally sound for `_:x` (its realised
            // types come from the same realisation snapshot).
            let mut individuals: Vec<Individual> = guard.signature()?.individuals.clone();
            individuals.sort_by_key(std::string::ToString::to_string);
            individuals.truncate(MAX_DISJOINT_CANDIDATES);
            individuals
        };
        candidates.retain(|other| !same.contains(other));

        if candidates.is_empty() {
            return Ok(NodeSet::new(nodes));
        }

        // Most-specific realised types of the queried individual.
        let my_types = self.get_types(individual, true)?.get_flattened();
        // Unsatisfiable classes can never be realised types of a consistent
        // ontology's individual; treating them as "disjoint with everything"
        // would fabricate differences on an inconsistent ontology.
        let unsatisfiable = {
            self.ensure_classified()?;
            let cache = self
                .cached_hierarchy
                .lock()
                .map_err(|e| crate::Error::Internal {
                    message: format!("Lock poisoned: {e}"),
                })?;
            cache
                .as_ref()
                .map(|c| c.unsatisfiable.clone())
                .unwrap_or_default()
        };
        let viable = |types: HashSet<ClassExpression>| -> HashSet<ClassExpression> {
            types
                .into_iter()
                .filter(|t| {
                    !unsatisfiable.contains(t)
                        && !matches!(t, ClassExpression::Class(c) if c.is_nothing())
                })
                .collect()
        };
        let my_types = viable(my_types);

        if !my_types.is_empty() {
            let mut memo: HashMap<(ClassExpression, ClassExpression), bool> = HashMap::new();
            let mut checks = 0usize;
            'candidates: for other in &candidates {
                let its_types = viable(self.get_types(other, true)?.get_flattened());
                for mine in &my_types {
                    for theirs in &its_types {
                        if mine == theirs {
                            continue;
                        }
                        if checks >= MAX_DISJOINT_PAIR_CHECKS {
                            break 'candidates;
                        }
                        checks += 1;
                        // Memo key is order-insensitive: `A ⊓ B` and `B ⊓ A`
                        // are the same satisfiability question.
                        let key = if format!("{mine:?}") <= format!("{theirs:?}") {
                            (mine.clone(), theirs.clone())
                        } else {
                            (theirs.clone(), mine.clone())
                        };
                        let disjoint = if let Some(answer) = memo.get(&key) {
                            *answer
                        } else {
                            let reasoner =
                                self.reasoner.lock().map_err(|e| crate::Error::Internal {
                                    message: format!("Lock poisoned: {e}"),
                                })?;
                            let complement = |c: &ClassExpression| {
                                ClassExpression::ObjectComplementOf(Box::new(c.clone()))
                            };
                            // Two complementary decision procedures, ordered
                            // cheapest-first, because neither covers every
                            // shape on its own:
                            //  * the classification/index closure
                            //    (`is_subclass_of(A, ¬B)`) answers GCIs such
                            //    as `A ⊑ ¬B`, which the consistency tableau
                            //    does not unfold into a clash;
                            //  * the tableau (`is_class_satisfiable(A ⊓ B)`)
                            //    answers `DisjointClasses`, equivalent-class
                            //    and complex-shape clashes the subsumption
                            //    closure cannot see.
                            let intersection = ClassExpression::ObjectIntersectionOf(vec![
                                mine.clone(),
                                theirs.clone(),
                            ]);
                            let answer = reasoner.is_subclass_of(mine, &complement(theirs))?
                                || reasoner.is_subclass_of(theirs, &complement(mine))?
                                || !reasoner.is_class_satisfiable(&intersection)?;
                            drop(reasoner);
                            memo.insert(key, answer);
                            answer
                        };
                        if disjoint {
                            // Group the whole same-as component, matching the
                            // explicit `DifferentIndividuals` node shape.
                            nodes.insert(equality.same_as(other));
                            continue 'candidates;
                        }
                    }
                }
            }
        }

        Ok(NodeSet::new(nodes))
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
        // `object_subs` already holds the full transitive strict sub set
        // (`direct = false`); `direct = true` applies the OWL API
        // `DirectSubObjectPropertyOf` filter on top of it.
        let Some(subs) = cache.object_subs.get(prop) else {
            return Ok(NodeSet::empty());
        };
        let members = if direct {
            cache.direct_object_subs(subs)
        } else {
            subs.clone()
        };
        Ok(NodeSet::new(cache.group_object_nodes(members)))
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
        // `object_supers` already holds the full transitive strict super set
        // (`direct = false`); `direct = true` filters to the immediate supers.
        let Some(supers) = cache.object_supers.get(prop) else {
            return Ok(NodeSet::empty());
        };
        let members = if direct {
            cache.direct_object_supers(supers)
        } else {
            supers.clone()
        };
        Ok(NodeSet::new(cache.group_object_nodes(members)))
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
            Ok(NodeSet::new(
                disj.iter().map(|d| Node::singleton(d.clone())).collect(),
            ))
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
            Ok(NodeSet::new(
                domains.iter().map(|d| Node::singleton(d.clone())).collect(),
            ))
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
            Ok(NodeSet::new(
                ranges.iter().map(|r| Node::singleton(r.clone())).collect(),
            ))
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
        // Full transitive strict sub set (`direct = false`), filtered to the
        // immediate subs for `direct = true`.
        let Some(subs) = cache.data_subs.get(prop) else {
            return Ok(NodeSet::empty());
        };
        let members = if direct {
            cache.direct_data_subs(subs)
        } else {
            subs.clone()
        };
        Ok(NodeSet::new(cache.group_data_nodes(members)))
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
        // Full transitive strict super set (`direct = false`), filtered to the
        // immediate supers for `direct = true`.
        let Some(supers) = cache.data_supers.get(prop) else {
            return Ok(NodeSet::empty());
        };
        let members = if direct {
            cache.direct_data_supers(supers)
        } else {
            supers.clone()
        };
        Ok(NodeSet::new(cache.group_data_nodes(members)))
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
            Ok(NodeSet::new(
                disj.iter().map(|d| Node::singleton(d.clone())).collect(),
            ))
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
            Ok(NodeSet::new(
                domains.iter().map(|d| Node::singleton(d.clone())).collect(),
            ))
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
            Ok(NodeSet::new(
                ranges.iter().map(|r| Node::singleton(r.clone())).collect(),
            ))
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

    // ── OWLAPI / HermiT parity helpers ───────────────────────────────────────

    fn has_type(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
        // `Reasoner::is_instance_of` is the semantically complete answer: it
        // falls back to the materialised tableau closure for memberships that
        // follow from a derived edge (`A ≡ ≥1 R.{b}` materialising `R(a,b)`),
        // which the `type_map` alone can miss.
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        reasoner.is_instance_of(individual, class)
    }

    fn is_same_individual(&self, a: &Individual, b: &Individual) -> Result<bool> {
        Ok(self.get_same_individuals(a)?.contains(b))
    }

    fn has_object_property_relationship(
        &self,
        subject: &crate::ontology::NamedIndividual,
        property: &ObjectPropertyExpression,
        object: &crate::ontology::NamedIndividual,
    ) -> Result<bool> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let values =
            reasoner.get_object_property_values(&Individual::Named(subject.clone()), property)?;
        Ok(values.iter().any(|value| value.iri() == Some(&object.iri)))
    }

    fn has_data_property_relationship(
        &self,
        subject: &crate::ontology::NamedIndividual,
        property: &DataPropertyExpression,
        value: &crate::ontology::Literal,
    ) -> Result<bool> {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let values =
            reasoner.get_data_property_values(&Individual::Named(subject.clone()), property)?;
        Ok(values.iter().any(|v| v == value))
    }

    fn get_object_property_instances(
        &self,
        property: &ObjectPropertyExpression,
    ) -> Result<
        Vec<(
            crate::ontology::NamedIndividual,
            crate::ontology::NamedIndividual,
        )>,
    > {
        let reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        let pairs = reasoner.get_object_property_instances(property)?;
        // Core returns `Individual` pairs; the OWLAPI surface is named-only
        // (the core method already skips anonymous endpoints).
        Ok(pairs
            .into_iter()
            .filter_map(|(source, target)| match (source, target) {
                (Individual::Named(s), Individual::Named(t)) => Some((s, t)),
                _ => None,
            })
            .collect())
    }

    fn is_entailed(&self, axiom: &Axiom) -> Result<bool> {
        let mut reasoner = self.reasoner.lock().map_err(|e| crate::Error::Internal {
            message: format!("Lock poisoned: {e}"),
        })?;
        // Route SubClassOf/ClassAssertion through the typed reasoner methods
        // (is_subclass_of / is_instance_of), which operate directly on
        // ClassExpression values rather than the general check_entailment path.
        // Every other logical axiom type is answered semantically by
        // `check_entailment` (workstream A7): property-characteristic and
        // assertion axioms via the completed-tableau closure / consistency
        // reductions, data-property hierarchy axioms via the asserted hierarchy
        // closure.
        match axiom {
            Axiom::SubClassOf(a) => reasoner.is_subclass_of(&a.subclass, &a.superclass),
            Axiom::ClassAssertion(a) => reasoner.is_instance_of(&a.individual, &a.class),
            _ => {
                let mut stats = crate::core::reasoner::ReasoningStatistics::default();
                reasoner.check_entailment(axiom, &self.ontology, &mut stats)
            }
        }
    }

    fn is_entailment_checking_supported(
        &self,
        axiom_type: &crate::ontology::axioms::AxiomType,
    ) -> bool {
        use crate::ontology::axioms::AxiomType;
        // Workstream A7: every logical axiom type is answered semantically by
        // `ReasoningTaskService::check_entailment_semantic`. The non-logical
        // types listed below are answered **presence-only**: `is_entailed`
        // returns true iff an axiom with the same content (ignoring the
        // parser-minted id) is asserted — `Axiom::content_key` — which is the
        // only sound answer for declarations, annotation assertions and SWRL
        // rules (their consequences are already materialised by the SWRL
        // forward-chainer, so an equal rule being present *is* the entailment
        // question). `HasKey` / `DatatypeDefinition` are also answered by
        // presence in `is_entailed` but remain unadvertised here.
        matches!(
            axiom_type,
            AxiomType::Declaration
                | AxiomType::AnnotationAssertion
                | AxiomType::Rule
                | AxiomType::SubClassOf
                | AxiomType::EquivalentClasses
                | AxiomType::DisjointClasses
                | AxiomType::DisjointUnion
                | AxiomType::SubObjectPropertyOf
                | AxiomType::EquivalentObjectProperties
                | AxiomType::DisjointObjectProperties
                | AxiomType::InverseObjectProperties
                | AxiomType::ObjectPropertyDomain
                | AxiomType::ObjectPropertyRange
                | AxiomType::FunctionalObjectProperty
                | AxiomType::InverseFunctionalObjectProperty
                | AxiomType::ReflexiveObjectProperty
                | AxiomType::IrreflexiveObjectProperty
                | AxiomType::SymmetricObjectProperty
                | AxiomType::AsymmetricObjectProperty
                | AxiomType::TransitiveObjectProperty
                | AxiomType::SubDataPropertyOf
                | AxiomType::EquivalentDataProperties
                | AxiomType::DisjointDataProperties
                | AxiomType::DataPropertyDomain
                | AxiomType::DataPropertyRange
                | AxiomType::FunctionalDataProperty
                | AxiomType::SameIndividual
                | AxiomType::DifferentIndividuals
                | AxiomType::ClassAssertion
                | AxiomType::ObjectPropertyAssertion
                | AxiomType::DataPropertyAssertion
                | AxiomType::NegativeObjectPropertyAssertion
                | AxiomType::NegativeDataPropertyAssertion
        )
    }

    fn precompute_inferences(&self, inference_types: &[InferenceType]) -> Result<()> {
        for inference_type in inference_types {
            match inference_type {
                // Classification + realisation: the class hierarchy, the
                // type/instance maps behind `ClassAssertions`, and the
                // unsatisfiable set `DisjointClasses` answers from.
                InferenceType::ClassHierarchy
                | InferenceType::ClassAssertions
                | InferenceType::DisjointClasses => self.ensure_classified()?,
                // Sub/equivalent property closures — the object- and data-side
                // maps are built together by one pass over the property axioms.
                InferenceType::ObjectPropertyHierarchy | InferenceType::DataPropertyHierarchy => {
                    self.ensure_property_hierarchy()?;
                }
                // Asserted assertions are scanned on demand (cheap); what needs
                // warming is the materialised tableau closure the query merges
                // in afterwards.
                InferenceType::PropertyAssertions => self.warm_realization_snapshot()?,
                // Syntactic same/different components plus the tableau merge
                // map (`TableauSnapshot::individual_nodes`) behind them.
                InferenceType::SameIndividual | InferenceType::DifferentIndividuals => {
                    self.ensure_individual_equality()?;
                    self.warm_realization_snapshot()?;
                }
            }
            // Marked only *after* the computation above succeeded, so a failed
            // precomputation never reports as precomputed.
            let mut precomputed = self
                .precomputed
                .lock()
                .map_err(|e| crate::Error::Internal {
                    message: format!("Lock poisoned: {e}"),
                })?;
            precomputed.insert(*inference_type);
        }
        Ok(())
    }

    fn is_precomputed(&self, inference_type: InferenceType) -> bool {
        // Explicitly warmed by `precompute_inferences`.
        if self
            .precomputed
            .lock()
            .is_ok_and(|set| set.contains(&inference_type))
        {
            return true;
        }
        // A lazily computed inference is computed too — its answer sits in the
        // corresponding cache. `DisjointClasses` and `ClassAssertions` are both
        // answered from the classification/realisation cache, and
        // `PropertyAssertions` from the one-shot tableau snapshot.
        match inference_type {
            InferenceType::ClassHierarchy
            | InferenceType::ClassAssertions
            | InferenceType::DisjointClasses => {
                self.cached_hierarchy.lock().is_ok_and(|c| c.is_some())
            }
            InferenceType::ObjectPropertyHierarchy | InferenceType::DataPropertyHierarchy => self
                .cached_property_hierarchy
                .lock()
                .is_ok_and(|c| c.is_some()),
            InferenceType::SameIndividual | InferenceType::DifferentIndividuals => self
                .cached_individual_equality
                .lock()
                .is_ok_and(|c| c.is_some()),
            InferenceType::PropertyAssertions => self
                .reasoner
                .lock()
                .is_ok_and(|reasoner| reasoner.has_realization_snapshot()),
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

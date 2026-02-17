//! Change Tracking System
//!
//! This module implements comprehensive change tracking for ontology modifications,
//! dependency analysis, and efficient invalidation strategies.

use crate::{
    error::Result,
    ontology::{
        DataProperty, DataPropertyExpression, ObjectProperty, ObjectPropertyExpression, Ontology,
        axioms::Axiom,
        concepts::{Class, ClassExpression},
        individuals::Individual,
    },
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::RwLock,
    time::Instant,
};

/// Represents a change to the `TBox` (terminological knowledge)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TBoxChange {
    /// A new axiom was added to the ontology
    AxiomAdded { axiom: Axiom, timestamp: Instant },
    /// An existing axiom was removed from the ontology
    AxiomRemoved { axiom: Axiom, timestamp: Instant },
    /// A new class was introduced
    ClassAdded { class: Class, timestamp: Instant },
    /// A class was removed
    ClassRemoved { class: Class, timestamp: Instant },
    /// An object property was added
    ObjectPropertyAdded {
        property: ObjectProperty,
        timestamp: Instant,
    },
    /// An object property was removed
    ObjectPropertyRemoved {
        property: ObjectProperty,
        timestamp: Instant,
    },
    /// A data property was added
    DataPropertyAdded {
        property: DataProperty,
        timestamp: Instant,
    },
    /// A data property was removed
    DataPropertyRemoved {
        property: DataProperty,
        timestamp: Instant,
    },
}

impl TBoxChange {
    /// Get the timestamp when this change occurred
    #[must_use] 
    pub fn timestamp(&self) -> Instant {
        match self {
            TBoxChange::AxiomAdded { timestamp, .. } => *timestamp,
            TBoxChange::AxiomRemoved { timestamp, .. } => *timestamp,
            TBoxChange::ClassAdded { timestamp, .. } => *timestamp,
            TBoxChange::ClassRemoved { timestamp, .. } => *timestamp,
            TBoxChange::ObjectPropertyAdded { timestamp, .. } => *timestamp,
            TBoxChange::ObjectPropertyRemoved { timestamp, .. } => *timestamp,
            TBoxChange::DataPropertyAdded { timestamp, .. } => *timestamp,
            TBoxChange::DataPropertyRemoved { timestamp, .. } => *timestamp,
        }
    }

    /// Get a human-readable description of the change
    #[must_use] 
    pub fn description(&self) -> String {
        match self {
            TBoxChange::AxiomAdded { axiom, .. } => format!("Added axiom: {axiom:?}"),
            TBoxChange::AxiomRemoved { axiom, .. } => format!("Removed axiom: {axiom:?}"),
            TBoxChange::ClassAdded { class, .. } => format!("Added class: {}", class.iri),
            TBoxChange::ClassRemoved { class, .. } => format!("Removed class: {}", class.iri),
            TBoxChange::ObjectPropertyAdded { property, .. } => {
                format!("Added object property: {}", property.iri)
            }
            TBoxChange::ObjectPropertyRemoved { property, .. } => {
                format!("Removed object property: {}", property.iri)
            }
            TBoxChange::DataPropertyAdded { property, .. } => {
                format!("Added data property: {}", property.iri)
            }
            TBoxChange::DataPropertyRemoved { property, .. } => {
                format!("Removed data property: {}", property.iri)
            }
        }
    }

    /// Extract the classes that are directly affected by this change
    #[must_use] 
    pub fn affected_classes(&self) -> HashSet<Class> {
        let mut classes = HashSet::new();

        match self {
            TBoxChange::AxiomAdded { axiom, .. } | TBoxChange::AxiomRemoved { axiom, .. } => {
                classes.extend(extract_classes_from_axiom(axiom));
            }
            TBoxChange::ClassAdded { class, .. } | TBoxChange::ClassRemoved { class, .. } => {
                classes.insert(class.clone());
            }
            _ => {} // Properties don't directly affect classes in this simple analysis
        }

        classes
    }
}

/// Represents a change to the `ABox` (assertional knowledge)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ABoxChange {
    /// A new individual was introduced
    IndividualAdded {
        individual: Individual,
        timestamp: Instant,
    },
    /// An individual was removed
    IndividualRemoved {
        individual: Individual,
        timestamp: Instant,
    },
    /// A class assertion was added (individual is instance of class)
    ClassAssertionAdded {
        individual: Individual,
        class: ClassExpression,
        timestamp: Instant,
    },
    /// A class assertion was removed
    ClassAssertionRemoved {
        individual: Individual,
        class: ClassExpression,
        timestamp: Instant,
    },
    /// An object property assertion was added
    ObjectPropertyAssertionAdded {
        subject: Individual,
        property: ObjectPropertyExpression,
        object: Individual,
        timestamp: Instant,
    },
    /// An object property assertion was removed
    ObjectPropertyAssertionRemoved {
        subject: Individual,
        property: ObjectPropertyExpression,
        object: Individual,
        timestamp: Instant,
    },
    /// A data property assertion was added
    DataPropertyAssertionAdded {
        subject: Individual,
        property: DataPropertyExpression,
        value: String, // Simplified - could be more complex literal
        timestamp: Instant,
    },
    /// A data property assertion was removed
    DataPropertyAssertionRemoved {
        subject: Individual,
        property: DataPropertyExpression,
        value: String,
        timestamp: Instant,
    },
}

impl ABoxChange {
    /// Get the timestamp when this change occurred
    #[must_use] 
    pub fn timestamp(&self) -> Instant {
        match self {
            ABoxChange::IndividualAdded { timestamp, .. } => *timestamp,
            ABoxChange::IndividualRemoved { timestamp, .. } => *timestamp,
            ABoxChange::ClassAssertionAdded { timestamp, .. } => *timestamp,
            ABoxChange::ClassAssertionRemoved { timestamp, .. } => *timestamp,
            ABoxChange::ObjectPropertyAssertionAdded { timestamp, .. } => *timestamp,
            ABoxChange::ObjectPropertyAssertionRemoved { timestamp, .. } => *timestamp,
            ABoxChange::DataPropertyAssertionAdded { timestamp, .. } => *timestamp,
            ABoxChange::DataPropertyAssertionRemoved { timestamp, .. } => *timestamp,
        }
    }

    /// Get a human-readable description of the change
    #[must_use] 
    pub fn description(&self) -> String {
        match self {
            ABoxChange::IndividualAdded { individual, .. } => {
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!("Added individual: {iri}")
            }
            ABoxChange::IndividualRemoved { individual, .. } => {
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!("Removed individual: {iri}")
            }
            ABoxChange::ClassAssertionAdded {
                individual, class, ..
            } => {
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!("Added assertion: {iri} is instance of {class:?}")
            }
            ABoxChange::ClassAssertionRemoved {
                individual, class, ..
            } => {
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!("Removed assertion: {iri} is instance of {class:?}")
            }
            ABoxChange::ObjectPropertyAssertionAdded {
                subject,
                property,
                object,
                ..
            } => {
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                let object_iri = match object {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!(
                    "Added property assertion: {subject_iri} {property:?} {object_iri}"
                )
            }
            ABoxChange::ObjectPropertyAssertionRemoved {
                subject,
                property,
                object,
                ..
            } => {
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                let object_iri = match object {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!(
                    "Removed property assertion: {subject_iri} {property:?} {object_iri}"
                )
            }
            ABoxChange::DataPropertyAssertionAdded {
                subject,
                property,
                value,
                ..
            } => {
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!(
                    "Added data assertion: {subject_iri} {property:?} {value}"
                )
            }
            ABoxChange::DataPropertyAssertionRemoved {
                subject,
                property,
                value,
                ..
            } => {
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                format!(
                    "Removed data assertion: {subject_iri} {property:?} {value}"
                )
            }
        }
    }

    /// Get the individuals directly affected by this change
    #[must_use] 
    pub fn affected_individuals(&self) -> HashSet<Individual> {
        let mut individuals = HashSet::new();

        match self {
            ABoxChange::IndividualAdded { individual, .. }
            | ABoxChange::IndividualRemoved { individual, .. }
            | ABoxChange::ClassAssertionAdded { individual, .. }
            | ABoxChange::ClassAssertionRemoved { individual, .. }
            | ABoxChange::DataPropertyAssertionAdded {
                subject: individual,
                ..
            }
            | ABoxChange::DataPropertyAssertionRemoved {
                subject: individual,
                ..
            } => {
                individuals.insert(individual.clone());
            }
            ABoxChange::ObjectPropertyAssertionAdded {
                subject, object, ..
            }
            | ABoxChange::ObjectPropertyAssertionRemoved {
                subject, object, ..
            } => {
                individuals.insert(subject.clone());
                individuals.insert(object.clone());
            }
        }

        individuals
    }
}

/// Dependency graph for tracking relationships between ontology entities
#[derive(Debug)]
pub struct DependencyGraph {
    /// Class dependencies (subclass relationships, etc.)
    class_dependencies: HashMap<Class, HashSet<Class>>,
    /// Property dependencies
    property_dependencies: HashMap<String, HashSet<String>>,
    /// Individual to class dependencies
    individual_class_dependencies: HashMap<Individual, HashSet<Class>>,
    /// Cache of transitive closures for performance
    transitive_cache: RwLock<HashMap<String, HashSet<String>>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph
    #[must_use] 
    pub fn new() -> Self {
        Self {
            class_dependencies: HashMap::new(),
            property_dependencies: HashMap::new(),
            individual_class_dependencies: HashMap::new(),
            transitive_cache: RwLock::new(HashMap::new()),
        }
    }

    /// Build dependency graph from an ontology
    pub fn from_ontology(ontology: &Ontology) -> Result<Self> {
        let mut graph = Self::new();

        // Analyze axioms to build dependencies
        for axiom in &ontology.axioms {
            graph.analyze_axiom_dependencies(axiom)?;
        }

        Ok(graph)
    }

    /// Add a class dependency (e.g., `SubClass` relationship)
    pub fn add_class_dependency(&mut self, dependent: Class, depends_on: Class) {
        self.class_dependencies
            .entry(dependent)
            .or_default()
            .insert(depends_on);
        self.invalidate_transitive_cache();
    }

    /// Get all classes that transitively depend on the given class
    pub fn get_dependent_classes(&self, class: &Class) -> HashSet<Class> {
        let mut dependents = HashSet::new();
        self.collect_class_dependents(class, &mut dependents);
        dependents
    }

    /// Get all individuals that depend on the given class
    pub fn get_dependent_individuals(&self, class: &Class) -> HashSet<Individual> {
        self.individual_class_dependencies
            .iter()
            .filter_map(|(individual, classes)| {
                if classes.contains(class) {
                    Some(individual.clone())
                } else {
                    None
                }
            })
            .collect()
    }

    /// Analyze an axiom to extract dependencies
    fn analyze_axiom_dependencies(&mut self, axiom: &Axiom) -> Result<()> {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                // Extract classes from subclass and superclass expressions
                let subclasses = extract_classes_from_class_expression(&subclass_axiom.subclass);
                let superclasses =
                    extract_classes_from_class_expression(&subclass_axiom.superclass);

                // Add dependencies
                for subclass in &subclasses {
                    for superclass in &superclasses {
                        self.add_class_dependency(subclass.clone(), superclass.clone());
                    }
                }
            }
            Axiom::EquivalentClasses(equiv_axiom) => {
                let classes: Vec<HashSet<Class>> = equiv_axiom
                    .classes
                    .iter()
                    .map(extract_classes_from_class_expression)
                    .collect();

                // Add bidirectional dependencies for equivalent classes
                for i in 0..classes.len() {
                    for j in 0..classes.len() {
                        if i != j {
                            for class1 in &classes[i] {
                                for class2 in &classes[j] {
                                    self.add_class_dependency(class1.clone(), class2.clone());
                                }
                            }
                        }
                    }
                }
            }
            Axiom::DisjointClasses(_) => {
                // Disjoint classes create negative dependencies -
                // for simplicity, we're not tracking these in this implementation
            }
            _ => {
                // Other axiom types don't directly create class dependencies
                // in this simplified implementation
            }
        }

        Ok(())
    }

    /// Recursively collect all classes that depend on the given class
    fn collect_class_dependents(&self, class: &Class, visited: &mut HashSet<Class>) {
        self.collect_class_dependents_with_depth(class, visited, 0);
    }
    
    /// Maximum recursion depth to prevent stack overflow
    const MAX_DEPENDENT_DEPTH: usize = 500;
    
    fn collect_class_dependents_with_depth(&self, class: &Class, visited: &mut HashSet<Class>, depth: usize) {
        if visited.contains(class) || depth > Self::MAX_DEPENDENT_DEPTH {
            return; // Avoid cycles and stack overflow
        }
        visited.insert(class.clone());

        // Find all classes that directly depend on this class
        for (dependent, dependencies) in &self.class_dependencies {
            if dependencies.contains(class) && !visited.contains(dependent) {
                self.collect_class_dependents_with_depth(dependent, visited, depth + 1);
            }
        }
    }

    /// Invalidate transitive closure cache when dependencies change
    fn invalidate_transitive_cache(&self) {
        if let Ok(mut cache) = self.transitive_cache.write() {
            cache.clear();
        }
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for DependencyGraph {
    fn clone(&self) -> Self {
        Self {
            class_dependencies: self.class_dependencies.clone(),
            property_dependencies: self.property_dependencies.clone(),
            individual_class_dependencies: self.individual_class_dependencies.clone(),
            transitive_cache: RwLock::new(HashMap::new()), // Start with empty cache
        }
    }
}

/// Change tracking system that monitors ontology modifications
#[derive(Debug)]
pub struct ChangeTracker {
    /// History of `TBox` changes
    tbox_history: RwLock<VecDeque<TBoxChange>>,
    /// History of `ABox` changes
    abox_history: RwLock<VecDeque<ABoxChange>>,
    /// Dependency graph for impact analysis
    dependency_graph: RwLock<DependencyGraph>,
    /// Queue of invalidation events to process
    invalidation_queue: RwLock<VecDeque<InvalidationEvent>>,
    /// Configuration
    config: super::IncrementalConfig,
}

/// Event indicating that certain cached results need to be invalidated
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidationEvent {
    /// Concept satisfiability cache entries for these classes need invalidation
    ConceptSatisfiability(HashSet<Class>),
    /// Subclass relationships involving these classes need invalidation
    SubclassRelations(HashSet<Class>),
    /// Instance relationships for these individuals need invalidation
    InstanceRelations(HashSet<Individual>),
    /// Query results for queries involving these entities need invalidation
    QueryResults(HashSet<String>), // Query identifiers
    /// Full reasoning cache invalidation (last resort)
    FullInvalidation,
}

impl ChangeTracker {
    /// Create a new change tracker
    #[must_use] 
    pub fn new(config: super::IncrementalConfig) -> Self {
        Self {
            tbox_history: RwLock::new(VecDeque::new()),
            abox_history: RwLock::new(VecDeque::new()),
            dependency_graph: RwLock::new(DependencyGraph::new()),
            invalidation_queue: RwLock::new(VecDeque::new()),
            config,
        }
    }

    /// Initialize the tracker with an existing ontology
    pub fn initialize_from_ontology(&self, ontology: &Ontology) -> Result<()> {
        let graph = DependencyGraph::from_ontology(ontology)?;
        if let Ok(mut dep_graph) = self.dependency_graph.write() {
            *dep_graph = graph;
        }
        Ok(())
    }

    /// Track a `TBox` change
    pub fn track_tbox_change(&self, change: TBoxChange) -> Result<()> {
        // Add to history
        if let Ok(mut history) = self.tbox_history.write() {
            history.push_back(change.clone());

            // Limit history size
            while history.len() > self.config.max_change_history {
                history.pop_front();
            }
        }

        // Generate invalidation events
        self.generate_invalidation_events_for_tbox_change(&change)?;

        // Update dependency graph if needed
        self.update_dependency_graph_for_tbox_change(&change)?;

        Ok(())
    }

    /// Track an `ABox` change
    pub fn track_abox_change(&self, change: ABoxChange) -> Result<()> {
        // Add to history
        if let Ok(mut history) = self.abox_history.write() {
            history.push_back(change.clone());

            // Limit history size
            while history.len() > self.config.max_change_history {
                history.pop_front();
            }
        }

        // Generate invalidation events
        self.generate_invalidation_events_for_abox_change(&change)?;

        Ok(())
    }

    /// Get pending invalidation events
    pub fn get_pending_invalidations(&self) -> Vec<InvalidationEvent> {
        if let Ok(mut queue) = self.invalidation_queue.write() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    /// Get recent `TBox` changes since a given timestamp
    pub fn get_tbox_changes_since(&self, since: Instant) -> Vec<TBoxChange> {
        if let Ok(history) = self.tbox_history.read() {
            history
                .iter()
                .filter(|change| change.timestamp() >= since)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get recent `ABox` changes since a given timestamp
    pub fn get_abox_changes_since(&self, since: Instant) -> Vec<ABoxChange> {
        if let Ok(history) = self.abox_history.read() {
            history
                .iter()
                .filter(|change| change.timestamp() >= since)
                .cloned()
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Generate invalidation events for a `TBox` change
    fn generate_invalidation_events_for_tbox_change(&self, change: &TBoxChange) -> Result<()> {
        let affected_classes = change.affected_classes();

        if !affected_classes.is_empty() {
            // Get dependent classes using dependency graph
            let mut all_affected = affected_classes.clone();
            if let Ok(graph) = self.dependency_graph.read() {
                for class in &affected_classes {
                    all_affected.extend(graph.get_dependent_classes(class));
                }
            }

            // Queue invalidation events
            if let Ok(mut queue) = self.invalidation_queue.write() {
                queue.push_back(InvalidationEvent::ConceptSatisfiability(
                    all_affected.clone(),
                ));
                queue.push_back(InvalidationEvent::SubclassRelations(all_affected));
            }
        }

        Ok(())
    }

    /// Generate invalidation events for an `ABox` change
    fn generate_invalidation_events_for_abox_change(&self, change: &ABoxChange) -> Result<()> {
        let affected_individuals = change.affected_individuals();

        if !affected_individuals.is_empty()
            && let Ok(mut queue) = self.invalidation_queue.write() {
                queue.push_back(InvalidationEvent::InstanceRelations(affected_individuals));
            }

        Ok(())
    }

    /// Update dependency graph based on `TBox` change
    fn update_dependency_graph_for_tbox_change(&self, change: &TBoxChange) -> Result<()> {
        if let Ok(mut graph) = self.dependency_graph.write() {
            match change {
                TBoxChange::AxiomAdded { axiom, .. } => {
                    graph.analyze_axiom_dependencies(axiom)?;
                }
                TBoxChange::AxiomRemoved { .. } => {
                    // For simplicity, we rebuild the graph when axioms are removed
                    // A more sophisticated implementation would selectively remove dependencies
                    tracing::warn!(
                        "Axiom removal requires dependency graph rebuild for optimal performance"
                    );
                }
                _ => {
                    // Other changes don't directly affect the dependency graph structure
                }
            }
        }

        Ok(())
    }
}

/// Extract all atomic classes from a class expression
#[must_use] 
pub fn extract_classes_from_class_expression(expr: &ClassExpression) -> HashSet<Class> {
    let mut classes = HashSet::new();
    extract_classes_from_class_expression_with_depth(expr, &mut classes, 0);
    classes
}

/// Maximum recursion depth for extraction to prevent stack overflow
const MAX_CLASS_EXTRACTION_DEPTH: usize = 500;

fn extract_classes_from_class_expression_with_depth(expr: &ClassExpression, classes: &mut HashSet<Class>, depth: usize) {
    // Prevent stack overflow on deeply nested expressions
    if depth > MAX_CLASS_EXTRACTION_DEPTH {
        return;
    }

    match expr {
        ClassExpression::Class(class) => {
            classes.insert(class.clone());
        }
        ClassExpression::ObjectIntersectionOf(expressions)
        | ClassExpression::ObjectUnionOf(expressions) => {
            for sub_expr in expressions {
                extract_classes_from_class_expression_with_depth(sub_expr, classes, depth + 1);
            }
        }
        ClassExpression::ObjectComplementOf(inner) => {
            extract_classes_from_class_expression_with_depth(inner, classes, depth + 1);
        }
        ClassExpression::ObjectSomeValuesFrom { filler, .. }
        | ClassExpression::ObjectAllValuesFrom { filler, .. } => {
            extract_classes_from_class_expression_with_depth(filler, classes, depth + 1);
        }
        ClassExpression::ObjectHasValue { .. } => {
            // No classes in HasValue restrictions
        }
        ClassExpression::ObjectMinCardinality { filler, .. }
        | ClassExpression::ObjectMaxCardinality { filler, .. }
        | ClassExpression::ObjectExactCardinality { filler, .. } => {
            extract_classes_from_class_expression_with_depth(filler, classes, depth + 1);
        }
        ClassExpression::ObjectHasSelf { .. } => {
            // No classes in HasSelf restrictions
        }
        ClassExpression::ObjectOneOf(_individuals) => {
            // ObjectOneOf (nominal classes) don't contribute class dependencies directly
        }
        ClassExpression::DataSomeValuesFrom { .. }
        | ClassExpression::DataAllValuesFrom { .. }
        | ClassExpression::DataHasValue { .. }
        | ClassExpression::DataMinCardinality { .. }
        | ClassExpression::DataMaxCardinality { .. }
        | ClassExpression::DataExactCardinality { .. } => {
            // Data property restrictions don't directly involve classes
        }
    }
}

/// Extract all classes mentioned in an axiom
fn extract_classes_from_axiom(axiom: &Axiom) -> HashSet<Class> {
    let mut classes = HashSet::new();

    match axiom {
        Axiom::SubClassOf(axiom) => {
            classes.extend(extract_classes_from_class_expression(&axiom.subclass));
            classes.extend(extract_classes_from_class_expression(&axiom.superclass));
        }
        Axiom::EquivalentClasses(axiom) => {
            for class_expr in &axiom.classes {
                classes.extend(extract_classes_from_class_expression(class_expr));
            }
        }
        Axiom::DisjointClasses(axiom) => {
            for class_expr in &axiom.classes {
                classes.extend(extract_classes_from_class_expression(class_expr));
            }
        }
        Axiom::ClassAssertion(axiom) => {
            classes.extend(extract_classes_from_class_expression(&axiom.class));
        }
        // Other axiom types may not directly involve classes
        _ => {}
    }

    classes
}

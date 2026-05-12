//! Delta Computation Engine
//!
//! This module implements algorithms for computing minimal reasoning updates
//! based on ontology changes, avoiding expensive full re-reasoning operations.

#![allow(dead_code)]

use super::change_tracking::{ABoxChange, ChangeTracker, TBoxChange};
use crate::{
    core::saturation::{SaturationEngine, SaturationResult},
    error::Result,
    ontology::{
        Ontology,
        axioms::Axiom,
        concepts::{Class, ClassExpression},
        individuals::Individual,
    },
    query::advanced::conjunctive::{ConjunctiveQuery, QueryAtom},
    reasoning::ReasoningService,
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Instant,
};

/// Represents the minimal changes needed for reasoning updates
#[derive(Debug, Clone)]
pub struct ReasoningDelta {
    /// Concepts that need satisfiability re-checking
    pub concepts_to_recheck: HashSet<ClassExpression>,
    /// Class hierarchy relationships to re-compute
    pub hierarchy_updates: HashSet<(Class, Class)>,
    /// Individual classifications to re-evaluate  
    pub individual_updates: HashSet<Individual>,
    /// Cached results that should be invalidated
    pub cache_invalidations: HashSet<String>,
    /// Estimated cost of applying this delta (for optimization)
    pub estimated_cost: f64,
    /// Whether a full reasoning pass is recommended instead
    pub recommend_full_reasoning: bool,
    /// Concepts affected by changes that need re-saturation
    pub saturation_updates: HashSet<ClassExpression>,
    /// Whether to use incremental saturation instead of full re-saturation
    pub use_incremental_saturation: bool,
}

impl ReasoningDelta {
    /// Create an empty reasoning delta
    #[must_use]
    pub fn new() -> Self {
        Self {
            concepts_to_recheck: HashSet::new(),
            hierarchy_updates: HashSet::new(),
            individual_updates: HashSet::new(),
            cache_invalidations: HashSet::new(),
            estimated_cost: 0.0,
            recommend_full_reasoning: false,
            saturation_updates: HashSet::new(),
            use_incremental_saturation: true,
        }
    }

    /// Check if this delta represents no changes
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.concepts_to_recheck.is_empty()
            && self.hierarchy_updates.is_empty()
            && self.saturation_updates.is_empty()
            && self.individual_updates.is_empty()
            && self.cache_invalidations.is_empty()
    }

    /// Merge another delta into this one
    pub fn merge(&mut self, other: ReasoningDelta) {
        self.concepts_to_recheck.extend(other.concepts_to_recheck);
        self.hierarchy_updates.extend(other.hierarchy_updates);
        self.individual_updates.extend(other.individual_updates);
        self.cache_invalidations.extend(other.cache_invalidations);
        self.saturation_updates.extend(other.saturation_updates);
        self.estimated_cost += other.estimated_cost;
        self.use_incremental_saturation &= other.use_incremental_saturation;
        self.recommend_full_reasoning |= other.recommend_full_reasoning;
    }

    /// Estimate the complexity of applying this delta
    #[must_use]
    pub fn complexity_score(&self) -> usize {
        self.concepts_to_recheck.len() * 10
            + self.hierarchy_updates.len() * 5
            + self.saturation_updates.len() * 2
            + self.individual_updates.len() * 3
            + self.cache_invalidations.len()
    }
}

impl Default for ReasoningDelta {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents minimal changes needed for query result updates
#[derive(Debug, Clone)]
pub struct QueryDelta {
    /// Query atoms that need re-evaluation
    pub atoms_to_reevaluate: HashSet<QueryAtom>,
    /// Variables whose bindings may have changed
    pub affected_variables: HashSet<String>,
    /// Cached query results that should be invalidated
    pub result_invalidations: HashSet<String>,
    /// New results that can be directly added (optimization)
    pub incremental_additions: Vec<HashMap<String, String>>,
    /// Results that should be removed
    pub incremental_removals: Vec<HashMap<String, String>>,
    /// Estimated cost of re-evaluating vs full query execution
    pub estimated_cost: f64,
    /// Whether full query re-execution is recommended
    pub recommend_full_reexecution: bool,
}

impl QueryDelta {
    /// Create an empty query delta
    #[must_use]
    pub fn new() -> Self {
        Self {
            atoms_to_reevaluate: HashSet::new(),
            affected_variables: HashSet::new(),
            result_invalidations: HashSet::new(),
            incremental_additions: Vec::new(),
            incremental_removals: Vec::new(),
            estimated_cost: 0.0,
            recommend_full_reexecution: false,
        }
    }

    /// Check if this delta represents no changes
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.atoms_to_reevaluate.is_empty()
            && self.affected_variables.is_empty()
            && self.incremental_additions.is_empty()
            && self.incremental_removals.is_empty()
    }

    /// Merge another query delta into this one
    pub fn merge(&mut self, other: QueryDelta) {
        self.atoms_to_reevaluate.extend(other.atoms_to_reevaluate);
        self.affected_variables.extend(other.affected_variables);
        self.result_invalidations.extend(other.result_invalidations);
        self.incremental_additions
            .extend(other.incremental_additions);
        self.incremental_removals.extend(other.incremental_removals);
        self.estimated_cost += other.estimated_cost;
        self.recommend_full_reexecution |= other.recommend_full_reexecution;
    }
}

impl Default for QueryDelta {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for delta computation algorithms
#[derive(Debug, Clone)]
pub struct DeltaComputationConfig {
    /// Maximum cost threshold before recommending full reasoning
    pub max_incremental_cost: f64,
    /// Weight factor for different types of updates in cost calculation
    pub concept_cost_weight: f64,
    /// Weight factor for hierarchy updates  
    pub hierarchy_cost_weight: f64,
    /// Weight factor for individual updates
    pub individual_cost_weight: f64,
    /// Enable aggressive optimization strategies
    pub enable_optimizations: bool,
    /// Maximum number of changes to process in one delta computation
    pub max_changes_per_batch: usize,
    /// Enable saturation-aware delta computation
    pub enable_saturation_deltas: bool,
    /// Threshold for incremental vs full re-saturation (% of concepts affected)
    pub saturation_incremental_threshold: f64,
}

impl Default for DeltaComputationConfig {
    fn default() -> Self {
        Self {
            max_incremental_cost: 1000.0,
            concept_cost_weight: 10.0,
            hierarchy_cost_weight: 5.0,
            individual_cost_weight: 3.0,
            enable_optimizations: true,
            max_changes_per_batch: 100,
            enable_saturation_deltas: true,
            saturation_incremental_threshold: 0.3, // 30% of concepts
        }
    }
}

/// Engine for computing minimal reasoning and query update deltas
pub struct DeltaComputer {
    /// Reference to the ontology being tracked
    ontology: Arc<Ontology>,
    /// Change tracking system
    change_tracker: Arc<ChangeTracker>,
    /// Base reasoning service for cost estimation
    reasoning_service: Arc<ReasoningService>,
    /// Configuration settings
    config: DeltaComputationConfig,
    /// Performance statistics
    statistics: RwLock<DeltaComputationStatistics>,
    /// Optional saturation engine for incremental saturation
    saturation_engine: Option<Arc<SaturationEngine>>,
    /// Cached saturation result for incremental updates
    cached_saturation: RwLock<Option<SaturationResult>>,
}

/// Statistics for delta computation performance monitoring
#[derive(Debug, Default, Clone)]
pub struct DeltaComputationStatistics {
    /// Number of delta computations performed
    pub delta_computations: usize,
    /// Number of times full reasoning was recommended
    pub full_reasoning_recommendations: usize,
    /// Total time spent computing deltas (in milliseconds)
    pub computation_time_ms: u64,
    /// Average delta complexity
    pub average_delta_complexity: f64,
    /// Number of optimizations applied
    pub optimizations_applied: usize,
}

impl DeltaComputer {
    /// Create a new delta computer
    pub fn new(
        ontology: Arc<Ontology>,
        change_tracker: Arc<ChangeTracker>,
        reasoning_service: Arc<ReasoningService>,
        config: Option<DeltaComputationConfig>,
    ) -> Self {
        Self {
            ontology,
            change_tracker,
            reasoning_service,
            config: config.unwrap_or_default(),
            statistics: RwLock::new(DeltaComputationStatistics::default()),
            saturation_engine: None,
            cached_saturation: RwLock::new(None),
        }
    }

    /// Create a new delta computer with saturation engine
    pub fn with_saturation(
        ontology: Arc<Ontology>,
        change_tracker: Arc<ChangeTracker>,
        reasoning_service: Arc<ReasoningService>,
        saturation_engine: Arc<SaturationEngine>,
        config: Option<DeltaComputationConfig>,
    ) -> Self {
        Self {
            ontology,
            change_tracker,
            reasoning_service,
            config: config.unwrap_or_default(),
            statistics: RwLock::new(DeltaComputationStatistics::default()),
            saturation_engine: Some(saturation_engine),
            cached_saturation: RwLock::new(None),
        }
    }

    /// Set the cached saturation result for incremental updates
    pub fn set_cached_saturation(&self, result: SaturationResult) -> Result<()> {
        if let Ok(mut cache) = self.cached_saturation.write() {
            *cache = Some(result);
        }
        Ok(())
    }

    /// Get the cached saturation result
    pub fn get_cached_saturation(&self) -> Option<SaturationResult> {
        self.cached_saturation.read().ok()?.clone()
    }

    /// Compute reasoning delta for recent changes
    pub async fn compute_reasoning_delta_since(&self, since: Instant) -> Result<ReasoningDelta> {
        let start_time = Instant::now();

        // Get recent changes
        let tbox_changes = self.change_tracker.get_tbox_changes_since(since);
        let abox_changes = self.change_tracker.get_abox_changes_since(since);

        // Compute delta
        let delta = self
            .compute_reasoning_delta_for_changes(&tbox_changes, &abox_changes)
            .await?;

        // Update statistics
        self.update_computation_statistics(start_time, &delta)
            .await?;

        Ok(delta)
    }

    /// Compute reasoning delta for specific changes
    pub async fn compute_reasoning_delta_for_changes(
        &self,
        tbox_changes: &[TBoxChange],
        abox_changes: &[ABoxChange],
    ) -> Result<ReasoningDelta> {
        let mut delta = ReasoningDelta::new();

        // Process TBox changes
        for change in tbox_changes.iter().take(self.config.max_changes_per_batch) {
            let change_delta = self.compute_tbox_change_delta(change).await?;
            delta.merge(change_delta);
        }

        // Process ABox changes
        for change in abox_changes.iter().take(self.config.max_changes_per_batch) {
            let change_delta = self.compute_abox_change_delta(change).await?;
            delta.merge(change_delta);
        }

        // Estimate cost and determine if full reasoning is better
        delta.estimated_cost = self.estimate_delta_cost(&delta);
        delta.recommend_full_reasoning = delta.estimated_cost > self.config.max_incremental_cost;

        // Apply optimizations if enabled
        if self.config.enable_optimizations {
            self.optimize_delta(&mut delta).await?;
        }

        Ok(delta)
    }

    /// Compute query delta for a specific query given recent changes
    pub async fn compute_query_delta(
        &self,
        query: &ConjunctiveQuery,
        since: Instant,
    ) -> Result<QueryDelta> {
        let _start_time = Instant::now();

        // Get recent changes
        let tbox_changes = self.change_tracker.get_tbox_changes_since(since);
        let abox_changes = self.change_tracker.get_abox_changes_since(since);

        // Analyze which query atoms are affected by the changes
        let mut delta = QueryDelta::new();

        for atom in &query.body_atoms {
            if self
                .is_atom_affected_by_changes(atom, &tbox_changes, &abox_changes)
                .await?
            {
                delta.atoms_to_reevaluate.insert(atom.clone());

                // Add affected variables
                match atom {
                    QueryAtom::ClassAtom { variable, .. } => {
                        delta.affected_variables.insert(variable.name.clone());
                    }
                    QueryAtom::ObjectPropertyAtom {
                        subject, object, ..
                    } => {
                        delta.affected_variables.insert(subject.name.clone());
                        delta.affected_variables.insert(object.name.clone());
                    }
                    QueryAtom::DataPropertyAtom {
                        subject, literal, ..
                    } => {
                        delta.affected_variables.insert(subject.name.clone());
                        delta.affected_variables.insert(literal.name.clone());
                    }
                    QueryAtom::SameIndividualAtom { left, right } => {
                        delta.affected_variables.insert(left.name.clone());
                        delta.affected_variables.insert(right.name.clone());
                    }
                    QueryAtom::DifferentIndividualsAtom { left, right } => {
                        delta.affected_variables.insert(left.name.clone());
                        delta.affected_variables.insert(right.name.clone());
                    }
                    QueryAtom::ConcreteIndividualAtom { variable, .. } => {
                        delta.affected_variables.insert(variable.name.clone());
                    }
                    QueryAtom::ConcreteLiteralAtom { variable, .. } => {
                        delta.affected_variables.insert(variable.name.clone());
                    }
                }
            }
        }

        // Estimate cost
        delta.estimated_cost = self.estimate_query_delta_cost(&delta, query);
        delta.recommend_full_reexecution =
            delta.estimated_cost > (self.config.max_incremental_cost * 0.5);

        Ok(delta)
    }

    /// Get computation statistics
    pub async fn get_statistics(&self) -> DeltaComputationStatistics {
        if let Ok(stats) = self.statistics.read() {
            stats.clone()
        } else {
            DeltaComputationStatistics::default()
        }
    }

    /// Compute delta for a single `TBox` change
    async fn compute_tbox_change_delta(&self, change: &TBoxChange) -> Result<ReasoningDelta> {
        let mut delta = ReasoningDelta::new();

        match change {
            TBoxChange::AxiomAdded { axiom, .. } => {
                delta = self.compute_axiom_addition_delta(axiom).await?;
            }
            TBoxChange::AxiomRemoved { axiom, .. } => {
                delta = self.compute_axiom_removal_delta(axiom).await?;
            }
            TBoxChange::ClassAdded { class, .. } => {
                // New class affects hierarchy reasoning
                delta
                    .concepts_to_recheck
                    .insert(ClassExpression::Class(class.clone()));
                delta
                    .cache_invalidations
                    .insert(format!("concept_sat_{}", class.iri));
            }
            TBoxChange::ClassRemoved { class, .. } => {
                // Class removal affects all dependent reasoning
                delta
                    .concepts_to_recheck
                    .insert(ClassExpression::Class(class.clone()));
                delta
                    .cache_invalidations
                    .insert(format!("concept_sat_{}", class.iri));
                delta.recommend_full_reasoning = true; // Conservative approach
            }
            TBoxChange::ObjectPropertyAdded { property, .. } => {
                // Property addition may affect existential/universal restrictions
                delta
                    .cache_invalidations
                    .insert(format!("property_{}", property.iri));
            }
            TBoxChange::ObjectPropertyRemoved { property, .. } => {
                // Property removal requires careful handling
                delta
                    .cache_invalidations
                    .insert(format!("property_{}", property.iri));
                delta.recommend_full_reasoning = true; // Conservative approach
            }
            TBoxChange::DataPropertyAdded { property, .. } => {
                delta
                    .cache_invalidations
                    .insert(format!("data_property_{}", property.iri));
            }
            TBoxChange::DataPropertyRemoved { property, .. } => {
                delta
                    .cache_invalidations
                    .insert(format!("data_property_{}", property.iri));
                delta.recommend_full_reasoning = true; // Conservative approach
            }
        }

        Ok(delta)
    }

    /// Compute delta for a single `ABox` change
    async fn compute_abox_change_delta(&self, change: &ABoxChange) -> Result<ReasoningDelta> {
        let mut delta = ReasoningDelta::new();

        match change {
            ABoxChange::IndividualAdded { individual, .. } => {
                delta.individual_updates.insert(individual.clone());
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("individual_{iri}"));
            }
            ABoxChange::IndividualRemoved { individual, .. } => {
                delta.individual_updates.insert(individual.clone());
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("individual_{iri}"));
            }
            ABoxChange::ClassAssertionAdded {
                individual, class, ..
            } => {
                delta.individual_updates.insert(individual.clone());
                // Invalidate concept satisfiability for the class
                delta.concepts_to_recheck.insert(class.clone());
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("individual_class_{iri}_{class:?}"));
            }
            ABoxChange::ClassAssertionRemoved {
                individual, class, ..
            } => {
                delta.individual_updates.insert(individual.clone());
                delta.concepts_to_recheck.insert(class.clone());
                let iri = match individual {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("individual_class_{iri}_{class:?}"));
            }
            ABoxChange::ObjectPropertyAssertionAdded {
                subject,
                object,
                property,
                ..
            } => {
                delta.individual_updates.insert(subject.clone());
                delta.individual_updates.insert(object.clone());
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                let object_iri = match object {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("prop_{subject_iri}_{property:?}_{object_iri}"));
            }
            ABoxChange::ObjectPropertyAssertionRemoved {
                subject,
                object,
                property,
                ..
            } => {
                delta.individual_updates.insert(subject.clone());
                delta.individual_updates.insert(object.clone());
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                let object_iri = match object {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("prop_{subject_iri}_{property:?}_{object_iri}"));
            }
            ABoxChange::DataPropertyAssertionAdded {
                subject,
                property,
                value,
                ..
            } => {
                delta.individual_updates.insert(subject.clone());
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("data_prop_{subject_iri}_{property:?}_{value}"));
            }
            ABoxChange::DataPropertyAssertionRemoved {
                subject,
                property,
                value,
                ..
            } => {
                delta.individual_updates.insert(subject.clone());
                let subject_iri = match subject {
                    Individual::Named(named) => named.iri.to_string(),
                    Individual::Anonymous(anon) => anon.id.clone(),
                };
                delta
                    .cache_invalidations
                    .insert(format!("data_prop_{subject_iri}_{property:?}_{value}"));
            }
        }

        Ok(delta)
    }

    /// Compute delta for axiom addition
    async fn compute_axiom_addition_delta(&self, axiom: &Axiom) -> Result<ReasoningDelta> {
        let mut delta = ReasoningDelta::new();

        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                // Subclass axiom affects hierarchy
                delta
                    .concepts_to_recheck
                    .insert(subclass_axiom.subclass.clone());
                delta
                    .concepts_to_recheck
                    .insert(subclass_axiom.superclass.clone());

                // Extract classes for hierarchy updates
                let subclasses = super::change_tracking::extract_classes_from_class_expression(
                    &subclass_axiom.subclass,
                );
                let superclasses = super::change_tracking::extract_classes_from_class_expression(
                    &subclass_axiom.superclass,
                );

                for subclass in &subclasses {
                    for superclass in &superclasses {
                        delta
                            .hierarchy_updates
                            .insert((subclass.clone(), superclass.clone()));
                    }
                }
            }
            Axiom::EquivalentClasses(equiv_axiom) => {
                // Equivalent classes affect multiple concepts
                for class_expr in &equiv_axiom.classes {
                    delta.concepts_to_recheck.insert(class_expr.clone());
                }

                // Add pairwise hierarchy updates
                let class_sets: Vec<_> = equiv_axiom
                    .classes
                    .iter()
                    .map(super::change_tracking::extract_classes_from_class_expression)
                    .collect();

                for i in 0..class_sets.len() {
                    for j in 0..class_sets.len() {
                        if i != j {
                            for class1 in &class_sets[i] {
                                for class2 in &class_sets[j] {
                                    delta
                                        .hierarchy_updates
                                        .insert((class1.clone(), class2.clone()));
                                }
                            }
                        }
                    }
                }
            }
            Axiom::DisjointClasses(disjoint_axiom) => {
                // Disjoint classes affect satisfiability
                for class_expr in &disjoint_axiom.classes {
                    delta.concepts_to_recheck.insert(class_expr.clone());
                }
            }
            Axiom::ClassAssertion(class_assertion) => {
                // Class assertion affects individual classification
                delta
                    .individual_updates
                    .insert(class_assertion.individual.clone());
                delta
                    .concepts_to_recheck
                    .insert(class_assertion.class.clone());
            }
            _ => {
                // Other axiom types - conservative approach
                delta.recommend_full_reasoning = true;
            }
        }

        Ok(delta)
    }

    /// Compute delta for axiom removal (more complex due to dependencies)
    async fn compute_axiom_removal_delta(&self, axiom: &Axiom) -> Result<ReasoningDelta> {
        let mut delta = ReasoningDelta::new();

        // Axiom removal is more complex - we need to invalidate potentially
        // more reasoning results. For now, we use a conservative approach.
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                delta
                    .concepts_to_recheck
                    .insert(subclass_axiom.subclass.clone());
                delta
                    .concepts_to_recheck
                    .insert(subclass_axiom.superclass.clone());

                // For removal, we're more conservative
                delta.recommend_full_reasoning = true;
            }
            _ => {
                delta.recommend_full_reasoning = true;
            }
        }

        Ok(delta)
    }

    /// Check if a query atom is affected by the given changes
    async fn is_atom_affected_by_changes(
        &self,
        atom: &QueryAtom,
        tbox_changes: &[TBoxChange],
        abox_changes: &[ABoxChange],
    ) -> Result<bool> {
        match atom {
            QueryAtom::ClassAtom {
                class_expression, ..
            } => {
                // Check if any TBox changes affect this class expression
                let classes =
                    super::change_tracking::extract_classes_from_class_expression(class_expression);

                for change in tbox_changes {
                    let affected_classes = change.affected_classes();
                    if !classes.is_disjoint(&affected_classes) {
                        return Ok(true);
                    }
                }

                // Check ABox changes for class assertions
                for change in abox_changes {
                    match change {
                        ABoxChange::ClassAssertionAdded { class, .. }
                        | ABoxChange::ClassAssertionRemoved { class, .. }
                            if class == class_expression => {
                                return Ok(true);
                            }
                        _ => {}
                    }
                }
            }
            QueryAtom::ObjectPropertyAtom { property, .. } => {
                // Check if property-related changes affect this atom
                for change in tbox_changes {
                    match change {
                        TBoxChange::ObjectPropertyAdded { property: prop, .. }
                        | TBoxChange::ObjectPropertyRemoved { property: prop, .. }
                            // Simplified check - would need more sophisticated property matching
                            if format!("{property:?}").contains(&prop.iri.to_string()) => {
                                return Ok(true);
                            }
                        _ => {}
                    }
                }

                // Check ABox property assertions
                for change in abox_changes {
                    match change {
                        ABoxChange::ObjectPropertyAssertionAdded { property: prop, .. }
                        | ABoxChange::ObjectPropertyAssertionRemoved { property: prop, .. }
                            if prop == property => {
                                return Ok(true);
                            }
                        _ => {}
                    }
                }
            }
            QueryAtom::DataPropertyAtom { property, .. } => {
                // Check if data property-related changes affect this atom
                for change in abox_changes {
                    match change {
                        ABoxChange::DataPropertyAssertionAdded { property: prop, .. }
                        | ABoxChange::DataPropertyAssertionRemoved { property: prop, .. }
                            if prop == property => {
                                return Ok(true);
                            }
                        _ => {}
                    }
                }
            }
            QueryAtom::ConcreteIndividualAtom { .. } => {
                // Concrete individual atoms are affected by individual changes
                return Ok(!abox_changes.is_empty());
            }
            QueryAtom::ConcreteLiteralAtom { .. } => {
                // Concrete literal atoms are affected by data property changes
                for change in abox_changes {
                    match change {
                        ABoxChange::DataPropertyAssertionAdded { .. }
                        | ABoxChange::DataPropertyAssertionRemoved { .. } => {
                            return Ok(true);
                        }
                        _ => {}
                    }
                }
            }
            QueryAtom::SameIndividualAtom { .. } | QueryAtom::DifferentIndividualsAtom { .. } => {
                // Individual-related atoms are affected by ABox changes
                return Ok(!abox_changes.is_empty());
            }
        }

        Ok(false)
    }

    /// Estimate the computational cost of applying a reasoning delta
    fn estimate_delta_cost(&self, delta: &ReasoningDelta) -> f64 {
        let concept_cost = delta.concepts_to_recheck.len() as f64 * self.config.concept_cost_weight;
        let hierarchy_cost =
            delta.hierarchy_updates.len() as f64 * self.config.hierarchy_cost_weight;
        let individual_cost =
            delta.individual_updates.len() as f64 * self.config.individual_cost_weight;

        concept_cost + hierarchy_cost + individual_cost
    }

    /// Estimate the computational cost of applying a query delta
    fn estimate_query_delta_cost(&self, delta: &QueryDelta, query: &ConjunctiveQuery) -> f64 {
        let atom_cost = delta.atoms_to_reevaluate.len() as f64 * 5.0;
        let variable_cost = delta.affected_variables.len() as f64 * 2.0;
        let total_atoms = query.body_atoms.len() as f64;

        // If most atoms need re-evaluation, full re-execution might be better
        let selectivity = if total_atoms > 0.0 {
            atom_cost / (total_atoms * 5.0)
        } else {
            1.0
        };

        atom_cost + variable_cost + (selectivity * 10.0)
    }

    /// Apply optimizations to reduce delta complexity
    async fn optimize_delta(&self, delta: &mut ReasoningDelta) -> Result<()> {
        if !self.config.enable_optimizations {
            return Ok(());
        }

        // Remove redundant concept checks
        let original_concept_count = delta.concepts_to_recheck.len();
        self.remove_redundant_concept_checks(delta);

        // Consolidate hierarchy updates
        let original_hierarchy_count = delta.hierarchy_updates.len();
        self.consolidate_hierarchy_updates(delta);

        // Update statistics
        if let Ok(mut stats) = self.statistics.write()
            && (original_concept_count > delta.concepts_to_recheck.len()
                || original_hierarchy_count > delta.hierarchy_updates.len())
        {
            stats.optimizations_applied += 1;
        }

        Ok(())
    }

    /// Remove redundant concept satisfiability checks
    fn remove_redundant_concept_checks(&self, delta: &mut ReasoningDelta) {
        // If we have both a class and a more complex expression containing it,
        // we can often just check the simpler case
        let mut to_remove = HashSet::new();
        let expressions: Vec<_> = delta.concepts_to_recheck.iter().cloned().collect();

        for expr1 in &expressions {
            for expr2 in &expressions {
                if expr1 != expr2 && self.is_expression_subsumed_by(expr1, expr2) {
                    to_remove.insert(expr1.clone());
                }
            }
        }

        for expr in to_remove {
            delta.concepts_to_recheck.remove(&expr);
        }
    }

    /// Consolidate redundant hierarchy updates
    fn consolidate_hierarchy_updates(&self, delta: &mut ReasoningDelta) {
        // Remove transitive redundancies in hierarchy updates
        let updates: Vec<_> = delta.hierarchy_updates.iter().cloned().collect();
        let mut to_remove = HashSet::new();

        for (sub1, super1) in &updates {
            for (sub2, super2) in &updates {
                // If we have A -> B and A -> C where B -> C, we can remove A -> C
                if sub1 == sub2 && super1 != super2 {
                    // Check if there's a path from super1 to super2
                    if self.has_hierarchy_path(super1, super2, &updates) {
                        to_remove.insert((sub1.clone(), super2.clone()));
                    }
                }
            }
        }

        for update in to_remove {
            delta.hierarchy_updates.remove(&update);
        }
    }

    /// Check if one class expression is subsumed by another (simplified)
    fn is_expression_subsumed_by(&self, expr1: &ClassExpression, expr2: &ClassExpression) -> bool {
        match (expr1, expr2) {
            (ClassExpression::Class(class1), ClassExpression::ObjectIntersectionOf(exprs)) => {
                // A class is subsumed by an intersection if the class appears in it
                exprs
                    .iter()
                    .any(|e| matches!(e, ClassExpression::Class(class2) if class1 == class2))
            }
            _ => false, // More complex subsumption checking would go here
        }
    }

    /// Check if there's a hierarchy path between two classes in the updates
    fn has_hierarchy_path(&self, from: &Class, to: &Class, updates: &[(Class, Class)]) -> bool {
        if from == to {
            return true;
        }

        // Simple path finding - in practice would use more sophisticated algorithm
        for (sub, super_class) in updates {
            if sub == from && self.has_hierarchy_path(super_class, to, updates) {
                return true;
            }
        }

        false
    }

    /// Update computation statistics
    async fn update_computation_statistics(
        &self,
        start_time: Instant,
        delta: &ReasoningDelta,
    ) -> Result<()> {
        let computation_time = start_time.elapsed().as_millis() as u64;

        if let Ok(mut stats) = self.statistics.write() {
            stats.delta_computations += 1;
            stats.computation_time_ms += computation_time;

            if delta.recommend_full_reasoning {
                stats.full_reasoning_recommendations += 1;
            }

            let complexity = delta.complexity_score() as f64;
            stats.average_delta_complexity = (stats.average_delta_complexity
                * (stats.delta_computations - 1) as f64
                + complexity)
                / stats.delta_computations as f64;
        }

        Ok(())
    }

    /// Compute affected saturation nodes from `TBox` changes
    pub fn compute_affected_saturation_nodes(
        &self,
        tbox_changes: &[TBoxChange],
    ) -> Result<HashSet<ClassExpression>> {
        let mut affected = HashSet::new();

        for change in tbox_changes {
            let changed_concepts = match change {
                TBoxChange::AxiomAdded { axiom, .. } | TBoxChange::AxiomRemoved { axiom, .. } => {
                    self.extract_concepts_from_axiom(axiom)
                }
                TBoxChange::ClassAdded { class, .. } | TBoxChange::ClassRemoved { class, .. } => {
                    vec![ClassExpression::Class(class.clone())]
                }
                TBoxChange::ObjectPropertyAdded { .. }
                | TBoxChange::ObjectPropertyRemoved { .. }
                | TBoxChange::DataPropertyAdded { .. }
                | TBoxChange::DataPropertyRemoved { .. } => {
                    // Property changes may affect all concepts with restrictions
                    // For now, mark as affecting nothing specific
                    vec![]
                }
            };

            affected.extend(changed_concepts);
        }

        // If we have a cached saturation result, compute transitive dependencies
        if let Some(cached) = self.get_cached_saturation() {
            affected = self.compute_transitive_affected(&affected, &cached);
        }

        Ok(affected)
    }

    /// Compute affected saturation nodes from `ABox` changes
    pub fn compute_affected_saturation_from_abox(
        &self,
        abox_changes: &[ABoxChange],
    ) -> Result<HashSet<ClassExpression>> {
        let mut affected = HashSet::new();

        for change in abox_changes {
            match change {
                ABoxChange::ClassAssertionAdded { class, .. }
                | ABoxChange::ClassAssertionRemoved { class, .. } => {
                    affected.insert(class.clone());
                }
                _ => {}
            }
        }

        Ok(affected)
    }

    /// Compute transitive dependencies from changed concepts
    fn compute_transitive_affected(
        &self,
        changed: &HashSet<ClassExpression>,
        saturation: &SaturationResult,
    ) -> HashSet<ClassExpression> {
        let mut affected = changed.clone();
        let mut to_process: Vec<_> = changed.iter().cloned().collect();

        while let Some(concept) = to_process.pop() {
            // Find all concepts that depend on this concept
            for (other_concept, node) in &saturation.nodes {
                if (node.saturated_concepts.contains(&concept)
                    || node.direct_subsumers.contains(&concept)
                    || node.all_subsumers.contains(&concept))
                    && affected.insert(other_concept.clone())
                {
                    to_process.push(other_concept.clone());
                }
            }
        }

        affected
    }

    /// Apply saturation-aware delta computation
    pub async fn compute_saturation_delta(
        &self,
        tbox_changes: &[TBoxChange],
        abox_changes: &[ABoxChange],
    ) -> Result<ReasoningDelta> {
        if !self.config.enable_saturation_deltas || self.saturation_engine.is_none() {
            // Fall back to standard delta computation
            return self
                .compute_reasoning_delta_for_changes(tbox_changes, abox_changes)
                .await;
        }

        let mut delta = ReasoningDelta::new();

        // Compute affected saturation nodes
        let tbox_affected = self.compute_affected_saturation_nodes(tbox_changes)?;
        let abox_affected = self.compute_affected_saturation_from_abox(abox_changes)?;

        delta.saturation_updates.extend(tbox_affected);
        delta.saturation_updates.extend(abox_affected);

        // Compute standard delta components
        for change in tbox_changes {
            let change_delta = self.compute_tbox_change_delta(change).await?;
            delta.merge(change_delta);
        }

        for change in abox_changes {
            let change_delta = self.compute_abox_change_delta(change).await?;
            delta.merge(change_delta);
        }

        // Determine if incremental saturation is appropriate
        let total_concepts = if let Some(cached) = self.get_cached_saturation() {
            cached.nodes.len()
        } else {
            100 // Default estimate
        };

        let affected_ratio = delta.saturation_updates.len() as f64 / total_concepts.max(1) as f64;
        delta.use_incremental_saturation =
            affected_ratio <= self.config.saturation_incremental_threshold;

        // Update cost estimate
        delta.estimated_cost = self.estimate_delta_cost(&delta);

        Ok(delta)
    }

    /// Extract concepts from an axiom
    fn extract_concepts_from_axiom(&self, axiom: &Axiom) -> Vec<ClassExpression> {
        match axiom {
            Axiom::SubClassOf(ax) => {
                vec![ax.subclass.clone(), ax.superclass.clone()]
            }
            Axiom::EquivalentClasses(ax) => ax.classes.clone(),
            Axiom::DisjointClasses(ax) => ax.classes.clone(),
            Axiom::DisjointUnion(ax) => {
                let mut concepts = ax.disjoint_classes.clone();
                concepts.push(ax.class.clone());
                concepts
            }
            Axiom::ClassAssertion(ax) => vec![ax.class.clone()],
            Axiom::ObjectPropertyDomain(ax) => vec![ax.domain.clone()],
            Axiom::ObjectPropertyRange(ax) => vec![ax.range.clone()],
            Axiom::DataPropertyDomain(ax) => vec![ax.domain.clone()],
            _ => vec![],
        }
    }
}

// Helper function to extract classes from class expression (re-exported for convenience)
pub use super::change_tracking::extract_classes_from_class_expression;

//! High-level Reasoning Interface for Oxidowl
//!
//! `ReasoningService` is a lightweight, `Clone`-able handle that communicates
//! with a background `ReasoningActor` via `tokio::sync::mpsc` channels.
//! All mutable state (`Reasoner`, `CacheManager`, `SWRLRuleEngine`) is owned
//! exclusively by the actor, eliminating `Arc<RwLock<>>` contention.

#![allow(dead_code)]

pub mod actor;
// Incremental reasoning framework
pub mod incremental;

// Re-export core reasoner types for public API
pub use crate::core::reasoner::{
    ClassificationResult, RealizationResult, ReasoningResult, ReasoningTask,
};

// Re-export incremental reasoning types for public API
pub use incremental::{
    ChangeEvent, ChangeTracker, DeltaComputer, IncrementalCacheManager, IncrementalConfig,
    IncrementalReasoningService, IncrementalStatistics,
};

use actor::{ReasoningRequest, spawn_actor};

use crate::{
    Error, Result,
    config::ReasonerConfig,
    ontology::{
        ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression, Ontology,
    },
    query::{DLQuery, DLQueryEngine, QueryResult},
    swrl::SWRLExecutionResult,
};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

// ── Shutdown guard ────────────────────────────────────────────────────────────

/// Signals the actor to stop when the last `ReasoningService` clone is dropped.
struct ShutdownGuard(Option<oneshot::Sender<()>>);

impl std::fmt::Debug for ShutdownGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShutdownGuard").finish_non_exhaustive()
    }
}

impl Drop for ShutdownGuard {
    fn drop(&mut self) {
        if let Some(tx) = self.0.take() {
            let _ = tx.send(());
        }
    }
}

// ── Public handle ─────────────────────────────────────────────────────────────

/// Lightweight, cheaply-cloneable handle to the `ReasoningActor`.
///
/// Creating a `ReasoningService` spawns a background actor task that owns all
/// reasoning state exclusively (no `Arc<RwLock<>>`). Public methods send a
/// typed message via `mpsc` and await a `oneshot` reply.
#[derive(Debug, Clone)]
pub struct ReasoningService {
    sender: mpsc::Sender<ReasoningRequest>,
    pub(crate) config: ReasonerConfig,
    /// Shared guard: actor is stopped when every clone of this service is dropped.
    _shutdown: Arc<ShutdownGuard>,
}

impl ReasoningService {
    /// Creates a new reasoning service, spawning its background actor.
    pub fn new(ontology: Ontology, config: ReasonerConfig) -> Result<Self> {
        let (sender, shutdown_tx) = spawn_actor(ontology, config.clone())?;
        Ok(Self {
            sender,
            config,
            _shutdown: Arc::new(ShutdownGuard(Some(shutdown_tx))),
        })
    }

    // ─── private channel helpers ──────────────────────────────────────────────

    async fn send<F, T>(&self, build: F) -> Result<T>
    where
        F: FnOnce(oneshot::Sender<Result<T>>) -> ReasoningRequest,
    {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(build(tx))
            .await
            .map_err(|_| Error::reasoning("Reasoning actor has shut down"))?;
        rx.await
            .map_err(|_| Error::reasoning("Reasoning actor dropped reply channel"))?
    }

    /// Block-in-place bridge for callers inside a tokio runtime that cannot `.await`.
    fn send_sync<F, T>(&self, build: F) -> Result<T>
    where
        F: FnOnce(oneshot::Sender<Result<T>>) -> ReasoningRequest,
        T: Send + 'static,
    {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.send(build))
        })
    }

    /// Check consistency of the ontology
    pub async fn is_consistent(&self) -> Result<bool> {
        self.send(|r| ReasoningRequest::IsConsistent { reply: r }).await
    }

    /// Check satisfiability of a class expression
    pub async fn is_satisfiable(&self, expression: &ClassExpression) -> Result<bool> {
        let expression = expression.clone();
        self.send(|r| ReasoningRequest::IsSatisfiable { expression, reply: r }).await
    }

    /// Check subsumption of two class expressions
    pub async fn is_subsumed_by(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        let (subclass, superclass) = (subclass.clone(), superclass.clone());
        self.send(|r| ReasoningRequest::IsSubsumedBy { subclass, superclass, reply: r }).await
    }

    /// Check equivalence of two class expressions
    pub async fn is_equivalent_to(
        &self,
        class1: &ClassExpression,
        class2: &ClassExpression,
    ) -> Result<bool> {
        let subsumes_1_2 = self.is_subsumed_by(class1, class2).await?;
        let subsumes_2_1 = self.is_subsumed_by(class2, class1).await?;
        Ok(subsumes_1_2 && subsumes_2_1)
    }

    // Check disjointness of two class expressions
    pub async fn is_disjoint_with(
        &self,
        class1: &ClassExpression,
        class2: &ClassExpression,
    ) -> Result<bool> {
        let intersection =
            ClassExpression::ObjectIntersectionOf(vec![class1.clone(), class2.clone()]);
        let satisfiable = self.is_satisfiable(&intersection).await?;
        Ok(!satisfiable)
    }

    // Get all direct superclasses of a class expression
    pub async fn get_superclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let class = class.clone();
        self.send(|r| ReasoningRequest::GetSuperclasses { class, direct, reply: r }).await
    }

    /// Get all direct subclasses of a class expression
    pub async fn get_subclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let class = class.clone();
        self.send(|r| ReasoningRequest::GetSubclasses { class, direct, reply: r }).await
    }

    /// Get all equivalent classes of a class expression
    pub async fn get_equivalent_classes(
        &self,
        class: &ClassExpression,
    ) -> Result<HashSet<ClassExpression>> {
        let class = class.clone();
        self.send(|r| ReasoningRequest::GetEquivalentClasses { class, reply: r }).await
    }

    /// Get all instances of a class expression
    pub async fn get_instances(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<Individual>> {
        let class = class.clone();
        self.send(|r| ReasoningRequest::GetInstances { class, direct, reply: r }).await
    }

    /// Get all types of an individual
    pub async fn get_types(
        &self,
        individual: &Individual,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let individual = individual.clone();
        self.send(|r| ReasoningRequest::GetTypes { individual, direct, reply: r }).await
    }

    /// Check if an individual is an instance of a class expression.
    pub async fn is_instance_of(
        &self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        let (individual, class) = (individual.clone(), class.clone());
        self.send(|r| ReasoningRequest::IsInstanceOf { individual, class, reply: r }).await
    }

    /// OWL DL membership query: is `individual` a member of `class_expr`?
    pub async fn is_member_of(
        &self,
        individual: &Individual,
        class_expr: &ClassExpression,
    ) -> Result<bool> {
        self.is_instance_of(individual, class_expr).await
    }

    /// Get object property values for an individual
    pub async fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<HashSet<Individual>> {
        let (individual, property) = (individual.clone(), property.clone());
        self.send(|r| ReasoningRequest::GetObjectPropertyValues { individual, property, reply: r })
            .await
    }

    /// Get data property values for an individual
    pub async fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<HashSet<crate::ontology::Literal>> {
        let (individual, property) = (individual.clone(), property.clone());
        self.send(|r| ReasoningRequest::GetDataPropertyValues { individual, property, reply: r })
            .await
    }

    /// Classify the ontology (compute class hierarchy)
    pub async fn classify(&self) -> Result<ClassificationResult> {
        self.send(|r| ReasoningRequest::Classify { reply: r }).await
    }

    /// Execute SWRL rules and apply inferences to the ontology
    pub async fn execute_swrl_rules(&self) -> Result<SWRLExecutionResult> {
        self.send(|r| ReasoningRequest::ExecuteSwrlRules { reply: r }).await
    }

    /// Execute a DL query using Manchester Syntax
    pub async fn dl_query(&self, query_string: &str) -> Result<QueryResult> {
        let query_engine = DLQueryEngine::new(Arc::new(self.clone()));
        query_engine.execute_query(query_string).await
    }

    /// Parse a DL query without executing it
    pub async fn parse_dl_query(&self, query_string: &str) -> Result<DLQuery> {
        let query_engine = DLQueryEngine::new(Arc::new(self.clone()));
        query_engine.parse_query(query_string).await
    }

    /// Realize the ontology (compute individuals' types)
    pub async fn realize(&self) -> Result<RealizationResult> {
        self.send(|r| ReasoningRequest::Realize { reply: r }).await
    }

    /// Get explanation for an entailment
    pub async fn explain_entailment(
        &self,
        axiom: &crate::ontology::Axiom,
    ) -> Result<Vec<ExplanationSet>> {
        let axiom = axiom.clone();
        self.send(|r| ReasoningRequest::ExplainEntailment { axiom, reply: r }).await
    }

    /// Get explanation for inconsistent ontology
    pub async fn explain_inconsistency(&self) -> Result<Vec<ExplanationSet>> {
        self.send(|r| ReasoningRequest::ExplainInconsistency { reply: r }).await
    }

    /// Add axioms incrementally to the ontology
    pub async fn add_axioms(&self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        self.send(|r| ReasoningRequest::AddAxioms { axioms, reply: r }).await
    }

    /// Remove axioms incrementally from the ontology
    pub async fn remove_axioms(&self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        self.send(|r| ReasoningRequest::RemoveAxioms { axioms, reply: r }).await
    }

    /// Get reasoning statistics
    pub async fn get_statistics(&self) -> Result<ReasoningStatistics> {
        self.send(|r| ReasoningRequest::GetStatistics { reply: r }).await
    }

    /// Query property chain reasoning
    pub async fn query_property_chain(
        &self,
        individual: &Individual,
        property_chain: &[ObjectPropertyExpression],
    ) -> Result<HashSet<Individual>> {
        if property_chain.is_empty() {
            return Ok(HashSet::new());
        }
        if property_chain.len() == 1 {
            return self.get_object_property_values(individual, &property_chain[0]).await;
        }
        let mut current = HashSet::new();
        current.insert(individual.clone());
        for property in property_chain {
            let mut next = HashSet::new();
            for curr_ind in &current {
                let targets = self.get_object_property_values(curr_ind, property).await?;
                next.extend(targets);
            }
            current = next;
            if current.is_empty() {
                break;
            }
        }
        Ok(current)
    }

    /// Get SWRL execution statistics
    pub async fn get_swrl_statistics(&self) -> Result<crate::swrl::SWRLStatistics> {
        self.send(|r| ReasoningRequest::GetSwrlStatistics { reply: r }).await
    }

    /// Set SWRL rule priority
    pub async fn set_swrl_rule_priority(&self, rule_id: u64, priority: u32) -> Result<()> {
        self.send(|r| ReasoningRequest::SetSwrlRulePriority { rule_id, priority, reply: r }).await
    }

    /// Get ordered SWRL rules by priority
    pub async fn get_swrl_rule_order(&self) -> Result<Vec<u64>> {
        self.send(|r| ReasoningRequest::GetSwrlRuleOrder { reply: r }).await
    }

    /// Enable or disable a specific SWRL rule
    pub async fn set_swrl_rule_active(&self, rule_id: u64, active: bool) -> Result<()> {
        self.send(|r| ReasoningRequest::SetSwrlRuleActive { rule_id, active, reply: r }).await
    }

    /// Validate the loaded ontology against a SHACL shapes graph.
    pub fn validate_shacl(
        &self,
        shapes_turtle: &str,
        data_turtle: &str,
    ) -> Result<crate::validation::shacl::ShaclValidationReport> {
        let mut validator =
            crate::validation::shacl::ShaclValidator::new(shapes_turtle, data_turtle)?;
        validator.validate()
    }

    /// Invalidate all caches
    pub async fn invalidate_all_caches(&self) -> Result<()> {
        self.send(|r| ReasoningRequest::InvalidateAllCaches { reply: r }).await
    }

    /// Get the IRI of the current ontology
    pub async fn get_ontology_iri(&self) -> Result<Option<crate::ontology::IRI>> {
        self.send(|r| ReasoningRequest::GetOntologyIri { reply: r }).await
    }

    /// Synchronous version of `get_instances` for use in advanced query processing
    pub fn get_instances_sync(&self, class: &ClassExpression) -> Result<Vec<Individual>> {
        let class = class.clone();
        self.send_sync(|r| ReasoningRequest::GetInstancesSync { class, reply: r })
    }

    /// Synchronous version of `is_instance_of` for use in advanced query processing
    pub fn is_instance_of_sync(
        &self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        let (individual, class) = (individual.clone(), class.clone());
        self.send_sync(|r| ReasoningRequest::IsInstanceOfSync { individual, class, reply: r })
    }

    /// Get object property assertions (for advanced query processing)
    pub fn get_object_property_assertions_sync(
        &self,
        _property: &ObjectPropertyExpression,
    ) -> Result<Vec<(Individual, Individual)>> {
        Ok(Vec::new())
    }

    /// Create an incremental reasoning service wrapper
    pub async fn into_incremental(
        self: Arc<Self>,
        ontology: Arc<tokio::sync::RwLock<Ontology>>,
        config: Option<IncrementalConfig>,
    ) -> Result<incremental::IncrementalReasoningService> {
        incremental::IncrementalReasoningService::new(self, ontology, config).await
    }
}

/// Explanation set for reasoning entailments
#[derive(Debug, Clone)]
pub struct ExplanationSet {
    pub axioms: HashSet<crate::ontology::Axiom>,
    pub minimal: bool,
}

impl ExplanationSet {
    // Create a new explanation set
    #[must_use]
    pub fn new(axioms: HashSet<crate::ontology::Axiom>) -> Self {
        Self {
            axioms,
            minimal: true, // Default to minimal explanations
        }
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.axioms.len()
    }

    #[must_use]
    pub fn is_minimal(&self) -> bool {
        self.minimal
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.axioms.is_empty()
    }
}

/// Reasoning statistics for the service
#[derive(Debug, Clone)]
pub struct ReasoningStatistics {
    pub ontology_size: usize,
    pub reasoning_time: Duration,
    pub cache_stats: crate::cache::CacheStats,
    pub memory_usage: usize, // In bytes
}

pub struct QueryInterface {
    reasoning_service: ReasoningService,
}

impl QueryInterface {
    #[must_use]
    pub fn new(reasoning_service: ReasoningService) -> Self {
        Self { reasoning_service }
    }

    /// Execute a subsumption query
    pub async fn execute_subsumption_query(
        &self,
        subclass: ClassExpression,
        superclass: ClassExpression,
    ) -> Result<bool> {
        self.reasoning_service
            .is_subsumed_by(&subclass, &superclass)
            .await
    }

    /// Execute an instance query
    pub async fn execute_instance_query(
        &self,
        individual: Individual,
        class: ClassExpression,
    ) -> Result<bool> {
        self.reasoning_service
            .is_instance_of(&individual, &class)
            .await
    }

    pub async fn query_instances(
        &self,
        class: ClassExpression,
        direct: bool,
    ) -> Result<HashSet<Individual>> {
        self.reasoning_service.get_instances(&class, direct).await
    }

    /// Execute a property value query
    pub async fn execute_property_value_query(
        &self,
        individual: Individual,
        property_chain: Vec<ObjectPropertyExpression>,
    ) -> Result<HashSet<Individual>> {
        if property_chain.len() == 1 {
            // Single property query
            self.reasoning_service
                .get_object_property_values(&individual, &property_chain[0])
                .await
        } else {
            // Multi-step property chain query
            self.reasoning_service
                .query_property_chain(&individual, &property_chain)
                .await
        }
    }

    /// Execute batch queries
    pub async fn batch_satisfiability_check(
        &self,
        concepts: Vec<ClassExpression>,
    ) -> Result<HashMap<ClassExpression, bool>> {
        let mut results = HashMap::with_capacity(concepts.len());

        for concept in concepts {
            let result = self.reasoning_service.is_satisfiable(&concept).await?;
            results.insert(concept, result);
        }

        Ok(results)
    }

    /// Execute batch subsumption check
    pub async fn batch_subsumption_check(
        &self,
        queries: Vec<(ClassExpression, ClassExpression)>,
    ) -> Result<HashMap<(ClassExpression, ClassExpression), bool>> {
        let mut results = HashMap::with_capacity(queries.len());

        for (subclass, superclass) in queries {
            let result = self
                .reasoning_service
                .is_subsumed_by(&subclass, &superclass)
                .await?;
            results.insert((subclass, superclass), result);
        }

        Ok(results)
    }
}



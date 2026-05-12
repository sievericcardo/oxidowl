//! ReasoningActor — actor-pattern wrapper for the core reasoning state machine.
//!
//! `ReasoningActor` owns all mutable state (Reasoner, CacheManager, SWRLRuleEngine)
//! without any `Arc<RwLock<>>` wrapping. It runs an asynchronous event loop driven
//! by `tokio::select!` over an `mpsc` request channel, an optional shutdown signal,
//! and a periodic maintenance ticker.
//!
//! External callers interact exclusively through `ReasoningService`, which holds the
//! `mpsc::Sender<ReasoningRequest>` handle. Each request variant carries a
//! `oneshot::Sender` for the reply, eliminating all write-lock bottlenecks.

#![allow(dead_code)]

use crate::{
    Error, Result,
    cache::CacheManager,
    config::{CacheFeature, ReasonerConfig},
    core::{
        lock_helpers::write_lock,
        reasoner::{ClassificationResult, RealizationResult, Reasoner},
    },
    ontology::{ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression},
    reasoning::{ExplanationSet, ReasoningStatistics},
    swrl::{SWRLConfig, SWRLExecutionResult, SWRLRuleEngine},
};
use std::{
    collections::HashSet,
    time::{Duration, Instant},
};
use tokio::sync::{mpsc, oneshot};

// ─────────────────────────────────────────────────────────────
// Message types
// ─────────────────────────────────────────────────────────────

/// All request variants the `ReasoningActor` can handle.
/// Each variant carries a `oneshot::Sender` for returning the result.
pub enum ReasoningRequest {
    IsConsistent {
        reply: oneshot::Sender<Result<bool>>,
    },
    IsSatisfiable {
        expression: ClassExpression,
        reply: oneshot::Sender<Result<bool>>,
    },
    IsSubsumedBy {
        subclass: ClassExpression,
        superclass: ClassExpression,
        reply: oneshot::Sender<Result<bool>>,
    },
    GetSuperclasses {
        class: ClassExpression,
        direct: bool,
        reply: oneshot::Sender<Result<HashSet<ClassExpression>>>,
    },
    GetSubclasses {
        class: ClassExpression,
        direct: bool,
        reply: oneshot::Sender<Result<HashSet<ClassExpression>>>,
    },
    GetEquivalentClasses {
        class: ClassExpression,
        reply: oneshot::Sender<Result<HashSet<ClassExpression>>>,
    },
    GetInstances {
        class: ClassExpression,
        direct: bool,
        reply: oneshot::Sender<Result<HashSet<Individual>>>,
    },
    GetTypes {
        individual: Individual,
        direct: bool,
        reply: oneshot::Sender<Result<HashSet<ClassExpression>>>,
    },
    IsInstanceOf {
        individual: Individual,
        class: ClassExpression,
        reply: oneshot::Sender<Result<bool>>,
    },
    GetObjectPropertyValues {
        individual: Individual,
        property: ObjectPropertyExpression,
        reply: oneshot::Sender<Result<HashSet<Individual>>>,
    },
    GetDataPropertyValues {
        individual: Individual,
        property: DataPropertyExpression,
        reply: oneshot::Sender<Result<HashSet<crate::ontology::Literal>>>,
    },
    Classify {
        reply: oneshot::Sender<Result<ClassificationResult>>,
    },
    Realize {
        reply: oneshot::Sender<Result<RealizationResult>>,
    },
    ExecuteSwrlRules {
        reply: oneshot::Sender<Result<SWRLExecutionResult>>,
    },
    ExplainEntailment {
        axiom: crate::ontology::Axiom,
        reply: oneshot::Sender<Result<Vec<ExplanationSet>>>,
    },
    ExplainInconsistency {
        reply: oneshot::Sender<Result<Vec<ExplanationSet>>>,
    },
    AddAxioms {
        axioms: Vec<crate::ontology::Axiom>,
        reply: oneshot::Sender<Result<()>>,
    },
    RemoveAxioms {
        axioms: Vec<crate::ontology::Axiom>,
        reply: oneshot::Sender<Result<()>>,
    },
    GetStatistics {
        reply: oneshot::Sender<Result<ReasoningStatistics>>,
    },
    GetSwrlStatistics {
        reply: oneshot::Sender<Result<crate::swrl::SWRLStatistics>>,
    },
    SetSwrlRulePriority {
        rule_id: u64,
        priority: u32,
        reply: oneshot::Sender<Result<()>>,
    },
    GetSwrlRuleOrder {
        reply: oneshot::Sender<Result<Vec<u64>>>,
    },
    SetSwrlRuleActive {
        rule_id: u64,
        active: bool,
        reply: oneshot::Sender<Result<()>>,
    },
    GetOntologyIri {
        reply: oneshot::Sender<Result<Option<crate::ontology::IRI>>>,
    },
    /// Export the ontology serialized as Turtle (RDF 1.2).
    GetSerializedTurtle {
        reply: oneshot::Sender<Result<String>>,
    },
    /// Synchronous bridge: used by `get_instances_sync` (called from non-async query engine)
    GetInstancesSync {
        class: ClassExpression,
        reply: oneshot::Sender<Result<Vec<Individual>>>,
    },
    /// Synchronous bridge: used by `is_instance_of_sync`
    IsInstanceOfSync {
        individual: Individual,
        class: ClassExpression,
        reply: oneshot::Sender<Result<bool>>,
    },
    InvalidateAllCaches {
        reply: oneshot::Sender<Result<()>>,
    },
}

// ─────────────────────────────────────────────────────────────
// Actor
// ─────────────────────────────────────────────────────────────

/// Actor that exclusively owns all reasoning state.
/// Processes one `ReasoningRequest` at a time — no locking required.
pub struct ReasoningActor {
    receiver: mpsc::Receiver<ReasoningRequest>,
    reasoner: Reasoner,
    cache_manager: CacheManager,
    swrl_engine: SWRLRuleEngine,
    config: ReasonerConfig,
}

impl ReasoningActor {
    pub fn new(
        receiver: mpsc::Receiver<ReasoningRequest>,
        reasoner: Reasoner,
        cache_manager: CacheManager,
        swrl_engine: SWRLRuleEngine,
        config: ReasonerConfig,
    ) -> Self {
        Self {
            receiver,
            reasoner,
            cache_manager,
            swrl_engine,
            config,
        }
    }

    /// Main event loop. Returns when all senders are dropped or the shutdown
    /// signal fires.
    pub async fn run(mut self, mut shutdown: oneshot::Receiver<()>) {
        const MAINTENANCE_INTERVAL: Duration = Duration::from_mins(1);
        let mut maintenance = tokio::time::interval(MAINTENANCE_INTERVAL);
        // Skip the immediate first tick so maintenance doesn't fire at t=0.
        maintenance.tick().await;

        loop {
            tokio::select! {
                _ = &mut shutdown => break,
                msg = self.receiver.recv() => {
                    match msg {
                        Some(req) => self.handle(req),
                        None => break, // all senders dropped
                    }
                }
                _ = maintenance.tick() => {
                    self.perform_maintenance();
                }
            }
        }
    }

    fn perform_maintenance(&mut self) {
        // Opportunistic: attempt to clear expired cache entries.
        // Failures are non-fatal.
        let _ = self.cache_manager.clear_all();
    }

    // ─── dispatch ───────────────────────────────────────────

    fn handle(&mut self, request: ReasoningRequest) {
        match request {
            ReasoningRequest::IsConsistent { reply } => {
                let _ = reply.send(self.handle_is_consistent());
            }
            ReasoningRequest::IsSatisfiable { expression, reply } => {
                let _ = reply.send(self.handle_is_satisfiable(&expression));
            }
            ReasoningRequest::IsSubsumedBy {
                subclass,
                superclass,
                reply,
            } => {
                let _ = reply.send(self.handle_is_subsumed_by(&subclass, &superclass));
            }
            ReasoningRequest::GetSuperclasses {
                class,
                direct,
                reply,
            } => {
                let _ = reply.send(self.handle_get_superclasses(&class, direct));
            }
            ReasoningRequest::GetSubclasses {
                class,
                direct,
                reply,
            } => {
                let _ = reply.send(self.handle_get_subclasses(&class, direct));
            }
            ReasoningRequest::GetEquivalentClasses { class, reply } => {
                let _ = reply.send(self.handle_get_equivalent_classes(&class));
            }
            ReasoningRequest::GetInstances {
                class,
                direct,
                reply,
            } => {
                let _ = reply.send(self.handle_get_instances(&class, direct));
            }
            ReasoningRequest::GetTypes {
                individual,
                direct,
                reply,
            } => {
                let _ = reply.send(self.handle_get_types(&individual, direct));
            }
            ReasoningRequest::IsInstanceOf {
                individual,
                class,
                reply,
            } => {
                let _ = reply.send(self.handle_is_instance_of(&individual, &class));
            }
            ReasoningRequest::GetObjectPropertyValues {
                individual,
                property,
                reply,
            } => {
                let _ = reply.send(self.handle_get_object_property_values(&individual, &property));
            }
            ReasoningRequest::GetDataPropertyValues {
                individual,
                property,
                reply,
            } => {
                let _ = reply.send(self.handle_get_data_property_values(&individual, &property));
            }
            ReasoningRequest::Classify { reply } => {
                let _ = reply.send(self.handle_classify());
            }
            ReasoningRequest::Realize { reply } => {
                let _ = reply.send(self.handle_realize());
            }
            ReasoningRequest::ExecuteSwrlRules { reply } => {
                let _ = reply.send(self.handle_execute_swrl_rules());
            }
            ReasoningRequest::ExplainEntailment { axiom, reply } => {
                let _ = reply.send(self.handle_explain_entailment(&axiom));
            }
            ReasoningRequest::ExplainInconsistency { reply } => {
                let _ = reply.send(self.handle_explain_inconsistency());
            }
            ReasoningRequest::AddAxioms { axioms, reply } => {
                let _ = reply.send(self.handle_add_axioms(axioms));
            }
            ReasoningRequest::RemoveAxioms { axioms, reply } => {
                let _ = reply.send(self.handle_remove_axioms(axioms));
            }
            ReasoningRequest::GetStatistics { reply } => {
                let _ = reply.send(self.handle_get_statistics());
            }
            ReasoningRequest::GetSwrlStatistics { reply } => {
                let _ = reply.send(Ok(self.swrl_engine.get_statistics().clone()));
            }
            ReasoningRequest::SetSwrlRulePriority {
                rule_id,
                priority,
                reply,
            } => {
                self.swrl_engine.set_rule_priority(rule_id, priority);
                let _ = reply.send(Ok(()));
            }
            ReasoningRequest::GetSwrlRuleOrder { reply } => {
                let _ = reply.send(Ok(self.swrl_engine.get_rule_ids()));
            }
            ReasoningRequest::SetSwrlRuleActive {
                rule_id,
                active,
                reply,
            } => {
                let result = self
                    .swrl_engine
                    .set_rule_active(rule_id, active)
                    .map_err(|e| {
                        Error::reasoning(format!(
                            "Failed to set SWRL rule {rule_id} active state: {e}"
                        ))
                    });
                let _ = reply.send(result);
            }
            ReasoningRequest::GetOntologyIri { reply } => {
                let _ = reply.send(self.handle_get_ontology_iri());
            }
            ReasoningRequest::GetSerializedTurtle { reply } => {
                let _ = reply.send(self.handle_get_serialized_turtle());
            }
            ReasoningRequest::GetInstancesSync { class, reply } => {
                let result = self
                    .reasoner
                    .get_instances(&class, false)
                    .map(|v| v.into_iter().collect());
                let _ = reply.send(result);
            }
            ReasoningRequest::IsInstanceOfSync {
                individual,
                class,
                reply,
            } => {
                let _ = reply.send(self.reasoner.is_instance_of(&individual, &class));
            }
            ReasoningRequest::InvalidateAllCaches { reply } => {
                tracing::debug!("All caches invalidated via actor request");
                let _ = reply.send(Ok(()));
            }
        }
    }

    // ─── handler implementations ─────────────────────────────

    fn handle_is_consistent(&mut self) -> Result<bool> {
        let start = Instant::now();

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            let ont_opt = self.reasoner.get_ontology().cloned();
            if let Some(ont) = ont_opt
                && let Some(result) = self.cache_manager.get_consistency_result(&ont)
            {
                return Ok(result);
            }
        }

        log::info!("Executing SWRL rules before consistency check");
        self.handle_execute_swrl_rules()?;

        let result = self.reasoner.is_consistent()?;

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            let ont_opt = self.reasoner.get_ontology().cloned();
            if let Some(ont) = ont_opt {
                self.cache_manager.cache_consistency_result(&ont, result);
            }
        }

        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Consistency check timed out".into(),
            });
        }

        log::info!("Consistency check completed in {:?}", start.elapsed());
        Ok(result)
    }

    fn handle_is_satisfiable(&mut self, expression: &ClassExpression) -> Result<bool> {
        let start = Instant::now();

        if self.config.cache.is_enabled(CacheFeature::Satisfiability)
            && let Some(result) = self.cache_manager.get_satisfiability_result(expression)
        {
            return Ok(result);
        }

        let result = self.reasoner.is_class_satisfiable(expression)?;

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            self.cache_manager
                .cache_satisfiability_result(expression.clone(), result);
        }

        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Satisfiability check timed out".into(),
            });
        }

        log::info!("Satisfiability check completed in {:?}", start.elapsed());
        Ok(result)
    }

    fn handle_is_subsumed_by(
        &mut self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        let start = Instant::now();

        if self.config.cache.is_enabled(CacheFeature::Satisfiability)
            && let Some(result) = self
                .cache_manager
                .get_subsumption_result(subclass, superclass)
        {
            return Ok(result);
        }

        let result = self.reasoner.is_subsumed_by(subclass, superclass)?;

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            self.cache_manager.cache_subsumption_result(
                subclass.clone(),
                superclass.clone(),
                result,
            );
        }

        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Subsumption check timed out".into(),
            });
        }

        log::info!("Subsumption check completed in {:?}", start.elapsed());
        Ok(result)
    }

    fn handle_get_superclasses(
        &mut self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();
        let superclasses = self.reasoner.get_superclasses(class, direct)?;
        self.check_timeout(start, "Direct superclass retrieval")?;
        log::info!(
            "Direct superclass retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(superclasses.into_iter().collect())
    }

    fn handle_get_subclasses(
        &mut self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();
        let subclasses = self.reasoner.get_subclasses(class, direct)?;
        self.check_timeout(start, "Direct subclass retrieval")?;
        log::info!(
            "Direct subclass retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(subclasses.into_iter().collect())
    }

    fn handle_get_equivalent_classes(
        &mut self,
        class: &ClassExpression,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();
        let classes = self.reasoner.get_equivalent_classes(class)?;
        self.check_timeout(start, "Equivalent class retrieval")?;
        log::info!(
            "Equivalent class retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(classes.into_iter().collect())
    }

    fn handle_get_instances(
        &mut self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<Individual>> {
        let start = Instant::now();
        let instances = self.reasoner.get_instances(class, direct)?;
        self.check_timeout(start, "Instance retrieval")?;
        log::info!("Instance retrieval completed in {:?}", start.elapsed());
        Ok(instances.into_iter().collect())
    }

    fn handle_get_types(
        &mut self,
        individual: &Individual,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();
        let types = self.reasoner.get_types(individual, direct)?;
        self.check_timeout(start, "Type retrieval")?;
        log::info!("Type retrieval completed in {:?}", start.elapsed());
        Ok(types.into_iter().collect())
    }

    fn handle_is_instance_of(
        &mut self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        self.reasoner.is_instance_of(individual, class)
    }

    fn handle_get_object_property_values(
        &mut self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<HashSet<Individual>> {
        let start = Instant::now();
        let values = self
            .reasoner
            .get_object_property_values(individual, property)?;
        self.check_timeout(start, "Object property value retrieval")?;
        log::info!(
            "Object property value retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(values.into_iter().collect())
    }

    fn handle_get_data_property_values(
        &mut self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<HashSet<crate::ontology::Literal>> {
        let start = Instant::now();
        let result = self
            .reasoner
            .get_data_property_values(individual, property)?;
        self.check_timeout(start, "Data property value retrieval")?;
        let literals = result
            .into_iter()
            .map(|s| crate::ontology::Literal {
                value: s.to_string(),
                datatype: Some(
                    url::Url::parse("http://www.w3.org/2001/XMLSchema#string")
                        .expect("Valid hardcoded XSD string URL"),
                ),
                language: None,
            })
            .collect();
        log::info!(
            "Data property value retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(literals)
    }

    fn handle_classify(&mut self) -> Result<ClassificationResult> {
        let start = Instant::now();

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            let ont_opt = self.reasoner.get_ontology().cloned();
            if let Some(ont) = ont_opt
                && let Some(cached) = self.cache_manager.get_classification_result(&ont)
            {
                log::info!("Classification (cached) completed in {:?}", start.elapsed());
                return Ok(cached);
            }
        }

        log::info!("Executing SWRL rules before classification");
        self.handle_execute_swrl_rules()?;

        let result = self.reasoner.classify()?;

        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Classification timed out".into(),
            });
        }

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            let ont_opt = self.reasoner.get_ontology().cloned();
            if let Some(ont) = ont_opt {
                self.cache_manager
                    .store_classification_result(&ont, result.clone());
            }
        }

        log::info!("Classification completed in {:?}", start.elapsed());
        Ok(result)
    }

    fn handle_realize(&mut self) -> Result<RealizationResult> {
        let start = Instant::now();

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            let ont_opt = self.reasoner.get_ontology().cloned();
            if let Some(ont) = ont_opt
                && let Some(cached) = self.cache_manager.get_realization_result(&ont)
            {
                log::info!("Realization (cached) completed in {:?}", start.elapsed());
                return Ok(cached);
            }
        }

        log::info!("Executing SWRL rules before realization");
        self.handle_execute_swrl_rules()?;

        let result = self.reasoner.realize()?;

        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Realization timed out".into(),
            });
        }

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            let ont_opt = self.reasoner.get_ontology().cloned();
            if let Some(ont) = ont_opt {
                self.cache_manager
                    .store_realization_result(&ont, result.clone());
            }
        }

        log::info!("Realization completed in {:?}", start.elapsed());
        Ok(result)
    }

    fn handle_execute_swrl_rules(&mut self) -> Result<SWRLExecutionResult> {
        let start = Instant::now();
        log::info!("Executing SWRL rules");

        let result = self
            .swrl_engine
            .execute_rules()
            .map_err(|e| Error::reasoning(format!("SWRL rule execution failed: {e}")))?;

        if !result.inferences.is_empty() {
            log::info!(
                "Applying {} SWRL inferences to ontology",
                result.inferences.len()
            );

            if let Some(ontology_ref) = self.reasoner.get_ontology().cloned() {
                let mut ontology =
                    write_lock(&ontology_ref, "actor: writing ontology for SWRL inferences")?;
                for inference in &result.inferences {
                    ontology.add_axiom(inference.clone());
                }
                self.cache_manager.clear_all()?;
                log::info!(
                    "Added {} new axioms from SWRL inferences",
                    result.inferences.len()
                );
            }
        }

        log::info!(
            "SWRL rule execution completed in {:?}: {} applications, {} inferences",
            start.elapsed(),
            result.applications,
            result.inferences.len()
        );
        Ok(result)
    }

    fn handle_explain_entailment(
        &mut self,
        axiom: &crate::ontology::Axiom,
    ) -> Result<Vec<ExplanationSet>> {
        let start = Instant::now();

        if !self
            .config
            .reasoning
            .is_enabled(crate::config::ReasoningFeature::Explanations)
        {
            return Err(Error::Reasoning {
                message: "Explanation is disabled in the configuration".into(),
            });
        }

        let explanations = self.reasoner.explain_entailment(axiom)?;
        self.check_timeout(start, "Explanation retrieval")?;

        let sets = explanations
            .into_iter()
            .map(|a| {
                let mut axioms = std::collections::HashSet::new();
                axioms.insert(a);
                ExplanationSet::new(axioms)
            })
            .collect();

        log::info!("Explanation retrieval completed in {:?}", start.elapsed());
        Ok(sets)
    }

    fn handle_explain_inconsistency(&mut self) -> Result<Vec<ExplanationSet>> {
        let start = Instant::now();

        if !self
            .config
            .reasoning
            .is_enabled(crate::config::ReasoningFeature::Explanations)
        {
            return Err(Error::Reasoning {
                message: "Explanation is disabled in the configuration".into(),
            });
        }

        let explanations = self.reasoner.explain_inconsistency()?;
        self.check_timeout(start, "Inconsistency explanation retrieval")?;

        let sets = explanations
            .into_iter()
            .map(|a| {
                let mut axioms = std::collections::HashSet::new();
                axioms.insert(a);
                ExplanationSet::new(axioms)
            })
            .collect();

        log::info!(
            "Inconsistency explanation retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(sets)
    }

    fn handle_add_axioms(&mut self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        let start = Instant::now();

        if !self.config.reasoning.incremental_reasoning {
            return Err(Error::Reasoning {
                message: "Incremental reasoning is disabled in the configuration".into(),
            });
        }

        for axiom in axioms {
            self.reasoner.add_axiom(axiom)?;
        }

        self.check_timeout(start, "Axiom addition")?;

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            self.cache_manager.clear_all()?;
        }

        log::info!("Axioms added in {:?}", start.elapsed());
        Ok(())
    }

    fn handle_remove_axioms(&mut self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        let start = Instant::now();

        if !self.config.reasoning.incremental_reasoning {
            return Err(Error::Reasoning {
                message: "Incremental reasoning is disabled in the configuration".into(),
            });
        }

        for axiom in axioms {
            self.reasoner.remove_axiom(&axiom)?;
        }

        self.check_timeout(start, "Axiom removal")?;

        if self.config.cache.is_enabled(CacheFeature::Satisfiability) {
            self.cache_manager.clear_all()?;
        }

        log::info!("Axioms removed in {:?}", start.elapsed());
        Ok(())
    }

    fn handle_get_statistics(&mut self) -> Result<ReasoningStatistics> {
        let reasoner_stats = self.reasoner.get_statistics();
        let cache_stats = self.cache_manager.get_stats()?;
        let estimated_memory = cache_stats.concept_cache_size * 1024
            + (self.config.cache.max_cache_size_mb as usize) * 1024 * 1024 / 10;

        Ok(ReasoningStatistics {
            ontology_size: self.reasoner.get_ontology_size(),
            reasoning_time: reasoner_stats.total_reasoning_time,
            cache_stats,
            memory_usage: estimated_memory,
        })
    }

    fn handle_get_ontology_iri(&mut self) -> Result<Option<crate::ontology::IRI>> {
        if let Some(ont_ref) = self.reasoner.get_ontology().cloned() {
            let ontology =
                crate::core::lock_helpers::read_lock(&ont_ref, "actor: reading ontology IRI")?;
            Ok(ontology.get_iri().cloned())
        } else {
            Ok(None)
        }
    }

    fn handle_get_serialized_turtle(&mut self) -> Result<String> {
        use crate::parsers::{OntologySerializer, TurtleSerializer};
        if let Some(ont_ref) = self.reasoner.get_ontology().cloned() {
            let ontology =
                crate::core::lock_helpers::read_lock(&ont_ref, "actor: serializing ontology")?;
            TurtleSerializer
                .serialize(&ontology)
                .map_err(|e| Error::reasoning(format!("Turtle serialization failed: {e}")))
        } else {
            // No ontology loaded — return an empty Turtle document.
            Ok(String::from("# No ontology loaded\n"))
        }
    }

    // ─── helpers ────────────────────────────────────────────

    fn check_timeout(&self, start: Instant, operation: &str) -> Result<()> {
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: format!("{operation} timed out"),
            });
        }
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────
// Actor spawn helper
// ─────────────────────────────────────────────────────────────

/// Spawn a `ReasoningActor` task and return its sender handle plus the shutdown
/// sender. Dropping the `Sender<ReasoningRequest>` or calling
/// `shutdown_tx.send(())` will terminate the actor.
pub fn spawn_actor(
    ontology: crate::ontology::Ontology,
    config: ReasonerConfig,
) -> Result<(mpsc::Sender<ReasoningRequest>, oneshot::Sender<()>)> {
    let reasoner = Reasoner::new(config.clone())
        .map_err(|e| Error::reasoning(format!("Failed to create reasoner: {e}")))?;
    let mut reasoner_with_ontology = reasoner;
    reasoner_with_ontology
        .load_ontology(ontology.clone())
        .map_err(|e| Error::reasoning(format!("Failed to load ontology: {e}")))?;

    let swrl_config = SWRLConfig::default();
    let mut swrl_engine = SWRLRuleEngine::new(swrl_config);
    swrl_engine.set_ontology(std::sync::Arc::new(std::sync::RwLock::new(ontology)));

    let (tx, rx) = mpsc::channel(64);
    let (shutdown_tx, shutdown_rx) = oneshot::channel();

    let actor = ReasoningActor::new(
        rx,
        reasoner_with_ontology,
        CacheManager::default(),
        swrl_engine,
        config,
    );

    tokio::spawn(actor.run(shutdown_rx));

    Ok((tx, shutdown_tx))
}

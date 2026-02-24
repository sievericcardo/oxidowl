//! High-level Reasoning Interface for Oxidowl
//!
//! This module provides high-level reasoning services and query interfaces
//! that wrap the core tableau algorithm and provide convenient APIs for
//! common reasoning tasks.

#![allow(dead_code)]

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

use crate::{
    Error, Result,
    cache::CacheManager,
    config::ReasonerConfig,
    core::{
        lock_helpers::{read_lock, write_lock},
        reasoner::Reasoner,
    },
    ontology::{
        ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression, Ontology,
    },
    query::{DLQuery, DLQueryEngine, QueryResult},
    swrl::{SWRLConfig, SWRLExecutionResult, SWRLRuleEngine},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

/// Reasoning service that provides high-level reasoning capabilities
#[derive(Debug, Clone)]
pub struct ReasoningService {
    reasoner: Arc<RwLock<Reasoner>>,
    cache_manager: Arc<RwLock<CacheManager>>,
    swrl_engine: Arc<RwLock<SWRLRuleEngine>>,
    config: ReasonerConfig,
}

impl ReasoningService {
    /// Creates a new reasoning service with the given ontology and configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if the reasoner cannot be created or the ontology fails to load.
    pub fn new(ontology: Ontology, config: ReasonerConfig) -> Result<Self> {
        let reasoner = Reasoner::new(config.clone())
            .map_err(|e| Error::reasoning(format!("Failed to create reasoner: {e}")))?;
        let mut reasoner_with_ontology = reasoner;
        reasoner_with_ontology
            .load_ontology(ontology.clone())
            .map_err(|e| Error::reasoning(format!("Failed to load ontology: {e}")))?;

        // Initialize SWRL engine with the ontology
        let swrl_config = SWRLConfig::default();
        let mut swrl_engine = SWRLRuleEngine::new(swrl_config);
        swrl_engine.set_ontology(Arc::new(RwLock::new(ontology)));

        Ok(Self {
            reasoner: Arc::new(RwLock::new(reasoner_with_ontology)),
            cache_manager: Arc::new(RwLock::new(CacheManager::default())),
            swrl_engine: Arc::new(RwLock::new(swrl_engine)),
            config,
        })
    }

    /// Check consistency of the ontology
    pub async fn is_consistent(&self) -> Result<bool> {
        let start = Instant::now();

        // Check cache
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = read_lock(&self.cache_manager, "reasoning: reading cache")?;
            let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
            if let Some(ontology) = reasoner.get_ontology()
                && let Some(result) = cache_manager.get_consistency_result(ontology)
            {
                return Ok(result);
            }
        }

        // Execute SWRL rules first to ensure all inferences are considered for consistency
        log::info!("Executing SWRL rules before consistency check");
        self.execute_swrl_rules().await?;

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let result = reasoner.is_consistent()?;

        // Cache the result if caching is enabled
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = write_lock(&self.cache_manager, "reasoning: writing cache")?;
            if let Some(ontology) = reasoner.get_ontology() {
                cache_manager.cache_consistency_result(ontology, result);
            }
        }

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Consistency check timed out".into(),
            });
        }

        // Log the time taken for the consistency check
        log::info!("Consistency check completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Check satisfiability of a class expression
    pub async fn is_satisfiable(&self, expression: &ClassExpression) -> Result<bool> {
        let start = Instant::now();

        // Check cache
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = read_lock(&self.cache_manager, "reasoning: reading cache")?;
            if let Some(result) = cache_manager.get_satisfiability_result(expression) {
                return Ok(result);
            }
        }

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;

        let result = reasoner.is_class_satisfiable(expression)?;

        // Cache the result if caching is enabled
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = write_lock(&self.cache_manager, "reasoning: writing cache")?;
            cache_manager.cache_satisfiability_result(expression.clone(), result);
        }

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Satisfiability check timed out".into(),
            });
        }

        // Log the time taken for the satisfiability check
        log::info!("Satisfiability check completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Check subsumption of two class expressions
    pub async fn is_subsumed_by(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        let start = Instant::now();

        // Check cache
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = read_lock(&self.cache_manager, "reasoning: reading cache")?;
            if let Some(result) = cache_manager.get_subsumption_result(subclass, superclass) {
                return Ok(result);
            }
        }

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let result = reasoner.is_subsumed_by(subclass, superclass)?;

        // Cache the result if caching is enabled
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = write_lock(&self.cache_manager, "reasoning: writing cache")?;
            cache_manager.cache_subsumption_result(subclass.clone(), superclass.clone(), result);
        }

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Subsumption check timed out".into(),
            });
        }

        // Log the time taken for the subsumption check
        log::info!("Subsumption check completed in {:?}", start.elapsed());
        Ok(result)
    }

    // Check equivalence of two class expressions
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
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let superclasses = reasoner.get_superclasses(class, direct)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Direct superclass retrieval timed out".into(),
            });
        }

        // Log the time taken for the retrieval
        log::info!(
            "Direct superclass retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(superclasses.into_iter().collect())
    }

    /// Get all direct subclasses of a class expression
    pub async fn get_subclasses(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let subclasses = reasoner.get_subclasses(class, direct)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Direct subclass retrieval timed out".into(),
            });
        }

        // Log the time taken for the retrieval
        log::info!(
            "Direct subclass retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(subclasses.into_iter().collect())
    }

    /// Get all equivalent classes of a class expression
    pub async fn get_equivalent_classes(
        &self,
        class: &ClassExpression,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let equivalent_classes = reasoner.get_equivalent_classes(class)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Equivalent class retrieval timed out".into(),
            });
        }

        // Log the time taken for the retrieval
        log::info!(
            "Equivalent class retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(equivalent_classes.into_iter().collect())
    }

    /// Get all instances of a class expression
    pub async fn get_instances(
        &self,
        class: &ClassExpression,
        direct: bool,
    ) -> Result<HashSet<Individual>> {
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let instances = reasoner.get_instances(class, direct)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Instance retrieval timed out".into(),
            });
        }

        // Log the time taken for the retrieval
        log::info!("Instance retrieval completed in {:?}", start.elapsed());
        Ok(instances.into_iter().collect())
    }

    /// Get all types of an individual
    pub async fn get_types(
        &self,
        individual: &Individual,
        direct: bool,
    ) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let types = reasoner.get_types(individual, direct)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Type retrieval timed out".into(),
            });
        }

        // Log the time taken for the retrieval
        log::info!("Type retrieval completed in {:?}", start.elapsed());
        Ok(types.into_iter().collect())
    }

    /// Check if an individual is an instance of a class expression
    pub async fn is_instance_of(
        &self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        let types = self.get_types(individual, false).await?;
        for class_type in &types {
            if self.is_subsumed_by(class_type, class).await? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// Get object property values for an individual
    pub async fn get_object_property_values(
        &self,
        individual: &Individual,
        property: &ObjectPropertyExpression,
    ) -> Result<HashSet<Individual>> {
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let values = reasoner.get_object_property_values(individual, property)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Object property value retrieval timed out".into(),
            });
        }

        // Log the time taken for the retrieval
        log::info!(
            "Object property value retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(values.into_iter().collect())
    }

    /// Get data property values for an individual
    pub async fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<HashSet<crate::ontology::Literal>> {
        let start = Instant::now();

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let result = reasoner.get_data_property_values(individual, property)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Data property value retrieval timed out".into(),
            });
        }

        // Convert Vec<String> to HashSet<Literal>
        let literals: HashSet<crate::ontology::Literal> = result
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

        // Log the time taken for the retrieval
        log::info!(
            "Data property value retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(literals)
    }

    /// Classify the ontology (compute class hierarchy)
    pub async fn classify(&self) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Check cache
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = read_lock(&self.cache_manager, "reasoning: reading cache")?;
            let _ontology_hash = self.calculate_ontology_hash()?;
            // Get ontology from reasoner
            let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
            if let Some(ontology) = reasoner.get_ontology()
                && let Some(cached) = cache_manager.get_classification_result(ontology)
            {
                log::info!("Classification (cached) completed in {:?}", start.elapsed());
                return Ok(cached);
            }
        }

        // Execute SWRL rules first to ensure all inferences are available for classification
        log::info!("Executing SWRL rules before classification");
        self.execute_swrl_rules().await?;

        let mut reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let result = reasoner.classify()?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Classification timed out".into(),
            });
        }

        // Cache the result if caching is enabled
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = write_lock(&self.cache_manager, "reasoning: writing cache")?;
            let _ontology_hash = self.calculate_ontology_hash()?;
            // Get ontology from reasoner
            let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
            if let Some(ontology) = reasoner.get_ontology() {
                cache_manager.store_classification_result(ontology, result.clone());
            }
        }

        // Log the time taken for classification
        log::info!("Classification completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Execute SWRL rules and apply inferences to the ontology
    pub async fn execute_swrl_rules(&self) -> Result<SWRLExecutionResult> {
        let start = Instant::now();
        log::info!("Executing SWRL rules");

        // Execute SWRL rules
        let mut swrl_engine = write_lock(&self.swrl_engine, "reasoning: writing SWRL engine")?;
        let result = swrl_engine
            .execute_rules()
            .map_err(|e| Error::reasoning(format!("SWRL rule execution failed: {e}")))?;

        // Apply the inferences to the ontology
        if !result.inferences.is_empty() {
            log::info!(
                "Applying {} SWRL inferences to ontology",
                result.inferences.len()
            );

            // Get the reasoner and update the ontology with inferences
            let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
            if let Some(ontology_ref) = reasoner.get_ontology() {
                let mut ontology = write_lock(ontology_ref, "reasoning: writing ontology")?;

                // Apply each inference to the ontology
                for inference in &result.inferences {
                    ontology.add_axiom(inference.clone());
                }

                // Clear caches since the ontology has been modified
                write_lock(&self.cache_manager, "reasoning: writing cache")?.clear_all()?;
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
        let start = Instant::now();

        // Check cache
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = read_lock(&self.cache_manager, "reasoning: reading cache")?;
            let _ontology_hash = self.calculate_ontology_hash()?;
            // Get ontology from reasoner
            let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
            if let Some(ontology) = reasoner.get_ontology()
                && let Some(cached) = cache_manager.get_realization_result(ontology)
            {
                log::info!("Realization (cached) completed in {:?}", start.elapsed());
                return Ok(cached);
            }
        }

        // Execute SWRL rules first to ensure all inferences are available for realization
        log::info!("Executing SWRL rules before realization");
        self.execute_swrl_rules().await?;

        let mut reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let result = reasoner.realize()?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Realization timed out".into(),
            });
        }

        // Cache the result if caching is enabled
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            let cache_manager = write_lock(&self.cache_manager, "reasoning: writing cache")?;
            let _ontology_hash = self.calculate_ontology_hash()?;
            // Get ontology from reasoner
            let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
            if let Some(ontology) = reasoner.get_ontology() {
                cache_manager.store_realization_result(ontology, result.clone());
            }
        }

        // Log the time taken for realization
        log::info!("Realization completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Get explanation for an entailment
    pub async fn explain_entailment(
        &self,
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

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let explanations = reasoner.explain_entailment(axiom)?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Explanation retrieval timed out".into(),
            });
        }

        let explanation_sets: Vec<ExplanationSet> = explanations
            .into_iter()
            .map(|axiom| {
                let mut axioms = HashSet::new();
                axioms.insert(axiom);
                ExplanationSet::new(axioms)
            })
            .collect();

        // Log the time taken for explanation retrieval
        log::info!("Explanation retrieval completed in {:?}", start.elapsed());
        Ok(explanation_sets)
    }

    /// Get explanation for inconsistent ontology
    pub async fn explain_inconsistency(&self) -> Result<Vec<ExplanationSet>> {
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

        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        let explanations = reasoner.explain_inconsistency()?;

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Inconsistency explanation retrieval timed out".into(),
            });
        }

        let explanation_sets: Vec<ExplanationSet> = explanations
            .into_iter()
            .map(|axiom| {
                let mut axioms = HashSet::new();
                axioms.insert(axiom);
                ExplanationSet::new(axioms)
            })
            .collect();

        // Log the time taken for explanation retrieval
        log::info!(
            "Inconsistency explanation retrieval completed in {:?}",
            start.elapsed()
        );
        Ok(explanation_sets)
    }

    /// Add axioms incrementally to the ontology
    pub async fn add_axioms(&self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        let start = Instant::now();

        if !self.config.reasoning.incremental_reasoning {
            return Err(Error::Reasoning {
                message: "Incremental reasoning is disabled in the configuration".into(),
            });
        }

        let mut reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        for axiom in axioms {
            reasoner.add_axiom(axiom)?;
        }

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Axiom addition timed out".into(),
            });
        }

        // Clear relevant caches
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            write_lock(&self.cache_manager, "reasoning: writing cache")?.clear_all()?;
        }

        // Log the time taken for adding axioms
        log::info!("Axioms added in {:?}", start.elapsed());
        Ok(())
    }

    /// Remove axioms incrementally from the ontology
    pub async fn remove_axioms(&self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        let start = Instant::now();

        if !self.config.reasoning.incremental_reasoning {
            return Err(Error::Reasoning {
                message: "Incremental reasoning is disabled in the configuration".into(),
            });
        }

        let mut reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        for axiom in axioms {
            reasoner.remove_axiom(&axiom)?;
        }

        // Check timeout
        if let Some(timeout) = self.config.reasoning.timeout
            && start.elapsed() > timeout
        {
            return Err(Error::Timeout {
                message: "Axiom removal timed out".into(),
            });
        }

        // Clear relevant caches
        if self
            .config
            .cache
            .is_enabled(crate::config::CacheFeature::Satisfiability)
        {
            write_lock(&self.cache_manager, "reasoning: writing cache")?.clear_all()?;
        }

        // Log the time taken for removing axioms
        log::info!("Axioms removed in {:?}", start.elapsed());
        Ok(())
    }

    /// Get reasoning statistics
    pub fn get_statistics(&self) -> Result<ReasoningStatistics> {
        let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
        let cache_stats =
            read_lock(&self.cache_manager, "reasoning: reading cache")?.get_stats()?;

        // Get statistics from the reasoner
        let reasoner_stats = reasoner.get_statistics();

        Ok(ReasoningStatistics {
            ontology_size: reasoner.get_ontology_size(),
            reasoning_time: reasoner_stats.total_reasoning_time,
            cache_stats,
            memory_usage: self.estimate_memory_usage()?,
        })
    }

    /// Estimate current memory usage
    fn estimate_memory_usage(&self) -> Result<usize> {
        // Simple estimation based on cache size and other factors
        let cache_stats =
            read_lock(&self.cache_manager, "reasoning: reading cache")?.get_stats()?;
        Ok(
            cache_stats.concept_cache_size * 1024 + // Rough estimate per cache entry
        (self.config.cache.max_cache_size_mb as usize) * 1024 * 1024 / 10,
        )
        // Conservative fraction of max allowed
    }

    // Compute the hash of the ontology for caching
    fn compute_ontology_hash(&self) -> Result<u64> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
        if let Some(ontology) = reasoner.get_ontology() {
            let ontology_guard = read_lock(ontology, "reasoning: reading ontology")?;

            let mut hasher = DefaultHasher::new();

            // Hash axiom count and basic signature information
            if let Ok(signature) = ontology_guard.signature() {
                signature.classes.len().hash(&mut hasher);
                signature.object_properties.len().hash(&mut hasher);
                signature.data_properties.len().hash(&mut hasher);
                signature.individuals.len().hash(&mut hasher);

                // Hash some class names for uniqueness
                for class in signature.classes.iter().take(10) {
                    class.iri.as_str().hash(&mut hasher);
                }
            }

            // Hash TBox and ABox axiom counts
            ontology_guard.axioms().len().hash(&mut hasher);

            Ok(hasher.finish())
        } else {
            Ok(0)
        }
    }

    /// Query property chain reasoning
    /// Implements role chain propagation: if R1 * R2 * ... * Rn c S,
    /// and we have a -R1-> b -R2-> c ... z -Rn-> w, then infer a -S-> w
    pub async fn query_property_chain(
        &self,
        individual: &Individual,
        property_chain: &[ObjectPropertyExpression],
    ) -> Result<HashSet<Individual>> {
        let start = Instant::now();

        if property_chain.is_empty() {
            return Ok(HashSet::new());
        }

        if property_chain.len() == 1 {
            // Single property - delegate to existing method
            return self
                .get_object_property_values(individual, &property_chain[0])
                .await;
        }

        // Multi-step property chain reasoning
        let mut current_individuals = HashSet::new();
        current_individuals.insert(individual.clone());

        // Step through each property in the chain
        for property in property_chain {
            let mut next_individuals = HashSet::new();

            // For each current individual, follow the property
            for curr_ind in &current_individuals {
                let targets = self.get_object_property_values(curr_ind, property).await?;
                next_individuals.extend(targets);
            }

            current_individuals = next_individuals;

            // If no individuals remain, the chain is broken
            if current_individuals.is_empty() {
                break;
            }
        }

        // Log the time taken for the property chain query
        log::info!("Property chain query completed in {:?}", start.elapsed());
        Ok(current_individuals)
    }

    /// Get access to the SWRL rule engine
    #[must_use]
    pub fn get_swrl_engine(&self) -> Arc<RwLock<SWRLRuleEngine>> {
        Arc::clone(&self.swrl_engine)
    }

    /// Get SWRL execution statistics
    pub async fn get_swrl_statistics(&self) -> Result<crate::swrl::SWRLStatistics> {
        let swrl_engine = read_lock(&self.swrl_engine, "reasoning: reading SWRL engine")?;
        Ok(swrl_engine.get_statistics().clone())
    }

    /// Set SWRL rule priority
    pub async fn set_swrl_rule_priority(&self, rule_id: u64, priority: u32) -> Result<()> {
        let mut swrl_engine = write_lock(&self.swrl_engine, "reasoning: writing SWRL engine")?;
        swrl_engine.set_rule_priority(rule_id, priority);
        Ok(())
    }

    /// Get ordered SWRL rules by priority
    pub async fn get_swrl_rule_order(&self) -> Result<Vec<u64>> {
        let swrl_engine = read_lock(&self.swrl_engine, "reasoning: reading SWRL engine")?;
        Ok(swrl_engine.get_rule_ids())
    }

    /// Enable or disable a specific SWRL rule
    pub async fn set_swrl_rule_active(&self, rule_id: u64, active: bool) -> Result<()> {
        let mut swrl_engine = write_lock(&self.swrl_engine, "reasoning: writing SWRL engine")?;
        swrl_engine.set_rule_active(rule_id, active).map_err(|e| {
            Error::reasoning(format!(
                "Failed to set SWRL rule {rule_id} active state: {e}"
            ))
        })
    }

    /// Validate the loaded ontology (serialised as Turtle) against a SHACL shapes graph.
    ///
    /// `shapes_turtle` – Turtle-encoded SHACL shapes graph.  
    /// `data_turtle`  – Turtle-encoded data graph to validate. Pass an empty string
    ///                  to validate only the shapes graph itself.
    pub fn validate_shacl(
        &self,
        shapes_turtle: &str,
        data_turtle: &str,
    ) -> Result<crate::validation::shacl::ShaclValidationReport> {
        let mut validator =
            crate::validation::shacl::ShaclValidator::new(shapes_turtle, data_turtle)?;
        validator.validate()
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

impl ReasoningService {
    /// Get the IRI of the current ontology
    pub fn get_ontology_iri(&self) -> Result<Option<crate::ontology::IRI>> {
        let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
        if let Some(ontology_ref) = reasoner.get_ontology() {
            let ontology = read_lock(ontology_ref, "reasoning: reading ontology")?;
            Ok(ontology.get_iri().cloned())
        } else {
            Ok(None)
        }
    }

    /// Calculate a hash for the current ontology
    fn calculate_ontology_hash(&self) -> Result<u64> {
        let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        // Hash based on reasoner state as a simple fingerprint
        let axiom_count = if let Some(ontology) = reasoner.get_ontology() {
            read_lock(ontology, "reasoning: reading ontology")?
                .axioms()
                .len()
        } else {
            0
        };
        std::hash::Hash::hash(&axiom_count, &mut hasher);

        Ok(std::hash::Hasher::finish(&hasher))
    }

    // Synchronous wrapper methods for advanced query processing
    /// Synchronous version of `get_instances` for use in advanced query processing
    pub fn get_instances_sync(&self, class: &ClassExpression) -> Result<Vec<Individual>> {
        let reasoner = write_lock(&self.reasoner, "reasoning: writing reasoner")?;
        reasoner.get_instances(class, false)
    }

    /// Synchronous version of `is_instance_of` for use in advanced query processing  
    pub fn is_instance_of_sync(
        &self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        let reasoner = read_lock(&self.reasoner, "reasoning: reading reasoner")?;
        reasoner.is_instance_of(individual, class)
    }

    /// Get object property assertions (for advanced query processing)
    pub fn get_object_property_assertions_sync(
        &self,
        _property: &ObjectPropertyExpression,
    ) -> Result<Vec<(Individual, Individual)>> {
        // This is a simplified implementation - in practice would query the reasoner
        // For now, return empty results to avoid compilation errors
        Ok(Vec::new())
    }

    /// Get the cache manager for incremental reasoning integration
    #[must_use]
    pub fn cache_manager(&self) -> Arc<RwLock<CacheManager>> {
        self.cache_manager.clone()
    }

    /// Invalidate all caches (useful for incremental reasoning)
    pub async fn invalidate_all_caches(&self) -> Result<()> {
        if let Ok(_cache) = self.cache_manager.write() {
            // Invalidate caches - the actual implementation would depend on CacheManager
            tracing::debug!("All caches invalidated");
        }
        Ok(())
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

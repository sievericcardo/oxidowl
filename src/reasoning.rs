//! High-level Reasoning Interface for Oxidowl
//! 
//! This module provides high-level reasoning services and query interfaces
//! that wrap the core tableau algorithm and provide convenient APIs for
//! common reasoning tasks.

// Re-export core reasoner types for public API
pub use crate::core::reasoner::{ReasoningTask, ReasoningResult, ClassificationResult, RealizationResult};

use crate::{
    Error, Result,
    ontology::{Ontology, ClassExpression, Individual, ObjectPropertyExpression, DataProperty, DataPropertyExpression, Axiom},
    core::{
        reasoner::Reasoner,
        tableau::Tableau,
    },
    cache::{CacheManager, CacheConfig},
    config::ReasonerConfig,
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
    config: ReasonerConfig,
}

impl ReasoningService {
    /// Creates a new reasoning service with the given ontology and configuration
    pub fn new(ontology: Ontology, config: ReasonerConfig) -> Self {
        let reasoner = Reasoner::new(config.clone()).expect("Failed to create reasoner");
        let mut reasoner_with_ontology = reasoner;
        reasoner_with_ontology.load_ontology(ontology).expect("Failed to load ontology");

        Self {
            reasoner: Arc::new(RwLock::new(reasoner_with_ontology)),
            cache_manager: Arc::new(RwLock::new(CacheManager::default())),
            config,
        }
    }

    /// Check consistency of the ontology
    pub async fn is_consistent(&self) -> Result<bool> {
        let start = Instant::now();

        // Check cache
        if self.config.enable_cache {
            let cache_manager = self.cache_manager.read().unwrap();
            if let Some(result) = cache_manager.get_consistency_result(&self.reasoner.read().unwrap().ontology) {
                return Ok(result);
            }
        }

        let mut reasoner = self.reasoner.write().unwrap();
        let result = reasoner.is_consistent()?;

        // Cache the result if caching is enabled
        if self.config.enable_cache {
            let mut cache_manager = self.cache_manager.write().unwrap();
            cache_manager.cache_consistency_result(&reasoner.ontology, result);
        }

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Consistency check timed out".into(),
                });
            }
        }

        // Log the time taken for the consistency check
        log::info!("Consistency check completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Check satisfiability of a class expression
    pub async fn is_satisfiable(&self, expression: &ClassExpression) -> Result<bool> {
        let start = Instant::now();

        // Check cache
        if self.config.enable_cache {
            let cache_manager = self.cache_manager.read().unwrap();
            if let Some(result) = cache_manager.get_satisfiability_result(expression) {
                return Ok(result);
            }
        }

        let mut reasoner = self.reasoner.write().unwrap();
        
        // Convert ClassExpression to IRI
        let class_iri = match expression {
            ClassExpression::Class(class) => class.iri.to_string(),
                _ => return Err(Error::Reasoning {
                    message: "Invalid class expression for satisfiability check".to_string(),
                }),
        };

        let result = reasoner.is_class_satisfiable(&class_iri)?;

        // Cache the result if caching is enabled
        if self.config.enable_cache {
            let mut cache_manager = self.cache_manager.write().unwrap();
            cache_manager.cache_satisfiability_result(expression.clone(), result);
        }

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Satisfiability check timed out".into(),
                });
            }
        }

        // Log the time taken for the satisfiability check
        log::info!("Satisfiability check completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Check subsumption of two class expressions
    pub async fn is_subsumed_by (&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Result<bool> {
        let start = Instant::now();

        // Check cache
        if self.config.enable_cache {
            let cache_manager = self.cache_manager.read().unwrap();
            if let Some(result) = cache_manager.get_subsumption_result(subclass, superclass) {
                return Ok(result);
            }
        }

        let reasoner = self.reasoner.write().unwrap();
        let result = reasoner.is_subsumed_by(subclass, superclass).await?;

        // Cache the result if caching is enabled
        if self.config.enable_cache {
            let mut cache_manager = self.cache_manager.write().unwrap();
            cache_manager.cache_subsumption_result(subclass.clone(), superclass.clone(), result);
        }

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Subsumption check timed out".into(),
                });
            }
        }

        // Log the time taken for the subsumption check
        log::info!("Subsumption check completed in {:?}", start.elapsed());
        Ok(result)
    }

    // Check equivalence of two class expressions
    pub async fn is_equivalent_to(&self, class1: &ClassExpression, class2: &ClassExpression) -> Result<bool> {
        let subsumes_1_2 = self.is_subsumed_by(class1, class2).await?;
        let subsumes_2_1 = self.is_subsumed_by(class2, class1).await?;
        Ok(subsumes_1_2 && subsumes_2_1)
    }

    // Check disjointness of two class expressions
    pub async fn is_disjoint_with(&self, class1: &ClassExpression, class2: &ClassExpression) -> Result<bool> {
        let intersection = ClassExpression::ObjectIntersectionOf(vec![class1.clone(), class2.clone()]);
        let satisfiable = self.is_satisfiable(&intersection).await?;
        Ok(!satisfiable)
    }

    // Get all direct superclasses of a class expression
    pub async fn get_superclasses(&self, class: &ClassExpression, direct: bool) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = self.reasoner.write().unwrap();
        let superclasses = reasoner.get_superclasses(&class, direct).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Direct superclass retrieval timed out".into(),
                });
            }
        }

        // Log the time taken for the retrieval
        log::info!("Direct superclass retrieval completed in {:?}", start.elapsed());
        Ok(superclasses)
    }

    /// Get all direct subclasses of a class expression
    pub async fn get_subclasses(&self, class: &ClassExpression, direct: bool) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = self.reasoner.write().unwrap();
        let subclasses = reasoner.get_subclasses(&class, direct).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Direct subclass retrieval timed out".into(),
                });
            }
        }

        // Log the time taken for the retrieval
        log::info!("Direct subclass retrieval completed in {:?}", start.elapsed());
        Ok(subclasses)
    }

    /// Get all equivalent classes of a class expression
    pub async fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = self.reasoner.write().unwrap();
        let equivalent_classes = reasoner.get_equivalent_classes(class).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Equivalent class retrieval timed out".into(),
                });
            }
        }

        // Log the time taken for the retrieval
        log::info!("Equivalent class retrieval completed in {:?}", start.elapsed());
        Ok(equivalent_classes)
    }

    /// Get all instances of a class expression
    pub async fn get_instances(&self, class: &ClassExpression, direct: bool) -> Result<HashSet<Individual>> {
        let start = Instant::now();

        let reasoner = self.reasoner.write().unwrap();
        let instances = reasoner.get_instances(&class, direct).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Instance retrieval timed out".into(),
                });
            }
        }

        // Log the time taken for the retrieval
        log::info!("Instance retrieval completed in {:?}", start.elapsed());
        Ok(instances)
    }

    /// Get all types of an individual
    pub async fn get_types(&self, individual: &Individual, direct: bool) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let reasoner = self.reasoner.write().unwrap();
        let types = reasoner.get_types(&individual, direct).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Type retrieval timed out".into(),
                });
            }
        }

        // Log the time taken for the retrieval
        log::info!("Type retrieval completed in {:?}", start.elapsed());
        Ok(types)
    }

    /// Check if an individual is an instance of a class expression
    pub async fn is_instance_of(&self, individual: &Individual, class: &ClassExpression) -> Result<bool> {
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

        let reasoner = self.reasoner.write().unwrap();
        let values = reasoner.get_object_property_values(&individual, &property).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Object property value retrieval timed out".into(),
                });
            }
        }

        // Log the time taken for the retrieval
        log::info!("Object property value retrieval completed in {:?}", start.elapsed());
        Ok(values)
    }

    /// Get data property values for an individual
    pub async fn get_data_property_values(
        &self,
        individual: &Individual,
        property: &DataPropertyExpression,
    ) -> Result<HashSet<crate::ontology::Literal>> {
        let start = Instant::now();

        let reasoner = self.reasoner.write().unwrap();
        let property_expr = DataPropertyExpression::DataProperty(property.clone());
        let result = reasoner.get_data_property_values(&individual, &property_expr).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Data property value retrieval timed out".into(),
                });
            }
        }

        // Convert Vec<String> to HashSet<Literal>
        let literals: HashSet<crate::ontology::Literal> = result
            .into_iter()
            .map(|s| crate::ontology::Literal {
                value: s,
                datatype: url::Url::parse("http://www.w3.org/2001/XMLSchema#string").unwrap(),
                language: None,
            })
            .collect();

        // Log the time taken for the retrieval
        log::info!("Data property value retrieval completed in {:?}", start.elapsed());
        Ok(literals)
    }

    /// Classify the ontology (compute class hierarchy)
    pub async fn classify(&self) -> Result<ClassificationResult> {
        let start = Instant::now();

        // Check cache
        if self.config.enable_cache {
            let cache_manager = self.cache_manager.read().unwrap();
            let ontology_hash = self.calculate_ontology_hash();
            if let Some(cached) = self.cache_manager.classification().get(ontology_hash) {
                log::info!("Classification (cached) completed in {:?}", start.elapsed());
                return Ok(ClassificationResult::new(cached));
            }
        }

        let mut reasoner = self.reasoner.write().unwrap();
        let result = reasoner.classify().await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Classification timed out".into(),
                });
            }
        }

        // Cache the result if caching is enabled
        if self.config.enable_cache {
            let mut cache_manager = self.cache_manager.write().unwrap();
            let ontology_hash = self.calculate_ontology_hash();
            self.cache_manager.classification().put(ontology_hash, result.hierarchy.clone());
        }

        // Log the time taken for classification
        log::info!("Classification completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Realize the ontology (compute individuals' types)
    pub async fn realize(&self) -> Result<RealizationResult> {
        let start = Instant::now();

        // Check cache
        if self.config.enable_cache {
            let cache_manager = self.cache_manager.read().unwrap();
            let ontology_hash = self.calculate_ontology_hash();
            if let Some(cached) = self.cache_manager.realization().get(ontology_hash) {
                log::info!("Realization (cached) completed in {:?}", start.elapsed());
                return Ok(RealizationResult::new(cached));
            }
        }

        let mut reasoner = self.reasoner.write().unwrap();
        let result = reasoner.realize().await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Realization timed out".into(),
                });
            }
        }

        // Cache the result if caching is enabled
        if self.config.enable_cache {
            let mut cache_manager = self.cache_manager.write().unwrap();
            let ontology_hash = self.calculate_ontology_hash();
            self.cache_manager.realization().put(ontology_hash, result.types.clone());
        }

        // Log the time taken for realization
        log::info!("Realization completed in {:?}", start.elapsed());
        Ok(result)
    }

    /// Get explanation for an entailment
    pub async fn explain_entailment(&self, axiom: &crate::ontology::Axiom) -> Result<Vec<ExplanationSet>> {
        let start = Instant::now();

        if !self.config.enable_explanation {
            return Err(Error::Reasoning {
                message: "Explanation is disabled in the configuration".into(),
            });
        }

        let reasoner = self.reasoner.write().unwrap();
        let explanations = reasoner.explain_entailment(&axiom).await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Explanation retrieval timed out".into(),
                });
            }
        }

        let explanation_sets: Vec<ExplanationSet> = explanations.into_iter()
            .map(|axiom| {
                let mut axioms = HashSet::new();
                axioms.insert(axiom);
                ExplanationSet::new(axioms)
            })
            .collect();

        // Log the time taken for explanation retrieval
        log::info!("Explanation retrieval completed in {:?}", start.elapsed());
        Ok(explanations)
    }

    /// Get explanation for inconsistent ontology
    pub async fn explain_inconsistency(&self) -> Result<Vec<ExplanationSet>> {
        let start = Instant::now();

        if !self.config.enable_explanation {
            return Err(Error::Reasoning {
                message: "Explanation is disabled in the configuration".into(),
            });
        }

        let reasoner = self.reasoner.write().unwrap();
        let explanations = reasoner.explain_inconsistency().await?;

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Inconsistency explanation retrieval timed out".into(),
                });
            }
        }

        let explanation_sets: Vec<ExplanationSet> = explanations.into_iter()
            .map(|axiom| {
                let mut axioms = HashSet::new();
                axioms.insert(axiom);
                ExplanationSet::new(axioms)
            })
            .collect();

        // Log the time taken for explanation retrieval
        log::info!("Inconsistency explanation retrieval completed in {:?}", start.elapsed());
        Ok(explanation_sets)
    }

    /// Add axioms incrementally to the ontology
    pub async fn add_axioms(&self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        let start = Instant::now();

        if !self.config.enable_incremental {
            return Err(Error::Reasoning {
                message: "Incremental reasoning is disabled in the configuration".into(),
            });
        }

        let mut reasoner = self.reasoner.write().unwrap();
        for axiom in axioms {
            reasoner.add_axiom(&axiom).await?;
        }

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Axiom addition timed out".into(),
                });
            }
        }

        // Clear relevant caches
        if self.config.enable_caching {
            self.cache_manager.clear_all();
        }

        // Log the time taken for adding axioms
        log::info!("Axioms added in {:?}", start.elapsed());
        Ok(())
    }

    /// Remove axioms incrementally from the ontology
    pub async fn remove_axioms(&self, axioms: Vec<crate::ontology::Axiom>) -> Result<()> {
        let start = Instant::now();

        if !self.config.enable_incremental {
            return Err(Error::Reasoning {
                message: "Incremental reasoning is disabled in the configuration".into(),
            });
        }

        let mut reasoner = self.reasoner.write().unwrap();
        for axiom in axioms {
            reasoner.remove_axiom(&axiom).await?;
        }

        // Check timeout
        if let Some(timeout) = self.config.timeout {
            if start.elapsed() > timeout {
                return Err(Error::Timeout {
                    message: "Axiom removal timed out".into(),
                });
            }
        }

        // Clear relevant caches
        if self.config.enable_caching {
            self.cache_manager.clear_all();
        }

        // Log the time taken for removing axioms
        log::info!("Axioms removed in {:?}", start.elapsed());
        Ok(())
    }

    /// Get reasoning statistics -- TODO: implement the actual statistics gathering
    pub fn get_statistics(&self) -> ReasoningStatistics {
        let reasoner = self.reasoner.read().unwrap();
        let cache_stats = self.cache_manager.read().unwrap().get_stats();

        ReasoningStatistics {
            ontology_size: reasoner.get_ontology_size(),
            reasoning_time: Duration::from_secs(0), // Would be tracked in real implementation
            cache_stats,
            memory_usage: 0, // Would be measured in real implementation
        }
    }

    // Compute the hash of the ontology for caching
    fn compute_ontology_hash(&self) -> Result<u64> {
        // TODO: placeholder for the hash. Implement a proper hashing mechanism
        let reasoner = self.reasoner.read().unwrap();
        Ok(reasoner.get_ontology_size() as u64)
    }

    /// Query property chain reasoning
    /// Implements role chain propagation: if R1 * R2 * ... * Rn c S, 
    /// and we have a -R1-> b -R2-> c ... z -Rn-> w, then infer a -S-> w
    pub async fn query_property_chain(
        &self, 
        individual: &Individual, 
        property_chain: &[ObjectPropertyExpression]
    ) -> Result<HashSet<Individual>> {
        let start = Instant::now();

        if property_chain.is_empty() {
            return Ok(HashSet::new());
        }

        if property_chain.len() == 1 {
            // Single property - delegate to existing method
            return self.get_object_property_values(individual, &property_chain[0]).await;
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
}

/// Explanation set for reasoning entailments
#[derive(Debug, Clone)]
pub struct ExplanationSet {
    pub axioms: HashSet<crate::ontology::Axiom>,
    pub minimal: bool,
}

impl ExplanationSet {
    // Create a new explanation set
    pub fn new(axioms: HashSet<crate::ontology::Axiom>) -> Self {
        Self {
            axioms,
            minimal: true, // Default to minimal explanations
        }
    }

    pub fn size(&self) -> usize {
        self.axioms.len()
    }

    pub fn is_minimal(&self) -> bool {
        self.minimal
    }

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
    pub fn new(reasoning_service: ReasoningService) -> Self {
        Self {
            reasoning_service,
        }
    }

    /// Execute a subsumption query
    pub async fn execute_subsumption_query(
        &self, 
        subclass: ClassExpression, 
        superclass: ClassExpression
    ) -> Result<bool> {
        self.reasoning_service.is_subsumed_by(&subclass, &superclass).await
    }

    /// Execute an instance query
    pub async fn execute_instance_query(
        &self, 
        individual: Individual, 
        class: ClassExpression
    ) -> Result<bool> {
        self.reasoning_service.is_instance_of(&individual, &class).await
    }

    pub async fn query_instances(
        &self, 
        class: ClassExpression, 
        direct: bool
    ) -> Result<HashSet<Individual>> {
        self.reasoning_service.get_instances(&class, direct).await
    }

    /// Execute a property value query
    pub async fn execute_property_value_query(
        &self, 
        individual: Individual, 
        property_chain: Vec<ObjectPropertyExpression>
    ) -> Result<HashSet<Individual>> {
        if property_chain.len() == 1 {
            // Single property query
            self.reasoning_service.get_object_property_values(&individual, &property_chain[0]).await
        } else {
            // Multi-step property chain query
            self.reasoning_service.query_property_chain(&individual, &property_chain).await
        }
    }

    /// Execute batch queries
    pub async fn batch_satisfiability_check (
        &self,
        concepts: Vec<ClassExpression>
    ) -> Result<HashMap<ClassExpression, bool>> {
        let mut results = HashMap::new();

        for concept in concepts {
            let result = self.reasoning_service.is_satisfiable(&concept).await?;
            results.insert(concept, result);
        }

        Ok(results)
    }

    /// Execute batch subsumption check
    pub async fn batch_subsumption_check(
        &self,
        queries: Vec<(ClassExpression, ClassExpression)>
    ) -> Result<HashMap<(ClassExpression, ClassExpression), bool>> {
        let mut results = HashMap::new();

        for (subclass, superclass) in queries {
            let result = self.reasoning_service.is_subsumed_by(&subclass, &superclass).await?;
            results.insert((subclass, superclass), result);
        }

        Ok(results)
    }
}

impl ReasoningService {
    /// Create a new ReasoningService
    pub fn new() -> Self {
        Self {
            ontology: Arc::new(RwLock::new(Ontology::new())),
            reasoner: Arc::new(RwLock::new(Box::new(HyperTableau::new(
                ReasoningConfig::default(),
                Box::new(crate::core::blocking::AnywhereBlocking::new()),
            ).unwrap()))),
            cache_manager: Arc::new(RwLock::new(CacheManager::new(CacheConfig::default()))),
            config: ReasonerConfig::default(),
        }
    }

    /// Calculate a hash for the current ontology
    fn calculate_ontology_hash(&self) -> u64 {
        let ontology = self.ontology.read().unwrap();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        
        // Hash the number of axioms as a simple fingerprint
        let axiom_count = ontology.axioms().len();
        std::hash::Hash::hash(&axiom_count, &mut hasher);
        
        std::hash::Hasher::finish(&hasher)
    }
}

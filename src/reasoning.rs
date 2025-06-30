//! High-level Reasoning Interface for Oxidowl
//! 
//! This module provides high-level reasoning services and query interfaces
//! that wrap the core tableau algorithm and provide convenient APIs for
//! common reasoning tasks.

// Re-export core reasoner types for public API
pub use crate::core::reasoner::{ReasoningTask, ReasoningResult, ClassificationResult, RealizationResult};

use create::{
    Error, Result,
    ontology::{Ontology, ClassExpression, Individual, ObjectPropertyExpression, DataProperty, DataPropertyExpression, Axiom},
    core::{
        reasoner::Reasoner,
        tableau::Tableau,
    }
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::{Duration, Instant};
};

/// Reasoning service that provides high-level reasoning capabilities
#[derive(Debug, Clone)]
pub struct ReasoningService {
    reasoner: Arc<RwLock<Reasoner>>,
    cache_manager: Arc<RwLock<CacheManager>>,
    config: ReasonerConfig,
}

/// Configuration for the reasoning service
#[derive(Debug, Clone)]
pub struct ReasonerConfig {
    pub enable_cache: bool,
    val timeout: Option<Duration>,
    pub max_concurrent_tasks: usize,
    pub enable_explanation: bool,
    pub enable_incremental: bool,
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        Self {
            enable_cache: true,
            timeout: Some(Duration::from_secs(300)), // Default timeout of 5 minutes
            max_concurrent_tasks: 4,
            enable_explanation: false,
            enable_incremental: false,
        }
    }
}

impl ReasoningService {
    /// Creates a new reasoning service with the given ontology and configuration
    pub fn new(ontology: Ontology, config: ReasonerConfig) -> Self {
        let reasoner_config = crate::config::ReasonerConfig::default();
        let reasoner = Reasoner::new(reasoner_config).expect("Failed to create reasoner");
        let mut reasoner_with_ontology = reasoner;
        reasoner_with_ontology.load_ontology(ontology).unwrap().expect("Failed to load ontology");

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
        let result = reasoner.is_consistent().await?;

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
        
        /// Convert ClassExpression to IRI
        let class_iri = match concept {
            ClassExpression::Class(class) => class.iri.to_string(),
                _ => return Err(Error::Reasoning {
                    message: "Invalid class expression for satisfiability check".to_String(),
                }),
        };

        let result = reasoner.is_class_satisfiable(&class_iri).await?;

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
    pub async fn get_superclasses(&self, class: &ClassExpression) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let mut reasoner = self.reasoner.write().unwrap();
        let superclasses = reasoner.get_superclasses(class).await?;

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
    pub async fn get_subclasses(&self, class: &ClassExpression) -> Result<HashSet<ClassExpression>> {
        let start = Instant::now();

        let mut reasoner = self.reasoner.write().unwrap();
        let subclasses = reasoner.get_subclasses(class).await?;

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

        let mut reasoner = self.reasoner.write().unwrap();
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
}

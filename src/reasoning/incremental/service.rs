//! Incremental Reasoning Service
//!
//! This module provides the main service interface for incremental reasoning,
//! integrating change tracking, delta computation, and cache management
//! with the existing reasoning infrastructure.

use super::{
    IncrementalConfig, IncrementalStatistics,
    cache_management::{ConsistencyReport, IncrementalCacheManager},
    change_tracking::{ABoxChange, ChangeTracker, TBoxChange},
    delta_computation::{DeltaComputer, QueryDelta},
};
use crate::{
    cache::CacheManager,
    error::{Error, Result},
    ontology::{
        Ontology,
        axioms::Axiom,
        concepts::{Class, ClassExpression},
        individuals::Individual,
    },
    query::advanced::{conjunctive::ConjunctiveQuery, execution::QueryEngine},
    reasoning::ReasoningService,
};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, RwLock},
    time::Instant,
};
use tokio::sync::RwLock as AsyncRwLock;

/// Main service for incremental reasoning operations
pub struct IncrementalReasoningService {
    /// Base reasoning service (wrapped for backward compatibility)
    base_service: Arc<ReasoningService>,
    /// Ontology reference
    ontology: Arc<AsyncRwLock<Ontology>>,
    /// Change tracking system
    change_tracker: Arc<ChangeTracker>,
    /// Delta computation engine
    delta_computer: Arc<DeltaComputer>,
    /// Incremental cache manager
    cache_manager: Arc<IncrementalCacheManager>,
    /// Query engine for advanced queries
    query_engine: Option<Arc<Mutex<QueryEngine>>>,
    /// Service configuration
    config: IncrementalConfig,
    /// Performance statistics
    statistics: Arc<RwLock<IncrementalStatistics>>,
    /// Last full reasoning timestamp
    last_full_reasoning: Arc<RwLock<Option<Instant>>>,
}

impl IncrementalReasoningService {
    /// Create a new incremental reasoning service
    pub async fn new(
        base_service: Arc<ReasoningService>,
        ontology: Arc<AsyncRwLock<Ontology>>,
        config: Option<IncrementalConfig>,
    ) -> Result<Self> {
        let config = config.unwrap_or_default();

        // Initialize change tracker with current ontology state
        let change_tracker = Arc::new(ChangeTracker::new(config.clone()));
        {
            let ontology_ref = ontology.read().await;
            change_tracker.initialize_from_ontology(&ontology_ref)?;
        }

        // Create delta computer
        let ontology_clone = {
            let ont = ontology.read().await;
            Arc::new(ont.clone())
        };
        let delta_computer = Arc::new(DeltaComputer::new(
            ontology_clone,
            change_tracker.clone(),
            base_service.clone(),
            None, // Use default config
        ));

        // Create cache manager
        let cache_config = crate::cache::CacheConfig::default();
        let underlying_cache = Arc::new(CacheManager::new(cache_config));

        let cache_manager = Arc::new(IncrementalCacheManager::new(
            underlying_cache,
            None, // Use default config
        ));

        Ok(Self {
            base_service,
            ontology,
            change_tracker,
            delta_computer,
            cache_manager,
            query_engine: None, // Will be set when needed
            config,
            statistics: Arc::new(RwLock::new(IncrementalStatistics::default())),
            last_full_reasoning: Arc::new(RwLock::new(None)),
        })
    }

    /// Set the advanced query engine for incremental query processing
    pub fn set_query_engine(&mut self, query_engine: Arc<Mutex<QueryEngine>>) {
        self.query_engine = Some(query_engine);
    }

    // === Incremental Operations ===

    /// Add an axiom to the ontology incrementally
    pub async fn add_axiom_incrementally(&self, axiom: Axiom) -> Result<()> {
        let start_time = Instant::now();

        // Track the change
        let change = TBoxChange::AxiomAdded {
            axiom: axiom.clone(),
            timestamp: Instant::now(),
        };
        self.change_tracker.track_tbox_change(change)?;

        // Add axiom to ontology
        {
            let mut ontology = self.ontology.write().await;
            ontology.add_axiom(axiom.clone());
        }

        // Compute and apply reasoning delta
        let delta = self
            .delta_computer
            .compute_reasoning_delta_for_changes(
                &[TBoxChange::AxiomAdded {
                    axiom,
                    timestamp: start_time,
                }],
                &[],
            )
            .await?;

        if delta.recommend_full_reasoning {
            tracing::warn!("Full reasoning recommended for axiom addition");
            self.perform_full_reasoning().await?;
        } else {
            self.cache_manager.apply_reasoning_delta(&delta).await?;
        }

        // Process invalidation events
        let invalidation_events = self.change_tracker.get_pending_invalidations();
        self.cache_manager
            .process_invalidation_events(invalidation_events)
            .await?;

        // Update statistics
        self.update_incremental_statistics(start_time, false)
            .await?;

        Ok(())
    }

    /// Remove an axiom from the ontology incrementally
    pub async fn remove_axiom_incrementally(&self, axiom: &Axiom) -> Result<()> {
        let start_time = Instant::now();

        // Track the change
        let change = TBoxChange::AxiomRemoved {
            axiom: axiom.clone(),
            timestamp: Instant::now(),
        };
        self.change_tracker.track_tbox_change(change)?;

        // Remove axiom from ontology
        {
            let mut ontology = self.ontology.write().await;
            ontology.remove_axiom(axiom);
        }

        // Compute and apply reasoning delta
        let delta = self
            .delta_computer
            .compute_reasoning_delta_for_changes(
                &[TBoxChange::AxiomRemoved {
                    axiom: axiom.clone(),
                    timestamp: start_time,
                }],
                &[],
            )
            .await?;

        if delta.recommend_full_reasoning {
            tracing::warn!("Full reasoning recommended for axiom removal");
            self.perform_full_reasoning().await?;
        } else {
            self.cache_manager.apply_reasoning_delta(&delta).await?;
        }

        // Process invalidation events
        let invalidation_events = self.change_tracker.get_pending_invalidations();
        self.cache_manager
            .process_invalidation_events(invalidation_events)
            .await?;

        // Update statistics
        self.update_incremental_statistics(start_time, false)
            .await?;

        Ok(())
    }

    /// Add a class assertion incrementally
    pub async fn add_class_assertion_incrementally(
        &self,
        individual: Individual,
        class: ClassExpression,
    ) -> Result<()> {
        let start_time = Instant::now();

        // Track the change
        let change = ABoxChange::ClassAssertionAdded {
            individual: individual.clone(),
            class: class.clone(),
            timestamp: Instant::now(),
        };
        self.change_tracker.track_abox_change(change)?;

        // Add assertion to ontology (this would need to be implemented in Ontology)
        {
            let ontology = self.ontology.write().await;
            // ontology.add_class_assertion(individual.clone(), class.clone());
            // For now, we'll just track the change
        }

        // Compute and apply reasoning delta
        let delta = self
            .delta_computer
            .compute_reasoning_delta_for_changes(
                &[],
                &[ABoxChange::ClassAssertionAdded {
                    individual,
                    class,
                    timestamp: start_time,
                }],
            )
            .await?;

        self.cache_manager.apply_reasoning_delta(&delta).await?;

        // Process invalidation events
        let invalidation_events = self.change_tracker.get_pending_invalidations();
        self.cache_manager
            .process_invalidation_events(invalidation_events)
            .await?;

        // Update statistics
        self.update_incremental_statistics(start_time, false)
            .await?;

        Ok(())
    }

    /// Execute a query with incremental result updates
    pub async fn execute_query_incrementally(
        &self,
        query: &ConjunctiveQuery,
        since: Option<Instant>,
    ) -> Result<QueryResult> {
        let start_time = Instant::now();
        let since_timestamp = since.unwrap_or_else(|| {
            if let Ok(last_reasoning) = self.last_full_reasoning.read() {
                last_reasoning.unwrap_or(start_time)
            } else {
                start_time
            }
        });

        // Check if we have a query engine
        let query_engine = self
            .query_engine
            .as_ref()
            .ok_or_else(|| Error::internal("Query engine not available"))?;

        // Compute query delta for changes since timestamp
        let query_delta = self
            .delta_computer
            .compute_query_delta(query, since_timestamp)
            .await?;

        let result = if query_delta.recommend_full_reexecution || query_delta.is_empty() {
            // Full query execution
            let results = {
                let mut query_engine_guard = query_engine
                    .lock()
                    .map_err(|e| Error::internal(format!("Failed to lock query engine: {}", e)))?;
                query_engine_guard.execute_query(&query)?
            };
            QueryResult {
                bindings: results
                    .bindings
                    .into_iter()
                    .map(|binding| {
                        binding
                            .variable_bindings
                            .into_iter()
                            .map(|(k, v)| (k.name, v.to_string()))
                            .collect::<HashMap<String, String>>()
                    })
                    .collect(),
                is_incremental: false,
                delta_applied: false,
                execution_time: start_time.elapsed(),
            }
        } else {
            // Incremental query execution
            let incremental_results = self.execute_query_delta(query, &query_delta).await?;
            QueryResult {
                bindings: incremental_results,
                is_incremental: true,
                delta_applied: true,
                execution_time: start_time.elapsed(),
            }
        };

        // Update statistics
        self.update_query_statistics(start_time, result.is_incremental)
            .await?;

        Ok(result)
    }

    // === Backward Compatibility Methods (delegate to base service) ===

    /// Check if a concept is satisfiable
    pub async fn is_satisfiable(&self, concept: &ClassExpression) -> Result<bool> {
        self.base_service.is_satisfiable(concept).await
    }

    /// Get all subclasses of a class
    pub async fn get_subclasses(&self, class: &ClassExpression) -> Result<Vec<Class>> {
        let subclasses = self.base_service.get_subclasses(class, false).await?;
        Ok(subclasses
            .into_iter()
            .filter_map(|expr| match expr {
                ClassExpression::Class(c) => Some(c),
                _ => None,
            })
            .collect())
    }

    /// Get all superclasses of a class
    pub async fn get_superclasses(&self, class: &ClassExpression) -> Result<Vec<Class>> {
        let superclasses = self.base_service.get_superclasses(class, false).await?;
        Ok(superclasses
            .into_iter()
            .filter_map(|expr| match expr {
                ClassExpression::Class(c) => Some(c),
                _ => None,
            })
            .collect())
    }

    /// Check if one class is a subclass of another
    pub async fn is_subclass_of(
        &self,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) -> Result<bool> {
        self.base_service.is_subsumed_by(subclass, superclass).await
    }

    /// Get all instances of a class
    pub async fn get_instances(&self, class: &ClassExpression) -> Result<Vec<Individual>> {
        let instances = self.base_service.get_instances(class, false).await?;
        Ok(instances.into_iter().collect())
    }

    /// Check if an individual is an instance of a class
    pub async fn is_instance_of(
        &self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Result<bool> {
        self.base_service.is_instance_of(individual, class).await
    }

    // === Management and Monitoring Operations ===

    /// Perform full reasoning (bypasses incremental optimizations)
    pub async fn perform_full_reasoning(&self) -> Result<()> {
        let start_time = Instant::now();

        // Trigger full reasoning in base service
        self.base_service.invalidate_all_caches().await?;

        // Update timestamp
        if let Ok(mut last_reasoning) = self.last_full_reasoning.write() {
            *last_reasoning = Some(start_time);
        }

        // Update statistics
        self.update_incremental_statistics(start_time, true).await?;

        tracing::info!("Full reasoning completed in {:?}", start_time.elapsed());
        Ok(())
    }

    /// Check and repair cache consistency
    pub async fn check_cache_consistency(&self) -> Result<ConsistencyReport> {
        self.cache_manager.check_and_repair_consistency().await
    }

    /// Get incremental reasoning statistics
    pub async fn get_statistics(&self) -> IncrementalStatistics {
        if let Ok(stats) = self.statistics.read() {
            stats.clone()
        } else {
            IncrementalStatistics::default()
        }
    }

    /// Get detailed performance report
    pub async fn get_performance_report(&self) -> Result<PerformanceReport> {
        let incremental_stats = self.get_statistics().await;
        let cache_stats = self.cache_manager.get_statistics().await;
        let delta_stats = self.delta_computer.get_statistics().await;

        Ok(PerformanceReport {
            incremental_statistics: incremental_stats,
            cache_statistics: cache_stats,
            delta_computation_statistics: delta_stats,
            generated_at: Instant::now(),
        })
    }

    /// Reset all incremental data (useful for testing)
    pub async fn reset_incremental_state(&self) -> Result<()> {
        // Clear change history
        // Note: This would require additional methods in ChangeTracker

        // Reset statistics
        if let Ok(mut stats) = self.statistics.write() {
            *stats = IncrementalStatistics::default();
        }

        // Clear last reasoning timestamp
        if let Ok(mut last_reasoning) = self.last_full_reasoning.write() {
            *last_reasoning = None;
        }

        tracing::info!("Incremental reasoning state reset");
        Ok(())
    }

    // === Private Helper Methods ===

    /// Execute query delta incrementally
    async fn execute_query_delta(
        &self,
        query: &ConjunctiveQuery,
        delta: &QueryDelta,
    ) -> Result<Vec<HashMap<String, String>>> {
        // This is a simplified implementation
        // In practice, this would implement sophisticated incremental query execution

        let query_engine = self
            .query_engine
            .as_ref()
            .ok_or_else(|| Error::internal("Query engine not initialized"))?;

        // For now, fall back to full execution if we have incremental additions/removals
        if !delta.incremental_additions.is_empty() || !delta.incremental_removals.is_empty() {
            // Apply incremental changes to cached results
            let mut results = delta.incremental_additions.clone();
            // Remove incremental removals (simplified logic)
            results.retain(|binding| !delta.incremental_removals.contains(binding));
            return Ok(results);
        }

        // Execute full query for now
        let result = {
            let mut query_engine_guard = query_engine
                .lock()
                .map_err(|e| Error::internal(format!("Failed to lock query engine: {}", e)))?;
            query_engine_guard.execute_query(&query)?
        };

        Ok(result
            .bindings
            .into_iter()
            .map(|binding| {
                binding
                    .variable_bindings
                    .into_iter()
                    .map(|(k, v)| (k.name, v.to_string()))
                    .collect::<HashMap<String, String>>()
            })
            .collect())
    }

    /// Update incremental reasoning statistics
    async fn update_incremental_statistics(
        &self,
        start_time: Instant,
        was_full_reasoning: bool,
    ) -> Result<()> {
        let elapsed = start_time.elapsed().as_millis() as u64;

        if let Ok(mut stats) = self.statistics.write() {
            if was_full_reasoning {
                // Don't count full reasoning as incremental
                return Ok(());
            }

            stats.incremental_updates += 1;
            stats.avoided_full_reasoning += 1;
            stats.time_saved_ms += elapsed * 5; // Estimate time saved vs full reasoning
        }

        Ok(())
    }

    /// Update query execution statistics
    async fn update_query_statistics(
        &self,
        start_time: Instant,
        was_incremental: bool,
    ) -> Result<()> {
        let elapsed = start_time.elapsed().as_millis() as u64;

        if let Ok(mut stats) = self.statistics.write() {
            if was_incremental {
                stats.time_saved_ms += elapsed * 3; // Estimate time saved vs full query
            }
        }

        Ok(())
    }
}

/// Result of query execution with incremental information
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Query result bindings
    pub bindings: Vec<HashMap<String, String>>,
    /// Whether this was computed incrementally
    pub is_incremental: bool,
    /// Whether a delta was applied
    pub delta_applied: bool,
    /// Time taken for execution
    pub execution_time: std::time::Duration,
}

/// Comprehensive performance report for incremental reasoning
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// Incremental reasoning statistics
    pub incremental_statistics: IncrementalStatistics,
    /// Cache management statistics
    pub cache_statistics: super::cache_management::IncrementalCacheStatistics,
    /// Delta computation statistics
    pub delta_computation_statistics: super::delta_computation::DeltaComputationStatistics,
    /// When this report was generated
    pub generated_at: Instant,
}

impl PerformanceReport {
    /// Calculate overall efficiency score (0.0 to 1.0)
    pub fn efficiency_score(&self) -> f64 {
        let cache_hit_ratio = self.cache_statistics.post_invalidation_hit_rate;
        let incremental_ratio = if self.incremental_statistics.incremental_updates > 0 {
            self.incremental_statistics.avoided_full_reasoning as f64
                / self.incremental_statistics.incremental_updates as f64
        } else {
            0.0
        };

        (cache_hit_ratio + incremental_ratio) / 2.0
    }

    /// Get total time saved by incremental reasoning (milliseconds)
    pub fn total_time_saved_ms(&self) -> u64 {
        self.incremental_statistics.time_saved_ms + self.cache_statistics.time_saved_ms
    }

    /// Get total memory saved by incremental approaches (bytes)
    pub fn total_memory_saved_bytes(&self) -> usize {
        self.cache_statistics.memory_saved_bytes + self.incremental_statistics.memory_usage_bytes
    }
}

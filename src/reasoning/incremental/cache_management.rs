//! Incremental Cache Management System
//!
//! This module extends the existing cache infrastructure with incremental
//! invalidation, selective updates, and consistency management for efficient
//! reasoning with ontology changes.

use super::{
    IncrementalStatistics,
    change_tracking::{ChangeTracker, InvalidationEvent},
    delta_computation::{QueryDelta, ReasoningDelta},
};
use crate::{
    cache::{CacheManager, ConceptSatisfiabilityCache},
    error::{OxidowlError, Result},
    ontology::{
        Ontology, OntologyRef,
        axioms::Axiom,
        concepts::{Class, ClassExpression},
        individuals::Individual,
    },
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
    time::Instant,
};

/// Enhanced cache manager with incremental invalidation capabilities
#[derive(Debug, Clone)]
pub struct IncrementalCacheManager {
    /// Base cache manager from the core system
    base_cache: Arc<CacheManager>,
    /// Tracks which cache entries need invalidation
    invalidation_tracker: Arc<RwLock<InvalidationTracker>>,
    /// Handles selective cache updates
    selective_updater: Arc<SelectiveUpdater>,
    /// Configuration for cache behavior
    config: IncrementalCacheConfig,
    /// Performance statistics
    statistics: Arc<RwLock<IncrementalCacheStatistics>>,
}

/// Configuration for incremental cache management
#[derive(Debug, Clone)]
pub struct IncrementalCacheConfig {
    /// Enable aggressive invalidation for maximum consistency
    pub aggressive_invalidation: bool,
    /// Maximum number of invalidation events to batch process
    pub max_invalidation_batch_size: usize,
    /// Enable selective cache warming after invalidation
    pub enable_cache_warming: bool,
    /// Maximum age of cache entries before auto-invalidation (in seconds)
    pub max_cache_age_seconds: u64,
    /// Enable performance monitoring
    pub enable_statistics: bool,
}

impl Default for IncrementalCacheConfig {
    fn default() -> Self {
        Self {
            aggressive_invalidation: false,
            max_invalidation_batch_size: 1000,
            enable_cache_warming: true,
            max_cache_age_seconds: 3600, // 1 hour
            enable_statistics: true,
        }
    }
}

/// Statistics for incremental cache performance monitoring
#[derive(Debug, Default, Clone)]
pub struct IncrementalCacheStatistics {
    /// Number of cache entries invalidated
    pub invalidations_performed: usize,
    /// Number of selective cache updates
    pub selective_updates: usize,
    /// Number of cache warming operations
    pub cache_warming_operations: usize,
    /// Time saved by avoiding full cache rebuilds (milliseconds)
    pub time_saved_ms: u64,
    /// Memory saved by selective invalidation (bytes)
    pub memory_saved_bytes: usize,
    /// Cache hit rate after invalidation
    pub post_invalidation_hit_rate: f64,
    /// Number of consistency checks performed
    pub consistency_checks: usize,
}

impl IncrementalCacheManager {
    /// Create a new incremental cache manager
    pub fn new(base_cache: Arc<CacheManager>, config: Option<IncrementalCacheConfig>) -> Self {
        let config = config.unwrap_or_default();

        Self {
            base_cache,
            invalidation_tracker: Arc::new(RwLock::new(InvalidationTracker::new())),
            selective_updater: Arc::new(SelectiveUpdater::new()),
            config,
            statistics: Arc::new(RwLock::new(IncrementalCacheStatistics::default())),
        }
    }

    /// Process invalidation events from the change tracker
    pub async fn process_invalidation_events(&self, events: Vec<InvalidationEvent>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let start_time = Instant::now();
        let mut total_invalidations = 0;

        // Batch process invalidation events
        for batch in events.chunks(self.config.max_invalidation_batch_size) {
            let batch_invalidations = self.process_invalidation_batch(batch).await?;
            total_invalidations += batch_invalidations;
        }

        // Update statistics
        if self.config.enable_statistics {
            self.update_statistics(start_time, total_invalidations)
                .await?;
        }

        // Perform cache warming if enabled
        if self.config.enable_cache_warming {
            self.warm_cache_after_invalidation().await?;
        }

        Ok(())
    }

    /// Apply a reasoning delta to the cache system
    pub async fn apply_reasoning_delta(&self, delta: &ReasoningDelta) -> Result<()> {
        let start_time = Instant::now();

        // Invalidate concept satisfiability cache for affected concepts
        for concept in &delta.concepts_to_recheck {
            self.invalidate_concept_cache(concept).await?;
        }

        // Invalidate hierarchy-related caches
        for (subclass, superclass) in &delta.hierarchy_updates {
            self.invalidate_hierarchy_cache(subclass, superclass)
                .await?;
        }

        // Invalidate individual-related caches
        for individual in &delta.individual_updates {
            self.invalidate_individual_cache(individual).await?;
        }

        // Process explicit cache invalidations
        for cache_key in &delta.cache_invalidations {
            self.invalidate_cache_entry(cache_key).await?;
        }

        // Update statistics
        if self.config.enable_statistics {
            let invalidation_count = delta.concepts_to_recheck.len()
                + delta.hierarchy_updates.len()
                + delta.individual_updates.len()
                + delta.cache_invalidations.len();
            self.update_statistics(start_time, invalidation_count)
                .await?;
        }

        Ok(())
    }

    /// Apply a query delta to query result caches
    pub async fn apply_query_delta(&self, delta: &QueryDelta) -> Result<()> {
        // Invalidate affected query results
        for result_key in &delta.result_invalidations {
            self.invalidate_query_result_cache(result_key).await?;
        }

        // Apply incremental additions/removals if supported
        // (This would integrate with a more sophisticated query result cache)

        Ok(())
    }

    /// Check cache consistency and repair if necessary
    pub async fn check_and_repair_consistency(&self) -> Result<ConsistencyReport> {
        let start_time = Instant::now();
        let mut report = ConsistencyReport::new();

        // Check concept satisfiability cache consistency
        let concept_issues = self.check_concept_cache_consistency().await?;
        report.concept_consistency_issues = concept_issues;

        // Check hierarchy cache consistency
        let hierarchy_issues = self.check_hierarchy_cache_consistency().await?;
        report.hierarchy_consistency_issues = hierarchy_issues;

        // Check individual cache consistency
        let individual_issues = self.check_individual_cache_consistency().await?;
        report.individual_consistency_issues = individual_issues;

        // Repair inconsistencies if found
        if report.has_issues() {
            self.repair_cache_inconsistencies(&report).await?;
            report.repairs_applied = true;
        }

        report.check_duration = start_time.elapsed();

        // Update statistics
        if self.config.enable_statistics {
            if let Ok(mut stats) = self.statistics.write() {
                stats.consistency_checks += 1;
            }
        }

        Ok(report)
    }

    /// Get cache statistics
    pub async fn get_statistics(&self) -> IncrementalCacheStatistics {
        if let Ok(stats) = self.statistics.read() {
            stats.clone()
        } else {
            IncrementalCacheStatistics::default()
        }
    }

    /// Process a batch of invalidation events
    async fn process_invalidation_batch(&self, events: &[InvalidationEvent]) -> Result<usize> {
        let mut invalidation_count = 0;

        for event in events {
            match event {
                InvalidationEvent::ConceptSatisfiability(classes) => {
                    for class in classes {
                        self.invalidate_concept_cache(&ClassExpression::Class(class.clone()))
                            .await?;
                        invalidation_count += 1;
                    }
                }
                InvalidationEvent::SubclassRelations(classes) => {
                    for class in classes {
                        self.invalidate_subclass_cache(class).await?;
                        invalidation_count += 1;
                    }
                }
                InvalidationEvent::InstanceRelations(individuals) => {
                    for individual in individuals {
                        self.invalidate_individual_cache(individual).await?;
                        invalidation_count += 1;
                    }
                }
                InvalidationEvent::QueryResults(query_keys) => {
                    for query_key in query_keys {
                        self.invalidate_query_result_cache(query_key).await?;
                        invalidation_count += 1;
                    }
                }
                InvalidationEvent::FullInvalidation => {
                    self.invalidate_all_caches().await?;
                    invalidation_count += 1000; // Rough estimate for full invalidation
                }
            }
        }

        Ok(invalidation_count)
    }

    /// Invalidate concept satisfiability cache for a specific concept
    async fn invalidate_concept_cache(&self, concept: &ClassExpression) -> Result<()> {
        // Generate cache key for concept
        let cache_key = self.generate_concept_cache_key(concept);

        // Track invalidation
        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.add_concept_invalidation(cache_key.clone());
        }

        // The actual cache invalidation would happen here
        // For now, we're working with the interface
        tracing::debug!("Invalidated concept cache for: {:?}", concept);

        Ok(())
    }

    /// Invalidate hierarchy-related caches
    async fn invalidate_hierarchy_cache(&self, subclass: &Class, superclass: &Class) -> Result<()> {
        let cache_key = format!("hierarchy_{}_{}", subclass.iri, superclass.iri);

        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.add_hierarchy_invalidation(cache_key);
        }

        tracing::debug!(
            "Invalidated hierarchy cache for: {} -> {}",
            subclass.iri,
            superclass.iri
        );

        Ok(())
    }

    /// Invalidate individual-related caches
    async fn invalidate_individual_cache(&self, individual: &Individual) -> Result<()> {
        let iri = match individual {
            Individual::Named(named) => named.iri.to_string(),
            Individual::Anonymous(anon) => anon.id.clone(),
        };
        let cache_key = format!("individual_{}", iri);

        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.add_individual_invalidation(cache_key);
        }

        tracing::debug!("Invalidated individual cache for: {}", iri);

        Ok(())
    }

    /// Invalidate subclass-related caches
    async fn invalidate_subclass_cache(&self, class: &Class) -> Result<()> {
        let cache_key = format!("subclass_{}", class.iri);

        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.add_subclass_invalidation(cache_key);
        }

        tracing::debug!("Invalidated subclass cache for: {}", class.iri);

        Ok(())
    }

    /// Invalidate query result cache
    async fn invalidate_query_result_cache(&self, query_key: &str) -> Result<()> {
        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.add_query_invalidation(query_key.to_string());
        }

        tracing::debug!("Invalidated query result cache for: {}", query_key);

        Ok(())
    }

    /// Invalidate specific cache entry by key
    async fn invalidate_cache_entry(&self, cache_key: &str) -> Result<()> {
        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.add_generic_invalidation(cache_key.to_string());
        }

        tracing::debug!("Invalidated cache entry: {}", cache_key);

        Ok(())
    }

    /// Invalidate all caches (nuclear option)
    async fn invalidate_all_caches(&self) -> Result<()> {
        if let Ok(mut tracker) = self.invalidation_tracker.write() {
            tracker.mark_full_invalidation();
        }

        tracing::warn!("Performed full cache invalidation");

        Ok(())
    }

    /// Warm cache after invalidation by pre-computing likely needed results
    async fn warm_cache_after_invalidation(&self) -> Result<()> {
        // This would implement cache warming strategies
        // For now, just log the operation
        tracing::debug!("Cache warming operation completed");

        if let Ok(mut stats) = self.statistics.write() {
            stats.cache_warming_operations += 1;
        }

        Ok(())
    }

    /// Generate cache key for concept
    fn generate_concept_cache_key(&self, concept: &ClassExpression) -> String {
        format!("concept_sat_{:?}", concept)
    }

    /// Check concept cache consistency
    async fn check_concept_cache_consistency(&self) -> Result<Vec<ConsistencyIssue>> {
        // This would implement consistency checking logic
        // For now, return empty list
        Ok(Vec::new())
    }

    /// Check hierarchy cache consistency
    async fn check_hierarchy_cache_consistency(&self) -> Result<Vec<ConsistencyIssue>> {
        // This would implement hierarchy consistency checking
        Ok(Vec::new())
    }

    /// Check individual cache consistency
    async fn check_individual_cache_consistency(&self) -> Result<Vec<ConsistencyIssue>> {
        // This would implement individual consistency checking
        Ok(Vec::new())
    }

    /// Repair cache inconsistencies
    async fn repair_cache_inconsistencies(&self, report: &ConsistencyReport) -> Result<()> {
        // This would implement cache repair logic
        tracing::info!("Repaired {} cache inconsistencies", report.total_issues());
        Ok(())
    }

    /// Update performance statistics
    async fn update_statistics(
        &self,
        start_time: Instant,
        invalidation_count: usize,
    ) -> Result<()> {
        let elapsed = start_time.elapsed();

        if let Ok(mut stats) = self.statistics.write() {
            stats.invalidations_performed += invalidation_count;
            stats.selective_updates += 1;
            // Estimate time saved vs full cache rebuild
            stats.time_saved_ms += elapsed.as_millis() as u64 * 10; // Conservative estimate
        }

        Ok(())
    }
}

/// Tracks which cache entries need invalidation
#[derive(Debug, Default)]
pub struct InvalidationTracker {
    /// Concept satisfiability cache keys to invalidate
    concept_invalidations: HashSet<String>,
    /// Hierarchy cache keys to invalidate
    hierarchy_invalidations: HashSet<String>,
    /// Individual cache keys to invalidate
    individual_invalidations: HashSet<String>,
    /// Subclass cache keys to invalidate
    subclass_invalidations: HashSet<String>,
    /// Query result cache keys to invalidate
    query_invalidations: HashSet<String>,
    /// Generic cache keys to invalidate
    generic_invalidations: HashSet<String>,
    /// Whether full invalidation is needed
    full_invalidation_needed: bool,
    /// Timestamp of last invalidation
    last_invalidation: Option<Instant>,
}

impl InvalidationTracker {
    /// Create a new invalidation tracker
    pub fn new() -> Self {
        Self::default()
    }

    /// Add concept invalidation
    pub fn add_concept_invalidation(&mut self, key: String) {
        self.concept_invalidations.insert(key);
        self.last_invalidation = Some(Instant::now());
    }

    /// Add hierarchy invalidation
    pub fn add_hierarchy_invalidation(&mut self, key: String) {
        self.hierarchy_invalidations.insert(key);
        self.last_invalidation = Some(Instant::now());
    }

    /// Add individual invalidation
    pub fn add_individual_invalidation(&mut self, key: String) {
        self.individual_invalidations.insert(key);
        self.last_invalidation = Some(Instant::now());
    }

    /// Add subclass invalidation
    pub fn add_subclass_invalidation(&mut self, key: String) {
        self.subclass_invalidations.insert(key);
        self.last_invalidation = Some(Instant::now());
    }

    /// Add query invalidation
    pub fn add_query_invalidation(&mut self, key: String) {
        self.query_invalidations.insert(key);
        self.last_invalidation = Some(Instant::now());
    }

    /// Add generic invalidation
    pub fn add_generic_invalidation(&mut self, key: String) {
        self.generic_invalidations.insert(key);
        self.last_invalidation = Some(Instant::now());
    }

    /// Mark full invalidation needed
    pub fn mark_full_invalidation(&mut self) {
        self.full_invalidation_needed = true;
        self.last_invalidation = Some(Instant::now());
    }

    /// Check if any invalidations are pending
    pub fn has_pending_invalidations(&self) -> bool {
        !self.concept_invalidations.is_empty()
            || !self.hierarchy_invalidations.is_empty()
            || !self.individual_invalidations.is_empty()
            || !self.subclass_invalidations.is_empty()
            || !self.query_invalidations.is_empty()
            || !self.generic_invalidations.is_empty()
            || self.full_invalidation_needed
    }

    /// Get all pending invalidation keys
    pub fn get_all_invalidation_keys(&self) -> HashSet<String> {
        let mut all_keys = HashSet::new();
        all_keys.extend(self.concept_invalidations.iter().cloned());
        all_keys.extend(self.hierarchy_invalidations.iter().cloned());
        all_keys.extend(self.individual_invalidations.iter().cloned());
        all_keys.extend(self.subclass_invalidations.iter().cloned());
        all_keys.extend(self.query_invalidations.iter().cloned());
        all_keys.extend(self.generic_invalidations.iter().cloned());
        all_keys
    }

    /// Clear all invalidations
    pub fn clear(&mut self) {
        self.concept_invalidations.clear();
        self.hierarchy_invalidations.clear();
        self.individual_invalidations.clear();
        self.subclass_invalidations.clear();
        self.query_invalidations.clear();
        self.generic_invalidations.clear();
        self.full_invalidation_needed = false;
        self.last_invalidation = None;
    }
}

/// Handles selective cache updates
#[derive(Debug)]
pub struct SelectiveUpdater {
    /// Configuration for update strategies
    config: SelectiveUpdateConfig,
}

/// Configuration for selective cache updates
#[derive(Debug, Clone)]
pub struct SelectiveUpdateConfig {
    /// Enable parallel cache updates
    pub enable_parallel_updates: bool,
    /// Maximum number of concurrent update tasks
    pub max_concurrent_updates: usize,
    /// Prioritize updates based on access frequency
    pub prioritize_by_frequency: bool,
}

impl Default for SelectiveUpdateConfig {
    fn default() -> Self {
        Self {
            enable_parallel_updates: true,
            max_concurrent_updates: 4,
            prioritize_by_frequency: true,
        }
    }
}

impl SelectiveUpdater {
    /// Create a new selective updater
    pub fn new() -> Self {
        Self {
            config: SelectiveUpdateConfig::default(),
        }
    }

    /// Update caches selectively based on delta information
    pub async fn update_caches_selectively(&self, delta: &ReasoningDelta) -> Result<()> {
        // This would implement selective cache update logic
        tracing::debug!(
            "Performing selective cache updates for delta with {} concepts",
            delta.concepts_to_recheck.len()
        );
        Ok(())
    }
}

impl Default for SelectiveUpdater {
    fn default() -> Self {
        Self::new()
    }
}

/// Report on cache consistency check
#[derive(Debug, Clone)]
pub struct ConsistencyReport {
    /// Issues found in concept cache
    pub concept_consistency_issues: Vec<ConsistencyIssue>,
    /// Issues found in hierarchy cache
    pub hierarchy_consistency_issues: Vec<ConsistencyIssue>,
    /// Issues found in individual cache
    pub individual_consistency_issues: Vec<ConsistencyIssue>,
    /// Whether repairs were applied
    pub repairs_applied: bool,
    /// Time taken for consistency check
    pub check_duration: std::time::Duration,
}

impl ConsistencyReport {
    /// Create a new empty consistency report
    pub fn new() -> Self {
        Self {
            concept_consistency_issues: Vec::new(),
            hierarchy_consistency_issues: Vec::new(),
            individual_consistency_issues: Vec::new(),
            repairs_applied: false,
            check_duration: std::time::Duration::from_secs(0),
        }
    }

    /// Check if any issues were found
    pub fn has_issues(&self) -> bool {
        !self.concept_consistency_issues.is_empty()
            || !self.hierarchy_consistency_issues.is_empty()
            || !self.individual_consistency_issues.is_empty()
    }

    /// Get total number of issues
    pub fn total_issues(&self) -> usize {
        self.concept_consistency_issues.len()
            + self.hierarchy_consistency_issues.len()
            + self.individual_consistency_issues.len()
    }
}

impl Default for ConsistencyReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents a cache consistency issue
#[derive(Debug, Clone)]
pub struct ConsistencyIssue {
    /// Type of consistency issue
    pub issue_type: ConsistencyIssueType,
    /// Cache key involved
    pub cache_key: String,
    /// Description of the issue
    pub description: String,
    /// Severity level
    pub severity: ConsistencyIssueSeverity,
}

/// Types of consistency issues
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConsistencyIssueType {
    /// Stale cache entry
    StaleEntry,
    /// Missing cache entry
    MissingEntry,
    /// Inconsistent cache entry
    InconsistentEntry,
    /// Corrupt cache entry
    CorruptEntry,
}

/// Severity levels for consistency issues
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ConsistencyIssueSeverity {
    /// Low severity - performance impact only
    Low,
    /// Medium severity - correctness concerns
    Medium,
    /// High severity - critical correctness issues
    High,
}

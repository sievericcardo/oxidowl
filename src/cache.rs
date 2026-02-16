//! Cache System for Oxidowl
//!
//! This module implements efficient caching strategies for ontology reasoning,
//! including concept and role satisfiability caches, subsumption caches,
//! inference caches, and RDF-star quoted triple optimizations.

use crate::{
    core::lock_helpers::{read_lock, write_lock},
    ontology::{ClassExpression, Individual, OntologyRef},
    performance::MemoryTracker,
    reasoning::{ClassificationResult, RealizationResult},
    semantics::quoted_triple_optimizer::{QuotedTripleOptimizer, QuotedTripleOptimizerConfig},
};

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

/// Cache entry with Timetolive (TTL) support
#[derive(Debug, Clone)]
pub struct CacheEntry<T> {
    pub value: T,
    pub timestamp: Instant,
    pub hit_count: u64,
}

/// Internal cache statistics tracking
#[derive(Debug, Clone, Default)]
struct CacheMetrics {
    hits: u64,
    misses: u64,
    evictions: u64,
}

impl<T> CacheEntry<T> {
    pub fn new(value: T) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
            hit_count: 0,
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }

    pub fn hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size: usize, // Maximum number of entries in the cache
    pub ttl: Duration,   // Time to live for cache entries
    pub enable_concept_cache: bool,
    pub enable_subsumption_cache: bool,
    pub enable_satisfiability_cache: bool,
    pub enable_classification_cache: bool,
    pub enable_realization_cache: bool,
    pub enable_completion_graph_cache: bool,
    pub enable_quoted_triple_cache: bool, // RDF-star quoted triple optimization
    pub completion_graph_max_memory_mb: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10000,                // Default maximum size
            ttl: Duration::from_secs(3600), // Default TTL of 1 hour
            enable_concept_cache: true,
            enable_subsumption_cache: true,
            enable_satisfiability_cache: true,
            enable_completion_graph_cache: true,
            enable_quoted_triple_cache: true, // Enable RDF-star optimization by default
            completion_graph_max_memory_mb: 512,
            enable_classification_cache: true,
            enable_realization_cache: true,
        }
    }
}

/// Cache for concept satisfiability
#[derive(Debug, Clone)]
pub struct ConceptSatisfiabilityCache {
    cache: Arc<RwLock<HashMap<ClassExpression, CacheEntry<bool>>>>,
    config: CacheConfig,
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl ConceptSatisfiabilityCache {
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    #[must_use]
    pub fn get(&self, expression: &ClassExpression) -> Option<bool> {
        if !self.config.enable_satisfiability_cache {
            return None; // Cache is disabled
        }

        let mut cache = write_lock(&self.cache, "cache get").ok()?;
        if let Some(entry) = cache.get_mut(expression) {
            if entry.is_expired(self.config.ttl) {
                cache.remove(expression); // Remove expired entry
                if let Ok(mut metrics) = write_lock(&self.metrics, "metrics update") {
                    metrics.misses += 1;
                }
                None
            } else {
                entry.hit(); // Increment hit count
                if let Ok(mut metrics) = write_lock(&self.metrics, "metrics update") {
                    metrics.hits += 1;
                }
                Some(entry.value)
            }
        } else {
            // If not found, we can return None
            if let Ok(mut metrics) = write_lock(&self.metrics, "metrics update") {
                metrics.misses += 1;
            }
            None
        }
    }

    pub fn put(&self, expression: ClassExpression, result: bool) {
        if !self.config.enable_satisfiability_cache {
            return; // Cache is disabled
        }

        if let Ok(mut cache) = write_lock(&self.cache, "cache put") {
            if cache.len() >= self.config.max_size {
                // Evict the oldest entry if max size exceeded
                self.evict_lru(&mut cache);
            }
            cache.insert(expression, CacheEntry::new(result));
        }
    }

    fn evict_lru(&self, cache: &mut HashMap<ClassExpression, CacheEntry<bool>>) {
        if let Some((key, _)) = cache.iter().min_by_key(|(_, entry)| entry.timestamp) {
            let key_to_remove = key.clone();
            cache.remove(&key_to_remove);
            if let Ok(mut metrics) = write_lock(&self.metrics, "metrics update") {
                metrics.evictions += 1;
            }
        }
    }

    pub fn clear(&self) {
        if let Ok(mut cache) = write_lock(&self.cache, "cache clear") {
            cache.clear();
        }
        if let Ok(mut metrics) = write_lock(&self.metrics, "metrics reset") {
            *metrics = CacheMetrics::default();
        }
    }

    #[must_use]
    pub fn get_metrics(&self) -> CacheMetrics {
        read_lock(&self.metrics, "get metrics")
            .map(|m| m.clone())
            .unwrap_or_default()
    }

    #[must_use]
    pub fn size(&self) -> usize {
        read_lock(&self.cache, "cache size")
            .map(|c| c.len())
            .unwrap_or(0)
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        if let Ok(cache) = read_lock(&self.cache, "cache hit rate") {
            let total_hits: u64 = cache.values().map(|entry| entry.hit_count).sum();
            let entries = cache.len() as u64;

            if entries == 0 {
                0.0 // Avoid division by zero
            } else {
                total_hits as f64 / cache.len() as f64
            }
        } else {
            0.0
        }
    }
}

/// Compressed completion graph for caching
#[derive(Debug, Clone)]
pub struct CompletedGraph {
    /// Unique signature hash for this graph
    pub signature: u64,

    /// Compressed node data (shared via Arc)
    pub nodes: Arc<Vec<CompressedNode>>,

    /// Compressed edge data (shared via Arc)
    pub edges: Arc<Vec<CompressedEdge>>,

    /// Graph metadata
    pub metadata: GraphMetadata,

    /// Estimated memory size in bytes
    pub memory_size: usize,
}

/// Compressed node representation
#[derive(Debug, Clone)]
pub struct CompressedNode {
    pub id: usize,
    pub concepts: Arc<Vec<String>>, // Shared concept strings
    pub is_blocked: bool,
}

/// Compressed edge representation
#[derive(Debug, Clone)]
pub struct CompressedEdge {
    pub from: usize,
    pub to: usize,
    pub label: Arc<String>, // Shared label strings
}

/// Metadata for a completion graph
#[derive(Debug, Clone)]
pub struct GraphMetadata {
    pub node_count: usize,
    pub edge_count: usize,
    pub max_depth: usize,
    pub is_complete: bool,
    pub timestamp: Instant,
}

/// Tier for cache eviction
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CacheTier {
    Hot,   // Frequently accessed
    Warm,  // Moderately accessed
    Cold,  // Rarely accessed
}

/// Entry in the completion graph cache
#[derive(Debug, Clone)]
struct CompletionGraphEntry {
    graph: Arc<CompletedGraph>,
    tier: CacheTier,
    access_count: u64,
    last_access: Instant,
}

/// Completion graph cache with tiered LRU eviction
#[derive(Debug, Clone)]
pub struct CompletionGraphCache {
    /// Cache storage
    cache: Arc<RwLock<HashMap<u64, CompletionGraphEntry>>>,

    /// Hot tier (most frequently accessed)
    hot_tier: Arc<RwLock<VecDeque<u64>>>,

    /// Warm tier
    warm_tier: Arc<RwLock<VecDeque<u64>>>,

    /// Cold tier
    cold_tier: Arc<RwLock<VecDeque<u64>>>,

    /// Configuration
    config: CacheConfig,

    /// Current memory usage in bytes
    memory_usage: Arc<RwLock<usize>>,

    /// Memory pressure threshold (bytes)
    memory_threshold: usize,

    /// Statistics
    metrics: Arc<RwLock<CacheMetrics>>,
}

impl CompletionGraphCache {
    pub fn new(config: CacheConfig) -> Self {
        let memory_threshold = config.completion_graph_max_memory_mb * 1024 * 1024;

        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            hot_tier: Arc::new(RwLock::new(VecDeque::new())),
            warm_tier: Arc::new(RwLock::new(VecDeque::new())),
            cold_tier: Arc::new(RwLock::new(VecDeque::new())),
            config,
            memory_usage: Arc::new(RwLock::new(0)),
            memory_threshold,
            metrics: Arc::new(RwLock::new(CacheMetrics::default())),
        }
    }

    /// Get a completion graph from cache
    pub fn get(&self, signature: u64) -> Option<Arc<CompletedGraph>> {
        if !self.config.enable_completion_graph_cache {
            return None;
        }

        if let Ok(mut cache) = self.cache.write() {
            if let Some(entry) = cache.get_mut(&signature) {
                entry.access_count += 1;
                entry.last_access = Instant::now();

                // Update metrics
                if let Ok(mut metrics) = self.metrics.write() {
                    metrics.hits += 1;
                }

                // Promote to higher tier if needed
                self.promote_tier(signature, entry.access_count);

                return Some(Arc::clone(&entry.graph));
            }
        }

        // Update miss metrics
        if let Ok(mut metrics) = self.metrics.write() {
            metrics.misses += 1;
        }

        None
    }

    /// Store a completion graph in cache
    pub fn put(&self, graph: Arc<CompletedGraph>) {
        if !self.config.enable_completion_graph_cache {
            return;
        }

        let signature = graph.signature;
        let memory_size = graph.memory_size;

        // Check memory pressure and evict if necessary
        if let Ok(current_usage) = self.memory_usage.read() {
            if *current_usage + memory_size > self.memory_threshold {
                self.evict_to_fit(memory_size);
            }
        }

        // Create entry in cold tier initially
        let entry = CompletionGraphEntry {
            graph,
            tier: CacheTier::Cold,
            access_count: 0,
            last_access: Instant::now(),
        };

        if let Ok(mut cache) = self.cache.write() {
            cache.insert(signature, entry);
        }

        if let Ok(mut cold_tier) = self.cold_tier.write() {
            cold_tier.push_back(signature);
        }

        // Update memory usage
        if let Ok(mut usage) = self.memory_usage.write() {
            *usage += memory_size;
        }
    }

    /// Promote an entry to a higher tier based on access count
    fn promote_tier(&self, signature: u64, access_count: u64) {
        // Hot tier: >10 accesses
        // Warm tier: 3-10 accesses
        // Cold tier: <3 accesses

        if let Ok(mut cache) = self.cache.write() {
            if let Some(entry) = cache.get_mut(&signature) {
                let old_tier = entry.tier;
                let new_tier = if access_count > 10 {
                    CacheTier::Hot
                } else if access_count > 3 {
                    CacheTier::Warm
                } else {
                    CacheTier::Cold
                };

                if old_tier != new_tier {
                    entry.tier = new_tier;

                    // Move between tier queues
                    self.move_between_tiers(signature, old_tier, new_tier);
                }
            }
        }
    }

    /// Move an entry between tier queues
    fn move_between_tiers(&self, signature: u64, old_tier: CacheTier, new_tier: CacheTier) {
        // Remove from old tier
        match old_tier {
            CacheTier::Hot => {
                if let Ok(mut hot) = self.hot_tier.write() {
                    hot.retain(|&s| s != signature);
                }
            }
            CacheTier::Warm => {
                if let Ok(mut warm) = self.warm_tier.write() {
                    warm.retain(|&s| s != signature);
                }
            }
            CacheTier::Cold => {
                if let Ok(mut cold) = self.cold_tier.write() {
                    cold.retain(|&s| s != signature);
                }
            }
        }

        // Add to new tier
        match new_tier {
            CacheTier::Hot => {
                if let Ok(mut hot) = self.hot_tier.write() {
                    hot.push_back(signature);
                }
            }
            CacheTier::Warm => {
                if let Ok(mut warm) = self.warm_tier.write() {
                    warm.push_back(signature);
                }
            }
            CacheTier::Cold => {
                if let Ok(mut cold) = self.cold_tier.write() {
                    cold.push_back(signature);
                }
            }
        }
    }

    /// Evict entries to make room for new entry
    fn evict_to_fit(&self, required_space: usize) {
        let mut freed_space = 0;

        // Evict from cold tier first
        while freed_space < required_space {
            if let Ok(mut cold) = self.cold_tier.write() {
                if let Some(signature) = cold.pop_front() {
                    freed_space += self.evict_entry(signature);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Evict from warm tier if needed
        while freed_space < required_space {
            if let Ok(mut warm) = self.warm_tier.write() {
                if let Some(signature) = warm.pop_front() {
                    freed_space += self.evict_entry(signature);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        // Evict from hot tier as last resort
        while freed_space < required_space {
            if let Ok(mut hot) = self.hot_tier.write() {
                if let Some(signature) = hot.pop_front() {
                    freed_space += self.evict_entry(signature);
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    /// Evict a single entry and return freed memory
    fn evict_entry(&self, signature: u64) -> usize {
        if let Ok(mut cache) = self.cache.write() {
            if let Some(entry) = cache.remove(&signature) {
                let freed = entry.graph.memory_size;

                // Update memory usage
                if let Ok(mut usage) = self.memory_usage.write() {
                    *usage = usage.saturating_sub(freed);
                }

                // Update metrics
                if let Ok(mut metrics) = self.metrics.write() {
                    metrics.evictions += 1;
                }

                return freed;
            }
        }
        0
    }

    /// Get current memory usage
    pub fn memory_usage(&self) -> usize {
        self.memory_usage.read().map(|u| *u).unwrap_or(0)
    }

    /// Get cache statistics
    pub fn get_metrics(&self) -> CacheMetrics {
        self.metrics.read().map(|m| m.clone()).unwrap_or_default()
    }

    /// Clear the cache
    pub fn clear(&self) {
        if let Ok(mut cache) = self.cache.write() {
            cache.clear();
        }
        if let Ok(mut hot) = self.hot_tier.write() {
            hot.clear();
        }
        if let Ok(mut warm) = self.warm_tier.write() {
            warm.clear();
        }
        if let Ok(mut cold) = self.cold_tier.write() {
            cold.clear();
        }
        if let Ok(mut usage) = self.memory_usage.write() {
            *usage = 0;
        }
    }
}

/// Cache manager that coordinates different caches
#[derive(Debug, Clone)]
pub struct CacheManager {
    concept_cache: ConceptSatisfiabilityCache,
    completion_graph_cache: CompletionGraphCache,
    quoted_triple_optimizer: QuotedTripleOptimizer,
    config: CacheConfig,
    memory_tracker: Option<Arc<MemoryTracker>>,
}

impl CacheManager {
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        // Create optimizer config based on cache config
        let optimizer_config = if config.enable_quoted_triple_cache {
            QuotedTripleOptimizerConfig::default()
        } else {
            QuotedTripleOptimizerConfig::rdf11_mode()
        };

        Self {
            concept_cache: ConceptSatisfiabilityCache::new(config.clone()),
            completion_graph_cache: CompletionGraphCache::new(config.clone()),
            quoted_triple_optimizer: QuotedTripleOptimizer::new(optimizer_config),
            config,
            memory_tracker: None,
        }
    }

    /// Create a new cache manager with memory tracking
    #[must_use]
    pub fn with_memory_tracking(config: CacheConfig, memory_tracker: Arc<MemoryTracker>) -> Self {
        // Create optimizer config based on cache config
        let optimizer_config = if config.enable_quoted_triple_cache {
            QuotedTripleOptimizerConfig::default()
        } else {
            QuotedTripleOptimizerConfig::rdf11_mode()
        };

        Self {
            concept_cache: ConceptSatisfiabilityCache::new(config.clone()),
            completion_graph_cache: CompletionGraphCache::new(config.clone()),
            quoted_triple_optimizer: QuotedTripleOptimizer::new(optimizer_config),
            config,
            memory_tracker: Some(memory_tracker),
        }
    }

    /// Get estimated cache memory usage in bytes
    #[must_use]
    pub fn estimated_memory_usage(&self) -> usize {
        // Rough estimate: each cache entry is approximately 100 bytes
        // (ClassExpression + CacheEntry overhead)
        self.concept_cache.size() * 100
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.concept_cache.clear();
        self.completion_graph_cache.clear();
        self.quoted_triple_optimizer.clear();
    }

    /// Get consistency result from cache
    pub fn get_consistency_result(&self, _ontology: &OntologyRef) -> Option<bool> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store consistency result in cache
    pub fn cache_consistency_result(&self, _ontology: &OntologyRef, _result: bool) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get satisfiability result from cache
    #[must_use]
    pub fn get_satisfiability_result(&self, expression: &ClassExpression) -> Option<bool> {
        self.concept_cache.get(expression)
    }

    /// Store satisfiability result in cache
    pub fn cache_satisfiability_result(&self, expression: ClassExpression, result: bool) {
        self.concept_cache.put(expression, result);
    }

    /// Get subsumption result from cache
    #[must_use]
    pub fn get_subsumption_result(
        &self,
        _sub: &ClassExpression,
        _sup: &ClassExpression,
    ) -> Option<bool> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store subsumption result in cache
    pub fn cache_subsumption_result(
        &self,
        _sub: ClassExpression,
        _sup: ClassExpression,
        _result: bool,
    ) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get classification result from cache
    pub fn get_classification_result(
        &self,
        _ontology: &OntologyRef,
    ) -> Option<ClassificationResult> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store classification result in cache
    pub fn store_classification_result(
        &self,
        _ontology: &OntologyRef,
        _result: ClassificationResult,
    ) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get realization result from cache
    pub fn get_realization_result(&self, _ontology: &OntologyRef) -> Option<RealizationResult> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store realization result in cache
    pub fn store_realization_result(&self, _ontology: &OntologyRef, _result: RealizationResult) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get instance result from cache
    #[must_use]
    pub fn get_instance_result(
        &self,
        _individual: &Individual,
        _class: &ClassExpression,
    ) -> Option<bool> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store instance result in cache
    pub fn store_instance_result(
        &self,
        _individual: Individual,
        _class: ClassExpression,
        _result: bool,
    ) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get subsumption cache
    #[must_use]
    pub fn subsumption(&self, sub: &ClassExpression, sup: &ClassExpression) -> Option<bool> {
        self.get_subsumption_result(sub, sup)
    }

    /// Store subsumption cache
    pub fn store_subsumption(&self, sub: ClassExpression, sup: ClassExpression, result: bool) {
        self.cache_subsumption_result(sub, sup, result);
    }

    /// Get classification cache
    pub fn classification(&self, ontology: &OntologyRef) -> Option<ClassificationResult> {
        self.get_classification_result(ontology)
    }

    /// Store classification cache
    pub fn store_classification(&self, ontology: &OntologyRef, result: ClassificationResult) {
        self.store_classification_result(ontology, result);
    }

    /// Get realization cache
    pub fn realization(&self, ontology: &OntologyRef) -> Option<RealizationResult> {
        self.get_realization_result(ontology)
    }

    /// Store realization cache
    pub fn store_realization(&self, ontology: &OntologyRef, result: RealizationResult) {
        self.store_realization_result(ontology, result);
    }

    /// Get cache statistics
    #[must_use]
    pub fn get_stats(&self) -> CacheStats {
        let metrics = self.concept_cache.get_metrics();
        let total_accesses = metrics.hits + metrics.misses;
        let hit_rate = if total_accesses > 0 {
            metrics.hits as f64 / total_accesses as f64
        } else {
            0.0
        };

        let graph_metrics = self.completion_graph_cache.get_metrics();
        let graph_total_accesses = graph_metrics.hits + graph_metrics.misses;
        let graph_hit_rate = if graph_total_accesses > 0 {
            graph_metrics.hits as f64 / graph_total_accesses as f64
        } else {
            0.0
        };

        // Get quoted triple optimizer stats
        let qt_stats = self.quoted_triple_optimizer.stats();

        CacheStats {
            concept_cache_size: self.concept_cache.size(),
            concept_cache_hit_rate: hit_rate,
            completion_graph_cache_memory: self.completion_graph_cache.memory_usage(),
            completion_graph_cache_hit_rate: graph_hit_rate,
            quoted_triple_cache_hit_rate: qt_stats.hit_rate(),
            quoted_triple_intern_pool_size: qt_stats.intern_pool.pool_size,
            quoted_triple_memory_saved_bytes: qt_stats.intern_pool.memory_saved_bytes,
            total_memory_bytes: self.estimated_memory_usage(),
            hit_count: metrics.hits,
            miss_count: metrics.misses,
            eviction_count: metrics.evictions,
        }
    }

    /// Get the concept satisfiability cache
    #[must_use]
    pub fn concept_cache(&self) -> &ConceptSatisfiabilityCache {
        &self.concept_cache
    }

    /// Get the completion graph cache
    #[must_use]
    pub fn completion_graph_cache(&self) -> &CompletionGraphCache {
        &self.completion_graph_cache
    }

    /// Get the quoted triple optimizer (RDF-star)
    #[must_use]
    pub fn quoted_triple_optimizer(&self) -> &QuotedTripleOptimizer {
        &self.quoted_triple_optimizer
    }

    /// Get a completion graph from cache
    pub fn get_completion_graph(&self, signature: u64) -> Option<Arc<CompletedGraph>> {
        self.completion_graph_cache.get(signature)
    }

    /// Store a completion graph in cache
    pub fn store_completion_graph(&self, graph: Arc<CompletedGraph>) {
        self.completion_graph_cache.put(graph);
    }

    /// Get completion graph cache memory usage
    pub fn completion_graph_memory_usage(&self) -> usize {
        self.completion_graph_cache.memory_usage()
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new(CacheConfig::default())
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub concept_cache_size: usize,
    pub concept_cache_hit_rate: f64,
    pub completion_graph_cache_memory: usize,
    pub completion_graph_cache_hit_rate: f64,
    pub quoted_triple_cache_hit_rate: f64,
    pub quoted_triple_intern_pool_size: usize,
    pub quoted_triple_memory_saved_bytes: u64,
    pub total_memory_bytes: usize,
    pub hit_count: u64,
    pub miss_count: u64,
    pub eviction_count: u64,
}

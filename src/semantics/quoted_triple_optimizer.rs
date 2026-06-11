//! Quoted Triple Optimization Module
//!
//! This module provides performance optimizations for RDF-star quoted triples including:
//! - Triple interning for deduplication
//! - Caching for repeated operations
//! - Memory tracking and statistics
//! - Zero-overhead mode for RDF 1.1 compatibility

use super::Triple;
use crate::{
    core::lock_helpers::{read_lock, write_lock},
    error::Result,
};
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

/// Fast hash for triple deduplication
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TripleHash(u64);

impl TripleHash {
    fn from_triple(triple: &Triple) -> Self {
        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        triple.hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Interning pool for quoted triples
/// Deduplicates identical quoted triples to save memory
#[derive(Debug, Clone)]
pub struct QuotedTripleInternPool {
    /// Map from triple hash to interned triple
    pool: Arc<RwLock<HashMap<TripleHash, Arc<Triple>>>>,
    /// Statistics
    stats: Arc<RwLock<InternPoolStats>>,
    /// Whether interning is enabled (disabled for RDF 1.1 mode)
    enabled: bool,
}

/// Statistics for the intern pool
#[derive(Debug, Clone, Default)]
pub struct InternPoolStats {
    /// Total number of triples interned
    pub total_interns: u64,
    /// Number of cache hits (reused triples)
    pub hits: u64,
    /// Number of cache misses (new triples)
    pub misses: u64,
    /// Current pool size
    pub pool_size: usize,
    /// Memory saved through deduplication (estimated bytes)
    pub memory_saved_bytes: u64,
}

impl QuotedTripleInternPool {
    /// Create a new intern pool
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            pool: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(InternPoolStats::default())),
            enabled,
        }
    }

    /// Create a disabled intern pool (zero overhead for RDF 1.1 mode)
    #[must_use]
    pub fn disabled() -> Self {
        Self::new(false)
    }

    /// Intern a quoted triple
    /// Returns an Arc to the interned triple
    pub fn intern(&self, triple: Triple) -> Result<Arc<Triple>> {
        if !self.enabled {
            // Zero overhead mode: just wrap in Arc
            return Ok(Arc::new(triple));
        }

        let hash = TripleHash::from_triple(&triple);

        // Check if already interned
        {
            let pool = read_lock(&self.pool, "intern pool: read")?;
            if let Some(interned) = pool.get(&hash) {
                // Update stats
                let mut stats = write_lock(&self.stats, "intern stats: write")?;
                stats.hits += 1;
                return Ok(Arc::clone(interned));
            }
        }

        // Not found - intern it
        let arc_triple = Arc::new(triple);
        {
            let mut pool = write_lock(&self.pool, "intern pool: write")?;
            pool.insert(hash, Arc::clone(&arc_triple));

            // Update stats
            let mut stats = write_lock(&self.stats, "intern stats: write on miss")?;
            stats.misses += 1;
            stats.total_interns += 1;
            stats.pool_size = pool.len();

            // Estimate memory saved (very rough: assume 200 bytes per triple)
            if stats.hits > 0 {
                stats.memory_saved_bytes = stats.hits * 200;
            }
        }

        Ok(arc_triple)
    }

    /// Get current statistics
    pub fn stats(&self) -> Result<InternPoolStats> {
        Ok(read_lock(&self.stats, "intern stats: read")?.clone())
    }

    /// Clear the intern pool
    pub fn clear(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        write_lock(&self.pool, "intern pool: clear")?.clear();
        *write_lock(&self.stats, "intern stats: clear")? = InternPoolStats::default();
        Ok(())
    }

    /// Get the number of interned triples
    pub fn size(&self) -> Result<usize> {
        if !self.enabled {
            return Ok(0);
        }
        Ok(read_lock(&self.pool, "intern pool: size")?.len())
    }

    /// Check if a triple is already interned
    pub fn contains(&self, triple: &Triple) -> Result<bool> {
        if !self.enabled {
            return Ok(false);
        }
        let hash = TripleHash::from_triple(triple);
        Ok(read_lock(&self.pool, "intern pool: contains")?.contains_key(&hash))
    }
}

/// Cache for quoted triple operations
#[derive(Debug, Clone)]
pub struct QuotedTripleCache {
    /// Cache for depth calculations
    depth_cache: Arc<RwLock<HashMap<TripleHash, usize>>>,
    /// Cache for flattened triples
    flatten_cache: Arc<RwLock<HashMap<TripleHash, Vec<Triple>>>>,
    /// Cache for reification conversions
    reification_cache: Arc<RwLock<HashMap<TripleHash, Vec<Triple>>>>,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
    /// Whether caching is enabled
    enabled: bool,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub depth_hits: u64,
    pub depth_misses: u64,
    pub flatten_hits: u64,
    pub flatten_misses: u64,
    pub reification_hits: u64,
    pub reification_misses: u64,
}

impl QuotedTripleCache {
    /// Create a new quoted triple cache
    #[must_use]
    pub fn new(enabled: bool) -> Self {
        Self {
            depth_cache: Arc::new(RwLock::new(HashMap::new())),
            flatten_cache: Arc::new(RwLock::new(HashMap::new())),
            reification_cache: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(CacheStats::default())),
            enabled,
        }
    }

    /// Create a disabled cache (zero overhead for RDF 1.1 mode)
    #[must_use]
    pub fn disabled() -> Self {
        Self::new(false)
    }

    /// Get cached depth or None if not cached
    pub fn get_depth(&self, triple: &Triple) -> Result<Option<usize>> {
        if !self.enabled {
            return Ok(None);
        }

        let hash = TripleHash::from_triple(triple);
        let cache = read_lock(&self.depth_cache, "depth cache: read")?;

        if let Some(&depth) = cache.get(&hash) {
            write_lock(&self.stats, "depth stats: hit")?.depth_hits += 1;
            Ok(Some(depth))
        } else {
            write_lock(&self.stats, "depth stats: miss")?.depth_misses += 1;
            Ok(None)
        }
    }

    /// Cache a depth calculation
    pub fn cache_depth(&self, triple: &Triple, depth: usize) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let hash = TripleHash::from_triple(triple);
        write_lock(&self.depth_cache, "depth cache: write")?.insert(hash, depth);
        Ok(())
    }

    /// Get cached flattened triples or None if not cached
    pub fn get_flattened(&self, triple: &Triple) -> Result<Option<Vec<Triple>>> {
        if !self.enabled {
            return Ok(None);
        }

        let hash = TripleHash::from_triple(triple);
        let cache = read_lock(&self.flatten_cache, "flatten cache: read")?;

        if let Some(flattened) = cache.get(&hash) {
            write_lock(&self.stats, "flatten stats: hit")?.flatten_hits += 1;
            Ok(Some(flattened.clone()))
        } else {
            write_lock(&self.stats, "flatten stats: miss")?.flatten_misses += 1;
            Ok(None)
        }
    }

    /// Cache a flatten operation
    pub fn cache_flattened(&self, triple: &Triple, flattened: Vec<Triple>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let hash = TripleHash::from_triple(triple);
        write_lock(&self.flatten_cache, "flatten cache: write")?.insert(hash, flattened);
        Ok(())
    }

    /// Get cached reification or None if not cached
    pub fn get_reification(&self, triple: &Triple) -> Result<Option<Vec<Triple>>> {
        if !self.enabled {
            return Ok(None);
        }

        let hash = TripleHash::from_triple(triple);
        let cache = read_lock(&self.reification_cache, "reification cache: read")?;

        if let Some(reification) = cache.get(&hash) {
            write_lock(&self.stats, "reification stats: hit")?.reification_hits += 1;
            Ok(Some(reification.clone()))
        } else {
            write_lock(&self.stats, "reification stats: miss")?.reification_misses += 1;
            Ok(None)
        }
    }

    /// Cache a reification conversion
    pub fn cache_reification(&self, triple: &Triple, reification: Vec<Triple>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let hash = TripleHash::from_triple(triple);
        write_lock(&self.reification_cache, "reification cache: write")?.insert(hash, reification);
        Ok(())
    }

    /// Get current statistics
    pub fn stats(&self) -> Result<CacheStats> {
        Ok(read_lock(&self.stats, "cache stats: read")?.clone())
    }

    /// Clear all caches
    pub fn clear(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        write_lock(&self.depth_cache, "depth cache: clear")?.clear();
        write_lock(&self.flatten_cache, "flatten cache: clear")?.clear();
        write_lock(&self.reification_cache, "reification cache: clear")?.clear();
        *write_lock(&self.stats, "cache stats: clear")? = CacheStats::default();
        Ok(())
    }

    /// Get total cache size (number of entries)
    pub fn size(&self) -> Result<usize> {
        if !self.enabled {
            return Ok(0);
        }

        let depth_size = read_lock(&self.depth_cache, "depth cache: size")?.len();
        let flatten_size = read_lock(&self.flatten_cache, "flatten cache: size")?.len();
        let reification_size = read_lock(&self.reification_cache, "reification cache: size")?.len();
        Ok(depth_size + flatten_size + reification_size)
    }
}

/// Configuration for quoted triple optimizations
#[derive(Debug, Clone)]
pub struct QuotedTripleOptimizerConfig {
    /// Enable interning (deduplication)
    pub enable_interning: bool,
    /// Enable operation caching
    pub enable_caching: bool,
    /// Maximum intern pool size (0 = unlimited)
    pub max_pool_size: usize,
    /// Maximum cache size (0 = unlimited)
    pub max_cache_size: usize,
}

impl Default for QuotedTripleOptimizerConfig {
    fn default() -> Self {
        Self {
            enable_interning: true,
            enable_caching: true,
            max_pool_size: 10000,
            max_cache_size: 5000,
        }
    }
}

impl QuotedTripleOptimizerConfig {
    /// Create a configuration for RDF 1.1 mode (zero overhead)
    #[must_use]
    pub fn rdf11_mode() -> Self {
        Self {
            enable_interning: false,
            enable_caching: false,
            max_pool_size: 0,
            max_cache_size: 0,
        }
    }

    /// Create a configuration for maximum performance
    #[must_use]
    pub fn max_performance() -> Self {
        Self {
            enable_interning: true,
            enable_caching: true,
            max_pool_size: 50000,
            max_cache_size: 25000,
        }
    }
}

/// Comprehensive optimizer for quoted triples
#[derive(Debug, Clone)]
pub struct QuotedTripleOptimizer {
    intern_pool: QuotedTripleInternPool,
    cache: QuotedTripleCache,
    config: QuotedTripleOptimizerConfig,
}

impl QuotedTripleOptimizer {
    /// Create a new optimizer with given configuration
    #[must_use]
    pub fn new(config: QuotedTripleOptimizerConfig) -> Self {
        Self {
            intern_pool: QuotedTripleInternPool::new(config.enable_interning),
            cache: QuotedTripleCache::new(config.enable_caching),
            config,
        }
    }

    /// Create a disabled optimizer (zero overhead for RDF 1.1 mode)
    #[must_use]
    pub fn disabled() -> Self {
        Self::new(QuotedTripleOptimizerConfig::rdf11_mode())
    }

    /// Intern a quoted triple
    pub fn intern(&self, triple: Triple) -> Result<Arc<Triple>> {
        self.intern_pool.intern(triple)
    }

    /// Get depth with caching
    pub fn depth(&self, triple: &Triple) -> Result<usize> {
        if let Some(cached) = self.cache.get_depth(triple)? {
            return Ok(cached);
        }

        // Compute depth
        let depth = triple.depth();
        self.cache.cache_depth(triple, depth)?;
        Ok(depth)
    }

    /// Flatten with caching
    pub fn flatten(&self, triple: &Triple) -> Result<Vec<Triple>> {
        if let Some(cached) = self.cache.get_flattened(triple)? {
            return Ok(cached);
        }

        // Compute flattened
        let flattened = triple.flatten();
        self.cache.cache_flattened(triple, flattened.clone())?;
        Ok(flattened)
    }

    /// Convert to reification with caching
    pub fn to_reification(
        &self,
        triple: &Triple,
        statement_id: &str,
    ) -> crate::Result<Vec<Triple>> {
        if let Some(cached) = self.cache.get_reification(triple)? {
            return Ok(cached);
        }

        // Compute reification
        let reification = triple.to_rdf11_reification(statement_id)?;
        self.cache.cache_reification(triple, reification.clone())?;
        Ok(reification)
    }

    /// Get comprehensive statistics
    pub fn stats(&self) -> Result<OptimizerStats> {
        Ok(OptimizerStats {
            intern_pool: self.intern_pool.stats()?,
            cache: self.cache.stats()?,
        })
    }

    /// Clear all caches and pools
    pub fn clear(&self) -> Result<()> {
        self.intern_pool.clear()?;
        self.cache.clear()
    }

    /// Check if optimizations are enabled
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.config.enable_interning || self.config.enable_caching
    }
}

/// Combined statistics from all optimizers
#[derive(Debug, Clone)]
pub struct OptimizerStats {
    pub intern_pool: InternPoolStats,
    pub cache: CacheStats,
}

impl OptimizerStats {
    /// Total cache hit rate across all caches
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let total_hits =
            self.cache.depth_hits + self.cache.flatten_hits + self.cache.reification_hits;
        let total_operations = total_hits
            + self.cache.depth_misses
            + self.cache.flatten_misses
            + self.cache.reification_misses;

        if total_operations == 0 {
            0.0
        } else {
            (total_hits as f64) / (total_operations as f64)
        }
    }

    /// Intern pool hit rate
    #[must_use]
    pub fn intern_hit_rate(&self) -> f64 {
        let total = self.intern_pool.hits + self.intern_pool.misses;
        if total == 0 {
            0.0
        } else {
            (self.intern_pool.hits as f64) / (total as f64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::semantics::RdfTerm;
    use url::Url;

    fn create_test_triple() -> Triple {
        Triple::new(
            RdfTerm::Iri(Url::parse("http://example.org/s").unwrap()),
            RdfTerm::Iri(Url::parse("http://example.org/p").unwrap()),
            RdfTerm::Iri(Url::parse("http://example.org/o").unwrap()),
        )
    }

    #[test]
    fn test_intern_pool_basic() {
        let pool = QuotedTripleInternPool::new(true);
        let triple = create_test_triple();

        let interned1 = pool.intern(triple.clone()).expect("intern should succeed");
        let interned2 = pool.intern(triple.clone()).expect("intern should succeed");

        // Should be the same Arc
        assert!(Arc::ptr_eq(&interned1, &interned2));

        let stats = pool.stats().expect("stats should succeed");
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn test_intern_pool_disabled() {
        let pool = QuotedTripleInternPool::disabled();
        let triple = create_test_triple();

        let interned1 = pool.intern(triple.clone()).expect("intern should succeed");
        let interned2 = pool.intern(triple.clone()).expect("intern should succeed");

        // Different Arcs in disabled mode
        assert!(!Arc::ptr_eq(&interned1, &interned2));

        let stats = pool.stats().expect("stats should succeed");
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_cache_depth() {
        let cache = QuotedTripleCache::new(true);
        let triple = create_test_triple();

        // First access - miss
        assert!(
            cache
                .get_depth(&triple)
                .expect("get_depth should succeed")
                .is_none()
        );
        cache
            .cache_depth(&triple, 5)
            .expect("cache_depth should succeed");

        // Second access - hit
        assert_eq!(
            cache.get_depth(&triple).expect("get_depth should succeed"),
            Some(5)
        );

        let stats = cache.stats().expect("stats should succeed");
        assert_eq!(stats.depth_hits, 1);
        assert_eq!(stats.depth_misses, 1);
    }

    #[test]
    fn test_cache_disabled() {
        let cache = QuotedTripleCache::disabled();
        let triple = create_test_triple();

        // Always misses in disabled mode
        assert!(cache.get_depth(&triple).expect("ok").is_none());
        cache.cache_depth(&triple, 5).expect("ok");
        assert!(cache.get_depth(&triple).expect("ok").is_none());

        let stats = cache.stats().expect("stats should succeed");
        assert_eq!(stats.depth_hits, 0);
        assert_eq!(stats.depth_misses, 0);
    }

    #[test]
    fn test_optimizer_comprehensive() {
        let config = QuotedTripleOptimizerConfig::default();
        let optimizer = QuotedTripleOptimizer::new(config);

        let triple = create_test_triple();

        // Test interning
        let interned1 = optimizer.intern(triple.clone()).expect("intern ok");
        let interned2 = optimizer.intern(triple.clone()).expect("intern ok");
        assert!(Arc::ptr_eq(&interned1, &interned2));

        // Test depth caching
        let depth1 = optimizer.depth(&triple).expect("depth ok");
        let depth2 = optimizer.depth(&triple).expect("depth ok");
        assert_eq!(depth1, depth2);

        // Check stats
        let stats = optimizer.stats().expect("stats ok");
        assert!(stats.intern_hit_rate() > 0.0);
        assert!(stats.hit_rate() > 0.0);
    }

    #[test]
    fn test_optimizer_disabled() {
        let optimizer = QuotedTripleOptimizer::disabled();
        assert!(!optimizer.is_enabled());

        let triple = create_test_triple();

        // Should work but with no caching
        let depth = optimizer.depth(&triple).expect("depth ok");
        assert_eq!(depth, 0);

        let stats = optimizer.stats().expect("stats ok");
        assert_eq!(stats.hit_rate(), 0.0);
    }
}

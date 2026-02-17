//! Lock-Free Cache System using `DashMap`
//!
//! This module provides high-performance concurrent caching without traditional locks,
//! using `DashMap` for lock-free concurrent access patterns inspired by Konclude's approach.

use crate::{cache::CacheFeature, ontology::ClassExpression, reasoning::ClassificationResult};

use dashmap::DashMap;
use enumset::EnumSet;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

/// Cache entry with TTL support (lock-free)
#[derive(Debug)]
pub struct LockFreeCacheEntry<T: Clone> {
    pub value: T,
    pub timestamp: Instant,
    pub hit_count: AtomicU64,
}

impl<T: Clone> Clone for LockFreeCacheEntry<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            timestamp: self.timestamp,
            hit_count: AtomicU64::new(self.hit_count.load(Ordering::Relaxed)),
        }
    }
}

impl<T: Clone> LockFreeCacheEntry<T> {
    #[must_use]
    pub fn new(value: T) -> Self {
        Self {
            value,
            timestamp: Instant::now(),
            hit_count: AtomicU64::new(0),
        }
    }

    #[must_use]
    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }

    pub fn hit(&self) {
        self.hit_count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_hit_count(&self) -> u64 {
        self.hit_count.load(Ordering::Relaxed)
    }
}

/// Lock-free cache metrics
#[derive(Debug, Default)]
pub struct LockFreeCacheMetrics {
    pub hits: AtomicU64,
    pub misses: AtomicU64,
    pub evictions: AtomicU64,
}

impl LockFreeCacheMetrics {
    pub fn record_hit(&self) {
        self.hits.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_miss(&self) {
        self.misses.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_eviction(&self) {
        self.evictions.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_hit_rate(&self) -> f64 {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;

        if total == 0 {
            0.0
        } else {
            // Hit rate calculation: precision loss only occurs beyond 2^52 cache accesses (~4.5 quadrillion)
            // which is impractical for in-memory caching. F64 provides sufficient precision for statistics.
            #[allow(clippy::cast_precision_loss)]
            let rate = hits as f64 / total as f64;
            rate
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct LockFreeCacheConfig {
    pub max_size: usize,
    pub ttl: Duration,
    pub features: EnumSet<CacheFeature>,
}

impl LockFreeCacheConfig {
    /// Check if a specific cache feature is enabled
    #[must_use]
    pub fn is_enabled(&self, feature: CacheFeature) -> bool {
        self.features.contains(feature)
    }

    /// Enable a cache feature
    pub fn enable(&mut self, feature: CacheFeature) {
        self.features.insert(feature);
    }

    /// Disable a cache feature
    pub fn disable(&mut self, feature: CacheFeature) {
        self.features.remove(feature);
    }
}

impl Default for LockFreeCacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10000,
            ttl: Duration::from_secs(3600),
            features: CacheFeature::Concept
                | CacheFeature::Subsumption
                | CacheFeature::Satisfiability
                | CacheFeature::Classification
                | CacheFeature::Realization,
        }
    }
}

/// Lock-free concept satisfiability cache using `DashMap`
#[derive(Debug)]
pub struct LockFreeConceptCache {
    cache: DashMap<ClassExpression, LockFreeCacheEntry<bool>>,
    config: LockFreeCacheConfig,
    metrics: Arc<LockFreeCacheMetrics>,
    size: Arc<AtomicUsize>,
}

impl LockFreeConceptCache {
    #[must_use]
    pub fn new(config: LockFreeCacheConfig) -> Self {
        Self {
            cache: DashMap::with_capacity(config.max_size),
            config,
            metrics: Arc::new(LockFreeCacheMetrics::default()),
            size: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[must_use]
    pub fn get(&self, expression: &ClassExpression) -> Option<bool> {
        if !self.config.is_enabled(CacheFeature::Satisfiability) {
            return None;
        }

        if let Some(entry) = self.cache.get(expression) {
            if entry.is_expired(self.config.ttl) {
                // Remove expired entry
                drop(entry); // Release read lock before removing
                self.cache.remove(expression);
                self.size.fetch_sub(1, Ordering::Relaxed);
                self.metrics.record_miss();
                None
            } else {
                entry.hit();
                self.metrics.record_hit();
                Some(entry.value)
            }
        } else {
            self.metrics.record_miss();
            None
        }
    }

    pub fn put(&self, expression: ClassExpression, result: bool) {
        if !self.config.is_enabled(CacheFeature::Satisfiability) {
            return;
        }

        // Check size and evict if necessary
        while self.size.load(Ordering::Relaxed) >= self.config.max_size {
            self.evict_one();
        }

        if self
            .cache
            .insert(expression, LockFreeCacheEntry::new(result))
            .is_none()
        {
            self.size.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn evict_one(&self) {
        // Evict the first entry (simple FIFO for now; can be improved with LRU)
        if let Some(entry) = self.cache.iter().next() {
            let key = entry.key().clone();
            drop(entry); // Release read lock
            self.cache.remove(&key);
            self.size.fetch_sub(1, Ordering::Relaxed);
            self.metrics.record_eviction();
        }
    }

    pub fn clear(&self) {
        self.cache.clear();
        self.size.store(0, Ordering::Relaxed);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        self.metrics.get_hit_rate()
    }
}

/// Lock-free subsumption cache
#[derive(Debug)]
pub struct LockFreeSubsumptionCache {
    cache: DashMap<(ClassExpression, ClassExpression), LockFreeCacheEntry<bool>>,
    config: LockFreeCacheConfig,
    metrics: Arc<LockFreeCacheMetrics>,
    size: Arc<AtomicUsize>,
}

impl LockFreeSubsumptionCache {
    #[must_use]
    pub fn new(config: LockFreeCacheConfig) -> Self {
        Self {
            cache: DashMap::with_capacity(config.max_size),
            config,
            metrics: Arc::new(LockFreeCacheMetrics::default()),
            size: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[must_use]
    pub fn get(&self, subclass: &ClassExpression, superclass: &ClassExpression) -> Option<bool> {
        if !self.config.is_enabled(CacheFeature::Subsumption) {
            return None;
        }

        let key = (subclass.clone(), superclass.clone());
        if let Some(entry) = self.cache.get(&key) {
            if entry.is_expired(self.config.ttl) {
                drop(entry);
                self.cache.remove(&key);
                self.size.fetch_sub(1, Ordering::Relaxed);
                self.metrics.record_miss();
                None
            } else {
                entry.hit();
                self.metrics.record_hit();
                Some(entry.value)
            }
        } else {
            self.metrics.record_miss();
            None
        }
    }

    pub fn put(&self, subclass: ClassExpression, superclass: ClassExpression, result: bool) {
        if !self.config.is_enabled(CacheFeature::Subsumption) {
            return;
        }

        while self.size.load(Ordering::Relaxed) >= self.config.max_size {
            self.evict_one();
        }

        let key = (subclass, superclass);
        if self
            .cache
            .insert(key, LockFreeCacheEntry::new(result))
            .is_none()
        {
            self.size.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn evict_one(&self) {
        if let Some(entry) = self.cache.iter().next() {
            let key = entry.key().clone();
            drop(entry);
            self.cache.remove(&key);
            self.size.fetch_sub(1, Ordering::Relaxed);
            self.metrics.record_eviction();
        }
    }

    pub fn clear(&self) {
        self.cache.clear();
        self.size.store(0, Ordering::Relaxed);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        self.metrics.get_hit_rate()
    }
}

/// Lock-free classification result cache
#[derive(Debug)]
pub struct LockFreeClassificationCache {
    cache: DashMap<String, LockFreeCacheEntry<ClassificationResult>>,
    config: LockFreeCacheConfig,
    metrics: Arc<LockFreeCacheMetrics>,
    size: Arc<AtomicUsize>,
}

impl LockFreeClassificationCache {
    #[must_use]
    pub fn new(config: LockFreeCacheConfig) -> Self {
        Self {
            cache: DashMap::with_capacity(16), // Typically small number of ontologies
            config,
            metrics: Arc::new(LockFreeCacheMetrics::default()),
            size: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[must_use]
    pub fn get(&self, ontology_iri: &str) -> Option<ClassificationResult> {
        if !self.config.is_enabled(CacheFeature::Classification) {
            return None;
        }

        if let Some(entry) = self.cache.get(ontology_iri) {
            if entry.is_expired(self.config.ttl) {
                drop(entry);
                self.cache.remove(ontology_iri);
                self.size.fetch_sub(1, Ordering::Relaxed);
                self.metrics.record_miss();
                None
            } else {
                entry.hit();
                self.metrics.record_hit();
                Some(entry.value.clone())
            }
        } else {
            self.metrics.record_miss();
            None
        }
    }

    pub fn put(&self, ontology_iri: String, result: ClassificationResult) {
        if !self.config.is_enabled(CacheFeature::Classification) {
            return;
        }

        if self
            .cache
            .insert(ontology_iri, LockFreeCacheEntry::new(result))
            .is_none()
        {
            self.size.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn clear(&self) {
        self.cache.clear();
        self.size.store(0, Ordering::Relaxed);
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.size.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        self.metrics.get_hit_rate()
    }
}

/// Unified lock-free cache manager
#[derive(Debug)]
pub struct LockFreeCacheManager {
    pub concept_cache: LockFreeConceptCache,
    pub subsumption_cache: LockFreeSubsumptionCache,
    pub classification_cache: LockFreeClassificationCache,
}

impl LockFreeCacheManager {
    #[must_use]
    pub fn new(config: LockFreeCacheConfig) -> Self {
        Self {
            concept_cache: LockFreeConceptCache::new(config.clone()),
            subsumption_cache: LockFreeSubsumptionCache::new(config.clone()),
            classification_cache: LockFreeClassificationCache::new(config),
        }
    }

    pub fn clear_all(&self) {
        self.concept_cache.clear();
        self.subsumption_cache.clear();
        self.classification_cache.clear();
    }

    #[must_use]
    pub fn total_size(&self) -> usize {
        self.concept_cache.len() + self.subsumption_cache.len() + self.classification_cache.len()
    }

    pub fn print_stats(&self) {
        println!("=== Lock-Free Cache Statistics ===");
        println!(
            "Concept Cache: {} entries, {:.1}% hit rate",
            self.concept_cache.len(),
            self.concept_cache.hit_rate() * 100.0
        );
        println!(
            "Subsumption Cache: {} entries, {:.1}% hit rate",
            self.subsumption_cache.len(),
            self.subsumption_cache.hit_rate() * 100.0
        );
        println!(
            "Classification Cache: {} entries, {:.1}% hit rate",
            self.classification_cache.len(),
            self.classification_cache.hit_rate() * 100.0
        );
        println!("Total Entries: {}", self.total_size());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};

    #[test]
    fn test_lock_free_concept_cache() {
        let config = LockFreeCacheConfig::default();
        let cache = LockFreeConceptCache::new(config);

        let concept = ClassExpression::Class(Class {
            iri: IRI::new("test"),
        });

        assert_eq!(cache.get(&concept), None);

        cache.put(concept.clone(), true);
        assert_eq!(cache.get(&concept), Some(true));
        assert_eq!(cache.len(), 1);

        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_lock_free_subsumption_cache() {
        let config = LockFreeCacheConfig::default();
        let cache = LockFreeSubsumptionCache::new(config);

        let sub = ClassExpression::Class(Class { iri: IRI::new("A") });
        let sup = ClassExpression::Class(Class { iri: IRI::new("B") });

        assert_eq!(cache.get(&sub, &sup), None);

        cache.put(sub.clone(), sup.clone(), true);
        assert_eq!(cache.get(&sub, &sup), Some(true));
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn test_concurrent_access() {
        use std::thread;

        let config = LockFreeCacheConfig::default();
        let cache = Arc::new(LockFreeConceptCache::new(config));

        let handles: Vec<_> = (0..10)
            .map(|i| {
                let cache = cache.clone();
                thread::spawn(move || {
                    let concept = ClassExpression::Class(Class {
                        iri: format!("test{}", i).into(),
                    });
                    cache.put(concept.clone(), true);
                    cache.get(&concept)
                })
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.join().unwrap(), Some(true));
        }

        assert_eq!(cache.len(), 10);
    }
}

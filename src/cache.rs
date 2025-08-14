//! Cache System for Oxidowl
//!
//! This module implements efficient caching strategies for ontology reasoning,
//! including concept and role satisfiability caches, subsumption caches,
//! and inference caches.

use crate::{
    ontology::{ClassExpression, Individual, Ontology, OntologyRef},
    reasoning::{ClassificationResult, RealizationResult},
};

use std::{
    collections::HashMap,
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
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10000,                // Default maximum size
            ttl: Duration::from_secs(3600), // Default TTL of 1 hour
            enable_concept_cache: true,
            enable_subsumption_cache: true,
            enable_satisfiability_cache: true,
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
}

impl ConceptSatisfiabilityCache {
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    #[must_use]
    pub fn get(&self, expression: &ClassExpression) -> Option<bool> {
        if !self.config.enable_satisfiability_cache {
            return None; // Cache is disabled
        }

        let mut cache = self.cache.write().unwrap();
        if let Some(entry) = cache.get_mut(expression) {
            if entry.is_expired(self.config.ttl) {
                cache.remove(expression); // Remove expired entry
                None
            } else {
                entry.hit(); // Increment hit count
                Some(entry.value)
            }
        } else {
            // If not found, we can return None
            None
        }
    }

    pub fn put(&self, expression: ClassExpression, result: bool) {
        if !self.config.enable_satisfiability_cache {
            return; // Cache is disabled
        }

        let mut cache = self.cache.write().unwrap();

        if cache.len() >= self.config.max_size {
            // Evict the oldest entry if max size exceeded
            self.evict_lru(&mut cache);
        }
        cache.insert(expression, CacheEntry::new(result));
    }

    fn evict_lru(&self, cache: &mut HashMap<ClassExpression, CacheEntry<bool>>) {
        if let Some((key, _)) = cache.iter().min_by_key(|(_, entry)| entry.timestamp) {
            let key_to_remove = key.clone();
            cache.remove(&key_to_remove);
        }
    }

    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }

    #[must_use]
    pub fn size(&self) -> usize {
        self.cache.read().unwrap().len()
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let cache = self.cache.read().unwrap();

        let total_hits: u64 = cache.values().map(|entry| entry.hit_count).sum();
        let entries = cache.len() as u64;

        if entries == 0 {
            0.0 // Avoid division by zero
        } else {
            total_hits as f64 / cache.len() as f64
        }
    }
}

/// Cache manager that coordinates different caches
#[derive(Debug, Clone)]
pub struct CacheManager {
    concept_cache: ConceptSatisfiabilityCache,
    config: CacheConfig,
}

impl CacheManager {
    #[must_use]
    pub fn new(config: CacheConfig) -> Self {
        Self {
            concept_cache: ConceptSatisfiabilityCache::new(config.clone()),
            config,
        }
    }

    /// Clear all caches
    pub fn clear_all(&self) {
        self.concept_cache.clear();
    }

    /// Get consistency result from cache
    pub fn get_consistency_result(&self, ontology: &OntologyRef) -> Option<bool> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store consistency result in cache
    pub fn cache_consistency_result(&self, ontology: &OntologyRef, result: bool) {
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
        sub: &ClassExpression,
        sup: &ClassExpression,
    ) -> Option<bool> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store subsumption result in cache
    pub fn cache_subsumption_result(
        &self,
        sub: ClassExpression,
        sup: ClassExpression,
        result: bool,
    ) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get classification result from cache
    pub fn get_classification_result(
        &self,
        ontology: &OntologyRef,
    ) -> Option<ClassificationResult> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store classification result in cache
    pub fn store_classification_result(
        &self,
        ontology: &OntologyRef,
        result: ClassificationResult,
    ) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get realization result from cache
    pub fn get_realization_result(&self, ontology: &OntologyRef) -> Option<RealizationResult> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store realization result in cache
    pub fn store_realization_result(&self, ontology: &OntologyRef, result: RealizationResult) {
        // Simple implementation - would need more sophisticated caching in practice
    }

    /// Get instance result from cache
    #[must_use]
    pub fn get_instance_result(
        &self,
        individual: &Individual,
        class: &ClassExpression,
    ) -> Option<bool> {
        // Simple implementation - would need more sophisticated caching in practice
        None
    }

    /// Store instance result in cache
    pub fn store_instance_result(
        &self,
        individual: Individual,
        class: ClassExpression,
        result: bool,
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
        CacheStats {
            concept_cache_size: self.concept_cache.size(),
            concept_cache_hit_rate: self.concept_cache.hit_rate(),
        }
    }

    /// Get the concept satisfiability cache
    #[must_use]
    pub fn concept_cache(&self) -> &ConceptSatisfiabilityCache {
        &self.concept_cache
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
}

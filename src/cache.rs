//! Cache System for Oxidowl
//! 
//! This module implements efficient caching strategies for ontology reasoning,
//! including concept and role satisfiability caches, subsumption caches,
//! and inference caches.

use crate::{
    Error, Result,
    ontology::{Ontology, ClassExpression, Individual, Class, IRI},
    core::{
        reasoner::ReasoningTask,
        tableau::{TableauNode, TableauEdge},
    },
};

use std::{
    collections::{HashMap, HashSet},
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
    pub ttl: Duration, // Time to live for cache entries
    pub enable_concept_cache: bool,
    pub enable subsumption_cache: bool,
    pub enable_satisfiability_cache: bool,
    pub enable_classification_cache: bool,
    pub enable_realization_cache: bool,
}

imple Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size: 10000, // Default maximum size
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
    pub fn new(config: CacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            config,
        }
    }

    pub fn get(&self, expression: &ClassExpression) -> Option<bool> {
        if !self.config.enable_satisfiability_cache {
            return None; // Cache is disabled
        }

        let mut cache = self.cache.write().unwrap();
        if let Some(entry) = cache.get_mut(expression) {
            if !entry.is_expired(self.config.ttl) {
                entry.hit(); // Increment hit count
                return Some(entry.value);
            } else {
                cache.remove(expression); // Remove expired entry
            }
        } else {
            // If not found, we can return None
            return None;
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
            cache.remove(key);
        }
    }

    pub fn clear(&self) {
        self.cache.write().unwrap().clear();
    }

    pub fn size(&self) -> usize {
        self.cache.read().unwrap().len()
    }

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

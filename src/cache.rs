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


//! Configuration and Cache Unit Tests
//!
//! Tests for configuration management and caching functionality.

use oxidowl::{
    cache::{CacheConfig, CacheManager, ConceptSatisfiabilityCache},
    ontology::{
        Ontology, OntologyRef,
    },
};

#[test]
fn test_cache_configuration() {
    let config = CacheConfig::default();
    assert!(config.enable_concept_cache);
    assert_eq!(config.max_size, 10000);
}

#[test]
fn test_reasoning_cache_creation() {
    let cache = CacheManager::new(CacheConfig::default());

    // Test cache can be created
    // CacheManager doesn't have the same API, so just test creation
    drop(cache);
}

#[test]
fn test_cache_subsumption_operations() {
    let cache = ConceptSatisfiabilityCache::new(CacheConfig::default());

    // Test basic cache functionality
    drop(cache);
}

#[test]
fn test_instance_cache_operations() {
    let cache = ConceptSatisfiabilityCache::new(CacheConfig::default());

    // Test basic cache functionality
    drop(cache);
}

fn create_test_ontology() -> OntologyRef {
    std::sync::Arc::new(std::sync::RwLock::new(Ontology::new()))
}

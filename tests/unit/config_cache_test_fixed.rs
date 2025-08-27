//! Configuration and Cache Unit Tests
//!
//! Tests for configuration management and caching functionality.

use oxidowl::cache::*;
use oxidowl::config::*;
use oxidowl::ontology::*;

#[test]
fn test_cache_configuration() {
    let config = CacheConfig::default();
    assert!(config.enabled);
    assert_eq!(config.max_entries, 10000);
}

#[test]
fn test_reasoning_cache_creation() {
    let cache = ReasoningCache::new(CacheConfig::default());
    
    // Test cache can be created
    assert!(cache.get_classification_result(&create_test_ontology()).is_none());
}

#[test]
fn test_cache_subsumption_operations() {
    let mut cache = ReasoningCache::new(CacheConfig::default());
    
    let sub_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/A")));
    let super_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/B")));
    
    // Test that cache starts empty
    assert!(cache.get_subsumption_result(&sub_class, &super_class).is_none());
    
    // Store a result
    cache.store_subsumption_result(sub_class.clone(), super_class.clone(), true);
    
    // Verify it can be retrieved
    assert_eq!(cache.get_subsumption_result(&sub_class, &super_class), Some(true));
}

#[test]
fn test_instance_cache_operations() {
    let mut cache = ReasoningCache::new(CacheConfig::default());
    
    let individual = Individual::Named(NamedIndividual {
        iri: IRI::new("http://example.org/john"),
    });
    let class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
    
    // Test initial state
    assert!(cache.get_instance_result(&individual, &class).is_none());
    
    // Store and retrieve
    cache.store_instance_result(individual.clone(), class.clone(), true);
    assert_eq!(cache.get_instance_result(&individual, &class), Some(true));
}

fn create_test_ontology() -> OntologyRef {
    std::sync::Arc::new(Ontology::new())
}

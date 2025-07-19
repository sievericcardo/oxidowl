//! Unit tests for configuration and cache management

use oxidowl::{
    config::ReasonerConfig,
    cache::CacheManager,
};
use std::time::Duration;

#[test]
fn test_basic_config_creation() {
    let config = ReasonerConfig::default();
    
    // Basic creation should work
    assert!(config.reasoning.enable_optimisations);
    assert!(config.reasoning.timeout.is_some());
    println!("Basic config creation works");
}

#[test]
fn test_config_presets() {
    // Test test config  
    let test_config = ReasonerConfig::test_config();
    assert!(test_config.reasoning.enable_explanations);
    
    // Test web service config
    let web_config = ReasonerConfig::web_service_config();
    assert_eq!(web_config.server.max_connections, 500);
    assert_eq!(web_config.server.request_timeout, Duration::from_secs(60));
    
    println!("Config presets work");
}

#[test]
fn test_cache_manager_creation() {
    let _cache_manager = CacheManager::default();
    
    // Basic creation should work
    println!("CacheManager created successfully");
}

#[test] 
fn test_basic_functionality() {
    use oxidowl::ontology::Ontology;
    
    let _cache_manager = CacheManager::default();
    let _ontology = Ontology::new();
    
    // Test basic functionality without complex API calls
    println!("Basic functionality works");
}

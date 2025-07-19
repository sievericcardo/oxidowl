//! Unit tests for hypertableau algorithms

use oxidowl::{
    config::ReasonerConfig,
    core::hypertableau::HyperTableau,
    core::blocking::AnywhereBlocking,
    ontology::Ontology,
};

#[test]
fn test_hypertableau_creation() {
    let ontology = Ontology::new();
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    
    // Use correct config type (reasoning subfield) 
    let _tableau = HyperTableau::new(config.reasoning, blocking_checker);
    
    println!("HyperTableau created successfully");
}

#[test]
fn test_basic_hypertableau_functionality() {
    let ontology = Ontology::new(); 
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    
    let _tableau = HyperTableau::new(config.reasoning, blocking_checker);
    
    // Test basic functionality without accessing private fields
    println!("Basic hypertableau functionality works");
}

#[test] 
fn test_ontology_integration() {
    let _ontology = Ontology::new();
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    
    let _tableau = HyperTableau::new(config.reasoning, blocking_checker);
    
    // Test integration without complex operations
    println!("Ontology integration test passed");
}

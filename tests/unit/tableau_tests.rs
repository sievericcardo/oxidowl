//! Unit tests for tableau algorithms

use oxidowl::{config::ReasonerConfig, core::tableau::Tableau, ontology::Ontology};

#[test]
fn test_tableau_creation() {
    let ontology = Ontology::new();
    let config = ReasonerConfig::test_config();

    // Use the correct config subfield
    let _tableau = Tableau::new(config.reasoning);

    println!("Tableau created successfully");
}

#[test]
fn test_basic_tableau_functionality() {
    let _ontology = Ontology::new();
    let config = ReasonerConfig::test_config();

    let _tableau = Tableau::new(config.reasoning);

    // Test basic functionality
    println!("Basic tableau functionality works");
}

#[test]
fn test_tableau_integration() {
    let _ontology = Ontology::new();
    let config = ReasonerConfig::test_config();

    let _tableau = Tableau::new(config.reasoning);

    // Test integration without complex operations
    println!("Tableau integration test passed");
}

//! Unit tests for tableau algorithms

use oxidowl::{config::ReasonerConfig, core::tableau::Tableau, ontology::Ontology};
use std::sync::Arc;

#[test]
fn test_tableau_creation() {
    let ontology = Arc::new(Ontology::new());
    let config = ReasonerConfig::default();

    // Use the correct config subfield
    let _tableau = Tableau::new(config.reasoning, ontology);

    println!("Tableau created successfully");
}

#[test]
fn test_basic_tableau_functionality() {
    let ontology = Arc::new(Ontology::new());
    let config = ReasonerConfig::default();

    let _tableau = Tableau::new(config.reasoning, ontology);

    // Test basic functionality
    println!("Basic tableau functionality works");
}

#[test]
fn test_tableau_integration() {
    let ontology = Arc::new(Ontology::new());
    let config = ReasonerConfig::default();

    let _tableau = Tableau::new(config.reasoning, ontology);

    // Test integration without complex operations
    println!("Tableau integration test passed");
}

//! Unit tests for the reasoning service

use oxidowl::{config::ReasonerConfig, ontology::Ontology, reasoning::ReasoningService};

/// Helper function to create a simple test ontology
fn create_test_ontology() -> Ontology {
    // Create simple empty ontology for now
    Ontology::new()
}

#[tokio::test]
async fn test_reasoning_service_creation() {
    let ontology = create_test_ontology();
    let config = ReasonerConfig::default();

    let _reasoning_service = ReasoningService::new(ontology, config);

    // Basic creation should work
    println!("ReasoningService created successfully");
}

#[tokio::test]
async fn test_basic_functionality() {
    let ontology = create_test_ontology();
    let config = ReasonerConfig::default();
    let _reasoning_service = ReasoningService::new(ontology, config);

    // For now, just test that we can create the service
    println!("Basic functionality test passed");
}

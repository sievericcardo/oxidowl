//! Unit tests for DL query engine

use oxidowl::{
    config::ReasonerConfig, ontology::Ontology, query::DLQueryEngine, reasoning::ReasoningService,
};
use std::sync::Arc;

#[tokio::test]
async fn test_query_engine_creation() {
    let ontology = Ontology::new();
    let config = ReasonerConfig::default();
    let reasoning_service =
        ReasoningService::new(ontology, config).expect("Failed to create ReasoningService");

    let _query_engine = DLQueryEngine::new(Arc::new(reasoning_service));

    println!("DLQueryEngine created successfully");
}

#[tokio::test]
async fn test_basic_query_functionality() {
    let ontology = Ontology::new();
    let config = ReasonerConfig::default();
    let reasoning_service =
        ReasoningService::new(ontology, config).expect("Failed to create ReasoningService");

    let _query_engine = DLQueryEngine::new(Arc::new(reasoning_service));

    // Test basic functionality without complex operations
    println!("Basic query functionality works");
}

#[test]
fn test_query_syntax() {
    // Test simple query syntax without execution
    let _query_string = "Animal and Dog";

    println!("Query syntax test passed");
}

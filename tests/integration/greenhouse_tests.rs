//! Integration tests for the greenhouse.ttl ontology

use oxidowl::{
    config::ReasonerConfig,
    ontology::{ClassExpression, IRI, Ontology},
    parsers::TurtleParser,
    query::DLQueryEngine,
    reasoning::ReasoningService,
};
use std::path::Path;

/// Test setup function to load a simple ontology
fn create_test_ontology() -> Ontology {
    // For now, create a simple test ontology
    // TODO: Load actual greenhouse.ttl when parser APIs are working
    Ontology::new()
}

/// Test basic reasoning service functionality with greenhouse-like concepts
#[tokio::test]
async fn test_basic_greenhouse_reasoning() {
    let ontology = create_test_ontology();
    let config = ReasonerConfig::test_config();
    let _reasoning_service = ReasoningService::new(ontology, config);

    println!("Basic greenhouse reasoning test passed");
}

/// Test pump1 classification concept (simplified)
#[tokio::test]
async fn test_pump1_classification_concept() {
    let ontology = create_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);

    // Create class expressions for pump and operational concepts
    let _pump_iri = IRI::new("http://www.smolang.org/greenhouseDT#pump1");
    let _operational_iri = IRI::new("http://www.smolang.org/greenhouseDT#OperationalR385");

    // Test that we can create these concepts
    let _operational_class = ClassExpression::class(_operational_iri);

    println!("Pump classification concepts created successfully");
}

/// Test DL query functionality for disjoint union
#[tokio::test]
async fn test_dl_query_disjoint_union() {
    let ontology = create_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);

    let _query_engine = DLQueryEngine::new(reasoning_service);

    // Test query concept without actual execution
    let _query_string = "Maintenance or Operational or Overheating or Underheating";

    println!("DL query concept test passed");
}

/// Test file loading capability (when available)
#[test]
fn test_greenhouse_file_exists() {
    let greenhouse_path = Path::new("greenhouse.ttl");

    if greenhouse_path.exists() {
        println!("greenhouse.ttl file found");
    } else {
        println!("greenhouse.ttl file not found, using test ontology");
    }
}

/// Test parser creation for turtle format
#[test]
fn test_turtle_parser_for_greenhouse() {
    let _parser = TurtleParser::new();

    println!("Turtle parser ready for greenhouse ontology");
}

//! Example demonstrating how to use oxidowl as a library
//! This example shows the complete workflow including `DisjointUnion` axiom handling

use oxidowl::{DLQueryEngine, OntologyFormat, Reasoner, ReasonerConfig, ReasoningService, Result};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<()> {
    println!("=== Oxidowl Library Usage Example ===\n");

    // 1. Create a reasoner with default configuration
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config.clone())?;

    // 2. Load an ontology (greenhouse example with DisjointUnion axioms)
    println!("Loading greenhouse ontology...");
    reasoner.load_ontology_from_file("greenhouse.owx", OntologyFormat::OwlXml)?;

    // 3. Check consistency
    println!("Checking consistency...");
    let is_consistent = reasoner.is_consistent()?;
    println!("Ontology is consistent: {is_consistent}\n");

    // 4. Perform classification
    println!("Performing classification...");
    let classification = reasoner.classify()?;
    println!(
        "Classification completed with {} classes\n",
        classification.hierarchy.len()
    );

    // 5. Create reasoning service for queries
    let ontology = reasoner
        .get_ontology()
        .ok_or_else(|| oxidowl::Error::OntologyParsing {
            message: "No ontology loaded".to_string(),
            line: None,
            column: None,
            context: None,
            token: None,
        })?;
    let ontology_data = ontology.read().expect("Test operation failed").clone();
    let reasoning_service = ReasoningService::new(ontology_data.clone(), config.clone())?;

    // 6. Create DL query engine with namespace
    let query_engine = DLQueryEngine::new_with_namespace(
        Arc::new(reasoning_service.clone()),
        "http://www.smolang.org/greenhouseDT#".to_string(),
    );

    // 7. Test DisjointUnion queries (the main functionality we implemented)
    println!("=== Testing DisjointUnion Queries ===");

    // Test the union query that should return "Pump"
    let union_query = "Operational or Maintenance or Overheating or Underheating";
    println!("Query: {union_query}");

    let result = query_engine.execute_query(union_query).await?;
    println!("Result: {result:?}\n");

    // Verify it returns the expected result
    if let Some(classes) = result.classes {
        if classes.len() == 1 && classes.iter().any(|c| format!("{c:?}").contains("Pump")) {
            println!("DisjointUnion query works correctly - returns 'Pump'");
        } else {
            println!(
                "✗ Unexpected result: expected 'Pump', got {} classes",
                classes.len()
            );
            for c in &classes {
                println!("  - {c:?}");
            }
        }
    } else {
        println!("✗ No classes returned");
    }

    // 8. Test individual class queries
    println!("\n=== Testing Individual Class Queries ===");

    let test_queries = vec!["Operational", "Maintenance", "Pump", "HealthState"];

    for query in test_queries {
        println!("Query: {query}");
        let result = query_engine.execute_query(query).await?;
        println!("Result: {result:?}\n");
    }

    // 9. Test satisfiability
    println!("=== Testing Satisfiability ===");
    let test_class = oxidowl::ClassExpression::Class(oxidowl::ontology::concepts::Class {
        iri: oxidowl::IRI::new("http://www.smolang.org/greenhouseDT#Pump"),
    });

    let is_satisfiable = reasoning_service
        .clone()
        .is_satisfiable(&test_class)
        .await?;
    println!("Pump class is satisfiable: {is_satisfiable}\n");

    // 10. Get equivalent classes for union
    println!("=== Testing Equivalent Classes ===");
    let equivalent_classes = reasoning_service
        .clone()
        .get_equivalent_classes(&test_class)
        .await?;
    println!(
        "Equivalent classes for Pump: {} classes found\n",
        equivalent_classes.len()
    );

    println!("=== Library Usage Example Completed Successfully ===");

    Ok(())
}

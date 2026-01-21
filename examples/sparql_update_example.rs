//! Example demonstrating SPARQL INSERT and DELETE operations
//!
//! This example shows how to use INSERT DATA and DELETE DATA queries
//! to modify an ontology dynamically.

use oxidowl::{
    Result,
    config::ReasonerConfig,
    core::reasoner::Reasoner,
    ontology::{IRI, Ontology},
};

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== SPARQL UPDATE Example ===\n");

    // Create a reasoner with an empty ontology
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config)?;

    // Create a simple ontology
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/ontology#"));
    reasoner.load_ontology(ontology)?;

    println!("1. Starting with empty ontology\n");

    // INSERT DATA - Add class assertions
    println!("2. Inserting triples using INSERT DATA...\n");
    let insert_query = r#"
        INSERT DATA {
            <http://example.org/John> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
            <http://example.org/Mary> <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person> .
            <http://example.org/John> <http://example.org/age> "30" .
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_query)?;
    println!("Insert result: {}\n", result);

    // Query to verify insertion
    println!("3. Verifying inserted data with SELECT query...\n");
    let select_query = r#"
        SELECT ?person WHERE {
            ?person <http://www.w3.org/1999/02/22-rdf-syntax-ns#type> <http://example.org/Person>
        }
    "#;

    let query_result = reasoner.execute_sparql_query(select_query)?;
    println!("Query result: {}\n", query_result);

    // INSERT more data
    println!("4. Inserting object property assertion...\n");
    let insert_property = r#"
        INSERT DATA {
            <http://example.org/John> <http://example.org/knows> <http://example.org/Mary>
        }
    "#;

    let result = reasoner.execute_sparql_query(insert_property)?;
    println!("Insert result: {}\n", result);

    // DELETE DATA - Remove a triple
    println!("5. Deleting a triple using DELETE DATA...\n");
    let delete_query = r#"
        DELETE DATA {
            <http://example.org/John> <http://example.org/age> "30"
        }
    "#;

    let result = reasoner.execute_sparql_query(delete_query)?;
    println!("Delete result: {}\n", result);

    // Query again to verify deletion
    println!("6. Verifying deletion with SELECT query...\n");
    let verify_query = r#"
        SELECT ?s ?p ?o WHERE {
            ?s ?p ?o
        }
    "#;

    let final_result = reasoner.execute_sparql_query(verify_query)?;
    println!("Final query result: {}\n", final_result);

    println!("\n=== SPARQL UPDATE Example Complete ===");

    Ok(())
}

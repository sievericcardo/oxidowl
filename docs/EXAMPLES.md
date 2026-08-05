# Examples

## Basic Reasoning

```rust
use oxidowl::{Ontology, ReasonerConfig, ReasoningService};
use oxidowl::ontology::{OntologyFormat, ClassExpression, Class, IRI};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load ontology from file
    let mut config = ReasonerConfig::default();
    let ontology = Ontology::load_from_file("pizza.owl", OntologyFormat::Auto)?;

    // Create reasoning service (auto-detects EL profile for 100x speedup)
    let service = ReasoningService::new(ontology, config)?;

    // Check consistency
    let is_consistent = service.is_consistent().await?;
    println!("Ontology is consistent: {is_consistent}");

    // Classify ontology
    let classification = service.classify().await?;
    println!("Classification: {} subsumptions", classification.hierarchy.len());

    // Get superclasses of Pizza
    let pizza = ClassExpression::Class(Class { iri: IRI::new("http://example.org/Pizza") });
    let superclasses = service.get_superclasses(&pizza, false).await?;
    println!("Pizza superclasses: {superclasses:?}");

    Ok(())
}
```

## DL Queries

```rust
use std::sync::Arc;
use oxidowl::{DLQueryEngine, ReasoningService};

// Create query engine with namespace support
let query_engine = DLQueryEngine::new_with_namespace(
    Arc::new(service),
    "http://example.org/pizza#".to_string()
);

// Find all vegetarian pizzas
let vegetarian_pizzas = query_engine.execute_query(
    "Pizza and (hasTopping only VegetarianTopping)"
).await?;

println!("Vegetarian pizzas: {vegetarian_pizzas:?}");
```

## SWRL Rule Execution

```rust
use oxidowl::{ReasoningService, ReasoningRequest};

// Execute SWRL rules
let swrl_result = service.execute_swrl_rules().await?;
println!("SWRL execution: {} rules fired, {} new inferences",
         swrl_result.applications, swrl_result.inferences.len());
```

## Server Mode

```rust
#[cfg(feature = "server")]
use oxidowl::server::{OWLlinkServer, SPARQLServer};

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Start OWLlink server
    let server = OWLlinkServer::new(8080);
    server.add_ontology("pizza", "pizza.owl")?;
    server.start().await?;

    Ok(())
}
```

## EL Profile Classification

```rust
use oxidowl::profiles::el_reasoner::ELReasoner;
use oxidowl::config::ReasonerConfig;

// Use EL reasoner directly for polynomial-time classification
let mut el_reasoner = ELReasoner::new(ReasonerConfig::default());
el_reasoner.initialize(&ontology)?;
let classification = el_reasoner.classify()?;

println!("EL classification: {} subsumptions", classification.hierarchy.len());
```

## RDF-star Examples

See [RDF_STAR_GUIDE.md](RDF_STAR_GUIDE.md) for comprehensive RDF-star and RDF 1.2 usage examples.

## More Examples

- [examples/dl_query_example.rs](../examples/dl_query_example.rs) - DL query engine usage
- [examples/sparql_update_example.rs](../examples/sparql_update_example.rs) - SPARQL INSERT/DELETE
- [examples/server_example.rs](../examples/server_example.rs) - REST / OWLlink server
- [examples/library_usage.rs](../examples/library_usage.rs) - Library API walkthrough
- [tests/rdf_star_integration_tests.rs](../tests/rdf_star_integration_tests.rs) - RDF-star integration tests

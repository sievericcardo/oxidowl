# Examples

## Basic Reasoning

```rust
use oxidowl::{OxidOwl, ReasoningService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load ontology with horned-owl integration
    let reasoner = OxidOwl::from_file("pizza.owl")?;

    // Check consistency
    let is_consistent = reasoner.is_consistent().await?;
    println!("Ontology is consistent: {}", is_consistent);

    // Classify ontology to compute inferences
    reasoner.classify().await?;

    // Create reasoning service for advanced operations
    let reasoning_service = ReasoningService::new(reasoner);

    // Get all subclasses of Pizza
    let subclasses = reasoning_service.get_subclasses("Pizza").await?;
    println!("Pizza subclasses: {:?}", subclasses);

    Ok(())
}
```

## DL Queries

```rust
use oxidowl::{DLQueryEngine, ReasoningService};

// Create query engine with namespace support
let query_engine = DLQueryEngine::new_with_namespace(
    reasoning_service,
    "http://example.org/pizza#".to_string()
);

// Find all vegetarian pizzas
let vegetarian_pizzas = query_engine.execute_query(
    "Pizza and (hasTopping only VegetarianTopping)"
).await?;

// Test DisjointUnion queries - returns equivalent class
let pump_query = query_engine.execute_query(
    "Operational or Maintenance or Overheating or Underheating"
).await?;
// Returns: {"classes": ["Pump"]} if DisjointUnion axiom exists

println!("Vegetarian pizzas: {:?}", vegetarian_pizzas);
println!("Union query result: {:?}", pump_query);
```

## SWRL Rule Execution

```rust
use oxidowl::{ReasoningService, SWRLRuleEngine, SWRLConfig, SWRLReasoningStrategy};

// Create SWRL configuration
let swrl_config = SWRLConfig {
    strategy: SWRLReasoningStrategy::ForwardChaining,
    max_rule_applications: 1000,
    enable_builtins: true,
    debug: true,
    ..Default::default()
};

// Create reasoning service with SWRL support
let reasoning_service = ReasoningService::new(ontology, reasoner_config);

// Execute SWRL rules
let swrl_result = reasoning_service.execute_swrl_rules().await?;
println!("SWRL execution: {} rules fired, {} new inferences",
         swrl_result.applications, swrl_result.inferences.len());

// Get SWRL statistics
let stats = reasoning_service.get_swrl_statistics().await?;
println!("Total rule applications: {}", stats.total_rule_applications);
println!("Rules fired: {}", stats.rules_fired);
println!("Inferences generated: {}", stats.inferences_generated);

// Control individual rules
reasoning_service.set_swrl_rule_active(rule_id, false).await?; // Disable rule
reasoning_service.set_swrl_rule_priority(rule_id, 10).await?;  // Set priority
```

## SWRL Built-in Predicates

Oxidowl supports 30+ SWRL built-in predicates across multiple categories:

```rust
// Mathematical built-ins
// swrlb:add(?x, ?y, ?z) - z = x + y
// swrlb:subtract(?x, ?y, ?z) - z = x - y
// swrlb:multiply(?x, ?y, ?z) - z = x * y
// swrlb:divide(?x, ?y, ?z) - z = x / y
// swrlb:mod(?x, ?y, ?z) - z = x % y
// swrlb:pow(?x, ?y, ?z) - z = x^y

// String built-ins
// swrlb:stringLength(?s, ?len) - length of string s
// swrlb:stringConcat(?s1, ?s2, ?result) - concatenate strings
// swrlb:contains(?s, ?sub) - true if s contains sub
// swrlb:startsWith(?s, ?prefix) - true if s starts with prefix
// swrlb:endsWith(?s, ?suffix) - true if s ends with suffix

// Date/time built-ins (15+ predicates)
// swrlb:dateTimeEqual(?dt1, ?dt2) - compare date/times
// swrlb:dateTimeLessThan(?dt1, ?dt2) - dt1 < dt2
// swrlb:yearFromDateTime(?dt, ?year) - extract year
// swrlb:monthFromDateTime(?dt, ?month) - extract month
// swrlb:dayFromDateTime(?dt, ?day) - extract day

// Regular expression built-ins
// swrlb:matches(?text, ?pattern) - pattern matching
// swrlb:replace(?text, ?pattern, ?replacement, ?result) - text replacement
// swrlb:tokenize(?text, ?pattern, ?tokens) - tokenization
// swrlb:extract(?text, ?pattern, ?result) - extract first match

// Boolean built-ins
// swrlb:booleanNot(?x, ?result) - logical NOT
```

## Custom SWRL Built-ins

You can also register custom built-in predicates:

```rust
use oxidowl::swrl::{SWRLBuiltIn, SWRLValue};

struct CustomBuiltIn;

impl SWRLBuiltIn for CustomBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        // Custom logic here
        Ok(SWRLValue::Boolean(true))
    }

    fn name(&self) -> &str {
        "http://example.org/customBuiltIn"
    }

    fn arity(&self) -> Option<usize> {
        Some(2) // Fixed arity of 2 arguments
    }
}

// Register the custom built-in
let mut swrl_engine = SWRLRuleEngine::new(swrl_config);
swrl_engine.add_builtin(Box::new(CustomBuiltIn));
```

## Server Mode

```rust
use oxidowl::server::{OWLlinkServer, SPARQLServer};

// Start OWLlink server
let server = OWLlinkServer::new(8080);
server.add_ontology("pizza", "pizza.owl")?;
server.start().await?;

// Server now accepts OWLlink requests at http://localhost:8080
```

## RDF-star Examples

See [RDF_STAR_GUIDE.md](RDF_STAR_GUIDE.md) for comprehensive RDF-star and RDF 1.2 usage examples.

## More Examples

- [examples/dl_query_example.rs](../examples/dl_query_example.rs) - DL query engine usage
- [examples/sparql_update_example.rs](../examples/sparql_update_example.rs) - SPARQL INSERT/DELETE
- [examples/server_example.rs](../examples/server_example.rs) - REST / OWLlink server
- [examples/library_usage.rs](../examples/library_usage.rs) - Library API walkthrough
- [tests/rdf_star_integration_tests.rs](../tests/rdf_star_integration_tests.rs) - RDF-star integration tests

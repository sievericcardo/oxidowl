# <img src="oxidowl.webp" alt="Oxidowl Logo generated with Google Gemini" width="35"/> Oxidowl

A high-performance Description Logic reasoner for OWL 2 DL ontologies, implemented in Rust with advanced tableau algorithms, parallel computation, and integrated horned-owl support.

[![License: LGPL-3.0](https://img.shields.io/badge/License-LGPL%20v3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.88+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#installation)

## Overview

Oxidowl is a tableau-based reasoner for the Description Logic SROIQV(D), supporting nearly all features of OWL 2 DL. Built on the robust [horned-owl](https://github.com/phillord/horned-owl) foundation, it implements efficient tableau algorithms while leveraging Rust's memory safety and performance characteristics.

### Key Features

#### Core Reasoning Engine

- 🚀 **High Performance**: Advanced tableau algorithms with parallel computation
- 🔧 **Complete OWL 2 DL Support**: Handles SROIQV(D) description logic with DisjointUnion axioms
- 🧠 **Multiple Reasoning Tasks**: Consistency, satisfiability, classification, and instance checking
- 📊 **DL Query Engine**: Manchester Syntax support with union queries and DisjointUnion detection
- 🔄 **Multiple Input Formats**: OWL XML, Functional Syntax, RDF/XML, Turtle, N-Triples via horned-owl
- ⚡ **Dual Tableau Algorithms**: Traditional and Hypertableau (3-9x faster for disjointness reasoning)
- 🎨 **Structural Sharing**: Hypergraph-based reasoning with reduced memory usage
- 🦀 **Horned-OWL Integration**: Built on proven OWL parsing and modeling foundation

#### Enhancements

- 🎯 **EL Profile Optimization**: Polynomial-time reasoning for OWL 2 EL ontologies
- 💡 **Comprehensive Explanations**: Justification generation, proof tracking, and laconic explanations
- 🌐 **Multi-Protocol Server Support**: OWLlink, SPARQL endpoint, and REST API interfaces
- 📥 **Advanced Import Resolution**: Recursive imports, cycle detection, and IRI mapping
- 🧪 **SWRL Rule Support**: Full Semantic Web Rule Language implementation with 30+ built-in predicates

#### v0.8.0 Highlights (Latest)

- 🔥 **Zero-Overhead Parser State**: Compile-time optimizations with no runtime cost for successful parsing
- ⚡ **O(1) Keyword Validation**: Perfect hash tables for comprehensive OWL keyword checking
- 📝 **Configurable Error Verbosity**: Three levels (Minimal/Standard/Detailed) for performance tuning
- 🔧 **SWRL Validation**: Basic parsing support for DLSafeRule constructs
- 🚀 **Performance Improvements**: <2% overhead for minimal verbosity, inline hot paths
- 🎨 **Enhanced Error Messages**: Optional line/column/context information for debugging

#### Competitive Advantages

- **Performance**: EL reasoning with polynomial complexity vs exponential in full DL
- **Ecosystem Integration**: SPARQL endpoint for Semantic Web compatibility
- **Developer Experience**: RESTful APIs, comprehensive explanations, and detailed error reporting
- **Standards Compliance**: OWLlink protocol support for interoperability with existing tools

## Installation

For usage with Docker, see [DOCKER.md](DOCKER.md).

### Prerequisites

- Rust 1.88 or higher
- Cargo (comes with Rust)

### From Source

```bash
git clone https://github.com/sievericcardo/oxidowl.git
cd oxidowl
cargo build --release
```

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test unit::reasoning
cargo test integration::greenhouse

# Run with performance benchmarks
cargo test --release -- --ignored
```

## Quick Start

### Command Line Usage

```bash
# Check ontology consistency
oxidowl consistency -i ontology.owl

# Classify an ontology
oxidowl classify -i ontology.owl -o hierarchy.json

# Run DL queries with DisjointUnion support
oxidowl query -i ontology.owl -q "Person and (hasChild some Thing)"
oxidowl query -i greenhouse.owx -q "Operational or Maintenance or Overheating or Underheating" --namespace "http://www.smolang.org/greenhouseDT#"

# Execute SWRL rules
oxidowl swrl -i ontology-with-rules.owl --strategy forward

# Run performance benchmarks
oxidowl benchmark -i ontology.owl --algorithm all
```

### Library Usage

```rust
use oxidowl::{
    Reasoner, ReasonerConfig, DLQueryEngine, ReasoningService,
    OntologyFormat, Result, SWRLRuleEngine, SWRLConfig, TableauAlgorithm
};

#[tokio::main]
async fn main() -> Result<()> {
    // Create reasoner with Hypertableau algorithm for better performance
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let mut reasoner = Reasoner::new(config.clone())?;

    // Load an ontology
    reasoner.load_ontology_from_file("example.owl", OntologyFormat::OwlXml)?;

    // Check consistency (uses Hypertableau for 3-9x speedup on disjointness)
    let is_consistent = reasoner.is_consistent()?;
    println!("Ontology is consistent: {}", is_consistent);

    // Perform classification
    let classification = reasoner.classify()?;
    println!("Classification completed with {} classes", classification.hierarchy.len());

    // Create reasoning service and DL query engine
    let ontology = reasoner.get_ontology()?;
    let ontology_data = ontology.read().unwrap().clone();
    let reasoning_service = ReasoningService::new(ontology_data, config);
    let query_engine = DLQueryEngine::new_with_namespace(
        reasoning_service,
        "http://example.org#".to_string()
    );

    // Execute DL queries with DisjointUnion support
    let result = query_engine.execute_query("Person and (hasChild some Thing)").await?;
    println!("Query returned: {:?}", result);

    // Test union queries that resolve to DisjointUnion equivalents
    let union_result = query_engine.execute_query("ClassA or ClassB or ClassC").await?;
    println!("Union query result: {:?}", union_result);

    // Execute SWRL rules
    let swrl_result = reasoning_service.execute_swrl_rules().await?;
    println!("SWRL execution result: {} rules fired, {} inferences", 
             swrl_result.applications, swrl_result.inferences.len());

    Ok(())
}
```

### Parser Error Verbosity Configuration (v0.8.0)

Control the level of detail in parsing error messages for performance tuning:

```rust
use oxidowl::parsers::{FunctionalParser, ParserConfig, ErrorVerbosity};

// Minimal verbosity - best performance (<2% overhead)
let parser = FunctionalParser::with_config(ParserConfig::minimal());
let result = parser.parse_string(ontology_content);

// Standard verbosity - balanced (default, <5% overhead)
let parser = FunctionalParser::new(); // Uses Standard by default
let result = parser.parse_string(ontology_content);

// Detailed verbosity - full debugging information
let parser = FunctionalParser::with_config(ParserConfig::detailed());
let result = parser.parse_string(ontology_content);

// Custom configuration
let config = ParserConfig {
    error_verbosity: ErrorVerbosity::Detailed,
};
let parser = FunctionalParser::with_config(config);
```

**Performance Impact:**
- **Minimal**: Just error messages, <2% slowdown
- **Standard**: Adds line/column info, <5% slowdown  
- **Detailed**: Full context and tokens, overhead only on error paths

### Algorithm Selection: Traditional vs Hypertableau

Oxidowl provides two tableau-based reasoning algorithms optimized for different ontology characteristics:

#### Traditional Tableau (Default)
- Classic tableau expansion with blocking
- Best for general-purpose reasoning
- Consistent performance across all scenarios
- **Use when:** Ontology has many equivalent classes, large linear taxonomies, or unknown characteristics

#### Hypertableau Algorithm  
- Hypergraph-based structural sharing
- **faster** for disjointness-heavy ontologies
- **faster** for complex class expressions
- Reduced memory usage through node reuse
- **Use when:** Ontology has many disjoint class axioms or complex intersections/unions

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

let mut config = ReasonerConfig::default();

// For disjointness-heavy ontologies (3-9x speedup)
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;

// For general-purpose reasoning (safe default)
config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
```

**Performance Guidance:**

| Ontology Characteristic | Recommended Algorithm | Expected Speedup |
|------------------------|----------------------|------------------|
| Many disjoint classes (>10) | Hypertableau | 3-9x faster |
| Complex expressions (<50) | Hypertableau | 8-13% faster |
| Large linear taxonomy (>50 classes) | Traditional | 2-5x faster |
| Many equivalent classes (>10) | Traditional | 2-7x faster |
| General purpose / unknown | Traditional | Safer default |

## Features (Latest)

Oxidowl has been significantly enhanced with improvements that bring it up to competitive standards with major OWL reasoners while adding unique capabilities.

### EL Profile Optimization

For ontologies that conform to the OWL 2 EL profile, Oxidowl provides specialized polynomial-time reasoning:

```rust
use oxidowl::{
    profiles::el_reasoner::{ELReasoner, CompletionConfig},
    config::OWLProfile,
    ontology::Ontology,
};

// Configure for EL profile optimization
let config = CompletionConfig {
    max_iterations: 1000,
    enable_caching: true,
    batch_size: 50,
    convergence_threshold: 0.01,
};

let mut el_reasoner = ELReasoner::new(ontology, config);
let classification = el_reasoner.classify().await?;

println!("EL classification completed in polynomial time!");
```

### Comprehensive Explanation Generation

Generate detailed explanations for reasoning results with multiple output formats:

```rust
use oxidowl::explanation::{
    ExplanationService, ExplanationType, ExplanationFormat
};

let explanation_service = ExplanationService::new();

// Explain why a subsumption holds
let explanation = explanation_service
    .explain_inference(&ontology, &inference_axiom, ExplanationType::Subsumption)
    .await?;

// Generate human-readable explanation
let natural_language = explanation_service
    .format_explanation(&explanation, ExplanationFormat::NaturalLanguage)
    .await?;

// Generate proof tree
let proof_tree = explanation_service
    .format_explanation(&explanation, ExplanationFormat::ProofTree)
    .await?;

println!("Explanation: {}", natural_language);
println!("Proof Tree:\n{}", proof_tree);
```

### Multi-Protocol Server Support

Run Oxidowl as a server with multiple protocol interfaces. **Note:** By default, the reasoner runs without servers. Use the `--enable-server` flags to start web services.

#### Starting the Server

```bash
# Start REST API server on default port (8080)
oxidowl --enable-server ontology.owl

# Start on a custom port
oxidowl --enable-server --server-port 9090 ontology.owl

# Start OWLlink server
oxidowl --enable-owllink --owllink-port 8081 ontology.owl

# Start SPARQL endpoint
oxidowl --enable-sparql --sparql-port 8082 ontology.owl

# Start all server types with custom ports
oxidowl --enable-server --server-port 8080 \
        --enable-owllink --owllink-port 8081 \
        --enable-sparql --sparql-port 8082 \
        ontology.owl

# Bind to all interfaces (for remote access)
oxidowl --enable-server --server-bind 0.0.0.0 --server-port 8080 ontology.owl
```

#### SPARQL Endpoint

```bash
# Query via SPARQL (using oxigraph integration)
curl -X POST http://localhost:8081/sparql \
  -H "Content-Type: application/sparql-query" \
  -d "SELECT ?class WHERE { ?class a owl:Class }"
```

#### OWLlink Protocol

```xml
<!-- OWLlink request -->
<RequestMessage xmlns="http://www.owllink.org/owllink#">
    <CreateKB kb="medical" />
    <LoadOntology kb="medical">
        <IRI>http://example.org/medical.owl</IRI>
    </LoadOntology>
    <IsConsistent kb="medical" />
</RequestMessage>
```

#### REST API

```bash
# Check consistency via REST API
curl -X GET http://localhost:8082/api/v1/consistency

# Perform classification
curl -X POST http://localhost:8082/api/v1/classify

# Get explanations
curl -X POST http://localhost:8082/api/v1/explain \
  -H "Content-Type: application/json" \
  -d '{"inference_type": "subsumption", "axiom": "Human ⊑ Animal"}'
```

#### Library Usage with Server

```rust
use oxidowl::{Reasoner, ReasonerConfig, OntologyFormat};

#[cfg(feature = "server")]
#[tokio::main]
async fn main() -> oxidowl::Result<()> {
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology_from_file("ontology.owl", OntologyFormat::Auto)?;
    
    // Start server on port 8080
    let mut server_manager = reasoner.start_server_on_port(8080).await?;
    
    println!("Server running on http://127.0.0.1:8080");
    
    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await.unwrap();
    
    // Stop servers gracefully
    server_manager.stop_all().await?;
    Ok(())
}
```

See [examples/server_example.rs](examples/server_example.rs) for a complete example.

### Advanced Import Resolution

Handle complex ontology imports with sophisticated resolution:

```rust
use oxidowl::import::resolver::{ImportResolver, ImportResolverConfig};

let config = ImportResolverConfig {
    base_directories: vec![
        PathBuf::from("./ontologies"),
        PathBuf::from("./imports"),
    ],
    allow_remote: true,
    max_depth: 10,
};

let resolver = ImportResolver::new().with_config(config);

// Add IRI mappings for local resolution
resolver.add_iri_mapping(
    "http://purl.obolibrary.org/obo/go.owl".to_string(),
    "./local-cache/go.owl".to_string(),
).await?;

// Load ontology with all imports resolved
let imported_ontology = resolver
    .load_with_imports("http://example.org/my-ontology.owl", None)
    .await?;
```

### Configuration

Oxidowl supports extensive configuration options:

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};
use oxidowl::swrl::{SWRLConfig, SWRLReasoningStrategy};

let config = ReasonerConfig {
    algorithm: TableauAlgorithm::Traditional,
    parallel_processing: true,
    max_threads: Some(8),
    timeout: Some(std::time::Duration::from_secs(300)),
    blocking_strategy: BlockingStrategy::Anywhere,
    caching_enabled: true,
    max_cache_size: 10_000,
    monitoring_level: MonitoringLevel::Basic,
    // ... additional options
};

// Configure SWRL rule execution
let swrl_config = SWRLConfig {
    strategy: SWRLReasoningStrategy::ForwardChaining,
    max_rule_applications: 1000,
    max_execution_depth: 100,
    enable_builtins: true,
    debug: false,
    timeout_ms: Some(30000),
};
```

## Architecture

### Core Components

- **`core`** - Core reasoning engine with tableau algorithms
  - `reasoner.rs` - Main reasoner interface
  - `tableau.rs` - Tableau implementation with node and edge management

- **`ontology`** - Ontology representation and management (built on horned-owl)
  - `axioms.rs` - Axiom structures and operations including DisjointUnion
  - `concepts.rs` - Class expressions and concepts
  - `properties.rs` - Object and data properties
  - `individuals.rs` - ABox individuals and assertions

- **`parsers`** - Input format support via horned-owl integration
  - `owl_xml.rs` - OWL XML parser with DisjointUnion support
  - `functional.rs` - Functional syntax parser
  - `rdf_xml.rs` - RDF/XML parser
  - `turtle.rs` - Turtle format parser

- **`reasoning`** - High-level reasoning coordination
- **`query`** - DL query engine with Manchester Syntax and union query support
- **`swrl`** - SWRL (Semantic Web Rule Language) implementation
  - `engine.rs` - SWRL rule execution engine with multiple strategies
  - `interpreter.rs` - Individual rule interpretation and execution
  - `parser.rs` - SWRL syntax parsing
  - `builtins.rs` - Core built-in predicates (math, string, boolean)
  - `datetime_builtins.rs` - Date/time built-in predicates
  - `regex_builtins.rs` - Regular expression built-in predicates
  - `validation.rs` - SWRL rule validation
- **`adapter`** - Horned-OWL integration layer for enhanced parsing and modeling
- **`config`** - Configuration management and optimization

### Algorithms

#### Tableau Algorithm

Oxidowl implements an efficient tableau algorithm that provides:

- **Systematic Expansion**: Sound and complete reasoning through node expansion
- **Optimized Rule Application**: Smart ordering and caching of tableau rules
- **Advanced Blocking**: Anywhere blocking with cycle detection for termination
- **Dependency Tracking**: Intelligent backtracking and conflict resolution
- **Memory Management**: Efficient data structures for large ontologies

#### Performance Optimizations

- **Parallel Processing**: Multi-threaded reasoning for large ontologies
- **Caching**: LRU caches for frequent operations
- **Memory Management**: Optimized data structures and memory pools
- **Incremental Reasoning**: Support for ontology updates

## Examples

### Basic Reasoning

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

### DL Queries

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

### SWRL Rule Execution

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

### SWRL Built-in Predicates

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

### Custom SWRL Built-ins

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

### Server Mode

```rust
use oxidowl::server::{OWLlinkServer, SPARQLServer};

// Start OWLlink server
let server = OWLlinkServer::new(8080);
server.add_ontology("pizza", "pizza.owl")?;
server.start().await?;

// Server now accepts OWLlink requests at http://localhost:8080
```

## Testing

### Test Categories

- **Unit Tests**: Individual component testing
  - Reasoning algorithms
  - Ontology operations
  - Parser functionality
  - Configuration management

- **Integration Tests**: End-to-end testing
  - Real ontology processing
  - Algorithm comparison

### Test Execution

```bash
# Quick test suite
cargo test quick

# Full test suite
cargo test

# Performance tests (slower)
cargo test --release performance

# Specific test categories
cargo test unit::reasoning
cargo test integration::greenhouse
```

## Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup

1. Fork the repository
2. Create a feature branch: `git checkout -b feature-name`
3. Make your changes with tests
4. Run the test suite: `cargo test`
5. Submit a pull request

### Code Standards

- Follow Rust naming conventions
- Include comprehensive tests
- Document public APIs
- Run `cargo fmt` and `cargo clippy`

## Roadmap

### Nice features to add in the future

- [ ] OWL 2 RL profile support
- [ ] Incremental classification
- [ ] Distributed reasoning
- [ ] WebAssembly compilation
- [ ] Python bindings
- [ ] Docker containerization

### Recently Implemented

- [x] **SWRL rule support** - Complete implementation with 30+ built-in predicates
- [x] **DisjointUnion axiom support** - Full support in DL queries and reasoning
- [x] **Advanced DL Query Engine** - Manchester Syntax with union query optimization

### Performance Improvements

- [ ] GPU acceleration for large-scale reasoning
- [ ] Advanced caching strategies
- [ ] Streaming ontology processing
- [ ] Memory-mapped storage backends

## License

This project is licensed under the BSD 3-Clause License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the [HermiT](https://github.com/owlcs/hermit-reasoner/) reasoner and [Konclude](https://github.com/konclude/Konclude) for their advanced reasoning techniques
- Built with the Rust ecosystem's excellent crates
- Thanks to the OWL and Semantic Web communities

## Support

Please report any issues or feature requests and feel free to contribute!

- 📖 [Documentation](https://docs.rs/oxidowl)
- 🐛 [Issue Tracker](https://github.com/sievericcardo/oxidowl/issues)
- 💬 [Discussions](https://github.com/sievericcardo/oxidowl/discussions)

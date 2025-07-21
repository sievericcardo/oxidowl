# Oxidowl

A high-performance Description Logic reasoner for OWL 2 DL ontologies, implemented in Rust with advanced hypertableau algorithms, parallel computation, and integrated horned-owl support.

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
[![Rust](https://img.shields.io/badge/rust-1.88+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#installation)

## Overview

Oxidowl is a tableau-based reasoner for the Description Logic SROIQV(D), supporting nearly all features of OWL 2 DL. Built on the robust [horned-owl](https://github.com/phillord/horned-owl) foundation, it implements HermiT's hypertableau algorithm with hyperresolution and ground disjunctions for efficient reasoning, while leveraging Rust's memory safety and performance characteristics.

### Key Features

- 🚀 **High Performance**: Advanced hypertableau algorithms with parallel computation
- 🔧 **Complete OWL 2 DL Support**: Handles SROIQV(D) description logic with DisjointUnion axioms
- 🧠 **Multiple Reasoning Tasks**: Consistency, satisfiability, classification, and instance checking
- 📊 **DL Query Engine**: Manchester Syntax support with union queries and DisjointUnion detection
- 🔄 **Multiple Input Formats**: OWL XML, Functional Syntax, RDF/XML, Turtle, N-Triples via horned-owl
- ⚡ **Optimized Algorithms**: Hyperresolution, ground disjunctions, and advanced blocking strategies
- 🔍 **Performance Analysis**: Comprehensive benchmarking and algorithm comparison tools
- 🦀 **Horned-OWL Integration**: Built on proven OWL parsing and modeling foundation

## Installation

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
cargo test performance

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

# Run performance benchmarks
oxidowl benchmark -i ontology.owl --algorithm all
```

### Library Usage

```rust
use oxidowl::{
    Reasoner, ReasonerConfig, DLQueryEngine, ReasoningService,
    OntologyFormat, Result
};

#[tokio::main]
async fn main() -> Result<()> {
    // Create reasoner with default configuration
    let config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config.clone())?;

    // Load an ontology
    reasoner.load_ontology_from_file("example.owl", OntologyFormat::OwlXml)?;

    // Check consistency
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

    Ok(())
}
```

### Configuration

Oxidowl supports extensive configuration options:

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

let config = ReasonerConfig {
    algorithm: TableauAlgorithm::HyperTableau,
    parallel_processing: true,
    max_threads: Some(8),
    timeout: Some(std::time::Duration::from_secs(300)),
    blocking_strategy: BlockingStrategy::Anywhere,
    caching_enabled: true,
    max_cache_size: 10_000,
    monitoring_level: MonitoringLevel::Basic,
    // ... additional options
};
```

## Architecture

### Core Components

- **`core`** - Core reasoning engine with tableau algorithms
  - `reasoner.rs` - Main reasoner interface
  - `tableau.rs` - Traditional tableau implementation
  - `hypertableau/` - Advanced hypertableau algorithms
    - `hyperresolution.rs` - Hyperresolution inference
    - `ground_disjunction.rs` - Ground disjunction handling
    - `extension_table.rs` - Extension table management

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
- **`adapter`** - Horned-OWL integration layer for enhanced parsing and modeling
- **`config`** - Configuration management and optimization

### Algorithms

#### HyperTableau Algorithm

Oxidowl implements HermiT's hypertableau algorithm, which combines:

- **Hyperresolution**: Efficient inference using resolution-based techniques
- **Ground Disjunctions**: Optimized handling of disjunctive information
- **Extension Tables**: Compact representation of concept and role assertions
- **Dependency Tracking**: Intelligent backtracking and clause learning
- **Advanced Blocking**: Anywhere blocking with cycle detection

#### Performance Optimizations

- **Parallel Processing**: Multi-threaded reasoning for large ontologies
- **Caching**: LRU caches for frequent operations
- **Memory Management**: Optimized data structures and memory pools
- **Incremental Reasoning**: Support for ontology updates

## Performance

### Benchmarking

Oxidowl includes comprehensive performance testing:

```bash
# Run algorithm comparison benchmarks
cargo run --bin run_performance_tests algorithm

# Run scalability tests
cargo run --bin run_performance_tests scalability

# Run memory benchmarks
cargo run --bin run_performance_tests memory

# Generate performance reports
cargo run --bin run_performance_tests report -i results.json -f html
```

### Performance Characteristics

| Operation | Small Ontology | Medium Ontology | Large Ontology |
|-----------|---------------|-----------------|----------------|
| Consistency | < 1ms | 10-100ms | 1-10s |
| Classification | < 10ms | 100ms-1s | 10s-5min |
| Instance Check | < 1ms | 1-10ms | 10-100ms |
| DL Query | 1-10ms | 10-100ms | 100ms-1s |

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
  - Performance benchmarking
  - Algorithm comparison

- **Performance Tests**: Scalability and efficiency
  - Memory usage analysis
  - Concurrent processing
  - Large ontology handling

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
cargo test performance::scalability
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
- [ ] SWRL rule support
- [ ] Incremental classification
- [ ] Distributed reasoning
- [ ] WebAssembly compilation
- [ ] Python bindings
- [ ] Docker containerization

### Performance Improvements

- [ ] GPU acceleration for large-scale reasoning
- [ ] Advanced caching strategies
- [ ] Streaming ontology processing
- [ ] Memory-mapped storage backends

## License

This project is licensed under the BSD 3-Clause License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the [HermiT](https://github.com/owlcs/hermit-reasoner/) reasoner and its hypertableau algorithms and [Konclude](https://github.com/konclude/Konclude) for its advanced reasoning techniques
- Built with the Rust ecosystem's excellent crates
- Thanks to the OWL and Semantic Web communities

## Support

Please report any issues or feature requests and feel free to contribute!

- 📖 [Documentation](https://docs.rs/oxidowl)
- 🐛 [Issue Tracker](https://github.com/sievericcardo/oxidowl/issues)
- 💬 [Discussions](https://github.com/sievericcardo/oxidowl/discussions)

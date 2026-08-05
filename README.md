# <img src="oxidowl.webp" alt="Oxidowl Logo generated with Google Gemini" width="35"/> Oxidowl

A high-performance Description Logic reasoner for OWL 2 DL ontologies, implemented in Rust with advanced tableau algorithms, parallel computation, and integrated horned-owl support.

[![License: LGPL-3.0](https://img.shields.io/badge/License-LGPL%20v3-blue.svg)](https://www.gnu.org/licenses/lgpl-3.0)
[![Rust](https://img.shields.io/badge/rust-1.88+-orange.svg)](https://www.rust-lang.org)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)](#installation)

## Overview

Oxidowl is a tableau-based reasoner for the Description Logic SROIQV(D), supporting nearly all features of OWL 2 DL. Built on the robust [horned-owl](https://github.com/phillord/horned-owl) foundation, it implements efficient tableau algorithms while leveraging Rust's memory safety and performance characteristics. Inspired by [Konclude](https://github.com/konclude/Konclude) and [HermiT](https://github.com/owlcs/hermit-reasoner).

## Table of Contents

- [Key Features](#key-features)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Algorithm Selection](#algorithm-selection-traditional-vs-hypertableau)
- [Features In-Depth](#features-in-depth)
- [Architecture](#architecture)
- [RDF 1.2 and RDF-star Support](#rdf-12-and-rdf-star-support)
- [Examples](#examples)
- [Testing](#testing)
- [Contributing](#contributing)
- [Roadmap](#roadmap)
- [License](#license)
- [Acknowledgments](#acknowledgments)
- [Support](#support)

## Key Features

### Core Reasoning Engine

- **High Performance**: Advanced tableau algorithms with parallel computation
- **Complete OWL 2 DL Support**: Handles SROIQV(D) description logic with DisjointUnion axioms
- **Multiple Reasoning Tasks**: Consistency, satisfiability, TBox classification, ABox realisation, and full OWL DL instance checking
- **DL Query Engine**: Manchester Syntax support with union queries and DisjointUnion detection
- **Multiple Input Formats**: OWL XML, Functional Syntax, RDF/XML, Turtle, N-Triples via horned-owl
- **Dual Tableau Algorithms**: Traditional and Hypertableau (3-9x faster for disjointness reasoning)
- **Structural Sharing**: Hypergraph-based reasoning with reduced memory usage
- **Horned-OWL Integration**: Built on proven OWL parsing and modeling foundation

### Enhancements

- **OWL 2 Profile Support**: EL (polynomial), QL (query), RL (rule) optimised reasoners plus full DL validator
- **Comprehensive Explanations**: Justification generation, proof tracking, and laconic explanations
- **Multi-Protocol Server Support**: OWLlink, SPARQL endpoint, and REST API interfaces
- **Advanced Import Resolution**: Recursive imports, cycle detection, and IRI mapping
- **SWRL Rule Support**: Full Semantic Web Rule Language implementation with 30+ built-in predicates
- **SHACL Validation**: Complete SHACL Core + SHACL-SPARQL backed by Oxigraph
- **Distributed Reasoning**: Cluster-based horizontal scaling with fault tolerance and load balancing
- **ML-Enhanced Queries**: Neural-network-assisted conjunctive query optimization (Candle framework)
- **Incremental Classification**: Only re-reasons over concepts affected by ontology changes
- **Formal Verification**: Kani harnesses for memory-safety proofs
- **Advanced Caching**: LRU, LFU, LRUFU, TTL, and size-based eviction strategies
- **Advanced Preprocessing Pipeline**: Triggered-implication absorption, common-disjunct extraction, disjunct sorting, role automata construction, and nominal-schema processing
- **Datatype Value Space Handlers**: XSD-compliant value-space reasoning for boolean, string, numeric, datetime, and IRI datatypes
- **Parallel Tableau Expansion**: Rayon-powered parallel tableau node expansion for large ontologies
- **Saturation Cycle Detection**: DashMap-backed cycle detector for the saturation engine with atomic counters

### v1.0.0 Highlights

- **OWL 2 RL Profile Reasoner**: Forward-chaining materialization with incremental update support
- **EL Reasoner Performance**: O(1) indexed subsumption rules, queue deduplication, and skipped redundant Floyd-Warshall — 200-class classification in <1s release mode
- **Distributed Reasoning**: Cluster-based horizontal scaling with automatic node discovery and fault tolerance
- **ML-Enhanced Query Engine**: Candle-backed neural query optimizer for conjunctive query execution
- **SHACL Core + SHACL-SPARQL**: Full W3C SHACL validation backed by the embedded Oxigraph SPARQL store
- **SPARQL UPDATE**: INSERT DATA and DELETE DATA support for dynamic ontology mutation
- **Incremental Classification**: Dependency-tracked reclassification of only affected concepts on ontology updates
- **Lock-Free Data Structures**: DashMap-backed caches for highly concurrent access patterns
- **Kani Formal Verification**: Harnesses for memory-safety and correctness proofs via `cargo kani`
- **Advanced Cache Strategies**: LRU, LFU, LRUFU, TTL and size-based eviction policies
- **Advanced Preprocessing Pipeline**: Triggered-implication absorption, common-disjunct extraction, disjunct sorting, role automata, and nominal-schema processing
- **Multi-Level Tableau Caching**: Dedicated caches for unsatisfiability, SAT expansion, completion graphs, saturation results, and consequences
- **Parallel Tableau Expansion**: `ParallelTableauExpander` powered by Rayon for large-ontology node expansion
- **Datatype Value Space Handlers**: XSD-compliant `ValueSpaceHandler` trait and `ValueSpaceRegistry` for boolean, string, numeric, datetime, and IRI value spaces
- **Saturation Cycle Detection**: `CycleDetector` with DashMap and atomics to guard the saturation engine against infinite loops
- **Enhanced Server Completeness**: 15+ additional OWLlink request variants and 5 new REST routes
- **W3C OWL 2 Conformance Test Suite**: 63 tests covering all six W3C test categories (consistency, inconsistency, entailment, non-entailment, profile conformance, syntax round-trip) across all five OWL 2 profiles
- **Transitive SubClassOf Closure**: `is_subclass_of` now traverses multi-hop subsumption chains via BFS (A⊑B, B⊑C ⊢ A⊑C), correctly handling deep class hierarchies
- **Pre-consistency Fast Checks**: O(n) axiom scans detect `ClassAssertion(owl:Nothing :x)`, `SubClassOf(owl:Thing owl:Nothing)`, and functional property violations before tableau invocation
- **Named-class Concept Unfolding**: Tableau now stores `SubClassOf(:A :B)` as unfolding rules alongside complex expressions, enabling complement-clash detection for nominal nodes
- **ObjectPropertyChain Functional Syntax**: Parser now handles `SubObjectPropertyOf(ObjectPropertyChain(:p :q) :r)` per the W3C OWL 2 Functional Syntax specification
- **Performance Regression Tests**: 6 ORE-2015 regression tests guarding ontology loading, consistency, classification, and query latency

### v0.10.0 Highlights

- **RDF-star Support**: Full implementation of quoted triples (`<< >>` syntax) for meta-level statements
- **RDF 1.2 Compliance**: Directional literals (`rdf:dirLangString`), well-formedness rules
- **Automatic Reification**: Bidirectional conversion between RDF-star and RDF 1.1 reification
- **SPARQL-star**: Query quoted triples with `<< ?s ?p ?o >>` patterns
- **RDF-star Validation**: Structural constraints, nesting limits, position rules
- **Comprehensive Documentation**: Complete guides for RDF-star features and migration

### v0.8.0 Highlights

- **Zero-Overhead Parser State**: Compile-time optimizations with no runtime cost for successful parsing
- **O(1) Keyword Validation**: Perfect hash tables for comprehensive OWL keyword checking
- **Configurable Error Verbosity**: Three levels (Minimal/Standard/Detailed) for performance tuning

### Competitive Advantages

- **Performance**: EL reasoning with polynomial complexity vs exponential in full DL; O(1) indexed completion rules eliminate O(N<sup>2</sup>) subsumption scans
- **Ecosystem Integration**: SPARQL endpoint for Semantic Web compatibility
- **Developer Experience**: RESTful APIs, comprehensive explanations, and detailed error reporting
- **Standards Compliance**: OWLlink protocol support for interoperability with existing tools

## Installation

For usage with Docker, see [DOCKER.md](DOCKER.md).
For Podman, see [PODMAN.md](PODMAN.md).

### Prerequisites

- Rust 1.88 or higher
- Cargo (comes with Rust)

### System Dependencies

Oxidowl uses [Oxigraph](https://github.com/oxigraph/oxigraph) with the `rocksdb` feature, which compiles [RocksDB](https://rocksdb.org/) from source during the build. This requires a **C++17-capable compiler** and **CMake** to be installed on your system.

#### Linux (Ubuntu / Debian)

```bash
sudo apt-get update
sudo apt-get install -y build-essential cmake clang libclang-dev
```

#### Linux (Fedora / RHEL / Rocky)

```bash
sudo dnf install gcc-c++ cmake clang clang-devel
```

#### macOS

Xcode Command Line Tools provide the necessary compiler. CMake can be installed via Homebrew:

```bash
xcode-select --install
brew install cmake
```

#### Windows

Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the **"Desktop development with C++"** workload selected. CMake is bundled with Visual Studio, or can be installed separately from [cmake.org](https://cmake.org/download/).

> **Note**: If you do not need the on-disk SPARQL store you can opt out of the RocksDB dependency by disabling the `sparql-store` and `sparql` features in your `Cargo.toml`:
> ```toml
> oxidowl = { version = "1.0", default-features = false }
> ```

### From Source

```bash
git clone https://github.com/sievericcardo/oxidowl.git
cd oxidowl
cargo build --release
```

### As a Library

Add to your `Cargo.toml`:

```toml
[dependencies]
oxidowl = { version = "1.0", features = ["default"] }

# Or with specific features:
oxidowl = { version = "1.0", features = ["server", "ml", "sparql"] }
```

### Cargo Feature Flags

| Feature | Default | Description |
|---|---|---|
| `parallel` | ✅ | Rayon-based multi-threaded reasoning |
| `cache` | ✅ | DashMap-backed concurrent caches |
| `high-performance` | ✅ | mimalloc high-performance allocator |
| `http-imports` | ✅ | HTTP-based ontology import fetching (reqwest) |
| `sparql-store` | ✅ | In-process Oxigraph SPARQL store |
| `server` | ❌ | REST API, OWLlink, and SPARQL HTTP servers |
| `sparql` | ❌ | Full Oxigraph SPARQL engine (alias for `sparql-store`) |
| `ml` | ❌ | Candle-backed ML-enhanced query engine |
| `explanations` | ❌ | Justification and proof generation |
| `profiling` | ❌ | pprof flamegraph + dhat heap profiling |
| `kani` | ❌ | Kani formal-verification harnesses |

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test unit::reasoning
cargo test integration::greenhouse

# RDF-star integration tests
cargo test --test rdf_star_integration_tests

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
use std::sync::Arc;
use oxidowl::{
    Reasoner, ReasonerConfig, DLQueryEngine, ReasoningService,
    OntologyFormat, Result, TableauAlgorithm
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
    println!("Ontology is consistent: {is_consistent}");

    // Perform classification (auto-detects EL profile for 100x speedup)
    let classification = reasoner.classify()?;
    println!("Classification completed with {} classes", classification.hierarchy.len());

    // Create reasoning service and DL query engine
    let ontology = reasoner.get_ontology()?;
    let ontology_data = ontology.read().unwrap().clone();
    let reasoning_service = ReasoningService::new(ontology_data, config);
    let query_engine = DLQueryEngine::new_with_namespace(
        Arc::new(reasoning_service),
        "http://example.org#".to_string()
    );

    // Execute DL queries with DisjointUnion support
    let result = query_engine.execute_query("Person and (hasChild some Thing)").await?;
    println!("Query returned: {result:?}");

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

## Algorithm Selection: Traditional vs Hypertableau

Oxidowl provides two tableau-based reasoning algorithms optimized for different ontology characteristics:

### Traditional Tableau (Default)

- Classic tableau expansion with blocking
- Best for general-purpose reasoning
- Consistent performance across all scenarios
- **Use when:** Ontology has many equivalent classes, large linear taxonomies, or unknown characteristics

### Hypertableau Algorithm

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
|---|---|---|
| Many disjoint classes (>10) | Hypertableau | 3-9x faster |
| Complex expressions (<50) | Hypertableau | 8-13% faster |
| Large linear taxonomy (>50 classes) | Traditional | 2-5x faster |
| Many equivalent classes (>10) | Traditional | 2-7x faster |
| General purpose / unknown | Traditional | Safer default |

## Features In-Depth

For detailed documentation of all features including code examples, see:

- **[Features Guide](docs/FEATURES.md)** — EL Profile Optimization, Explanations, Server Support, Import Resolution, Configuration, SHACL Validation, OWL 2 Profile Reasoning, Distributed Reasoning, SPARQL UPDATE, ABox Reasoning, ML-Enhanced Query Engine, Incremental Classification, Kani Verification, Cache Strategies

## Architecture

For the full architecture overview and component tree, see:

- **[Architecture Guide](docs/ARCHITECTURE.md)** — Core components, tableau algorithms, and performance optimizations

## RDF 1.2 and RDF-star Support

Oxidowl provides comprehensive support for [RDF-star](https://w3c.github.io/rdf-star/) and [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) specifications, enabling direct representation of metadata about statements without cumbersome reification.

### Key Features

- **Quoted Triples**: Use `<< >>` syntax for meta-level statements
- **SPARQL-star**: Query quoted triples with `<< ?s ?p ?o >>` patterns
- **Nested Structures**: Support for deeply nested quoted triples (up to 5 levels by default)
- **RDF 1.1 Compatibility**: Automatic bidirectional conversion between RDF-star and RDF 1.1 reification
- **Directional Literals**: `rdf:dirLangString` for RTL/LTR text direction

For detailed guides and code examples, see:

- [RDF-star Guide](docs/RDF_STAR_GUIDE.md) — Comprehensive usage guide
- [RDF Compatibility Guide](docs/RDF_COMPATIBILITY.md) — Migration and conversion details

Additional documentation:

- [SHACL Guide](docs/SHACL.md) — SHACL Core and SHACL-SPARQL reference
- [OWL 2 Profile Reasoners](docs/PROFILE_REASONERS.md) — EL / QL / RL profile details
- [Error Handling](docs/ERROR_HANDLING.md) — Error types and handling guide

## Examples

Code examples have been moved to **[docs/EXAMPLES.md](docs/EXAMPLES.md)**:

- Basic Reasoning with classification and consistency checking
- DL Queries with Manchester Syntax and DisjointUnion support
- SWRL Rule Execution with Forward/Backward/Hybrid chaining strategies
- SWRL Built-in Predicates (30+ predicates across math, string, datetime, regex categories)
- Custom SWRL Built-in registration
- Server Mode (OWLlink and SPARQL)

Runnable examples are also available:

- [examples/dl_query_example.rs](examples/dl_query_example.rs)
- [examples/sparql_update_example.rs](examples/sparql_update_example.rs)
- [examples/server_example.rs](examples/server_example.rs)
- [examples/library_usage.rs](examples/library_usage.rs)

## Testing

### Test Categories

- **Unit Tests**: Reasoning algorithms, ontology operations, parser functionality, configuration, RDF-star semantics, cache strategy correctness
- **Integration Tests**: Real ontology processing (greenhouse DT), algorithm comparison, RDF-star workflows, SHACL validation, ML engine, SPARQL UPDATE
- **Performance Regression Tests**: ORE-2015 classification guards — consistency, subsumption, classification latency, and query responsiveness
- **Benchmarks**: Core tableau performance, hypertableau vs traditional, SHACL constraint throughput, parsing throughput, ML query engine latency

### Test Execution

```bash
# Quick sanity suite
cargo test quick

# Full test suite
cargo test

# Performance tests (slower, release mode)
cargo test --release -- --ignored

# Specific test suites
cargo test --test rdf_star_integration_tests
cargo test --test shacl_tests
cargo test --test ml_engine_integration_tests_simple

# W3C OWL 2 conformance suite (63 tests)
cargo test --test owl2_conformance_suite

# Criterion benchmarks
cargo bench
cargo bench --bench hypertableau_benchmark
cargo bench --bench shacl_benchmark
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

### Planned for future releases

- [ ] WebAssembly (WASM) compilation
- [ ] Python bindings
- [ ] GPU acceleration for large-scale reasoning
- [ ] Streaming / memory-mapped ontology processing

### Implemented in v1.0.0

- [x] **OWL 2 RL profile reasoner** — Forward-chaining materialisation with incremental update support
- [x] **Incremental classification** — Dependency-tracked reclassification of affected concepts only
- [x] **Distributed reasoning** — Cluster engine with automatic node discovery and fault tolerance
- [x] **SHACL Core + SHACL-SPARQL** — Full W3C SHACL validation backed by Oxigraph
- [x] **SPARQL UPDATE** — `INSERT DATA` / `DELETE DATA` for dynamic ontology mutation
- [x] **ML-enhanced query engine** — Candle-backed neural conjunctive query optimizer
- [x] **Kani formal verification** — Harnesses for memory-safety and correctness proofs
- [x] **Advanced cache strategies** — LRU, LFU, LRUFU, size-based, TTL eviction policies
- [x] **Lock-free data structures** — DashMap-backed caches for concurrent access
- [x] **Docker/Podman containerisation** — Official `Dockerfile` and `docker-compose.yml`
- [x] **Advanced preprocessing pipeline** — Triggered-implication absorption, common-disjunct extraction, disjunct sorting, role automata, nominal-schema processing
- [x] **Multi-level tableau caching** — Dedicated caches for unsatisfiability, SAT expansion, completion graphs, saturation, and consequences
- [x] **Parallel tableau expansion** — `ParallelTableauExpander` with Rayon for large-ontology node expansion
- [x] **Datatype value-space handlers** — XSD-compliant `ValueSpaceHandler` trait for boolean, string, numeric, datetime, and IRI datatypes
- [x] **Saturation cycle detection** — `CycleDetector` to guard saturation engine against infinite loops
- [x] **Enhanced server completeness** — 15+ new OWLlink request variants and 5 new REST API routes
- [x] **W3C OWL 2 Conformance Test Suite** — 63 tests covering consistency, inconsistency, entailment, non-entailment, profile conformance, and syntax round-trip across all five OWL 2 profiles
- [x] **Transitive SubClassOf closure** — BFS-based multi-hop subsumption in `is_subclass_of` (A⊑B, B⊑C ⊢ A⊑C)
- [x] **Pre-consistency fast checks** — O(n) axiom scans for owl:Nothing assertions, owl:Thing⊑owl:Nothing, and functional property violations before tableau invocation
- [x] **Named-class concept unfolding** — Tableau unfolding rules for `SubClassOf(:A :B)` enabling complement-clash detection on nominal nodes
- [x] **ObjectPropertyChain functional syntax** — `SubObjectPropertyOf(ObjectPropertyChain(:p :q) :r)` now parsed and emitted correctly

### Implemented in v0.10.0

- [x] **RDF-star and RDF 1.2 support** — Complete quoted triple implementation with SPARQL-star
- [x] **OWL 2 EL/QL/RL profile validators** — Full W3C profile validation for all sub-profiles
- [x] **SWRL rule support** — Complete implementation with 30+ built-in predicates
- [x] **DisjointUnion axiom support** — Full support in DL queries and reasoning
- [x] **Advanced DL Query Engine** — Manchester Syntax with union query optimisation

## License

This project is licensed under the GNU Lesser General Public License v3.0 (LGPL-3.0) — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the [HermiT](https://github.com/owlcs/hermit-reasoner/) reasoner and [Konclude](https://github.com/konclude/Konclude) for their advanced reasoning techniques
- Built with the Rust ecosystem's excellent crates
- Thanks to the OWL and Semantic Web communities
- Thanks to Eduard Kamburjan for his insightful feedback to the development of Oxidowl
- Thanks to Tobias John for the valuable discussions on language-based testing and mutation-based testing that helped improve the robustness of Oxidowl's reasoning engine

## Support

Please report any issues or feature requests and feel free to contribute!

- [Documentation](docs/)
- [Issue Tracker](https://github.com/sievericcardo/oxidowl/issues)
- [Discussions](CONTRIBUTING.md)

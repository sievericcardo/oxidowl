# <img src="oxidowl.webp" alt="Oxidowl Logo generated with Google Gemini" width="35"/> Oxidowl

A high-performance Description Logic reasoner for OWL 2 DL ontologies, implemented in Rust with advanced tableau algorithms, parallel computation, and integrated horned-owl support.

[![License: BSD-3-Clause](https://img.shields.io/badge/License-BSD%203--Clause-blue.svg)](https://opensource.org/licenses/BSD-3-Clause)
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

- 🎯 **OWL 2 Profile Support**: EL (polynomial), QL (query), RL (rule) optimised reasoners plus full DL validator
- 💡 **Comprehensive Explanations**: Justification generation, proof tracking, and laconic explanations
- 🌐 **Multi-Protocol Server Support**: OWLlink, SPARQL endpoint, and REST API interfaces
- 📥 **Advanced Import Resolution**: Recursive imports, cycle detection, and IRI mapping
- 🧪 **SWRL Rule Support**: Full Semantic Web Rule Language implementation with 30+ built-in predicates
- ✅ **SHACL Validation**: Complete SHACL Core + SHACL-SPARQL backed by Oxigraph
- 🌐 **Distributed Reasoning**: Cluster-based horizontal scaling with fault tolerance and load balancing
- 🤖 **ML-Enhanced Queries**: Neural-network-assisted conjunctive query optimization (Candle framework)
- 📈 **Incremental Classification**: Only re-reasons over concepts affected by ontology changes
- 🛡️ **Formal Verification**: Kani harnesses for memory-safety proofs
- 🗃️ **Advanced Caching**: LRU, LFU, LRUFU, TTL, and size-based eviction strategies

#### v1.0.0 Highlights (Latest)

- 🏗️ **OWL 2 RL Profile Reasoner**: Forward-chaining materialization with incremental update support
- 🌐 **Distributed Reasoning**: Cluster-based horizontal scaling with automatic node discovery and fault tolerance
- 🤖 **ML-Enhanced Query Engine**: Candle-backed neural query optimizer for conjunctive query execution
- ✅ **SHACL Core + SHACL-SPARQL**: Full W3C SHACL validation backed by the embedded Oxigraph SPARQL store
- 🔄 **SPARQL UPDATE**: INSERT DATA and DELETE DATA support for dynamic ontology mutation
- 📈 **Incremental Classification**: Dependency-tracked reclassification of only affected concepts on ontology updates
- 🔒 **Lock-Free Data Structures**: DashMap-backed caches for highly concurrent access patterns
- 🛡️ **Kani Formal Verification**: Harnesses for memory-safety and correctness proofs via `cargo kani`
- 🗃️ **Advanced Cache Strategies**: LRU, LFU, LRUFU, TTL and size-based eviction policies

#### v0.10.0 Highlights

- ⭐ **RDF-star Support**: Full implementation of quoted triples (`<< >>` syntax) for meta-level statements
- 🔄 **RDF 1.2 Compliance**: Directional literals (`rdf:dirLangString`), well-formedness rules
- 🔁 **Automatic Reification**: Bidirectional conversion between RDF-star and RDF 1.1 reification
- 🎯 **SPARQL-star**: Query quoted triples with `<< ?s ?p ?o >>` patterns
- 🔍 **RDF-star Validation**: Structural constraints, nesting limits, position rules
- 📚 **Comprehensive Documentation**: Complete guides for RDF-star features and migration

#### v0.8.0 Highlights

- 🔥 **Zero-Overhead Parser State**: Compile-time optimizations with no runtime cost for successful parsing
- ⚡ **O(1) Keyword Validation**: Perfect hash tables for comprehensive OWL keyword checking
- 📝 **Configurable Error Verbosity**: Three levels (Minimal/Standard/Detailed) for performance tuning

#### Competitive Advantages

- **Performance**: EL reasoning with polynomial complexity vs exponential in full DL
- **Ecosystem Integration**: SPARQL endpoint for Semantic Web compatibility
- **Developer Experience**: RESTful APIs, comprehensive explanations, and detailed error reporting
- **Standards Compliance**: OWLlink protocol support for interoperability with existing tools

## Installation

For usage with Docker, see [DOCKER.md](DOCKER.md).
For Podman, see [PODMAN.md](PODMAN.md).

### Prerequisites

- Rust 1.88 or higher
- Cargo (comes with Rust)

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

### SHACL Validation

Oxidowl implements the full [W3C SHACL specification](https://www.w3.org/TR/shacl/) — both **SHACL Core** and **SHACL-SPARQL** — backed by the embedded Oxigraph SPARQL store.

```rust
use oxidowl::validation::shacl::ShaclValidator;

let shapes_ttl = r#"
  @prefix sh:  <http://www.w3.org/ns/shacl#> .
  @prefix ex:  <http://example.org/> .
  @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

  ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
      sh:path     ex:name ;
      sh:datatype xsd:string ;
      sh:minCount 1 ;
    ] .
"#;

let data_ttl = r#"
  @prefix ex: <http://example.org/> .
  ex:Alice a ex:Person ; ex:name "Alice" .
  ex:Bob   a ex:Person .   # missing ex:name — violation
"#;

let mut validator = ShaclValidator::new(shapes_ttl, data_ttl)?;
let report = validator.validate()?;

println!("conforms: {}", report.conforms);
for result in &report.results {
    println!("  violation at: {:?}", result.focus_node);
}
```

**Supported SHACL constraints include:**

| Category | Constraints |
|---|---|
| Value type | `sh:class`, `sh:datatype`, `sh:nodeKind` |
| Cardinality | `sh:minCount`, `sh:maxCount` |
| Value range | `sh:minExclusive/Inclusive`, `sh:maxExclusive/Inclusive` |
| String-based | `sh:minLength`, `sh:maxLength`, `sh:pattern`, `sh:languageIn`, `sh:uniqueLang` |
| Property pair | `sh:equals`, `sh:disjoint`, `sh:lessThan`, `sh:lessThanOrEquals` |
| Logical | `sh:not`, `sh:and`, `sh:or`, `sh:xone` |
| Shape-based | `sh:node`, `sh:qualifiedValueShape` |
| Other | `sh:closed`, `sh:hasValue`, `sh:in` |
| SPARQL | `sh:sparql` SELECT-based constraints |

All five W3C target types are supported: `sh:targetClass`, `sh:targetNode`, `sh:targetSubjectsOf`, `sh:targetObjectsOf`, and implicit class targets.

See [docs/SHACL.md](docs/SHACL.md) for the full constraint reference and architecture overview.

### OWL 2 Profile Reasoning

Oxidowl ships specialised reasoners and validators for all four OWL 2 sub-profiles, allowing ontologies to opt into lighter-weight reasoning engines when the full DL semantics are not required:

| Profile | Complexity | Implementation |
|---|---|---|
| OWL 2 EL | Polynomial time | `ELReasoner` — completion-rule saturation |
| OWL 2 QL | Polynomial query answering | `QLValidator` — conjunctive query rewriting |
| OWL 2 RL | Polynomial materialisation | `RLReasoner` — forward-chaining materialisation |
| OWL 2 DL | ExpTime-complete | Default tableau / hypertableau engine |

#### OWL 2 RL Reasoner

```rust
use oxidowl::profiles::rl_reasoner::RLReasoner;
use oxidowl::config::ReasonerConfig;

let mut rl_reasoner = RLReasoner::new(ReasonerConfig::default());
rl_reasoner.load_axioms(ontology.axioms())?;
let classification = rl_reasoner.classify().await?;
println!("RL materialisation complete — {} facts inferred",
         classification.hierarchy.len());
```

#### Profile Validation

```rust
use oxidowl::profiles::{ProfileValidator, OWL2Profile};

// Validate that an ontology conforms to a specific profile
let report = ontology.validate_profile(OWL2Profile::EL)?;
if !report.is_valid() {
    for violation in report.violations() {
        eprintln!("Profile violation: {:?}", violation);
    }
}
```

### Distributed Reasoning

Oxidowl supports horizontal scaling through a built-in cluster engine. Large ontology reasoning tasks are automatically partitioned, distributed, and their results aggregated:

```rust
use oxidowl::distributed::{DistributedConfig, ClusterManager};

let config = DistributedConfig {
    node_config: NodeConfig::local(8100),
    cluster_config: ClusterConfig {
        discovery: DiscoveryMode::Static(vec![
            "127.0.0.1:8101".parse()?,
            "127.0.0.1:8102".parse()?,
        ]),
        heartbeat_interval: Duration::from_secs(5),
        ..Default::default()
    },
    ..Default::default()
};

let cluster = ClusterManager::start(config).await?;
let result = cluster.execute_distributed_query(query).await?;
println!("Distributed result from {} nodes", result.node_count());
```

**Cluster capabilities:**

- Automatic node discovery and health monitoring
- Intelligent query decomposition and partitioning
- Parallel result collection and merging
- Automatic re-execution on node failure
- Dynamic load balancing based on node capacity

### SPARQL UPDATE

Oxidowl supports dynamic ontology mutation via SPARQL `INSERT DATA` and `DELETE DATA` operations, proxied through the embedded Oxigraph store:

```rust
// INSERT DATA — add class assertions at runtime
let insert = r#"
    INSERT DATA {
        <http://example.org/John> rdf:type <http://example.org/Person> .
        <http://example.org/John> <http://example.org/age> "30" .
    }
"#;
reasoner.execute_sparql_query(insert)?;

// DELETE DATA — retract triples
let delete = r#"
    DELETE DATA {
        <http://example.org/John> <http://example.org/age> "30" .
    }
"#;
reasoner.execute_sparql_query(delete)?;
```

See [examples/sparql_update_example.rs](examples/sparql_update_example.rs) for complete usage.

### ML-Enhanced Query Engine

The advanced conjunctive query engine integrates a Candle-backed neural optimizer to predict join orderings, pruning strategies, and execution priorities at runtime:

```rust
use oxidowl::query::advanced::{AdvancedExecutionEngine, AdvancedExecutionConfig};
use oxidowl::query::advanced::execution_engine::{ExecutionConstraints, ExecutionPriority};

let config = AdvancedExecutionConfig::default();
let engine = AdvancedExecutionEngine::new(ontology_arc, reasoning.clone(), config);

let constraints = ExecutionConstraints {
    max_execution_time: Some(Duration::from_secs(10)),
    min_confidence: Some(0.8),
    priority: ExecutionPriority::High,
    ..Default::default()
};

let result = engine.execute(query, constraints).await?;
println!("Executed {} atoms with {} joins", result.atom_count, result.join_count);
```

Enable with the `ml` feature flag:

```toml
oxidowl = { version = "1.0", features = ["ml"] }
```

### Incremental Classification

When ontologies are updated during a session, Oxidowl's incremental classifier re-reasons only over the concepts whose dependencies have changed, avoiding a full re-classification:

```rust
use oxidowl::core::incremental::DependencyTracker;

// The reasoner automatically tracks inter-concept dependencies
// When an axiom is added or removed, only affected concepts are re-classified
reasoner.add_axiom(new_axiom)?;
let delta = reasoner.classify_incrementally()?;

println!("Incremental update: {} concepts reclassified (of {} total)",
         delta.reclassified, delta.total);
```

### Formal Verification with Kani

Oxidowl ships Kani proof harnesses that can be run with `cargo kani` to verify critical memory-safety and correctness properties of the core algorithms:

```bash
# Run all Kani proof harnesses (requires kani installed)
cargo kani

# Enable the kani feature manually
cargo build --features kani
```

The harnesses cover tableau expansion rules, blocking checks, and cache invariants. See the `src/proofs/` directory for the full set of proofs.

### Advanced Cache Strategies

Oxidowl provides a tunable multi-strategy cache layer for blocking candidates, subsumption results, and classification outcomes:

```rust
use oxidowl::cache_strategies::{LRUCache, LFUCache, EvictionStrategy};

// Choose an eviction policy suited to your workload
let mut cache = LRUCache::new(10_000);                  // Recency-based
let mut cache = LFUCache::with_strategy(                 // Frequency-based
    EvictionStrategy::LRUFU, 10_000);

// TTL-based expiry for time-sensitive data
let mut ttl_cache = LRUCache::with_ttl(1_000, Duration::from_secs(60));
```

Available eviction strategies:

| Strategy | Description |
|---|---|
| `LRU` | Evict least-recently-used entries |
| `LFU` | Evict least-frequently-used entries |
| `LRUFU` | Combine recency and frequency scoring |
| `SizeBased` | Evict when aggregate byte size exceeds limit |
| `TTL` | Evict entries older than a time-to-live threshold |

## Architecture

### Core Components

- **`core`** - Core reasoning engine with tableau algorithms
  - `reasoner/` - Main reasoner interface
  - `tableau/` - Tableau expansion with node and edge management
  - `blocking.rs` - Anywhere blocking with cycle detection
  - `completion.rs` - Completion rules and caching
  - `incremental.rs` - Dependency-tracked incremental classification
  - `hypergraph/` - Structural-sharing hypergraph for Hypertableau
  - `saturation/` - Rule-saturation engine for RL/EL profiles
  - `inverted_index.rs` - Inverted index for fast concept lookup
  - `persistent_collections.rs` - Immutable persistent data structures

- **`ontology`** - Ontology representation and management (built on horned-owl)
  - `axioms.rs` - Axiom structures and operations including DisjointUnion
  - `concepts.rs` - Class expressions and concepts
  - `properties.rs` - Object and data properties
  - `individuals.rs` - ABox individuals and assertions

- **`parsers`** - Input format support via horned-owl integration
  - `owl_xml.rs` - OWL XML parser with DisjointUnion support
  - `functional.rs` - Functional syntax parser with configurable error verbosity
  - `rdf_xml.rs` - RDF/XML parser
  - `turtle.rs` - Turtle format parser

- **`reasoning`** - High-level reasoning coordination

- **`query`** - Query engines
  - `dl_query.rs` - DL query engine with Manchester Syntax and union query support
  - `sparql_store.rs` - In-process Oxigraph SPARQL store wrapper
  - `advanced/` - ML-enhanced conjunctive query execution engine

- **`profiles`** - OWL 2 sub-profile support
  - `el_reasoner.rs` - OWL 2 EL polynomial-time reasoner
  - `ql.rs` - OWL 2 QL profile validator
  - `rl.rs` - OWL 2 RL profile validator
  - `rl_reasoner.rs` - OWL 2 RL forward-chaining materialisation engine
  - `dl.rs` - OWL 2 DL validator
  - `validator.rs` - Unified profile validation interface

- **`validation`** - Structural validation
  - `owl2_dl.rs` - OWL 2 DL validator
  - `shacl/` - Full SHACL Core + SHACL-SPARQL engine (10+ constraint categories)

- **`swrl`** - SWRL (Semantic Web Rule Language) implementation
  - `engine.rs` - SWRL rule execution engine with multiple strategies
  - `interpreter.rs` - Individual rule interpretation and execution
  - `parser.rs` - SWRL syntax parsing
  - `builtins.rs` - Core built-in predicates (math, string, boolean)
  - `datetime_builtins.rs` - Date/time built-in predicates (15+ predicates)
  - `regex_builtins.rs` - Regular expression built-in predicates
  - `validation.rs` - SWRL rule validation

- **`distributed`** - Cluster-based horizontal scaling
  - `cluster.rs` - Node discovery, health monitoring, and lifecycle
  - `query_distribution.rs` - Intelligent query splitting and distribution
  - `result_aggregation.rs` - Parallel result collection and merging
  - `fault_tolerance.rs` - Failure detection and recovery strategies
  - `load_balancing.rs` - Dynamic workload distribution

- **`semantics`** - RDF, RDFS, and OWL 2 semantics
  - RDF-star support (quoted triples, `rdf:reifies`, directional literals)
  - RDF 1.2 compliance

- **`import`** - Import resolution
  - Recursive imports, cycle detection, and IRI-to-file mapping
  - Optional HTTP fetching (feature `http-imports`)

- **`adapter`** - Horned-OWL integration layer for enhanced parsing and modeling

- **`explanation`** - Justification and proof generation

- **`cache`** / **`cache_lockfree`** / **`cache_strategies`** - Multi-strategy caching layer
  - LRU, LFU, LRUFU, size-based, and TTL eviction policies
  - Lock-free DashMap-backed variant for highly concurrent access

- **`performance`** / **`profiling`** - Performance monitoring and flamegraph/heap profiling

- **`dl_clauses`** - DL clause generation and dumping

- **`server`** *(feature `server`)* - Web interfaces
  - `rest.rs` - REST API (port configurable)
  - `owllink.rs` - OWLlink XML protocol
  - `sparql.rs` - SPARQL HTTP endpoint

- **`proofs`** *(feature `kani`)* - Kani formal-verification harnesses

- **`config`** - Configuration management and optimisation

- **`visitor`** - Visitor pattern for ontology traversal

### Algorithms

#### Tableau Algorithm

Oxidowl implements an efficient tableau algorithm that provides:

- **Systematic Expansion**: Sound and complete reasoning through node expansion
- **Optimized Rule Application**: Smart ordering and caching of tableau rules
- **Advanced Blocking**: Anywhere blocking with cycle detection for termination
- **Dependency Tracking**: Intelligent backtracking and conflict resolution
- **Memory Management**: Efficient data structures for large ontologies

#### Performance Optimizations

- **Parallel Processing**: Multi-threaded reasoning with Rayon for large ontologies
- **Caching**: LRU/LFU/LRUFU caches with configurable eviction policies
- **Lock-Free Structures**: DashMap-backed caches for concurrent access  
- **Incremental Reasoning**: Dependency-tracked reclassification on updates
- **High-Performance Allocator**: mimalloc (feature `high-performance`)
- **Compile-Time Hashing**: Perfect hash tables (`phf`) for O(1) keyword lookup

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

## RDF 1.2 and RDF-star Support

Oxidowl provides comprehensive support for [RDF-star](https://w3c.github.io/rdf-star/) and [RDF 1.2](https://www.w3.org/TR/rdf12-concepts/) specifications, enabling direct representation of metadata about statements without cumbersome reification.

### Key Features

#### 1. Quoted Triples

Use `<< >>` syntax to quote triples for meta-level statements:

```rust
use oxidowl::semantics::{RdfGraph, RdfTerm, Triple};

// Create a quoted triple: << :alice :knows :bob >>
let alice = RdfTerm::iri("http://example.org/alice")?;
let knows = RdfTerm::iri("http://example.org/knows")?;
let bob = RdfTerm::iri("http://example.org/bob")?;

let base_triple = Triple::new(alice, knows, bob);
let quoted = RdfTerm::QuotedTriple(Box::new(base_triple));

// Add metadata: << :alice :knows :bob >> :confidence 0.95
let confidence = RdfTerm::iri("http://example.org/confidence")?;
let value = RdfTerm::literal("0.95");

graph.add_triple(Triple::new(quoted, confidence, value));
```

#### 2. SPARQL-star Queries

Query quoted triples using SPARQL-star syntax:

```sparql
PREFIX ex: <http://example.org/>

SELECT ?s ?conf WHERE {
  << ?s ex:knows ex:bob >> ex:confidence ?conf .
  FILTER(?conf > 0.8)
}
```

#### 3. Nested Structures

Support for deeply nested quoted triples (up to 5 levels by default):

```rust
// 2-level nesting: << << :a :b :c >> :d :e >> :f :g
let inner = Triple::new(a, b, c);
let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner));
let middle = Triple::new(inner_quoted, d, e);
let middle_quoted = RdfTerm::QuotedTriple(Box::new(middle));
let outer = Triple::new(middle_quoted, f, g);
```

#### 4. RDF 1.1 Compatibility

Automatic bidirectional conversion between RDF-star and RDF 1.1 reification:

```rust
use oxidowl::adapter::HornedOwlAdapter;

let mut adapter = HornedOwlAdapter::new();
adapter.set_rdf11_mode(true);

// Automatically converts quoted triples to reification vocabulary
let (reified_term, reification_triples) = adapter.reify_rdf_term(&quoted)?;
```

#### 5. RDF 1.2 Features

- **Directional Literals**: `rdf:dirLangString` for RTL/LTR text direction
- **Well-Formedness Rules**: Strict validation of blank node labels
- **Semantic Extensions**: Support for `rdf:reifies` predicate

### Use Cases

- **Provenance Tracking**: Record source and timestamp of statements
- **Confidence Scores**: Annotate statements with certainty levels
- **Named Graphs Metadata**: Add metadata about entire graphs
- **Temporal Information**: Track when statements become valid/invalid
- **Access Control**: Attach permissions to specific statements

### Documentation

- [RDF-star Guide](docs/RDF_STAR_GUIDE.md) - Comprehensive usage guide
- [RDF Compatibility Guide](docs/RDF_COMPATIBILITY.md) - Migration and conversion details
- [SHACL Guide](docs/SHACL.md) - SHACL Core and SHACL-SPARQL reference
- [OWL 2 Profile Reasoners](docs/PROFILE_REASONERS.md) - EL / QL / RL profile details
- [Error Handling](docs/ERROR_HANDLING.md) - Error types and handling guide

### Examples

- [examples/dl_query_example.rs](examples/dl_query_example.rs) - DL query engine usage
- [examples/sparql_update_example.rs](examples/sparql_update_example.rs) - SPARQL INSERT/DELETE
- [examples/server_example.rs](examples/server_example.rs) - REST / OWLlink server
- [examples/library_usage.rs](examples/library_usage.rs) - Library API walkthrough
- [tests/rdf_star_integration_tests.rs](tests/rdf_star_integration_tests.rs) - RDF-star integration tests

### Validation

oxidowl validates RDF-star structures according to W3C specifications:

- ✅ Quoted triples in subject/object positions
- ❌ Quoted triples in predicate position (forbidden)
- ✅ Configurable nesting depth limits (default: 5)
- ✅ Well-formed blank node labels
- ✅ Directional literal validation

```rust
use oxidowl::validation::owl2_dl::OWL2DLValidator;

let validator = OWL2DLValidator::new();
let result = validator.validate(&ontology)?;

if !result.is_valid() {
    for error in result.errors() {
        eprintln!("Validation error: {:?}", error);
    }
}
```

## Testing

### Test Categories

- **Unit Tests**: Individual component testing
  - Reasoning algorithms (tableau, hypertableau, EL, RL)
  - Ontology operations
  - Parser functionality (OWL XML, Functional, Turtle, RDF/XML)
  - Configuration management
  - RDF-star semantics
  - Cache strategy correctness

- **Integration Tests**: End-to-end testing
  - Real ontology processing (greenhouse DT ontology)
  - Algorithm comparison (traditional vs hypertableau)
  - RDF-star workflows
  - SHACL validation end-to-end
  - ML engine integration
  - SPARQL UPDATE operations

- **Benchmarks** (Criterion-based):
  - `reasoning_benchmark` — core tableau performance
  - `hypertableau_benchmark` — hypertableau vs traditional
  - `shacl_benchmark` — SHACL constraint throughput
  - `parser_benchmark` — parsing throughput
  - `ml_engine_benchmark` — ML query engine latency

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

### Implemented in v0.10.0

- [x] **RDF-star and RDF 1.2 support** — Complete quoted triple implementation with SPARQL-star
- [x] **OWL 2 EL/QL/RL profile validators** — Full W3C profile validation for all sub-profiles
- [x] **SWRL rule support** — Complete implementation with 30+ built-in predicates
- [x] **DisjointUnion axiom support** — Full support in DL queries and reasoning
- [x] **Advanced DL Query Engine** — Manchester Syntax with union query optimisation

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

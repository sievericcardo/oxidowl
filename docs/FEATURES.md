# Features

Oxidowl has been significantly enhanced with improvements that bring it up to competitive standards with major OWL reasoners while adding unique capabilities.

## EL Profile Optimization

For ontologies that conform to the OWL 2 EL profile, Oxidowl provides specialized polynomial-time reasoning. The `Reasoner.classify()` method **auto-detects** EL-conforming ontologies and invokes the EL reasoner automatically, offering a ~100x speedup over full tableau classification:

```rust
use oxidowl::{
    profiles::el_reasoner::ELReasoner,
    config::ReasonerConfig,
    ontology::Ontology,
};

// The main Reasoner auto-detects EL profiles:
//   reasoner.classify() → EL reasoner (if EL-conformant) else full tableau

// Or use the EL reasoner directly:
let mut el_reasoner = ELReasoner::new(ReasonerConfig::default());
el_reasoner.initialize(&ontology)?;
let classification = el_reasoner.classify()?;

println!("EL classification completed in polynomial time!");
```

## Comprehensive Explanation Generation

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

## Multi-Protocol Server Support

Run Oxidowl as a server with multiple protocol interfaces. **Note:** By default, the reasoner runs without servers. Use the `--enable-server` flags to start web services.

### Starting the Server

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

### SPARQL Endpoint

```bash
# Query via SPARQL (using oxigraph integration)
curl -X POST http://localhost:8081/sparql \
  -H "Content-Type: application/sparql-query" \
  -d "SELECT ?class WHERE { ?class a owl:Class }"
```

### OWLlink Protocol

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

### REST API

```bash
# Check consistency via REST API
curl -X GET http://localhost:8082/api/v1/consistency

# Perform classification
curl -X POST http://localhost:8082/api/v1/classify

# Get explanations
curl -X POST http://localhost:8082/api/v1/explain \
  -H "Content-Type: application/json" \
  -d '{"inference_type": "subsumption", "axiom": "Human \u2286 Animal"}'
```

### Library Usage with Server

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

See [examples/server_example.rs](../examples/server_example.rs) for a complete example.

## Advanced Import Resolution

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

## Configuration

Oxidowl supports extensive configuration options:

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

let config = ReasonerConfig {
    reasoning: ReasoningConfig {
        tableau_algorithm: TableauAlgorithm::Traditional,
        timeout: Some(std::time::Duration::from_secs(300)),
        ..Default::default()
    },
    ..Default::default()
};
```

## SHACL Validation

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

### Supported SHACL Constraints

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

See [SHACL.md](SHACL.md) for the full constraint reference and architecture overview.

## OWL 2 Profile Reasoning

Oxidowl ships specialised reasoners and validators for all four OWL 2 sub-profiles, allowing ontologies to opt into lighter-weight reasoning engines when the full DL semantics are not required:

| Profile | Complexity | Implementation |
|---|---|---|
| OWL 2 EL | Polynomial time | `ELReasoner` — completion-rule saturation |
| OWL 2 QL | Polynomial query answering | `QLValidator` — conjunctive query rewriting |
| OWL 2 RL | Polynomial materialisation | `RLReasoner` — forward-chaining materialisation |
| OWL 2 DL | ExpTime-complete | Default tableau / hypertableau engine |

### OWL 2 RL Reasoner

```rust
use oxidowl::profiles::rl_reasoner::RLReasoner;
use oxidowl::config::ReasonerConfig;

let mut rl_reasoner = RLReasoner::new(ReasonerConfig::default());
rl_reasoner.load_axioms(ontology.axioms())?;
let classification = rl_reasoner.classify().await?;
println!("RL materialisation complete — {} facts inferred",
         classification.hierarchy.len());
```

### Profile Validation

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

## Distributed Reasoning

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

## SPARQL UPDATE

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

See [examples/sparql_update_example.rs](../examples/sparql_update_example.rs) for complete usage.

## ABox Reasoning and `member()` Queries

Oxidowl performs full **OWL DL ABox classification**: given a set of individuals and a TBox, it can determine whether each individual is a member of any class expression — including complex expressions defined via `owl:equivalentClass`, `owl:someValuesFrom`, `owl:intersectionOf`, `owl:unionOf`, and `owl:hasValue`.

This enables the `member(classExpr)` pattern used in SMOL/SPARQL rule engines:

```rust
use oxidowl::ontology::{ClassExpression, Individual, NamedIndividual, IRI};
use oxidowl::ReasoningService;

// Build class expression: domain:Overloaded (defined in TBox via equivalentClass + someValuesFrom)
let overloaded = ClassExpression::class(IRI::new("http://example.org/Overloaded"));

// Build individual reference
let server = Individual::named("http://example.org/server1");

// OWL DL membership query: is server \u2208 Overloaded?
// This unfolds owl:equivalentClass chains and checks owl:someValuesFrom restrictions
let is_overloaded = reasoning_service.is_member_of(&server, &overloaded).await?;
println!("Server is overloaded: {is_overloaded}");

// Equivalent call via is_instance_of
let also_overloaded = reasoning_service.is_instance_of(&server, &overloaded).await?;

// Get all instances of a class expression (runs over every ABox individual)
let overloaded_servers = reasoning_service.get_instances(&overloaded, false).await?;
println!("Overloaded servers: {}", overloaded_servers.len());
```

### How Instance Membership Is Decided

The checker applies these strategies in order, falling through to the tableau only when earlier stages cannot determine the answer:

| Step | What is checked |
|---|---|
| 1 | Explicit `owl:ClassAssertion` for the target class |
| 2 | Subclass hierarchy — asserted type \u2286 target |
| 3 | `owl:equivalentClass` unfolding — evaluate complex expression against the ABox |
| 4 | Transitive `owl:subClassOf` chains up through the hierarchy |
| Fallback | Full tableau expansion via `check_instance` |

**Step 3** supports the following complex constructors in equivalent-class definitions:

| Constructor | Semantics |
|---|---|
| `owl:intersectionOf` | All operands must be satisfied (conjunction) |
| `owl:unionOf` | At least one operand must be satisfied (disjunction) |
| `owl:someValuesFrom` | \u2265 1 role-filler satisfying the restriction class |
| `owl:hasValue` | Role-filler equals the specified individual |
| `xsd:DataSomeValuesFrom` | \u2265 1 data property value satisfying the datatype |
| `xsd:DataHasValue` | Data property value equals the specified literal |

This means ontologies where a class `C` is defined as:
```turtle
:Overloaded owl:equivalentClass [
    owl:intersectionOf (
        :Server
        [ owl:onProperty :hasLoad ;
          owl:someValuesFrom :HighLoad ]
    )
] .
```
will correctly classify individual `server1` as `:Overloaded` if it has:
- an explicit `rdf:type :Server` assertion, **and**
- an `:hasLoad` property pointing to an individual typed as `:HighLoad`.

## ML-Enhanced Query Engine

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

## Incremental Classification

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

## Formal Verification with Kani

Oxidowl ships Kani proof harnesses that can be run with `cargo kani` to verify critical memory-safety and correctness properties of the core algorithms:

```bash
# Run all Kani proof harnesses (requires kani installed)
cargo kani

# Enable the kani feature manually
cargo build --features kani
```

The harnesses cover tableau expansion rules, blocking checks, and cache invariants. See the `src/proofs/` directory for the full set of proofs.

## Advanced Cache Strategies

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

### Available Eviction Strategies

| Strategy | Description |
|---|---|
| `LRU` | Evict least-recently-used entries |
| `LFU` | Evict least-frequently-used entries |
| `LRUFU` | Combine recency and frequency scoring |
| `SizeBased` | Evict when aggregate byte size exceeds limit |
| `TTL` | Evict entries older than a time-to-live threshold |

# Hypertableau API Reference

Quick reference for the Hypertableau algorithm configuration and usage.

## Configuration Types

### `TableauAlgorithm`

Enum for selecting the tableau algorithm.

```rust
pub enum TableauAlgorithm {
    /// Traditional tableau with blocking
    Traditional,
    
    /// Hypertableau with hypergraph structures
    Hypertableau,
}
```

**Default:** `Traditional`

### `ReasonerConfig`

Main configuration structure for the reasoner.

```rust
pub struct ReasonerConfig {
    pub reasoning: ReasoningConfig,
    // ... other fields
}

pub struct ReasoningConfig {
    /// Tableau algorithm to use
    pub tableau_algorithm: TableauAlgorithm,
    
    /// Enable incremental reasoning
    pub enable_incremental: bool,
    
    // ... other fields
}
```

## Usage Patterns

### Basic Usage

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

// Create config
let mut config = ReasonerConfig::default();

// Set algorithm
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;

// Use config with reasoner
let reasoner = ReasoningService::new(ontology, config);
```

### With Reasoner

```rust
use oxidowl::{Reasoner, ReasonerConfig, TableauAlgorithm};

let mut config = ReasonerConfig::default();
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;

let mut reasoner = Reasoner::new(config)?;
reasoner.load_ontology_from_file("ontology.owl", OntologyFormat::OwlXml)?;

let is_consistent = reasoner.is_consistent()?;
```

### With ReasoningService

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::Ontology,
};

let ontology = Ontology::new();
// ... add axioms ...

let mut config = ReasonerConfig::default();
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;

let service = ReasoningService::new(ontology, config);
let result = service.is_consistent().await?;
```

## Reasoning Operations

All standard reasoning operations work with both algorithms:

### Consistency Checking

```rust
let is_consistent = reasoner.is_consistent().await?;
```

Checks if the ontology contains logical contradictions.

### Satisfiability Checking

```rust
let class = ClassExpression::Class(IRI::new("http://example.org/Person"));
let is_satisfiable = reasoner.is_satisfiable(&class).await?;
```

Checks if a class expression can have instances.

### Subsumption Checking

```rust
let human = ClassExpression::Class(IRI::new("http://example.org/Human"));
let animal = ClassExpression::Class(IRI::new("http://example.org/Animal"));

let is_subclass = reasoner.is_subclass_of(&human, &animal).await?;
```

Checks if one class is a subclass of another.

### Classification

```rust
let classification = reasoner.classify().await?;
```

Computes the class hierarchy.

## Performance Characteristics

### Time Complexity

| Operation | Traditional | Hypertableau (Best Case) |
|-----------|-------------|--------------------------|
| Consistency (disjoint-heavy) | O(2^n) | O(n²) |
| Consistency (general) | O(2^n) | O(2^n) |
| Satisfiability | O(2^n) | O(2^n) |
| Classification | O(n² × 2^n) | O(n² × 2^n) |

### Space Complexity

| Algorithm | Space Usage |
|-----------|-------------|
| Traditional | O(n × d) where d = depth |
| Hypertableau | O(n) with structural sharing |

## Algorithm Selection Decision Tree

```
Has many disjoint classes (>10)?
├─ YES → Use Hypertableau (3-9x faster)
└─ NO → Continue

Has many equivalent classes (>10)?
├─ YES → Use Traditional (2-7x faster)
└─ NO → Continue

Has large linear taxonomy (>50 classes)?
├─ YES → Use Traditional (2-5x faster)
└─ NO → Continue

Has complex expressions (<50)?
├─ YES → Use Hypertableau (8-13% faster)
└─ NO → Use Traditional (safe default)
```

## Benchmarking API

### Running Benchmarks

```bash
# Run all benchmarks
cargo bench --bench hypertableau_benchmark

# Run specific scenario
cargo bench --bench hypertableau_benchmark -- linear_hierarchy

# Run with fewer samples (faster)
cargo bench --bench hypertableau_benchmark -- --quick
```

### Benchmark Scenarios

Available benchmark groups:

1. **linear_hierarchy** - Tests: 10, 50, 100, 200 classes
2. **tree_hierarchy** - Tests: Various depth/branching configurations
3. **complex_expressions** - Tests: 10, 20, 50, 100 axioms
4. **equivalent_classes** - Tests: 10, 50, 100 pairs
5. **disjoint_classes** - Tests: 10, 50, 100 pairs

### Interpreting Results

Criterion outputs:

- **time**: Mean execution time with confidence interval
- **Found X outliers**: Statistical outliers detected
- **Speedup**: Calculated from mean times

Example output:
```
disjoint_classes/traditional/50
                        time:   [1.2268 ms 1.2632 ms 1.3191 ms]

disjoint_classes/hypertableau/50
                        time:   [380.15 µs 390.12 µs 401.97 µs]

Speedup: 3.24x
```

## Error Handling

Both algorithms use the same error types:

```rust
use oxidowl::error::OxidOwlError;

match reasoner.is_consistent().await {
    Ok(result) => println!("Result: {}", result),
    Err(OxidOwlError::ReasoningError(msg)) => {
        eprintln!("Reasoning failed: {}", msg);
    }
    Err(e) => eprintln!("Other error: {}", e),
}
```

## Compatibility

### OWL 2 DL Support

Both algorithms support:

- Class expressions (intersection, union, complement)
- Property restrictions (existential, universal)
- Cardinality restrictions
- Nominals (oneOf)
- Datatypes
- Property chains
- Transitive properties
- Inverse properties
- Reflexive/Irreflexive properties
- Symmetric/Asymmetric properties
- Functional properties
- Keys

### Known Limitations

**Hypertableau specific:**

- Not optimized for many equivalent classes (>10)
- Higher overhead for very small ontologies (<10 classes)
- Slower for large linear taxonomies (>50 classes)

**Both algorithms:**

- Instance checking not yet optimized with hypertableau
- No incremental classification yet

## Feature Flags

If using specific features:

```toml
[dependencies]
oxidowl = { version = "0.3", features = ["hypertableau", "benchmarks"] }
```

Available features:
- `hypertableau` - Enable Hypertableau algorithm (enabled by default)
- `benchmarks` - Enable benchmark suite
- `explanations` - Enable explanation generation
- `server` - Enable server interfaces

## Migration Guide

### From Traditional to Hypertableau

**Before:**
```rust
let config = ReasonerConfig::default();
let reasoner = ReasoningService::new(ontology, config);
```

**After:**
```rust
let mut config = ReasonerConfig::default();
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
let reasoner = ReasoningService::new(ontology, config);
```

### Verifying Equivalence

```rust
// Compare results
let result_traditional = reasoner_traditional.is_consistent().await?;
let result_hypertableau = reasoner_hypertableau.is_consistent().await?;

assert_eq!(result_traditional, result_hypertableau, 
    "Algorithms should produce identical results");
```

## Debugging

### Enabling Verbose Output

```rust
use oxidowl::config::ReasonerConfig;

let mut config = ReasonerConfig::default();
config.logging.verbose = true;
config.logging.trace_reasoning = true;
```

### Performance Profiling

```bash
# Profile with cargo flamegraph
cargo flamegraph --bench hypertableau_benchmark

# Profile with perf
cargo bench --bench hypertableau_benchmark -- --profile-time=10
```

## See Also

- [Hypertableau Guide](HYPERTABLEAU_GUIDE.md) - Complete guide
- [Examples](HYPERTABLEAU_EXAMPLES.md) - Code examples
- [Performance Analysis](../PERFORMANCE_ANALYSIS.md) - Benchmark results
- [Main API Docs](https://docs.rs/oxidowl) - Full API documentation

# Hypertableau Algorithm Guide

## Table of Contents

- [Introduction](#introduction)
- [What is Hypertableau?](#what-is-hypertableau)
- [When to Use Hypertableau](#when-to-use-hypertableau)
- [Configuration](#configuration)
- [API Usage](#api-usage)
- [Performance Characteristics](#performance-characteristics)
- [Algorithm Details](#algorithm-details)
- [Troubleshooting](#troubleshooting)
- [References](#references)

## Introduction

OxidOwl provides two tableau-based reasoning algorithms:

1. **Traditional Tableau** (default) - Classic tableau expansion with blocking
2. **Hypertableau** - Optimized algorithm using hypergraph structures

This guide explains when and how to use the Hypertableau algorithm for improved reasoning performance.

## What is Hypertableau?

Hypertableau is an optimized tableau-based reasoning algorithm that uses **hypergraph structures** instead of tree structures for representing the reasoning state. This provides several advantages:

### Key Differences from Traditional Tableau

| Aspect | Traditional Tableau | Hypertableau |
|--------|-------------------|--------------|
| **Data Structure** | Tree of nodes | Hypergraph with shared nodes |
| **Node Sharing** | Minimal (blocking) | Extensive structural sharing |
| **Memory Usage** | Higher (duplicate nodes) | Lower (shared representations) |
| **Disjointness** | Exponential expansion | Linear/polynomial expansion |
| **Best For** | General purpose | Disjointness-heavy ontologies |

## When to Use Hypertableau

### Recommended For

**Disjointness-Heavy Ontologies** (3-9x speedup):
```turtle
:Cat owl:disjointWith :Dog .
:Bird owl:disjointWith :Mammal .
:Car owl:disjointWith :Person .
# Many disjoint class pairs
```

**Medium-Sized Ontologies with Complex Expressions** (8-13% speedup):
```turtle
:HappyPerson owl:equivalentClass [
    a owl:Class ;
    owl:intersectionOf (:Person :Happy)
] .
```

**Deep, Narrow Class Hierarchies** (5-15% speedup):
```turtle
:Animal
  ⊑ :Mammal
    ⊑ :Primate
      ⊑ :Human
        ⊑ :Adult
```

### Not Recommended For

**Many Equivalent Classes** (2-7x slower):
```turtle
:Person owl:equivalentClass :Human .
:Car owl:equivalentClass :Automobile .
:Company owl:equivalentClass :Organization .
# >10 equivalence pairs
```

**Large Linear Taxonomies** (2-5x slower):
```turtle
:A ⊑ :B ⊑ :C ⊑ :D ⊑ ... (>50 classes)
```

**Wide, Bushy Hierarchies** (1.6x slower):
```turtle
:Animal
  ⊑ :Mammal, :Bird, :Reptile, :Fish, :Amphibian, ...
    # High branching factor (>3)
```

### Use Either (Performance Similar)

- Very small ontologies (<10 classes)
- Simple hierarchies with 20-50 classes
- Balanced tree structures
- General-purpose ontologies without specific patterns

## Configuration

### Basic Configuration

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

// Create config with Hypertableau
let mut config = ReasonerConfig::default();
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
```

### Complete Example

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::Ontology,
};

// Load your ontology
let ontology = Ontology::new();
// ... add axioms ...

// Configure reasoning
let mut config = ReasonerConfig::default();
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;

// Create reasoning service
let reasoner = ReasoningService::new(ontology, config);

// Perform reasoning
let is_consistent = reasoner.is_consistent().await?;
```

### Configuration Options

```rust
pub struct ReasoningConfig {
    /// Tableau algorithm to use
    pub tableau_algorithm: TableauAlgorithm,
    
    /// Enable incremental reasoning (enabled by default)
    pub enable_incremental: bool,
    
    /// Other reasoning options...
}

pub enum TableauAlgorithm {
    /// Traditional tableau with blocking (default)
    Traditional,
    
    /// Hypertableau with hypergraph structures
    Hypertableau,
}
```

## API Usage

### Consistency Checking

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create ontology with disjoint classes
    let mut ontology = Ontology::new();
    
    let cat = ClassExpression::Class(IRI::new("http://example.org/Cat"));
    let dog = ClassExpression::Class(IRI::new("http://example.org/Dog"));
    
    ontology.add_axiom(Axiom::DisjointClasses(vec![cat.clone(), dog.clone()]));
    
    // Use Hypertableau for fast disjointness reasoning
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    // Check consistency (should be consistent)
    let consistent = reasoner.is_consistent().await?;
    println!("Ontology is consistent: {}", consistent);
    
    Ok(())
}
```

### Satisfiability Checking

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = Ontology::new();
    // ... add axioms ...
    
    // Configure with Hypertableau
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    // Check if a class is satisfiable
    let class = ClassExpression::Class(IRI::new("http://example.org/Person"));
    let satisfiable = reasoner.is_satisfiable(&class).await?;
    
    println!("Class is satisfiable: {}", satisfiable);
    
    Ok(())
}
```

### Subsumption Checking

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = Ontology::new();
    // ... add axioms ...
    
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    // Check if Human ⊑ Animal
    let human = ClassExpression::Class(IRI::new("http://example.org/Human"));
    let animal = ClassExpression::Class(IRI::new("http://example.org/Animal"));
    
    let is_subclass = reasoner.is_subclass_of(&human, &animal).await?;
    
    println!("Human ⊑ Animal: {}", is_subclass);
    
    Ok(())
}
```

### Algorithm Comparison

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::Ontology,
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = Ontology::new();
    // ... add axioms ...
    
    // Test Traditional
    let mut config_traditional = ReasonerConfig::default();
    config_traditional.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
    
    let reasoner_traditional = ReasoningService::new(ontology.clone(), config_traditional);
    
    let start = Instant::now();
    let result_traditional = reasoner_traditional.is_consistent().await?;
    let time_traditional = start.elapsed();
    
    // Test Hypertableau
    let mut config_hypertableau = ReasonerConfig::default();
    config_hypertableau.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner_hypertableau = ReasoningService::new(ontology.clone(), config_hypertableau);
    
    let start = Instant::now();
    let result_hypertableau = reasoner_hypertableau.is_consistent().await?;
    let time_hypertableau = start.elapsed();
    
    println!("Traditional: {:?} (result: {})", time_traditional, result_traditional);
    println!("Hypertableau: {:?} (result: {})", time_hypertableau, result_hypertableau);
    println!("Speedup: {:.2}x", time_traditional.as_secs_f64() / time_hypertableau.as_secs_f64());
    
    Ok(())
}
```

## Performance Characteristics

### Benchmark Results Summary

Based on comprehensive benchmarks with 34 test cases across 5 scenarios:

#### Scenario 1: Linear Hierarchy

| Size | Traditional | Hypertableau | Speedup |
|------|-------------|--------------|---------|
| 10   | 363 µs      | 343 µs       | 1.06x   |
| 50   | 482 µs      | 482 µs       | 1.00x   |
| 100  | 677 µs      | 1,460 µs     | 0.46x |
| 200  | 1,146 µs    | 6,251 µs     | 0.18x |

**Recommendation:** Use Traditional for linear taxonomies >50 classes

#### Scenario 2: Tree Hierarchy

| Configuration | Traditional | Hypertableau | Speedup |
|---------------|-------------|--------------|---------|
| d=3, b=3      | 432 µs      | 410 µs       | 1.05x |
| d=4, b=2      | 427 µs      | 371 µs       | 1.15x |
| d=3, b=4      | 620 µs      | 993 µs       | 0.62x |

**Recommendation:** Use Hypertableau for deep, narrow trees (branching <3)

#### Scenario 3: Complex Expressions

| Size | Traditional | Hypertableau | Speedup |
|------|-------------|--------------|---------|
| 10   | 369 µs      | 326 µs       | 1.13x |
| 20   | 404 µs      | 359 µs       | 1.13x |
| 50   | 502 µs      | 464 µs       | 1.08x |
| 100  | 754 µs      | 1,392 µs     | 0.54x |

**Recommendation:** Use Hypertableau for <50 complex expressions

#### Scenario 4: Equivalent Classes

| Size | Traditional | Hypertableau | Speedup |
|------|-------------|--------------|---------|
| 10   | 400 µs      | 359 µs       | 1.12x |
| 50   | 690 µs      | 1,401 µs     | 0.49x |
| 100  | 1,213 µs    | 8,894 µs     | 0.14x |

**Recommendation:** Use Traditional for many equivalent classes (>10)

#### Scenario 5: Disjoint Classes ⭐

| Size | Traditional | Hypertableau | Speedup |
|------|-------------|--------------|---------|
| 10   | 393 µs      | 326 µs       | 1.21x |
| 50   | 1,263 µs    | 390 µs       | 3.24x |
| 100  | 3,831 µs    | 441 µs       | 8.68x |

**Recommendation:** **Always use Hypertableau** for disjointness-heavy ontologies!

### Running Benchmarks

Profile your specific ontology:

```bash
cargo bench --bench hypertableau_benchmark
```

View results:
```bash
open target/criterion/report/index.html
```

## Algorithm Details

### How Hypertableau Works

1. **Hypergraph Construction**
   - Ontology axioms → Hypergraph nodes and edges
   - Classes → Nodes
   - Relationships → Hyperedges
   - Structural sharing from the start

2. **Expansion Phase**
   - Apply tableau expansion rules
   - Reuse existing nodes (avoiding duplication)
   - Track conflicts efficiently
   - Propagate constraints through hyperedges

3. **Clash Detection**
   - Check for logical contradictions
   - Disjointness handled efficiently via hyperedges
   - Early termination on clash

4. **Result Extraction**
   - Hypergraph state → Reasoning result
   - No clash = Satisfiable/Consistent
   - Clash found = Unsatisfiable/Inconsistent

### Key Data Structures

#### Hypergraph
```rust
pub struct Hypergraph {
    nodes: HashMap<NodeId, Node>,
    edges: Vec<Hyperedge>,
    root: NodeId,
}
```

#### Node
```rust
pub struct Node {
    id: NodeId,
    concepts: HashSet<Concept>,
    blocked_by: Option<NodeId>,
    successors: Vec<(Role, NodeId)>,
}
```

#### Hyperedge
```rust
pub struct Hyperedge {
    source: NodeId,
    targets: Vec<NodeId>,
    label: EdgeLabel,
}
```

### Expansion Rules

Hypertableau implements standard tableau expansion rules:

- **⊓-Rule**: `A ⊓ B` → `A`, `B`
- **⊔-Rule**: `A ⊔ B` → `A` or `B` (branching)
- **∃-Rule**: `∃R.C` → Create successor with `C`
- **∀-Rule**: `∀R.C` → Propagate to all R-successors
- **Clash Detection**: `A ⊓ ¬A` → Contradiction

The key difference is **structural sharing** - nodes are reused across branches.

## Troubleshooting

### Issue: Hypertableau is Slower

**Symptom:** Reasoning with Hypertableau takes longer than Traditional.

**Diagnosis:**
```rust
// Profile your ontology
cargo bench --bench hypertableau_benchmark -- your_test
```

**Common Causes:**
1. **Many equivalent classes** (>10 pairs) - Use Traditional
2. **Large linear taxonomy** (>50 classes) - Use Traditional
3. **Wide hierarchies** (branching factor >3) - Use Traditional
4. **Very small ontology** (<10 classes) - Overhead not worth it

**Solution:** Switch back to Traditional:
```rust
config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
```

### Issue: Different Results from Traditional

**Symptom:** Hypertableau gives different reasoning results.

**Diagnosis:**
```rust
// Compare both algorithms
let result_traditional = reasoner_traditional.is_consistent().await?;
let result_hypertableau = reasoner_hypertableau.is_consistent().await?;
assert_eq!(result_traditional, result_hypertableau);
```

**This Should Not Happen:** Both algorithms are logically equivalent and pass 9/9 integration tests.

**If it does:**
1. Check for non-deterministic elements (e.g., hash map iteration order)
2. Report as bug with minimal reproduction case
3. Use Traditional until fixed

### Issue: High Memory Usage

**Symptom:** Memory usage increases significantly with Hypertableau.

**Expected:** Hypertableau should use **less** memory due to structural sharing.

**If memory is high:**
1. Check for memory leaks (profile with `valgrind` or `heaptrack`)
2. Verify incremental reasoning is not accumulating state
3. Consider disabling incremental caching:
   ```rust
   config.reasoning.enable_incremental = false;
   ```

### Issue: Unexpected Performance

**Symptom:** Performance doesn't match benchmarks.

**Factors Affecting Performance:**
- Ontology size and structure
- Axiom complexity
- Hardware (CPU, memory)
- Parallelism (currently single-threaded)
- Caching (incremental reasoning enabled by default)

**Best Practices:**
1. **Profile your specific ontology** - Benchmarks are synthetic
2. **Test both algorithms** - Choose based on your data
3. **Monitor performance** - Set up regression testing
4. **Consider hybrid approach** - Use different algorithms for different tasks

## References

### Papers

1. **Motik, B., Shearer, R., & Horrocks, I. (2009)**  
   "Hypertableau reasoning for description logics"  
   *Journal of Artificial Intelligence Research*, 36, 165-228.

2. **Kazakov, Y. (2009)**  
   "Consequence-driven reasoning for Horn SHIQ ontologies"  
   *Proceedings of IJCAI 2009*

### OxidOwl Documentation

- [Main README](../README.md)
- [Performance Analysis](../PERFORMANCE_ANALYSIS.md)
- [Verification Report](../HYPERTABLEAU_VERIFICATION.md)
- [API Documentation](https://docs.rs/oxidowl)

### Source Code

- [Hypergraph Implementation](../src/core/tableau/hypergraph.rs)
- [Expansion Algorithm](../src/core/tableau/expansion.rs)
- [Hypertableau Runner](../src/core/tableau/hypertableau_runner.rs)
- [Adapter](../src/core/tableau/hypertableau_adapter.rs)
- [Benchmarks](../benches/hypertableau_benchmark.rs)

### Community

- **Issues:** [GitHub Issues](https://github.com/sievericcardo/oxidowl/issues)
- **Discussions:** [GitHub Discussions](https://github.com/sievericcardo/oxidowl/discussions)
- **Contributing:** [CONTRIBUTING.md](../CONTRIBUTING.md)

---

## Quick Reference

### When to Use Each Algorithm

| Ontology Characteristic | Algorithm | Speedup |
|------------------------|-----------|---------|
| Many disjoint classes (>10) | **Hypertableau** | 3-9x faster |
| Complex expressions (<50) | **Hypertableau** | 8-13% faster |
| Deep, narrow hierarchies | **Hypertableau** | 5-15% faster |
| Large linear taxonomy (>50) | **Traditional** | 2-5x faster |
| Many equivalent classes (>10) | **Traditional** | 2-7x faster |
| Wide hierarchies (branching >3) | **Traditional** | 1.6x faster |
| General purpose / unknown | **Traditional** | Safer default |

### Configuration Snippet

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

let mut config = ReasonerConfig::default();

// For disjointness-heavy ontologies
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;

// For general-purpose / safe default
config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
```

### Benchmark Command

```bash
cargo bench --bench hypertableau_benchmark
```


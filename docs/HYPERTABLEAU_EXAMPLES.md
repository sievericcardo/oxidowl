# Hypertableau Algorithm Examples

This file contains practical examples of using the Hypertableau algorithm in various scenarios.

## Table of Contents

- [Basic Configuration](#basic-configuration)
- [Disjointness Reasoning](#disjointness-reasoning)
- [Complex Class Expressions](#complex-class-expressions)
- [Algorithm Comparison](#algorithm-comparison)
- [Performance Profiling](#performance-profiling)
- [Real-World Scenarios](#real-world-scenarios)

## Basic Configuration

### Example 1: Enabling Hypertableau

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::Ontology,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create ontology
    let ontology = Ontology::new();
    
    // Enable Hypertableau algorithm
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    // Create reasoning service
    let reasoner = ReasoningService::new(ontology, config);
    
    // Perform reasoning
    let result = reasoner.is_consistent().await?;
    println!("Ontology is consistent: {}", result);
    
    Ok(())
}
```

### Example 2: Switching Between Algorithms

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::Ontology,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = Ontology::new();
    // ... populate ontology ...
    
    // Try with Traditional
    let mut config_traditional = ReasonerConfig::default();
    config_traditional.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
    let reasoner_traditional = ReasoningService::new(ontology.clone(), config_traditional);
    
    // Try with Hypertableau
    let mut config_hypertableau = ReasonerConfig::default();
    config_hypertableau.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    let reasoner_hypertableau = ReasoningService::new(ontology.clone(), config_hypertableau);
    
    // Compare results (should be identical)
    let result_traditional = reasoner_traditional.is_consistent().await?;
    let result_hypertableau = reasoner_hypertableau.is_consistent().await?;
    
    assert_eq!(result_traditional, result_hypertableau);
    println!("Both algorithms agree: {}", result_traditional);
    
    Ok(())
}
```

## Disjointness Reasoning

### Example 3: Animal Taxonomy with Disjointness

This is where Hypertableau excels (3-9x speedup).

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ontology = Ontology::new();
    
    // Define animal classes
    let mammal = ClassExpression::Class(IRI::new("http://example.org/Mammal"));
    let bird = ClassExpression::Class(IRI::new("http://example.org/Bird"));
    let reptile = ClassExpression::Class(IRI::new("http://example.org/Reptile"));
    let fish = ClassExpression::Class(IRI::new("http://example.org/Fish"));
    let amphibian = ClassExpression::Class(IRI::new("http://example.org/Amphibian"));
    
    // Add disjointness axioms (where Hypertableau excels)
    ontology.add_axiom(Axiom::DisjointClasses(vec![
        mammal.clone(),
        bird.clone(),
        reptile.clone(),
        fish.clone(),
        amphibian.clone(),
    ]));
    
    // Define specific animals
    let cat = ClassExpression::Class(IRI::new("http://example.org/Cat"));
    let dog = ClassExpression::Class(IRI::new("http://example.org/Dog"));
    let eagle = ClassExpression::Class(IRI::new("http://example.org/Eagle"));
    
    // Cat and Dog are both Mammals (and thus disjoint from birds, etc.)
    ontology.add_axiom(Axiom::SubClassOf {
        sub: cat.clone(),
        super_: mammal.clone(),
    });
    ontology.add_axiom(Axiom::SubClassOf {
        sub: dog.clone(),
        super_: mammal.clone(),
    });
    
    // Cat and Dog are disjoint
    ontology.add_axiom(Axiom::DisjointClasses(vec![cat.clone(), dog.clone()]));
    
    // Eagle is a Bird
    ontology.add_axiom(Axiom::SubClassOf {
        sub: eagle.clone(),
        super_: bird.clone(),
    });
    
    // Use Hypertableau for fast reasoning
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    // Check consistency
    let consistent = reasoner.is_consistent().await?;
    println!("Animal taxonomy is consistent: {}", consistent);
    
    // Check if something is both a Cat and a Dog (should be unsatisfiable)
    let cat_and_dog = ClassExpression::Intersection(vec![cat.clone(), dog.clone()]);
    let satisfiable = reasoner.is_satisfiable(&cat_and_dog).await?;
    println!("Cat AND Dog is satisfiable: {} (should be false)", satisfiable);
    
    Ok(())
}
```

### Example 4: Vehicle Classification

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ontology = Ontology::new();
    
    // Vehicle types
    let car = ClassExpression::Class(IRI::new("http://example.org/Car"));
    let truck = ClassExpression::Class(IRI::new("http://example.org/Truck"));
    let motorcycle = ClassExpression::Class(IRI::new("http://example.org/Motorcycle"));
    let bicycle = ClassExpression::Class(IRI::new("http://example.org/Bicycle"));
    
    // All vehicles are pairwise disjoint
    ontology.add_axiom(Axiom::DisjointClasses(vec![
        car.clone(),
        truck.clone(),
        motorcycle.clone(),
        bicycle.clone(),
    ]));
    
    // Motorized vs Non-motorized (also disjoint)
    let motorized = ClassExpression::Class(IRI::new("http://example.org/Motorized"));
    let non_motorized = ClassExpression::Class(IRI::new("http://example.org/NonMotorized"));
    
    ontology.add_axiom(Axiom::DisjointClasses(vec![
        motorized.clone(),
        non_motorized.clone(),
    ]));
    
    // Cars, trucks, and motorcycles are motorized
    ontology.add_axiom(Axiom::SubClassOf {
        sub: car.clone(),
        super_: motorized.clone(),
    });
    ontology.add_axiom(Axiom::SubClassOf {
        sub: truck.clone(),
        super_: motorized.clone(),
    });
    ontology.add_axiom(Axiom::SubClassOf {
        sub: motorcycle.clone(),
        super_: motorized.clone(),
    });
    
    // Bicycles are non-motorized
    ontology.add_axiom(Axiom::SubClassOf {
        sub: bicycle.clone(),
        super_: non_motorized.clone(),
    });
    
    // Use Hypertableau (excellent for disjointness)
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    println!("Checking vehicle taxonomy with Hypertableau...");
    let consistent = reasoner.is_consistent().await?;
    println!("Taxonomy is consistent: {}", consistent);
    
    Ok(())
}
```

## Complex Class Expressions

### Example 5: Intersection and Union Reasoning

Hypertableau shows 8-13% improvement for small-medium complex expressions.

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ontology = Ontology::new();
    
    // Basic classes
    let person = ClassExpression::Class(IRI::new("http://example.org/Person"));
    let happy = ClassExpression::Class(IRI::new("http://example.org/Happy"));
    let employed = ClassExpression::Class(IRI::new("http://example.org/Employed"));
    let parent = ClassExpression::Class(IRI::new("http://example.org/Parent"));
    
    // Complex expressions using intersection
    let happy_person = ClassExpression::Intersection(vec![
        person.clone(),
        happy.clone(),
    ]);
    
    let employed_parent = ClassExpression::Intersection(vec![
        employed.clone(),
        parent.clone(),
    ]);
    
    // Union expression
    let happy_or_employed = ClassExpression::Union(vec![
        happy.clone(),
        employed.clone(),
    ]);
    
    // Add axioms
    let satisfied_person = ClassExpression::Class(IRI::new("http://example.org/SatisfiedPerson"));
    
    // SatisfiedPerson ≡ Person ⊓ (Happy ⊔ Employed)
    ontology.add_axiom(Axiom::EquivalentClasses(vec![
        satisfied_person.clone(),
        ClassExpression::Intersection(vec![
            person.clone(),
            happy_or_employed,
        ]),
    ]));
    
    // Working parent: Person ⊓ Employed ⊓ Parent
    let working_parent = ClassExpression::Class(IRI::new("http://example.org/WorkingParent"));
    ontology.add_axiom(Axiom::EquivalentClasses(vec![
        working_parent.clone(),
        ClassExpression::Intersection(vec![
            person.clone(),
            employed_parent,
        ]),
    ]));
    
    // Use Hypertableau for complex expressions
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    println!("Reasoning with complex class expressions...");
    let consistent = reasoner.is_consistent().await?;
    println!("Ontology is consistent: {}", consistent);
    
    // Check if WorkingParent ⊑ SatisfiedPerson
    let is_subclass = reasoner.is_subclass_of(&working_parent, &satisfied_person).await?;
    println!("WorkingParent ⊑ SatisfiedPerson: {}", is_subclass);
    
    Ok(())
}
```

## Algorithm Comparison

### Example 6: Benchmark Both Algorithms

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression, IRI},
};
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a disjointness-heavy ontology
    let mut ontology = Ontology::new();
    
    // Add 50 pairwise disjoint classes
    let mut classes = Vec::new();
    for i in 0..50 {
        let class = ClassExpression::Class(
            IRI::new(&format!("http://example.org/Class{}", i))
        );
        classes.push(class);
    }
    
    ontology.add_axiom(Axiom::DisjointClasses(classes));
    
    println!("Created ontology with 50 disjoint classes");
    
    // Benchmark Traditional
    let mut config_traditional = ReasonerConfig::default();
    config_traditional.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
    
    let reasoner_traditional = ReasoningService::new(ontology.clone(), config_traditional);
    
    let start = Instant::now();
    let result_traditional = reasoner_traditional.is_consistent().await?;
    let duration_traditional = start.elapsed();
    
    println!("\nTraditional Algorithm:");
    println!("  Result: {}", result_traditional);
    println!("  Time: {:?}", duration_traditional);
    
    // Benchmark Hypertableau
    let mut config_hypertableau = ReasonerConfig::default();
    config_hypertableau.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner_hypertableau = ReasoningService::new(ontology.clone(), config_hypertableau);
    
    let start = Instant::now();
    let result_hypertableau = reasoner_hypertableau.is_consistent().await?;
    let duration_hypertableau = start.elapsed();
    
    println!("\nHypertableau Algorithm:");
    println!("  Result: {}", result_hypertableau);
    println!("  Time: {:?}", duration_hypertableau);
    
    // Calculate speedup
    let speedup = duration_traditional.as_secs_f64() / duration_hypertableau.as_secs_f64();
    println!("\nSpeedup: {:.2}x", speedup);
    println!("Expected: 3-9x for 50 disjoint classes");
    
    Ok(())
}
```

## Performance Profiling

### Example 7: Adaptive Algorithm Selection

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression},
};

/// Analyze ontology and recommend best algorithm
fn recommend_algorithm(ontology: &Ontology) -> TableauAlgorithm {
    let mut disjoint_count = 0;
    let mut equivalent_count = 0;
    let mut subclass_count = 0;
    
    // Analyze axioms
    for axiom in ontology.get_axioms() {
        match axiom {
            Axiom::DisjointClasses(classes) => {
                disjoint_count += classes.len();
            }
            Axiom::EquivalentClasses(classes) => {
                equivalent_count += classes.len();
            }
            Axiom::SubClassOf { .. } => {
                subclass_count += 1;
            }
            _ => {}
        }
    }
    
    println!("Ontology analysis:");
    println!("  Disjoint class declarations: {}", disjoint_count);
    println!("  Equivalent class declarations: {}", equivalent_count);
    println!("  Subclass axioms: {}", subclass_count);
    
    // Decision logic
    if disjoint_count > 10 {
        println!("Recommendation: Hypertableau (many disjoint classes)");
        TableauAlgorithm::Hypertableau
    } else if equivalent_count > 10 {
        println!("Recommendation: Traditional (many equivalent classes)");
        TableauAlgorithm::Traditional
    } else if subclass_count > 50 {
        println!("Recommendation: Traditional (large linear taxonomy)");
        TableauAlgorithm::Traditional
    } else {
        println!("Recommendation: Traditional (general purpose / safe default)");
        TableauAlgorithm::Traditional
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = Ontology::new();
    // ... load ontology ...
    
    // Get recommendation
    let algorithm = recommend_algorithm(&ontology);
    
    // Configure with recommended algorithm
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = algorithm;
    
    let reasoner = ReasoningService::new(ontology, config);
    let result = reasoner.is_consistent().await?;
    
    println!("\nReasoning result: {}", result);
    
    Ok(())
}
```

## Real-World Scenarios

### Example 8: Medical Ontology

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::{Ontology, Axiom, ClassExpression, IRI},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut ontology = Ontology::new();
    
    // Disease categories (disjoint)
    let infectious = ClassExpression::Class(IRI::new("http://medical.org/InfectiousDisease"));
    let genetic = ClassExpression::Class(IRI::new("http://medical.org/GeneticDisease"));
    let autoimmune = ClassExpression::Class(IRI::new("http://medical.org/AutoimmuneDisease"));
    let cancer = ClassExpression::Class(IRI::new("http://medical.org/Cancer"));
    
    ontology.add_axiom(Axiom::DisjointClasses(vec![
        infectious.clone(),
        genetic.clone(),
        autoimmune.clone(),
        cancer.clone(),
    ]));
    
    // Specific diseases
    let covid = ClassExpression::Class(IRI::new("http://medical.org/COVID19"));
    let diabetes_t1 = ClassExpression::Class(IRI::new("http://medical.org/DiabetesType1"));
    let lupus = ClassExpression::Class(IRI::new("http://medical.org/Lupus"));
    
    ontology.add_axiom(Axiom::SubClassOf {
        sub: covid.clone(),
        super_: infectious.clone(),
    });
    
    // Type 1 Diabetes is both genetic AND autoimmune
    ontology.add_axiom(Axiom::SubClassOf {
        sub: diabetes_t1.clone(),
        super_: ClassExpression::Intersection(vec![
            genetic.clone(),
            autoimmune.clone(),
        ]),
    });
    
    ontology.add_axiom(Axiom::SubClassOf {
        sub: lupus.clone(),
        super_: autoimmune.clone(),
    });
    
    // Use Hypertableau for disjointness reasoning
    let mut config = ReasonerConfig::default();
    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner = ReasoningService::new(ontology, config);
    
    println!("Medical ontology reasoning with Hypertableau...");
    let consistent = reasoner.is_consistent().await?;
    println!("Medical ontology is consistent: {}", consistent);
    
    Ok(())
}
```

### Example 9: Loading OWL File and Choosing Algorithm

```rust
use oxidowl::{
    Reasoner, ReasonerConfig, OntologyFormat, TableauAlgorithm,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load ontology from file
    let mut config = ReasonerConfig::default();
    let mut reasoner = Reasoner::new(config.clone())?;
    
    reasoner.load_ontology_from_file("my_ontology.owl", OntologyFormat::OwlXml)?;
    
    // Analyze ontology characteristics
    let ontology = reasoner.get_ontology()?;
    let ontology_data = ontology.read().unwrap();
    
    let has_many_disjoint = ontology_data
        .get_axioms()
        .iter()
        .filter(|a| matches!(a, oxidowl::ontology::Axiom::DisjointClasses(_)))
        .count() > 10;
    
    // Reconfigure if needed
    if has_many_disjoint {
        println!("Detected disjointness-heavy ontology, switching to Hypertableau");
        config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
        
        // Recreate reasoner with new config
        reasoner = Reasoner::new(config)?;
        reasoner.load_ontology_from_file("my_ontology.owl", OntologyFormat::OwlXml)?;
    }
    
    // Perform reasoning
    let is_consistent = reasoner.is_consistent()?;
    println!("Ontology is consistent: {}", is_consistent);
    
    Ok(())
}
```

### Example 10: Integration with Existing Code

```rust
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    reasoning::ReasoningService,
    ontology::Ontology,
};

/// Wrapper function that tries Hypertableau first, falls back to Traditional
async fn robust_consistency_check(
    ontology: Ontology,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Try Hypertableau first (might be faster)
    let mut config_hyper = ReasonerConfig::default();
    config_hyper.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
    
    let reasoner_hyper = ReasoningService::new(ontology.clone(), config_hyper);
    
    match reasoner_hyper.is_consistent().await {
        Ok(result) => {
            println!("Hypertableau succeeded");
            Ok(result)
        }
        Err(e) => {
            println!("Hypertableau failed: {}, trying Traditional", e);
            
            // Fall back to Traditional
            let mut config_trad = ReasonerConfig::default();
            config_trad.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
            
            let reasoner_trad = ReasoningService::new(ontology, config_trad);
            reasoner_trad.is_consistent().await.map_err(|e| e.into())
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = Ontology::new();
    // ... populate ontology ...
    
    let result = robust_consistency_check(ontology).await?;
    println!("Ontology is consistent: {}", result);
    
    Ok(())
}
```

## Running the Examples

Save any example to a file (e.g., `example.rs`) and run with:

```bash
cargo run --example example
```

Or include in your project's `examples/` directory:

```bash
cargo run --example disjointness_reasoning
```

## Benchmarking Your Ontology

To benchmark your specific ontology:

```bash
# Run the comprehensive benchmark suite
cargo bench --bench hypertableau_benchmark

# View results
open target/criterion/report/index.html
```

## Further Reading

- [Hypertableau Guide](HYPERTABLEAU_GUIDE.md) - Complete configuration guide
- [Performance Analysis](../PERFORMANCE_ANALYSIS.md) - Detailed benchmark results
- [Verification Report](../HYPERTABLEAU_VERIFICATION.md) - Implementation verification
- [Main README](../README.md) - Project overview

---

*Last updated: January 2025*

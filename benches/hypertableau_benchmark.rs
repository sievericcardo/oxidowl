//! Performance benchmarks comparing Traditional vs Hypertableau tableau algorithms
//!
//! This benchmark suite measures:
//! - Execution time for consistency checking
//! - Performance across different ontology sizes
//! - Behavior with varying axiom complexity
//! - Memory usage patterns

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use oxidowl::{
    config::{ReasonerConfig, TableauAlgorithm},
    ontology::{
        Class, ClassExpression, IRI, Ontology,
        axioms::{Axiom, DisjointClassesAxiom, EquivalentClassesAxiom, SubClassOfAxiom},
    },
    reasoning::ReasoningService,
};

/// Create a simple ontology with a linear class hierarchy
/// A ⊑ B ⊑ C ⊑ ... ⊑ Z
fn create_linear_hierarchy(size: usize) -> Ontology {
    let mut ontology = Ontology::new();

    for i in 0..size {
        let subclass = Class {
            iri: IRI::new(&format!("http://example.org/Class{}", i)),
        };
        let superclass = Class {
            iri: IRI::new(&format!("http://example.org/Class{}", i + 1)),
        };

        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: i as u64,
            subclass: ClassExpression::Class(subclass),
            superclass: ClassExpression::Class(superclass),
            annotations: vec![],
        }));
    }

    ontology
}

/// Create an ontology with a tree-like hierarchy
/// Root class with `branching_factor` children, each with `branching_factor` children, etc.
fn create_tree_hierarchy(depth: usize, branching_factor: usize) -> Ontology {
    let mut ontology = Ontology::new();
    let mut id = 0u64;

    fn add_tree_level(
        ontology: &mut Ontology,
        id: &mut u64,
        parent_class: Class,
        depth: usize,
        branching_factor: usize,
    ) {
        if depth == 0 {
            return;
        }

        for i in 0..branching_factor {
            let child_iri = format!("{}_{}", parent_class.iri.as_str(), i);
            let child = Class {
                iri: IRI::new(&child_iri),
            };

            ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                id: *id,
                subclass: ClassExpression::Class(child.clone()),
                superclass: ClassExpression::Class(parent_class.clone()),
                annotations: vec![],
            }));
            *id += 1;

            add_tree_level(ontology, id, child, depth - 1, branching_factor);
        }
    }

    let root = Class {
        iri: IRI::new("http://example.org/Root"),
    };

    add_tree_level(&mut ontology, &mut id, root, depth, branching_factor);
    ontology
}

/// Create an ontology with complex class expressions using intersections and unions
fn create_complex_expressions(size: usize) -> Ontology {
    let mut ontology = Ontology::new();

    for i in 0..(size / 2) {
        let class_a = Class {
            iri: IRI::new(&format!("http://example.org/A{}", i)),
        };
        let class_b = Class {
            iri: IRI::new(&format!("http://example.org/B{}", i)),
        };
        let class_c = Class {
            iri: IRI::new(&format!("http://example.org/C{}", i)),
        };

        // Create: C ⊑ A ⊓ B (intersection)
        let intersection = ClassExpression::ObjectIntersectionOf(vec![
            ClassExpression::Class(class_a.clone()),
            ClassExpression::Class(class_b.clone()),
        ]);

        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: (i * 2) as u64,
            subclass: ClassExpression::Class(class_c.clone()),
            superclass: intersection,
            annotations: vec![],
        }));

        // Create: A ⊔ B ⊑ D (union)
        let class_d = Class {
            iri: IRI::new(&format!("http://example.org/D{}", i)),
        };
        let union = ClassExpression::ObjectUnionOf(vec![
            ClassExpression::Class(class_a),
            ClassExpression::Class(class_b),
        ]);

        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: (i * 2 + 1) as u64,
            subclass: union,
            superclass: ClassExpression::Class(class_d),
            annotations: vec![],
        }));
    }

    ontology
}

/// Create an ontology with equivalent classes
fn create_equivalent_classes(size: usize) -> Ontology {
    let mut ontology = Ontology::new();

    for i in 0..size {
        let class_a = Class {
            iri: IRI::new(&format!("http://example.org/A{}", i)),
        };
        let class_b = Class {
            iri: IRI::new(&format!("http://example.org/B{}", i)),
        };

        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: i as u64,
            classes: vec![
                ClassExpression::Class(class_a),
                ClassExpression::Class(class_b),
            ],
            annotations: vec![],
        }));
    }

    ontology
}

/// Create an ontology with disjoint classes
fn create_disjoint_classes(size: usize) -> Ontology {
    let mut ontology = Ontology::new();

    for i in 0..size {
        let class_a = Class {
            iri: IRI::new(&format!("http://example.org/A{}", i)),
        };
        let class_b = Class {
            iri: IRI::new(&format!("http://example.org/B{}", i)),
        };

        ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
            id: i as u64,
            classes: vec![
                ClassExpression::Class(class_a),
                ClassExpression::Class(class_b),
            ],
            annotations: vec![],
        }));
    }

    ontology
}

/// Benchmark consistency checking with linear hierarchies
fn bench_linear_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_hierarchy");

    for size in [10, 50, 100, 200].iter() {
        let ontology = create_linear_hierarchy(*size);

        group.bench_with_input(BenchmarkId::new("traditional", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("hypertableau", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });
    }

    group.finish();
}

/// Benchmark consistency checking with tree hierarchies
fn bench_tree_hierarchy(c: &mut Criterion) {
    let mut group = c.benchmark_group("tree_hierarchy");

    // Test different tree configurations: (depth, branching_factor)
    for (depth, branching) in [(3, 3), (4, 2), (3, 4)].iter() {
        let ontology = create_tree_hierarchy(*depth, *branching);
        let label = format!("d{}_b{}", depth, branching);

        group.bench_with_input(
            BenchmarkId::new("traditional", &label),
            &label,
            |b, _label| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new()
                        .expect("Failed to create tokio runtime for benchmark");
                    rt.block_on(async {
                        let mut config = ReasonerConfig::default();
                        config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
                        let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                        let result = reasoner.is_consistent().await;
                        black_box(result)
                    })
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("hypertableau", &label),
            &label,
            |b, _label| {
                b.iter(|| {
                    let rt = tokio::runtime::Runtime::new()
                        .expect("Failed to create tokio runtime for benchmark");
                    rt.block_on(async {
                        let mut config = ReasonerConfig::default();
                        config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
                        let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                        let result = reasoner.is_consistent().await;
                        black_box(result)
                    })
                });
            },
        );
    }

    group.finish();
}

/// Benchmark consistency checking with complex expressions
fn bench_complex_expressions(c: &mut Criterion) {
    let mut group = c.benchmark_group("complex_expressions");

    for size in [10, 20, 50, 100].iter() {
        let ontology = create_complex_expressions(*size);

        group.bench_with_input(BenchmarkId::new("traditional", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("hypertableau", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });
    }

    group.finish();
}

/// Benchmark consistency checking with equivalent classes
fn bench_equivalent_classes(c: &mut Criterion) {
    let mut group = c.benchmark_group("equivalent_classes");

    for size in [10, 50, 100].iter() {
        let ontology = create_equivalent_classes(*size);

        group.bench_with_input(BenchmarkId::new("traditional", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("hypertableau", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });
    }

    group.finish();
}

/// Benchmark consistency checking with disjoint classes
fn bench_disjoint_classes(c: &mut Criterion) {
    let mut group = c.benchmark_group("disjoint_classes");

    for size in [10, 50, 100].iter() {
        let ontology = create_disjoint_classes(*size);

        group.bench_with_input(BenchmarkId::new("traditional", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });

        group.bench_with_input(BenchmarkId::new("hypertableau", size), size, |b, _size| {
            b.iter(|| {
                let rt = tokio::runtime::Runtime::new()
                    .expect("Failed to create tokio runtime for benchmark");
                rt.block_on(async {
                    let mut config = ReasonerConfig::default();
                    config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau;
                    let reasoner = ReasoningService::new(ontology.clone(), config)
                        .expect("Failed to create reasoning service");
                    let result = reasoner.is_consistent().await;
                    black_box(result)
                })
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_linear_hierarchy,
    bench_tree_hierarchy,
    bench_complex_expressions,
    bench_equivalent_classes,
    bench_disjoint_classes,
);
criterion_main!(benches);

use criterion::{Criterion, criterion_group, criterion_main};
use oxidowl::{
    Ontology, ReasonerConfig, ReasoningService,
    ontology::{Class, ClassExpression, IRI},
};
use std::hint::black_box;

fn satisfiability_benchmark(c: &mut Criterion) {
    let config = ReasonerConfig::default();
    let ontology = Ontology::new();
    let reasoning_service = ReasoningService::new(ontology, config)
        .expect("Failed to create reasoning service");

    c.bench_function("simple_concept_satisfiability", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            rt.block_on(async {
                let class = Class::new(IRI::new("http://example.org/A"));
                let concept = ClassExpression::Class(class);
                let result = reasoning_service.is_satisfiable(&concept).await;
                black_box(result)
            })
        });
    });

    c.bench_function("complex_concept_satisfiability", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            rt.block_on(async {
                let class_a = Class::new(IRI::new("http://example.org/A"));
                let class_b = Class::new(IRI::new("http://example.org/B"));
                let class_c = Class::new(IRI::new("http://example.org/C"));

                let concept_a = ClassExpression::Class(class_a);
                let concept_b = ClassExpression::Class(class_b);
                let concept_c = ClassExpression::Class(class_c);

                // Create a complex concept: A ∩ B ∪ ¬C
                let intersection = ClassExpression::intersection_of(vec![concept_a, concept_b]);
                let negation = ClassExpression::complement_of(concept_c);
                let complex_concept = ClassExpression::union_of(vec![intersection, negation]);

                let result = reasoning_service.is_satisfiable(&complex_concept).await;
                black_box(result)
            })
        });
    });
}

fn consistency_benchmark(c: &mut Criterion) {
    c.bench_function("empty_ontology_consistency", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            rt.block_on(async {
                let config = ReasonerConfig::default();
                let ontology = Ontology::new();
                let reasoning_service = ReasoningService::new(ontology, config)
                    .expect("Failed to create reasoning service");
                let result = reasoning_service.is_consistent().await;
                black_box(result)
            })
        });
    });

    c.bench_function("simple_ontology_consistency", |b| {
        b.iter(|| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            rt.block_on(async {
                let config = ReasonerConfig::default();
                let mut ontology = Ontology::new();

                // Add some simple axioms
                let person = Class::new(IRI::new("http://example.org/Person"));
                let animal = Class::new(IRI::new("http://example.org/Animal"));

                // Person ⊑ Animal - create SubClassOfAxiom
                use oxidowl::ontology::axioms::{Axiom, SubClassOfAxiom};
                let subclass_axiom = SubClassOfAxiom {
                    id: 1,
                    subclass: ClassExpression::Class(person),
                    superclass: ClassExpression::Class(animal),
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::SubClassOf(subclass_axiom));

                let reasoning_service = ReasoningService::new(ontology, config)
                    .expect("Failed to create reasoning service");
                let result = reasoning_service.is_consistent().await;
                black_box(result)
            })
        });
    });
}

criterion_group!(benches, satisfiability_benchmark, consistency_benchmark);
criterion_main!(benches);

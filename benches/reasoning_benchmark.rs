use criterion::{black_box, criterion_group, criterion_main, Criterion};
use oxidowl::{
    concept::{concept, ConceptExpression},
    parallel::{ParallelReasoningEngine, ParallelConfig, TaskPriority},
    blocking::SimpleBlockingStrategy,
    cache::ReasoningCache,
};
use std::sync::Arc;

fn satisfiability_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = ParallelConfig::default();
    let cache = ReasoningCache::new();
    let blocking_strategy = Arc::new(SimpleBlockingStrategy::new());
    let engine = ParallelReasoningEngine::new(config, cache, blocking_strategy);
    
    c.bench_function("simple_concept_satisfiability", |b| {
        b.to_async(&rt).iter(|| async {
            let concept = concept("A");
            let result = engine.check_satisfiability_priority(
                black_box(concept),
                TaskPriority::Normal
            ).await;
            black_box(result)
        })
    });
    
    c.bench_function("complex_concept_satisfiability", |b| {
        b.to_async(&rt).iter(|| async {
            let concept = concept("A")
                .and(concept("B"))
                .or(concept("C").negate());
            let result = engine.check_satisfiability_priority(
                black_box(concept),
                TaskPriority::Normal
            ).await;
            black_box(result)
        })
    });
}

fn batch_processing_benchmark(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    
    let config = ParallelConfig::default();
    let cache = ReasoningCache::new();
    let blocking_strategy = Arc::new(SimpleBlockingStrategy::new());
    let engine = ParallelReasoningEngine::new(config, cache, blocking_strategy);
    
    c.bench_function("batch_satisfiability_10", |b| {
        b.to_async(&rt).iter(|| async {
            let concepts: Vec<ConceptExpression> = (0..10)
                .map(|i| concept(&format!("Concept{}", i)))
                .collect();
            let result = engine.check_satisfiability_batch(black_box(concepts)).await;
            black_box(result)
        })
    });
    
    c.bench_function("batch_satisfiability_100", |b| {
        b.to_async(&rt).iter(|| async {
            let concepts: Vec<ConceptExpression> = (0..100)
                .map(|i| concept(&format!("Concept{}", i)))
                .collect();
            let result = engine.check_satisfiability_batch(black_box(concepts)).await;
            black_box(result)
        })
    });
}

criterion_group!(benches, satisfiability_benchmark, batch_processing_benchmark);
criterion_main!(benches);


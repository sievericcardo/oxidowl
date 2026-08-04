use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use oxidowl::{
    Ontology, ReasonerConfig, ReasoningService,
    ontology::{
        Class, ClassExpression, IRI,
        axioms::{Axiom, SubClassOfAxiom},
        indexes::AxiomIndex,
    },
};
use std::hint::black_box;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_class(name: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(&format!("http://bench.example.org/{name}"))))
}

/// Build an ontology with a linear chain: C0 ⊑ C1 ⊑ … ⊑ C(n-1)
fn build_linear_chain(n: usize) -> Ontology {
    let mut ont = Ontology::new();
    for i in 0..n.saturating_sub(1) {
        let sub = make_class(&format!("C{i}"));
        let sup = make_class(&format!("C{}", i + 1));
        ont.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: i as u64 + 1,
            subclass: sub,
            superclass: sup,
            annotations: vec![],
        }));
    }
    ont
}

// ─────────────────────────────────────────────────────────────────────────────
// Axiom index lookups (Phase 1 — O(1) lookups)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_axiom_index_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("axiom_index");

    for size in [10usize, 100, 1000] {
        let ont = build_linear_chain(size);
        let index = AxiomIndex::build(ont.axioms());
        let target = make_class("C0");

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("direct_superclasses", size),
            &size,
            |b, _| {
                b.iter(|| {
                    let result = index.direct_superclasses(black_box(&target));
                    black_box(result);
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("build_index", size),
            &size,
            |b, _| {
                b.iter(|| {
                    let idx = AxiomIndex::build(black_box(ont.axioms()));
                    black_box(idx);
                });
            },
        );
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// BFS subsumption (Phase 1 — O(N+E) vs O(N·M))
// ─────────────────────────────────────────────────────────────────────────────

fn bench_subsumption(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("subsumption");

    for size in [10usize, 50, 200] {
        let ont = build_linear_chain(size);
        let config = ReasonerConfig::default();
        let service = rt.block_on(async { ReasoningService::new(ont, config) }).unwrap();

        let sub = make_class("C0");
        let sup = make_class(&format!("C{}", size - 1));

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(
            BenchmarkId::new("is_subsumed_by_chain", size),
            &size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let r = service.is_subsumed_by(black_box(&sub), black_box(&sup)).await;
                        black_box(r)
                    })
                });
            },
        );
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Classification performance
// ─────────────────────────────────────────────────────────────────────────────

fn bench_classification(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("classification");
    group.sample_size(10); // Classification is expensive — fewer samples

    for size in [10usize, 100] {
        let ont = build_linear_chain(size);
        let config = ReasonerConfig::default();
        let service = rt.block_on(async { ReasoningService::new(ont, config) }).unwrap();

        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(
            BenchmarkId::new("classify_linear_chain", size),
            &size,
            |b, _| {
                b.iter(|| {
                    rt.block_on(async {
                        let r = service.classify().await;
                        black_box(r)
                    })
                });
            },
        );
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Performance regression guards
// ─────────────────────────────────────────────────────────────────────────────

fn bench_regression_guards(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("regression");

    // Single consistency query must stay under 1 ms on an empty ontology.
    group.throughput(Throughput::Elements(1));
    group.bench_function("single_query_latency", |b| {
        let config = ReasonerConfig::default();
        let service = rt.block_on(async { ReasoningService::new(Ontology::new(), config) }).unwrap();
        b.iter(|| {
            rt.block_on(async {
                let r = service.is_consistent().await;
                black_box(r)
            })
        });
    });

    // 100-class chain consistency must stay under 500 ms.
    group.sample_size(10);
    group.bench_function("medium_ontology_consistency", |b| {
        let ont = build_linear_chain(100);
        let config = ReasonerConfig::default();
        let service = rt.block_on(async { ReasoningService::new(ont, config) }).unwrap();
        b.iter(|| {
            rt.block_on(async {
                let r = service.is_consistent().await;
                black_box(r)
            })
        });
    });

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Batch subsumption check (Phase 5.5)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_batch_subsumption(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("batch");

    let ont = build_linear_chain(200);
    let config = ReasonerConfig::default();
    let service = Arc::new(rt.block_on(async { ReasoningService::new(ont, config) }).unwrap());

    let sub = make_class("C0");
    let sup = make_class("C100");

    for batch_size in [1usize, 10, 100] {
        group.throughput(Throughput::Elements(batch_size as u64));

        // Sequential baseline: N actor round-trips
        group.bench_with_input(
            BenchmarkId::new("sequential_subsumptions", batch_size),
            &batch_size,
            |b, &n| {
                let svc = Arc::clone(&service);
                b.iter(|| {
                    rt.block_on(async {
                        let mut results = Vec::with_capacity(n);
                        for _ in 0..n {
                            let r = svc.is_subsumed_by(&sub, &sup).await;
                            results.push(r);
                        }
                        black_box(results)
                    })
                });
            },
        );

        // Batch: 1 actor round-trip for all N pairs
        group.bench_with_input(
            BenchmarkId::new("batch_subsumptions", batch_size),
            &batch_size,
            |b, &n| {
                let svc = Arc::clone(&service);
                let pairs: Vec<_> = (0..n).map(|_| (sub.clone(), sup.clone())).collect();
                b.iter(|| {
                    let pairs = pairs.clone();
                    rt.block_on(async {
                        let r = svc.batch_check_subsumptions(pairs).await;
                        black_box(r)
                    })
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_axiom_index_lookup,
    bench_subsumption,
    bench_classification,
    bench_regression_guards,
    bench_batch_subsumption,
);
criterion_main!(benches);

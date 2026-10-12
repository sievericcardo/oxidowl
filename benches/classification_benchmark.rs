//! Classification performance benchmark (workstream A8).
//!
//! Gates:
//! - 1000-class linear chain classify (with / without role axioms) < 500 ms.
//! - ORE-scale ontology (ore_ont_10004.owl) classify < 1 s.
//! - 100 repeated subsumption queries after classification < 100 ms (cache).

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use oxidowl::{
    Ontology, ReasonerConfig, ReasoningService,
    ontology::{
        Class, ClassExpression, IRI, ObjectProperty, ObjectPropertyExpression,
        axioms::{Axiom, ObjectPropertyDomainAxiom, SubClassOfAxiom},
    },
    parsers::parse_file_auto,
};
use std::hint::black_box;

fn make_class(name: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(&format!(
        "http://bench.example.org/{name}"
    ))))
}

fn make_prop(name: &str) -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty {
        iri: IRI::new(&format!("http://bench.example.org/{name}")),
    })
}

fn sub(sub: ClassExpression, sup: ClassExpression, id: u64) -> Axiom {
    Axiom::SubClassOf(SubClassOfAxiom {
        id,
        subclass: sub,
        superclass: sup,
        annotations: vec![],
    })
}

/// Pure linear chain: C0 ⊑ C1 ⊑ … ⊑ C(n-1).  EL-conforming → EL fast path.
fn build_linear_chain(n: usize) -> Ontology {
    let mut ont = Ontology::new();
    for i in 0..n.saturating_sub(1) {
        ont.add_axiom(sub(
            make_class(&format!("C{i}")),
            make_class(&format!("C{}", i + 1)),
            i as u64 + 1,
        ));
    }
    ont
}

/// Chain with role axioms: `C_i ⊑ ∃R.C_{i+1}` plus `Domain(R) = C_{n-1}`.
/// Still EL-conforming, but exercises role-successor + domain absorption.
fn build_role_chain(n: usize) -> Ontology {
    let mut ont = Ontology::new();
    for i in 0..n.saturating_sub(1) {
        ont.add_axiom(sub(
            make_class(&format!("C{i}")),
            make_class(&format!("C{}", i + 1)),
            i as u64 + 1,
        ));
        ont.add_axiom(sub(
            make_class(&format!("C{i}")),
            ClassExpression::ObjectSomeValuesFrom {
                property: make_prop("R"),
                filler: Box::new(make_class(&format!("C{}", i + 1))),
            },
            (n + i) as u64 + 1,
        ));
    }
    ont.add_axiom(Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
        id: 2 * n as u64,
        property: make_prop("R"),
        domain: make_class(&format!("C{}", n - 1)),
        annotations: vec![],
    }));
    ont
}

/// Mostly-deterministic ontology that is *not* EL (contains unions), forcing
/// the general absorption + dependency-graph path.  The union leaves are the
/// only genuinely non-deterministic classes.
fn build_absorption_mixed(n: usize) -> Ontology {
    let mut ont = build_role_chain(n);
    let mut id = 100_000u64;
    for (leaf, a, b) in [("U1", "X1", "Y1"), ("U2", "X2", "Y2")] {
        ont.add_axiom(sub(
            make_class(leaf),
            ClassExpression::ObjectUnionOf(vec![make_class(a), make_class(b)]),
            id,
        ));
        id += 1;
    }
    ont
}

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build runtime")
}

// ─────────────────────────────────────────────────────────────────────────────
// Classification benches
// ─────────────────────────────────────────────────────────────────────────────

fn bench_classification(c: &mut Criterion) {
    let rt = rt();
    let mut group = c.benchmark_group("classification");
    group.sample_size(10);

    // NOTE: a fresh `ReasoningService` is created per iteration so the *cold*
    // classification path is measured (the classification result is cached on
    // the reasoner, so re-classifying the same service would hit the cache).

    for size in [100usize, 1000] {
        let ont = build_linear_chain(size);
        let config = ReasonerConfig::default();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("linear_chain", size), &size, |b, _| {
            b.iter(|| {
                let ont = ont.clone();
                rt.block_on(async {
                    let service = ReasoningService::new(ont, config.clone()).unwrap();
                    let r = service.classify().await;
                    black_box(r)
                })
            });
        });
    }

    for size in [1000usize] {
        // Non-EL role chain (role-successor + domain absorption).  A single
        // union leaf forces the general absorption + dependency-graph path
        // (workstream A8); the pure-EL chain timing gate is `linear_chain`.
        let mut ont = build_role_chain(size);
        ont.add_axiom(sub(
            make_class("U0"),
            ClassExpression::ObjectUnionOf(vec![make_class("X0"), make_class("Y0")]),
            200_000,
        ));
        let config = ReasonerConfig::default();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("role_chain", size), &size, |b, _| {
            b.iter(|| {
                let ont = ont.clone();
                rt.block_on(async {
                    let service = ReasoningService::new(ont, config.clone()).unwrap();
                    let r = service.classify().await;
                    black_box(r)
                })
            });
        });
    }

    for size in [1000usize] {
        let ont = build_absorption_mixed(size);
        let config = ReasonerConfig::default();
        group.throughput(Throughput::Elements(size as u64));
        group.bench_with_input(BenchmarkId::new("absorption_mixed", size), &size, |b, _| {
            b.iter(|| {
                let ont = ont.clone();
                rt.block_on(async {
                    let service = ReasoningService::new(ont, config.clone()).unwrap();
                    let r = service.classify().await;
                    black_box(r)
                })
            });
        });
    }

    // ORE-scale ontology (EL-approximated clinical measurement ontology).
    {
        let ont = parse_file_auto("ontologies-test/ore_ont_10004.owl")
            .expect("Failed to parse ORE ontology");
        let config = ReasonerConfig::default();
        group.bench_function("ore_ont_10004", |b| {
            b.iter(|| {
                let ont = ont.clone();
                rt.block_on(async {
                    let service = ReasoningService::new(ont, config.clone()).unwrap();
                    let r = service.classify().await;
                    black_box(r)
                })
            });
        });
    }

    group.finish();
}

// ─────────────────────────────────────────────────────────────────────────────
// Query-after-classification bench (cache fast path)
// ─────────────────────────────────────────────────────────────────────────────

fn bench_queries_after_classification(c: &mut Criterion) {
    let rt = rt();

    // Non-EL ontology so the general classification path populates the cache.
    let ont = build_absorption_mixed(1000);
    let config = ReasonerConfig::default();
    let service = rt
        .block_on(async { ReasoningService::new(ont, config) })
        .unwrap();
    rt.block_on(service.classify()).unwrap();

    let sub = make_class("C0");
    let sup = make_class("C500");

    let mut group = c.benchmark_group("queries_after_classification");
    group.sample_size(10);
    group.throughput(Throughput::Elements(100));

    group.bench_function("100_subsumption_queries", |b| {
        b.iter(|| {
            rt.block_on(async {
                for _ in 0..100 {
                    let r = service.is_subsumed_by(&sub, &sup).await;
                    let _ = black_box(r);
                }
            })
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_classification,
    bench_queries_after_classification,
);
criterion_main!(benches);

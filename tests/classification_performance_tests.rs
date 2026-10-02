//! Workstream A8 — classification performance & correctness tests.
//!
//! Verifies the absorption + dependency-graph (Elgot-style) classification
//! path: correctness of the absorbed subsumptions, that most classes avoid the
//! full tableau on a highly-absorbable ontology, and coarse latency gates
//! (CI-safe, 5x the production targets).
//!
//! NOTE: this file deliberately avoids touching `tests/` files owned by the
//! concurrent C3 agent; it uses its own unique name.

use oxidowl::{
    Ontology, ReasonerConfig, ReasoningService,
    ontology::{
        Class, ClassExpression, IRI, ObjectProperty, ObjectPropertyExpression,
        axioms::{Axiom, ObjectPropertyDomainAxiom, SubClassOfAxiom},
    },
    parsers::parse_file_auto,
};
use std::time::Instant;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build runtime")
}

/// Size override via env (`A8_SIZE_<KEY>`); falls back to a debug / release
/// default so CI can tune without editing code.
fn env_size(key: &str, debug_default: usize, release_default: usize) -> usize {
    if let Ok(v) = std::env::var(format!("A8_SIZE_{key}")) {
        if let Ok(n) = v.parse::<usize>() {
            return n;
        }
    }
    if cfg!(debug_assertions) {
        debug_default
    } else {
        release_default
    }
}

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

/// Non-EL ontology: role chain + union leaves.  Exercises the general
/// absorption path; the union leaves are the only non-deterministic classes.
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

/// Small correctness fixture: `A ⊑ ∃R.B`, `Domain(R) = D`, `∃R.B ⊑ E`,
/// `A ⊑ C`, and a union leaf `U ⊑ (X ⊔ Y)` to force the general path.
fn build_absorption_correctness() -> Ontology {
    let mut ont = Ontology::new();
    let mut id = 1u64;
    // A ⊑ ∃R.B
    ont.add_axiom(sub(
        make_class("A"),
        ClassExpression::ObjectSomeValuesFrom {
            property: make_prop("R"),
            filler: Box::new(make_class("B")),
        },
        id,
    ));
    id += 1;
    // Domain(R) = D  →  A ⊑ D
    ont.add_axiom(Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
        id,
        property: make_prop("R"),
        domain: make_class("D"),
        annotations: vec![],
    }));
    id += 1;
    // ∃R.B ⊑ E  →  A ⊑ E
    ont.add_axiom(sub(
        ClassExpression::ObjectSomeValuesFrom {
            property: make_prop("R"),
            filler: Box::new(make_class("B")),
        },
        make_class("E"),
        id,
    ));
    id += 1;
    // A ⊑ C (plain told subsumption)
    ont.add_axiom(sub(make_class("A"), make_class("C"), id));
    id += 1;
    // U ⊑ (X ⊔ Y) — non-deterministic leaf
    ont.add_axiom(sub(
        make_class("U"),
        ClassExpression::ObjectUnionOf(vec![make_class("X"), make_class("Y")]),
        id,
    ));
    ont
}

// ─────────────────────────────────────────────────────────────────────────────
// Correctness
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_absorption_correctness() {
    let ontology = build_absorption_correctness();
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let result = rt
        .block_on(service.classify())
        .expect("Classification failed");

    // Absorbed subsumptions must hold.
    let a = make_class("A");
    for (sup, should_hold) in [
        (make_class("D"), true),  // role domain
        (make_class("E"), true),  // role-range GCI
        (make_class("C"), true),  // told
        (make_class("B"), false), // not implied
    ] {
        let holds = rt
            .block_on(service.is_subsumed_by(&a, &sup))
            .expect("Subsumption check failed");
        assert_eq!(holds, should_hold, "A ⊑ {sup:?} should be {should_hold}");
    }

    // The union leaf is genuinely non-deterministic and must have reached the
    // tableau fallback, while the deterministic classes must not have.
    assert!(
        result.counters.classes_needing_tableau >= 1,
        "expected the union leaf to need the tableau, counters: {:?}",
        result.counters
    );
    assert!(
        result.counters.classes_avoiding_tableau >= 3,
        "expected deterministic classes to avoid the tableau, counters: {:?}",
        result.counters
    );

    // Deterministic subsumers must appear in the returned hierarchy.
    let a_sups = result.hierarchy.get(&a).cloned().unwrap_or_default();
    assert!(a_sups.contains(&make_class("D")));
    assert!(a_sups.contains(&make_class("E")));
    assert!(a_sups.contains(&make_class("C")));
}

// ─────────────────────────────────────────────────────────────────────────────
// Absorption effectiveness: most classes avoid the tableau
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_highly_absorbable_ontology_avoids_tableau_for_most_classes() {
    let n = env_size("MIXED", 60, 500);
    let ontology = build_absorption_mixed(n);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let result = rt
        .block_on(service.classify())
        .expect("Classification failed");

    // Only the union leaves (2) should need the tableau; the ~n chain classes
    // must be classified by absorption + saturation + told subsumers alone.
    assert!(
        result.counters.classes_needing_tableau <= 4,
        "expected at most the union leaves to need the tableau, counters: {:?}",
        result.counters
    );
    assert!(
        result.counters.classes_avoiding_tableau as usize >= n.saturating_sub(4),
        "expected most classes to avoid the tableau, counters: {:?}",
        result.counters
    );
    assert!(
        result.counters.real_tableau_checks <= 64,
        "real-tableau probes should be bounded by the candidate filter, counters: {:?}",
        result.counters
    );

    // Chain subsumption must still hold end-to-end.
    let c0 = make_class("C0");
    let c_last = make_class(&format!("C{}", n - 1));
    let holds = rt
        .block_on(service.is_subsumed_by(&c0, &c_last))
        .expect("Subsumption check failed");
    assert!(holds, "C0 ⊑ C{n} should hold via the chain");
}

// ─────────────────────────────────────────────────────────────────────────────
// Coarse latency gates (5x CI-safe margins)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_linear_chain_classification_latency() {
    let n = env_size("CHAIN", 100, 1000);
    let budget_ms = if cfg!(debug_assertions) { 5_000 } else { 2_500 }; // 5x 500ms

    let ontology = build_linear_chain(n);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let start = Instant::now();
    let _result = rt
        .block_on(service.classify())
        .expect("Classification failed");
    let elapsed = start.elapsed().as_millis();

    assert!(
        elapsed < budget_ms,
        "linear-chain classification of {n} classes took {elapsed}ms, expected <{budget_ms}ms"
    );
}

#[test]
fn test_role_chain_classification_latency() {
    // A role chain (`C_i ⊑ ∃R.C_{i+1}`, `Domain(R) = C_top`) — exercises the
    // role-successor + role-domain absorption.  The EL fast path is excluded by
    // adding a single union leaf, so this measures the general absorption +
    // dependency-graph path (workstream A8); the pure-EL timing gate is covered
    // by `test_linear_chain_classification_latency` and the release benches.
    let n = env_size("ROLE", 60, 1000);
    let budget_ms = if cfg!(debug_assertions) {
        12_000
    } else {
        2_500
    }; // 5x 500ms

    let mut ontology = build_role_chain(n);
    ontology.add_axiom(sub(
        make_class("U0"),
        ClassExpression::ObjectUnionOf(vec![make_class("X0"), make_class("Y0")]),
        200_000,
    ));
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let start = Instant::now();
    let result = rt
        .block_on(service.classify())
        .expect("Classification failed");
    let elapsed = start.elapsed().as_millis();

    assert!(
        elapsed < budget_ms,
        "role-chain classification of {n} classes took {elapsed}ms, expected <{budget_ms}ms (counters: {:?})",
        result.counters
    );
}

#[test]
fn test_ore_scale_classification_latency() {
    // Try to locate the ORE fixture; skip gracefully if unavailable.
    let path = [
        "ontologies-test/ore_ont_10004.owl",
        "../ontologies-test/ore_ont_10004.owl",
    ]
    .iter()
    .find(|p| std::path::Path::new(p).exists());
    let Some(path) = path else {
        eprintln!("Skipping ORE latency test: fixture not found");
        return;
    };

    let ontology = match parse_file_auto(path) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Skipping ORE latency test: parse failed: {e}");
            return;
        }
    };

    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let budget_ms = if cfg!(debug_assertions) {
        15_000
    } else {
        5_000
    }; // 5x 1s
    let start = Instant::now();
    let result = rt
        .block_on(service.classify())
        .expect("Classification failed");
    let elapsed = start.elapsed().as_millis();

    assert!(
        elapsed < budget_ms,
        "ORE classification took {elapsed}ms, expected <{budget_ms}ms (counters: {:?})",
        result.counters
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Diagnostic: print actual release-mode timings + counters.
// No-op unless `A8_DIAG=1` is set; run with
// `cargo test --release --test classification_performance_tests a8_diag -- --nocapture`
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn a8_diag_print_timings() {
    if std::env::var("A8_DIAG").is_err() {
        return;
    }
    let rt = rt();

    let chain_general = {
        let mut ont = build_linear_chain(1000);
        ont.add_axiom(sub(
            make_class("U0"),
            ClassExpression::ObjectUnionOf(vec![make_class("X0"), make_class("Y0")]),
            1_000_000,
        ));
        ont
    };
    let role_chain = {
        let mut ont = build_role_chain(1000);
        ont.add_axiom(sub(
            make_class("U0"),
            ClassExpression::ObjectUnionOf(vec![make_class("X0"), make_class("Y0")]),
            2_000_000,
        ));
        ont
    };

    for (name, ont) in [
        ("chain_general_1000", chain_general),
        ("role_chain_1000", role_chain),
        ("absorption_mixed_1000", build_absorption_mixed(1000)),
    ] {
        let service = rt
            .block_on(async { ReasoningService::new(ont, ReasonerConfig::default()) })
            .unwrap();
        let start = Instant::now();
        let r = rt.block_on(service.classify()).unwrap();
        println!(
            "[A8-DIAG] {name}: {:?} counters={:?}",
            start.elapsed(),
            r.counters
        );
    }

    if let Ok(ont) = parse_file_auto("ontologies-test/ore_ont_10004.owl") {
        let service = rt
            .block_on(async { ReasoningService::new(ont, ReasonerConfig::default()) })
            .unwrap();
        let start = Instant::now();
        let r = rt.block_on(service.classify()).unwrap();
        println!(
            "[A8-DIAG] ore_ont_10004: {:?} counters={:?}",
            start.elapsed(),
            r.counters
        );
    } else {
        println!("[A8-DIAG] ore_ont_10004: fixture not found");
    }

    // 100 subsumption queries after classification (cache fast path).
    let service = rt
        .block_on(async {
            ReasoningService::new(build_absorption_mixed(1000), ReasonerConfig::default())
        })
        .unwrap();
    rt.block_on(service.classify()).unwrap();
    let sub = make_class("C0");
    let sup = make_class("C500");
    let start = Instant::now();
    for _ in 0..100 {
        let _ = rt.block_on(service.is_subsumed_by(&sub, &sup)).unwrap();
    }
    println!("[A8-DIAG] 100_subsumption_queries: {:?}", start.elapsed());
}

#[test]
fn test_repeated_subsumption_queries_after_classification() {
    let n = env_size("QUERY", 60, 1000);
    let budget_ms = if cfg!(debug_assertions) { 2_000 } else { 500 }; // 5x 100ms

    let ontology = build_absorption_mixed(n);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    rt.block_on(service.classify())
        .expect("Classification failed");

    let sub = make_class("C0");
    let sup = make_class(&format!("C{}", n - 1));

    let start = Instant::now();
    for _ in 0..100 {
        let holds = rt
            .block_on(service.is_subsumed_by(&sub, &sup))
            .expect("Subsumption check failed");
        assert!(holds, "C0 ⊑ C{n} should hold");
    }
    let elapsed = start.elapsed().as_millis();

    assert!(
        elapsed < budget_ms,
        "100 subsumption queries took {elapsed}ms, expected <{budget_ms}ms"
    );
}

/// CR-011 M1 — `get_types(direct = true)` must filter via the shared
/// incrementally-maintained axiom index plus a per-query subclass closure,
/// **not** O(types²) `is_subclass_of` calls that each fell through to a full
/// `build_index()` rebuild (measured 3/34/120 ms per call at 2 k/20 k/60 k
/// axioms — 30 types ≈ 2.7 s at 2 k alone).
///
/// Fixture: a 2 000-link subclass chain plus an individual typed by the
/// first 30 chain classes.  Pre-fix this took seconds per call; the gate
/// below is a generous CI-safe 1 s (≈5× the observed post-fix time, ≈5×
/// *below* the pre-fix time).
#[test]
fn get_types_direct_filters_without_index_rebuild() {
    use oxidowl::{Reasoner, ReasonerConfig};
    use std::time::Duration;

    const N_CHAIN: usize = 2000;
    const N_TYPES: usize = 30;

    let cls = |i: usize| {
        ClassExpression::Class(Class::new(IRI::new(&format!(
            "http://bench.example.org/C{i}"
        ))))
    };
    let mut ont = Ontology::new();
    for i in 0..N_CHAIN {
        ont.add_axiom(sub(cls(i), cls(i + 1), i as u64));
    }
    let x = oxidowl::ontology::Individual::Named(oxidowl::ontology::NamedIndividual {
        iri: IRI::new("http://bench.example.org/x"),
    });
    for i in 0..N_TYPES {
        ont.add_axiom(Axiom::ClassAssertion(
            oxidowl::ontology::axioms::ClassAssertionAxiom {
                id: (N_CHAIN + i) as u64,
                class: cls(i),
                individual: x.clone(),
                annotations: vec![],
            },
        ));
    }
    assert_eq!(ont.axioms().len(), N_CHAIN + N_TYPES);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    reasoner.load_ontology(ont).expect("load ontology");

    let start = Instant::now();
    let direct = reasoner
        .get_types(&x, true)
        .expect("get_types(direct=true)");
    let elapsed = start.elapsed();

    // Correctness: among the chain types C0…C29 only C0 is most-specific.
    assert_eq!(
        direct.len(),
        1,
        "only the bottom chain type is a direct type; got {direct:?}"
    );
    assert_eq!(
        direct[0],
        cls(0),
        "direct type must be C0; got {:?}",
        direct[0]
    );

    // Performance gate (CR-011 M1): a single near-linear pass.
    assert!(
        elapsed < Duration::from_millis(1000),
        "get_types(direct=true) took {elapsed:?} at {N_CHAIN} axioms / \
         {N_TYPES} types — expected well under 1 s (CR-011 M1 regression)"
    );
}

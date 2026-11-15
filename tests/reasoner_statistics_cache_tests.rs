//! Feature-parity tests: shared reasoning statistics and query caching.
//!
//! - `get_statistics()` must return *real* numbers (it historically always
//!   returned zeros because the per-operation statistics were local);
//! - `classify()` is memoised (a second call does not re-run the tableau);
//! - `is_consistent()` is memoised against the ontology version and
//!   invalidated by every mutation.

use oxidowl::{
    config::ReasonerConfig,
    core::reasoner::Reasoner,
    ontology::{
        Axiom, Class, ClassAssertionAxiom, ClassExpression, IRI, Individual, Ontology,
        SubClassOfAxiom,
    },
};
use std::time::Duration;

// ── Helpers ──────────────────────────────────────────────────────────────────

fn cls(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn sub(sub: ClassExpression, sup: ClassExpression) -> Axiom {
    Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: sub,
        superclass: sup,
        annotations: Vec::new(),
    })
}

/// `A ⊑ B`, `A(a)` — a small consistent ontology (EL-conforming).
fn consistent_ontology() -> Ontology {
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/stats"));
    ontology.add_axiom(sub(
        cls("http://example.org/A"),
        cls("http://example.org/B"),
    ));
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 0,
        class: cls("http://example.org/A"),
        individual: Individual::named(IRI::new("http://example.org/a")),
        annotations: Vec::new(),
    }));
    ontology
}

fn reasoner() -> Reasoner {
    Reasoner::new(ReasonerConfig::default()).expect("reasoner construction")
}

// ── Statistics ───────────────────────────────────────────────────────────────

#[test]
fn classify_updates_shared_statistics_and_is_memoised() {
    let mut r = reasoner();
    r.load_ontology(consistent_ontology()).expect("load");

    assert_eq!(
        r.get_statistics().classification_runs,
        0,
        "no classification has run yet"
    );

    r.classify().expect("first classify");
    let stats = r.get_statistics();
    assert_eq!(
        stats.classification_runs, 1,
        "the first classify must be counted: {stats:?}"
    );
    assert!(
        stats.total_reasoning_time > Duration::ZERO,
        "classification wall time must be recorded: {stats:?}"
    );

    // Memoised: the second call must not re-run classification.
    r.classify().expect("second classify");
    let stats = r.get_statistics();
    assert_eq!(
        stats.classification_runs, 1,
        "a cache-served classify must not count as a run: {stats:?}"
    );
}

#[test]
fn get_instances_records_query_statistics() {
    let mut r = reasoner();
    r.load_ontology(consistent_ontology()).expect("load");

    let instances = r
        .get_instances(&cls("http://example.org/B"), false)
        .expect("instances");
    assert_eq!(instances.len(), 1);

    let stats = r.get_statistics();
    assert_eq!(
        stats.instance_queries, 1,
        "the query must be counted: {stats:?}"
    );
    assert!(
        stats.total_reasoning_time > Duration::ZERO,
        "query wall time must be recorded: {stats:?}"
    );
}

// ── Consistency memoisation ──────────────────────────────────────────────────

#[test]
fn is_consistent_is_memoized_and_invalidated_by_mutation() {
    let mut r = reasoner();
    r.load_ontology(consistent_ontology()).expect("load");

    assert!(r.is_consistent().expect("consistency"));
    assert!(r.is_consistent().expect("consistency (cached)"));

    let stats = r.get_statistics();
    assert_eq!(
        stats.consistency_checks, 1,
        "the tableau must run exactly once: {stats:?}"
    );
    assert_eq!(
        stats.consistency_cache_hits, 1,
        "the second call must be served from the memo: {stats:?}"
    );

    // Mutating the ontology invalidates the memo.
    r.add_axiom(sub(
        cls("http://example.org/A"),
        ClassExpression::Class(Class::new(IRI::owl_nothing())),
    ))
    .expect("add axiom");

    assert!(
        !r.is_consistent().expect("consistency after mutation"),
        "A ⊑ owl:Nothing makes the ontology inconsistent"
    );
    let stats = r.get_statistics();
    assert_eq!(
        stats.consistency_checks, 2,
        "the mutation must invalidate the memo: {stats:?}"
    );
    assert_eq!(stats.consistency_cache_hits, 1, "no stale hit allowed");

    // The invalid answer is itself memoised for the new version.
    assert!(!r.is_consistent().expect("consistency (re-cached)"));
    let stats = r.get_statistics();
    assert_eq!(stats.consistency_checks, 2, "no re-run without mutation");
    assert_eq!(stats.consistency_cache_hits, 2, "second stale-free hit");
}

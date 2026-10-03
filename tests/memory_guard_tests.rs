//! Enforced reasoning memory cap (`reasoning.max_memory_mb`).
//!
//! Regression tests for the in-process RSS guard (`MemoryGuard`): a reasoning
//! call under a cap the process already exceeds must return a *graceful*
//! `Err("Memory limit exceeded: …")` — never abort / SIGKILL the test process —
//! while the same ontology under a sane (or unlimited) cap must still reason
//! normally.
//!
//! Semantics under test (`None`/`0` = unlimited):
//! - `max_memory_mb = Some(1)` → every guarded reasoning loop aborts with `Err`
//! - `max_memory_mb = Some(4096)` (the shipped default) → small ontologies pass
//! - `max_memory_mb = None` / `Some(0)` → no enforcement at all
//!
//! The unit-level guard behaviour (interval gating, sampling) lives in
//! `src/performance.rs` (`MemoryGuard` tests).

use oxidowl::{
    Result,
    config::{OWLProfile, ReasonerConfig, ReasoningConfig},
    core::reasoner::Reasoner,
    ontology::{
        Axiom, Class, ClassExpression, IRI, Individual, NamedIndividual, Ontology,
        axioms::{ClassAssertionAxiom, SubClassOfAxiom},
    },
};

const EX: &str = "http://ex.org/";

fn class(local: &str) -> ClassExpression {
    ClassExpression::Class(Class {
        iri: IRI::new(&format!("{EX}{local}")),
    })
}

fn sub(sub: &str, sup: &str, id: u64) -> Axiom {
    Axiom::SubClassOf(SubClassOfAxiom {
        id,
        subclass: class(sub),
        superclass: class(sup),
        annotations: vec![],
    })
}

fn assertion(individual: &str, c: &str, id: u64) -> Axiom {
    Axiom::ClassAssertion(ClassAssertionAxiom {
        id,
        individual: Individual::Named(NamedIndividual {
            iri: IRI::new(&format!("{EX}{individual}")),
        }),
        class: class(c),
        annotations: vec![],
    })
}

/// Small EL-friendly ontology: A ⊑ B, B ⊑ C, A(i).
fn small_ontology() -> Ontology {
    let mut o = Ontology::new();
    o.add_axiom(sub("A", "B", 1));
    o.add_axiom(sub("B", "C", 2));
    o.add_axiom(assertion("i", "A", 3));
    o
}

fn reasoner_with(max_memory_mb: Option<u64>, target_profile: OWLProfile) -> Result<Reasoner> {
    let config = ReasonerConfig {
        reasoning: ReasoningConfig {
            max_memory_mb,
            target_profile,
            ..ReasoningConfig::default()
        },
        ..ReasonerConfig::default()
    };
    let mut reasoner = Reasoner::new(config)?;
    reasoner.load_ontology(small_ontology())?;
    Ok(reasoner)
}

/// Assert the error is the memory-guard abort (not a crash, not another error).
fn assert_is_memory_limit_err(err: &oxidowl::Error) {
    assert!(
        err.is_memory_limit(),
        "expected a memory-limit abort, got: {err}"
    );
    let msg = err.to_string();
    assert!(
        msg.contains("Memory limit exceeded"),
        "message must carry the memory-limit text, got: {msg}"
    );
    assert!(
        msg.contains("max_memory_mb"),
        "message must point at the config knob, got: {msg}"
    );
}

// ── 1. Cap of 1 MB: graceful Err, no abort ──────────────────────────────────

#[test]
fn classify_with_tiny_memory_cap_errors_gracefully() -> Result<()> {
    let mut reasoner = reasoner_with(Some(1), OWLProfile::Auto)?;

    let err = reasoner
        .classify()
        .expect_err("a 1 MB cap must abort classification");
    assert_is_memory_limit_err(&err);
    Ok(())
}

#[test]
fn dl_profile_classify_with_tiny_memory_cap_errors_gracefully() -> Result<()> {
    // Explicit DL target forces the general (absorption/tableau) classification
    // path instead of the EL fast path — the guard must abort that path too.
    let mut reasoner = reasoner_with(Some(1), OWLProfile::DL)?;

    let err = reasoner
        .classify()
        .expect_err("a 1 MB cap must abort DL classification");
    assert_is_memory_limit_err(&err);
    Ok(())
}

#[test]
fn consistency_check_with_tiny_memory_cap_errors_gracefully() -> Result<()> {
    // Consistency runs `TableauExecutor::run` directly, i.e. the expansion-loop
    // guard (next to the timeout check).
    let reasoner = reasoner_with(Some(1), OWLProfile::DL)?;

    let err = reasoner
        .is_consistent()
        .expect_err("a 1 MB cap must abort the tableau consistency check");
    assert_is_memory_limit_err(&err);
    Ok(())
}

// ── 2. Sane cap: reasoning still works ──────────────────────────────────────

#[test]
fn classify_with_sane_memory_cap_succeeds() -> Result<()> {
    let mut reasoner = reasoner_with(Some(4096), OWLProfile::Auto)?;

    let result = reasoner.classify()?;
    let a = class("A");
    let c = class("C");
    assert!(
        result.is_subsumed(&a, &c),
        "A ⊑ C must still be derived under a 4 GB cap"
    );
    Ok(())
}

// ── 3. None / 0 = unlimited ─────────────────────────────────────────────────

#[test]
fn unlimited_memory_caps_never_enforce() -> Result<()> {
    for cap in [None, Some(0)] {
        let mut reasoner = reasoner_with(cap, OWLProfile::Auto)?;
        let result = reasoner
            .classify()
            .unwrap_or_else(|e| panic!("cap {cap:?} must be unlimited, got: {e}"));
        assert!(result.is_subsumed(&class("A"), &class("C")));
        assert!(reasoner.is_consistent()?);
    }
    Ok(())
}

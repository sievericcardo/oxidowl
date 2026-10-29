//! Workstream A9 — incremental reasoning is OPT-IN and OFF by default.
//!
//! User decision: proper incremental reasoning is hard; full recompute remains
//! the default reasoning path (`incremental_reasoning: false`). When explicitly
//! enabled, the incremental service exposes `add_axiom_incrementally` /
//! `remove_axiom_incrementally`; these currently fall back to a version-stamped
//! full recompute inside the base reasoning service so their results are
//! identical to a full recompute of the updated ontology.
//!
//! These tests pin the default, verify the opt-in path agrees with a full
//! recompute, and confirm the default (non-incremental) path is unaffected.

use oxidowl::{
    Result,
    config::ReasonerConfig,
    ontology::{
        Axiom, Class, ClassExpression, IRI, Individual, NamedIndividual, Ontology,
        axioms::{ClassAssertionAxiom, SubClassOfAxiom},
    },
    reasoning::ReasoningService,
};
use std::sync::Arc;

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

/// Small ontology: A ⊑ B, B ⊑ C, A(i).
fn small_ontology() -> Ontology {
    let mut o = Ontology::new();
    o.add_axiom(sub("A", "B", 1));
    o.add_axiom(sub("B", "C", 2));
    o.add_axiom(assertion("i", "A", 3));
    o
}

// ── 1. Default must stay OFF ─────────────────────────────────────────────────

#[test]
fn default_incremental_reasoning_is_off() {
    let config = ReasonerConfig::default();
    assert!(
        !config.reasoning.incremental_reasoning,
        "incremental_reasoning must default to `false` (A9: opt-in, off by default)"
    );
}

// ── 2. Default full-recompute path is unaffected ────────────────────────────

#[tokio::test]
async fn default_full_recompute_path_is_unaffected() -> Result<()> {
    let config = ReasonerConfig::default();
    assert!(!config.reasoning.incremental_reasoning);

    let service = ReasoningService::new(small_ontology(), config)?;

    // Consistency.
    assert!(service.is_consistent().await?);

    // Classification / entailments: A ⊑ B, B ⊑ C ⇒ A ⊑ C, A(i) ⇒ C(i).
    assert!(service.is_subsumed_by(&class("A"), &class("B")).await?);
    assert!(service.is_subsumed_by(&class("A"), &class("C")).await?);
    assert!(!service.is_subsumed_by(&class("C"), &class("A")).await?);

    let superclasses = service.get_superclasses(&class("A"), false).await?;
    assert!(superclasses.contains(&class("B")));
    assert!(superclasses.contains(&class("C")));

    assert!(
        service
            .is_instance_of(&individual("i"), &class("C"))
            .await?
    );

    Ok(())
}

fn individual(local: &str) -> Individual {
    Individual::Named(NamedIndividual {
        iri: IRI::new(&format!("{EX}{local}")),
    })
}

// ── 3. Opt-in incremental add matches a full recompute ──────────────────────
//
// The incremental service is a wrapper over a base `ReasoningService`. Enabling
// `incremental_reasoning` makes the actor accept `add_axioms` and routes the
// change through the version-stamped full recompute, so every result must agree
// with a freshly built reasoner over the *same* final ontology.

#[tokio::test]
async fn incremental_add_axiom_matches_full_recompute() -> Result<()> {
    let mut config = ReasonerConfig::default();
    config.reasoning.incremental_reasoning = true;

    // Build the incremental path by adding axioms one at a time onto an empty
    // ontology, mirroring the "incremental axiom-add" usage pattern.
    let mut built = Ontology::new();
    let (incremental_service, base_service) = {
        let base = Arc::new(ReasoningService::new(Ontology::new(), config)?);
        let ontology_arc = Arc::new(tokio::sync::RwLock::new(Ontology::new()));
        let incremental = base.clone().into_incremental(ontology_arc, None).await?;

        incremental
            .add_axiom_incrementally(sub("A", "B", 1))
            .await?;
        built.add_axiom(sub("A", "B", 1));

        incremental
            .add_axiom_incrementally(sub("B", "C", 2))
            .await?;
        built.add_axiom(sub("B", "C", 2));

        incremental
            .add_axiom_incrementally(assertion("i", "A", 3))
            .await?;
        built.add_axiom(assertion("i", "A", 3));

        (incremental, base)
    };
    assert_eq!(
        built.axioms().len(),
        3,
        "sanity: final ontology has 3 axioms"
    );

    // Full recompute over the same final axiom set (default config).
    let full = ReasoningService::new(small_ontology(), ReasonerConfig::default())?;

    // Consistency must agree (queried through the base service, which the
    // incremental path routed the added axioms into).
    assert_eq!(
        base_service.is_consistent().await?,
        full.is_consistent().await?,
        "consistency diverged between incremental and full recompute"
    );

    // Classification / entailments must agree.
    let pairs = [
        ("A", "A"),
        ("A", "B"),
        ("A", "C"),
        ("B", "C"),
        ("C", "A"),
        ("B", "A"),
    ];
    for (sub, sup) in pairs {
        assert_eq!(
            incremental_service
                .is_subclass_of(&class(sub), &class(sup))
                .await?,
            full.is_subsumed_by(&class(sub), &class(sup)).await?,
            "subsumption {sub} ⊑ {sup} diverged between incremental and full recompute"
        );
    }

    // Classification (superclass sets) must agree.
    for c in ["A", "B", "C"] {
        let incremental_sups: std::collections::HashSet<String> = incremental_service
            .get_superclasses(&class(c))
            .await?
            .into_iter()
            .map(|c| c.iri.to_string())
            .collect();
        let full_sups: std::collections::HashSet<String> = full
            .get_superclasses(&class(c), false)
            .await?
            .into_iter()
            .filter_map(|ce| match ce {
                ClassExpression::Class(c) => Some(c.iri.to_string()),
                _ => None,
            })
            .collect();
        assert_eq!(
            incremental_sups, full_sups,
            "superclasses of {c} diverged between incremental and full recompute"
        );
    }

    // Instance entailment must agree.
    assert_eq!(
        incremental_service
            .is_instance_of(&individual("i"), &class("C"))
            .await?,
        full.is_instance_of(&individual("i"), &class("C")).await?,
        "instance entailment diverged between incremental and full recompute"
    );

    Ok(())
}

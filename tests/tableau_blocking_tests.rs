//! Integration tests for tableau blocking (workstream A1).
//!
//! These tests verify that:
//! 1. Cyclic existential `TBoxes` (e.g. `A ⊑ ∃R.A`) terminate and stay consistent.
//! 2. Blocking actually marks nodes and prevents re-expansion of blocked nodes.
//! 3. Nominal nodes are never wrongly blocked.
//! 4. The configured `BlockingStrategy` is respected by the executor.

use oxidowl::{
    Result,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::{
        reasoner::Reasoner,
        tableau::{NodeType, Tableau, TableauState},
    },
    ontology::{
        Axiom, Class, ClassAssertionAxiom, ClassExpression, DisjointClassesAxiom,
        EquivalentClassesAxiom, IRI, Individual, NamedIndividual, ObjectProperty,
        ObjectPropertyExpression, Ontology, SubClassOfAxiom,
    },
};
use std::sync::Arc;
use std::time::Duration;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn cls(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn prop(iri: &str) -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: IRI::new(iri) })
}

fn some(p: &str, filler: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectSomeValuesFrom {
        property: prop(p),
        filler: Box::new(filler),
    }
}

fn subclass(sub: ClassExpression, sup: ClassExpression) -> Axiom {
    Axiom::SubClassOf(SubClassOfAxiom {
        id: 1,
        subclass: sub,
        superclass: sup,
        annotations: vec![],
    })
}

fn equiv(a: ClassExpression, b: ClassExpression) -> Axiom {
    Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 1,
        classes: vec![a, b],
        annotations: vec![],
    })
}

fn disjoint(a: ClassExpression, b: ClassExpression) -> Axiom {
    Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 1,
        classes: vec![a, b],
        annotations: vec![],
    })
}

fn assert_class(individual_iri: &str, class: ClassExpression) -> Axiom {
    Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 1,
        class,
        individual: Individual::Named(NamedIndividual {
            iri: IRI::new(individual_iri),
        }),
        annotations: vec![],
    })
}

fn ontology(axioms: Vec<Axiom>) -> Ontology {
    let mut o = Ontology::new();
    for axiom in axioms {
        o.add_axiom(axiom);
    }
    o
}

fn config_with_timeout(strategy: BlockingStrategy, timeout_ms: u64) -> ReasoningConfig {
    ReasoningConfig {
        blocking_strategy: strategy,
        timeout: Some(Duration::from_millis(timeout_ms)),
        ..Default::default()
    }
}

fn build_tableau(strategy: BlockingStrategy, onto: Ontology, timeout_ms: u64) -> Result<Tableau> {
    Tableau::from_ontology(Arc::new(onto), config_with_timeout(strategy, timeout_ms))
}

/// Default blocking strategy: `Anywhere` is the configured default in
/// `ReasoningConfig::default()`. It is sound for SHI-termination: every unblocked
/// node has a label strictly incomparable with (or larger than) all earlier labels,
/// and labels are drawn from the finite set of ontology subexpressions, so the
/// number of unblocked nodes is bounded and blocked nodes never expand.
#[test]
fn default_blocking_strategy_is_anywhere() {
    assert_eq!(
        ReasoningConfig::default().blocking_strategy,
        BlockingStrategy::Anywhere
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Cyclic existential chains
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.A` with an `ABox` individual `a : A`. Without blocking, a cyclic
/// existential chain would expand unboundedly; with blocking it must terminate
/// quickly and report the ontology as consistent.
#[test]
fn cyclic_existential_chain_terminates_and_consistent() -> Result<()> {
    let a = cls("http://example.org/A");
    let onto = ontology(vec![
        subclass(a.clone(), some("http://example.org/R", a.clone())),
        assert_class("http://example.org/a", a.clone()),
    ]);

    let mut tableau = build_tableau(BlockingStrategy::Anywhere, onto, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(
        state,
        TableauState::Satisfiable,
        "A ⊑ ∃R.A with a:A should be consistent and terminate"
    );
    // The expansion must remain bounded — blocking must prevent an unbounded chain.
    assert!(
        tableau.nodes().len() <= 10,
        "node count blew up for cyclic chain: {}",
        tableau.nodes().len()
    );
    Ok(())
}

/// The same cyclic `TBox` must terminate for every supported blocking strategy.
#[test]
fn cyclic_chain_terminates_for_all_strategies() -> Result<()> {
    let a = cls("http://example.org/A");
    let onto = ontology(vec![
        subclass(a.clone(), some("http://example.org/R", a.clone())),
        assert_class("http://example.org/a", a.clone()),
    ]);

    for strategy in [
        BlockingStrategy::Anywhere,
        BlockingStrategy::Ancestor,
        BlockingStrategy::Pairwise,
        BlockingStrategy::Single,
        BlockingStrategy::Core,
        BlockingStrategy::Dynamic,
        BlockingStrategy::Optimal,
        BlockingStrategy::Indexed,
    ] {
        let mut tableau = build_tableau(strategy, onto.clone(), 5000)?;
        assert_eq!(
            tableau.blocking_strategy, strategy,
            "tableau must honour the configured blocking strategy"
        );
        let state = tableau.expand()?;
        assert_eq!(
            state,
            TableauState::Satisfiable,
            "strategy {strategy:?} failed to terminate/be consistent on cyclic chain"
        );
    }
    Ok(())
}

/// `C ⊑ ∃R.C`, `A ⊑ C`: the subclass `A` must remain satisfiable and the whole
/// ontology consistent, terminating within the timeout.
#[test]
fn cyclic_with_subclass_a_satisfiable() -> Result<()> {
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    let onto = ontology(vec![
        subclass(c.clone(), some("http://example.org/R", c.clone())),
        subclass(a.clone(), c.clone()),
    ]);

    let mut r = Reasoner::new(ReasonerConfig {
        reasoning: config_with_timeout(BlockingStrategy::Anywhere, 5000),
        ..Default::default()
    })?;
    r.load_ontology(onto)?;

    assert!(
        r.is_class_satisfiable(&a)?,
        "A should be satisfiable with C ⊑ ∃R.C, A ⊑ C"
    );
    assert!(
        r.is_consistent()?,
        "ontology with C ⊑ ∃R.C, A ⊑ C should be consistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Blocking actually marks nodes
// ──────────────────────────────────────────────────────────────────────────────

/// Two `∃R.C` restrictions rooted at two individuals produce two R-successors
/// with identical labels `{C}`. Anywhere blocking must mark the later successor
/// as blocked by the earlier one, and the expansion must stay bounded.
#[test]
fn identical_successor_is_blocked() -> Result<()> {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    let onto = ontology(vec![
        subclass(a.clone(), some("http://example.org/R", c.clone())),
        subclass(b.clone(), some("http://example.org/R", c.clone())),
        assert_class("http://example.org/a", a),
        assert_class("http://example.org/b", b),
    ]);

    let mut tableau = build_tableau(BlockingStrategy::Anywhere, onto, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let generated: Vec<_> = tableau
        .nodes()
        .iter()
        .filter(|n| n.node_type == NodeType::Generated)
        .collect();
    assert!(
        generated.len() >= 2,
        "expected at least two generated R-successors, got {}",
        generated.len()
    );

    let blocked: Vec<_> = generated.iter().filter(|n| n.is_blocked()).collect();
    assert!(
        !blocked.is_empty(),
        "expected at least one generated node to be blocked; nodes: {:?}",
        tableau
            .nodes()
            .iter()
            .map(|n| (n.id, n.node_type, n.is_blocked(), n.blocking_info.blocker))
            .collect::<Vec<_>>()
    );

    // The blocker must be an earlier generated node (never the root / a nominal).
    for node in blocked {
        let blocker = node
            .blocking_info
            .blocker
            .expect("blocked node must record its blocker");
        assert!(
            blocker < node.id,
            "blocker {} must precede blocked node {}",
            blocker,
            node.id
        );
        assert!(
            matches!(
                tableau.nodes()[blocker].node_type,
                NodeType::Generated | NodeType::Individual
            ),
            "blocker of node {} should be a non-root, non-nominal node",
            node.id
        );
    }
    Ok(())
}

/// Blocking must stop expansion even when the ontology also contains
/// disjointness constraints on the originating classes (a configuration that
/// would otherwise keep generating successors in a non-blocking tableau).
#[test]
fn blocking_vs_disjointness_terminates() -> Result<()> {
    let a = cls("http://example.org/A");
    let b = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    let onto = ontology(vec![
        subclass(a.clone(), some("http://example.org/R", c.clone())),
        subclass(b.clone(), some("http://example.org/R", c.clone())),
        disjoint(a.clone(), b.clone()),
        assert_class("http://example.org/a", a),
        assert_class("http://example.org/b", b),
    ]);

    let mut tableau = build_tableau(BlockingStrategy::Anywhere, onto, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        tableau.nodes().len() <= 10,
        "node count blew up: {}",
        tableau.nodes().len()
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Nominal safety
// ──────────────────────────────────────────────────────────────────────────────

/// Nominal (named-individual) nodes must never be marked blocked, even in a
/// cyclic ontology where generated successors are blocked.
#[test]
fn nominal_nodes_never_blocked() -> Result<()> {
    let a = cls("http://example.org/A");
    let onto = ontology(vec![
        subclass(a.clone(), some("http://example.org/R", a.clone())),
        assert_class("http://example.org/a", a.clone()),
        assert_class("http://example.org/b", a.clone()),
    ]);

    let mut tableau = build_tableau(BlockingStrategy::Anywhere, onto, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let nominals: Vec<_> = tableau
        .nodes()
        .iter()
        .filter(|n| n.node_type == NodeType::Nominal)
        .collect();
    assert!(
        !nominals.is_empty(),
        "expected nominal nodes in the tableau"
    );
    for n in nominals {
        assert!(
            !n.is_blocked(),
            "nominal node {} must never be blocked",
            n.id
        );
    }

    // The chain generated successors (a : A unfolds to ∃R.A), and at least one
    // generated node is blocked, proving the blocking machinery ran alongside
    // the nominal nodes.
    let blocked_generated = tableau
        .nodes()
        .iter()
        .filter(|n| n.node_type == NodeType::Generated && n.is_blocked())
        .count();
    assert!(
        blocked_generated >= 1,
        "expected at least one blocked generated node; nodes: {:?}",
        tableau
            .nodes()
            .iter()
            .map(|n| (n.id, n.node_type, n.is_blocked()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Termination guarantee
// ──────────────────────────────────────────────────────────────────────────────

/// Termination guard: a strongly cyclic `TBox` (`A ≡ ∃R.A` plus `A ⊑ ∃R.A`) with an
/// `ABox` individual is the worst case for unbounded expansion. It must return a
/// completed state (never time out / hang), and do so quickly.
#[test]
fn strongly_cyclic_terminates_within_timeout() -> Result<()> {
    let a = cls("http://example.org/A");
    let some_a = some("http://example.org/R", a.clone());
    let onto = ontology(vec![
        equiv(a.clone(), some_a.clone()),
        subclass(a.clone(), some_a.clone()),
        assert_class("http://example.org/a", a.clone()),
    ]);

    let mut tableau = build_tableau(BlockingStrategy::Anywhere, onto, 3000)?;
    let start = std::time::Instant::now();
    let state = tableau.expand()?;
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "strongly cyclic expansion took too long (potential hang)"
    );
    assert_eq!(
        state,
        TableauState::Satisfiable,
        "strongly cyclic ontology should terminate and be consistent"
    );
    Ok(())
}

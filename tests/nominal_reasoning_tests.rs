//! Integration tests for nominal (O) reasoning (workstream A2).
//!
//! These tests verify that:
//! 1. `ObjectHasValue` (∃R.{a}) creates an R-edge to the existing nominal node for `a`.
//! 2. `∃R.{a}` (`ObjectSomeValuesFrom` with a single-nominal filler) reuses the nominal
//!    node instead of minting a fresh successor, so it interacts correctly with `≤n R.…`.
//! 3. `{a} ⊑ owl:Nothing` is detected as an inconsistency (nominal bottom clash).
//! 4. Nominal-only cycles (`A ⊑ ∃R.{a}`) terminate.
//! 5. Inverse-role names use the canonical `inv(IRI)` form end-to-end.
//! 6. The RL saturation path derives `S(b,a)` from `InverseObjectProperties(R,S)` + `R(a,b)`.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    Result,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::{
        reasoner::Reasoner,
        tableau::{ConceptLabel, Tableau, TableauState},
    },
    ontology::{ClassExpression, Ontology},
};
use std::sync::Arc;
use std::time::Duration;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

const EX: &str = "http://ex.org/";

fn config_with_timeout(strategy: BlockingStrategy, timeout_ms: u64) -> ReasoningConfig {
    ReasoningConfig {
        blocking_strategy: strategy,
        timeout: Some(Duration::from_millis(timeout_ms)),
        ..Default::default()
    }
}

fn build_tableau(onto: Ontology, timeout_ms: u64) -> Result<Tableau> {
    Tableau::from_ontology(
        Arc::new(onto),
        config_with_timeout(BlockingStrategy::Anywhere, timeout_ms),
    )
}

fn reasoner_for(onto: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(ReasonerConfig {
        reasoning: config_with_timeout(BlockingStrategy::Anywhere, 5000),
        ..Default::default()
    })?;
    r.load_ontology(onto)?;
    Ok(r)
}

/// Find the node id whose label set carries `ConceptLabel::Nominal(name)`.
fn nominal_node_id(tableau: &Tableau, name: &str) -> Option<usize> {
    tableau
        .nodes()
        .iter()
        .find(|n| {
            n.concepts
                .iter()
                .any(|c| matches!(c, ConceptLabel::Nominal(x) if x == name))
        })
        .map(|n| n.id)
}

/// Whether the tableau contains an edge from → to with the given canonical role name.
fn has_edge(tableau: &Tableau, from: usize, to: usize, role: &str) -> bool {
    tableau
        .edges()
        .iter()
        .any(|e| e.from == from && e.to == to && e.role.to_string() == role)
}

// ──────────────────────────────────────────────────────────────────────────────
// ObjectHasValue rule
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ObjectHasValue(R, b)`, `A(a)`, `b:B` — the `HAS_VALUE` rule must create an
/// R-edge from the nominal node for `a` to the nominal node for `b`.
#[test]
fn object_has_value_creates_edge_to_nominal() -> Result<()> {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let has_value = df.has_value(df.obj_prop(format!("{EX}R")), df.named(format!("{EX}b")));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), has_value.clone()),
        df.class_assertion(a, df.named(format!("{EX}a"))),
        df.class_assertion(df.class_ce(format!("{EX}B")), df.named(format!("{EX}b"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(
        state,
        TableauState::Satisfiable,
        "A ⊑ ObjectHasValue(R,b), A(a), b:B should be consistent"
    );

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal node for b");
    assert!(
        has_edge(&tableau, a_id, b_id, &format!("{EX}R")),
        "ObjectHasValue must create an R-edge from a's node to b's node; edges: {:?}",
        tableau
            .edges()
            .iter()
            .map(|e| (e.from, e.to, e.role.to_string()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// `A ≡ ∃R.{b}` with `b:B` and `A(a)` — the SOME rule with a single-nominal filler must
/// reuse the existing nominal node for `b` (nominal merging) rather than a fresh node.
#[test]
fn object_some_values_oneof_reuses_nominal() -> Result<()> {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let b_individual = df.named(format!("{EX}b"));
    let some_r_b = df.some_values_from(
        df.obj_prop(format!("{EX}R")),
        df.one_of(vec![b_individual.clone()]),
    );
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a.clone(), some_r_b.clone()]),
        df.class_assertion(a.clone(), df.named(format!("{EX}a"))),
        df.class_assertion(df.class_ce(format!("{EX}B")), b_individual),
    ]);
    df.auto_declare(&mut ont);

    // Reasoner-level: consistent and A satisfiable.
    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ≡ ∃R.{{b}}, b:B, A(a) must be consistent"
    );
    assert!(
        r.is_class_satisfiable(&a)?,
        "A should be satisfiable with A ≡ ∃R.{{b}}"
    );

    // Tableau-level: the R-successor of `a` is exactly the nominal node for `b`.
    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal node for b");
    assert!(
        has_edge(&tableau, a_id, b_id, &format!("{EX}R")),
        "∃R.{{b}} must produce an R-edge from a's node to b's nominal node"
    );

    // No spurious generated node for the nominal filler: the successor IS the nominal.
    let successors = tableau.nodes()[a_id]
        .role_successors
        .get(format!("{EX}R").as_str())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        successors.len(),
        1,
        "a must have exactly one R-successor (the nominal b node)"
    );
    assert!(
        successors.contains(&b_id),
        "the single R-successor must be the nominal node for b"
    );
    Ok(())
}

/// Instance query for `ObjectHasValue(R,b)`: given `a : ObjectHasValue(R,b)` and the
/// `ABox` assertion `R(a,b)`, `get_instances(ObjectHasValue(R,b))` must return `a`.
#[test]
fn object_has_value_instance_query() -> Result<()> {
    let df = DF::new();
    let has_value = df.has_value(df.obj_prop(format!("{EX}R")), df.named(format!("{EX}b")));
    let mut ont = df.build_ontology(vec![
        df.class_assertion(has_value.clone(), df.named(format!("{EX}a"))),
        df.object_property_assertion(
            df.obj_prop(format!("{EX}R")),
            df.named(format!("{EX}a")),
            df.named(format!("{EX}b")),
        ),
        df.class_assertion(df.class_ce(format!("{EX}B")), df.named(format!("{EX}b"))),
    ]);
    df.auto_declare(&mut ont);

    let mut r = reasoner_for(ont)?;
    let instances = r.get_instances(&has_value, false)?;
    assert!(
        instances
            .iter()
            .any(|i| i.iri().map(oxidowl::IRI::as_str) == Some(&format!("{EX}a"))),
        "instance query for ObjectHasValue(R,b) should return a; got: {instances:?}"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Nominal + AtMost interplay
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.{b}`, `A ⊑ ≤1 R.Thing`, `A(a)` — the single R-successor is the nominal `b`
/// node. No spurious nodes are created and the ontology stays consistent.
#[test]
fn nominal_atmost_single_successor() -> Result<()> {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let some_r_b = df.some_values_from(
        df.obj_prop(format!("{EX}R")),
        df.one_of(vec![df.named(format!("{EX}b"))]),
    );
    let atmost = df.max_cardinality(1, df.obj_prop(format!("{EX}R")), df.owl_thing());
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), some_r_b),
        df.sub_class_of(a.clone(), atmost),
        df.class_assertion(a, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ⊑ ∃R.{{b}}, A ⊑ ≤1 R.⊤, A(a) must be consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal node for b");
    let successors = tableau.nodes()[a_id]
        .role_successors
        .get(format!("{EX}R").as_str())
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        successors.len(),
        1,
        "a must have exactly one R-successor; got {successors:?}"
    );
    assert!(
        successors.contains(&b_id),
        "the single R-successor must be the nominal node for b"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Nominal clash detection: {a} ⊑ owl:Nothing
// ──────────────────────────────────────────────────────────────────────────────

/// `ObjectOneOf([a]) ⊑ owl:Nothing` with `a` an individual is an inconsistency.
#[test]
fn nominal_subclass_of_bottom_inconsistent() -> Result<()> {
    let df = DF::new();
    let mut ont = df.build_ontology(vec![df.sub_class_of(
        df.one_of(vec![df.named(format!("{EX}a"))]),
        df.owl_nothing(),
    )]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "ObjectOneOf([a]) ⊑ owl:Nothing must be inconsistent"
    );
    Ok(())
}

/// `{a} ⊑ ¬{a}` — a nominal node required to be in the complement of its own oneOf clashes.
#[test]
fn nominal_and_its_complement_clash() -> Result<()> {
    let df = DF::new();
    let a_individual = df.named(format!("{EX}a"));
    let not_oneof = df.complement_of(df.one_of(vec![a_individual.clone()]));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(df.one_of(vec![a_individual]), not_oneof),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(!r.is_consistent()?, "{{a}} ⊑ ¬{{a}} must be inconsistent");
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Nominal cycle termination
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.{a}` with `a:A` — the existential is witnessed by a self-loop on the nominal
/// node for `a`; expansion must terminate (blocking + nominal reuse).
#[test]
fn nominal_cycle_terminates() -> Result<()> {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let some_r_a = df.some_values_from(
        df.obj_prop(format!("{EX}R")),
        df.one_of(vec![df.named(format!("{EX}a"))]),
    );
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), some_r_a),
        df.class_assertion(a, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let start = std::time::Instant::now();
    let state = tableau.expand()?;
    assert!(
        start.elapsed() < Duration::from_secs(5),
        "nominal cycle A ⊑ ∃R.{{a}}, a:A must terminate quickly"
    );
    assert_eq!(
        state,
        TableauState::Satisfiable,
        "nominal cycle must be consistent and terminate"
    );
    assert!(
        tableau.nodes().len() <= 10,
        "node count blew up for nominal cycle: {}",
        tableau.nodes().len()
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Inverse-role naming (canonical `inv(...)` form)
// ──────────────────────────────────────────────────────────────────────────────

/// The canonical string form of an inverse property expression is `inv(IRI)` — the same
/// form used by the DL-clause layer, `RoleLabel` and the tableau executor's role keys.
#[test]
fn inverse_role_name_is_canonical_inv_form() {
    let df = DF::new();
    let inv = df.inv_obj_prop(format!("{EX}R"));
    assert_eq!(inv.to_string(), format!("inv({EX}R)"));
}

/// `A ⊑ ObjectHasValue(R⁻, b)`, `A(a)` — the `HAS_VALUE` rule must create the edge using
/// the canonical `inv(...)` role name, end-to-end through the tableau.
#[test]
fn inverse_property_object_has_value_edge() -> Result<()> {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let has_value = df.has_value(
        df.inv_obj_prop(format!("{EX}R")),
        df.named(format!("{EX}b")),
    );
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), has_value),
        df.class_assertion(a, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let b_id = nominal_node_id(&tableau, &format!("{EX}b")).expect("nominal node for b");
    assert!(
        has_edge(&tableau, a_id, b_id, &format!("inv({EX}R)")),
        "inverse-property hasValue edge must use the canonical inv(...) role name; edges: {:?}",
        tableau
            .edges()
            .iter()
            .map(|e| (e.from, e.to, e.role.to_string()))
            .collect::<Vec<_>>()
    );
    Ok(())
}

/// `InverseObjectProperties(R,S)` + `ObjectPropertyDomain(S, B)` + `R(a,b)` — the RL
/// saturation path must derive `S(b,a)` (via the inverse rule) and then `b:B` (via the
/// domain rule). This exercises the inverse-role handling end-to-end at the `ABox` level.
#[test]
fn inverse_object_properties_rl_path() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let mut ont = df.build_ontology(vec![
        df.inverse_object_properties(r.clone(), s.clone()),
        df.object_property_domain(s.clone(), b_cls),
        df.object_property_assertion(r, df.named(format!("{EX}a")), df.named(format!("{EX}b"))),
    ]);
    df.auto_declare(&mut ont);

    let mut rl = oxidowl::RLReasoner::new(ReasonerConfig::default());
    rl.initialize(&ont)?;
    rl.materialize()?;

    // S(b,a) is derived by the inverse rule; domain(S) = B then gives b : B.
    let derived = rl.class_assertion_pairs().iter().any(|(ind, classes)| {
        ind.iri().map(oxidowl::IRI::as_str) == Some(&format!("{EX}b"))
            && classes.iter().any(|c| match c {
                ClassExpression::Class(cls) => cls.iri.as_str() == format!("{EX}B"),
                _ => false,
            })
    });
    assert!(
        derived,
        "RL must derive b : B from InverseObjectProperties(R,S), domain(S)=B and R(a,b)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Nominal-aware blocking safety
// ──────────────────────────────────────────────────────────────────────────────

/// A generated node that carries a nominal label must never be blocked, and nominal
/// nodes remain unblocked even in a cyclic ontology.
#[test]
fn nominal_labeled_nodes_never_blocked() -> Result<()> {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let some_r_a = df.some_values_from(
        df.obj_prop(format!("{EX}R")),
        df.one_of(vec![df.named(format!("{EX}a"))]),
    );
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), some_r_a),
        df.class_assertion(a.clone(), df.named(format!("{EX}a"))),
        df.class_assertion(a, df.named(format!("{EX}b"))),
    ]);
    df.auto_declare(&mut ont);

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    for n in tableau.nodes() {
        let has_nominal = n
            .concepts
            .iter()
            .any(|c| matches!(c, ConceptLabel::Nominal(_)));
        if has_nominal {
            assert!(
                !n.is_blocked(),
                "node {} carries a nominal label and must never be blocked",
                n.id
            );
        }
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// CR-008 MAJOR #2 — diff_005 nominal visible on BOTH reasoner surfaces
// ──────────────────────────────────────────────────────────────────────────────

/// `diff_005` ontology: `EquivalentClasses(:A ObjectOneOf(:a))` +
/// `ClassAssertion(:A :x)`.
///
/// `a` occurs *only* inside the nominal, so it is visible solely through the
/// `Ontology::signature()` individual-extraction root fix (CR-008 MAJOR #2).
/// Both the core `Reasoner` **and** the OWL API trait surface
/// (`TableauReasonerFactory` + `OWLReasoner`, whose instance/type maps are
/// built from `realize()` → `signature().individuals`) must see `{a, x}` as
/// instances of `A` and `a : A` as a type.  The QA probe before the root fix
/// showed the OWLAPI surface returning `[x]` / `[]`.
#[test]
fn diff_005_nominal_visible_on_core_and_owlapi_surfaces() -> Result<()> {
    use oxidowl::{OWLReasoner, OWLReasonerConfiguration, ReasonerFactory, TableauReasonerFactory};

    let df = DF::new();
    let a_class = df.class_ce(format!("{EX}A"));
    let nominal_a = df.named(format!("{EX}a"));
    let x = df.named(format!("{EX}x"));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_class.clone(), df.one_of(vec![nominal_a.clone()])]),
        df.class_assertion(a_class.clone(), x.clone()),
    ]);
    df.auto_declare(&mut ont);

    // ── 1. Core surface (`core::reasoner::Reasoner`) ────────────────────────
    let mut core = reasoner_for(ont.clone())?;
    let instances = core.get_instances(&a_class, false)?;
    assert!(
        instances.contains(&nominal_a),
        "core get_instances(A) must include the nominal a; got {instances:?}"
    );
    assert!(
        instances.contains(&x),
        "core get_instances(A) must include x; got {instances:?}"
    );

    let types_a = core.get_types(&nominal_a, false)?;
    assert!(
        types_a.iter().any(|t| matches!(t, ClassExpression::Class(c)
                if c.iri.as_str() == format!("{EX}A"))),
        "core get_types(a) must include A; got {types_a:?}"
    );

    // ── 2. OWL API surface (`TableauReasonerFactory` + `OWLReasoner`) ───────
    let onto: oxidowl::ontology::OntologyRef = Arc::new(std::sync::RwLock::new(ont));
    let owl =
        TableauReasonerFactory.create_reasoner(&onto, &OWLReasonerConfiguration::default())?;

    let owl_instances = owl.get_instances(&a_class, false)?;
    assert!(
        owl_instances.contains_entity(&nominal_a),
        "OWLAPI get_instances(A) must include the nominal a; got {owl_instances:?}"
    );
    assert!(
        owl_instances.contains_entity(&x),
        "OWLAPI get_instances(A) must include x; got {owl_instances:?}"
    );

    let owl_types = owl.get_types(&nominal_a, false)?;
    assert!(
        owl_types.contains_entity(&a_class),
        "OWLAPI get_types(a) must include A; got {owl_types:?}"
    );

    Ok(())
}

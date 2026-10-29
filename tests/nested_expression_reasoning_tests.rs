//! Integration tests for rule re-queueing (workstream A6).
//!
//! Nested class-expression constructors introduced by tableau rules must be re-expanded
//! (completeness for nested expressions). These tests verify that:
//!
//! 1. A nested existential `A ⊑ ∃R.(B ⊓ ∃S.C)` expands fully: the R-successor gets B
//!    (AND re-expansion) and C is reached via the R-then-S path.
//! 2. A nested union under a universal `∀R.(B ⊔ C)` combined with `∃R.¬C` forces the
//!    R-successor to derive B (disjunction + universal interplay).
//! 3. A nested cardinality filler `∃R.(B ⊓ ≥2 S.D)` gives the R-successor B and exactly
//!    two S-successors with D.
//! 4. Deep nesting `∃R.(∃S.(∃T.A))` expands through all three levels to reach A.
//! 5. A nested data existential `∃R.(B ⊓ ∃P.xsd:int)` gives B and a concrete data value
//!    successor via P, without recursively minting spurious data successors.
//! 6. A cyclic nested ontology `C ⊑ ∃R.(C ⊓ D)` terminates (blocking + dedup) and is
//!    consistent.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    Result,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::{
        reasoner::Reasoner,
        tableau::{ConceptLabel, NodeType, Tableau, TableauState},
    },
    ontology::{ClassExpression, DataRange, IRI, Ontology},
};
use std::sync::Arc;
use std::time::Duration;

const EX: &str = "http://ex.org/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn config_with_timeout(timeout_ms: u64) -> ReasoningConfig {
    ReasoningConfig {
        blocking_strategy: BlockingStrategy::Anywhere,
        timeout: Some(Duration::from_millis(timeout_ms)),
        ..Default::default()
    }
}

fn build_tableau(onto: Ontology, timeout_ms: u64) -> Result<Tableau> {
    Tableau::from_ontology(Arc::new(onto), config_with_timeout(timeout_ms))
}

fn reasoner_for(onto: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(ReasonerConfig {
        reasoning: config_with_timeout(5000),
        ..Default::default()
    })?;
    r.load_ontology(onto)?;
    Ok(r)
}

/// Node id of the first node carrying `ConceptLabel::Nominal(name)`.
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

/// The role successors of a node for a canonical role name.
fn successors(tableau: &Tableau, node_id: usize, role: &str) -> Vec<usize> {
    tableau.nodes()[node_id]
        .role_successors
        .get(role)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect()
}

/// Whether a node's label set contains a named class (either as an `Atomic` label or as a
/// `Complex(Class(...))` expression).
fn node_has_class(tableau: &Tableau, node_id: usize, iri: &str) -> bool {
    tableau.nodes()[node_id].concepts.iter().any(|c| match c {
        ConceptLabel::Atomic(name) => name == iri,
        ConceptLabel::Complex(expr) => {
            matches!(&**expr, ClassExpression::Class(cls) if cls.iri.as_str() == iri)
        }
        _ => false,
    })
}

/// Whether a node's label set contains exactly the given complex class expression.
fn node_has_concept(tableau: &Tableau, node_id: usize, ce: &ClassExpression) -> bool {
    let label = ConceptLabel::Complex(Box::new(ce.clone()));
    tableau.nodes()[node_id].concepts.contains(&label)
}

// ──────────────────────────────────────────────────────────────────────────────
// 1. Nested existential: A ⊑ ∃R.(B ⊓ ∃S.C)
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.(B ⊓ ∃S.C)`, `A(a)`. The R-successor must be re-expanded: the AND rule fires
/// on the intersection filler (B, ∃S.C), and the nested ∃S.C must itself create an
/// S-successor carrying C.
#[test]
fn nested_existential_intersection_expands() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));

    let filler = df.intersection_of(vec![
        b_cls.clone(),
        df.some_values_from(s.clone(), c_cls.clone()),
    ]);
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.some_values_from(r.clone(), filler)),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_consistent()?,
        "A ⊑ ∃R.(B ⊓ ∃S.C) with A(a) must be consistent"
    );
    assert!(
        r_reasoner.is_class_satisfiable(&a_cls)?,
        "A must be satisfiable under A ⊑ ∃R.(B ⊓ ∃S.C)"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let r_succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(
        r_succs.len(),
        1,
        "∃R.(B ⊓ ∃S.C) must create exactly one R-successor; got {r_succs:?}"
    );
    let y = r_succs[0];

    // The AND rule must have re-expanded the intersection filler on the successor.
    assert!(
        node_has_class(&tableau, y, &format!("{EX}B")),
        "R-successor must carry B (AND re-expansion of the filler)"
    );
    assert!(
        node_has_concept(&tableau, y, &df.some_values_from(s.clone(), c_cls.clone()),),
        "R-successor must carry the nested ∃S.C"
    );

    // The nested ∃S.C must itself have expanded into an S-successor with C.
    let s_succs = successors(&tableau, y, &format!("{EX}S"));
    assert_eq!(
        s_succs.len(),
        1,
        "nested ∃S.C must create exactly one S-successor; got {s_succs:?}"
    );
    let z = s_succs[0];
    assert!(
        node_has_class(&tableau, z, &format!("{EX}C")),
        "S-successor must carry C (nested existential reached)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 2. Nested in universal: ∀R.(B ⊔ C) + ∃R.¬C
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∀R.(B ⊔ C)` and `A ⊑ ∃R.¬C`. The R-successor receives the union `B ⊔ C` (from the
/// universal, propagated when the existential creates the successor) and `¬C` (the
/// existential filler). Re-queueing must fire the OR rule on the successor so it picks the
/// non-clashing disjunct B.
#[test]
fn universal_union_with_existential_complement_derives_b() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let r = df.obj_prop(format!("{EX}R"));

    let union = df.union_of(vec![b_cls.clone(), c_cls.clone()]);
    let not_c = df.complement_of(c_cls.clone());
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.all_values_from(r.clone(), union)),
        df.sub_class_of(a_cls.clone(), df.some_values_from(r.clone(), not_c.clone())),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_consistent()?,
        "A ⊑ ∀R.(B⊔C), A ⊑ ∃R.¬C with A(a) must be consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let r_succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(
        r_succs.len(),
        1,
        "∃R.¬C must create exactly one R-successor; got {r_succs:?}"
    );
    let y = r_succs[0];

    assert!(
        node_has_concept(&tableau, y, &not_c),
        "R-successor must carry ¬C (the existential filler)"
    );
    assert!(
        node_has_class(&tableau, y, &format!("{EX}B")),
        "R-successor must derive B from ∀R.(B⊔C) via the OR rule (A6 re-queueing)"
    );
    assert!(
        !node_has_class(&tableau, y, &format!("{EX}C")),
        "R-successor must not carry C (would clash with ¬C)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 3. Nested cardinality filler: ∃R.(B ⊓ ≥2 S.D)
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.(B ⊓ ≥2 S.D)`, `A(a)`. The R-successor gets B and `≥2 S.D`; the cardinality
/// rule must fire on the successor and create two S-successors each carrying D.
#[test]
fn nested_cardinality_filler_expands() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let d_cls = df.class_ce(format!("{EX}D"));
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));

    let filler = df.intersection_of(vec![
        b_cls.clone(),
        df.min_cardinality(2, s.clone(), d_cls.clone()),
    ]);
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.some_values_from(r.clone(), filler)),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_consistent()?,
        "A ⊑ ∃R.(B ⊓ ≥2 S.D) with A(a) must be consistent"
    );
    assert!(
        r_reasoner.is_class_satisfiable(&a_cls)?,
        "A must be satisfiable under A ⊑ ∃R.(B ⊓ ≥2 S.D)"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let r_succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(
        r_succs.len(),
        1,
        "∃R.(B ⊓ ≥2 S.D) must create one R-successor"
    );
    let y = r_succs[0];

    assert!(
        node_has_class(&tableau, y, &format!("{EX}B")),
        "R-successor must carry B (AND re-expansion of the filler)"
    );

    let s_succs = successors(&tableau, y, &format!("{EX}S"));
    assert_eq!(
        s_succs.len(),
        2,
        "the nested ≥2 S.D on the R-successor must create two S-successors; got {s_succs:?}"
    );
    let d_label = ConceptLabel::Complex(Box::new(d_cls.clone()));
    for z in s_succs {
        assert!(
            tableau.nodes()[z].concepts.contains(&d_label),
            "each S-successor of ≥2 S.D must carry D"
        );
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 4. Deep nesting: ∃R.(∃S.(∃T.A))
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.(∃S.(∃T.A))`, `A(a)`. Every nesting level must expand: R-successor, then
/// S-successor, then T-successor carrying A.
#[test]
fn deep_nesting_three_levels_expands_to_leaf() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let t = df.obj_prop(format!("{EX}T"));

    let level3 = df.some_values_from(t.clone(), a_cls.clone());
    let level2 = df.some_values_from(s.clone(), level3);
    let level1 = df.some_values_from(r.clone(), level2);
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), level1),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_consistent()?,
        "A ⊑ ∃R.(∃S.(∃T.A)) with A(a) must be consistent"
    );
    assert!(
        r_reasoner.is_class_satisfiable(&a_cls)?,
        "A must be satisfiable under A ⊑ ∃R.(∃S.(∃T.A))"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let r_succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(r_succs.len(), 1, "level 1 (∃R) must create one successor");
    let y = r_succs[0];

    let s_succs = successors(&tableau, y, &format!("{EX}S"));
    assert_eq!(s_succs.len(), 1, "level 2 (∃S) must create one successor");
    let z = s_succs[0];

    let t_succs = successors(&tableau, z, &format!("{EX}T"));
    assert_eq!(t_succs.len(), 1, "level 3 (∃T) must create one successor");
    let w = t_succs[0];

    assert!(
        node_has_class(&tableau, w, &format!("{EX}A")),
        "the T-successor must carry A (all three nesting levels expanded)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 5. Mixed data: ∃R.(B ⊓ ∃P.xsd:int)
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ∃R.(B ⊓ DataSomeValuesFrom(P, xsd:int))`, `A(a)`. The R-successor gets B and a
/// concrete data value successor via P. The range-witness marker on the data node must
/// NOT recursively mint further data successors (A6 datatype guard).
#[test]
fn nested_expression_with_data_some_values_expands() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let r = df.obj_prop(format!("{EX}R"));
    let p = df.data_prop(format!("{EX}P"));
    let int_range: DataRange = DataRange::Datatype(IRI::new(&format!("{XSD}int")));

    let data_some = ClassExpression::DataSomeValuesFrom {
        property: p.clone(),
        filler: int_range.clone(),
    };
    let filler = df.intersection_of(vec![b_cls.clone(), data_some.clone()]);
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.some_values_from(r.clone(), filler)),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_consistent()?,
        "A ⊑ ∃R.(B ⊓ ∃P.xsd:int) with A(a) must be consistent"
    );
    assert!(
        r_reasoner.is_class_satisfiable(&a_cls)?,
        "A must be satisfiable under A ⊑ ∃R.(B ⊓ ∃P.xsd:int)"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);

    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let r_succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(
        r_succs.len(),
        1,
        "∃R.(B ⊓ ∃P.xsd:int) must create one R-successor"
    );
    let y = r_succs[0];

    assert!(
        node_has_class(&tableau, y, &format!("{EX}B")),
        "R-successor must carry B (AND re-expansion of the filler)"
    );

    // The nested data existential must mint a concrete data value successor via P.
    let data_succs = successors(&tableau, y, &format!("{EX}P"));
    assert_eq!(
        data_succs.len(),
        1,
        "the nested ∃P.xsd:int on the R-successor must mint one data successor; got {data_succs:?}"
    );
    let data_node = data_succs[0];
    assert!(
        tableau.nodes()[data_node]
            .concepts
            .iter()
            .any(|c| matches!(c, ConceptLabel::DataValue(_))),
        "the data successor must carry a concrete DataValue literal"
    );

    // The range-witness marker must not recursively mint data successors on the data node.
    let recursive = successors(&tableau, data_node, &format!("{EX}P"));
    assert_eq!(
        recursive.len(),
        0,
        "the range-witness marker must not recursively mint further data successors"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 6. Termination guard: cyclic nested ontology C ⊑ ∃R.(C ⊓ D)
// ──────────────────────────────────────────────────────────────────────────────

/// `C ⊑ ∃R.(C ⊓ D)`, `C(a)`, `C(b)`. Two identical successors `{C ⊓ D}` are created; the
/// second is blocked by the first (label-set dedup + Anywhere blocking), so the expansion
/// terminates quickly, stays bounded and the ontology is consistent. This guards against
/// re-queueing looping on cyclic nested constructors.
#[test]
fn cyclic_nested_expression_terminates_and_consistent() -> Result<()> {
    let df = DF::new();
    let c_cls = df.class_ce(format!("{EX}C"));
    let d_cls = df.class_ce(format!("{EX}D"));
    let r = df.obj_prop(format!("{EX}R"));

    let filler = df.intersection_of(vec![c_cls.clone(), d_cls.clone()]);
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(c_cls.clone(), df.some_values_from(r.clone(), filler)),
        df.class_assertion(c_cls.clone(), df.named(format!("{EX}a"))),
        df.class_assertion(c_cls.clone(), df.named(format!("{EX}b"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_consistent()?,
        "C ⊑ ∃R.(C ⊓ D) with C(a), C(b) must be consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let start = std::time::Instant::now();
    let state = tableau.expand()?;
    assert!(
        start.elapsed() < Duration::from_secs(3),
        "cyclic nested expansion must terminate quickly"
    );
    assert_eq!(state, TableauState::Satisfiable);
    assert!(
        tableau.nodes().len() <= 10,
        "node count must stay bounded (blocking + dedup): {}",
        tableau.nodes().len()
    );

    // The identical successors must be handled by blocking: at least one generated node is
    // blocked, proving that re-queueing did not bypass the blocking guard.
    let blocked_generated = tableau
        .nodes()
        .iter()
        .filter(|n| n.node_type == NodeType::Generated && n.is_blocked())
        .count();
    assert!(
        blocked_generated >= 1,
        "expected at least one blocked generated node (blocking + dedup)"
    );
    Ok(())
}

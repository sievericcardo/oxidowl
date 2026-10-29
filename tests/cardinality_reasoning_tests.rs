//! Integration tests for cardinality reasoning (workstream A3).
//!
//! These tests verify that:
//! 1. `ObjectExactCardinality` (`=n R.C`) is dispatched as `≥n R.C ⊓ ≤n R.C`.
//! 2. Exact cardinality inconsistency (`=1 R.⊤` + `≥2 R.⊤`) makes the class unsatisfiable.
//! 3. `AtMost` merging is non-deterministic and complete: when one merge choice clashes the
//!    executor backtracks and tries the next candidate merge.
//! 4. Data cardinalities (`DataMinCardinality`/`DataMaxCardinality`/`DataExactCardinality`)
//!    create/limit data successors and count them.
//! 5. `≥n R.{a}` with n > 1 is unsatisfiable (single nominal → cardinality clash).
//! 6. `=0 R.C` behaves like `∀R.¬C`.
//! 7. Inequality tracking keeps AtLeast-created successors distinct so `AtMost` cannot merge
//!    witnesses that a `≥n` constraint requires to be distinct.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    Result,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::{
        reasoner::Reasoner,
        tableau::{ConceptLabel, Tableau, TableauState},
    },
    ontology::{ClassExpression, DataRange, Ontology},
};
use std::sync::Arc;
use std::time::Duration;

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

// ──────────────────────────────────────────────────────────────────────────────
// ObjectExactCardinality: `=n R.C ≡ ≥n R.C ⊓ ≤n R.C`
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ =2 R.C` with C non-empty (`c : C`) is consistent and A satisfiable, and the
/// A-individual has exactly two R-successors each carrying C.
#[test]
fn cardinality_reasoning_exact_two_satisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let exact = df.exact_cardinality(2, df.obj_prop(format!("{EX}R")), c_cls.clone());
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), exact.clone()]),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
        // C is non-empty
        df.class_assertion(c_cls.clone(), df.named(format!("{EX}c"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ≡ =2 R.C with C non-empty must be consistent"
    );
    assert!(
        r.is_class_satisfiable(&a_cls)?,
        "A ≡ =2 R.C must make A satisfiable"
    );

    // Tableau-level: exactly two R-successors with filler C on the a node.
    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(
        succs.len(),
        2,
        "=2 R.C must produce exactly two R-successors; got {succs:?}"
    );
    let c_label = ConceptLabel::Complex(Box::new(c_cls));
    for s in succs {
        assert!(
            tableau.nodes()[s].concepts.contains(&c_label),
            "each R-successor of =2 R.C must carry the filler C"
        );
    }
    Ok(())
}

/// `A ≡ =2 R.⊤` together with `A ⊑ ≤2 R.⊤`: the two exact-cardinality witnesses are kept
/// distinct (the ≤-rule never merges them, because the ≥-half marked them `≠`), so the
/// reasoner finds a model with exactly the required two successors.
#[test]
fn cardinality_reasoning_exact_two_not_merged_by_atmost() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let exact = df.exact_cardinality(2, df.obj_prop(format!("{EX}R")), df.owl_thing());
    let atmost = df.max_cardinality(2, df.obj_prop(format!("{EX}R")), df.owl_thing());
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), exact.clone()]),
        df.sub_class_of(a_cls.clone(), atmost),
        df.class_assertion(a_cls, df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ≡ =2 R.⊤, A ⊑ ≤2 R.⊤ must be consistent"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    let a_id = nominal_node_id(&tableau, &format!("{EX}a")).expect("nominal node for a");
    let succs = successors(&tableau, a_id, &format!("{EX}R"));
    assert_eq!(
        succs.len(),
        2,
        "the two =2 R.⊤ witnesses must survive as two distinct successors; got {succs:?}"
    );
    Ok(())
}

/// `=n R.C` where the exact count collides with a stronger `≥`: `A ⊑ =1 R.⊤` and
/// `A ⊑ ≥2 R.⊤` force A to need exactly one *and* at least two distinct R-successors.
/// A becomes unsatisfiable (the ontology itself stays consistent).
#[test]
fn cardinality_reasoning_exact_one_and_atleast_two_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let exact_one = df.exact_cardinality(1, df.obj_prop(format!("{EX}R")), df.owl_thing());
    let atleast_two = df.min_cardinality(2, df.obj_prop(format!("{EX}R")), df.owl_thing());
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), exact_one),
        df.sub_class_of(a_cls.clone(), atleast_two),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "the ontology is consistent (no ABox forces an instance of A)"
    );
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ⊑ =1 R.⊤ and A ⊑ ≥2 R.⊤ must make A unsatisfiable"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// AtMost non-deterministic merge with backtracking
// ──────────────────────────────────────────────────────────────────────────────

/// The KEY completeness test: three R-successors with fillers C, ¬C, B must be reduced to
/// ≤2 by the `≤2 R.⊤` rule. The deterministic first merge choice (C with ¬C) clashes, so the
/// executor must backtrack and pick a different merge pair to find a model.
///
/// `A ⊑ ≥1 R.C`, `A ⊑ ≥1 R.¬C`, `A ⊑ ≥1 R.B`, `A ⊑ ≤2 R.⊤`.
#[test]
fn cardinality_reasoning_atmost_merge_backtracks_to_satisfiable_choice() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let not_c = df.complement_of(c_cls.clone());
    let b_cls = df.class_ce(format!("{EX}B"));
    let r = df.obj_prop(format!("{EX}R"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), c_cls)),
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), not_c)),
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), b_cls)),
        df.sub_class_of(a_cls.clone(), df.max_cardinality(2, r, df.owl_thing())),
        // An ABox witness so the tableau-level check below has a node to expand A on.
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_class_satisfiable(&a_cls)?,
        "A with three successors fillers C, ¬C, B and ≤2 R.⊤ must be satisfiable via a non-clashing merge"
    );

    // Tableau-level: the merge backtracking must actually fire (a clash forced it).
    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(
        state,
        TableauState::Satisfiable,
        "the merged tableau must end up satisfiable"
    );
    assert!(
        tableau.statistics().backtracks >= 1,
        "the first merge choice (C ⊓ ¬C) must clash and force a backtrack; backtracks = {}",
        tableau.statistics().backtracks
    );
    Ok(())
}

/// Soundness of inequality tracking: `A ⊑ ≥3 R.⊤` and `A ⊑ ≤2 R.⊤` are contradictory —
/// the three `≥3` witnesses are pairwise `≠`, so no merge is possible and A is
/// unsatisfiable (`AtMost` must NOT merge two witnesses a ≥ rule requires distinct).
#[test]
fn cardinality_reasoning_atmost_cannot_merge_inequal_witnesses() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(
            a_cls.clone(),
            df.min_cardinality(3, df.obj_prop(format!("{EX}R")), df.owl_thing()),
        ),
        df.sub_class_of(
            a_cls.clone(),
            df.max_cardinality(2, df.obj_prop(format!("{EX}R")), df.owl_thing()),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "the ontology itself is consistent (A has no forced instances)"
    );
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ⊑ ≥3 R.⊤ and A ⊑ ≤2 R.⊤ must make A unsatisfiable (witnesses are pairwise ≠)"
    );
    Ok(())
}

/// Multi-merge (k > 1) backtracking: four successors C, ¬C, E, F reduced to ≤2 R.⊤ needs
/// two merges. The preferred merges each clash (C with ¬C), so the executor must backtrack
/// at the outer merge step and re-resolve the inner step to reach a model.
#[test]
fn cardinality_reasoning_atmost_nested_merges_backtrack() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let not_c = df.complement_of(c_cls.clone());
    let e_cls = df.class_ce(format!("{EX}E"));
    let f_cls = df.class_ce(format!("{EX}F"));
    let r = df.obj_prop(format!("{EX}R"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), c_cls)),
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), not_c)),
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), e_cls)),
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r.clone(), f_cls)),
        df.sub_class_of(a_cls.clone(), df.max_cardinality(2, r, df.owl_thing())),
        df.class_assertion(a_cls.clone(), df.named(format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r_reasoner = reasoner_for(ont.clone())?;
    assert!(
        r_reasoner.is_class_satisfiable(&a_cls)?,
        "four successors (C, ¬C, E, F) reduced to ≤2 R.⊤ must be satisfiable via nested merge backtracking"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(
        state,
        TableauState::Satisfiable,
        "the nested-merge tableau must end up satisfiable"
    );
    assert!(
        tableau.statistics().backtracks >= 2,
        "two clash-forced merge backtracks are expected; backtracks = {}",
        tableau.statistics().backtracks
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Data cardinalities
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ DataMinCardinality 2 P xsd:integer` and `A ⊑ DataMaxCardinality 1 P xsd:integer`
/// force A to need at least two and at most one data successor in the integer range.
/// A is unsatisfiable; the two data witnesses are pairwise `≠` so no merge is possible.
#[test]
fn cardinality_reasoning_data_min_two_and_max_one_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let int_range = df.data_range_integer();

    let data_min: ClassExpression = ClassExpression::DataMinCardinality {
        property: p.clone(),
        cardinality: 2,
        filler: int_range.clone(),
    };
    let data_max: ClassExpression = ClassExpression::DataMaxCardinality {
        property: p.clone(),
        cardinality: 1,
        filler: int_range,
    };
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), data_min),
        df.sub_class_of(a_cls.clone(), data_max),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "the ontology is consistent (no ABox forces an instance of A)"
    );
    // Assert whatever behaviour A3 implements: the rules must fire and counting must work.
    // With inequality tracking, the two data witnesses are distinct, so DataMax(1) clashes.
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ⊑ DataMinCardinality 2 P integer and A ⊑ DataMaxCardinality 1 P integer must make A unsatisfiable"
    );
    Ok(())
}

/// `DataExactCardinality 2 P xsd:integer` alone is satisfiable and creates exactly two
/// data successors (the min and max halves agree).
#[test]
fn cardinality_reasoning_data_exact_two_satisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let p = df.data_prop(format!("{EX}P"));
    let int_range: DataRange = df.data_range_integer();
    let data_exact = ClassExpression::DataExactCardinality {
        property: p,
        cardinality: 2,
        filler: int_range,
    };
    let mut ont = df.build_ontology(vec![df.sub_class_of(a_cls.clone(), data_exact)]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_class_satisfiable(&a_cls)?,
        "A ⊑ DataExactCardinality 2 P integer must be satisfiable"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Nominal + cardinality
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ≥2 R.{a}` is unsatisfiable: a single nominal individual can witness at most one
/// R-successor, so ≥2 forces two distinct successors both equal to `{a}` — impossible.
#[test]
fn cardinality_reasoning_atleast_two_nominal_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let a_ind = df.named(format!("{EX}a"));
    let two_nominal = df.min_cardinality(
        2,
        df.obj_prop(format!("{EX}R")),
        df.one_of(vec![a_ind.clone()]),
    );
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), two_nominal),
        df.class_assertion(a_cls.clone(), a_ind),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ⊑ ≥2 R.{{a}} must be unsatisfiable (single nominal cannot witness two successors)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// =0 R.C behaves like ∀R.¬C
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ =0 R.C` with `A ⊑ ≥1 R.C` is unsatisfiable: the AtMost(0) half forbids any
/// C-successor, the AtLeast(1) half creates one.
#[test]
fn cardinality_reasoning_exact_zero_with_min_one_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let r = df.obj_prop(format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(
            a_cls.clone(),
            df.exact_cardinality(0, r.clone(), c_cls.clone()),
        ),
        df.sub_class_of(a_cls.clone(), df.min_cardinality(1, r, c_cls)),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ⊑ =0 R.C and A ⊑ ≥1 R.C must be unsatisfiable (=0 R.C forbids any C-successor)"
    );
    Ok(())
}

/// `=0 R.C` only restricts *C*-successors: `A ⊑ =0 R.C` together with `A ⊑ ∃R.D` is
/// satisfiable (the D-successor is unaffected), matching `∀R.¬C`.
#[test]
fn cardinality_reasoning_exact_zero_only_restricts_its_filler() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let d_cls = df.class_ce(format!("{EX}D"));
    let r = df.obj_prop(format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.exact_cardinality(0, r.clone(), c_cls)),
        df.sub_class_of(a_cls.clone(), df.some_values_from(r, d_cls)),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_class_satisfiable(&a_cls)?,
        "A ⊑ =0 R.C and A ⊑ ∃R.D must be satisfiable (only C-successors are forbidden)"
    );

    let mut tableau = build_tableau(ont, 5000)?;
    let state = tableau.expand()?;
    assert_eq!(state, TableauState::Satisfiable);
    Ok(())
}

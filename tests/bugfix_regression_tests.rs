//! Consolidated regression tests for the C3/C2 bug-fix wave.
//!
//! One test per verified production bug (workstream: tableau executor,
//! classification, reasoner API, profiles EL/QL/RL validators).  These are
//! self-contained repros; the same scenarios also live in
//! `tests/reasoning_coverage_tests.rs` (C3) and `tests/differential_parity_tests.rs`
//! (C2), but a single file keeps the bug→test mapping explicit.
//!
//! Bugs covered:
//! 1. `bugfix_union_soundness`        — A ⊑ (B ⊔ C), A ⊑ ¬B, A(a) ⇒ consistent, C(a).
//! 2. `bugfix_inverse_bidirectional`  — InverseObjectProperties(R,S) derives both directions.
//! 3. `bugfix_asymmetry_self_loop`    — Asymmetric(R) + ObjectHasSelf(R) ⇒ inconsistent.
//! 4. `bugfix_data_property_domain`   — DataPropertyDomain(dp,C) + dp(a,"x") ⇒ a : C.
//! 5. `bugfix_data_property_range`    — DataPropertyRange(dp,int) + string filler ⇒ inconsistent.
//! 6. `bugfix_sub_data_property_propagation` — P ⊑ Q + P(a,"x") ⇒ Q(a,"x").
//! 7. `bugfix_disjoint_union_instance` — DisjointUnion(A,[B,C]) + B(x) ⇒ A(x).
//! 8. `bugfix_object_one_of_instance` — A ≡ ≥1 R.{b}, A(a) ⇒ R(a,b) / a ∈ ∃R.{b}.
//! 9. `bugfix_el_accepts_disjoint_classes` — OWL 2 EL permits DisjointClasses.
//! 10. `bugfix_ql_rejects_class_datatype_punning` — QL rejects class↔datatype punning.
//! 11. `bugfix_rl_rejects_class_datatype_punning` — RL rejects class↔datatype punning.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    ELValidator, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory, Result,
    TableauReasonerFactory,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::reasoner::Reasoner,
    ontology::{Axiom, Entity, IRI, Ontology},
    profiles::{ProfileValidator, ql::QLValidator, rl::RLValidator},
};
use std::time::Duration;

const EX: &str = "http://ex.org/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn reasoning_config() -> ReasoningConfig {
    ReasoningConfig {
        blocking_strategy: BlockingStrategy::Anywhere,
        timeout: Some(Duration::from_millis(5000)),
        ..Default::default()
    }
}

/// Core tableau-backed `Reasoner` for consistency / satisfiability / instance queries.
fn reasoner_for(onto: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(ReasonerConfig {
        reasoning: reasoning_config(),
        ..Default::default()
    })?;
    r.load_ontology(onto)?;
    Ok(r)
}

/// OWLReasoner-trait reasoner for `is_entailed` / `get_same_individuals`.
fn api_reasoner(onto: Ontology) -> Box<dyn OWLReasoner> {
    let df = DF::new();
    let mut onto = onto;
    df.auto_declare(&mut onto);
    let onto_ref = df.onto_ref(onto);
    let factory = TableauReasonerFactory;
    factory
        .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
        .expect("create reasoner")
}

fn assert_entailed(r: &dyn OWLReasoner, ax: Axiom, msg: &str) {
    let result = r.is_entailed(&ax).expect("is_entailed failed");
    assert!(result, "{msg}: expected entailed, got false");
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 1: UNION SOUNDNESS — `A ⊑ (B ⊔ C)`, `A ⊑ ¬B`, `A(a)` must not clash.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_union_soundness() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let b_cls = df.class_ce(&format!("{EX}B"));
    let c_cls = df.class_ce(&format!("{EX}C"));
    let a = df.named(&format!("{EX}a"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(
            a_cls.clone(),
            df.union_of(vec![b_cls.clone(), c_cls.clone()]),
        ),
        df.sub_class_of(a_cls.clone(), df.complement_of(b_cls)),
        df.class_assertion(a_cls, a.clone()),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        r.is_consistent()?,
        "A ⊑ B⊔C, A ⊑ ¬B, A(a) must be consistent (model: a ∈ C)"
    );
    assert!(
        r.is_instance_of(&a, &c_cls)?,
        "A ⊑ (B ⊔ C) ∧ A ⊑ ¬B ∧ A(a) must entail C(a)"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 2: INVERSE RULE UNIDIRECTIONAL — InverseObjectProperties must derive both
// directions, and a functional inverse must merge individuals.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_inverse_bidirectional() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let s = df.obj_prop(&format!("{EX}S"));
    let a = df.named(&format!("{EX}a"));
    let b = df.named(&format!("{EX}b"));
    let x = df.named(&format!("{EX}x"));

    // Direction 1: R(a,b) must yield S(b,a).
    let ont1 = df.build_ontology(vec![
        df.inverse_object_properties(r.clone(), s.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
    ]);
    let reasoner1 = api_reasoner(ont1);
    assert_entailed(
        reasoner1.as_ref(),
        df.object_property_assertion(s.clone(), b.clone(), a.clone()),
        "InverseObjectProperties(R,S) ∧ R(a,b) ⊨ S(b,a)",
    );

    // Direction 2: S(a,x) must yield R(x,a).
    let ont2 = df.build_ontology(vec![
        df.inverse_object_properties(r.clone(), s.clone()),
        df.object_property_assertion(s.clone(), a.clone(), x.clone()),
    ]);
    let reasoner2 = api_reasoner(ont2);
    assert_entailed(
        reasoner2.as_ref(),
        df.object_property_assertion(r.clone(), x.clone(), a.clone()),
        "InverseObjectProperties(R,S) ∧ S(a,x) ⊨ R(x,a)",
    );

    // Full scenario: functional R + S(a,x) ∧ S(b,x) must merge a ≡ b.
    let ont3 = df.build_ontology(vec![
        df.inverse_object_properties(r.clone(), s.clone()),
        df.functional_object_property(r),
        df.object_property_assertion(s.clone(), a.clone(), x.clone()),
        df.object_property_assertion(s, b.clone(), x),
    ]);
    let reasoner3 = api_reasoner(ont3);
    assert_entailed(
        reasoner3.as_ref(),
        df.same_individual(vec![a, b]),
        "InverseObjectProperties(R,S) ∧ Functional(R) ∧ S(a,x) ∧ S(b,x) ⊨ a ≡ b",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 3: ASYMMETRY SKIPS SELF-LOOPS — asymmetric R + ObjectHasSelf(R) must clash.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_asymmetry_self_loop() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let r = df.obj_prop(&format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.has_self(r.clone())),
        df.asymmetric_object_property(r),
        df.class_assertion(a_cls, df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "Asymmetric(R) ∧ ∃R.Self with an asserted instance must be inconsistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 4: DATA PROPERTY DOMAIN — `Domain(dp, C)` must apply to data edges.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_data_property_domain() -> Result<()> {
    let df = DF::new();
    let c_cls = df.class_ce(&format!("{EX}C"));
    let dp = df.data_prop(&format!("{EX}dp"));
    let a = df.named(&format!("{EX}a"));
    let mut ont = df.build_ontology(vec![
        df.data_property_domain(dp.clone(), c_cls.clone()),
        df.data_property_assertion(
            dp,
            a.clone(),
            df.typed_literal("x", &format!("{XSD}string")),
        ),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        r.is_instance_of(&a, &c_cls)?,
        "DataPropertyDomain(dp,C) ∧ dp(a,\"x\") must put a in C"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 5: DATA PROPERTY RANGE — asserted literals must be validated against the range.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_data_property_range() -> Result<()> {
    let df = DF::new();
    let dp = df.data_prop(&format!("{EX}dp"));
    let a = df.named(&format!("{EX}a"));

    let mut ok_ont = df.build_ontology(vec![
        df.data_property_range(dp.clone(), df.data_range_integer()),
        df.data_property_assertion(
            dp.clone(),
            a.clone(),
            df.typed_literal("5", &format!("{XSD}int")),
        ),
    ]);
    df.auto_declare(&mut ok_ont);
    let r = reasoner_for(ok_ont)?;
    assert!(
        r.is_consistent()?,
        "DataPropertyRange(dp, xsd:integer) with a 5^^xsd:int filler is consistent"
    );

    let mut bad_ont = df.build_ontology(vec![
        df.data_property_range(dp.clone(), df.data_range_integer()),
        df.data_property_assertion(dp, a, df.typed_literal("abc", &format!("{XSD}string"))),
    ]);
    df.auto_declare(&mut bad_ont);
    let r = reasoner_for(bad_ont)?;
    assert!(
        !r.is_consistent()?,
        "DataPropertyRange(dp, xsd:integer) with a string filler must be inconsistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 6: SUB DATA PROPERTY OF — ABox propagation to the super-property.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_sub_data_property_propagation() -> Result<()> {
    let df = DF::new();
    let p = df.data_prop(&format!("{EX}P"));
    let q = df.data_prop(&format!("{EX}Q"));
    let a = df.named(&format!("{EX}a"));
    let lit = df.typed_literal("x", &format!("{XSD}string"));

    let onto = df.build_ontology(vec![
        df.sub_data_property_of(p.clone(), q.clone()),
        df.data_property_assertion(p, a.clone(), lit.clone()),
    ]);
    let reasoner = api_reasoner(onto);

    assert_entailed(
        reasoner.as_ref(),
        df.data_property_assertion(q, a, lit),
        "P ⊑ Q ∧ P(a,\"x\") ⊨ Q(a,\"x\")",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 7: DISJOINT UNION — members are subclasses; instance BFS must follow them.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_disjoint_union_instance() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let b_cls = df.class_ce(&format!("{EX}B"));
    let c_cls = df.class_ce(&format!("{EX}C"));
    let x = df.named(&format!("{EX}x"));
    let mut ont = df.build_ontology(vec![
        df.disjoint_union(a_cls.clone(), vec![b_cls.clone(), c_cls.clone()]),
        df.class_assertion(b_cls.clone(), x.clone()),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        r.is_consistent()?,
        "DisjointUnion(A,[B,C]) with B(x) is consistent"
    );
    assert!(
        r.is_instance_of(&x, &a_cls)?,
        "DisjointUnion(A,[B,C]) ∧ B(x) must put x in A"
    );
    assert!(
        !r.is_class_satisfiable(&df.intersection_of(vec![b_cls, c_cls]))?,
        "DisjointUnion(A,[B,C]) must make B ⊓ C unsatisfiable"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 8: OBJECT ONE OF — `is_instance_of` must use materialized nominal edges.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_object_one_of_instance() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let b_ind = df.named(&format!("{EX}b"));
    let rop = df.obj_prop(&format!("{EX}R"));
    let a = df.named(&format!("{EX}a"));
    let some_r_b = df.some_values_from(rop.clone(), df.one_of(vec![b_ind.clone()]));

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![
            a_cls.clone(),
            df.min_cardinality(1, rop.clone(), df.one_of(vec![b_ind.clone()])),
        ]),
        df.class_assertion(a_cls, a.clone()),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_instance_of(&a, &some_r_b)?,
        "A ≡ ≥1 R.{{b}} ∧ A(a) must make a an instance of ∃R.{{b}}"
    );

    let reasoner = api_reasoner(ont);
    assert_entailed(
        reasoner.as_ref(),
        df.object_property_assertion(rop, a, b_ind),
        "A ≡ ≥1 R.{{b}} ∧ A(a) ⊨ R(a,b)",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Bug 9: EL VALIDATOR — must ACCEPT DisjointClasses.
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn bugfix_el_accepts_disjoint_classes() {
    let df = DF::new();
    let a = df.class_ce(&format!("{EX}A"));
    let b = df.class_ce(&format!("{EX}B"));
    let onto = df.build_ontology(vec![df.disjoint_classes(vec![a, b])]);

    let validator = ELValidator::new();
    let report = validator.validate(&onto).expect("validate must not error");
    assert!(
        report.is_valid(),
        "OWL 2 EL permits DisjointClasses; violations: {:?}",
        report.violations
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Bugs 10/11: QL / RL VALIDATORS — must reject class↔datatype punning.
// ──────────────────────────────────────────────────────────────────────────────

fn punned_class_datatype_ontology() -> Ontology {
    let df = DF::new();
    let iri = IRI::new(&format!("{EX}x"));
    df.build_ontology(vec![
        df.declaration_axiom(Entity::Class(iri.clone())),
        df.declaration_axiom(Entity::Datatype(iri)),
    ])
}

#[test]
fn bugfix_ql_rejects_class_datatype_punning() {
    let onto = punned_class_datatype_ontology();
    let validator = QLValidator::new();
    let report = validator.validate(&onto).expect("validate must not error");
    assert!(
        !report.is_valid(),
        "OWL 2 QL must reject an IRI used as both Class and Datatype"
    );
}

#[test]
fn bugfix_rl_rejects_class_datatype_punning() {
    let onto = punned_class_datatype_ontology();
    let validator = RLValidator::new();
    let report = validator.validate(&onto).expect("validate must not error");
    assert!(
        !report.is_valid(),
        "OWL 2 RL must reject an IRI used as both Class and Datatype"
    );
}

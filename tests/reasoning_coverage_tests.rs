//! Reasoning-level coverage tests for every previously-unreasoned constructor
//! (workstream C3). This is a TEST-WRITING workstream: it closes the audit's
//! biggest blind spot — constructors that were implemented but had no
//! reasoning-level tests exercising them end-to-end through the reasoner.
//!
//! Each test here targets a *gap* (verified by grep against the other test
//! files). Where a scenario was already covered (e.g. `≥2 R.{a}` unsat in
//! `cardinality_reasoning_tests`, `R ∘ S ∘ T` chains in `rbox_reasoning_tests`,
//! HasKey in `semantic_entailment_tests`), no duplicate is added.
//!
//! Groups covered:
//! 1. ObjectHasSelf interactions: `∃R.Self ⊓ Irreflexive(R)` → unsat;
//!    `∃R.Self ⊓ Asymmetric(R)` → unsat.
//! 2. `InverseObjectProperties(R,S)` + `Functional(R)` ⇒ `S` inverse-functional
//!    ⇒ `SameIndividual` inference.
//! 3. `FunctionalObjectProperty` ⇒ `SameIndividual` via `get_same_individuals`
//!    and `is_entailed`.
//! 4. `DataPropertyDomain` / `DataPropertyRange` instance-level reasoning.
//! 5. `DisjointUnion(A,[B,C])`: B(x) → A(x); B ⊓ C unsat.
//! 6. `SubDataPropertyOf`: P ⊑ Q propagates P(a,"x") → Q(a,"x").
//! 7. AnnotationPropertyDomain/Range + SubAnnotationPropertyOf: non-logical,
//!    must not affect consistency and are not entailment-checked.
//! 8. owl:Thing ⊑ A forces everything into A; A ⊑ owl:Nothing makes A unsat.
//! 9. Empty data range `DataOneOf([])` (∃P.[] unsat; ∀P.[] + filler clash).
//! 10. Union-in-antecedent branching: A ⊑ B⊔C, A ⊑ ¬B, A(a) ⇒ C(a).
//! 11. Role hierarchy + transitivity composition: P ⊑ R, Trans(R), P(a,b), P(b,c)
//!     ⇒ R(a,c).
//! 12. Nominal + cardinality: A ≡ ≥1 R.{b} instance-level interaction.
//! 13. Import-driven reasoning: reason over the imports-union.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    OWLReasoner, OWLReasonerConfiguration, ReasonerFactory, Result, TableauReasonerFactory,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::reasoner::Reasoner,
    ontology::{
        Axiom, ClassExpression, IRI, ImportsDeclaration, Ontology,
        axioms::{AnnotationPropertyDomainAxiom, AnnotationPropertyRangeAxiom, AxiomType},
    },
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

/// Core tableau-backed `Reasoner` for consistency / satisfiability / instance
/// and instance-set queries.
fn reasoner_for(onto: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(ReasonerConfig {
        reasoning: config_with_timeout(5000),
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
// 1. ObjectHasSelf interactions
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ ObjectHasSelf(R)` with `Irreflexive(R)` and `A(a)` is inconsistent:
/// the HAS-SELF rule produces a self-loop R(a,a), which irreflexivity forbids.
#[test]
fn self_interaction_has_self_with_irreflexive_inconsistent() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let r = df.obj_prop(&format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.has_self(r.clone())),
        df.irreflexive_object_property(r),
        df.class_assertion(a_cls, df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "∃R.Self ⊓ Irreflexive(R) with an asserted instance must be inconsistent"
    );
    Ok(())
}

/// `A ⊑ ObjectHasSelf(R)` with `Asymmetric(R)` and `A(a)` is inconsistent:
/// the self-loop R(a,a) violates asymmetry (both directions present).
#[test]
fn self_interaction_has_self_with_asymmetric_inconsistent() -> Result<()> {
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
        "∃R.Self ⊓ Asymmetric(R) with an asserted instance must be inconsistent"
    );
    Ok(())
}

/// `A ⊑ ObjectHasSelf(R)` alone (no characteristic axioms) is satisfiable: the
/// self-loop is harmless, so the HAS-SELF rule must not over-clash.
#[test]
fn self_interaction_has_self_alone_satisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let r = df.obj_prop(&format!("{EX}R"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.has_self(r)),
        df.class_assertion(a_cls.clone(), df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        r.is_consistent()?,
        "∃R.Self alone with an asserted instance must be consistent"
    );
    assert!(
        r.is_class_satisfiable(&a_cls)?,
        "A ⊑ ∃R.Self must keep A satisfiable"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 2. InverseObjectProperties + Functional ⇒ inverse-functional ⇒ SameIndividual
// ──────────────────────────────────────────────────────────────────────────────

/// `InverseObjectProperties(R,S)` + `Functional(R)` makes `S` inverse-functional:
/// `S(a,x)`, `S(b,x)` must merge a ≡ b (via R(x,a), R(x,b) + functional R).
#[test]
fn inverse_object_properties_plus_functional_derives_same_individual() -> Result<()> {
    let df = DF::new();
    let r = df.obj_prop(&format!("{EX}R"));
    let s = df.obj_prop(&format!("{EX}S"));
    let a = df.named(&format!("{EX}a"));
    let b = df.named(&format!("{EX}b"));
    let x = df.named(&format!("{EX}x"));

    let onto = df.build_ontology(vec![
        df.inverse_object_properties(r.clone(), s.clone()),
        df.functional_object_property(r),
        df.object_property_assertion(s.clone(), a.clone(), x.clone()),
        df.object_property_assertion(s, b.clone(), x),
    ]);
    let reasoner = api_reasoner(onto);

    assert_entailed(
        reasoner.as_ref(),
        df.same_individual(vec![a.clone(), b.clone()]),
        "InverseObjectProperties(R,S) ∧ Functional(R) ∧ S(a,x) ∧ S(b,x) ⊨ a ≡ b",
    );
    let node = reasoner
        .get_same_individuals(&a)
        .expect("get_same_individuals");
    let contains_b = node
        .get_entities()
        .iter()
        .any(|i| i.iri().map(|x| x.as_str()) == Some(&format!("{EX}b")));
    assert!(
        contains_b,
        "get_same_individuals(a) must include b via inverse-functional S; got {:?}",
        node.get_entities()
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 3. FunctionalObjectProperty ⇒ SameIndividual
// ──────────────────────────────────────────────────────────────────────────────

/// `FunctionalObjectProperty(P)`, `P(a,b)`, `P(a,c)` entails `SameIndividual(b,c)`
/// and `get_same_individuals(b)` must contain c.
#[test]
fn functional_object_property_entails_same_individual() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let a = df.named(&format!("{EX}a"));
    let b = df.named(&format!("{EX}b"));
    let c = df.named(&format!("{EX}c"));

    let onto = df.build_ontology(vec![
        df.functional_object_property(p.clone()),
        df.object_property_assertion(p.clone(), a.clone(), b.clone()),
        df.object_property_assertion(p, a, c.clone()),
    ]);
    let reasoner = api_reasoner(onto);

    assert_entailed(
        reasoner.as_ref(),
        df.same_individual(vec![b.clone(), c.clone()]),
        "Functional(P) ∧ P(a,b) ∧ P(a,c) ⊨ b ≡ c",
    );
    let node = reasoner
        .get_same_individuals(&b)
        .expect("get_same_individuals");
    assert!(
        node.get_entities().contains(&c),
        "get_same_individuals(b) must include c (functional merge); got {:?}",
        node.get_entities()
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 4. DataPropertyDomain / DataPropertyRange instance-level reasoning
// ──────────────────────────────────────────────────────────────────────────────

/// `DataPropertyDomain(dp, C)` + `dp(a, "x")` ⇒ `a : C`.
#[test]
fn data_property_domain_drives_instance() -> Result<()> {
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

/// `DataPropertyRange(dp, xsd:integer)`: a conforming literal keeps the ontology
/// consistent, while a non-integer filler (`"abc"^^xsd:string`) is inconsistent.
#[test]
fn data_property_range_enforced() -> Result<()> {
    let df = DF::new();
    let dp = df.data_prop(&format!("{EX}dp"));
    let a = df.named(&format!("{EX}a"));

    // 5^^xsd:int is within xsd:integer → consistent.
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

    // "abc"^^xsd:string is outside xsd:integer → inconsistent.
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
// 5. DisjointUnion reasoning
// ──────────────────────────────────────────────────────────────────────────────

/// `DisjointUnion(A, [B, C])`, `B(x)` ⇒ `x : A`; and `B ⊓ C` is unsatisfiable.
#[test]
fn disjoint_union_members_are_subclasses_and_pairwise_disjoint() -> Result<()> {
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

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "DisjointUnion(A,[B,C]) with B(x) is consistent"
    );
    assert!(
        r.is_instance_of(&x, &a_cls)?,
        "DisjointUnion(A,[B,C]) ∧ B(x) must put x in A"
    );

    // B ⊓ C is empty because the union members are pairwise disjoint.
    let b_and_c = df.intersection_of(vec![b_cls.clone(), c_cls.clone()]);
    assert!(
        !r.is_class_satisfiable(&b_and_c)?,
        "DisjointUnion(A,[B,C]) must make B ⊓ C unsatisfiable"
    );

    // B and C are each subclasses of A (entailed via is_entailed).
    let reasoner = api_reasoner(ont);
    assert_entailed(
        reasoner.as_ref(),
        df.sub_class_of(b_cls, a_cls.clone()),
        "DisjointUnion(A,[B,C]) ⊨ B ⊑ A",
    );
    assert_entailed(
        reasoner.as_ref(),
        df.sub_class_of(c_cls, a_cls),
        "DisjointUnion(A,[B,C]) ⊨ C ⊑ A",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 6. SubDataPropertyOf ABox propagation
// ──────────────────────────────────────────────────────────────────────────────

/// `SubDataPropertyOf(P, Q)` + `P(a, "x")` entails `DataPropertyAssertion(Q, a, "x")`.
#[test]
fn sub_data_property_of_propagates_data_assertion() -> Result<()> {
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
// 7. AnnotationPropertyDomain / Range / SubAnnotationPropertyOf
// ──────────────────────────────────────────────────────────────────────────────

/// Annotation axioms are non-logical: they must not change consistency, must not
/// be entailment-checkable, and logical reasoning must be unaffected.
#[test]
fn annotation_axioms_do_not_affect_reasoning() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let b_cls = df.class_ce(&format!("{EX}B"));
    let a = df.named(&format!("{EX}a"));
    let ap = df.annotation_property(&format!("{EX}ap"));
    let ap2 = df.annotation_property(&format!("{EX}ap2"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), b_cls.clone()),
        df.class_assertion(a_cls.clone(), a.clone()),
        df.annotation_assertion(ap.clone(), IRI::new(&format!("{EX}a")), "note"),
        df.sub_annotation_property_of(ap.clone(), ap2.clone()),
        Axiom::AnnotationPropertyDomain(AnnotationPropertyDomainAxiom {
            id: df.next_id(),
            property: ap.clone(),
            domain: a_cls.clone(),
            annotations: vec![],
        }),
        Axiom::AnnotationPropertyRange(AnnotationPropertyRangeAxiom {
            id: df.next_id(),
            property: ap,
            range: df.data_range_integer(),
            annotations: vec![],
        }),
    ]);
    df.auto_declare(&mut ont);

    // Consistency is unaffected by the annotation axioms.
    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "annotation axioms must not break consistency"
    );
    assert!(
        r.is_instance_of(&a, &b_cls)?,
        "logical entailment (a:B via A ⊑ B) must hold alongside annotation axioms"
    );

    // The reasoner must not claim entailment support for annotation axiom types.
    let reasoner = api_reasoner(ont);
    for t in [
        AxiomType::AnnotationAssertion,
        AxiomType::SubAnnotationPropertyOf,
        AxiomType::AnnotationPropertyDomain,
        AxiomType::AnnotationPropertyRange,
    ] {
        assert!(
            !reasoner.is_entailment_checking_supported(&t),
            "annotation axiom type {t:?} must not be entailment-checkable"
        );
    }
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 8. owl:Thing / owl:Nothing interplay
// ──────────────────────────────────────────────────────────────────────────────

/// `owl:Thing ⊑ A` forces every individual into A.
#[test]
fn owl_thing_subclass_forces_all_individuals() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let a = df.named(&format!("{EX}a"));
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(df.owl_thing(), a_cls.clone()),
        df.class_assertion(df.owl_thing(), a.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut r = reasoner_for(ont)?;
    assert!(
        r.is_consistent()?,
        "owl:Thing ⊑ A with a:owl:Thing is consistent"
    );
    assert!(
        r.is_instance_of(&a, &a_cls)?,
        "owl:Thing ⊑ A must put every individual (a) in A"
    );
    let instances = r.get_instances(&a_cls, false)?;
    assert!(
        instances
            .iter()
            .any(|i| i.iri().map(|x| x.as_str()) == Some(&format!("{EX}a"))),
        "get_instances(A) must include a when owl:Thing ⊑ A; got {:?}",
        instances
    );
    Ok(())
}

/// `A ⊑ owl:Nothing` makes A unsatisfiable; an asserted instance is inconsistent.
#[test]
fn owl_nothing_superclass_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let mut ont = df.build_ontology(vec![df.sub_class_of(a_cls.clone(), df.owl_nothing())]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ⊑ owl:Nothing alone is consistent (A has no forced instances)"
    );
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ⊑ owl:Nothing must make A unsatisfiable"
    );

    let mut with_instance = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), df.owl_nothing()),
        df.class_assertion(a_cls, df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut with_instance);
    let r = reasoner_for(with_instance)?;
    assert!(
        !r.is_consistent()?,
        "A ⊑ owl:Nothing with a:A must be inconsistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 9. Empty data ranges
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ∃P.DataOneOf([])` is an existential over an empty data range: A can never
/// be witnessed, so A is unsatisfiable and an instance of A is inconsistent.
#[test]
fn empty_data_one_of_exists_unsatisfiable() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let p = df.data_prop(&format!("{EX}P"));
    let empty: ClassExpression = df.data_some_values_from(p.clone(), df.data_range_one_of(vec![]));
    let mut ont = df.build_ontology(vec![df.equivalent_classes(vec![a_cls.clone(), empty])]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ≡ ∃P.DataOneOf([]) with no instance of A is consistent"
    );
    assert!(
        !r.is_class_satisfiable(&a_cls)?,
        "A ≡ ∃P.DataOneOf([]) must make A unsatisfiable (empty value set)"
    );

    let mut with_instance = df.build_ontology(vec![
        df.equivalent_classes(vec![
            a_cls.clone(),
            df.data_some_values_from(p, df.data_range_one_of(vec![])),
        ]),
        df.class_assertion(a_cls, df.named(&format!("{EX}a"))),
    ]);
    df.auto_declare(&mut with_instance);
    let r = reasoner_for(with_instance)?;
    assert!(
        !r.is_consistent()?,
        "A(a) with A ≡ ∃P.DataOneOf([]) must be inconsistent"
    );
    Ok(())
}

/// `A ≡ ∀P.DataOneOf([])` requires every P-filler to lie in the empty set: an
/// asserted P-filler clashes.
#[test]
fn empty_data_one_of_all_values_clashes_on_filler() -> Result<()> {
    let df = DF::new();
    let a_cls = df.class_ce(&format!("{EX}A"));
    let p = df.data_prop(&format!("{EX}P"));
    let a = df.named(&format!("{EX}a"));
    let all: ClassExpression = df.data_all_values_from(p.clone(), df.data_range_one_of(vec![]));
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![a_cls.clone(), all]),
        df.class_assertion(a_cls.clone(), a.clone()),
        df.data_property_assertion(p, a, df.typed_literal("x", &format!("{XSD}string"))),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont)?;
    assert!(
        !r.is_consistent()?,
        "A ≡ ∀P.DataOneOf([]) with a P-filler must be inconsistent"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 10. Union-in-antecedent disjunctive branching
// ──────────────────────────────────────────────────────────────────────────────

/// `A ⊑ (B ⊔ C)`, `A ⊑ ¬B`, `A(a)` ⇒ `a : C`: the tableau must explore the B
/// branch (clash), backtrack, and close on C.
#[test]
fn union_antecedent_branching_entails_other_disjunct() -> Result<()> {
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
        df.class_assertion(a_cls.clone(), a.clone()),
    ]);
    df.auto_declare(&mut ont);

    let r = reasoner_for(ont.clone())?;
    assert!(
        r.is_consistent()?,
        "A ⊑ B⊔C, A ⊑ ¬B, A(a) must be consistent"
    );
    assert!(
        r.is_instance_of(&a, &c_cls)?,
        "A ⊑ (B ⊔ C) ∧ A ⊑ ¬B ∧ A(a) must entail C(a)"
    );

    // The complement negation is essential: without A ⊑ ¬B the branching would
    // not be forced onto C. Check the conjunction of the two subclass axioms is
    // entailed as a unit (both asserted axioms survive entailment checking).
    let reasoner = api_reasoner(ont);
    assert_entailed(
        reasoner.as_ref(),
        df.sub_class_of(
            a_cls,
            df.union_of(vec![c_cls.clone(), df.class_ce(&format!("{EX}B"))]),
        ),
        "A ⊑ (B ⊔ C) asserted ⊨ A ⊑ (B ⊔ C)",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 11. Role hierarchy + transitivity composition
// ──────────────────────────────────────────────────────────────────────────────

/// `P ⊑ R`, `Transitive(R)`, `P(a,b)`, `P(b,c)` ⇒ `R(a,c)` — the P-path first
/// lifts to R, then transitivity closes the R-path.
#[test]
fn role_hierarchy_plus_transitivity_composes() -> Result<()> {
    let df = DF::new();
    let p = df.obj_prop(&format!("{EX}P"));
    let r = df.obj_prop(&format!("{EX}R"));
    let a = df.named(&format!("{EX}a"));
    let b = df.named(&format!("{EX}b"));
    let c = df.named(&format!("{EX}c"));

    let onto = df.build_ontology(vec![
        df.sub_object_property_of(p.clone(), r.clone()),
        df.transitive_object_property(r.clone()),
        df.object_property_assertion(p.clone(), a.clone(), b.clone()),
        df.object_property_assertion(p, b, c.clone()),
    ]);
    let reasoner = api_reasoner(onto);

    assert_entailed(
        reasoner.as_ref(),
        df.object_property_assertion(r, a, c),
        "P ⊑ R ∧ Trans(R) ∧ P(a,b) ∧ P(b,c) ⊨ R(a,c)",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 12. Nominal + cardinality interaction (≥1 R.{b})
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ≥1 R.{b}`, `A(a)` ⇒ `a ∈ ∃R.{b}` and `R(a,b)` is entailed (the ≥1
/// cardinality is witnessed by the single nominal filler b).
#[test]
fn min_cardinality_nominal_instance_interaction() -> Result<()> {
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
        r.is_consistent()?,
        "A ≡ ≥1 R.{{b}} with A(a) must be consistent"
    );
    assert!(
        r.is_instance_of(&a, &some_r_b)?,
        "A ≡ ≥1 R.{{b}} ∧ A(a) must make a an instance of ∃R.{{b}}"
    );

    // The existential is witnessed by the real edge R(a,b).
    let reasoner = api_reasoner(ont);
    assert_entailed(
        reasoner.as_ref(),
        df.object_property_assertion(rop, a, b_ind),
        "A ≡ ≥1 R.{{b}} ∧ A(a) ⊨ R(a,b)",
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 13. Import-driven reasoning
// ──────────────────────────────────────────────────────────────────────────────

/// Reasoning over an ontology that imports another must use the union of both
/// (OWL import semantics). Main: A ⊑ B, a:A; imported: B ⊑ C ⇒ a:C follows from
/// the closure but NOT from the main ontology alone.
#[test]
fn import_driven_reasoning_over_closure_union() -> Result<()> {
    let df = DF::new();
    let base = "http://imp.org/";
    let main_iri = IRI::new(&format!("{base}main"));
    let imported_iri = IRI::new(&format!("{base}imported"));

    let mut imported = Ontology::new();
    imported.set_iri(imported_iri.clone());
    imported.add_axiom(df.sub_class_of(
        df.class_ce(&format!("{base}B")),
        df.class_ce(&format!("{base}C")),
    ));

    let mut main = Ontology::new();
    main.set_iri(main_iri.clone());
    main.imports.push(ImportsDeclaration {
        imported_ontology_iri: imported_iri.clone(),
    });
    main.add_axiom(df.sub_class_of(
        df.class_ce(&format!("{base}A")),
        df.class_ce(&format!("{base}B")),
    ));
    main.add_axiom(df.class_assertion(
        df.class_ce(&format!("{base}A")),
        df.named(&format!("{base}a")),
    ));

    let mut manager = oxidowl::OntologyManager::new();
    let main_ref = Arc::new(std::sync::RwLock::new(main));
    let imported_ref = Arc::new(std::sync::RwLock::new(imported));
    manager.register_ontology(main_ref.clone());
    manager.register_ontology(imported_ref.clone());
    manager.refresh_imports_closure();

    let closure = manager
        .get_imports_closure(&main_ref)
        .expect("imports closure must resolve");
    assert_eq!(
        closure.len(),
        2,
        "imports closure must contain main + imported"
    );

    // Union of the closure (this is what import semantics require reasoning over).
    let mut union = Ontology::new();
    union.set_iri(IRI::new(&format!("{base}union")));
    for ont_ref in &closure {
        let guard = ont_ref.read().unwrap();
        for ax in guard.axioms().to_vec() {
            union.add_axiom(ax);
        }
    }
    df.auto_declare(&mut union);

    let r = reasoner_for(union)?;
    let a = df.named(&format!("{base}a"));
    let c_cls = df.class_ce(&format!("{base}C"));
    assert!(r.is_consistent()?, "the imports-union must be consistent");
    assert!(
        r.is_instance_of(&a, &c_cls)?,
        "reasoning over the imports-union must entail a:C (via imported B ⊑ C)"
    );

    // Negative control: without the imported axiom, a:C must NOT follow.
    let mut main_only = Ontology::new();
    main_only.set_iri(IRI::new(&format!("{base}mainOnly")));
    main_only.add_axiom(df.sub_class_of(
        df.class_ce(&format!("{base}A")),
        df.class_ce(&format!("{base}B")),
    ));
    main_only.add_axiom(df.class_assertion(df.class_ce(&format!("{base}A")), a.clone()));
    df.auto_declare(&mut main_only);
    let r = reasoner_for(main_only)?;
    assert!(
        !r.is_instance_of(&a, &c_cls)?,
        "a:C must NOT be entailed from the main ontology alone (import ignored)"
    );
    Ok(())
}

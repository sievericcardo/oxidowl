//! Semantic entailment tests for every logical axiom type (workstream A7).
//!
//! These drive the reasoner API's `is_entailed` (and, in a couple of spots, the
//! lower-level `Reasoner::check_entailment`) and verify that non-`SubClassOf` /
//! non-`ClassAssertion` axioms are answered *semantically* rather than by
//! explicit-membership only:
//!
//! - Object property axioms: `SubObjectPropertyOf`, `EquivalentObjectProperties`,
//!   `Transitive` / `Symmetric` / `Reflexive` / `Asymmetric` / `Irreflexive`,
//!   `Functional` / `InverseFunctional`, `DisjointObjectProperties`, `Domain`,
//!   `Range` — via completed-tableau closure / witness consistency reductions.
//! - Data property axioms: `SubDataPropertyOf`, `DataPropertyDomain/Range`,
//!   `FunctionalDataProperty`, `DataPropertyAssertion`, negative data assertions
//!   — via the asserted data-property hierarchy closure / data-edge closure.
//! - Individual axioms: `SameIndividual` (including functional/HasKey-derived),
//!   `DifferentIndividuals`, `ObjectPropertyAssertion`,
//!   `NegativeObjectPropertyAssertion`.
//! - Top / bottom role handling for `owl:topObjectProperty`,
//!   `owl:bottomObjectProperty`, `owl:topDataProperty`, `owl:bottomDataProperty`.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    OWLReasoner, OWLReasonerConfiguration, ReasonerFactory, TableauReasonerFactory,
    ontology::{Axiom, ObjectPropertyExpression, Ontology},
};

const EX: &str = "http://ex.org/";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

fn semantic_entailment_top_object_property() -> ObjectPropertyExpression {
    let df = DF::new();
    df.obj_prop(format!("{OWL}topObjectProperty"))
}

fn semantic_entailment_bottom_object_property() -> ObjectPropertyExpression {
    let df = DF::new();
    df.obj_prop(format!("{OWL}bottomObjectProperty"))
}

fn semantic_entailment_top_data_property() -> oxidowl::ontology::DataPropertyExpression {
    let df = DF::new();
    df.data_prop(format!("{OWL}topDataProperty"))
}

fn semantic_entailment_bottom_data_property() -> oxidowl::ontology::DataPropertyExpression {
    let df = DF::new();
    df.data_prop(format!("{OWL}bottomDataProperty"))
}

/// Build a tableau-backed OWL reasoner for the ontology.
fn semantic_entailment_api_reasoner(onto: Ontology) -> Box<dyn OWLReasoner> {
    let df = DF::new();
    let mut onto = onto;
    df.auto_declare(&mut onto);
    let onto_ref = df.onto_ref(onto);
    let factory = TableauReasonerFactory;
    factory
        .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
        .expect("create reasoner")
}

fn semantic_entailment_assert_entailed(r: &dyn OWLReasoner, ax: Axiom, msg: &str) {
    let result = r.is_entailed(&ax).expect("is_entailed failed");
    assert!(result, "{msg}: expected entailed, got false");
}

fn semantic_entailment_assert_not_entailed(r: &dyn OWLReasoner, ax: Axiom, msg: &str) {
    let result = r.is_entailed(&ax).expect("is_entailed failed");
    assert!(!result, "{msg}: expected NOT entailed, got true");
}

// ─────────────────────────────────────────────────────────────────────────────
// SubObjectPropertyOf / EquivalentObjectProperties
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_sub_object_property_of_transitive_closure() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let t = df.obj_prop(format!("{EX}T"));
    let onto = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.sub_object_property_of(s.clone(), t.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);

    // R ⊑ T follows transitively (not asserted literally).
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(r, t.clone()),
        "R ⊑ S ∧ S ⊑ T ⊨ R ⊑ T",
    );
    // Converse does not hold.
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(t, s),
        "R ⊑ S ∧ S ⊑ T ⊭ T ⊑ S",
    );
}

#[test]
fn semantic_entailment_equivalent_object_properties_entailed() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let onto = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.sub_object_property_of(s.clone(), r.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);

    // R ≡ S entailed without an explicit EquivalentObjectProperties axiom.
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.equivalent_object_properties(vec![r, s]),
        "R ⊑ S ∧ S ⊑ R ⊨ R ≡ S",
    );
}

#[test]
fn semantic_entailment_sub_object_property_of_negative() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    // No inclusion between R and S; adding R(w1,w2) must not close to S.
    let onto = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(r, s),
        "∅ ⊭ R ⊑ S",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Object property characteristics
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_transitivity_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.transitive_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.transitive_object_property(r.clone()),
        "Transitive(R) asserted ⊨ Transitive(R)",
    );

    // Not entailed without the axiom: witness R(w1,w2),R(w2,w3) does not close.
    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.transitive_object_property(r),
        "∅ ⊭ Transitive(R)",
    );
}

#[test]
fn semantic_entailment_symmetric_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.symmetric_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.symmetric_object_property(r.clone()),
        "Symmetric(R) asserted ⊨ Symmetric(R)",
    );

    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.symmetric_object_property(r),
        "∅ ⊭ Symmetric(R)",
    );
}

#[test]
fn semantic_entailment_reflexive_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.reflexive_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.reflexive_object_property(r.clone()),
        "Reflexive(R) asserted ⊨ Reflexive(R)",
    );

    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.reflexive_object_property(r),
        "∅ ⊭ Reflexive(R)",
    );
}

#[test]
fn semantic_entailment_asymmetric_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.asymmetric_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.asymmetric_object_property(r.clone()),
        "Asymmetric(R) asserted ⊨ Asymmetric(R)",
    );

    // Not entailed: O ∪ {R(w1,w2), R(w2,w1)} is consistent.
    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.asymmetric_object_property(r),
        "∅ ⊭ Asymmetric(R)",
    );
}

#[test]
fn semantic_entailment_irreflexive_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.irreflexive_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.irreflexive_object_property(r.clone()),
        "Irreflexive(R) asserted ⊨ Irreflexive(R)",
    );

    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.irreflexive_object_property(r),
        "∅ ⊭ Irreflexive(R)",
    );
}

#[test]
fn semantic_entailment_functional_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.functional_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.functional_object_property(r.clone()),
        "Functional(R) asserted ⊨ Functional(R)",
    );

    // Not entailed: O ∪ {R(w1,w2), R(w1,w3), w2≠w3} is consistent.
    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.functional_object_property(r),
        "∅ ⊭ Functional(R)",
    );
}

#[test]
fn semantic_entailment_inverse_functional_entailed_and_not() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));

    let onto = df.build_ontology(vec![df.inverse_functional_object_property(r.clone())]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.inverse_functional_object_property(r.clone()),
        "InverseFunctional(R) asserted ⊨ InverseFunctional(R)",
    );

    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.inverse_functional_object_property(r),
        "∅ ⊭ InverseFunctional(R)",
    );
}

#[test]
fn semantic_entailment_disjoint_object_properties_entailed_and_not() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let q = df.obj_prop(format!("{EX}Q"));

    let onto = df.build_ontology(vec![
        df.disjoint_object_properties(vec![p.clone(), q.clone()]),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.disjoint_object_properties(vec![p.clone(), q.clone()]),
        "Disjoint(P,Q) asserted ⊨ Disjoint(P,Q)",
    );

    // Not entailed: O ∪ {P(w1,w2), Q(w1,w2)} is consistent.
    let reasoner2 = semantic_entailment_api_reasoner(df.build_ontology(vec![]));
    semantic_entailment_assert_not_entailed(
        reasoner2.as_ref(),
        df.disjoint_object_properties(vec![p, q]),
        "∅ ⊭ Disjoint(P,Q)",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Object property domain / range
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_object_property_domain_derived() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let c = df.class_ce(format!("{EX}C"));

    // R ⊑ S ∧ Domain(S,C) ⊨ Domain(R,C): witness w:∃R.⊤ must get C.
    let onto = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.object_property_domain(s, c.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.object_property_domain(r, c),
        "R ⊑ S ∧ Domain(S,C) ⊨ Domain(R,C)",
    );
}

#[test]
fn semantic_entailment_object_property_domain_not_entailed() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let c = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    // R(a,b) alone does not force Domain(R,C).
    let onto = df.build_ontology(vec![df.object_property_assertion(r.clone(), a, b)]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.object_property_domain(r, c),
        "R(a,b) ⊭ Domain(R,C)",
    );
}

#[test]
fn semantic_entailment_object_property_range_derived() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let c = df.class_ce(format!("{EX}C"));

    // R ⊑ S ∧ Range(S,C) ⊨ Range(R,C): witness x:∃R.{w} must put w in C.
    let onto = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.object_property_range(s, c.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.object_property_range(r, c),
        "R ⊑ S ∧ Range(S,C) ⊨ Range(R,C)",
    );
}

#[test]
fn semantic_entailment_object_property_range_not_entailed() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let c = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let onto = df.build_ontology(vec![df.object_property_assertion(r.clone(), a, b)]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.object_property_range(r, c),
        "R(a,b) ⊭ Range(R,C)",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Individual axioms: SameIndividual / DifferentIndividuals / assertions
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_same_individual_via_inverse_functional() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let x = df.named(format!("{EX}x"));

    // InverseFunctional(R), R(a,x), R(b,x) ⊨ SameIndividual(a,b).
    let onto = df.build_ontology(vec![
        df.inverse_functional_object_property(r.clone()),
        df.object_property_assertion(r.clone(), a.clone(), x.clone()),
        df.object_property_assertion(r, b.clone(), x),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.same_individual(vec![a, b]),
        "InverseFunctional(R) ∧ R(a,x) ∧ R(b,x) ⊨ a ≡ b",
    );
}

#[test]
fn semantic_entailment_same_individual_via_haskey() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let c = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let v = df.named(format!("{EX}v"));

    // HasKey(C, [P]), C(a), C(b), P(a,v), P(b,v) ⊨ SameIndividual(a,b).
    let onto = df.build_ontology(vec![
        df.has_key(c.clone(), vec![p.clone()], vec![]),
        df.class_assertion(c.clone(), a.clone()),
        df.class_assertion(c.clone(), b.clone()),
        df.object_property_assertion(p.clone(), a.clone(), v.clone()),
        df.object_property_assertion(p, b.clone(), v),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.same_individual(vec![a, b]),
        "HasKey-driven equality ⊨ a ≡ b",
    );
}

#[test]
fn semantic_entailment_same_individual_not_entailed() {
    let df = DF::new();
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let onto = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.same_individual(vec![a, b]),
        "∅ ⊭ a ≡ b",
    );
}

#[test]
fn semantic_entailment_different_individuals_via_disjoint_classes() {
    let df = DF::new();
    let p = df.class_ce(format!("{EX}P"));
    let q = df.class_ce(format!("{EX}Q"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    // a:P, b:Q, Disjoint(P,Q) ⊨ a ≠ b (a ≡ b would force an individual in P ⊓ Q).
    let onto = df.build_ontology(vec![
        df.class_assertion(p.clone(), a.clone()),
        df.class_assertion(q.clone(), b.clone()),
        df.disjoint_classes(vec![p, q]),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.different_individuals(vec![a, b]),
        "a:P ∧ b:Q ∧ Disjoint(P,Q) ⊨ a ≠ b",
    );
}

#[test]
fn semantic_entailment_different_individuals_not_entailed() {
    let df = DF::new();
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let onto = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.different_individuals(vec![a, b]),
        "∅ ⊭ a ≠ b",
    );
}

#[test]
fn semantic_entailment_object_property_assertion_via_transitivity() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    // Transitive(R), R(a,b), R(b,c) ⊨ R(a,c).
    let onto = df.build_ontology(vec![
        df.transitive_object_property(r.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
        df.object_property_assertion(r.clone(), b, c.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.object_property_assertion(r, a, c),
        "Transitive(R) ∧ R(a,b) ∧ R(b,c) ⊨ R(a,c)",
    );
}

#[test]
fn semantic_entailment_negative_object_property_assertion_via_subproperty() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    // R ⊑ S ∧ ¬S(a,b) ⊨ ¬R(a,b): any R(a,b) would force S(a,b), clashing.
    let onto = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.negative_object_property_assertion(s, a.clone(), b.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.negative_object_property_assertion(r, a, b),
        "R ⊑ S ∧ ¬S(a,b) ⊨ ¬R(a,b)",
    );
}

#[test]
fn semantic_entailment_negative_object_property_assertion_not_entailed() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let onto = df.build_ontology(vec![df.object_property_assertion(
        r.clone(),
        a.clone(),
        b.clone(),
    )]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.negative_object_property_assertion(r, a, b),
        "R(a,b) ⊭ ¬R(a,b)",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Data property axioms
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_sub_data_property_of_closure() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let q = df.data_prop(format!("{EX}Q"));
    let w = df.data_prop(format!("{EX}W"));

    // P ⊑ Q ∧ Q ⊑ W ⊨ P ⊑ W (transitive data-property hierarchy closure).
    let onto = df.build_ontology(vec![
        df.sub_data_property_of(p.clone(), q.clone()),
        df.sub_data_property_of(q.clone(), w.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.sub_data_property_of(p, w),
        "P ⊑ Q ∧ Q ⊑ W ⊨ P ⊑ W",
    );
}

#[test]
fn semantic_entailment_data_property_domain_derived() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let q = df.data_prop(format!("{EX}Q"));
    let c = df.class_ce(format!("{EX}C"));

    // P ⊑ Q ∧ Domain(Q,C) ⊨ Domain(P,C).
    let onto = df.build_ontology(vec![
        df.sub_data_property_of(p.clone(), q.clone()),
        df.data_property_domain(q, c.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.data_property_domain(p, c),
        "P ⊑ Q ∧ Domain(Q,C) ⊨ Domain(P,C)",
    );
}

#[test]
fn semantic_entailment_data_property_range_derived() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let q = df.data_prop(format!("{EX}Q"));
    let rng = df.data_range_integer();

    // P ⊑ Q ∧ Range(Q, xsd:integer) ⊨ Range(P, xsd:integer).
    let onto = df.build_ontology(vec![
        df.sub_data_property_of(p.clone(), q.clone()),
        df.data_property_range(q, rng.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.data_property_range(p, rng),
        "P ⊑ Q ∧ Range(Q,int) ⊨ Range(P,int)",
    );
}

#[test]
fn semantic_entailment_functional_data_property_derived() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let q = df.data_prop(format!("{EX}Q"));

    // P ⊑ Q ∧ Functional(Q) ⊨ Functional(P).
    let onto = df.build_ontology(vec![
        df.sub_data_property_of(p.clone(), q.clone()),
        df.functional_data_property(q),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.functional_data_property(p),
        "P ⊑ Q ∧ Functional(Q) ⊨ Functional(P)",
    );
}

#[test]
fn semantic_entailment_data_property_assertion_via_data_has_value() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let a = df.named(format!("{EX}a"));
    let x = df.typed_literal("x", "http://www.w3.org/2001/XMLSchema#string");
    let a_class = df.class_ce(format!("{EX}A"));

    // A ≡ DataHasValue(P, "x") ∧ A(a) ⊨ P(a, "x")  (the un-ignored C1 case).
    let onto = df.build_ontology(vec![
        df.equivalent_classes(vec![
            a_class.clone(),
            df.data_has_value(p.clone(), x.clone()),
        ]),
        df.class_assertion(a_class, a.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.data_property_assertion(p, a, x),
        "A ≡ DataHasValue(P,\"x\") ∧ A(a) ⊨ P(a,\"x\")",
    );
}

#[test]
fn semantic_entailment_data_property_assertion_not_entailed() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let a = df.named(format!("{EX}a"));
    let x = df.typed_literal("x", "http://www.w3.org/2001/XMLSchema#string");

    let onto = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.data_property_assertion(p, a, x),
        "∅ ⊭ P(a,\"x\")",
    );
}

#[test]
fn semantic_entailment_negative_data_property_assertion_via_all_values() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let a = df.named(format!("{EX}a"));
    let bad = df.typed_literal("abc", "http://www.w3.org/2001/XMLSchema#string");
    let a_class = df.class_ce(format!("{EX}A"));

    // A ≡ ∀P.xsd:integer ∧ A(a) ⊨ ¬P(a, "abc"^^xsd:string).
    let onto = df.build_ontology(vec![
        df.equivalent_classes(vec![
            a_class.clone(),
            df.data_all_values_from(p.clone(), df.data_range_integer()),
        ]),
        df.class_assertion(a_class, a.clone()),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.negative_data_property_assertion(p, a, bad),
        "A ≡ ∀P.int ∧ A(a) ⊨ ¬P(a,\"abc\")",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Top / bottom property handling
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_top_bottom_object_property_inclusions() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let empty = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(empty);

    // R ⊑ owl:topObjectProperty always entailed.
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(r.clone(), semantic_entailment_top_object_property()),
        "∅ ⊨ R ⊑ owl:topObjectProperty",
    );
    // owl:bottomObjectProperty ⊑ R always entailed.
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(semantic_entailment_bottom_object_property(), r.clone()),
        "∅ ⊨ owl:bottomObjectProperty ⊑ R",
    );
    // Converse directions are not entailed.
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(semantic_entailment_top_object_property(), r.clone()),
        "∅ ⊭ owl:topObjectProperty ⊑ R",
    );
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.sub_object_property_of(r, semantic_entailment_bottom_object_property()),
        "∅ ⊭ R ⊑ owl:bottomObjectProperty",
    );
}

#[test]
fn semantic_entailment_top_object_property_characteristics() {
    let df = DF::new();
    let empty = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(empty);
    let top = semantic_entailment_top_object_property();

    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.transitive_object_property(top.clone()),
        "∅ ⊨ Transitive(owl:topObjectProperty)",
    );
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.symmetric_object_property(top.clone()),
        "∅ ⊨ Symmetric(owl:topObjectProperty)",
    );
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.reflexive_object_property(top),
        "∅ ⊨ Reflexive(owl:topObjectProperty)",
    );
}

#[test]
fn semantic_entailment_top_bottom_disjointness() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let empty = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(empty);

    // bottomObjectProperty is disjoint with everything.
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.disjoint_object_properties(vec![
            r.clone(),
            semantic_entailment_bottom_object_property(),
        ]),
        "∅ ⊨ Disjoint(R, owl:bottomObjectProperty)",
    );
    // Nothing is disjoint with topObjectProperty.
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.disjoint_object_properties(vec![r, semantic_entailment_top_object_property()]),
        "∅ ⊭ Disjoint(R, owl:topObjectProperty)",
    );
}

#[test]
fn semantic_entailment_bottom_object_property_functional() {
    let df = DF::new();
    let empty = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(empty);

    // owl:bottomObjectProperty is trivially functional (no edges).
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.functional_object_property(semantic_entailment_bottom_object_property()),
        "∅ ⊨ Functional(owl:bottomObjectProperty)",
    );
}

#[test]
fn semantic_entailment_top_bottom_data_properties() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let empty = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(empty);

    // P ⊑ owl:topDataProperty and owl:bottomDataProperty ⊑ P always entailed.
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.sub_data_property_of(p.clone(), semantic_entailment_top_data_property()),
        "∅ ⊨ P ⊑ owl:topDataProperty",
    );
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.sub_data_property_of(semantic_entailment_bottom_data_property(), p.clone()),
        "∅ ⊨ owl:bottomDataProperty ⊑ P",
    );
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.sub_data_property_of(p.clone(), semantic_entailment_bottom_data_property()),
        "∅ ⊭ P ⊑ owl:bottomDataProperty",
    );
    // owl:bottomDataProperty disjoint with everything, functional, and range-vacuous.
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.disjoint_data_properties(vec![p.clone(), semantic_entailment_bottom_data_property()]),
        "∅ ⊨ Disjoint(P, owl:bottomDataProperty)",
    );
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.functional_data_property(semantic_entailment_bottom_data_property()),
        "∅ ⊨ Functional(owl:bottomDataProperty)",
    );
    semantic_entailment_assert_entailed(
        reasoner.as_ref(),
        df.data_property_range(
            semantic_entailment_bottom_data_property(),
            df.data_range_integer(),
        ),
        "∅ ⊨ Range(owl:bottomDataProperty, int)",
    );
    semantic_entailment_assert_not_entailed(
        reasoner.as_ref(),
        df.disjoint_data_properties(vec![p, semantic_entailment_top_data_property()]),
        "∅ ⊭ Disjoint(P, owl:topDataProperty)",
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// get_same_individuals backed by tableau merges
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_get_same_individuals_uses_tableau_haskey_merge() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let c = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let v = df.named(format!("{EX}v"));

    // HasKey(C, [P]) with shared key value forces a ≡ b in the tableau, but the
    // syntactic IndividualEquality structure has no HasKey rule — so the merge
    // must come from the completed tableau's individual→node map.
    let onto = df.build_ontology(vec![
        df.has_key(c.clone(), vec![p.clone()], vec![]),
        df.class_assertion(c.clone(), a.clone()),
        df.class_assertion(c.clone(), b.clone()),
        df.object_property_assertion(p.clone(), a.clone(), v.clone()),
        df.object_property_assertion(p, b.clone(), v),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);

    let node = reasoner
        .get_same_individuals(&a)
        .expect("get_same_individuals");
    assert!(
        node.get_entities().contains(&b),
        "get_same_individuals(a) must include b via the tableau HasKey merge"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// is_entailed_axioms end-to-end
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_is_entailed_end_to_end() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let c = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let onto = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.object_property_domain(s.clone(), c.clone()),
        df.transitive_object_property(s.clone()),
        df.object_property_assertion(s.clone(), a.clone(), b.clone()),
        df.negative_object_property_assertion(r.clone(), b.clone(), a.clone()),
        df.class_assertion(c.clone(), b.clone()),
        df.different_individuals(vec![a.clone(), b.clone()]),
    ]);
    let reasoner = semantic_entailment_api_reasoner(onto);

    let entailed = vec![
        // Derived via the RBox closure.
        df.sub_object_property_of(r.clone(), s.clone()),
        df.object_property_domain(r.clone(), c.clone()),
        // Derived domain of R (via R ⊑ S + Domain(S,C)).
        df.transitive_object_property(s.clone()),
        df.object_property_assertion(s.clone(), a.clone(), b.clone()),
        df.negative_object_property_assertion(r.clone(), b.clone(), a.clone()),
        df.different_individuals(vec![a.clone(), b.clone()]),
    ];

    let result = reasoner
        .is_entailed_axioms(&entailed)
        .expect("is_entailed_axioms");
    assert!(result, "all listed axioms should be entailed");
}

// ─────────────────────────────────────────────────────────────────────────────
// is_entailment_checking_supported coverage
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn semantic_entailment_entailment_checking_supported_coverage() {
    use oxidowl::ontology::axioms::AxiomType;
    let df = DF::new();
    let onto = df.build_ontology(vec![]);
    let reasoner = semantic_entailment_api_reasoner(onto);

    for t in [
        AxiomType::SubObjectPropertyOf,
        AxiomType::EquivalentObjectProperties,
        AxiomType::TransitiveObjectProperty,
        AxiomType::SymmetricObjectProperty,
        AxiomType::ReflexiveObjectProperty,
        AxiomType::AsymmetricObjectProperty,
        AxiomType::IrreflexiveObjectProperty,
        AxiomType::FunctionalObjectProperty,
        AxiomType::InverseFunctionalObjectProperty,
        AxiomType::DisjointObjectProperties,
        AxiomType::ObjectPropertyDomain,
        AxiomType::ObjectPropertyRange,
        AxiomType::SubDataPropertyOf,
        AxiomType::EquivalentDataProperties,
        AxiomType::DisjointDataProperties,
        AxiomType::DataPropertyDomain,
        AxiomType::DataPropertyRange,
        AxiomType::FunctionalDataProperty,
        AxiomType::SameIndividual,
        AxiomType::DifferentIndividuals,
        AxiomType::ObjectPropertyAssertion,
        AxiomType::DataPropertyAssertion,
        AxiomType::NegativeObjectPropertyAssertion,
        AxiomType::NegativeDataPropertyAssertion,
        AxiomType::ClassAssertion,
        AxiomType::SubClassOf,
        AxiomType::EquivalentClasses,
    ] {
        assert!(
            reasoner.is_entailment_checking_supported(&t),
            "is_entailment_checking_supported should cover {t:?}"
        );
    }

    // Non-logical types remain unsupported.
    assert!(!reasoner.is_entailment_checking_supported(&AxiomType::AnnotationAssertion));
    assert!(!reasoner.is_entailment_checking_supported(&AxiomType::Rule));
}

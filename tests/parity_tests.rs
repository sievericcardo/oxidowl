//! Covers the gaps closed by the parity wave:
//!
//! 1. `get_object_property_values` / `get_data_property_values` answer from the
//!    **materialised tableau closure**, not just the asserted axioms — inverse
//!    edges, sub-property edges, transitive / property-chain derived edges and
//!    deduplicated data values;
//! 2. the trait-level helpers `has_type`, `is_same_individual`,
//!    `has_object_property_relationship`, `has_data_property_relationship` and
//!    `get_object_property_instances`;
//! 3. `get_different_individuals` also fires on **disjoint-type entailment**
//!    (`A ⊑ ¬B`, `A(a)`, `B(b)` ⇒ `a ≠ b`) while still respecting `SameAs`
//!    merging;
//! 4. `precompute_inferences` really warms the requested caches and
//!    `is_precomputed` reflects exactly what was requested.

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ontology::*;
use oxidowl::{
    InferenceType, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory, TableauReasonerFactory,
};
use std::sync::Arc;

const EX: &str = "http://ex.org/";

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(std::sync::RwLock::new(o))
}

fn tableau_reasoner(ont: Ontology) -> Box<dyn OWLReasoner> {
    let onto = onto_ref(ont);
    TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .expect("tableau reasoner must be created")
}

fn named(individual: &Individual) -> NamedIndividual {
    match individual {
        Individual::Named(n) => n.clone(),
        other => panic!("expected a named individual, got {other:?}"),
    }
}

/// Sorted IRI strings of `get_object_property_values`.
fn op_values(
    reasoner: &dyn OWLReasoner,
    individual: &Individual,
    property: &ObjectPropertyExpression,
) -> Vec<String> {
    let mut values: Vec<String> = reasoner
        .get_object_property_values(&named(individual), property)
        .expect("get_object_property_values")
        .get_flattened()
        .iter()
        .map(|n| n.iri.to_string())
        .collect();
    values.sort();
    values
}

/// Sorted literals of `get_data_property_values`.
fn dp_values(
    reasoner: &dyn OWLReasoner,
    individual: &Individual,
    property: &DataPropertyExpression,
) -> Vec<Literal> {
    let mut values: Vec<Literal> = reasoner
        .get_data_property_values(&named(individual), property)
        .expect("get_data_property_values")
        .get_flattened()
        .into_iter()
        .collect();
    values.sort_by_key(|l| format!("{l:?}"));
    values
}

// ══════════════════════════════════════════════════════════════════════════════
// Object property values
// ══════════════════════════════════════════════════════════════════════════════

/// Asserted edge: `R(a, b)` ⇒ `values(a, R) = {b}`; an individual with no
/// `R` edge gets the empty set (negative control).
#[test]
fn object_property_values_asserted_edge_with_negative_control() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![df.object_property_assertion(
        r.clone(),
        a.clone(),
        b.clone(),
    )]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert_eq!(op_values(&*reasoner, &a, &r), vec![format!("{EX}b")]);
    // Negative controls: c has no outgoing R edge, and a has no successor c.
    assert!(
        op_values(&*reasoner, &c, &r).is_empty(),
        "c has no R edge, the value set must be empty"
    );
    assert!(
        !op_values(&*reasoner, &a, &r).contains(&format!("{EX}c")),
        "c must not leak into a's R values"
    );
}

/// Sub-property: `R ⊑ S`, `R(a, b)` ⇒ `values(a, S)` contains `b` (the edge is
/// labelled `R` in the tableau; the query matches through the sub-property
/// closure).
#[test]
fn object_property_values_sub_property_edge() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(r.clone(), s.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert_eq!(
        op_values(&*reasoner, &a, &s),
        vec![format!("{EX}b")],
        "R ⊑ S and R(a,b) must answer an S query with b"
    );
    // The sub-property itself is unaffected, and c stays unrelated.
    assert_eq!(op_values(&*reasoner, &a, &r), vec![format!("{EX}b")]);
    assert!(op_values(&*reasoner, &c, &s).is_empty());
}

/// Inverse: `R(a, b)` ⇒ `values(b, inv(R)) = {a}` and `values(a, R) = {b}`;
/// neither direction leaks the other way.
#[test]
fn object_property_values_inverse_property() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let inv_r = df.inv_obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let mut ont = df.build_ontology(vec![df.object_property_assertion(
        r.clone(),
        a.clone(),
        b.clone(),
    )]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert_eq!(
        op_values(&*reasoner, &b, &inv_r),
        vec![format!("{EX}a")],
        "R(a,b) must answer an inv(R) query on b with a"
    );
    assert_eq!(op_values(&*reasoner, &a, &r), vec![format!("{EX}b")]);
    // Negative controls: b has no outgoing R successor and a no incoming one.
    assert!(op_values(&*reasoner, &b, &r).is_empty());
    assert!(op_values(&*reasoner, &a, &inv_r).is_empty());
}

/// Chain-derived edge: `R∘T ⊑ S` with `R(a,b)`, `T(b,c)` materialises
/// `S(a, c)` in the completed tableau — no `S` assertion exists.
#[test]
fn object_property_values_chain_derived_from_tableau_snapshot() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let t = df.obj_prop(format!("{EX}T"));
    let s = df.obj_prop(format!("{EX}S"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let chain = ObjectPropertyExpression::PropertyChain(vec![r.clone(), t.clone()]);
    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(chain, s.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
        df.object_property_assertion(t.clone(), b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert_eq!(
        op_values(&*reasoner, &a, &s),
        vec![format!("{EX}c")],
        "R∘T ⊑ S with R(a,b), T(b,c) must yield S(a,c) from the snapshot"
    );
    // Negative control: c has no S successor of its own.
    assert!(op_values(&*reasoner, &c, &s).is_empty());
}

/// Transitive edge: `Transitive(R)`, `R(a,b)`, `R(b,c)` ⇒ `R(a,c)`.
#[test]
fn object_property_values_transitive_edge() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));
    let d = df.named(format!("{EX}d"));

    let mut ont = df.build_ontology(vec![
        df.transitive_object_property(r.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
        df.object_property_assertion(r.clone(), b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let values = op_values(&*reasoner, &a, &r);
    assert!(
        values.contains(&format!("{EX}b")) && values.contains(&format!("{EX}c")),
        "transitivity must derive R(a,c); got {values:?}"
    );
    assert!(
        !values.contains(&format!("{EX}d")),
        "d is unrelated and must not appear; got {values:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Data property values
// ══════════════════════════════════════════════════════════════════════════════

/// Asserted value + sub-data-property + deduplication:
/// `P ⊑ Q`, `P(a, 1)`, `Q(a, 2)` ⇒ `values(a, Q) = {1, 2}` and
/// `values(a, P) = {1}` — the asserted scan *and* the snapshot both produce
/// `1` for the direct `P` query, so an un-deduplicated answer would be `{1,1}`.
#[test]
fn data_property_values_asserted_sub_property_and_dedupe() {
    let df = DF::new();
    let p = df.data_prop(format!("{EX}P"));
    let q = df.data_prop(format!("{EX}Q"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let one = df.int_literal(1);
    let two = df.int_literal(2);

    let mut ont = df.build_ontology(vec![
        df.sub_data_property_of(p.clone(), q.clone()),
        df.data_property_assertion(p.clone(), a.clone(), one.clone()),
        df.data_property_assertion(q.clone(), a.clone(), two.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let direct = dp_values(&*reasoner, &a, &p);
    assert_eq!(
        direct,
        vec![one.clone()],
        "the direct P query must return 1 exactly once (asserted scan + snapshot dedup)"
    );

    let via_super = dp_values(&*reasoner, &a, &q);
    assert!(
        via_super.contains(&one) && via_super.contains(&two),
        "P ⊑ Q must surface both P(a,1) and Q(a,2); got {via_super:?}"
    );
    assert_eq!(via_super.len(), 2, "no duplicate values; got {via_super:?}");

    // Negative control: b asserts no data property at all.
    assert!(
        dp_values(&*reasoner, &b, &q).is_empty(),
        "b has no Q/P value"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// has_type / is_same_individual / has_*_relationship
// ══════════════════════════════════════════════════════════════════════════════

/// `A ⊑ B`, `A(a)` ⇒ `has_type(a, A)` and `has_type(a, B)` are true,
/// `has_type(a, C)` is false (negative control).
#[test]
fn has_type_positive_and_negative() {
    let df = DF::new();
    let a_class = df.class_ce(format!("{EX}A"));
    let b_class = df.class_ce(format!("{EX}B"));
    let c_class = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_class.clone(), b_class.clone()),
        df.class_assertion(a_class.clone(), a.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert!(reasoner.has_type(&a, &a_class).expect("has_type A"));
    assert!(
        reasoner.has_type(&a, &b_class).expect("has_type B"),
        "A ⊑ B and A(a) must make has_type(a, B) true"
    );
    assert!(
        !reasoner.has_type(&a, &c_class).expect("has_type C"),
        "negative control: a is not a C"
    );
}

/// `SameIndividual(a, b)` ⇒ `is_same_individual(a, b)` (and symmetrically);
/// an unrelated `c` is not the same (negative control).
#[test]
fn is_same_individual_positive_and_negative() {
    let df = DF::new();
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![df.same_individual(vec![a.clone(), b.clone()])]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert!(
        reasoner
            .is_same_individual(&a, &b)
            .expect("is_same_individual(a, b)")
    );
    assert!(
        reasoner
            .is_same_individual(&b, &a)
            .expect("is_same_individual(b, a)")
    );
    assert!(
        !reasoner
            .is_same_individual(&a, &c)
            .expect("is_same_individual(a, c)"),
        "negative control: c is unrelated"
    );
}

/// Relationship helpers: `R(a, b)` and `P(a, 1)` make the positive probes true
/// and the unrelated probes false.
#[test]
fn has_relationship_positive_and_negative() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let p = df.data_prop(format!("{EX}P"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
        df.data_property_assertion(p.clone(), a.clone(), df.int_literal(7)),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert!(
        reasoner
            .has_object_property_relationship(&named(&a), &r, &named(&b))
            .expect("has_object_property_relationship(R, a, b)"),
        "asserted R(a,b) must be reported"
    );
    assert!(
        !reasoner
            .has_object_property_relationship(&named(&a), &r, &named(&c))
            .expect("has_object_property_relationship(R, a, c)"),
        "negative control: no R(a,c)"
    );
    assert!(
        !reasoner
            .has_object_property_relationship(&named(&a), &s, &named(&b))
            .expect("has_object_property_relationship(S, a, b)"),
        "negative control: S has no assertion at all"
    );

    assert!(
        reasoner
            .has_data_property_relationship(&named(&a), &p, &df.int_literal(7))
            .expect("has_data_property_relationship(P, a, 7)"),
        "asserted P(a,7) must be reported"
    );
    assert!(
        !reasoner
            .has_data_property_relationship(&named(&a), &p, &df.int_literal(8))
            .expect("has_data_property_relationship(P, a, 8)"),
        "negative control: P(a,8) is not asserted"
    );
}

/// `get_object_property_instances` returns every materialised pair for the
/// property (and, for the inverse expression, every pair in the opposite
/// direction) while excluding pairs of unrelated properties.
#[test]
fn get_object_property_instances_returns_all_materialised_pairs() {
    let df = DF::new();
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let inv_r =
        ObjectPropertyExpression::InverseObjectProperty(df.object_property(format!("{EX}R")));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));
    let d = df.named(format!("{EX}d"));
    let e = df.named(format!("{EX}e"));

    let mut ont = df.build_ontology(vec![
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
        df.object_property_assertion(r.clone(), a.clone(), c.clone()),
        df.object_property_assertion(s.clone(), d.clone(), e.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let pairs = reasoner
        .get_object_property_instances(&r)
        .expect("get_object_property_instances(R)");
    let rendered: Vec<String> = pairs
        .iter()
        .map(|(src, tgt)| format!("{} {}", src.iri, tgt.iri))
        .collect();
    assert_eq!(
        rendered,
        vec![format!("{EX}a {EX}b"), format!("{EX}a {EX}c")],
        "all R pairs, sorted; S(d,e) must be excluded"
    );

    let inverse = reasoner
        .get_object_property_instances(&inv_r)
        .expect("get_object_property_instances(inv(R))");
    let inverse_rendered: Vec<String> = inverse
        .iter()
        .map(|(src, tgt)| format!("{} {}", src.iri, tgt.iri))
        .collect();
    assert_eq!(
        inverse_rendered,
        vec![format!("{EX}b {EX}a"), format!("{EX}c {EX}a")],
        "inv(R) must report the pairs in the opposite direction"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// get_different_individuals
// ══════════════════════════════════════════════════════════════════════════════

/// Explicit `DifferentIndividuals(a, b)` ⇒ `b ∈ different(a)` and
/// `a ∈ different(b)`; `c` stays out (negative control).
#[test]
fn get_different_individuals_explicit_axiom() {
    let df = DF::new();
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![df.different_individuals(vec![a.clone(), b.clone()])]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let diff_a = reasoner
        .get_different_individuals(&a)
        .expect("get_different_individuals(a)");
    assert!(diff_a.contains_entity(&b), "explicit DifferentIndividuals");
    assert!(
        !diff_a.contains_entity(&c),
        "negative control: c is not declared different"
    );

    let diff_b = reasoner
        .get_different_individuals(&b)
        .expect("get_different_individuals(b)");
    assert!(diff_b.contains_entity(&a), "different is symmetric");
}

/// Disjoint-type entailment: `A ⊑ ¬B`, `A(a)`, `B(b)` ⇒ `a` different from
/// `b` even without any `DifferentIndividuals` axiom. `c : C` (no disjointness
/// with `A`) must stay out — negative control.
#[test]
fn get_different_individuals_from_disjoint_types() {
    let df = DF::new();
    let a_class = df.class_ce(format!("{EX}A"));
    let b_class = df.class_ce(format!("{EX}B"));
    let c_class = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a_class.clone(), df.complement_of(b_class.clone())),
        df.class_assertion(a_class.clone(), a.clone()),
        df.class_assertion(b_class.clone(), b.clone()),
        df.class_assertion(c_class.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let diff_a = reasoner
        .get_different_individuals(&a)
        .expect("get_different_individuals(a)");
    assert!(
        diff_a.contains_entity(&b),
        "A(a) + B(b) + A ⊑ ¬B must entail a ≠ b"
    );
    assert!(
        !diff_a.contains_entity(&c),
        "negative control: C is not disjoint with A"
    );
    assert!(
        !diff_a.contains_entity(&a),
        "an individual is never different from itself"
    );

    let diff_b = reasoner
        .get_different_individuals(&b)
        .expect("get_different_individuals(b)");
    assert!(
        diff_b.contains_entity(&a),
        "disjoint-type difference must be symmetric"
    );
}

/// Same-as merging: `SameIndividual(a, b)` + `DifferentIndividuals(a, c)`
/// ⇒ `c` is different from *both* `a` and the merged `b`, and `a`/`b` are
/// never reported different from each other.
#[test]
fn get_different_individuals_respects_same_as_merging() {
    let df = DF::new();
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));

    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![a.clone(), b.clone()]),
        df.different_individuals(vec![a.clone(), c.clone()]),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let diff_b = reasoner
        .get_different_individuals(&b)
        .expect("get_different_individuals(b)");
    assert!(
        diff_b.contains_entity(&c),
        "b is merged with a, so c is different from b too"
    );
    assert!(
        !diff_b.contains_entity(&a),
        "a and b are the same individual and must never be different"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// precompute_inferences / is_precomputed
// ══════════════════════════════════════════════════════════════════════════════

/// Requested inference types report as precomputed after a successful
/// `precompute_inferences`; a type that was *not* requested stays `false`.
#[test]
fn precompute_inferences_warms_requested_types_only() {
    let df = DF::new();
    let a_class = df.class_ce(format!("{EX}A"));
    let r = df.obj_prop(format!("{EX}R"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a_class.clone(), a.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    // Nothing has been precomputed yet.
    assert!(!reasoner.is_precomputed(InferenceType::ClassHierarchy));
    assert!(!reasoner.is_precomputed(InferenceType::ObjectPropertyHierarchy));
    assert!(!reasoner.is_precomputed(InferenceType::ClassAssertions));
    assert!(!reasoner.is_precomputed(InferenceType::SameIndividual));

    reasoner
        .precompute_inferences(&[
            InferenceType::ClassHierarchy,
            InferenceType::ObjectPropertyHierarchy,
            InferenceType::ClassAssertions,
        ])
        .expect("precompute_inferences");

    assert!(reasoner.is_precomputed(InferenceType::ClassHierarchy));
    assert!(reasoner.is_precomputed(InferenceType::ObjectPropertyHierarchy));
    assert!(reasoner.is_precomputed(InferenceType::ClassAssertions));
    // Not requested ⇒ not precomputed.
    assert!(
        !reasoner.is_precomputed(InferenceType::SameIndividual),
        "SameIndividual was not requested"
    );
    assert!(
        !reasoner.is_precomputed(InferenceType::DifferentIndividuals),
        "DifferentIndividuals was not requested"
    );
    assert!(
        !reasoner.is_precomputed(InferenceType::PropertyAssertions),
        "PropertyAssertions was not requested"
    );

    // The advertised precomputable set is unchanged (8 types).
    assert_eq!(reasoner.get_precomputable_inference_types().len(), 8);
}

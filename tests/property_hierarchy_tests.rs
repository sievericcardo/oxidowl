//! Inferred object/data property hierarchy parity tests (OWL API / HermiT).
//!
//! Covers the final-wave gap A:
//!
//! 1. `getSuperObjectProperties` / `getSubObjectProperties` (and the data
//!    counterparts) answer from the **transitive strict closure** —
//!    `A ⊑ B ⊑ C` ⇒ `supers(A) = {B, C, top}` with `direct = true`
//!    filtered to `{B}`;
//! 2. `EquivalentObjectProperties` / `EquivalentDataProperties` are
//!    reachable through the hierarchy but never appear as *strict* supers
//!    (they are answered by `getEquivalentObjectProperties`);
//! 3. `InverseObjectProperties` introduces `inv(P)` expression nodes that
//!    inherit the hierarchy (`inv(S) ≡ P ⊑ Q`);
//! 4. `owl:topObjectProperty` / `owl:bottomObjectProperty` (and the data
//!    twins) anchor every hierarchy;
//! 5. domains/ranges are inherited from super properties (and equivalents);
//! 6. `Reasoner::classify_object_properties` / `classify_data_properties`
//!    return the same full closure in `PropertyClassificationResult`.

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ontology::*;
use oxidowl::{
    Node, NodeSet, OWLReasoner, OWLReasonerConfiguration, Reasoner, ReasonerConfig,
    ReasonerFactory, TableauReasonerFactory,
};
use std::sync::Arc;

const EX: &str = "http://ex.org/";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(std::sync::RwLock::new(o))
}

fn tableau_reasoner(ont: Ontology) -> Box<dyn OWLReasoner> {
    let onto = onto_ref(ont);
    TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .expect("tableau reasoner must be created")
}

/// Readable key for an object property expression: `IRI` or `inv(IRI)`.
fn obj_key(expr: &ObjectPropertyExpression) -> String {
    match expr {
        ObjectPropertyExpression::ObjectProperty(p) => p.iri.to_string(),
        ObjectPropertyExpression::InverseObjectProperty(p) => format!("inv({})", p.iri),
        ObjectPropertyExpression::PropertyChain(_) => "<chain>".to_string(),
    }
}

/// Readable key for a data property expression (always named).
fn data_key(expr: &DataPropertyExpression) -> String {
    match expr {
        DataPropertyExpression::DataProperty(p) => p.iri.to_string(),
    }
}

fn class_key(expr: &ClassExpression) -> String {
    match expr {
        ClassExpression::Class(cls) => cls.iri.to_string(),
        other => format!("{other:?}"),
    }
}

fn range_key(expr: &DataRange) -> String {
    match expr {
        DataRange::Datatype(iri) => iri.to_string(),
        other => format!("{other:?}"),
    }
}

/// Sorted `IRI` keys of a `NodeSet` of object property expressions.
fn obj_keys(nodes: NodeSet<ObjectPropertyExpression>) -> Vec<String> {
    let mut keys: Vec<String> = nodes.get_flattened().iter().map(obj_key).collect();
    keys.sort();
    keys
}

/// Sorted `IRI` keys of a `Node` of object property expressions.
fn obj_node_keys(node: Node<ObjectPropertyExpression>) -> Vec<String> {
    let mut keys: Vec<String> = node.get_entities().iter().map(obj_key).collect();
    keys.sort();
    keys
}

/// Sorted keys of a `NodeSet` of data property expressions.
fn data_keys(nodes: NodeSet<DataPropertyExpression>) -> Vec<String> {
    let mut keys: Vec<String> = nodes.get_flattened().iter().map(data_key).collect();
    keys.sort();
    keys
}

/// Sorted keys of a `Node` of data property expressions.
fn data_node_keys(node: Node<DataPropertyExpression>) -> Vec<String> {
    let mut keys: Vec<String> = node.get_entities().iter().map(data_key).collect();
    keys.sort();
    keys
}

/// Sorted keys of a `NodeSet` of class expressions.
fn class_keys(nodes: NodeSet<ClassExpression>) -> Vec<String> {
    let mut keys: Vec<String> = nodes.get_flattened().iter().map(class_key).collect();
    keys.sort();
    keys
}

/// Sorted keys of a `NodeSet` of data ranges.
fn range_keys(nodes: NodeSet<DataRange>) -> Vec<String> {
    let mut keys: Vec<String> = nodes.get_flattened().iter().map(range_key).collect();
    keys.sort();
    keys
}

/// Compare an already-sorted actual key list against an unsorted expectation.
fn assert_keys(actual: Vec<String>, expected: &[&str], what: &str) {
    let mut expected: Vec<String> = expected.iter().map(|s| (*s).to_string()).collect();
    expected.sort();
    assert_eq!(actual, expected, "{what}");
}

fn top() -> String {
    format!("{OWL}topObjectProperty")
}

fn bottom() -> String {
    format!("{OWL}bottomObjectProperty")
}

fn top_data() -> String {
    format!("{OWL}topDataProperty")
}

fn bottom_data() -> String {
    format!("{OWL}bottomDataProperty")
}

// ══════════════════════════════════════════════════════════════════════════════
// Object property chains
// ══════════════════════════════════════════════════════════════════════════════

/// `A ⊑ B`, `B ⊑ C` ⇒ `supers(A) = {B, C, top}` (transitive + top) while
/// `direct = true` filters to `{B}`; the mirror holds for subs of `C`.
#[test]
fn object_property_chain_supers_are_transitive_and_direct_is_filtered() {
    let df = DF::new();
    let a = df.obj_prop(format!("{EX}A"));
    let b = df.obj_prop(format!("{EX}B"));
    let c = df.obj_prop(format!("{EX}C"));

    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(a.clone(), b.clone()),
        df.sub_object_property_of(b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let supers_all = obj_keys(reasoner.get_super_object_properties(&a, false).unwrap());
    assert_keys(
        supers_all.clone(),
        &[&format!("{EX}B"), &format!("{EX}C"), &top()],
        "A's strict supers must be B, C and owl:topObjectProperty",
    );
    assert_keys(
        obj_keys(reasoner.get_super_object_properties(&a, true).unwrap()),
        &[&format!("{EX}B")],
        "only B sits directly above A (C and top are transitive)",
    );

    let subs_all = obj_keys(reasoner.get_sub_object_properties(&c, false).unwrap());
    assert_keys(
        subs_all.clone(),
        &[&bottom(), &format!("{EX}A"), &format!("{EX}B")],
        "C's strict subs must be A, B and owl:bottomObjectProperty",
    );
    assert_keys(
        obj_keys(reasoner.get_sub_object_properties(&c, true).unwrap()),
        &[&format!("{EX}B")],
        "only B sits directly below C",
    );

    // Middle of the chain.
    assert_keys(
        obj_keys(reasoner.get_super_object_properties(&b, true).unwrap()),
        &[&format!("{EX}C")],
        "C is the direct super of B",
    );
    assert_keys(
        obj_keys(reasoner.get_sub_object_properties(&b, true).unwrap()),
        &[&format!("{EX}A")],
        "A is the direct sub of B",
    );

    // Negative controls: bottom is never a super, top never a sub.
    assert!(
        !supers_all.contains(&bottom()),
        "owl:bottomObjectProperty must not be a super of A"
    );
    assert!(
        !subs_all.contains(&top()),
        "owl:topObjectProperty must not be a sub of C"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Equivalent properties
// ══════════════════════════════════════════════════════════════════════════════

/// `P ≡ Q`, `Q ⊑ R`: the equivalence is reachable (`P`'s supers contain `R`
/// through `Q`) and surfaced by `getEquivalentObjectProperties`, but `Q`
/// itself is **not** a *strict* super of `P` (OWL API `StrictSub…Of`).
#[test]
fn equivalent_properties_are_reachable_but_not_strict() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let q = df.obj_prop(format!("{EX}Q"));
    let r = df.obj_prop(format!("{EX}R"));

    let mut ont = df.build_ontology(vec![
        df.equivalent_object_properties(vec![p.clone(), q.clone()]),
        df.sub_object_property_of(q.clone(), r.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let supers_all = obj_keys(reasoner.get_super_object_properties(&p, false).unwrap());
    assert_keys(
        supers_all.clone(),
        &[&format!("{EX}R"), &top()],
        "P's strict supers are R and top — Q is equivalent, not strict",
    );
    assert_keys(
        obj_keys(reasoner.get_super_object_properties(&p, true).unwrap()),
        &[&format!("{EX}R")],
        "the direct super of P is R (Q is equivalent, top transitive)",
    );

    let equivalent = obj_node_keys(reasoner.get_equivalent_object_properties(&p).unwrap());
    assert!(
        equivalent.contains(&format!("{EX}P")) && equivalent.contains(&format!("{EX}Q")),
        "getEquivalentObjectProperties(P) must contain P and Q; got {equivalent:?}"
    );

    // Both equivalence members are strict (and direct) subs of R.
    let subs_all = obj_keys(reasoner.get_sub_object_properties(&r, false).unwrap());
    assert!(
        subs_all.contains(&format!("{EX}P")) && subs_all.contains(&format!("{EX}Q")),
        "both P and Q must be strict subs of R; got {subs_all:?}"
    );
    let subs_direct = obj_keys(reasoner.get_sub_object_properties(&r, true).unwrap());
    assert!(
        subs_direct.contains(&format!("{EX}P")) && subs_direct.contains(&format!("{EX}Q")),
        "neither P nor Q is strictly between the other and R; got {subs_direct:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Inverse properties
// ══════════════════════════════════════════════════════════════════════════════

/// `InverseObjectProperties(P, S)` introduces the `inv(S)` / `inv(P)`
/// expression nodes: `inv(S) ≡ P`, so `inv(S)` inherits `P ⊑ Q`, and `Q`'s
/// subs contain `inv(S)`.
#[test]
fn inverse_properties_propagate_through_the_hierarchy() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let q = df.obj_prop(format!("{EX}Q"));
    let s = df.obj_prop(format!("{EX}S"));

    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(p.clone(), q.clone()),
        df.inverse_object_properties(p.clone(), s.clone()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let inv_s = df.inv_obj_prop(format!("{EX}S"));

    // Declared inverse is reported by getInverseObjectProperties.
    let inverses = obj_node_keys(reasoner.get_inverse_object_properties(&p).unwrap());
    assert!(
        inverses.contains(&format!("{EX}S")) && inverses.contains(&format!("inv({EX}P)")),
        "getInverseObjectProperties(P) must contain S and inv(P); got {inverses:?}"
    );

    // inv(S) ≡ P ⊑ Q  ⇒  inv(S)'s strict supers are {Q, top}.
    let inv_s_supers = obj_keys(reasoner.get_super_object_properties(&inv_s, false).unwrap());
    assert!(
        inv_s_supers.contains(&format!("{EX}Q")) && inv_s_supers.contains(&top()),
        "inv(S) must inherit P's supers; got {inv_s_supers:?}"
    );
    assert_keys(
        obj_keys(reasoner.get_super_object_properties(&inv_s, true).unwrap()),
        &[&format!("{EX}Q")],
        "Q is the direct super of inv(S)",
    );

    // Q's strict subs contain both P and inv(S); direct keeps both (they are
    // equivalent, so neither sits strictly between the other and Q).
    let q_subs = obj_keys(reasoner.get_sub_object_properties(&q, false).unwrap());
    assert!(
        q_subs.contains(&format!("{EX}P")) && q_subs.contains(&format!("inv({EX}S)")),
        "Q's strict subs must contain P and inv(S); got {q_subs:?}"
    );
    let q_subs_direct = obj_keys(reasoner.get_sub_object_properties(&q, true).unwrap());
    assert!(
        q_subs_direct.contains(&format!("{EX}P")) && q_subs_direct.contains(&format!("inv({EX}S)")),
        "P and inv(S) are equivalent, so both stay direct subs of Q; got {q_subs_direct:?}"
    );

    // Equivalence surfaces through getEquivalentObjectProperties.
    let p_equiv = obj_node_keys(reasoner.get_equivalent_object_properties(&p).unwrap());
    assert!(
        p_equiv.contains(&format!("inv({EX}S)")),
        "P ≡ inv(S); got {p_equiv:?}"
    );

    // Negative control: an equivalent expression is not a *strict* super.
    let p_supers = obj_keys(reasoner.get_super_object_properties(&p, false).unwrap());
    assert!(
        !p_supers.contains(&format!("inv({EX}S)")),
        "inv(S) ≡ P must not appear among P's strict supers; got {p_supers:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// top / bottom
// ══════════════════════════════════════════════════════════════════════════════

/// `owl:topObjectProperty` is a (transitive) super of every property and
/// `owl:bottomObjectProperty` a sub of every property; each anchors exactly
/// one side of the hierarchy.
#[test]
fn top_and_bottom_anchor_the_object_property_hierarchy() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let q = df.obj_prop(format!("{EX}Q"));

    let mut ont = df.build_ontology(vec![df.sub_object_property_of(p.clone(), q.clone())]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let top_expr = reasoner.get_top_object_property();
    let bottom_expr = reasoner.get_bottom_object_property();

    // top is a super of everything, but only *directly* for a root property.
    let p_supers = obj_keys(reasoner.get_super_object_properties(&p, false).unwrap());
    assert!(p_supers.contains(&top()), "top must be a super of P");
    assert_keys(
        obj_keys(reasoner.get_super_object_properties(&p, true).unwrap()),
        &[&format!("{EX}Q")],
        "Q hides top from P's direct supers",
    );
    assert_keys(
        obj_keys(reasoner.get_super_object_properties(&q, true).unwrap()),
        &[&top()],
        "top is the direct super of a root property",
    );

    // bottom is a sub of everything; a leaf's only direct sub is bottom.
    let q_subs = obj_keys(reasoner.get_sub_object_properties(&q, false).unwrap());
    assert!(q_subs.contains(&bottom()), "bottom must be a sub of Q");
    assert_keys(
        obj_keys(reasoner.get_sub_object_properties(&q, true).unwrap()),
        &[&format!("{EX}P")],
        "P hides bottom from Q's direct subs",
    );
    assert_keys(
        obj_keys(reasoner.get_sub_object_properties(&p, true).unwrap()),
        &[&bottom()],
        "bottom is the direct sub of a leaf property",
    );

    // Anchors themselves.
    assert!(
        obj_keys(
            reasoner
                .get_super_object_properties(&top_expr, false)
                .unwrap()
        )
        .is_empty(),
        "owl:topObjectProperty has no supers"
    );
    assert!(
        obj_keys(
            reasoner
                .get_sub_object_properties(&bottom_expr, false)
                .unwrap()
        )
        .is_empty(),
        "owl:bottomObjectProperty has no subs"
    );
    // Every property is a strict sub of top; every property a strict super of bottom.
    let top_subs = obj_keys(
        reasoner
            .get_sub_object_properties(&top_expr, false)
            .unwrap(),
    );
    assert!(
        top_subs.contains(&format!("{EX}P")) && top_subs.contains(&format!("{EX}Q")),
        "every property must be a sub of top; got {top_subs:?}"
    );
    let bottom_supers = obj_keys(
        reasoner
            .get_super_object_properties(&bottom_expr, false)
            .unwrap(),
    );
    assert!(
        bottom_supers.contains(&format!("{EX}P")) && bottom_supers.contains(&format!("{EX}Q")),
        "bottom must be a sub of every property; got {bottom_supers:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Data properties
// ══════════════════════════════════════════════════════════════════════════════

/// The data-side mirror of the chain + equivalence + top/bottom contract.
#[test]
fn data_property_chain_equivalence_and_top_bottom() {
    let df = DF::new();
    let a = df.data_prop(format!("{EX}da"));
    let b = df.data_prop(format!("{EX}db"));
    let c = df.data_prop(format!("{EX}dc"));
    let d = df.data_prop(format!("{EX}dd"));
    let e = df.data_prop(format!("{EX}de"));

    let mut ont = df.build_ontology(vec![
        df.sub_data_property_of(a.clone(), b.clone()),
        df.sub_data_property_of(b.clone(), c.clone()),
        df.equivalent_data_properties(vec![d.clone(), e.clone()]),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    assert_keys(
        data_keys(reasoner.get_super_data_properties(&a, false).unwrap()),
        &[&format!("{EX}db"), &format!("{EX}dc"), &top_data()],
        "da's strict supers are db, dc and owl:topDataProperty",
    );
    assert_keys(
        data_keys(reasoner.get_super_data_properties(&a, true).unwrap()),
        &[&format!("{EX}db")],
        "only db is a direct super of da",
    );
    assert_keys(
        data_keys(reasoner.get_sub_data_properties(&c, false).unwrap()),
        &[&bottom_data(), &format!("{EX}da"), &format!("{EX}db")],
        "dc's strict subs are da, db and owl:bottomDataProperty",
    );
    assert_keys(
        data_keys(reasoner.get_sub_data_properties(&c, true).unwrap()),
        &[&format!("{EX}db")],
        "only db is a direct sub of dc",
    );

    // Equivalence: surfaced by getEquivalentDataProperties, never a strict super.
    assert_keys(
        data_keys(reasoner.get_super_data_properties(&d, false).unwrap()),
        &[&top_data()],
        "dd's only strict super is top",
    );
    let d_equiv = data_node_keys(reasoner.get_equivalent_data_properties(&d).unwrap());
    assert!(
        d_equiv.contains(&format!("{EX}dd")) && d_equiv.contains(&format!("{EX}de")),
        "getEquivalentDataProperties(dd) must contain dd and de; got {d_equiv:?}"
    );
    assert_keys(
        data_keys(reasoner.get_sub_data_properties(&d, true).unwrap()),
        &[&bottom_data()],
        "bottom is the direct sub of a root data property",
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Domain / range inheritance
// ══════════════════════════════════════════════════════════════════════════════

/// A property inherits the domains/ranges asserted on its **super**
/// properties (and on its equivalents): `Domain(Q, C)`, `P ⊑ Q` ⇒
/// `domains(P) ∋ C`.
#[test]
fn domains_and_ranges_are_inherited_from_super_properties() {
    let df = DF::new();
    let p = df.obj_prop(format!("{EX}P"));
    let q = df.obj_prop(format!("{EX}Q"));
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let big = df.class_ce(format!("{EX}Big"));
    let small = df.class_ce(format!("{EX}Small"));

    let dp = df.data_prop(format!("{EX}dp"));
    let dq = df.data_prop(format!("{EX}dq"));
    let dr = df.data_prop(format!("{EX}dr"));
    let ds = df.data_prop(format!("{EX}ds"));

    let mut ont = df.build_ontology(vec![
        // object side: P ⊑ Q with Q's domain/range
        df.sub_object_property_of(p.clone(), q.clone()),
        df.object_property_domain(q.clone(), big.clone()),
        df.object_property_range(q.clone(), small.clone()),
        // equivalents share their ranges
        df.equivalent_object_properties(vec![r.clone(), s.clone()]),
        df.object_property_range(s.clone(), big.clone()),
        // data side: dp ⊑ dq with dq's domain/range
        df.sub_data_property_of(dp.clone(), dq.clone()),
        df.data_property_domain(dq.clone(), big.clone()),
        df.data_property_range(dq.clone(), df.data_range_integer()),
        df.sub_data_property_of(dr.clone(), ds.clone()),
        df.data_property_range(ds.clone(), df.data_range_boolean()),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let p_domains = class_keys(reasoner.get_object_property_domains(&p, false).unwrap());
    assert_keys(
        p_domains.clone(),
        &[&format!("{EX}Big")],
        "P must inherit exactly Q's domain",
    );
    let p_ranges = class_keys(reasoner.get_object_property_ranges(&p, false).unwrap());
    assert_keys(
        p_ranges.clone(),
        &[&format!("{EX}Small")],
        "P must inherit exactly Q's range",
    );

    // Negative control: nothing is asserted on P itself, and a sub's
    // domain/range never flows *up* to its super.
    let q_domains = class_keys(reasoner.get_object_property_domains(&q, false).unwrap());
    assert!(
        !q_domains.contains(&format!("{EX}Small")),
        "ranges must not leak into the domain answer; got {q_domains:?}"
    );

    // Equivalent properties share their ranges.
    let r_ranges = class_keys(reasoner.get_object_property_ranges(&r, false).unwrap());
    assert_keys(
        r_ranges,
        &[&format!("{EX}Big")],
        "R must inherit the range asserted on its equivalent S",
    );

    // Data side: the sub inherits the super's domain and range.
    let dp_domains = class_keys(reasoner.get_data_property_domains(&dp, false).unwrap());
    assert_keys(
        dp_domains,
        &[&format!("{EX}Big")],
        "dp must inherit dq's domain",
    );
    let dp_ranges = range_keys(reasoner.get_data_property_ranges(&dp, false).unwrap());
    assert_keys(
        dp_ranges,
        &[&format!("{XSD}integer")],
        "dp must inherit dq's range",
    );
    let dr_ranges = range_keys(reasoner.get_data_property_ranges(&dr, false).unwrap());
    assert_keys(
        dr_ranges,
        &[&format!("{XSD}boolean")],
        "dr must inherit ds's range",
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// `Reasoner::classify_*_properties` (core surface)
// ══════════════════════════════════════════════════════════════════════════════

/// `PropertyClassificationResult` maps each property to its **full** strict
/// super set (transitive + top), not just the asserted edges.
#[test]
fn classify_object_properties_returns_the_full_closure() {
    let df = DF::new();
    let a = df.obj_prop(format!("{EX}A"));
    let b = df.obj_prop(format!("{EX}B"));
    let c = df.obj_prop(format!("{EX}C"));

    let mut ont = df.build_ontology(vec![
        df.sub_object_property_of(a.clone(), b.clone()),
        df.sub_object_property_of(b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    reasoner.load_ontology(ont).expect("load ontology");
    let result = reasoner
        .classify_object_properties()
        .expect("classify_object_properties");
    let hierarchy = result
        .object_property_hierarchy
        .as_ref()
        .expect("object property hierarchy");

    let a_key = a.clone();
    let entry = hierarchy.get(&a_key).expect("A must be a hierarchy key");
    let mut supers: Vec<String> = entry.iter().map(obj_key).collect();
    supers.sort();
    assert_keys(
        supers,
        &[&format!("{EX}B"), &format!("{EX}C"), &top()],
        "A's hierarchy entry must be the full transitive super set incl. top",
    );

    // Negative control: bottom never appears among supers.
    assert!(
        !entry.iter().any(|p| obj_key(p) == bottom()),
        "bottom must not appear among A's supers"
    );

    // An equivalent-only property still reaches top (and not its partner).
    let eq_b = df.obj_prop(format!("{EX}B2"));
    let mut ont2 = df.build_ontology(vec![
        df.equivalent_object_properties(vec![b.clone(), eq_b.clone()]),
    ]);
    df.auto_declare(&mut ont2);
    let mut reasoner2 = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    reasoner2.load_ontology(ont2).expect("load ontology");
    let result2 = reasoner2
        .classify_object_properties()
        .expect("classify_object_properties");
    let hierarchy2 = result2
        .object_property_hierarchy
        .as_ref()
        .expect("hierarchy");
    let b2_entry = hierarchy2.get(&eq_b).expect("B2 must be a hierarchy key");
    let mut b2_supers: Vec<String> = b2_entry.iter().map(obj_key).collect();
    b2_supers.sort();
    assert_keys(
        b2_supers,
        &[&top()],
        "an equivalent is not a strict super; only top remains",
    );
}

/// The data-side mirror of [`classify_object_properties_returns_the_full_closure`].
#[test]
fn classify_data_properties_returns_the_full_closure() {
    let df = DF::new();
    let a = df.data_prop(format!("{EX}da"));
    let b = df.data_prop(format!("{EX}db"));
    let c = df.data_prop(format!("{EX}dc"));

    let mut ont = df.build_ontology(vec![
        df.sub_data_property_of(a.clone(), b.clone()),
        df.sub_data_property_of(b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    reasoner.load_ontology(ont).expect("load ontology");
    let result = reasoner
        .classify_data_properties()
        .expect("classify_data_properties");
    let hierarchy = result
        .data_property_hierarchy
        .as_ref()
        .expect("data property hierarchy");

    let mut supers: Vec<String> = hierarchy[&a].iter().map(data_key).collect();
    supers.sort();
    assert_keys(
        supers,
        &[&format!("{EX}db"), &format!("{EX}dc"), &top_data()],
        "da's hierarchy entry must be the full transitive super set incl. top",
    );
}

/// Roundtrip tests: parse → serialize → re-parse → compare axiom sets.
///
/// This module ports the core pattern from OWL API v5's
/// `AbstractRoundTrippingTestCase.java` — the single most important
/// test class for verifying format correctness.
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

use crate::helpers::df::DF;
use crate::helpers::test_base::TestBase;
use crate::helpers::*;

// ══════════════════════════════════════════════════════════════════════════════
// Roundtrip Test Matrix — Functional Syntax
// ══════════════════════════════════════════════════════════════════════════════

/// Verify that roundtripping through Functional Syntax preserves all axioms.
fn functional_roundtrip(axioms: Vec<Axiom>) {
    let df = DF::new();
    let mut ontology = df.build_ontology(axioms);
    df.auto_declare(&mut ontology);
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&ontology, OntologyFormat::Functional)
        .expect("Functional roundtrip failed");
}

/// Verify roundtrip through Functional Syntax for a single axiom.
fn functional_roundtrip_single(ont: &Ontology) {
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(ont, OntologyFormat::Functional)
        .expect("Functional roundtrip failed");
}

// ── SubClassOf ──────────────────────────────────────────────────────────────

#[test]
fn rt_functional_subclass_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df.sub_class_of(a, b),
    ]);
}

#[test]
fn rt_functional_subclass_of_with_intersection() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let intersection = df.intersection_of(vec![a.clone(), b.clone()]);
    functional_roundtrip(vec![
        df.sub_class_of(intersection, c),
    ]);
}

// ── EquivalentClasses ───────────────────────────────────────────────────────

#[test]
fn rt_functional_equivalent_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    functional_roundtrip(vec![df.equivalent_classes(vec![a, b, c])]);
}

// ── DisjointClasses ─────────────────────────────────────────────────────────

#[test]
fn rt_functional_disjoint_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    functional_roundtrip(vec![df.disjoint_classes(vec![a, b])]);
}

// ── Declaration ─────────────────────────────────────────────────────────────

#[test]
fn rt_functional_declaration() {
    let df = DF::new();
    let entity = Entity::Class(IRI::new("http://ex.org/A"));
    functional_roundtrip(vec![df.declaration_axiom(entity)]);
}

#[test]
fn rt_functional_declaration_all_entity_types() {
    let df = DF::new();
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/C"))),
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.declaration_axiom(Entity::DataProperty(IRI::new("http://ex.org/D"))),
        df.declaration_axiom(Entity::NamedIndividual(IRI::new("http://ex.org/i"))),
        df.declaration_axiom(Entity::AnnotationProperty(IRI::new("http://ex.org/ap"))),
    ]);
}

// ── ClassAssertion ──────────────────────────────────────────────────────────

#[test]
fn rt_functional_class_assertion() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    functional_roundtrip(vec![df.class_assertion(a, i)]);
}

// ── ObjectPropertyAssertion ─────────────────────────────────────────────────

#[test]
fn rt_functional_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.object_property_assertion(p, i, j),
    ]);
}

// ── DataPropertyAssertion ───────────────────────────────────────────────────

#[test]
fn rt_functional_data_property_assertion() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let lit = df.literal("hello");
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::DataProperty(IRI::new("http://ex.org/dp"))),
        df.data_property_assertion(dp, i, lit),
    ]);
}

// ── SubObjectPropertyOf ─────────────────────────────────────────────────────

#[test]
fn rt_functional_sub_object_property_of() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    functional_roundtrip(vec![df.sub_object_property_of(p, q)]);
}

// ── SubDataPropertyOf ───────────────────────────────────────────────────────

#[test]
fn rt_functional_sub_data_property_of() {
    let df = DF::new();
    let dp1 = df.data_prop("http://ex.org/dp1");
    let dp2 = df.data_prop("http://ex.org/dp2");
    functional_roundtrip(vec![df.sub_data_property_of(dp1, dp2)]);
}

// ── Property Characteristics ────────────────────────────────────────────────

#[test]
fn rt_functional_functional_object_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.functional_object_property(p)]);
}

#[test]
fn rt_functional_transitive_object_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.transitive_object_property(p)]);
}

#[test]
fn rt_functional_symmetric_object_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.symmetric_object_property(p)]);
}

#[test]
fn rt_functional_asymmetric_object_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.asymmetric_object_property(p)]);
}

#[test]
fn rt_functional_reflexive_object_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.reflexive_object_property(p)]);
}

#[test]
fn rt_functional_irreflexive_object_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.irreflexive_object_property(p)]);
}

#[test]
fn rt_functional_inverse_functional() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    functional_roundtrip(vec![df.inverse_functional_object_property(p)]);
}

#[test]
fn rt_functional_inverse_object_properties() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    functional_roundtrip(vec![df.inverse_object_properties(p, q)]);
}

// ── Object Property Domain/Range ────────────────────────────────────────────

#[test]
fn rt_functional_object_property_domain() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let c = df.class_ce("http://ex.org/C");
    functional_roundtrip(vec![df.object_property_domain(p, c)]);
}

#[test]
fn rt_functional_object_property_range() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let c = df.class_ce("http://ex.org/C");
    functional_roundtrip(vec![df.object_property_range(p, c)]);
}

// ── Data Property Domain/Range ──────────────────────────────────────────────

#[test]
fn rt_functional_data_property_domain() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let c = df.class_ce("http://ex.org/C");
    functional_roundtrip(vec![df.data_property_domain(dp, c)]);
}

#[test]
fn rt_functional_data_property_range() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let dr = df.data_range_integer();
    functional_roundtrip(vec![df.data_property_range(dp, dr)]);
}

// ── Equivalent Object/Data Properties ───────────────────────────────────────

#[test]
fn rt_functional_equivalent_object_properties() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    functional_roundtrip(vec![df.equivalent_object_properties(vec![p, q])]);
}

#[test]
fn rt_functional_equivalent_data_properties() {
    let df = DF::new();
    let dp1 = df.data_prop("http://ex.org/dp1");
    let dp2 = df.data_prop("http://ex.org/dp2");
    functional_roundtrip(vec![df.equivalent_data_properties(vec![dp1, dp2])]);
}

// ── Disjoint Object/Data Properties ─────────────────────────────────────────

#[test]
fn rt_functional_disjoint_object_properties() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    functional_roundtrip(vec![df.disjoint_object_properties(vec![p, q])]);
}

#[test]
fn rt_functional_disjoint_data_properties() {
    let df = DF::new();
    let dp1 = df.data_prop("http://ex.org/dp1");
    let dp2 = df.data_prop("http://ex.org/dp2");
    functional_roundtrip(vec![df.disjoint_data_properties(vec![dp1, dp2])]);
}

// ── Same/Different Individuals ──────────────────────────────────────────────

#[test]
fn rt_functional_same_individual() {
    let df = DF::new();
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    functional_roundtrip(vec![df.same_individual(vec![i, j])]);
}

#[test]
fn rt_functional_different_individuals() {
    let df = DF::new();
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    functional_roundtrip(vec![df.different_individuals(vec![i, j])]);
}

// ── NegativeAssertions ──────────────────────────────────────────────────────

#[test]
fn rt_functional_negative_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    functional_roundtrip(vec![df.negative_object_property_assertion(p, i, j)]);
}

#[test]
fn rt_functional_negative_data_property_assertion() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let lit = df.literal("test");
    functional_roundtrip(vec![df.negative_data_property_assertion(dp, i, lit)]);
}

// ── Annotation Assertion ────────────────────────────────────────────────────

#[test]
fn rt_functional_annotation_assertion() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/label");
    let c_iri = IRI::new("http://ex.org/A");
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::Class(c_iri.clone())),
        df.declaration_axiom(Entity::AnnotationProperty(IRI::new("http://ex.org/label"))),
        df.annotation_assertion(ap, c_iri, "Class A"),
    ]);
}

// ── HasKey ──────────────────────────────────────────────────────────────────

#[test]
fn rt_functional_has_key() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let dp = df.data_prop("http://ex.org/dp");
    functional_roundtrip(vec![df.has_key(c, vec![p], vec![dp])]);
}

// ── DisjointUnion ───────────────────────────────────────────────────────────

#[test]
fn rt_functional_disjoint_union() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let d1 = df.class_ce("http://ex.org/D1");
    let d2 = df.class_ce("http://ex.org/D2");
    let d3 = df.class_ce("http://ex.org/D3");
    functional_roundtrip(vec![df.disjoint_union(c, vec![d1, d2, d3])]);
}

// ── Class Expressions ───────────────────────────────────────────────────────

#[test]
fn rt_functional_object_some_values_from() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let b = df.class_ce("http://ex.org/B");
    let svf = df.some_values_from(p, b);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, svf)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_all_values_from() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let b = df.class_ce("http://ex.org/B");
    let avf = df.all_values_from(p, b);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, avf)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_intersection_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let int = df.intersection_of(vec![a, b]);
    let mut o = df.build_ontology(vec![df.sub_class_of(int, c)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_union_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let union = df.union_of(vec![a, b]);
    let mut o = df.build_ontology(vec![df.sub_class_of(union, c)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_complement_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let not_b = df.complement_of(b);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, not_b)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_one_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let one_of = df.one_of(vec![i, j]);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, one_of)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_has_self() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let has_self = df.has_self(p);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, has_self)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_has_value() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let hv = df.has_value(p, i);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, hv)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

#[test]
fn rt_functional_object_cardinality() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let b = df.class_ce("http://ex.org/B");
    let min2 = df.min_cardinality(2, p.clone(), b);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, min2)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

// ── Inverse Object Property ─────────────────────────────────────────────────

#[test]
fn rt_functional_inverse_property_in_expression() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p_inv = df.inv_obj_prop("http://ex.org/P");
    let b = df.class_ce("http://ex.org/B");
    let svf = df.some_values_from(p_inv, b);
    let mut o = df.build_ontology(vec![df.sub_class_of(a, svf)]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

// ── Literal variants ────────────────────────────────────────────────────────

#[test]
fn rt_functional_literal_plain() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let lit = df.literal("plain text");
    functional_roundtrip(vec![df.data_property_assertion(dp, i, lit)]);
}

#[test]
fn rt_functional_literal_with_language() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let lit = df.lang_literal("bonjour", "fr");
    functional_roundtrip(vec![df.data_property_assertion(dp, i, lit)]);
}

#[test]
fn rt_functional_literal_integer() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let lit = df.int_literal(42);
    functional_roundtrip(vec![df.data_property_assertion(dp, i, lit)]);
}

// ── Empty ontology ──────────────────────────────────────────────────────────

#[test]
fn rt_functional_empty_ontology() {
    let df = DF::new();
    let o = df.new_ontology_with_iri("http://ex.org/empty");
    functional_roundtrip_single(&o);
}

// ── Multiple axioms together ────────────────────────────────────────────────

#[test]
fn rt_functional_mixed_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let dp = df.data_prop("http://ex.org/dp");

    functional_roundtrip(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(b.clone(), c.clone()),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), df.named("http://ex.org/j")),
        df.data_property_assertion(dp.clone(), i, df.literal("value")),
        df.symmetric_object_property(p),
        df.functional_data_property(dp),
    ]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Turtle Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

fn turtle_roundtrip(axioms: Vec<Axiom>) {
    let df = DF::new();
    let mut ontology = df.build_ontology(axioms);
    df.auto_declare(&mut ontology);
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&ontology, OntologyFormat::Turtle)
        .expect("Turtle roundtrip failed");
}

fn turtle_roundtrip_single(ont: &Ontology) {
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(ont, OntologyFormat::Turtle)
        .expect("Turtle roundtrip failed");
}

#[test]
fn rt_turtle_subclass_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    turtle_roundtrip(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df.sub_class_of(a, b),
    ]);
}

#[test]
fn rt_turtle_equivalent_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    turtle_roundtrip(vec![df.equivalent_classes(vec![a, b])]);
}

#[test]
fn rt_turtle_class_assertion() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    turtle_roundtrip(vec![df.class_assertion(a, i)]);
}

#[test]
fn rt_turtle_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    turtle_roundtrip(vec![
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.object_property_assertion(p, i, j),
    ]);
}

#[test]
fn rt_turtle_data_property_assertion() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let lit = df.literal("test value");
    turtle_roundtrip(vec![df.data_property_assertion(dp, i, lit)]);
}

#[test]
fn rt_turtle_functional_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    turtle_roundtrip(vec![df.functional_object_property(p)]);
}

#[test]
fn rt_turtle_transitive_property() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    turtle_roundtrip(vec![df.transitive_object_property(p)]);
}

#[test]
fn rt_turtle_disjoint_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    turtle_roundtrip(vec![df.disjoint_classes(vec![a, b])]);
}

#[test]
fn rt_turtle_empty_ontology() {
    let df = DF::new();
    let o = df.new_ontology_with_iri("http://ex.org/empty2");
    turtle_roundtrip_single(&o);
}

// ══════════════════════════════════════════════════════════════════════════════
// RDF/XML Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

fn rdfxml_roundtrip(axioms: Vec<Axiom>) {
    let df = DF::new();
    let mut ontology = df.build_ontology(axioms);
    df.auto_declare(&mut ontology);
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&ontology, OntologyFormat::RdfXml)
        .expect("RDF/XML roundtrip failed");
}

#[test]
fn rt_rdfxml_subclass_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    rdfxml_roundtrip(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df.sub_class_of(a, b),
    ]);
}

#[test]
fn rt_rdfxml_equivalent_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    rdfxml_roundtrip(vec![df.equivalent_classes(vec![a, b])]);
}

#[test]
fn rt_rdfxml_class_assertion() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    rdfxml_roundtrip(vec![df.class_assertion(a, i)]);
}

#[test]
fn rt_rdfxml_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    rdfxml_roundtrip(vec![
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.object_property_assertion(p, i, j),
    ]);
}

#[test]
fn rt_rdfxml_disjoint_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    rdfxml_roundtrip(vec![df.disjoint_classes(vec![a, b])]);
}

// ══════════════════════════════════════════════════════════════════════════════
// OWL/XML Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

fn owlxml_roundtrip(axioms: Vec<Axiom>) {
    let df = DF::new();
    let mut ontology = df.build_ontology(axioms);
    df.auto_declare(&mut ontology);
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&ontology, OntologyFormat::OwlXml)
        .expect("OWL/XML roundtrip failed");
}

#[test]
fn rt_owlxml_subclass_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    owlxml_roundtrip(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df.sub_class_of(a, b),
    ]);
}

#[test]
fn rt_owlxml_equivalent_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    owlxml_roundtrip(vec![df.equivalent_classes(vec![a, b])]);
}

#[test]
fn rt_owlxml_class_assertion() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    owlxml_roundtrip(vec![df.class_assertion(a, i)]);
}

#[test]
fn rt_owlxml_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    owlxml_roundtrip(vec![
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.object_property_assertion(p, i, j),
    ]);
}

#[test]
fn rt_owlxml_disjoint_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    owlxml_roundtrip(vec![df.disjoint_classes(vec![a, b])]);
}

#[test]
fn rt_owlxml_property_characteristics() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    owlxml_roundtrip(vec![
        df.functional_object_property(p.clone()),
        df.transitive_object_property(p),
    ]);
}

// ══════════════════════════════════════════════════════════════════════════════
// N-Triples Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

fn nt_roundtrip(axioms: Vec<Axiom>) {
    let df = DF::new();
    let mut ontology = df.build_ontology(axioms);
    df.auto_declare(&mut ontology);
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&ontology, OntologyFormat::NTriples)
        .expect("N-Triples roundtrip failed");
}

#[test]
fn rt_ntriples_subclass_of() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    nt_roundtrip(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df.sub_class_of(a, b),
    ]);
}

#[test]
fn rt_ntriples_class_assertion() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    nt_roundtrip(vec![df.class_assertion(a, i)]);
}

#[test]
fn rt_ntriples_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    nt_roundtrip(vec![
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.object_property_assertion(p, i, j),
    ]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Cross-Format Equivalence Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn cross_format_rdfxml_to_functional() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let mut tb = TestBase::new();
    tb.plain_equal(&onto, false)
        .expect("Cross-format RDF/XML ↔ Functional failed");
}

#[test]
fn cross_format_rdfxml_to_functional_with_input_check() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let mut tb = TestBase::new();
    tb.plain_equal(&onto, true)
        .expect("Cross-format with input check failed");
}

// ══════════════════════════════════════════════════════════════════════════════
// Anonymous Individuals Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn rt_functional_anonymous_class_assertion() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let anon = df.anon();
    functional_roundtrip(vec![df.class_assertion(a, anon)]);
}

#[test]
fn rt_functional_anonymous_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let anon1 = df.anon();
    let anon2 = df.anon();
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/P"))),
        df.object_property_assertion(p, anon1, anon2),
    ]);
}

#[test]
fn rt_functional_annotation_with_anon() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/ap");
    let a_iri = IRI::new("http://ex.org/A");
    let anon_ind = df.anonymous_individual();
    let ax = AnnotationAssertionAxiom {
        id: df.next_id(),
        subject: AnnotationSubject::IRI(a_iri.clone()),
        property: ap,
        value: AnnotationValue::AnonymousIndividual(anon_ind),
        annotations: vec![],
    };
    functional_roundtrip(vec![
        df.declaration_axiom(Entity::Class(a_iri)),
        df.declaration_axiom(Entity::AnnotationProperty(IRI::new("http://ex.org/ap"))),
        Axiom::AnnotationAssertion(ax),
    ]);
}

// ══════════════════════════════════════════════════════════════════════════════
// Ontology Metadata Roundtrip Tests (IRI, version IRI)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn rt_functional_ontology_version_iri() {
    let df = DF::new();
    let mut o = df.new_ontology_with_iri("http://ex.org/ont");
    o.set_version_iri(Some(IRI::new("http://ex.org/ont/1.0")));
    o.add_axiom(df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))));
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(&o, OntologyFormat::Functional)
        .expect("Ontology version IRI roundtrip failed");
}

// ══════════════════════════════════════════════════════════════════════════════
// Edge Case: Escaping, Special Characters
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn rt_functional_literal_with_escapes() {
    let df = DF::new();
    let dp = df.data_prop("http://ex.org/desc");
    let i = df.named("http://ex.org/i");
    // Test common escape sequences in literals
    for val in &[
        "hello world",
        "line1\nline2",
        "tab\there",
        "quote\"inside",
        "back\\slash",
        "angle<bracket>",
    ] {
        let lit = df.literal(*val);
        functional_roundtrip(vec![df.data_property_assertion(dp.clone(), i.clone(), lit)]);
    }
}

#[test]
fn rt_functional_escaped_iri() {
    let df = DF::new();
    let mut o = df.build_ontology(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/Class%20With%20Spaces"))),
    ]);
    df.auto_declare(&mut o);
    functional_roundtrip_single(&o);
}

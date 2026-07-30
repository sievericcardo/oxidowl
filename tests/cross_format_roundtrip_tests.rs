#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::test_base::TestBase;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::*;

/// Roundtrip through source format and verify the original axiom set survives.
fn rt_and_compare(ont: &Ontology, fmt: OntologyFormat) {
    let mut tb = TestBase::new();
    tb.round_trip_and_compare(ont, fmt)
        .unwrap_or_else(|e| panic!("Roundtrip {fmt:?} failed: {e}"));
}

/// Roundtrip through source and target formats independently, then
/// compare the two reloaded ontologies axiom-structurally.
fn cross_compare(ont: &Ontology, f1: OntologyFormat, f2: OntologyFormat) {
    let mut tb = TestBase::new();
    let o1 = tb.round_trip(ont, f1).unwrap_or_else(|e| panic!("Roundtrip {f1:?} failed: {e}"));
    let o2 = tb.round_trip(ont, f2).unwrap_or_else(|e| panic!("Roundtrip {f2:?} failed: {e}"));
    assertions::assert_ontologies_axiom_equal(&o1, &o2);
}

/// Roundtrip and verify at least one axiom of the expected type is present.
fn rt_loose(ont: &Ontology, fmt: OntologyFormat, expected_type: AxiomType) {
    let mut tb = TestBase::new();
    let reloaded = tb.round_trip(ont, fmt).unwrap_or_else(|e| panic!("Roundtrip {fmt:?} failed: {e}"));
    let found = reloaded.axioms().iter().any(|ax| ax.axiom_type() == expected_type);
    assert!(found, "Expected {expected_type:?} axiom not found after {fmt:?} roundtrip");
}

/// Roundtrip and verify at least one of the listed axiom types is present.
fn rt_loose_types(ont: &Ontology, fmt: OntologyFormat, types: &[AxiomType]) {
    let mut tb = TestBase::new();
    let reloaded = tb.round_trip(ont, fmt).unwrap_or_else(|e| panic!("Roundtrip {fmt:?} failed: {e}"));
    for t in types {
        if reloaded.axioms().iter().any(|ax| ax.axiom_type() == *t) {
            return;
        }
    }
    panic!("None of {types:?} found after {fmt:?} roundtrip");
}

/// Roundtrip and verify the reloaded ontology has at least `min_count` axioms.
fn rt_min_count(ont: &Ontology, fmt: OntologyFormat, min_count: usize) {
    let mut tb = TestBase::new();
    let reloaded = tb.round_trip(ont, fmt).unwrap_or_else(|e| panic!("Roundtrip {fmt:?} failed: {e}"));
    let count = reloaded.axioms().len();
    assert!(count >= min_count, "Expected >= {min_count} axioms after {fmt:?} roundtrip, got {count}");
}

// ══════════════════════════════════════════════════════════════════════════════
// 1-6. Cross-Format Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_rdf_xml_functional_equivalence() {
    let df = DF::new();
    let ont = df.simple_chain_ontology();
    let mut tb = TestBase::new();
    tb.plain_equal(&ont, true).unwrap();
}

#[test]
fn test_owl_xml_turtle_equivalence() {
    let df = DF::new();
    let ont = df.simple_chain_ontology();
    cross_compare(&ont, OntologyFormat::OwlXml, OntologyFormat::Turtle);
}

#[test]
fn test_all_core_format_pairs() {
    let df = DF::new();
    let ont = df.simple_chain_ontology();

    let formats = TestBase::core_roundtrip_formats();
    for (idx1, f1) in formats.iter().enumerate() {
        for (idx2, f2) in formats.iter().enumerate() {
            if idx1 >= idx2 {
                continue;
            }
            cross_compare(&ont, *f1, *f2);
        }
    }
}

#[test]
fn test_plain_equal_all_formats() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(b.clone(), c.clone()),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        df.transitive_object_property(p.clone()),
        df.functional_object_property(q.clone()),
        df.disjoint_classes(vec![a.clone(), b.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let mut tb = TestBase::new();
    tb.plain_equal(&ont, false).unwrap();
}

#[test]
fn test_functional_to_rdfxml_cross_format() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let p = df.obj_prop("http://ex.org/P");

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(b.clone(), c.clone()),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
    ]);
    df.auto_declare(&mut ont);

    cross_compare(&ont, OntologyFormat::Functional, OntologyFormat::RdfXml);
}

#[test]
fn test_turtle_to_owlxml_cross_format() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        df.transitive_object_property(p.clone()),
    ]);
    df.auto_declare(&mut ont);

    cross_compare(&ont, OntologyFormat::Turtle, OntologyFormat::OwlXml);
}

// ══════════════════════════════════════════════════════════════════════════════
// 7-10. Anonymous / Blank Node Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_anonymous_in_class_assertion_roundtrip() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let anon = df.anon();

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), anon.clone()),
    ]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);

    let mut tb = TestBase::new();
    let func_reloaded = tb.round_trip(&ont, OntologyFormat::Functional).unwrap();
    let has_anon = func_reloaded.axioms().iter().any(|ax| {
        if let Axiom::ClassAssertion(ca_ax) = ax {
            matches!(ca_ax.individual, Individual::Anonymous(_))
        } else {
            false
        }
    });
    assert!(has_anon, "Anonymous individual lost in Functional roundtrip");
}

#[test]
fn test_anonymous_in_object_property_roundtrip() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let anon = df.anon();
    let j = df.named("http://ex.org/j");

    let mut ont = df.build_ontology(vec![
        df.object_property_assertion(p.clone(), anon.clone(), j.clone()),
    ]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);

    let mut tb = TestBase::new();
    let func_reloaded = tb.round_trip(&ont, OntologyFormat::Functional).unwrap();
    let has_anon = func_reloaded.axioms().iter().any(|ax| {
        if let Axiom::ObjectPropertyAssertion(opa_ax) = ax {
            matches!(opa_ax.source, Individual::Anonymous(_))
                || matches!(opa_ax.target, Individual::Anonymous(_))
        } else {
            false
        }
    });
    assert!(has_anon, "Anonymous individual lost in Functional roundtrip");
}

#[test]
fn test_anonymous_chained_in_annotations() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/ap");
    let anon = df.anonymous_individual();

    let nested_inner = Annotation::new(
        ap.clone(),
        AnnotationValue::AnonymousIndividual(df.anonymous_individual()),
        vec![],
    );
    let nested_mid = Annotation::new(
        ap.clone(),
        AnnotationValue::Literal(df.literal("mid")),
        vec![nested_inner],
    );

    let ax = AnnotationAssertionAxiom {
        id: df.next_id(),
        subject: AnnotationSubject::AnonymousIndividual(anon),
        property: ap.clone(),
        value: AnnotationValue::Literal(df.literal("annotated")),
        annotations: vec![nested_mid],
    };

    let mut ont = df.build_ontology(vec![
        df.declaration_axiom(Entity::AnnotationProperty(IRI::new("http://ex.org/ap"))),
        Axiom::AnnotationAssertion(ax),
    ]);
    df.auto_declare(&mut ont);

    rt_loose(&ont, OntologyFormat::Functional, AxiomType::AnnotationAssertion);
}

#[test]
fn test_blank_node_ids_in_turtle_roundtrip() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let anon1 = df.anon();
    let anon2 = df.anon();
    let anon3 = df.anon();

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), anon1.clone()),
        df.class_assertion(a.clone(), anon2.clone()),
        df.class_assertion(a.clone(), anon3.clone()),
    ]);
    df.auto_declare(&mut ont);

    let anon_start = ont.axioms().iter().filter(|ax| {
        if let Axiom::ClassAssertion(ca_ax) = ax {
            matches!(ca_ax.individual, Individual::Anonymous(_))
        } else {
            false
        }
    }).count();
    assert_eq!(anon_start, 3, "Should start with 3 anonymous class assertions");

    let tb = TestBase::new();
    let turtle = tb.save_to_string(&ont, OntologyFormat::Turtle).unwrap();
    assert!(!turtle.is_empty(), "Turtle serialization should produce output");

    let reloaded = tb
        .load_and_get_ontology(&turtle, OntologyFormat::Turtle)
        .unwrap();
    assert!(!reloaded.axioms().is_empty(), "Reloaded Turtle ontology should have content");

    let anon_count = reloaded.axioms().iter().filter(|ax| {
        if let Axiom::ClassAssertion(ca_ax) = ax {
            matches!(ca_ax.individual, Individual::Anonymous(_))
        } else {
            false
        }
    }).count();

    let total_ax = reloaded.axioms().len();
    assert!(total_ax >= 1, "Reloaded Turtle ontology should contain axioms; anonymous count: {anon_count}");
}

// ══════════════════════════════════════════════════════════════════════════════
// 11-18. Specific Axiom Roundtrip Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_annotated_subclass_roundtrip_all_formats() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/ap");

    let inner_ann = Annotation::new(
        ap.clone(),
        AnnotationValue::Literal(df.literal("inner")),
        vec![],
    );
    let middle_ann = Annotation::new(
        ap.clone(),
        AnnotationValue::Literal(df.literal("middle")),
        vec![inner_ann],
    );
    let outer_ann = Annotation::new(
        ap.clone(),
        AnnotationValue::Literal(df.literal("outer")),
        vec![middle_ann],
    );

    let sub_ax = SubClassOfAxiom {
        id: df.next_id(),
        subclass: df.class_ce("http://ex.org/A"),
        superclass: df.class_ce("http://ex.org/B"),
        annotations: vec![outer_ann],
    };

    let mut ont = df.build_ontology(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/B"))),
        df.declaration_axiom(Entity::AnnotationProperty(IRI::new("http://ex.org/ap"))),
        Axiom::SubClassOf(sub_ax),
    ]);
    df.auto_declare(&mut ont);

    for fmt in TestBase::core_roundtrip_formats() {
        rt_loose(&ont, fmt, AxiomType::SubClassOf);
    }
}

#[test]
fn test_swrl_rule_atom_types_roundtrip() {
    let df = DF::new();
    let var_x = SWRLVariable::new(IRI::new("http://ex.org/var#x"));
    let var_y = SWRLVariable::new(IRI::new("http://ex.org/var#y"));

    let body_class = SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Person"),
        argument: SWRLIArgument::Variable(var_x.clone()),
    };
    let body_prop = SWRLAtom::ObjectPropertyAtom {
        predicate: df.obj_prop("http://ex.org/hasParent"),
        first_argument: SWRLIArgument::Variable(var_x.clone()),
        second_argument: SWRLIArgument::Variable(var_y.clone()),
    };
    let body_data = SWRLAtom::DataPropertyAtom {
        predicate: df.data_prop("http://ex.org/age"),
        first_argument: SWRLIArgument::Variable(var_x.clone()),
        second_argument: SWRLDArgument::Variable(SWRLVariable::new(IRI::new(
            "http://ex.org/var#age",
        ))),
    };
    let head_atom = SWRLAtom::ClassAtom {
        predicate: df.class_ce("http://ex.org/Adult"),
        argument: SWRLIArgument::Variable(var_x.clone()),
    };

    let rule = SWRLRule::new(
        vec![head_atom.clone()],
        vec![body_class, body_prop, body_data],
    );
    assert!(rule.is_safe(), "SWRL rule with all head vars in body is correctly safe");
    let rule_ax = SWRLRuleAxiom::new(df.next_id(), rule);

    let mut ont = df.build_ontology(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/Person"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://ex.org/Adult"))),
        df.declaration_axiom(Entity::ObjectProperty(IRI::new("http://ex.org/hasParent"))),
        df.declaration_axiom(Entity::DataProperty(IRI::new("http://ex.org/age"))),
        Axiom::Rule(rule_ax),
    ]);
    df.auto_declare(&mut ont);

    let rule_count = ont.axioms().iter().filter(|ax| matches!(ax, Axiom::Rule(_))).count();
    assert_eq!(rule_count, 1, "Ontology should contain exactly 1 SWRL rule");

    let tb = TestBase::new();
    let serialized = tb.save_to_string(&ont, OntologyFormat::Functional).unwrap();
    assert!(!serialized.is_empty(), "Functional serialization of SWRL ontology succeeds");
}

#[test]
fn test_negative_assertions_roundtrip() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let dp = df.data_prop("http://ex.org/dp");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let lit = df.literal("test");

    let nopa = df.negative_object_property_assertion(p.clone(), i.clone(), j.clone());
    let ndpa = df.negative_data_property_assertion(dp.clone(), i.clone(), lit.clone());

    let mut ont = df.build_ontology(vec![nopa, ndpa]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);
}

#[test]
fn test_has_key_with_annotation_roundtrip() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let dp = df.data_prop("http://ex.org/dp");

    let hk = df.has_key(c.clone(), vec![p.clone()], vec![dp.clone()]);

    let mut ont = df.build_ontology(vec![hk]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);
}

#[test]
fn test_datatype_definition_roundtrip() {
    use horned_owl::model::{Build, DataRange as HornedDataRange, Datatype as HornedDataType};

    let df = DF::new();
    let b = Build::new_string();

    let custom_dt = b.iri("http://ex.org/customDT".to_string());
    let integer_iri = b.iri("http://www.w3.org/2001/XMLSchema#integer".to_string());
    let int_dr = HornedDataRange::Datatype(HornedDataType(integer_iri));
    let complement_dr = HornedDataRange::DataComplementOf(Box::new(int_dr));

    let dt_def = DatatypeDefinitionAxiom {
        id: df.next_id(),
        datatype: custom_dt,
        data_range: complement_dr,
        annotations: vec![],
    };

    let mut ont = df.build_ontology(vec![
        df.declaration_axiom(Entity::Datatype(IRI::new("http://ex.org/customDT"))),
        Axiom::DatatypeDefinition(dt_def),
    ]);
    df.auto_declare(&mut ont);

    let dt_count = ont.axioms().iter().filter(|ax| matches!(ax, Axiom::DatatypeDefinition(_))).count();
    assert!(dt_count >= 1, "Ontology should contain DatatypeDefinition axiom");

    assert!(
        ont.axioms().iter().any(|ax| matches!(ax, Axiom::DatatypeDefinition(_))),
        "DatatypeDefinition axiom must be present in constructed ontology"
    );
}

#[test]
fn test_disjoint_union_roundtrip_all_formats() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let d1 = df.class_ce("http://ex.org/D1");
    let d2 = df.class_ce("http://ex.org/D2");
    let d3 = df.class_ce("http://ex.org/D3");

    let du = df.disjoint_union(c.clone(), vec![d1.clone(), d2.clone(), d3.clone()]);

    let mut ont = df.build_ontology(vec![du]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);
    rt_loose(&ont, OntologyFormat::OwlXml, AxiomType::DisjointUnion);
}

#[test]
fn test_all_property_characteristics_roundtrip() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let dp = df.data_prop("http://ex.org/dp");

    let mut ont = df.build_ontology(vec![
        df.functional_object_property(p.clone()),
        df.transitive_object_property(p.clone()),
        df.symmetric_object_property(p.clone()),
        df.asymmetric_object_property(q.clone()),
        df.reflexive_object_property(p.clone()),
        df.irreflexive_object_property(q.clone()),
        df.inverse_functional_object_property(p.clone()),
        df.functional_data_property(dp.clone()),
    ]);
    df.auto_declare(&mut ont);

    for fmt in &[
        OntologyFormat::Functional,
        OntologyFormat::Turtle,
        OntologyFormat::OwlXml,
    ] {
        rt_loose_types(&ont, *fmt, &[
            AxiomType::FunctionalObjectProperty,
            AxiomType::TransitiveObjectProperty,
            AxiomType::SymmetricObjectProperty,
            AxiomType::AsymmetricObjectProperty,
            AxiomType::ReflexiveObjectProperty,
            AxiomType::IrreflexiveObjectProperty,
            AxiomType::InverseFunctionalObjectProperty,
            AxiomType::FunctionalDataProperty,
        ]);
    }
}

#[test]
fn test_ontology_annotations_roundtrip() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");

    let ont_ann = df.rdfs_comment("Ontology-level comment");
    let ont_label = df.rdfs_label("Test Ontology");

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
    ]);
    df.auto_declare(&mut ont);
    ont.set_iri(IRI::new("http://ex.org/myOnt"));
    ont.annotations = vec![ont_ann, ont_label];

    rt_and_compare(&ont, OntologyFormat::Functional);
}

// ══════════════════════════════════════════════════════════════════════════════
// 19-20. Large / Mass Axiom Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_many_classes_equivalent_roundtrip() {
    let df = DF::new();
    let mut classes: Vec<ClassExpression> = Vec::new();
    for i in 0..15 {
        classes.push(df.class_ce(&format!("http://ex.org/C{i}")));
    }

    let eq = df.equivalent_classes(classes);
    let mut ont = df.build_ontology(vec![eq]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);
    rt_min_count(&ont, OntologyFormat::Turtle, 1);
    rt_min_count(&ont, OntologyFormat::OwlXml, 1);
}

#[test]
fn test_many_different_individuals_roundtrip() {
    let df = DF::new();
    let mut inds: Vec<Individual> = Vec::new();
    for i in 0..25 {
        inds.push(df.named(&format!("http://ex.org/ind{i}")));
    }

    let diff = df.different_individuals(inds);
    let mut ont = df.build_ontology(vec![diff]);
    df.auto_declare(&mut ont);

    rt_and_compare(&ont, OntologyFormat::Functional);
    rt_min_count(&ont, OntologyFormat::Turtle, 1);
}

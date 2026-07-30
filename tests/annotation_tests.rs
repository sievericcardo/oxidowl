#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::test_base::TestBase;
use helpers::*;
use oxidowl::factory::providers::AxiomCreationProvider;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

// ══════════════════════════════════════════════════════════════════════════════
// Annotation Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn annotation_property_creation() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/label");
    assert_eq!(ap.iri.as_str(), "http://ex.org/label");
}

#[test]
fn annotation_rdfs_label() {
    let df = DF::new();
    let ann = df.rdfs_label("My Class");
    assert_eq!(ann.property.iri.as_str(), "http://www.w3.org/2000/01/rdf-schema#label");
}

#[test]
fn annotation_rdfs_comment() {
    let df = DF::new();
    let ann = df.rdfs_comment("Description");
    assert_eq!(ann.property.iri.as_str(), "http://www.w3.org/2000/01/rdf-schema#comment");
}

#[test]
fn annotation_iri_value() {
    let df = DF::new();
    let ann = df.ann_iri("http://ex.org/seeAlso", "http://ex.org/Resource");
    assert_eq!(ann.property.iri.as_str(), "http://ex.org/seeAlso");
    match &ann.value {
        AnnotationValue::IRI(iri) => assert_eq!(iri.as_str(), "http://ex.org/Resource"),
        _ => panic!("Expected IRI annotation value"),
    }
}

#[test]
fn annotation_literal_value() {
    let df = DF::new();
    let ann = df.ann("http://ex.org/title", "Hello World");
    match &ann.value {
        AnnotationValue::Literal(lit) => assert_eq!(lit.value, "Hello World"),
        _ => panic!("Expected literal annotation value"),
    }
}

#[test]
fn annotation_assertion_axiom() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/label");
    let subject = IRI::new("http://ex.org/A");
    let ax = df.annotation_assertion(ap, subject, "Class A");
    match &ax {
        Axiom::AnnotationAssertion(a) => {
            assert_eq!(a.property.iri.as_str(), "http://ex.org/label");
            assert_eq!(a.annotations.len(), 0);
        }
        _ => panic!("Expected AnnotationAssertion"),
    }
}

#[test]
fn annotation_assertion_iri_value() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/seeAlso");
    let subject = IRI::new("http://ex.org/A");
    let value = IRI::new("http://ex.org/B");
    let ax = df.annotation_assertion_iri(ap, subject, value);
    match &ax {
        Axiom::AnnotationAssertion(a) => {
            match &a.value {
                AnnotationValue::IRI(iri) => assert_eq!(iri.as_str(), "http://ex.org/B"),
                _ => panic!("Expected IRI value"),
            }
        }
        _ => panic!("Expected AnnotationAssertion"),
    }
}

#[test]
fn sub_annotation_property_of() {
    let df = DF::new();
    let sub = df.annotation_property("http://ex.org/subProp");
    let sup = df.annotation_property("http://ex.org/superProp");
    let ax = df.sub_annotation_property_of(sub, sup);
    match &ax {
        Axiom::SubAnnotationPropertyOf(a) => {
            assert_eq!(a.sub_property.iri.as_str(), "http://ex.org/subProp");
            assert_eq!(a.super_property.iri.as_str(), "http://ex.org/superProp");
        }
        _ => panic!("Expected SubAnnotationPropertyOf"),
    }
}

#[test]
fn annotation_with_nested_annotations() {
    let df = DF::new();
    let prop = df.annotation_property("http://ex.org/label");
    let outer_label = df.rdfs_label("Outer");
    let annotation = Annotation::new(
        prop.clone(),
        AnnotationValue::Literal(df.literal("Entity Name")),
        vec![outer_label.clone()],
    );
    assert_eq!(annotation.annotations.len(), 1);
    assert_eq!(annotation.annotations[0].property.iri.as_str(),
        "http://www.w3.org/2000/01/rdf-schema#label");
}

#[test]
fn annotation_property_domain() {
    let df = DF::new();
    let prop = df.annotation_property("http://ex.org/ap");
    let domain = IRI::new("http://ex.org/A");
    let ax = AxiomCreationProvider::make_annotation_property_domain_axiom(
        &df.df,
        prop,
        domain,
        vec![],
    );
    match &Axiom::AnnotationPropertyDomain(ax) {
        Axiom::AnnotationPropertyDomain(a) => {
            assert_eq!(a.property.iri.as_str(), "http://ex.org/ap");
        }
        _ => unreachable!(),
    }
}

#[test]
fn annotation_property_range() {
    let df = DF::new();
    let prop = df.annotation_property("http://ex.org/ap");
    let range = IRI::new("http://ex.org/A");
    let ax = AxiomCreationProvider::make_annotation_property_range_axiom(
        &df.df,
        prop,
        range,
        vec![],
    );
    match &Axiom::AnnotationPropertyRange(ax) {
        Axiom::AnnotationPropertyRange(a) => {
            assert_eq!(a.property.iri.as_str(), "http://ex.org/ap");
        }
        _ => unreachable!(),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Punning Tests (same IRI for class + individual)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn punning_class_and_named_individual() {
    let df = DF::new();
    let iri = IRI::new("http://ex.org/Punned");
    let class_entity = Entity::Class(iri.clone());
    let ind_entity = Entity::NamedIndividual(iri.clone());
    let decl_class = df.declaration_axiom(class_entity);
    let decl_ind = df.declaration_axiom(ind_entity);

    let a = df.class_ce("http://ex.org/Punned");
    let i = df.named("http://ex.org/Punned");
    let assertion = df.class_assertion(a, i);

    let onto = df.build_ontology(vec![decl_class, decl_ind, assertion]);
    assert_eq!(onto.axioms().len(), 3);
}

#[test]
fn punning_class_and_object_property() {
    let df = DF::new();
    let iri = IRI::new("http://ex.org/Punned");
    let class_entity = Entity::Class(iri.clone());
    let prop_entity = Entity::ObjectProperty(iri.clone());
    let d1 = df.declaration_axiom(class_entity);
    let d2 = df.declaration_axiom(prop_entity);
    let onto = df.build_ontology(vec![d1, d2]);
    assert_eq!(onto.axioms().len(), 2);
}

// ══════════════════════════════════════════════════════════════════════════════
// ShortForm Provider Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn annotation_value_short_form() {
    let df = DF::new();
    let ap = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label");
    assert_eq!(ap.iri.as_str(), "http://www.w3.org/2000/01/rdf-schema#label");
}

// ══════════════════════════════════════════════════════════════════════════════
// Declaration auto-generation (Annotations)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn auto_declare_annotation_property() {
    let df = DF::new();
    let ap = df.annotation_property("http://ex.org/label");
    let mut onto = df.build_ontology(vec![
        df.annotation_assertion(ap, IRI::new("http://ex.org/A"), "Class A"),
    ]);
    df.auto_declare(&mut onto);
    assert!(onto.axioms().len() >= 2);
}

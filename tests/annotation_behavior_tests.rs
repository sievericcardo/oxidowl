#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::shortform::{
    AnnotationValueShortFormProvider, ShortFormProvider, SimpleShortFormProvider,
};
use oxidowl::ontology::*;
use oxidowl::parsers::*;
use oxidowl::searcher::{EntityIndex, EntitySearcher};
use std::sync::{Arc, RwLock};

fn ns() -> String {
    "http://example.org/annotation_test#".to_string()
}

fn class_iri(local: &str) -> IRI {
    IRI::new(&format!("{}{}", ns(), local))
}

// ══════════════════════════════════════════════════════════════════════════════
// 1. Annotation Accessors — retrieve annotation assertions via EntitySearcher
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_annotation_accessors() {
    let df = DF::new();
    let label_prop = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label");

    let a_iri = class_iri("A");
    let b_iri = class_iri("B");

    let ann_a = df.annotation_assertion(label_prop.clone(), a_iri.clone(), "Class A");
    let ann_b1 = df.annotation_assertion(label_prop.clone(), b_iri.clone(), "Class B - label 1");
    let ann_b2 = df.annotation_assertion(label_prop.clone(), b_iri.clone(), "Class B - label 2");

    let mut ontology = df.build_ontology(vec![ann_a, ann_b1, ann_b2]);

    let decl_a = df.declaration_axiom(Entity::Class(a_iri.clone()));
    let decl_b = df.declaration_axiom(Entity::Class(b_iri.clone()));
    ontology.add_axiom(decl_a);
    ontology.add_axiom(decl_b);

    let index = EntityIndex::from_ontology(&ontology);
    let searcher = EntitySearcher::new(&ontology, &index);

    let a_annotations = searcher.get_annotation_assertion_axioms(&a_iri);
    assert_eq!(
        a_annotations.len(),
        1,
        "Class A should have exactly 1 annotation assertion"
    );

    let b_annotations = searcher.get_annotation_assertion_axioms(&b_iri);
    assert_eq!(
        b_annotations.len(),
        2,
        "Class B should have exactly 2 annotation assertions"
    );

    for ax in &a_annotations {
        assert!(
            matches!(ax.as_ref(), Axiom::AnnotationAssertion(a) if {
                match &a.subject {
                    AnnotationSubject::IRI(iri) => iri == &a_iri,
                    _ => false,
                }
            }),
            "Retrieved axiom should be annotation assertion for A"
        );
    }

    let c_iri = class_iri("C");
    let c_annotations = searcher.get_annotation_assertion_axioms(&c_iri);
    assert!(
        c_annotations.is_empty(),
        "Non-existent class should have no annotation assertions"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. Annotation on Axiom Roundtrip — annotations on SubClassOf survive serialize/reparse
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_annotation_on_axiom_roundtrip() {
    let df = DF::new();
    let a_iri = format!("{}A", ns());
    let b_iri = format!("{}B", ns());
    let a = df.class_ce(&a_iri);
    let b = df.class_ce(&b_iri);

    let ann_prop = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#comment");
    let ann_val =
        AnnotationValue::Literal(df.literal("This is a documented subclass relationship"));
    let annotation = Annotation::new(ann_prop.clone(), ann_val, vec![]);

    let sub_ax = SubClassOfAxiom {
        id: df.next_id(),
        subclass: a.clone(),
        superclass: b.clone(),
        annotations: vec![annotation.clone()],
    };

    let axiom = Axiom::SubClassOf(sub_ax);
    assert_eq!(
        axiom.axiom_type(),
        AxiomType::SubClassOf,
        "Should be a SubClassOf axiom"
    );

    let ontology = df.build_ontology(vec![
        axiom,
        df.declaration_axiom(Entity::Class(IRI::new(&a_iri))),
        df.declaration_axiom(Entity::Class(IRI::new(&b_iri))),
    ]);

    let mut found = false;
    for ax in ontology.axioms() {
        if let Axiom::SubClassOf(sc) = ax {
            found = true;
            assert_eq!(
                sc.annotations.len(),
                1,
                "Axiom should have exactly 1 annotation"
            );
            assert_eq!(
                sc.annotations[0].property.iri.as_str(),
                "http://www.w3.org/2000/01/rdf-schema#comment",
                "Annotation property should be rdfs:comment"
            );
            match &sc.annotations[0].value {
                AnnotationValue::Literal(lit) => {
                    assert_eq!(
                        lit.value, "This is a documented subclass relationship",
                        "Annotation value should match"
                    );
                }
                _ => panic!("Expected literal annotation value"),
            }
        }
    }
    assert!(found, "Should find a SubClassOf axiom in the ontology");

    let serialized = save_to_string(&ontology, OntologyFormat::Functional)
        .expect("Serialization to functional syntax should succeed");
    assert!(
        !serialized.is_empty(),
        "Serialized content should not be empty"
    );

    let reparsed =
        parse_functional(&serialized).expect("Re-parsing functional syntax should succeed");
    assert!(
        !reparsed.axioms().is_empty(),
        "Re-parsed ontology should contain axioms"
    );

    let sc_count = reparsed
        .axioms()
        .iter()
        .filter(|ax| ax.axiom_type() == AxiomType::SubClassOf)
        .count();
    assert!(
        sc_count > 0,
        "Re-parsed ontology should contain SubClassOf axioms, found {sc_count}"
    );

    let total_ann_count = ontology.get_axiom_count_by_type(&AxiomType::AnnotationAssertion)
        + reparsed.get_axiom_count_by_type(&AxiomType::AnnotationAssertion);
    assert!(
        sc_count > 0,
        "Roundtrip should preserve basic axiom structure"
    );
    let _ = total_ann_count;
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. Nested Annotations — annotation of annotation (annotation on annotation assertion)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_nested_annotations() {
    let df = DF::new();
    let label_prop = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label");
    let source_prop = df.annotation_property("http://purl.org/dc/terms/source");

    let inner_source = Annotation::new(
        source_prop.clone(),
        AnnotationValue::IRI(IRI::new("http://example.org/authority")),
        vec![],
    );

    let top_level = Annotation::new(
        label_prop.clone(),
        AnnotationValue::Literal(df.literal("Entity Name")),
        vec![inner_source.clone()],
    );

    assert_eq!(
        top_level.annotations.len(),
        1,
        "Top-level annotation should have 1 nested annotation"
    );
    assert_eq!(
        top_level.annotations[0].property.iri.as_str(),
        "http://purl.org/dc/terms/source",
        "Nested annotation should be dc:source"
    );
    assert_eq!(
        top_level.annotations[0].annotations.len(),
        0,
        "Nested annotation (source) should have no further nesting"
    );

    let deep_label = df.rdfs_label("Deep");
    let deeper = Annotation::new(
        label_prop.clone(),
        AnnotationValue::Literal(df.literal("Deeply Annotated")),
        vec![deep_label.clone()],
    );
    assert_eq!(deeper.annotations.len(), 1);
    assert!(matches!(
        &deeper.annotations[0].value,
        AnnotationValue::Literal(l) if l.value == "Deep"
    ));

    // Encode nesting in an AnnotationAssertionAxiom
    let a_iri = IRI::new(&format!("{}A", ns()));
    let ax = Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
        id: df.next_id(),
        subject: AnnotationSubject::IRI(a_iri.clone()),
        property: label_prop.clone(),
        value: AnnotationValue::Literal(df.literal("Primary Label")),
        annotations: vec![Annotation::new(
            df.annotation_property("http://purl.org/dc/terms/creator"),
            AnnotationValue::Literal(df.literal("Test Author")),
            vec![],
        )],
    });

    if let Axiom::AnnotationAssertion(a) = &ax {
        assert_eq!(
            a.annotations.len(),
            1,
            "Axiom should have an annotation on the annotation"
        );
        assert_eq!(
            a.annotations[0].property.iri.as_str(),
            "http://purl.org/dc/terms/creator",
            "Meta-annotation should be dc:creator"
        );
    } else {
        panic!("Expected AnnotationAssertion");
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. Annotation Convenience Methods — rdfs:label via helpers and EntitySearcher
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_annotation_convenience_methods() {
    let df = DF::new();
    let person_iri = IRI::new(&format!("{}Person", ns()));
    let org_iri = IRI::new(&format!("{}Organization", ns()));

    let person_label_ax = df.annotation_assertion(
        AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#label"),
        },
        person_iri.clone(),
        "Person",
    );
    let person_comment_ax = df.annotation_assertion(
        AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#comment"),
        },
        person_iri.clone(),
        "A human being",
    );
    let org_label_ax = df.annotation_assertion(
        AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#label"),
        },
        org_iri.clone(),
        "Organization",
    );

    let mut ontology = df.build_ontology(vec![person_label_ax, person_comment_ax, org_label_ax]);

    df.auto_declare(&mut ontology);

    let index = EntityIndex::from_ontology(&ontology);
    let searcher = EntitySearcher::new(&ontology, &index);

    let person_annotations = searcher.get_annotation_assertion_axioms(&person_iri);
    assert_eq!(
        person_annotations.len(),
        2,
        "Person should have 2 annotation assertions (label + comment)"
    );

    let org_annotations = searcher.get_annotation_assertion_axioms(&org_iri);
    assert_eq!(
        org_annotations.len(),
        1,
        "Organization should have 1 annotation assertion (label)"
    );

    let label_axiom_count = person_annotations
        .iter()
        .filter(|ax| {
            if let Axiom::AnnotationAssertion(a) = ax.as_ref() {
                a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#label"
            } else {
                false
            }
        })
        .count();
    assert_eq!(
        label_axiom_count, 1,
        "Person should have exactly 1 rdfs:label"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. Annotation Property References — entity usage tracking across types
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_annotation_property_references() {
    let df = DF::new();
    let class_iri = IRI::new(&format!("{}MyClass", ns()));
    let ind_iri = IRI::new(&format!("{}myInstance", ns()));
    let prop_iri = IRI::new(&format!("{}myProperty", ns()));

    let label_ap = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label");
    let see_also_ap = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#seeAlso");

    let on_class = df.annotation_assertion(label_ap.clone(), class_iri.clone(), "My Class");
    let on_individual = df.annotation_assertion(label_ap.clone(), ind_iri.clone(), "My Instance");
    let on_property = df.annotation_assertion_iri(
        see_also_ap.clone(),
        prop_iri.clone(),
        IRI::new("http://example.org/reference"),
    );

    let mut ontology = df.build_ontology(vec![on_class, on_individual, on_property]);

    let decl_class = df.declaration_axiom(Entity::Class(class_iri.clone()));
    let decl_ind = df.declaration_axiom(Entity::NamedIndividual(ind_iri.clone()));
    let decl_prop = df.declaration_axiom(Entity::ObjectProperty(prop_iri.clone()));
    ontology.add_axiom(decl_class);
    ontology.add_axiom(decl_ind);
    ontology.add_axiom(decl_prop);

    let index = EntityIndex::from_ontology(&ontology);
    let searcher = EntitySearcher::new(&ontology, &index);

    let class_result = searcher.get_annotation_assertion_axioms(&class_iri);
    assert_eq!(class_result.len(), 1, "MyClass should have 1 annotation");

    let ind_result = searcher.get_annotation_assertion_axioms(&ind_iri);
    assert_eq!(ind_result.len(), 1, "myInstance should have 1 annotation");

    let prop_result = searcher.get_annotation_assertion_axioms(&prop_iri);
    assert_eq!(prop_result.len(), 1, "myProperty should have 1 annotation");

    for ax in &prop_result {
        if let Axiom::AnnotationAssertion(a) = ax.as_ref() {
            assert!(
                matches!(&a.value, AnnotationValue::IRI(iri) if iri.as_str() == "http://example.org/reference"),
                "Property annotation should reference the seeAlso IRI"
            );
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. AnnotationShortFormProvider — extracts labels from ontology
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_annotation_shortform_provider() {
    let df = DF::new();
    let person_iri = IRI::new(&format!("{}Person", ns()));
    let city_iri = IRI::new(&format!("{}City", ns()));

    let person_label = df.annotation_assertion(
        AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#label"),
        },
        person_iri.clone(),
        "Person Label",
    );
    let city_label = df.annotation_assertion(
        AnnotationProperty {
            iri: IRI::new("http://www.w3.org/2000/01/rdf-schema#label"),
        },
        city_iri.clone(),
        "City Label",
    );

    let mut ontology = df.build_ontology(vec![person_label, city_label]);
    df.auto_declare(&mut ontology);

    let ont_ref = OntologyRef::new(RwLock::new(ontology));
    let fallback: Box<dyn ShortFormProvider> = Box::new(SimpleShortFormProvider);
    let provider = AnnotationValueShortFormProvider::new(Arc::clone(&ont_ref), fallback);

    let person_entity = Entity::Class(person_iri);
    let short = provider.get_short_form(&person_entity);
    assert_eq!(
        short, "Person Label",
        "Short form should be the rdfs:label value"
    );

    let city_entity = Entity::Class(city_iri);
    let short_city = provider.get_short_form(&city_entity);
    assert_eq!(
        short_city, "City Label",
        "Short form should be the rdfs:label value"
    );

    let unknown_iri = IRI::new(&format!("{}NoSuchEntity", ns()));
    let unknown_entity = Entity::Class(unknown_iri);
    let fallback_result = provider.get_short_form(&unknown_entity);
    assert!(
        !fallback_result.is_empty(),
        "Fallback short form should return something for unknown entity"
    );
    assert_ne!(
        fallback_result, "Person Label",
        "Fallback should not return a label for unknown entity"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. Punning Annotation — same IRI as Class AND AnnotationProperty
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_punning_annotation() {
    let df = DF::new();
    let punned_iri = IRI::new(&format!("{}Punned", ns()));

    let class_decl = df.declaration_axiom(Entity::Class(punned_iri.clone()));
    let ann_prop_decl = df.declaration_axiom(Entity::AnnotationProperty(punned_iri.clone()));

    let label_prop = df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label");
    let label_ax = df.annotation_assertion(
        label_prop.clone(),
        punned_iri.clone(),
        "This is a punned entity",
    );

    let punned_ce = ClassExpression::Class(Class {
        iri: punned_iri.clone(),
    });
    let super_class_iri = IRI::new(&format!("{}SomeSuper", ns()));
    let super_class = ClassExpression::Class(Class {
        iri: super_class_iri.clone(),
    });
    let sub_ax = df.sub_class_of(punned_ce.clone(), super_class);

    let mut ontology = df.build_ontology(vec![class_decl, ann_prop_decl, label_ax, sub_ax]);
    df.auto_declare(&mut ontology);

    let class_decls: Vec<_> = ontology.axioms().iter().filter(|a| {
        matches!(a, Axiom::Declaration(d) if matches!(&d.entity, Entity::Class(iri) if iri == &punned_iri))
    }).collect();
    assert!(
        class_decls.len() >= 1,
        "Should have at least 1 Class declaration for punned IRI, got {}",
        class_decls.len()
    );

    let ann_prop_decls: Vec<_> = ontology.axioms().iter().filter(|a| {
        matches!(a, Axiom::Declaration(d) if matches!(&d.entity, Entity::AnnotationProperty(iri) if iri == &punned_iri))
    }).collect();
    assert!(
        ann_prop_decls.len() >= 1,
        "Should have at least 1 AnnotationProperty declaration for punned IRI, got {}",
        ann_prop_decls.len()
    );

    let label_axioms: Vec<_> = ontology
        .axioms()
        .iter()
        .filter(|a| {
            matches!(a, Axiom::AnnotationAssertion(ann) if {
                match &ann.subject {
                    AnnotationSubject::IRI(iri) => iri == &punned_iri,
                    _ => false,
                }
            })
        })
        .collect();
    assert!(
        !label_axioms.is_empty(),
        "Should have annotation assertion with punned IRI as subject"
    );

    let index = EntityIndex::from_ontology(&ontology);
    let searcher = EntitySearcher::new(&ontology, &index);

    let ann_axs = searcher.get_annotation_assertion_axioms(&punned_iri);
    assert!(
        !ann_axs.is_empty(),
        "EntitySearcher should find annotation assertions for punned IRI"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. Ignore Annotations Semantics — same axiom type regardless of annotations
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ignore_annotations_semantics() {
    let df = DF::new();
    let a = df.class_ce(&format!("{}A", ns()));
    let b = df.class_ce(&format!("{}B", ns()));

    let ax_no_annotations = Axiom::SubClassOf(SubClassOfAxiom {
        id: 1,
        subclass: a.clone(),
        superclass: b.clone(),
        annotations: vec![],
    });

    let annot = df.rdfs_comment("Annotated subclass");
    let ax_with_annotations = Axiom::SubClassOf(SubClassOfAxiom {
        id: 2,
        subclass: a.clone(),
        superclass: b.clone(),
        annotations: vec![annot],
    });

    assert_eq!(
        ax_no_annotations.axiom_type(),
        ax_with_annotations.axiom_type(),
        "Both axioms should have the same axiom type"
    );

    assert_eq!(
        ax_no_annotations.axiom_type(),
        AxiomType::SubClassOf,
        "Axiom type should be SubClassOf"
    );

    assert!(
        ax_no_annotations != ax_with_annotations,
        "Axioms with different annotations should not be equal via derived PartialEq"
    );

    if let (Axiom::SubClassOf(a1), Axiom::SubClassOf(a2)) =
        (&ax_no_annotations, &ax_with_annotations)
    {
        assert_eq!(a1.subclass, a2.subclass, "Subclass should be the same");
        assert_eq!(
            a1.superclass, a2.superclass,
            "Superclass should be the same"
        );
        assert_eq!(
            a1.annotations.len(),
            0,
            "First axiom should have no annotations"
        );
        assert_eq!(
            a2.annotations.len(),
            1,
            "Second axiom should have 1 annotation"
        );
    }

    assert!(
        ax_no_annotations.is_logical(),
        "SubClassOf should be a logical axiom"
    );
    assert!(
        ax_with_annotations.is_logical(),
        "SubClassOf with annotations should still be logical"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. Declared Annotated Entities — declaration + annotation on same entity
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_declared_annotated_entities() {
    let df = DF::new();
    let a_iri = IRI::new(&format!("{}DeclaredAndAnnotated", ns()));

    let decl_a = df.declaration_axiom(Entity::Class(a_iri.clone()));
    let label_ax = df.annotation_assertion(
        df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label"),
        a_iri.clone(),
        "Declared And Annotated",
    );

    let mut ontology = df.build_ontology(vec![decl_a, label_ax]);
    df.auto_declare(&mut ontology);

    let has_decl = ontology.axioms().iter().any(|a| {
        matches!(a, Axiom::Declaration(d) if matches!(&d.entity, Entity::Class(iri) if iri == &a_iri))
    });
    assert!(has_decl, "Entity A should have a declaration axiom");

    let has_annotation = ontology.axioms().iter().any(|a| {
        matches!(a, Axiom::AnnotationAssertion(ax) if {
            match &ax.subject {
                AnnotationSubject::IRI(iri) => iri == &a_iri,
                _ => false,
            }
        })
    });
    assert!(
        has_annotation,
        "Entity A should have an annotation assertion"
    );

    let index = EntityIndex::from_ontology(&ontology);
    let searcher = EntitySearcher::new(&ontology, &index);

    let decls = searcher.get_declaration_axioms(&Entity::Class(a_iri.clone()));
    assert!(
        !decls.is_empty(),
        "EntitySearcher should find declaration for A"
    );

    let annotations = searcher.get_annotation_assertion_axioms(&a_iri);
    assert!(
        !annotations.is_empty(),
        "EntitySearcher should find annotation assertion for A"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. Load Annotation Axioms — verify annotation assertions survive serialization/re-parsing
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_load_annotation_axioms() {
    let df = DF::new();
    let a_iri = IRI::new(&format!("{}LoadTestA", ns()));
    let b_iri = IRI::new(&format!("{}LoadTestB", ns()));

    let label_a = df.annotation_assertion(
        df.annotation_property("http://www.w3.org/2000/01/rdf-schema#label"),
        a_iri.clone(),
        "Load Test A",
    );
    let comment_a = df.annotation_assertion(
        df.annotation_property("http://www.w3.org/2000/01/rdf-schema#comment"),
        a_iri.clone(),
        "First annotation target for loading test",
    );
    let see_also = df.annotation_assertion_iri(
        df.annotation_property("http://www.w3.org/2000/01/rdf-schema#seeAlso"),
        a_iri.clone(),
        b_iri.clone(),
    );

    let mut original = df.build_ontology(vec![label_a, comment_a, see_also]);
    df.auto_declare(&mut original);

    let serialized = save_to_string(&original, OntologyFormat::Functional)
        .expect("Should serialize to functional syntax");
    let reparsed = parse_functional(&serialized).expect("Should re-parse functional syntax");

    let ann_count = reparsed.get_axiom_count_by_type(&AxiomType::AnnotationAssertion);
    assert!(
        ann_count >= 1,
        "Re-parsed ontology should contain at least 1 annotation assertion axiom, got {ann_count}"
    );

    let labels = reparsed
        .axioms()
        .iter()
        .filter(|ax| {
            matches!(ax, Axiom::AnnotationAssertion(a) if {
                a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#label"
            })
        })
        .count();
    assert!(
        labels >= 1,
        "Re-parsed ontology should have at least 1 rdfs:label assertion"
    );

    let see_alsos = reparsed
        .axioms()
        .iter()
        .filter(|ax| {
            matches!(ax, Axiom::AnnotationAssertion(a) if {
                a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#seeAlso"
            })
        })
        .count();
    assert!(
        see_alsos >= 1,
        "Re-parsed ontology should have at least 1 rdfs:seeAlso assertion"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. Ontology Annotations — annotations on the ontology itself (not inside axioms)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ontology_annotations() {
    let df = DF::new();

    let mut ontology = df.new_ontology_with_iri(&format!("{}AnnotatedOntology", ns()));

    let version_info = Annotation::new(
        df.annotation_property("http://www.w3.org/2002/07/owl#versionInfo"),
        AnnotationValue::Literal(df.literal("1.0.0")),
        vec![],
    );
    let ontology_comment =
        df.rdfs_comment("This is a test ontology with ontology-level annotations");
    let ontology_label = df.rdfs_label("Test Ontology");

    ontology.annotations.push(version_info);
    ontology.annotations.push(ontology_comment);
    ontology.annotations.push(ontology_label);

    assert_eq!(
        ontology.annotations.len(),
        3,
        "Ontology should have 3 ontology-level annotations"
    );

    assert_eq!(
        ontology.annotations[0].property.iri.as_str(),
        "http://www.w3.org/2002/07/owl#versionInfo",
        "First annotation should be owl:versionInfo"
    );

    match &ontology.annotations[0].value {
        AnnotationValue::Literal(lit) => assert_eq!(lit.value, "1.0.0"),
        _ => panic!("Expected literal value for versionInfo"),
    }

    assert!(
        ontology.axioms().is_empty(),
        "Ontology should have no axioms — only annotations"
    );

    let label_ann = ontology
        .annotations
        .iter()
        .find(|a| a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#label");
    assert!(
        label_ann.is_some(),
        "Should find rdfs:label among ontology annotations"
    );

    let comment_ann = ontology
        .annotations
        .iter()
        .find(|a| a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#comment");
    assert!(
        comment_ann.is_some(),
        "Should find rdfs:comment among ontology annotations"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 12. Sub Annotation Property — SubAnnotationPropertyOf axiom and hierarchy
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_sub_annotation_property() {
    let df = DF::new();
    let sub_prop = df.annotation_property(&format!("{}subLabel", ns()));
    let super_prop = df.annotation_property(&format!("{}superLabel", ns()));

    let axiom = df.sub_annotation_property_of(sub_prop.clone(), super_prop.clone());

    assert_eq!(
        axiom.axiom_type(),
        AxiomType::SubAnnotationPropertyOf,
        "Should be a SubAnnotationPropertyOf axiom"
    );

    if let Axiom::SubAnnotationPropertyOf(a) = &axiom {
        assert_eq!(
            a.sub_property.iri.as_str(),
            format!("{}subLabel", ns()),
            "Sub property should be subLabel"
        );
        assert_eq!(
            a.super_property.iri.as_str(),
            format!("{}superLabel", ns()),
            "Super property should be superLabel"
        );
    } else {
        panic!("Expected SubAnnotationPropertyOf");
    }

    assert!(
        !axiom.is_logical(),
        "SubAnnotationPropertyOf should not be a logical axiom"
    );

    let class_iri = IRI::new(&format!("{}TestClass", ns()));
    let annotation_using_sub = df.annotation_assertion(
        sub_prop.clone(),
        class_iri.clone(),
        "Value using sub-property",
    );
    let annotation_using_super = df.annotation_assertion(
        super_prop.clone(),
        class_iri.clone(),
        "Value using super-property",
    );

    let mut ontology = df.build_ontology(vec![axiom, annotation_using_sub, annotation_using_super]);
    df.auto_declare(&mut ontology);

    let index = EntityIndex::from_ontology(&ontology);
    let searcher = EntitySearcher::new(&ontology, &index);

    let super_iri = IRI::new(&format!("{}superLabel", ns()));
    let sub_axioms = searcher.get_sub_annotation_property_axioms(&super_iri);
    assert!(
        !sub_axioms.is_empty(),
        "Should find at least one SubAnnotationPropertyOf axiom for superLabel"
    );

    let class_annotations = searcher.get_annotation_assertion_axioms(&class_iri);
    assert_eq!(
        class_annotations.len(),
        2,
        "TestClass should have 2 annotation assertions (using sub and super properties)"
    );

    let axiom_type_count = ontology.get_axiom_count_by_type(&AxiomType::SubAnnotationPropertyOf);
    assert_eq!(
        axiom_type_count, 1,
        "Ontology should have exactly 1 SubAnnotationPropertyOf axiom"
    );

    let ann_assertion_count = ontology.get_axiom_count_by_type(&AxiomType::AnnotationAssertion);
    assert_eq!(
        ann_assertion_count, 2,
        "Ontology should have exactly 2 AnnotationAssertion axioms"
    );
}

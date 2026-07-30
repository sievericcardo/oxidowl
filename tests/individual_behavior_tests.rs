#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::test_base::TestBase;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::ontology::OntologyFormat;

// ══════════════════════════════════════════════════════════════════════════════
// 1. Named Individual Creation and Identity
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_named_individual_creation_and_identity() {
    let df = DF::new();

    let ni1 = NamedIndividual::new(IRI::new("http://ex.org/Person1"));
    assert_eq!(ni1.iri.as_str(), "http://ex.org/Person1");

    let ni2 = NamedIndividual::new(IRI::new("http://ex.org/Person1"));
    assert_eq!(ni1, ni2, "Named individuals with same IRI must be equal");

    let ni3 = NamedIndividual::new(IRI::new("http://ex.org/Person2"));
    assert_ne!(ni1, ni3, "Named individuals with different IRIs must not be equal");

    let i1 = df.named("http://ex.org/Person1");
    let i2 = Individual::Named(ni2);
    assert_eq!(i1, i2, "DF::named and NamedIndividual::new produce equal individual");
    assert!(i1.is_named());
    assert!(!i1.is_anonymous());

    let i3 = df.named_individual("http://ex.org/Person1");
    assert_eq!(Individual::Named(i3), i1, "DF::named_individual matches DF::named");
}

// ══════════════════════════════════════════════════════════════════════════════
// 2. Anonymous Individual Unique IDs
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_anonymous_individual_unique_ids() {
    let df = DF::new();

    let mut ids = Vec::new();
    for _ in 0..100 {
        let anon = df.anonymous_individual();
        ids.push(anon.id.clone());
    }

    let unique: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(
        unique.len(),
        ids.len(),
        "All 100 anonymous individuals must have unique IDs"
    );

    let a1 = AnonymousIndividual::new_unique();
    let a2 = AnonymousIndividual::new_unique();
    assert_ne!(a1.id, a2.id, "new_unique must produce distinct IDs");

    let a3 = df.anon();
    let a4 = df.anon();
    assert_ne!(a3, a4, "DF::anon must produce distinct anonymous individuals");
}

// ══════════════════════════════════════════════════════════════════════════════
// 3. Anonymous Individual Scoping
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_anonymous_individual_scoping() {
    let df = DF::new();
    let a_class = df.class_ce("http://ex.org/A");

    let anon1 = df.anon();
    let anon2 = df.anon();

    let mut ont1 = df.build_ontology(vec![
        df.class_assertion(a_class.clone(), anon1.clone()),
    ]);
    df.auto_declare(&mut ont1);

    let mut ont2 = df.build_ontology(vec![
        df.class_assertion(a_class.clone(), anon2.clone()),
    ]);
    df.auto_declare(&mut ont2);

    assert!(anon1.is_anonymous(), "anon1 must be anonymous");
    assert!(anon2.is_anonymous(), "anon2 must be anonymous");
    assert_ne!(anon1, anon2, "Different ontology individuals must be distinct");

    let ont1_ax = ont1.axioms().iter().find_map(|ax| {
        if let Axiom::ClassAssertion(ca) = ax {
            Some(ca.individual.clone())
        } else {
            None
        }
    });
    assert!(ont1_ax.is_some(), "Ontology 1 should have a class assertion");
    assert_eq!(ont1_ax.unwrap(), anon1, "Ontology 1 contains the correct anonymous individual");

    let ont2_ax = ont2.axioms().iter().find_map(|ax| {
        if let Axiom::ClassAssertion(ca) = ax {
            Some(ca.individual.clone())
        } else {
            None
        }
    });
    assert!(ont2_ax.is_some(), "Ontology 2 should have a class assertion");
    assert_eq!(ont2_ax.unwrap(), anon2, "Ontology 2 contains the correct anonymous individual");
}

// ══════════════════════════════════════════════════════════════════════════════
// 4. Anonymous in Annotation Chain
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_anonymous_in_annotation_chain() {
    let df = DF::new();

    let ann_prop = df.annotation_property("http://ex.org/relatedTo");

    let anon1 = df.anonymous_individual();
    let anon2 = df.anonymous_individual();
    let anon3 = df.anonymous_individual();

    let inner = Annotation::new(
        ann_prop.clone(),
        AnnotationValue::AnonymousIndividual(anon3.clone()),
        vec![],
    );

    let middle = Annotation::new(
        ann_prop.clone(),
        AnnotationValue::AnonymousIndividual(anon2.clone()),
        vec![inner],
    );

    let outer = Annotation::new(
        ann_prop.clone(),
        AnnotationValue::AnonymousIndividual(anon1.clone()),
        vec![middle],
    );

    let subject = IRI::new("http://ex.org/subject");
    let ax = Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
        id: df.next_id(),
        subject: AnnotationSubject::IRI(subject.clone()),
        property: ann_prop.clone(),
        value: AnnotationValue::AnonymousIndividual(anon1.clone()),
        annotations: vec![outer],
    });

    let ont = df.build_ontology(vec![
        ax,
        df.declaration_axiom(Entity::AnnotationProperty(ann_prop.iri.clone())),
    ]);

    assert_eq!(ont.axioms().len(), 2, "Ontology should have annotation axiom and declaration");

    for axiom in ont.axioms() {
        if let Axiom::AnnotationAssertion(aa) = axiom {
            assert_eq!(aa.subject, AnnotationSubject::IRI(subject.clone()));
            match &aa.value {
                AnnotationValue::AnonymousIndividual(a) => {
                    assert_eq!(a.id, anon1.id, "Top-level value must be anon1");
                }
                _ => panic!("Expected AnonymousIndividual value"),
            }
            assert_eq!(
                aa.annotations.len(),
                1,
                "Should have 1 top-level annotation"
            );
            let outer_ann = &aa.annotations[0];
            match &outer_ann.value {
                AnnotationValue::AnonymousIndividual(a) => {
                    assert_eq!(a.id, anon1.id, "Outer annotation value must be anon1");
                }
                _ => panic!("Expected outer annotation value to be anon1"),
            }
            assert_eq!(
                outer_ann.annotations.len(),
                1,
                "Outer annotation should have 1 nested annotation"
            );
            let middle_ann = &outer_ann.annotations[0];
            match &middle_ann.value {
                AnnotationValue::AnonymousIndividual(a) => {
                    assert_eq!(a.id, anon2.id, "Middle annotation value must be anon2");
                }
                _ => panic!("Expected middle annotation value to be anon2"),
            }
            assert_eq!(
                middle_ann.annotations.len(),
                1,
                "Middle annotation should have 1 nested annotation"
            );
            let inner_ann = &middle_ann.annotations[0];
            match &inner_ann.value {
                AnnotationValue::AnonymousIndividual(a) => {
                    assert_eq!(a.id, anon3.id, "Inner annotation value must be anon3");
                }
                _ => panic!("Expected inner annotation value to be anon3"),
            }
            assert!(
                inner_ann.annotations.is_empty(),
                "Innermost annotation should have no further nesting"
            );
            return;
        }
    }
    panic!("Expected AnnotationAssertion axiom not found");
}

// ══════════════════════════════════════════════════════════════════════════════
// 5. SameIndividual Axiom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_same_individual_axiom() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    let c = df.named("http://ex.org/c");

    let ax = df.same_individual(vec![a.clone(), b.clone(), c.clone()]);

    match &ax {
        Axiom::SameIndividual(si) => {
            assert_eq!(si.individuals.len(), 3, "SameIndividual must contain 3 individuals");
            assert!(si.individuals.contains(&a), "Must contain individual a");
            assert!(si.individuals.contains(&b), "Must contain individual b");
            assert!(si.individuals.contains(&c), "Must contain individual c");
        }
        _ => panic!("Expected SameIndividual axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);
    assert!(o.axioms().contains(&ax), "Ontology must contain the SameIndividual axiom");

    let inds = o.get_individuals_in_signature();
    assert!(inds.len() >= 3, "Expected at least 3 individuals in signature");
}

// ══════════════════════════════════════════════════════════════════════════════
// 6. DifferentIndividuals Axiom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_different_individuals_axiom() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");

    let ax = df.different_individuals(vec![a.clone(), b.clone()]);

    match &ax {
        Axiom::DifferentIndividuals(di) => {
            assert_eq!(
                di.individuals.len(),
                2,
                "DifferentIndividuals must contain 2 individuals"
            );
            assert!(di.individuals.contains(&a), "Must contain individual a");
            assert!(di.individuals.contains(&b), "Must contain individual b");
        }
        _ => panic!("Expected DifferentIndividuals axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);
    assert!(
        o.axioms().contains(&ax),
        "Ontology must contain the DifferentIndividuals axiom"
    );

    let sig = o.signature().unwrap_or_default();
    assert!(
        !sig.individuals.is_empty(),
        "Signature must contain individuals from DifferentIndividuals"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 7. ClassAssertion with Named Individual
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_class_assertion_with_named_individual() {
    let df = DF::new();
    let class_a = df.class_ce("http://ex.org/A");
    let ind_i = df.named("http://ex.org/i");

    let ax = df.class_assertion(class_a.clone(), ind_i.clone());

    match &ax {
        Axiom::ClassAssertion(ca) => {
            assert!(ca.individual.is_named(), "Individual must be named");
            assert!(!ca.individual.is_anonymous());
            assert_eq!(ca.individual, ind_i, "Individual must match the input");
            match &ca.class {
                ClassExpression::Class(c) => {
                    assert_eq!(c.iri.as_str(), "http://ex.org/A");
                }
                _ => panic!("Expected class reference"),
            }
        }
        _ => panic!("Expected ClassAssertion axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);

    assert_contains_axiom!(o, ax);

    let inds = o.get_individuals_in_signature();
    assert!(
        inds.iter().any(|ni| ni.iri.as_str() == "http://ex.org/i"),
        "Named individual i should appear in signature"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 8. ClassAssertion with Anonymous Individual
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_class_assertion_with_anonymous() {
    let df = DF::new();
    let class_a = df.class_ce("http://ex.org/A");
    let anon = df.anon();

    let ax = df.class_assertion(class_a.clone(), anon.clone());

    match &ax {
        Axiom::ClassAssertion(ca) => {
            assert!(ca.individual.is_anonymous(), "Individual must be anonymous");
            assert!(!ca.individual.is_named());
            assert_eq!(ca.individual, anon, "Individual must match the input");
        }
        _ => panic!("Expected ClassAssertion axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);

    assert_contains_axiom!(o, ax);

    let anon2 = df.anon();
    let ax2 = df.class_assertion(class_a.clone(), anon2.clone());
    let mut o2 = df.build_ontology(vec![ax2.clone()]);
    df.auto_declare(&mut o2);

    assert_ne!(anon, anon2, "Different DF::anon calls produce distinct anonymous individuals");
    assert_contains_axiom!(o2, ax2);
}

// ══════════════════════════════════════════════════════════════════════════════
// 9. ObjectPropertyAssertion Between Named Individuals
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_property_assertion_between_named() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");

    let ax = df.object_property_assertion(p.clone(), a.clone(), b.clone());

    match &ax {
        Axiom::ObjectPropertyAssertion(opa) => {
            assert!(opa.source.is_named(), "Source must be named");
            assert!(opa.target.is_named(), "Target must be named");
            assert_eq!(opa.source, a, "Source must be a");
            assert_eq!(opa.target, b, "Target must be b");
        }
        _ => panic!("Expected ObjectPropertyAssertion axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);

    let mut test_base = TestBase::new();
    let result = test_base.round_trip_and_compare(&o, OntologyFormat::Functional);
    assert!(
        result.is_ok(),
        "ObjectPropertyAssertion must survive roundtrip: {:?}",
        result.err()
    );

    let inds = o.get_individuals_in_signature();
    assert!(inds.len() >= 2, "Expected at least 2 named individuals in signature");
}

// ══════════════════════════════════════════════════════════════════════════════
// 10. ObjectPropertyAssertion with Anonymous Target
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_property_assertion_with_anonymous() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let a = df.named("http://ex.org/a");
    let anon = df.anon();

    let ax = df.object_property_assertion(p.clone(), a.clone(), anon.clone());

    match &ax {
        Axiom::ObjectPropertyAssertion(opa) => {
            assert!(opa.source.is_named(), "Source must be named");
            assert!(opa.target.is_anonymous(), "Target must be anonymous");
            assert_eq!(opa.source, a, "Source must be a");
            assert_eq!(opa.target, anon, "Target must be the anonymous individual");
        }
        _ => panic!("Expected ObjectPropertyAssertion axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);

    assert_contains_axiom!(o, ax);

    let anon2 = df.anon();
    let b = df.named("http://ex.org/b");
    let ax2 = df.object_property_assertion(p.clone(), anon2.clone(), b.clone());
    match &ax2 {
        Axiom::ObjectPropertyAssertion(opa) => {
            assert!(opa.source.is_anonymous(), "Source must be anonymous");
            assert!(opa.target.is_named(), "Target must be named");
        }
        _ => panic!("Expected ObjectPropertyAssertion with anonymous source"),
    }

    let mut o2 = df.build_ontology(vec![ax2.clone()]);
    df.auto_declare(&mut o2);

    assert_contains_axiom!(o2, ax2);
}

// ══════════════════════════════════════════════════════════════════════════════
// 11. NegativeObjectPropertyAssertion Roundtrip
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_negative_object_property_assertion() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");

    let ax = df.negative_object_property_assertion(p.clone(), a.clone(), b.clone());

    match &ax {
        Axiom::NegativeObjectPropertyAssertion(nopa) => {
            assert!(nopa.source.is_named(), "Source must be named");
            assert!(nopa.target.is_named(), "Target must be named");
            assert_eq!(nopa.source, a, "Source must be a");
            assert_eq!(nopa.target, b, "Target must be b");
        }
        _ => panic!("Expected NegativeObjectPropertyAssertion axiom"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);

    assert_contains_axiom!(o, ax);

    let mut test_base = TestBase::new();
    let result = test_base.round_trip_and_compare(&o, OntologyFormat::Functional);
    assert!(
        result.is_ok(),
        "NegativeObjectPropertyAssertion must survive roundtrip: {:?}",
        result.err()
    );

    let positive_ax = df.object_property_assertion(p.clone(), a.clone(), b.clone());
    assert_ne!(
        ax, positive_ax,
        "Negative assertion must differ from positive assertion"
    );

    let mut o2 = df.build_ontology(vec![positive_ax.clone()]);
    df.auto_declare(&mut o2);
    assert!(
        !o2.axioms().contains(&ax),
        "Ontology with positive assertion must not contain negative assertion"
    );
    assert!(
        !o.axioms().contains(&positive_ax),
        "Ontology with negative assertion must not contain positive assertion"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 12. is_anonymous / is_named Check
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_is_anonymous_check() {
    let df = DF::new();

    let named = df.named("http://ex.org/named");
    assert!(named.is_named(), "Named individual must be named");
    assert!(!named.is_anonymous(), "Named individual must not be anonymous");
    assert!(named.iri().is_some(), "Named individual must have IRI");
    assert!(named.named_iri().is_some());
    assert!(named.anonymous_id().is_none());

    let anon = df.anon();
    assert!(!anon.is_named(), "Anonymous individual must not be named");
    assert!(anon.is_anonymous(), "Anonymous individual must be anonymous");
    assert!(anon.iri().is_none(), "Anonymous individual must not have IRI");
    assert!(anon.named_iri().is_none());
    assert!(anon.anonymous_id().is_some());

    let direct_named = Individual::named(IRI::new("http://ex.org/direct"));
    assert!(direct_named.is_named());
    assert!(!direct_named.is_anonymous());

    let direct_anon = Individual::anonymous("custom_anon".to_string());
    assert!(!direct_anon.is_named());
    assert!(direct_anon.is_anonymous());
    assert_eq!(
        direct_anon.anonymous_id().unwrap().id,
        "custom_anon"
    );

    let fresh = Individual::fresh();
    assert!(!fresh.is_named());
    assert!(fresh.is_anonymous());
    assert!(fresh.anonymous_id().unwrap().id.starts_with("_fresh_"));

    let fresh2 = Individual::fresh();
    assert_ne!(fresh, fresh2, "Fresh individuals must be distinct");
}

// ══════════════════════════════════════════════════════════════════════════════
// 13. ObjectOneOf Class Expression
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_object_one_of_class_expression() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    let c = df.named("http://ex.org/c");

    let one_of_ce = df.one_of(vec![a.clone(), b.clone(), c.clone()]);

    match &one_of_ce {
        ClassExpression::ObjectOneOf(individuals) => {
            assert_eq!(individuals.len(), 3, "ObjectOneOf must contain 3 individuals");
            assert!(individuals.contains(&a), "Must contain a");
            assert!(individuals.contains(&b), "Must contain b");
            assert!(individuals.contains(&c), "Must contain c");
        }
        _ => panic!("Expected ObjectOneOf class expression"),
    }

    let d = df.named("http://ex.org/d");
    let ax_class_assertion = df.class_assertion(one_of_ce.clone(), d.clone());
    match &ax_class_assertion {
        Axiom::ClassAssertion(ca) => {
            assert_eq!(ca.individual, d, "Individual must be d");
            match &ca.class {
                ClassExpression::ObjectOneOf(inds) => {
                    assert_eq!(inds.len(), 3);
                }
                _ => panic!("Expected ObjectOneOf in class position"),
            }
        }
        _ => panic!("Expected ClassAssertion"),
    }

    let dom = df.obj_prop("http://ex.org/domProp");
    let ax_domain = df.object_property_assertion(dom.clone(), a.clone(), b.clone());

    let mut o = df.build_ontology(vec![ax_class_assertion.clone(), ax_domain]);
    df.auto_declare(&mut o);

    assert_contains_axiom!(o, ax_class_assertion);

    let inds = o.get_individuals_in_signature();
    assert!(inds.len() >= 4, "Expected at least 4 named individuals (a,b,c,d)");
}

// ══════════════════════════════════════════════════════════════════════════════
// 14. Individuals in Ontology Signature
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_individuals_in_ontology_signature() {
    let df = DF::new();
    let class_a = df.class_ce("http://ex.org/A");
    let prop_p = df.obj_prop("http://ex.org/P");

    let i1 = df.named("http://ex.org/ind_1");
    let i2 = df.named("http://ex.org/ind_2");
    let i3 = df.named("http://ex.org/ind_3");
    let i4 = df.named("http://ex.org/ind_4");
    let anon1 = df.anon();
    let anon2 = df.anon();

    let mut o = df.build_ontology(vec![
        df.class_assertion(class_a.clone(), i1.clone()),
        df.class_assertion(class_a.clone(), i2.clone()),
        df.object_property_assertion(prop_p.clone(), i3.clone(), i4.clone()),
        df.class_assertion(class_a.clone(), anon1.clone()),
        df.object_property_assertion(prop_p.clone(), i1.clone(), anon2.clone()),
        df.same_individual(vec![i2.clone(), i3.clone()]),
    ]);
    df.auto_declare(&mut o);

    let named_inds = o.get_individuals_in_signature();
    let named_iri_strings: std::collections::HashSet<String> = named_inds
        .iter()
        .map(|ni| ni.iri.as_str().to_string())
        .collect();

    assert!(
        named_iri_strings.contains("http://ex.org/ind_1"),
        "Signature must contain ind_1"
    );
    assert!(
        named_iri_strings.contains("http://ex.org/ind_2"),
        "Signature must contain ind_2"
    );
    assert!(
        named_iri_strings.contains("http://ex.org/ind_3"),
        "Signature must contain ind_3"
    );
    assert!(
        named_iri_strings.contains("http://ex.org/ind_4"),
        "Signature must contain ind_4"
    );
    assert!(
        named_inds.len() >= 4,
        "Expected at least 4 named individuals in signature, got {}",
        named_inds.len()
    );

    let sig = o.signature().unwrap_or_default();
    assert!(
        !sig.individuals.is_empty(),
        "Ontology signature must include individuals"
    );

    let named_in_sig: std::collections::HashSet<_> = sig
        .individuals
        .iter()
        .filter(|ind| ind.is_named())
        .map(|ind| ind.iri().unwrap().as_str().to_string())
        .collect();
    assert!(
        named_in_sig.contains("http://ex.org/ind_1"),
        "Signature must contain ind_1"
    );
    assert!(
        named_in_sig.contains("http://ex.org/ind_2"),
        "Signature must contain ind_2"
    );
    assert!(
        named_in_sig.contains("http://ex.org/ind_3"),
        "Signature must contain ind_3"
    );
    assert!(
        named_in_sig.contains("http://ex.org/ind_4"),
        "Signature must contain ind_4"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 15. Many Different Individuals
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_many_different_individuals() {
    let df = DF::new();

    let individuals: Vec<Individual> = (0..7)
        .map(|i| df.named(&format!("http://ex.org/ind_{i}")))
        .collect();

    assert_eq!(individuals.len(), 7, "Should have 7 individuals");

    let ax = df.different_individuals(individuals.clone());

    match &ax {
        Axiom::DifferentIndividuals(di) => {
            assert_eq!(
                di.individuals.len(),
                7,
                "DifferentIndividuals must contain all 7 individuals"
            );
            for (i, ind) in individuals.iter().enumerate() {
                assert!(
                    di.individuals.contains(ind),
                    "Must contain individual ind_{}",
                    i
                );
            }
        }
        _ => panic!("Expected DifferentIndividuals axiom"),
    }

    let ax3 = df.different_individuals(individuals[0..3].to_vec());
    match &ax3 {
        Axiom::DifferentIndividuals(di) => {
            assert_eq!(di.individuals.len(), 3);
        }
        _ => panic!("Expected DifferentIndividuals with 3"),
    }

    let mut o = df.build_ontology(vec![ax.clone()]);
    df.auto_declare(&mut o);

    assert_contains_axiom!(o, ax);

    let inds = o.get_individuals_in_signature();
    assert_eq!(
        inds.len(),
        7,
        "All 7 individuals must appear in signature"
    );

    let mut test_base = TestBase::new();
    let result = test_base.round_trip_and_compare(&o, OntologyFormat::Functional);
    assert!(
        result.is_ok(),
        "DifferentIndividuals with 7 individuals must survive roundtrip: {:?}",
        result.err()
    );
}

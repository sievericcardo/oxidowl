#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

// ══════════════════════════════════════════════════════════════════════════════
// Named Individual Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn named_individual_creation() {
    let ind = NamedIndividual::new(IRI::new("http://ex.org/i"));
    assert_eq!(ind.iri.as_str(), "http://ex.org/i");
}

#[test]
fn individual_named_variant() {
    let ind = Individual::named(IRI::new("http://ex.org/i"));
    assert!(ind.is_named());
    assert!(!ind.is_anonymous());
    assert_eq!(ind.iri().unwrap().as_str(), "http://ex.org/i");
}

#[test]
fn individual_anonymous_variant() {
    let anon = AnonymousIndividual::new("anon1".to_string());
    let ind = Individual::Anonymous(anon);
    assert!(!ind.is_named());
    assert!(ind.is_anonymous());
}

#[test]
fn anonymous_individual_unique() {
    let anon1 = AnonymousIndividual::new_unique();
    let anon2 = AnonymousIndividual::new_unique();
    assert_ne!(anon1.id, anon2.id);
}

#[test]
fn individual_named_iri_accessor() {
    let ind = Individual::Named(NamedIndividual {
        iri: IRI::new("http://ex.org/i"),
    });
    let named = ind.named_iri().unwrap();
    assert_eq!(named.iri.as_str(), "http://ex.org/i");
    assert!(ind.anonymous_id().is_none());
}

#[test]
fn individual_anonymous_id_accessor() {
    let anon = AnonymousIndividual::new("test-id".to_string());
    let ind = Individual::Anonymous(anon);
    let anon_ref = ind.anonymous_id().unwrap();
    assert_eq!(anon_ref.id, "test-id");
    assert!(ind.named_iri().is_none());
}

// ══════════════════════════════════════════════════════════════════════════════
// SameIndividual / DifferentIndividuals Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn same_individual_axiom() {
    let df = DF::new();
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ax = df.same_individual(vec![i, j]);
    match &ax {
        Axiom::SameIndividual(a) => assert_eq!(a.individuals.len(), 2),
        _ => panic!("Expected SameIndividual"),
    }
}

#[test]
fn different_individuals_axiom() {
    let df = DF::new();
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let k = df.named("http://ex.org/k");
    let ax = df.different_individuals(vec![i, j, k]);
    match &ax {
        Axiom::DifferentIndividuals(a) => assert_eq!(a.individuals.len(), 3),
        _ => panic!("Expected DifferentIndividuals"),
    }
}

#[test]
fn class_assertion_with_named_individual() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    let ax = df.class_assertion(a, i);
    match &ax {
        Axiom::ClassAssertion(ca) => assert!(ca.individual.is_named()),
        _ => panic!("Expected ClassAssertion"),
    }
}

#[test]
fn class_assertion_with_anonymous_individual() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let anon = df.anon();
    let ax = df.class_assertion(a, anon);
    match &ax {
        Axiom::ClassAssertion(ca) => assert!(ca.individual.is_anonymous()),
        _ => panic!("Expected ClassAssertion"),
    }
}

#[test]
fn object_property_assertion_named() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ax = df.object_property_assertion(p, i, j);
    match &ax {
        Axiom::ObjectPropertyAssertion(opa) => {
            assert!(opa.source.is_named());
            assert!(opa.target.is_named());
        }
        _ => panic!("Expected ObjectPropertyAssertion"),
    }
}

#[test]
fn object_property_assertion_with_anonymous() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let anon = df.anon();
    let ax = df.object_property_assertion(p, i, anon);
    match &ax {
        Axiom::ObjectPropertyAssertion(opa) => {
            assert!(opa.target.is_anonymous());
        }
        _ => panic!("Expected ObjectPropertyAssertion"),
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Individual in Ontology Signature
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn ontology_signature_includes_named_individuals() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let mut o = df.build_ontology(vec![
        df.class_assertion(a, i),
        df.class_assertion(df.class_ce("http://ex.org/A"), j),
    ]);
    df.auto_declare(&mut o);
    let inds = o.get_individuals_in_signature();
    assert!(inds.len() >= 2, "Expected at least 2 individuals");
}

#[test]
fn individual_equality_by_iri() {
    let i1 = NamedIndividual::new(IRI::new("http://ex.org/i"));
    let i2 = NamedIndividual::new(IRI::new("http://ex.org/i"));
    assert_eq!(i1, i2);
    let j = NamedIndividual::new(IRI::new("http://ex.org/j"));
    assert_ne!(i1, j);
}

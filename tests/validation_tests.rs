//! Validation Test Suite
//!
//! Verifies that core OWL reasoner modules are exported and behave correctly
//! on a small, hand-built ontology (subsumption and instance reasoning).

use oxidowl::ontology::axioms::{Axiom, ClassAssertionAxiom, SubClassOfAxiom};
use oxidowl::ontology::{Class, ClassExpression, IRI, Individual, NamedIndividual, Ontology};
use oxidowl::{Reasoner, ReasonerConfig};

fn named_class(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn named_individual(iri: &str) -> Individual {
    Individual::Named(NamedIndividual {
        iri: IRI::new(iri),
    })
}

// Test that the core reasoner module is exported and can reason over an empty
// ontology (which is trivially consistent).
#[test]
fn test_modules_accessible() {
    let reasoner =
        Reasoner::new(ReasonerConfig::default()).expect("Failed to construct the Reasoner");

    // A freshly constructed reasoner (no ontology loaded) is consistent.
    assert!(reasoner.is_consistent().expect("Consistency check failed"));
}

#[test]
fn test_basic_ontology_creation() {
    let mut ontology = Ontology::new();
    let test_iri = IRI::new("http://example.org/test");
    ontology.set_iri(test_iri);

    assert!(ontology.axioms.is_empty());
    assert!(ontology.id.ontology_iri.is_some());
}

#[test]
fn test_class_expression_creation() {
    let test_iri = IRI::new("http://example.org/TestClass");
    let class_expr = ClassExpression::class(test_iri);

    assert!(matches!(class_expr, ClassExpression::Class(_)));
}

#[test]
fn test_comprehensive_integration() {
    let a = named_class("http://example.org/A");
    let b = named_class("http://example.org/B");
    let i = named_individual("http://example.org/i");

    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new("http://example.org/onto"));
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 1,
        subclass: a.clone(),
        superclass: b.clone(),
        annotations: vec![],
    }));
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 2,
        class: a.clone(),
        individual: i.clone(),
        annotations: vec![],
    }));

    let mut reasoner =
        Reasoner::new(ReasonerConfig::default()).expect("Failed to construct the Reasoner");
    reasoner
        .load_ontology(ontology)
        .expect("Failed to load ontology");

    assert!(reasoner.is_consistent().expect("Consistency check failed"));
    assert!(reasoner
        .is_subclass_of(&a, &b)
        .expect("Subsumption check failed"));
    assert!(reasoner
        .is_instance_of(&i, &a)
        .expect("Direct instance check failed"));
    assert!(reasoner
        .is_instance_of(&i, &b)
        .expect("Inherited instance check failed"));
}

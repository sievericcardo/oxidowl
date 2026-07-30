#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::reasoner_api::{
    Node, NodeSet, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory,
};
use oxidowl::StructuralReasoner;
use oxidowl::StructuralReasonerFactory;
use oxidowl::TableauReasonerFactory;
use std::sync::Arc;

// Node/NodeSet tests are separated below; these types come from reasoner_api

// ══════════════════════════════════════════════════════════════════════════════
// StructuralReasoner — Class Hierarchy
// ══════════════════════════════════════════════════════════════════════════════

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(std::sync::RwLock::new(o))
}

fn test_ontology_ref() -> OntologyRef {
    let df = DF::new();
    onto_ref(df.simple_chain_ontology())
}

#[test]
fn structural_reasoner_is_consistent() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    assert!(reasoner.is_consistent().unwrap());
}

#[test]
fn structural_reasoner_sub_classes() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });

    // A ⊑ B, so B's subclasses should include A
    let subs = reasoner.get_sub_classes(&b, false).unwrap();
    assert!(!subs.is_empty(), "B should have subclasses (A ⊑ B)");
    let flattened = subs.get_flattened();
    assert!(flattened.contains(&a), "A should be a subclass of B");
}

#[test]
fn structural_reasoner_super_classes() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    let b = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/B") });
    let c = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/C") });

    // A ⊑ B ⊑ C, so B's superclasses should include C
    let sups = reasoner.get_super_classes(&b, false).unwrap();
    assert!(!sups.is_empty(), "B should have superclasses (B ⊑ C)");
    let flattened = sups.get_flattened();
    assert!(flattened.contains(&c), "C should be a superclass of B");
}

#[test]
fn structural_reasoner_instances() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let ind = Individual::Named(NamedIndividual { iri: IRI::new("http://ex.org/ind") });

    let instances = reasoner.get_instances(&a, false).unwrap();
    assert!(instances.contains_entity(&ind));
}

#[test]
fn structural_reasoner_types() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let ind = Individual::Named(NamedIndividual { iri: IRI::new("http://ex.org/ind") });

    let types = reasoner.get_types(&ind, false).unwrap();
    assert!(types.contains_entity(&a));
}

#[test]
fn structural_reasoner_equivalent_classes() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    let node = reasoner.get_equivalent_classes(&a).unwrap();
    assert!(node.is_singleton());
}

#[test]
fn structural_reasoner_satisfiable() {
    let onto = test_ontology_ref();
    let reasoner = StructuralReasoner::new(onto);
    let a = ClassExpression::Class(Class { iri: IRI::new("http://ex.org/A") });
    assert!(reasoner.is_satisfiable(&a).unwrap());
}

// ══════════════════════════════════════════════════════════════════════════════
// StructuralReasonerFactory
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn structural_reasoner_factory_create() {
    let onto = test_ontology_ref();
    let factory = StructuralReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    assert!(reasoner.is_consistent().unwrap());
    assert_eq!(factory.get_reasoner_name(), "Oxidowl Structural Reasoner");
}

// ══════════════════════════════════════════════════════════════════════════════
// TableauReasonerFactory
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn tableau_reasoner_factory_create() {
    let onto = test_ontology_ref();
    let factory = TableauReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    assert!(reasoner.is_consistent().unwrap());
    assert_eq!(factory.get_reasoner_name(), "Oxidowl Tableau Reasoner");
}

// ══════════════════════════════════════════════════════════════════════════════
// Reasoner Configuration Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn reasoner_config_default() {
    let config = OWLReasonerConfiguration::default();
    // Default configuration should exist and be usable
    assert!(config.timeout.is_none());
}

// ══════════════════════════════════════════════════════════════════════════════
// Node and NodeSet Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn node_singleton() {
    let n = Node::singleton(ClassExpression::class(IRI::new("http://ex.org/A")));
    assert!(n.is_singleton());
    assert_eq!(n.get_size(), 1);
}

#[test]
fn node_contains() {
    let ce = ClassExpression::class(IRI::new("http://ex.org/A"));
    let n = Node::singleton(ce.clone());
    assert!(n.contains(&ce));
}

#[test]
fn node_set_basic() {
    let n1 = Node::singleton(ClassExpression::class(IRI::new("http://ex.org/A")));
    let n2 = Node::singleton(ClassExpression::class(IRI::new("http://ex.org/B")));
    let mut nodes = std::collections::HashSet::new();
    nodes.insert(n1);
    nodes.insert(n2);
    let ns = NodeSet::new(nodes);
    assert!(!ns.is_empty());
}

#[test]
fn node_set_flattened() {
    let a = ClassExpression::class(IRI::new("http://ex.org/A"));
    let b = ClassExpression::class(IRI::new("http://ex.org/B"));
    let n1 = Node::singleton(a.clone());
    let n2 = Node::singleton(b.clone());
    let mut nodes = std::collections::HashSet::new();
    nodes.insert(n1);
    nodes.insert(n2);
    let ns = NodeSet::new(nodes);
    let flat = ns.get_flattened();
    assert_eq!(flat.len(), 2);
    assert!(flat.contains(&a));
    assert!(flat.contains(&b));
}

#[test]
fn node_set_contains_entity() {
    let ce = ClassExpression::class(IRI::new("http://ex.org/A"));
    let n = Node::singleton(ce.clone());
    let mut nodes = std::collections::HashSet::new();
    nodes.insert(n);
    let ns = NodeSet::new(nodes);
    assert!(ns.contains_entity(&ce));
}

#[test]
fn node_set_empty() {
    let ns: NodeSet<ClassExpression> = NodeSet::empty();
    assert!(ns.is_empty());
    assert!(ns.get_flattened().is_empty());
}

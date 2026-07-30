#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::{
    Node, NodeSet, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory,
    StructuralReasoner, StructuralReasonerFactory,
};
use std::sync::Arc;

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(std::sync::RwLock::new(o))
}

#[test]
fn test_structural_reasoner_consistent() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.sub_class_of(a, b)]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    assert!(reasoner.is_consistent().unwrap(), "Ontology should be consistent");
}

#[test]
fn test_structural_reasoner_inconsistent() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let c = df.class_ce("http://ex.org/C");
    let not_a = df.complement_of(a.clone());
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(c.clone(), a.clone()),
        df.sub_class_of(c, not_a),
    ]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    // Structural reasoner always returns true for consistency — it does not
    // perform logical consistency checking (no tableau).
    assert!(reasoner.is_consistent().unwrap(), "Structural reasoner always reports consistent");
}

#[test]
fn test_get_sub_classes_direct() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    // B ⊑ A, C ⊑ A — so B and C are direct subclasses of A
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(b.clone(), a.clone()),
        df.sub_class_of(c.clone(), a.clone()),
    ]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let subs = reasoner.get_sub_classes(&a, true).unwrap();
    assert!(!subs.is_empty(), "Should have direct subclasses");
    let flat = subs.get_flattened();
    assert!(flat.contains(&b), "B should be a direct subclass of A");
    assert!(flat.contains(&c), "C should be a direct subclass of A");
}

#[test]
fn test_get_sub_classes_all() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    // C ⊑ B ⊑ A → transitive subclasses of A are B and C
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(b.clone(), a.clone()),
        df.sub_class_of(c.clone(), b.clone()),
    ]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);

    let subs_all = reasoner.get_sub_classes(&a, false).unwrap();
    assert!(!subs_all.is_empty(), "Should have all subclasses");
    let flat = subs_all.get_flattened();
    assert!(flat.contains(&b), "B should be a subclass of A (all)");
    assert!(flat.contains(&c), "C should be a subclass of A (transitive)");
}

#[test]
fn test_get_super_classes_direct() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    // B ⊑ A, B ⊑ C — so B's direct superclasses are A and C
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(b.clone(), a.clone()),
        df.sub_class_of(b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let sups = reasoner.get_super_classes(&b, true).unwrap();
    assert!(!sups.is_empty(), "Should have direct superclasses");
    let flat = sups.get_flattened();
    assert!(flat.contains(&a), "A should be a direct superclass of B");
    assert!(flat.contains(&c), "C should be a direct superclass of B");
}

#[test]
fn test_get_super_classes_all() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(b.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let sups = reasoner.get_super_classes(&a, false).unwrap();
    assert!(!sups.is_empty(), "Should have all superclasses");
    let flat = sups.get_flattened();
    assert!(flat.contains(&b), "B should be a superclass of A");
    assert!(flat.contains(&c), "C should be a superclass of A (transitive)");
}

#[test]
fn test_get_equivalent_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.equivalent_classes(vec![a.clone(), b.clone()])]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let node = reasoner.get_equivalent_classes(&a).unwrap();
    assert!(!node.is_singleton(), "A should have equivalent classes");
    assert!(node.contains(&b), "Node should contain B");
    assert!(node.contains(&a), "Node should contain A");
}

#[test]
fn test_get_disjoint_classes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.disjoint_classes(vec![a.clone(), b.clone()])]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let disj = reasoner.get_disjoint_classes(&a).unwrap();
    assert!(!disj.is_empty(), "Should have disjoint classes for A");
    assert!(disj.contains_entity(&b), "B should be disjoint with A");
}

#[test]
fn test_get_instances() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    let mut ont = df.build_ontology(vec![df.class_assertion(a.clone(), i.clone())]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let instances = reasoner.get_instances(&a, false).unwrap();
    assert!(!instances.is_empty(), "A should have instances");
    assert!(instances.contains_entity(&i), "i should be an instance of A");
}

#[test]
fn test_get_types() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let i = df.named("http://ex.org/i");
    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), i.clone()),
        df.sub_class_of(a.clone(), b.clone()),
    ]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    let types = reasoner.get_types(&i, false).unwrap();
    assert!(!types.is_empty(), "i should have types");
    let flat = types.get_flattened();
    assert!(flat.contains(&a), "i should be of type A (direct assertion)");
    // Structural reasoner only returns directly asserted types (no subclass inference)
}

#[test]
fn test_top_bottom_class_nodes() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.sub_class_of(a, b)]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);

    let top = reasoner.get_top_class_node().unwrap();
    assert!(top.is_top_node(), "Should have top class node");
    assert!(!top.is_bottom_node(), "Top node should not be bottom");

    let bottom = reasoner.get_bottom_class_node().unwrap();
    assert!(bottom.is_bottom_node(), "Should have bottom class node");
    assert!(!bottom.is_top_node(), "Bottom node should not be top");
}

#[test]
fn test_is_satisfiable() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.sub_class_of(a.clone(), b)]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let reasoner = StructuralReasoner::new(ont_ref);
    assert!(reasoner.is_satisfiable(&a).unwrap(), "A should be satisfiable");
}

#[test]
fn test_node_set_operations() {
    let ce = ClassExpression::class(IRI::new("http://ex.org/A"));
    let node = Node::singleton(ce.clone());
    assert!(node.is_singleton());
    assert!(node.contains(&ce));

    let empty_set: NodeSet<ClassExpression> = NodeSet::empty();
    assert!(empty_set.is_empty());
    assert!(empty_set.get_flattened().is_empty());

    let ns: NodeSet<ClassExpression> = std::iter::once(node).collect();
    assert!(ns.contains_entity(&ce));
    assert!(!ns.get_flattened().is_empty());
}

#[test]
fn test_reasoner_factory() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.sub_class_of(a, b)]);
    df.auto_declare(&mut ont);
    let ont_ref = onto_ref(ont);

    let config = OWLReasonerConfiguration::default();
    let factory = StructuralReasonerFactory;
    let reasoner = factory.create_reasoner(&ont_ref, &config).unwrap();

    assert_eq!(factory.get_reasoner_name(), "Oxidowl Structural Reasoner");
    assert!(reasoner.is_consistent().unwrap(), "Factory-created reasoner should be consistent");
    let (major, minor, patch) = reasoner.get_reasoner_version();
    assert_eq!(major, 1);
    assert_eq!(minor, 0);
    assert_eq!(patch, 0);
}

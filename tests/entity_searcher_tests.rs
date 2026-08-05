#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::inference::metrics::OntologyMetrics;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::searcher::{EntityIndex, EntitySearcher};

/// Build a test ontology: A ⊑ B, B ⊑ C, ClassAssertion(A, i), P(i, j)
fn test_ontology() -> Ontology {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let mut o = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(b.clone(), c.clone()),
        df.class_assertion(a.clone(), i.clone()),
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
    ]);
    df.auto_declare(&mut o);
    o
}

// ── get_sub_class_axioms_for_lhs ────────────────────────────────────────────

#[test]
fn searcher_sub_class_axioms_for_lhs() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let axioms = searcher.get_sub_class_axioms_for_lhs(&a);
    assert!(!axioms.is_empty());
}

// ── get_sub_class_axioms_for_rhs ────────────────────────────────────────────

#[test]
fn searcher_sub_class_axioms_for_rhs() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let b = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/B"),
    });
    let axioms = searcher.get_sub_class_axioms_for_rhs(&b);
    assert!(!axioms.is_empty());
}

// ── get_class_assertion_axioms ──────────────────────────────────────────────

#[test]
fn searcher_class_assertion_axioms() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let i = Individual::Named(NamedIndividual {
        iri: IRI::new("http://ex.org/i"),
    });
    let axioms = searcher.get_class_assertion_axioms(&i);
    assert!(!axioms.is_empty());
}

// ── get_object_property_assertion_axioms ────────────────────────────────────

#[test]
fn searcher_object_property_assertion_axioms() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let i = Individual::Named(NamedIndividual {
        iri: IRI::new("http://ex.org/i"),
    });
    let axioms = searcher.get_object_property_assertion_axioms(&i);
    assert!(!axioms.is_empty());
}

// ── get_equivalent_classes_axioms ───────────────────────────────────────────

#[test]
fn searcher_equivalent_classes_empty_for_basic_onto() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let axioms = searcher.get_equivalent_classes_axioms(&a);
    // No EquivalentClasses in a subclass-only ontology
    assert!(axioms.is_empty());
}

// ── get_disjoint_classes_axioms ─────────────────────────────────────────────

#[test]
fn searcher_disjoint_classes_empty_for_basic_onto() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let axioms = searcher.get_disjoint_classes_axioms(&a);
    assert!(axioms.is_empty());
}

// ── EntityIndex — ids_for_entity ───────────────────────────────────────────

#[test]
fn entity_index_ids_for_entity() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let a_iri = IRI::new("http://ex.org/A");
    let ids = index.ids_for_entity(&a_iri);
    assert!(!ids.is_empty(), "Class A should have axiom IDs in index");
}

#[test]
fn entity_index_nonexistent_iri() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let ids = index.ids_for_entity(&IRI::new("http://ex.org/NonExistent"));
    assert!(ids.is_empty());
}

// ── EntityIndex — get_axiom ────────────────────────────────────────────────

#[test]
fn entity_index_get_axiom() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let a_iri = IRI::new("http://ex.org/A");
    let ids = index.ids_for_entity(&a_iri);
    for id in &ids {
        let axiom = index.get_axiom(*id);
        assert!(axiom.is_some(), "Axiom with id {id} should exist");
    }
}

// ── get_declaration_axioms_by_type ─────────────────────────────────────────

#[test]
fn searcher_declaration_axioms_by_class_type() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let axioms = searcher.get_declaration_axioms_by_type(&EntityType::Class);
    assert!(axioms.len() >= 3, "Expected at least 3 class declarations");
}

#[test]
fn searcher_declaration_axioms_by_obj_prop_type() {
    let o = test_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);
    let axioms = searcher.get_declaration_axioms_by_type(&EntityType::ObjectProperty);
    assert!(
        axioms.len() >= 1,
        "Expected at least 1 object property declaration"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Ontology Metrics Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn ontology_metrics_counts() {
    let o = test_ontology();
    let metrics = OntologyMetrics::compute(&o);
    assert!(metrics.contains_key("NumberOfAxioms"));
    assert!(metrics.contains_key("NumberOfClasses"));
    // Should have at least 3 classes (A, B, C) + declarations
    assert!(*metrics.get("NumberOfClasses").unwrap_or(&0.0) >= 3.0);
}

// ══════════════════════════════════════════════════════════════════════════════
// Entity Collector Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn entity_collector_classes() {
    let o = test_ontology();
    let classes = o.get_classes_in_signature();
    assert!(
        classes.len() >= 3,
        "Expected at least 3 classes, got {}",
        classes.len()
    );
}

#[test]
fn entity_collector_object_properties() {
    let o = test_ontology();
    let props = o.get_object_properties_in_signature();
    assert!(props.len() >= 1, "Expected at least 1 object property");
}

#[test]
fn entity_collector_individuals() {
    let o = test_ontology();
    let inds = o.get_individuals_in_signature();
    assert!(inds.len() >= 2, "Expected at least 2 individuals");
}

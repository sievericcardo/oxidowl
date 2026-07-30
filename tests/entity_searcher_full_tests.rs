#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::searcher::{EntityIndex, EntitySearcher};

#[test]
fn test_get_sub_class_axioms_for_lhs() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let sc1 = df.sub_class_of(a.clone(), b.clone());
    let sc2 = df.sub_class_of(a.clone(), c.clone());
    let mut ont = df.build_ontology(vec![sc1, sc2]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_sub_class_axioms_for_lhs(&a);
    assert!(!results.is_empty(), "Should find sub-class axioms where A is LHS");
    assert!(results.len() >= 2, "Should find at least 2 axioms");
}

#[test]
fn test_get_sub_class_axioms_for_rhs() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let sc1 = df.sub_class_of(a.clone(), b.clone());
    let sc2 = df.sub_class_of(c.clone(), b.clone());
    let mut ont = df.build_ontology(vec![sc1, sc2]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_sub_class_axioms_for_rhs(&b);
    assert!(!results.is_empty(), "Should find sub-class axioms where B is RHS");
    assert!(results.len() >= 2, "Should find at least 2 axioms");
}

#[test]
fn test_get_equivalent_classes_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let eq = df.equivalent_classes(vec![a.clone(), b.clone()]);
    let mut ont = df.build_ontology(vec![eq]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_equivalent_classes_axioms(&a);
    assert!(!results.is_empty(), "Should find equivalent class axioms for A");
}

#[test]
fn test_get_disjoint_classes_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let disj = df.disjoint_classes(vec![a.clone(), b.clone()]);
    let mut ont = df.build_ontology(vec![disj]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_disjoint_classes_axioms(&a);
    assert!(!results.is_empty(), "Should find disjoint class axioms for A");
}

#[test]
fn test_get_disjoint_union_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let du = df.disjoint_union(a.clone(), vec![b.clone(), c.clone()]);
    let mut ont = df.build_ontology(vec![du]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_disjoint_union_axioms(&a);
    assert!(!results.is_empty(), "Should find disjoint union axioms for A");
}

#[test]
fn test_get_has_key_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let p = df.obj_prop("http://ex.org/P");
    let hk = df.has_key(a.clone(), vec![p], vec![]);
    let mut ont = df.build_ontology(vec![hk]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_has_key_axioms(&a);
    assert!(!results.is_empty(), "Should find HasKey axioms for A");
}

#[test]
fn test_get_object_property_domain_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let a = df.class_ce("http://ex.org/A");
    let ax = df.object_property_domain(p.clone(), a.clone());
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_object_property_domain_axioms(&p);
    assert!(!results.is_empty(), "Should find object property domain axioms for P");
}

#[test]
fn test_get_object_property_range_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let a = df.class_ce("http://ex.org/A");
    let ax = df.object_property_range(p.clone(), a.clone());
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_object_property_range_axioms(&p);
    assert!(!results.is_empty(), "Should find object property range axioms for P");
}

#[test]
fn test_get_sub_object_property_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.sub_object_property_of(p.clone(), q.clone());
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_sub_object_property_axioms(&p);
    assert!(!results.is_empty(), "Should find sub-object-property axioms for P");
}

#[test]
fn test_get_equivalent_object_properties_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.equivalent_object_properties(vec![p.clone(), q.clone()]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_equivalent_object_properties_axioms(&p);
    assert!(!results.is_empty(), "Should find equivalent object property axioms for P");
}

#[test]
fn test_get_disjoint_object_properties_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.disjoint_object_properties(vec![p.clone(), q.clone()]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_disjoint_object_properties_axioms(&p);
    assert!(!results.is_empty(), "Should find disjoint object property axioms for P");
}

#[test]
fn test_get_inverse_object_properties_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let q = df.obj_prop("http://ex.org/Q");
    let ax = df.inverse_object_properties(p.clone(), q.clone());
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_inverse_object_properties_axioms(&p);
    assert!(!results.is_empty(), "Should find inverse object property axioms for P");
}

#[test]
fn test_get_class_assertion_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/i");
    let ax = df.class_assertion(a, i.clone());
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_class_assertion_axioms(&i);
    assert!(!results.is_empty(), "Should find class assertion axioms for i");
}

#[test]
fn test_get_object_property_assertion_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ax = df.object_property_assertion(p, i.clone(), j);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_object_property_assertion_axioms(&i);
    assert!(!results.is_empty(), "Should find object property assertion axioms for i");
}

#[test]
fn test_get_data_property_assertion_axioms() {
    let df = DF::new();
    let p = df.data_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let lit = df.literal("hello");
    let ax = df.data_property_assertion(p, i.clone(), lit);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_data_property_assertion_axioms(&i);
    assert!(!results.is_empty(), "Should find data property assertion axioms for i");
}

#[test]
fn test_get_negative_object_property_assertion_axioms() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ax = df.negative_object_property_assertion(p, i.clone(), j);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_negative_object_property_assertion_axioms(&i);
    assert!(!results.is_empty(), "Should find negative object property assertion axioms for i");
}

#[test]
fn test_get_negative_data_property_assertion_axioms() {
    let df = DF::new();
    let p = df.data_prop("http://ex.org/P");
    let i = df.named("http://ex.org/i");
    let lit = df.literal("hello");
    let ax = df.negative_data_property_assertion(p, i.clone(), lit);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_negative_data_property_assertion_axioms(&i);
    assert!(!results.is_empty(), "Should find negative data property assertion axioms for i");
}

#[test]
fn test_get_different_individual_axioms() {
    let df = DF::new();
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ax = df.different_individuals(vec![i.clone(), j.clone()]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_different_individual_axioms(&i);
    assert!(!results.is_empty(), "Should find different-individual axioms for i");
}

#[test]
fn test_get_same_individual_axioms() {
    let df = DF::new();
    let i = df.named("http://ex.org/i");
    let j = df.named("http://ex.org/j");
    let ax = df.same_individual(vec![i.clone(), j.clone()]);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_same_individual_axioms(&i);
    assert!(!results.is_empty(), "Should find same-individual axioms for i");
}

#[test]
fn test_get_annotation_assertion_axioms() {
    let df = DF::new();
    let subject_iri = IRI::new("http://ex.org/subject");
    let prop = df.annotation_property("http://ex.org/rdfsLabel");
    let ax = df.annotation_assertion(prop, subject_iri.clone(), "test label");
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_annotation_assertion_axioms(&subject_iri);
    assert!(!results.is_empty(), "Should find annotation assertion axioms for the subject");
}

#[test]
fn test_get_declaration_axioms() {
    let df = DF::new();
    let mut ont = df.build_ontology(vec![]);
    df.auto_declare(&mut ont);
    let index = EntityIndex::from_ontology(&ont);

    let entity = Entity::Class(IRI::new("http://ex.org/X"));
    let decl = df.declaration_axiom(entity.clone());
    ont.add_axiom(decl);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_declaration_axioms(&entity);
    assert!(!results.is_empty(), "Should find declaration axioms for entity X");
}

#[test]
fn test_get_declaration_axioms_by_type() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let mut ont = df.build_ontology(vec![df.sub_class_of(a, b)]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_declaration_axioms_by_type(&EntityType::Class);
    assert!(!results.is_empty(), "Should find class declaration axioms");
    assert!(results.len() >= 2, "Should have at least 2 class declarations");
}

#[test]
fn test_get_datatype_definition_axioms() {
    let df = DF::new();
    let dt_iri = IRI::new("http://ex.org/MyDatatype");
    let ax = df
        .df
        .get_declaration_axiom(Entity::Datatype(dt_iri.clone()));
    let mut ont = df.build_ontology(vec![Axiom::Declaration(ax)]);
    df.auto_declare(&mut ont);

    let index = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &index);

    let results = searcher.get_datatype_definition_axioms(&dt_iri);
    assert!(results.is_empty(), "No datatype definition axioms exist, only declarations");
}

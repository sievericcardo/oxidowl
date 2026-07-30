#[cfg(test)]
mod tests {
    use crate::{
        DataFactory, OntologyChange, ChangeHistory, OntologyManager,
        OntologyLoader,
    };
    use crate::factory::providers::AxiomCreationProvider;
    use crate::manager::iri_mapper::{
        SimpleIRIMapper, NonMappingOntologyIRIMapper, CompositeIRIMapper, OntologyIRIMapper,
    };
    use crate::manager::listeners::LoggingChangeListener;
    use crate::ontology::{IRI, OntologyFormat, ClassExpression};
    use crate::ontology::axioms::{Axiom, EntityType, SubClassOfAxiom};
    use std::sync::{Arc, RwLock};

    fn make_test_axiom(factory: &DataFactory) -> SubClassOfAxiom {
        let a = factory.get_class(&IRI::new("http://ex.org/A"));
        let b = factory.get_class(&IRI::new("http://ex.org/B"));
        factory.make_sub_class_of_axiom(
            ClassExpression::Class(a),
            ClassExpression::Class(b),
            vec![],
        )
    }

    fn make_add_change(iri: &IRI, ax: SubClassOfAxiom) -> OntologyChange {
        OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom: Axiom::SubClassOf(ax),
        }
    }

    // ── IRI Mapper Tests ────────────────────────────────────────────────────

    #[test]
    fn test_simple_iri_mapper() {
        let onto_iri = IRI::new("http://example.org/ont");
        let doc_iri = IRI::new("file:///tmp/ont.owl");
        let mapper = SimpleIRIMapper::new(onto_iri.clone(), doc_iri.clone());
        assert_eq!(mapper.get_document_iri(&onto_iri), Some(doc_iri));
        assert_eq!(mapper.get_document_iri(&IRI::new("http://other.org/other")), None);
        assert_eq!(mapper.name(), "Simple(http://example.org/ont)");
    }

    #[test]
    fn test_non_mapping_iri_mapper() {
        let mapper = NonMappingOntologyIRIMapper;
        assert_eq!(mapper.get_document_iri(&IRI::new("anything")), None);
    }

    #[test]
    fn test_composite_iri_mapper() {
        let m1 = SimpleIRIMapper::new(IRI::new("http://a.org/ont"), IRI::new("file:///a.owl"));
        let m2 = SimpleIRIMapper::new(IRI::new("http://b.org/ont"), IRI::new("file:///b.owl"));
        let composite = CompositeIRIMapper::new(vec![Box::new(m1), Box::new(m2)]);
        assert_eq!(composite.get_document_iri(&IRI::new("http://a.org/ont")), Some(IRI::new("file:///a.owl")));
        assert_eq!(composite.get_document_iri(&IRI::new("http://b.org/ont")), Some(IRI::new("file:///b.owl")));
        assert_eq!(composite.get_document_iri(&IRI::new("http://c.org/ont")), None);
    }

    // ── DataFactory Tests ────────────────────────────────────────────────────

    #[test]
    fn test_factory_entity_interning() {
        let factory = DataFactory::new();
        let iri = IRI::new("http://example.org/A");
        let c1 = factory.get_class(&iri);
        let c2 = factory.get_class(&iri);
        assert_eq!(c1.iri, c2.iri);
        let p1 = factory.get_object_property(&iri);
        let p2 = factory.get_object_property(&iri);
        assert_eq!(p1.iri, p2.iri);
        let i1 = factory.get_named_individual(&iri);
        let i2 = factory.get_named_individual(&iri);
        assert_eq!(i1.iri, i2.iri);
    }

    #[test]
    fn test_factory_literal_creation() {
        let factory = DataFactory::new();
        let s = factory.get_string_literal("hello");
        assert_eq!(s.value, "hello");
        let b = factory.get_boolean_literal(true);
        assert_eq!(b.value, "true");
        let i = factory.get_integer_literal(42);
        assert_eq!(i.value, "42");
        let t = factory.get_typed_literal("42", &IRI::new("http://www.w3.org/2001/XMLSchema#integer"));
        assert!(t.datatype.is_some());
        let l = factory.get_lang_literal("hello", "en");
        assert_eq!(l.language.as_deref(), Some("en"));
    }

    #[test]
    fn test_factory_punning() {
        let factory = DataFactory::new();
        let iri = IRI::new("http://example.org/X");
        let as_class = factory.get_entity(&iri, &EntityType::Class);
        let as_prop = factory.get_entity(&iri, &EntityType::ObjectProperty);
        assert!(matches!(as_class, crate::ontology::axioms::Entity::Class(ref i) if i == &iri));
        assert!(matches!(as_prop, crate::ontology::axioms::Entity::ObjectProperty(ref i) if i == &iri));
    }

    #[test]
    fn test_factory_class_expressions() {
        let factory = DataFactory::new();
        let c1 = factory.get_class(&IRI::new("http://example.org/C1"));
        let c2 = factory.get_class(&IRI::new("http://example.org/C2"));
        let ce1 = ClassExpression::Class(c1);
        let ce2 = ClassExpression::Class(c2);
        let intersection = factory.get_object_intersection_of(vec![ce1.clone(), ce2.clone()]);
        assert!(matches!(intersection, ClassExpression::ObjectIntersectionOf(_)));
        let union = factory.get_object_union_of(vec![ce1.clone(), ce2.clone()]);
        assert!(matches!(union, ClassExpression::ObjectUnionOf(_)));
        let comp = factory.get_object_complement_of(ce1.clone());
        assert!(matches!(comp, ClassExpression::ObjectComplementOf(_)));
    }

    #[test]
    fn test_factory_axiom_id_uniqueness() {
        let factory = DataFactory::new();
        let ce1 = ClassExpression::Class(factory.get_class(&IRI::new("http://example.org/A")));
        let ce2 = ClassExpression::Class(factory.get_class(&IRI::new("http://example.org/B")));
        let ax1 = factory.make_sub_class_of_axiom(ce1.clone(), ce2.clone(), vec![]);
        let ax2 = factory.make_sub_class_of_axiom(ce2.clone(), ce1.clone(), vec![]);
        assert_ne!(ax1.id, ax2.id);
    }

    // ── OntologyManager Tests ────────────────────────────────────────────────

    #[test]
    fn test_manager_create_ontology() {
        let mut manager = OntologyManager::new();
        let iri = IRI::new("http://example.org/test");
        let _ont = manager.create_ontology(iri.clone());
        assert!(manager.contains_ontology(&iri));
        assert_eq!(manager.ontology_count(), 1);
        assert!(manager.get_ontology(&iri).is_some());
    }

    #[test]
    fn test_manager_create_with_axioms() {
        let mut manager = OntologyManager::new();
        let iri = IRI::new("http://example.org/test");
        let ax = make_test_axiom(manager.get_data_factory());
        let ont = manager.create_ontology_with_axioms(iri.clone(), vec![Axiom::SubClassOf(ax)]);
        assert_eq!(ont.read().unwrap().axioms().len(), 1);
    }

    #[test]
    fn test_manager_remove_ontology() {
        let mut manager = OntologyManager::new();
        let iri = IRI::new("http://example.org/test");
        let ont = manager.create_ontology(iri.clone());
        assert!(manager.contains_ontology(&iri));
        manager.remove_ontology(&ont).unwrap();
        assert!(!manager.contains_ontology(&iri));
    }

    #[test]
    fn test_manager_imports_closure() {
        let mut manager = OntologyManager::new();
        let iri1 = IRI::new("http://example.org/ont1");
        let iri2 = IRI::new("http://example.org/ont2");
        let iri3 = IRI::new("http://example.org/ont3");
        let o1 = manager.create_ontology(iri1.clone());
        manager.create_ontology(iri2.clone());
        manager.create_ontology(iri3.clone());
        manager.add_import(iri1.clone(), iri2.clone());
        manager.add_import(iri2.clone(), iri3.clone());
        let closure = manager.get_imports_closure(&o1).unwrap();
        assert!(closure.iter().any(|r| r.read().unwrap().get_iri().cloned() == Some(iri1.clone())));
        assert!(closure.iter().any(|r| r.read().unwrap().get_iri().cloned() == Some(iri2.clone())));
        assert!(closure.iter().any(|r| r.read().unwrap().get_iri().cloned() == Some(iri3.clone())));
    }

    // ── OntologyChange Tests ─────────────────────────────────────────────────

    #[test]
    fn test_change_is_axiom_change() {
        let iri = IRI::new("http://example.org/ont");
        let factory = DataFactory::new();
        let ax = make_test_axiom(&factory);
        let add = make_add_change(&iri, ax.clone());
        let rem = OntologyChange::RemoveAxiom { ontology_iri: iri.clone(), axiom: Axiom::SubClassOf(ax) };
        assert!(add.is_axiom_change());
        assert!(rem.is_axiom_change());
        assert!(add.is_add_change());
        assert!(!add.is_remove_change());
        assert!(rem.is_remove_change());
        assert!(!rem.is_add_change());
    }

    #[test]
    fn test_change_inverse() {
        let iri = IRI::new("http://example.org/ont");
        let factory = DataFactory::new();
        let ax = make_test_axiom(&factory);
        let add = make_add_change(&iri, ax);
        let inv = add.inverse();
        assert!(inv.is_remove_change());
        assert_eq!(inv.affected_axiom_id(), add.affected_axiom_id());
    }

    // ── ChangeHistory Tests ──────────────────────────────────────────────────

    #[test]
    fn test_change_history_basic() {
        let mut history = ChangeHistory::new(10);
        assert!(!history.can_undo());
        assert!(!history.can_redo());
        let factory = DataFactory::new();
        let ax = make_test_axiom(&factory);
        history.record(vec![make_add_change(&IRI::new("http://ex.org/ont"), ax)]);
        assert!(history.can_undo());
        assert_eq!(history.undo_count(), 1);
        assert_eq!(history.total_batches(), 1);
    }

    #[test]
    fn test_change_history_undo_redo() {
        let mut history = ChangeHistory::new(10);
        let factory = DataFactory::new();
        let ax = make_test_axiom(&factory);
        let change = make_add_change(&IRI::new("http://ex.org/ont"), ax);
        history.record(vec![change.clone()]);
        let undone = history.undo(1);
        assert_eq!(undone.len(), 1);
        assert!(undone[0].is_remove_change());
        assert!(history.can_redo());
        let redone = history.redo(1);
        assert_eq!(redone.len(), 1);
    }

    #[test]
    fn test_change_history_prune() {
        let mut history = ChangeHistory::new(3);
        let factory = DataFactory::new();
        let ax = make_test_axiom(&factory);
        for _ in 0..5 {
            history.record(vec![make_add_change(&IRI::new("http://ex.org/ont"), ax.clone())]);
        }
        assert_eq!(history.total_batches(), 3);
    }

    // ── Manager Change Application Tests ─────────────────────────────────────

    #[test]
    fn test_manager_apply_axiom_change() {
        let mut manager = OntologyManager::new();
        manager.add_change_listener(Box::new(LoggingChangeListener::debug()));
        let iri = IRI::new("http://example.org/ont");
        let ont = manager.create_ontology(iri.clone());
        let ax = make_test_axiom(manager.get_data_factory());
        let change = make_add_change(&iri, ax);
        manager.apply_change(change).unwrap();
        assert_eq!(ont.read().unwrap().axioms().len(), 1);
    }

    #[test]
    fn test_manager_onto_id_change() {
        let mut manager = OntologyManager::new();
        let old_iri = IRI::new("http://example.org/old");
        let new_iri = IRI::new("http://example.org/new");
        let _ont = manager.create_ontology(old_iri.clone());
        let change = OntologyChange::SetOntologyId {
            ontology_iri: old_iri.clone(),
            new_iri: new_iri.clone(),
            new_version_iri: None,
        };
        manager.apply_change(change).unwrap();
        assert!(!manager.contains_ontology(&old_iri));
        assert!(manager.contains_ontology(&new_iri));
    }

    // ── Loader Tests ─────────────────────────────────────────────────────────

    #[test]
    fn test_loader_load_from_string() {
        let manager = Arc::new(RwLock::new(OntologyManager::new()));
        let loader = OntologyLoader::new(manager.clone());
        let content = r#"Ontology(<http://example.org/test>
            ClassAssertion(<http://example.org/A> <http://example.org/ind>)
        )"#;
        let result = loader.load_from_string(content, OntologyFormat::Functional, "test");
        assert!(result.is_ok());
        let ont_ref = result.unwrap();
        assert_eq!(ont_ref.read().unwrap().axioms().len(), 1);
    }
}

#[cfg(test)]
mod tests {
    use crate::ontology::axioms::{
        Axiom, ClassAssertionAxiom, EquivalentClassesAxiom, SameIndividualAxiom, SubClassOfAxiom,
    };
    use crate::ontology::individuals::NamedIndividual;
    use crate::ontology::{
        Class, ClassExpression, IRI, Individual, ObjectProperty, ObjectPropertyExpression,
        Ontology, OntologyRef,
    };
    use crate::{
        BufferingMode, InferenceType, Node, NodeSet, OWLReasoner, OWLReasonerConfiguration,
        ReasonerFactory, StructuralReasoner, StructuralReasonerFactory, TableauReasonerFactory,
    };
    use std::sync::{Arc, RwLock};

    fn make_simple_ontology() -> Ontology {
        let mut o = Ontology::new();
        let a = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        });
        let b = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        });
        o.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: a,
            superclass: b,
            annotations: vec![],
        }));
        o
    }

    fn make_onto_ref(o: Ontology) -> OntologyRef {
        Arc::new(RwLock::new(o))
    }

    // ── Node / NodeSet Tests ─────────────────────────────────────────────────

    #[test]
    fn test_node_singleton() {
        let n = Node::singleton("A");
        assert!(n.is_singleton());
        assert_eq!(n.get_size(), 1);
        assert!(n.contains(&"A"));
    }

    #[test]
    fn test_node_multiple() {
        let mut set = std::collections::HashSet::new();
        set.insert(1);
        set.insert(2);
        set.insert(3);
        let n = Node::new(set);
        assert!(!n.is_singleton());
        assert_eq!(n.get_size(), 3);
        assert!(n.contains(&1));
    }

    #[test]
    fn test_node_top_bottom() {
        let top = Node::<i32>::top_node(0);
        assert!(top.is_top_node());
        assert!(!top.is_bottom_node());

        let bot = Node::<i32>::bottom_node(0);
        assert!(bot.is_bottom_node());
    }

    #[test]
    fn test_node_set_empty() {
        let ns: NodeSet<i32> = NodeSet::empty();
        assert!(ns.is_empty());
    }

    #[test]
    fn test_node_set_flattened() {
        let n1 = Node::singleton(1);
        let n2 = Node::singleton(2);
        let ns = NodeSet::new([n1, n2].into_iter().collect());
        let flat = ns.get_flattened();
        assert_eq!(flat.len(), 2);
        assert!(flat.contains(&1));
        assert!(flat.contains(&2));
    }

    #[test]
    fn test_node_set_contains_entity() {
        let n = Node::singleton("X");
        let ns = NodeSet::new([n].into_iter().collect());
        assert!(ns.contains_entity(&"X"));
        assert!(!ns.contains_entity(&"Y"));
    }

    // ── StructuralReasoner Tests ─────────────────────────────────────────────

    #[test]
    fn test_structural_is_consistent() {
        let o = make_simple_ontology();
        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        assert!(reasoner.is_consistent().unwrap());
    }

    #[test]
    fn test_structural_sub_classes() {
        let o = make_simple_ontology();
        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        // B is superclass of A, so get_sub_classes(B) should return A
        let b = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        });
        let subs = reasoner.get_sub_classes(&b, false).unwrap();
        assert!(!subs.is_empty());
        let flat = subs.get_flattened();
        assert!(flat.iter().any(|c| {
            match c {
                ClassExpression::Class(cls) => cls.iri.as_str().contains("A"),
                _ => false,
            }
        }));
    }

    #[test]
    fn test_structural_super_classes() {
        let o = make_simple_ontology();
        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        // A is subclass of B, so get_super_classes(A) should return B
        let a = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        });
        let sups = reasoner.get_super_classes(&a, false).unwrap();
        assert!(!sups.is_empty());
        assert!(sups.get_flattened().iter().any(|c| {
            match c {
                ClassExpression::Class(cls) => cls.iri.as_str().contains("B"),
                _ => false,
            }
        }));
    }

    #[test]
    fn test_structural_instances() {
        let mut o = Ontology::new();
        let a = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        });
        let ind = Individual::Named(NamedIndividual {
            iri: IRI::new("http://ex.org/ind"),
        });
        o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 1,
            class: a.clone(),
            individual: ind.clone(),
            annotations: vec![],
        }));

        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        let instances = reasoner.get_instances(&a, false).unwrap();
        assert!(!instances.is_empty());
        assert!(instances.contains_entity(&ind));
    }

    #[test]
    fn test_structural_types() {
        let mut o = Ontology::new();
        let a = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        });
        let ind = Individual::Named(NamedIndividual {
            iri: IRI::new("http://ex.org/ind"),
        });
        o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 1,
            class: a.clone(),
            individual: ind.clone(),
            annotations: vec![],
        }));

        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        let types = reasoner.get_types(&ind, false).unwrap();
        assert!(!types.is_empty());
        assert!(types.contains_entity(&a));
    }

    #[test]
    fn test_structural_equivalent_classes() {
        let mut o = Ontology::new();
        let a = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        });
        let b = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        });
        o.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 1,
            classes: vec![a.clone(), b.clone()],
            annotations: vec![],
        }));

        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        let eq = reasoner.get_equivalent_classes(&a).unwrap();
        assert!(eq.get_size() >= 2);
        assert!(eq.contains(&b));
    }

    #[test]
    fn test_structural_same_individuals() {
        let mut o = Ontology::new();
        let i1 = Individual::Named(NamedIndividual {
            iri: IRI::new("http://ex.org/i1"),
        });
        let i2 = Individual::Named(NamedIndividual {
            iri: IRI::new("http://ex.org/i2"),
        });
        o.add_axiom(Axiom::SameIndividual(SameIndividualAxiom {
            id: 1,
            individuals: vec![i1.clone(), i2.clone()],
            annotations: vec![],
        }));

        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        let same = reasoner.get_same_individuals(&i1).unwrap();
        assert!(same.get_size() >= 2);
        assert!(same.contains(&i2));
    }

    #[test]
    fn test_structural_is_entailed() {
        let o = make_simple_ontology();
        let reasoner = StructuralReasoner::new(make_onto_ref(o));
        let a = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        });
        let b = ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        });
        let ax = Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: a,
            superclass: b,
            annotations: vec![],
        });
        assert!(reasoner.is_entailed(&ax).unwrap());
    }

    #[test]
    fn test_structural_factory() {
        let o = make_simple_ontology();
        let onto_ref = make_onto_ref(o);
        let factory = StructuralReasonerFactory;
        let reasoner = factory
            .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
            .unwrap();
        assert!(reasoner.is_consistent().unwrap());
        assert_eq!(factory.get_reasoner_name(), "Oxidowl Structural Reasoner");
    }

    // ── TableauOWLReasoner Tests ─────────────────────────────────────────────

    #[test]
    fn test_tableau_factory() {
        let o = make_simple_ontology();
        let onto_ref = make_onto_ref(o);
        let factory = TableauReasonerFactory;
        let reasoner = factory
            .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
            .unwrap();
        assert!(reasoner.is_consistent().unwrap());
        assert_eq!(factory.get_reasoner_name(), "Oxidowl Tableau Reasoner");
    }

    #[test]
    fn test_reasoner_top_properties() {
        let o = make_simple_ontology();
        let onto_ref = make_onto_ref(o);
        let factory = StructuralReasonerFactory;
        let reasoner = factory
            .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
            .unwrap();

        let top_op = reasoner.get_top_object_property();
        assert!(matches!(
            top_op,
            ObjectPropertyExpression::ObjectProperty(_)
        ));

        let bot_op = reasoner.get_bottom_object_property();
        assert!(matches!(
            bot_op,
            ObjectPropertyExpression::ObjectProperty(_)
        ));
    }

    #[test]
    fn test_reasoner_inverse_properties() {
        let o = make_simple_ontology();
        let onto_ref = make_onto_ref(o);
        let factory = StructuralReasonerFactory;
        let reasoner = factory
            .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
            .unwrap();

        let prop = ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new("http://ex.org/prop"),
        });
        let inv = reasoner.get_inverse_object_properties(&prop).unwrap();
        assert!(matches!(
            inv.get_representative_element(),
            ObjectPropertyExpression::InverseObjectProperty(_)
        ));
    }

    #[test]
    fn test_reasoner_config() {
        let config = OWLReasonerConfiguration::default();
        assert!(matches!(config.buffering_mode, BufferingMode::NonBuffering));
        assert!(config.timeout.is_none());
    }

    #[test]
    fn test_inference_types() {
        assert_eq!(
            InferenceType::ClassHierarchy as u8,
            InferenceType::ClassHierarchy as u8
        );
    }
}

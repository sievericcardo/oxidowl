//! Unit tests for ontology components

use oxidowl::{
    Result,
    ontology::{
        Ontology, Class, Individual, IRI, ClassExpression, 
        Axiom, SubClassOfAxiom, ClassAssertionAxiom,
        ObjectProperty, ObjectPropertyExpression,
        concepts::ConceptStore,
        individuals::IndividualStore,
        properties::PropertyStore,
        axioms::AxiomStore,
    },
};

#[test]
fn test_ontology_creation() {
    let ontology = Ontology::new();
    
    assert_eq!(ontology.classes().len(), 0);
    assert_eq!(ontology.individuals().len(), 0);
    assert_eq!(ontology.axioms().len(), 0);
    
    println!("✅ Ontology creation works");
}

#[test]
fn test_class_management() {
    let mut ontology = Ontology::new();
    
    // Add classes
    let animal_class = Class::new(IRI::new("http://example.org/Animal"));
    let dog_class = Class::new(IRI::new("http://example.org/Dog"));
    
    ontology.add_class(animal_class.clone());
    ontology.add_class(dog_class.clone());
    
    assert_eq!(ontology.classes().len(), 2);
    assert!(ontology.contains_class(&animal_class.iri));
    assert!(ontology.contains_class(&dog_class.iri));
    
    println!("✅ Class management works");
}

#[test]
fn test_individual_management() {
    let mut ontology = Ontology::new();
    
    // Add individuals
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    let rex = Individual::named(IRI::new("http://example.org/Rex"));
    
    ontology.add_individual(fido.clone());
    ontology.add_individual(rex.clone());
    
    assert_eq!(ontology.individuals().len(), 2);
    
    // Check if individuals exist
    if let Some(fido_iri) = fido.iri() {
        assert!(ontology.contains_individual(fido_iri));
    }
    
    println!("✅ Individual management works");
}

#[test]
fn test_axiom_management() {
    let mut ontology = Ontology::new();
    
    // Create SubClassOf axiom: Dog ⊑ Animal
    let dog_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog")));
    let animal_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal")));
    
    let axiom = Axiom::SubClassOf(SubClassOfAxiom {
        id: "dog_animal".to_string(),
        subclass: dog_class,
        superclass: animal_class,
        annotations: Vec::new(),
    });
    
    ontology.add_axiom(axiom.clone());
    
    assert_eq!(ontology.axioms().len(), 1);
    
    println!("✅ Axiom management works");
}

#[test]
fn test_class_assertion() {
    let mut ontology = Ontology::new();
    
    // Add class and individual
    let dog_class = Class::new(IRI::new("http://example.org/Dog"));
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    
    ontology.add_class(dog_class.clone());
    ontology.add_individual(fido.clone());
    
    // Create class assertion: Fido is a Dog
    let axiom = Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "fido_dog".to_string(),
        class: ClassExpression::Class(dog_class),
        individual: fido,
        annotations: Vec::new(),
    });
    
    ontology.add_axiom(axiom);
    
    assert_eq!(ontology.axioms().len(), 1);
    
    println!("✅ Class assertion works");
}

#[test]
fn test_object_property_management() {
    let mut ontology = Ontology::new();
    
    // Add object properties
    let has_child = ObjectProperty::new(IRI::new("http://example.org/hasChild"));
    let has_parent = ObjectProperty::new(IRI::new("http://example.org/hasParent"));
    
    ontology.add_object_property(has_child.clone());
    ontology.add_object_property(has_parent.clone());
    
    assert_eq!(ontology.object_properties().len(), 2);
    
    println!("✅ Object property management works");
}

#[test]
fn test_iri_functionality() {
    let iri1 = IRI::new("http://example.org/Animal");
    let iri2 = IRI::new("http://example.org/Animal");
    let iri3 = IRI::new("http://example.org/Dog");
    
    assert_eq!(iri1, iri2);
    assert_ne!(iri1, iri3);
    
    assert_eq!(iri1.to_string(), "http://example.org/Animal");
    
    println!("✅ IRI functionality works");
}

#[test]
fn test_class_expression_creation() {
    let animal_class = Class::new(IRI::new("http://example.org/Animal"));
    let dog_class = Class::new(IRI::new("http://example.org/Dog"));
    
    // Simple class expression
    let simple_expr = ClassExpression::Class(animal_class.clone());
    
    // Complex class expression (intersection)
    let intersection_expr = ClassExpression::Intersection(vec![
        ClassExpression::Class(animal_class),
        ClassExpression::Class(dog_class),
    ]);
    
    // Verify expressions can be created
    match simple_expr {
        ClassExpression::Class(_) => (),
        _ => panic!("Should be a simple class expression"),
    }
    
    match intersection_expr {
        ClassExpression::Intersection(ref classes) => {
            assert_eq!(classes.len(), 2);
        }
        _ => panic!("Should be an intersection expression"),
    }
    
    println!("✅ Class expression creation works");
}

#[test]
fn test_concept_store() {
    let mut store = ConceptStore::new();
    
    let animal_class = Class::new(IRI::new("http://example.org/Animal"));
    let dog_class = Class::new(IRI::new("http://example.org/Dog"));
    
    store.add_class(animal_class.clone());
    store.add_class(dog_class.clone());
    
    assert_eq!(store.classes().len(), 2);
    assert!(store.contains_class(&animal_class.iri));
    assert!(store.contains_class(&dog_class.iri));
    
    println!("✅ ConceptStore works");
}

#[test]
fn test_individual_store() {
    let mut store = IndividualStore::new();
    
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    let rex = Individual::named(IRI::new("http://example.org/Rex"));
    
    store.add_individual(fido.clone());
    store.add_individual(rex.clone());
    
    assert_eq!(store.individuals().len(), 2);
    
    println!("✅ IndividualStore works");
}

#[test]
fn test_property_store() {
    let mut store = PropertyStore::new();
    
    let has_child = ObjectProperty::new(IRI::new("http://example.org/hasChild"));
    let has_parent = ObjectProperty::new(IRI::new("http://example.org/hasParent"));
    
    store.add_object_property(has_child.clone());
    store.add_object_property(has_parent.clone());
    
    assert_eq!(store.object_properties().len(), 2);
    
    println!("✅ PropertyStore works");
}

#[test]
fn test_axiom_store() {
    let mut store = AxiomStore::new();
    
    let dog_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog")));
    let animal_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal")));
    
    let axiom = Axiom::SubClassOf(SubClassOfAxiom {
        id: "dog_animal".to_string(),
        subclass: dog_class,
        superclass: animal_class,
        annotations: Vec::new(),
    });
    
    store.add_axiom(axiom.clone());
    
    assert_eq!(store.axioms().len(), 1);
    
    println!("✅ AxiomStore works");
}

#[test]
fn test_anonymous_individuals() {
    let mut ontology = Ontology::new();
    
    // Create anonymous individual
    let anon_individual = Individual::anonymous("_:anon1".to_string());
    
    ontology.add_individual(anon_individual.clone());
    
    assert_eq!(ontology.individuals().len(), 1);
    
    // Anonymous individuals should not have IRIs
    assert!(anon_individual.iri().is_none());
    
    println!("✅ Anonymous individuals work");
}

#[test]
fn test_ontology_queries() {
    let mut ontology = Ontology::new();
    
    // Add classes
    let animal_class = Class::new(IRI::new("http://example.org/Animal"));
    let dog_class = Class::new(IRI::new("http://example.org/Dog"));
    
    ontology.add_class(animal_class.clone());
    ontology.add_class(dog_class.clone());
    
    // Add individuals
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    ontology.add_individual(fido.clone());
    
    // Add axiom
    let axiom = Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "fido_dog".to_string(),
        class: ClassExpression::Class(dog_class.clone()),
        individual: fido.clone(),
        annotations: Vec::new(),
    });
    ontology.add_axiom(axiom);
    
    // Query for class assertions
    let class_assertions = ontology.get_class_assertions();
    assert_eq!(class_assertions.len(), 1);
    
    // Query for subclass axioms
    let subclass_axioms = ontology.get_subclass_axioms();
    assert_eq!(subclass_axioms.len(), 0); // No subclass axioms added
    
    println!("✅ Ontology queries work");
}

#[test]
fn test_ontology_statistics() {
    let mut ontology = Ontology::new();
    
    // Add various elements
    ontology.add_class(Class::new(IRI::new("http://example.org/Animal")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Dog")));
    
    ontology.add_individual(Individual::named(IRI::new("http://example.org/Fido")));
    
    let axiom = Axiom::SubClassOf(SubClassOfAxiom {
        id: "dog_animal".to_string(),
        subclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        superclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal"))),
        annotations: Vec::new(),
    });
    ontology.add_axiom(axiom);
    
    // Get statistics
    let stats = ontology.get_statistics();
    
    assert_eq!(stats.class_count, 2);
    assert_eq!(stats.individual_count, 1);
    assert_eq!(stats.axiom_count, 1);
    
    println!("✅ Ontology statistics work");
}

#[test]
fn test_ontology_validation() -> Result<()> {
    let mut ontology = Ontology::new();
    
    // Add classes
    let animal_class = Class::new(IRI::new("http://example.org/Animal"));
    let dog_class = Class::new(IRI::new("http://example.org/Dog"));
    
    ontology.add_class(animal_class.clone());
    ontology.add_class(dog_class.clone());
    
    // Add individual
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    ontology.add_individual(fido.clone());
    
    // Add valid axiom
    let axiom = Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "fido_dog".to_string(),
        class: ClassExpression::Class(dog_class),
        individual: fido,
        annotations: Vec::new(),
    });
    ontology.add_axiom(axiom);
    
    // Validate ontology
    let validation_result = ontology.validate();
    assert!(validation_result.is_ok(), "Valid ontology should pass validation");
    
    println!("✅ Ontology validation works");
    Ok(())
}

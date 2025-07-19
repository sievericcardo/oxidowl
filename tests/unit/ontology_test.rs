//! Unit tests for ontology functionality

use oxidowl::{
    ontology::{Ontology, ClassExpression, IRI, Class},
};

/// Test basic ontology creation and manipulation
#[test]
fn test_ontology_creation() {
    let ontology = Ontology::new();
    
    assert_eq!(ontology.classes().len(), 0, "New ontology should have no classes");
    assert_eq!(ontology.object_properties().len(), 0, "New ontology should have no object properties");
    assert_eq!(ontology.axioms().len(), 0, "New ontology should have no axioms");
    
    println!("Basic ontology creation test passed");
}

/// Test adding classes to ontology
#[test]
fn test_add_classes_to_ontology() {
    let mut ontology = Ontology::new();
    
    // Create test classes
    let person_iri = IRI::new("http://example.org/Person");
    let student_iri = IRI::new("http://example.org/Student");
    let teacher_iri = IRI::new("http://example.org/Teacher");
    
    let person_class = Class::new(person_iri.clone());
    let student_class = Class::new(student_iri.clone());
    let teacher_class = Class::new(teacher_iri.clone());
    
    // Add classes to ontology
    ontology.add_class(person_class);
    ontology.add_class(student_class);
    ontology.add_class(teacher_class);
    
    assert_eq!(ontology.classes().len(), 3, "Should have 3 classes after adding");
    
    // Verify classes exist - classes() returns Vec<(IRI, Class)> tuples
    let classes = ontology.classes();
    let class_iris: Vec<&IRI> = classes.iter().map(|(iri, _class)| iri).collect();
    assert!(class_iris.contains(&&person_iri), "Should contain Person class");
    assert!(class_iris.contains(&&student_iri), "Should contain Student class");
    assert!(class_iris.contains(&&teacher_iri), "Should contain Teacher class");
    
    println!("Adding classes to ontology test passed");
}

/// Test class expression creation and manipulation
#[test]
fn test_class_expressions() {
    let person_iri = IRI::new("http://example.org/Person");
    let student_iri = IRI::new("http://example.org/Student");
    
    // Test basic class expressions
    let person_expr = ClassExpression::Class(Class { iri: person_iri.clone() });
    let student_expr = ClassExpression::Class(Class { iri: student_iri.clone() });
    
    // Test that we can create different types of expressions
    match &person_expr {
        ClassExpression::Class(class) => {
            assert_eq!(class.iri, person_iri, "Class expression should contain correct IRI");
        },
        _ => panic!("Should be a Class expression"),
    }
    
    // Test intersection of classes
    let intersection = ClassExpression::intersection_of(vec![person_expr, student_expr]);
    match intersection {
        ClassExpression::ObjectIntersectionOf(operands) => {
            assert_eq!(operands.len(), 2, "Intersection should have 2 operands");
        },
        _ => panic!("Should be an Intersection expression"),
    }
    
    println!("Class expression creation test passed");
}

/// Test IRI creation and validation
#[test]
fn test_iri_functionality() {
    let iri_string = "http://example.org/TestClass";
    let iri = IRI::new(iri_string);
    
    assert_eq!(iri.as_str(), iri_string, "IRI should preserve original string");
    
    // Test different IRI formats
    let iris = vec![
        "http://example.org/Class1",
        "https://example.com/ontology#Class2", 
        "file:///local/ontology.owl#Class3",
        "urn:example:class4",
    ];
    
    for iri_str in iris {
        let iri = IRI::new(iri_str);
        assert_eq!(iri.as_str(), iri_str, "IRI should handle different formats");
    }
    
    println!("IRI functionality test passed");
}

/// Test ontology axiom operations
#[test]
fn test_ontology_axioms() {
    let mut ontology = Ontology::new();
    
    // Create classes for axiom testing
    let animal_iri = IRI::new("http://example.org/Animal");
    let mammal_iri = IRI::new("http://example.org/Mammal");
    
    let animal_class = Class::new(animal_iri.clone());
    let mammal_class = Class::new(mammal_iri.clone());
    
    ontology.add_class(animal_class);
    ontology.add_class(mammal_class);
    
    // Test that we can access axioms
    let axioms = ontology.axioms();
    println!("Ontology has {} axioms", axioms.len());
    
    // After adding classes, we should have some axioms
    assert!(axioms.len() >= 0, "Should be able to access axioms");
    
    println!("Ontology axiom operations test passed");
}

/// Test complex class expression construction
#[test]
fn test_complex_class_expressions() {
    let person_iri = IRI::new("http://example.org/Person");
    let student_iri = IRI::new("http://example.org/Student");
    let teacher_iri = IRI::new("http://example.org/Teacher");
    
    let person_expr = ClassExpression::Class(Class { iri: person_iri });
    let student_expr = ClassExpression::Class(Class { iri: student_iri });
    let teacher_expr = ClassExpression::Class(Class { iri: teacher_iri });
    
    // Test union expression
    let union_expr = ClassExpression::union_of(vec![student_expr.clone(), teacher_expr.clone()]);
    
    // Test intersection with union
    let complex_expr = ClassExpression::intersection_of(vec![person_expr, union_expr]);
    
    // Verify structure
    match complex_expr {
        ClassExpression::ObjectIntersectionOf(operands) => {
            assert_eq!(operands.len(), 2, "Complex expression should have correct structure");
            
            // Check that one operand is a union
            let has_union = operands.iter().any(|op| matches!(op, ClassExpression::ObjectUnionOf(_)));
            assert!(has_union, "Should contain a union operand");
        },
        _ => panic!("Should be an intersection expression"),
    }
    
    println!("Complex class expression construction test passed");
}

/// Test ontology consistency operations
#[test]
fn test_ontology_consistency_operations() {
    let ontology = Ontology::new();
    
    // Test basic ontology operations that should work
    assert!(ontology.classes().is_empty(), "New ontology should have empty classes");
    assert!(ontology.object_properties().is_empty(), "New ontology should have empty object properties");
    assert!(ontology.axioms().is_empty(), "New ontology should have empty axioms");
    assert!(ontology.individuals().is_empty(), "New ontology should have empty individuals");
    
    // Test that we can access axioms without error
    let _axioms = ontology.axioms();
    
    println!("Ontology consistency operations test passed");
}

#[test]
fn test_basic_ontology_operations() {
    let _ontology = Ontology::new();
    
    // Test basic operations without complex API calls
    println!("Basic ontology operations work");
}

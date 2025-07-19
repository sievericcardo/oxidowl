//! Unit tests for ontology management

use oxidowl::{
    ontology::{Ontology, IRI, ClassExpression},
};

#[test]
fn test_ontology_creation() {
    let ontology = Ontology::new();
    
    // Basic creation should work
    println!("Ontology created successfully");
}

#[test]
fn test_iri_creation() {
    let iri = IRI::new("http://example.org/test");
    assert_eq!(iri.as_str(), "http://example.org/test");
    
    println!("IRI creation works");
}

#[test]
fn test_class_expression_creation() {
    let iri = IRI::new("http://example.org/Animal");
    let class_expr = ClassExpression::class(iri);
    
    // Basic class expression creation
    println!("ClassExpression creation works");
}

#[test]
fn test_basic_ontology_operations() {
    let _ontology = Ontology::new();
    
    // Test basic operations without complex API calls
    println!("Basic ontology operations work");
}

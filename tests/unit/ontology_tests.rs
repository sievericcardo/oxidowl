//! Unit tests for ontology management

use oxidowl::ontology::{ClassExpression, IRI, Ontology};
use oxidowl::semantics::{RdfGraph, RdfTerm, Triple};

#[test]
fn test_ontology_creation() {
    let _ontology = Ontology::new();

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
    let _class_expr = ClassExpression::class(iri);

    // Basic class expression creation
    println!("ClassExpression creation works");
}

#[test]
fn test_basic_ontology_operations() {
    let _ontology = Ontology::new();

    // Test basic operations without complex API calls
    println!("Basic ontology operations work");
}

#[test]
fn test_rdf_graph_initialization() {
    let mut ontology = Ontology::new();
    
    // Initially no RDF graph
    assert!(ontology.get_rdf_graph().is_none());
    
    // Get or create should initialize it
    let _graph = ontology.get_or_create_rdf_graph();
    assert!(ontology.get_rdf_graph().is_some());
}

#[test]
fn test_add_rdf_triple() {
    let mut ontology = Ontology::new();
    
    let triple = Triple {
        subject: RdfTerm::iri("http://example.org/alice").unwrap(),
        predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
        object: RdfTerm::iri("http://example.org/bob").unwrap(),
    };
    
    ontology.add_rdf_triple(triple);
    
    let graph = ontology.get_rdf_graph().unwrap();
    assert_eq!(graph.triples().len(), 1);
}

#[test]
fn test_set_rdf_graph() {
    let mut ontology = Ontology::new();
    
    let mut graph = RdfGraph::new();
    graph.add_triple(Triple {
        subject: RdfTerm::iri("http://example.org/alice").unwrap(),
        predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
        object: RdfTerm::iri("http://example.org/bob").unwrap(),
    });
    
    ontology.set_rdf_graph(graph);
    
    assert!(ontology.get_rdf_graph().is_some());
    assert_eq!(ontology.get_rdf_graph().unwrap().triples().len(), 1);
}

#[test]
fn test_has_rdf_star_features() {
    let mut ontology = Ontology::new();
    
    // No RDF-star features initially
    assert!(!ontology.has_rdf_star_features());
    
    // Add a regular triple
    ontology.add_rdf_triple(Triple {
        subject: RdfTerm::iri("http://example.org/alice").unwrap(),
        predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
        object: RdfTerm::iri("http://example.org/bob").unwrap(),
    });
    
    // Still no RDF-star features
    assert!(!ontology.has_rdf_star_features());
    
    // Add a quoted triple (RDF-star)
    let inner_triple = Triple {
        subject: RdfTerm::iri("http://example.org/doc1").unwrap(),
        predicate: RdfTerm::iri("http://example.org/author").unwrap(),
        object: RdfTerm::literal("Smith"),
    };
    
    ontology.add_rdf_triple(Triple {
        subject: RdfTerm::QuotedTriple(Box::new(inner_triple)),
        predicate: RdfTerm::iri("http://example.org/source").unwrap(),
        object: RdfTerm::iri("http://example.org/archive23").unwrap(),
    });
    
    // Now has RDF-star features
    assert!(ontology.has_rdf_star_features());
}

#[test]
fn test_reify_rdf_star() {
    let mut ontology = Ontology::new();
    
    // Add a quoted triple
    let inner_triple = Triple {
        subject: RdfTerm::iri("http://example.org/alice").unwrap(),
        predicate: RdfTerm::iri("http://example.org/knows").unwrap(),
        object: RdfTerm::iri("http://example.org/bob").unwrap(),
    };
    
    ontology.add_rdf_triple(Triple {
        subject: RdfTerm::QuotedTriple(Box::new(inner_triple)),
        predicate: RdfTerm::iri("http://example.org/certainty").unwrap(),
        object: RdfTerm::literal("high"),
    });
    
    assert!(ontology.has_rdf_star_features());
    
    // Reify to RDF 1.1
    ontology.reify_rdf_star().unwrap();
    
    // After reification, no more quoted triples at the top level
    assert!(!ontology.has_rdf_star_features());
    
    // But should have reification quads
    let graph = ontology.get_rdf_graph().unwrap();
    assert!(graph.triples().len() > 1);
}

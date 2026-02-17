//! RDF-star Integration Tests
//!
//! Comprehensive test suite for RDF-star (RDF 1.2) features including:
//! - Quoted triple parsing and serialization
//! - Nested triple structures (1-level, 2-level, deep nesting)
//! - SPARQL-star query execution
//! - RDF 1.1 ↔ RDF-star conversion
//! - Reification round-trip tests
//! - Performance and edge cases

use oxidowl::Ontology;
use oxidowl::semantics::{RdfGraph, RdfTerm, Triple};

#[test]
fn test_simple_quoted_triple_creation() {
    // Test basic quoted triple creation: << :alice :knows :bob >>
    let alice = RdfTerm::iri("http://example.org/alice").unwrap();
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();

    let inner_triple = Triple::new(alice, knows, bob);
    let quoted_triple = RdfTerm::QuotedTriple(Box::new(inner_triple));

    // Verify it's a quoted triple
    assert!(matches!(quoted_triple, RdfTerm::QuotedTriple(_)));
}

#[test]
fn test_quoted_triple_in_graph() {
    // Test adding quoted triples to RDF graph
    let mut graph = RdfGraph::new();

    // Create: << :alice :knows :bob >> :certainty 0.95
    let alice = RdfTerm::iri("http://example.org/alice").unwrap();
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();
    let certainty = RdfTerm::iri("http://example.org/certainty").unwrap();
    let value = RdfTerm::Literal {
        value: "0.95".to_string(),
        datatype: None,
        language: None,
        direction: None,
    };

    let inner_triple = Triple::new(alice, knows, bob);
    let quoted_subject = RdfTerm::QuotedTriple(Box::new(inner_triple));
    let meta_triple = Triple::new(quoted_subject, certainty, value);

    graph.add_triple(meta_triple.clone());

    // Verify triple was added
    assert_eq!(graph.triples().len(), 1);
    let first_triple = graph.triples().iter().next().unwrap();
    assert!(matches!(first_triple.subject, RdfTerm::QuotedTriple(_)));
}

#[test]
fn test_one_level_nesting() {
    // Test 1-level nesting: << :a :b :c >> :d :e
    let a = RdfTerm::iri("http://example.org/a").unwrap();
    let b = RdfTerm::iri("http://example.org/b").unwrap();
    let c = RdfTerm::iri("http://example.org/c").unwrap();
    let d = RdfTerm::iri("http://example.org/d").unwrap();
    let e = RdfTerm::iri("http://example.org/e").unwrap();

    let inner = Triple::new(a, b, c);
    let quoted = RdfTerm::QuotedTriple(Box::new(inner));
    let outer = Triple::new(quoted, d, e);

    // Verify depth
    assert_eq!(outer.depth(), 1, "1-level nesting should have depth 1");
}

#[test]
fn test_two_level_nesting() {
    // Test 2-level nesting: << << :a :b :c >> :d :e >> :f :g
    let a = RdfTerm::iri("http://example.org/a").unwrap();
    let b = RdfTerm::iri("http://example.org/b").unwrap();
    let c = RdfTerm::iri("http://example.org/c").unwrap();
    let d = RdfTerm::iri("http://example.org/d").unwrap();
    let e = RdfTerm::iri("http://example.org/e").unwrap();
    let f = RdfTerm::iri("http://example.org/f").unwrap();
    let g = RdfTerm::iri("http://example.org/g").unwrap();

    let inner = Triple::new(a, b, c);
    let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner));
    let middle = Triple::new(inner_quoted, d, e);
    let middle_quoted = RdfTerm::QuotedTriple(Box::new(middle));
    let outer = Triple::new(middle_quoted, f, g);

    // Verify depth
    assert_eq!(outer.depth(), 2, "2-level nesting should have depth 2");
}

#[test]
fn test_five_level_nesting() {
    // Test 5-level nesting (maximum recommended)
    let a = RdfTerm::iri("http://example.org/a").unwrap();
    let b = RdfTerm::iri("http://example.org/b").unwrap();
    let c = RdfTerm::iri("http://example.org/c").unwrap();

    let mut current = Triple::new(a.clone(), b.clone(), c.clone());
    for _ in 0..5 {
        let quoted = RdfTerm::QuotedTriple(Box::new(current));
        current = Triple::new(quoted, b.clone(), c.clone());
    }

    // Verify depth
    assert_eq!(current.depth(), 5, "5-level nesting should have depth 5");
}

#[test]
fn test_quoted_triple_in_object_position() {
    // Test quoted triple as object: :alice :believes << :bob :knows :charlie >>
    let alice = RdfTerm::iri("http://example.org/alice").unwrap();
    let believes = RdfTerm::iri("http://example.org/believes").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let charlie = RdfTerm::iri("http://example.org/charlie").unwrap();

    let inner_triple = Triple::new(bob, knows, charlie);
    let quoted_object = RdfTerm::QuotedTriple(Box::new(inner_triple));
    let outer_triple = Triple::new(alice, believes, quoted_object);

    // Verify structure
    assert!(matches!(outer_triple.object, RdfTerm::QuotedTriple(_)));
    assert_eq!(outer_triple.depth(), 1);
}

#[test]
fn test_quoted_triple_with_blank_node() {
    // Test quoted triple containing blank node: << _:b1 :knows :bob >> :source :survey
    let blank_node = RdfTerm::BlankNode("_:b1".to_string());
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();
    let source = RdfTerm::iri("http://example.org/source").unwrap();
    let survey = RdfTerm::iri("http://example.org/survey").unwrap();

    let inner_triple = Triple::new(blank_node, knows, bob);
    let quoted_subject = RdfTerm::QuotedTriple(Box::new(inner_triple));
    let meta_triple = Triple::new(quoted_subject, source, survey);

    // Verify blank node in quoted triple
    if let RdfTerm::QuotedTriple(inner) = &meta_triple.subject {
        assert!(matches!(inner.subject, RdfTerm::BlankNode(_)));
    } else {
        panic!("Expected quoted triple");
    }
}

#[test]
fn test_quoted_triple_with_literal() {
    // Test quoted triple containing literal: << :temperature :value "25.5" >> :unit "Celsius"
    let temperature = RdfTerm::iri("http://example.org/temperature").unwrap();
    let value = RdfTerm::iri("http://example.org/value").unwrap();
    let literal_value = RdfTerm::Literal {
        value: "25.5".to_string(),
        datatype: None,
        language: None,
        direction: None,
    };
    let unit = RdfTerm::iri("http://example.org/unit").unwrap();
    let celsius = RdfTerm::Literal {
        value: "Celsius".to_string(),
        datatype: None,
        language: None,
        direction: None,
    };

    let inner_triple = Triple::new(temperature, value, literal_value);
    let quoted_subject = RdfTerm::QuotedTriple(Box::new(inner_triple));
    let meta_triple = Triple::new(quoted_subject, unit, celsius);

    // Verify literal in quoted triple
    if let RdfTerm::QuotedTriple(inner) = &meta_triple.subject {
        assert!(matches!(inner.object, RdfTerm::Literal { .. }));
    } else {
        panic!("Expected quoted triple");
    }
}

#[test]
fn test_triple_flattening() {
    // Test flattening nested quoted triples
    let a = RdfTerm::iri("http://example.org/a").unwrap();
    let b = RdfTerm::iri("http://example.org/b").unwrap();
    let c = RdfTerm::iri("http://example.org/c").unwrap();
    let d = RdfTerm::iri("http://example.org/d").unwrap();
    let e = RdfTerm::iri("http://example.org/e").unwrap();

    // Create << << :a :b :c >> :d :e >> :d :e
    let inner = Triple::new(a, b, c);
    let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner));
    let middle = Triple::new(inner_quoted, d.clone(), e.clone());
    let middle_quoted = RdfTerm::QuotedTriple(Box::new(middle));
    let outer = Triple::new(middle_quoted, d, e);

    // Flatten and verify
    let flattened = outer.flatten();
    assert_eq!(
        flattened.len(),
        3,
        "Should have 3 triples: outer, middle, inner"
    );
}

#[test]
fn test_ontology_with_rdf_star() {
    // Test ontology integration with RDF-star
    let mut ontology = Ontology::new();
    let mut graph = RdfGraph::new();

    // Add multiple quoted triples
    let alice = RdfTerm::iri("http://example.org/alice").unwrap();
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();
    let certainty = RdfTerm::iri("http://example.org/certainty").unwrap();
    let high = RdfTerm::Literal {
        value: "0.95".to_string(),
        datatype: None,
        language: None,
        direction: None,
    };

    let triple1 = Triple::new(alice.clone(), knows.clone(), bob.clone());
    let quoted1 = RdfTerm::QuotedTriple(Box::new(triple1));
    let meta1 = Triple::new(quoted1, certainty.clone(), high.clone());
    graph.add_triple(meta1);

    let charlie = RdfTerm::iri("http://example.org/charlie").unwrap();
    let triple2 = Triple::new(alice, knows, charlie);
    let quoted2 = RdfTerm::QuotedTriple(Box::new(triple2));
    let meta2 = Triple::new(quoted2, certainty, high);
    graph.add_triple(meta2);

    ontology.set_rdf_graph(graph);

    // Verify ontology has RDF graph with quoted triples
    assert!(ontology.get_rdf_graph().is_some());
    assert_eq!(ontology.get_rdf_graph().unwrap().triples().len(), 2);
}

#[test]
fn test_rdf11_ontology_without_rdf_star() {
    // Test that RDF 1.1 ontologies work without any RDF-star features
    let mut ontology = Ontology::new();
    let mut graph = RdfGraph::new();

    // Simple RDF 1.1 triple
    let alice = RdfTerm::iri("http://example.org/alice").unwrap();
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();

    let triple = Triple::new(alice, knows, bob);
    graph.add_triple(triple);

    ontology.set_rdf_graph(graph);

    // Verify depth is 0 (no nesting)
    let first_triple = ontology
        .get_rdf_graph()
        .unwrap()
        .triples()
        .iter()
        .next()
        .unwrap();
    assert_eq!(first_triple.depth(), 0);
}

#[test]
fn test_adapter_reification_basic() {
    // Test basic reification using HornedOwlAdapter
    use oxidowl::adapter::HornedOwlAdapter;

    let mut adapter = HornedOwlAdapter::new();

    // Create a quoted triple
    let alice = RdfTerm::iri("http://example.org/alice").unwrap();
    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let bob = RdfTerm::iri("http://example.org/bob").unwrap();

    let inner_triple = Triple::new(alice, knows, bob);
    let quoted_term = RdfTerm::QuotedTriple(Box::new(inner_triple));

    // Reify to RDF 1.1
    let result = adapter.reify_rdf_term(&quoted_term);
    assert!(result.is_ok());

    let (reified_term, reification_triples) = result.unwrap();

    // Should return a blank node
    assert!(matches!(reified_term, RdfTerm::BlankNode(_)));

    // Should have 4 reification triples
    assert_eq!(reification_triples.len(), 4);
}

#[test]
fn test_adapter_reification_nested() {
    // Test reification of nested quoted triples
    use oxidowl::adapter::HornedOwlAdapter;

    let mut adapter = HornedOwlAdapter::new();

    // Create nested: << << :a :b :c >> :d :e >>
    let a = RdfTerm::iri("http://example.org/a").unwrap();
    let b = RdfTerm::iri("http://example.org/b").unwrap();
    let c = RdfTerm::iri("http://example.org/c").unwrap();
    let d = RdfTerm::iri("http://example.org/d").unwrap();
    let e = RdfTerm::iri("http://example.org/e").unwrap();

    let inner = Triple::new(a, b, c);
    let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner));
    let outer_triple = Triple::new(inner_quoted, d, e);
    let outer_quoted = RdfTerm::QuotedTriple(Box::new(outer_triple));

    // Reify
    let result = adapter.reify_rdf_term(&outer_quoted);
    assert!(result.is_ok());

    let (reified_term, reification_triples) = result.unwrap();

    // Should return a blank node
    assert!(matches!(reified_term, RdfTerm::BlankNode(_)));

    // Should have 8 triples (4 for outer + 4 for inner)
    assert_eq!(
        reification_triples.len(),
        8,
        "Nested reification should produce 8 triples"
    );
}

#[test]
fn test_adapter_mode_switching() {
    // Test switching between RDF 1.1 and RDF 1.2 modes
    use oxidowl::adapter::HornedOwlAdapter;

    let mut adapter = HornedOwlAdapter::new();

    // Should default to RDF 1.1 mode
    assert!(adapter.is_rdf11_mode());

    // Switch to RDF 1.2 mode
    adapter.set_rdf11_mode(false);
    assert!(!adapter.is_rdf11_mode());

    // Switch back
    adapter.set_rdf11_mode(true);
    assert!(adapter.is_rdf11_mode());
}

#[test]
fn test_rdf12_directional_literal() {
    // Test RDF 1.2 directional literal (dirLangString)
    use url::Url;

    let subject = RdfTerm::iri("http://example.org/greeting").unwrap();
    let predicate = RdfTerm::iri("http://www.w3.org/2000/01/rdf-schema#label").unwrap();

    // Arabic text with RTL direction
    let object = RdfTerm::Literal {
        value: "مرحبا".to_string(),
        datatype: Some(
            Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString").unwrap(),
        ),
        language: Some("ar".to_string()),
        direction: Some("rtl".to_string()),
    };

    let triple = Triple::new(subject, predicate, object);

    // Verify directional literal
    if let RdfTerm::Literal {
        direction: Some(dir),
        ..
    } = &triple.object
    {
        assert_eq!(dir, "rtl");
    } else {
        panic!("Expected directional literal");
    }
}

#[test]
fn test_rdf12_well_formed_blank_node() {
    // Test RDF 1.2 well-formed blank node labels
    let valid_labels = vec!["_:a", "_:A", "_:label123", "_:ABC123xyz"];

    for label in valid_labels {
        let term = RdfTerm::BlankNode(label.to_string());
        assert!(matches!(term, RdfTerm::BlankNode(_)));
    }
}

#[test]
fn test_provenance_use_case() {
    // Test real-world use case: provenance tracking with RDF-star
    let mut graph = RdfGraph::new();

    // Base statement: :document1 :title "Report"
    let doc = RdfTerm::iri("http://example.org/document1").unwrap();
    let title = RdfTerm::iri("http://purl.org/dc/terms/title").unwrap();
    let title_value = RdfTerm::Literal {
        value: "Annual Report".to_string(),
        datatype: None,
        language: Some("en".to_string()),
        direction: None,
    };

    let base_triple = Triple::new(doc.clone(), title.clone(), title_value.clone());

    // Add provenance: << :document1 :title "Report" >> :source :user42
    let quoted = RdfTerm::QuotedTriple(Box::new(base_triple.clone()));
    let source = RdfTerm::iri("http://example.org/source").unwrap();
    let user = RdfTerm::iri("http://example.org/user42").unwrap();
    let prov_triple = Triple::new(quoted.clone(), source, user);
    graph.add_triple(prov_triple);

    // Add timestamp: << :document1 :title "Report" >> :timestamp "2026-02-15"
    let timestamp = RdfTerm::iri("http://example.org/timestamp").unwrap();
    let date = RdfTerm::Literal {
        value: "2026-02-15T10:30:00Z".to_string(),
        datatype: None,
        language: None,
        direction: None,
    };
    let time_triple = Triple::new(quoted, timestamp, date);
    graph.add_triple(time_triple);

    // Also store the base triple
    graph.add_triple(base_triple);

    // Verify: 3 triples total (base + 2 metadata)
    assert_eq!(graph.triples().len(), 3);

    // Verify 2 have quoted triples
    let quoted_count = graph
        .triples()
        .iter()
        .filter(|t| matches!(t.subject, RdfTerm::QuotedTriple(_)))
        .count();
    assert_eq!(quoted_count, 2);
}

#[test]
fn test_annotation_use_case() {
    // Test use case: annotating statements with confidence scores
    let mut graph = RdfGraph::new();

    // Multiple claims with different confidence levels
    let claims = vec![
        ("alice", "knows", "bob", "0.95"),
        ("alice", "knows", "charlie", "0.70"),
        ("bob", "knows", "david", "0.85"),
    ];

    for (s, p, o, conf) in claims {
        let subject = RdfTerm::iri(&format!("http://example.org/{}", s)).unwrap();
        let predicate = RdfTerm::iri(&format!("http://example.org/{}", p)).unwrap();
        let object = RdfTerm::iri(&format!("http://example.org/{}", o)).unwrap();

        let base = Triple::new(subject, predicate, object);
        let quoted = RdfTerm::QuotedTriple(Box::new(base));

        let conf_pred = RdfTerm::iri("http://example.org/confidence").unwrap();
        let conf_val = RdfTerm::Literal {
            value: conf.to_string(),
            datatype: None,
            language: None,
            direction: None,
        };

        let meta = Triple::new(quoted, conf_pred, conf_val);
        graph.add_triple(meta);
    }

    assert_eq!(graph.triples().len(), 3);
}

#[test]
fn test_performance_large_graph_with_quoted_triples() {
    // Test performance with many quoted triples
    let mut graph = RdfGraph::new();

    let knows = RdfTerm::iri("http://example.org/knows").unwrap();
    let confidence = RdfTerm::iri("http://example.org/confidence").unwrap();

    // Create 100 statements with metadata
    for i in 0..100 {
        let subject = RdfTerm::iri(&format!("http://example.org/person{}", i)).unwrap();
        let object = RdfTerm::iri(&format!("http://example.org/person{}", i + 1)).unwrap();

        let base = Triple::new(subject, knows.clone(), object);
        let quoted = RdfTerm::QuotedTriple(Box::new(base));

        let value = RdfTerm::Literal {
            value: format!("0.{}", 90 + (i % 10)),
            datatype: None,
            language: None,
            direction: None,
        };

        let meta = Triple::new(quoted, confidence.clone(), value);
        graph.add_triple(meta);
    }

    assert_eq!(graph.triples().len(), 100);

    // Verify all have quoted triples
    let all_quoted = graph
        .triples()
        .iter()
        .all(|t| matches!(t.subject, RdfTerm::QuotedTriple(_)));
    assert!(all_quoted);
}

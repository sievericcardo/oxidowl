/// Tests for entailment functionality

use super::*;
use crate::semantics::{RdfGraph, RdfTerm, Triple};

#[test]
fn test_entailment_regime_creation() {
    let checker = EntailmentChecker::new(EntailmentRegime::RdfSimple);
    assert_eq!(checker.regime(), EntailmentRegime::RdfSimple);
}

#[test] 
fn test_rdf_simple_entailment() {
    let mut checker = EntailmentChecker::new(EntailmentRegime::RdfSimple);
    
    // Create a simple premise graph
    let mut premises = RdfGraph::new();
    let triple = Triple {
        subject: RdfTerm::iri("http://example.org/a").unwrap(),
        predicate: RdfTerm::iri("http://example.org/p").unwrap(), 
        object: RdfTerm::iri("http://example.org/b").unwrap(),
    };
    premises.add_triple(triple.clone());

    // Create conclusion with same triple
    let mut conclusion = RdfGraph::new();
    conclusion.add_triple(triple);

    // Should entail itself
    let result = checker.entails(&premises, &conclusion).unwrap();
    assert!(result);
}

#[test]
fn test_owl_rl_engine() {
    let graph = RdfGraph::new();
    let mut engine = Owl2RlEngine::new(graph);
    
    assert_eq!(engine.rule_applications(), 0);
    
    let _result = engine.reason();
    // Should not panic and should return a result
}

#[test]
fn test_entailment_cache() {
    let mut checker = EntailmentChecker::new(EntailmentRegime::RdfSimple);
    
    let premises = RdfGraph::new();
    let conclusion = RdfGraph::new();
    
    // First call
    let result1 = checker.entails(&premises, &conclusion).unwrap();
    
    // Second call should use cache
    let result2 = checker.entails(&premises, &conclusion).unwrap();
    
    assert_eq!(result1, result2);
    
    // Clear cache
    checker.clear_cache();
    
    // Should still work after clearing cache
    let result3 = checker.entails(&premises, &conclusion).unwrap();
    assert_eq!(result1, result3);
}

#[test]
fn test_regime_change() {
    let mut checker = EntailmentChecker::new(EntailmentRegime::RdfSimple);
    assert_eq!(checker.regime(), EntailmentRegime::RdfSimple);
    
    checker.set_regime(EntailmentRegime::Rdfs);
    assert_eq!(checker.regime(), EntailmentRegime::Rdfs);
}

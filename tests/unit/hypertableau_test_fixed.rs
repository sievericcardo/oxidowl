//! Hypertableau Unit Tests
//!
//! Tests for the hypertableau reasoning algorithm implementation.

use oxidowl::core::hypertableau::*;
use oxidowl::ontology::*;
use oxidowl::core::reasoner::*;

#[test]
fn test_hypertableau_creation() {
    let ontology = Ontology::new();
    let config = HyperTableauConfig::default();
    
    let result = HyperTableau::new(std::sync::Arc::new(ontology), config);
    assert!(result.is_ok());
}

#[test]
fn test_extension_table_operations() {
    let mut table = ExtensionTable::new();
    
    // Test basic operations
    let predicate = "http://example.org/Person";
    let args = vec!["http://example.org/john".to_string()];
    
    // Add tuple
    let result = table.add_tuple(predicate, args.clone());
    assert!(result.is_ok());
    
    // Check if tuple exists
    assert!(table.contains_tuple(predicate, &args));
}

#[test]
fn test_clause_evaluator_creation() {
    let config = ClauseEvaluatorConfig::default();
    let evaluator = ClauseEvaluator::new(config);
    
    // Test that evaluator can be created
    assert_eq!(evaluator.get_evaluation_count(), 0);
}

#[test]
fn test_dependency_tracking() {
    let mut tracker = DependencySet::new();
    
    let dependency = "test_dependency".to_string();
    tracker.add_dependency(dependency.clone());
    
    assert!(tracker.contains(&dependency));
}

#[test]
fn test_hyperresolution_basic() {
    let config = HyperResolutionConfig::default();
    let resolver = HyperResolution::new(config);
    
    // Test basic creation and configuration
    assert!(resolver.is_optimization_enabled());
}

#[test]
fn test_ground_disjunction_handling() {
    let individual = Individual::Named(NamedIndividual {
        iri: IRI::new("http://example.org/john"),
    });
    
    let class1 = ClassExpression::Class(Class::new(IRI::new("http://example.org/A")));
    let class2 = ClassExpression::Class(Class::new(IRI::new("http://example.org/B")));
    
    let disjuncts = vec![class1, class2];
    let disjunction = GroundDisjunction::new(individual, disjuncts);
    
    assert_eq!(disjunction.get_disjunct_count(), 2);
}

#[test]
fn test_monitor_integration() {
    let monitor = ReasoningMonitor::new();
    
    // Test monitor can track reasoning progress
    monitor.on_reasoning_start();
    
    let stats = ReasoningStats {
        total_clauses_processed: 100,
        total_inferences_made: 50,
        elapsed_time_ms: 1000,
    };
    
    monitor.on_reasoning_complete(&stats);
}

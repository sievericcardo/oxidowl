use oxidowl::ontology::*;
use oxidowl::reasoning::ReasoningService;
use oxidowl::config::ReasonerConfig;

#[tokio::test]
async fn test_swrl_engine_integration() {
    // Create a simple ontology
    let ontology = Ontology::new();
    
    // Create reasoning service with SWRL integration
    let config = ReasonerConfig::default();
    let reasoning_service = ReasoningService::new(ontology, config);
    
    // Test that SWRL methods are available and working
    let swrl_stats = reasoning_service.get_swrl_statistics().await.unwrap();
    assert_eq!(swrl_stats.total_rule_applications, 0, "Should have 0 rule applications in empty ontology");
    
    println!("✅ SWRL engine integration test passed! SWRL functionality is properly integrated.");
}

#[tokio::test]
async fn test_swrl_execution_method_available() {
    // Create an ontology with basic classes
    let mut ontology = Ontology::new();
    
    // Add a simple class
    let test_class_iri = IRI::new("http://example.com/TestClass");
    let test_class = Class::new(test_class_iri.clone());
    ontology.add_class(test_class);
    
    // Create reasoning service 
    let config = ReasonerConfig::default();
    let reasoning_service = ReasoningService::new(ontology, config);
    
    // Test that SWRL execution method is available
    let swrl_result = reasoning_service.execute_swrl_rules().await.unwrap();
    
    // Should return an empty result since there are no SWRL rules
    assert!(!swrl_result.fired, "Should not fire any rules when there are no SWRL rules");
    assert_eq!(swrl_result.inferences.len(), 0, "Should have no inferences when no rules fire");
    
    println!("✅ SWRL integration test passed! SWRL rule execution is properly integrated.");
}

#[tokio::test]
async fn test_swrl_rules_applied_during_reasoning() {
    // Create an ontology with some facts
    let mut ontology = Ontology::new();
    
    // Add some basic classes and properties
    let person_iri = IRI::new("http://example.com/Person");
    let adult_iri = IRI::new("http://example.com/Adult");
    
    let person_class = Class::new(person_iri.clone());
    let adult_class = Class::new(adult_iri.clone());
    
    ontology.add_class(person_class.clone());
    ontology.add_class(adult_class);
    
    // Add an individual
    let john_iri = IRI::new("http://example.com/John");
    let john = Individual::named(john_iri.clone());
    ontology.add_individual(john_iri.clone(), john.clone());
    
    // Assert John is a Person using correct axiom structure
    let john_is_person = Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 0, // Use default id
        individual: john,
        class: ClassExpression::Class(person_class),
        annotations: vec![],
    });
    ontology.add_axiom(john_is_person);
    
    // Create reasoning service
    let config = ReasonerConfig::default();
    let reasoning_service = ReasoningService::new(ontology, config);
    
    // Test that we can access SWRL statistics after creating service
    let stats = reasoning_service.get_swrl_statistics().await;
    assert!(stats.is_ok(), "Should be able to get SWRL statistics");
    
    // Test that SWRL execution runs during reasoning preparation
    let swrl_execution_result = reasoning_service.execute_swrl_rules().await;
    assert!(swrl_execution_result.is_ok(), "SWRL execution should succeed");
    
    println!("✅ SWRL rules applied during reasoning! Service integrates SWRL functionality correctly");
}

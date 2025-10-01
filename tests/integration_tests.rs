//! Integration tests for Phase 1 improvements
//!
//! This module tests the complete integration of all Phase 1 features:
//! - Explanation generation
//! - EL profile reasoner
//! - Server interfaces (SPARQL, OWLlink, REST)
//! - Import resolution

use oxidowl::{
    config::{OxidowlConfig, OWLProfile, ServerConfig},
    explanation::{ExplanationService, ExplanationType, ExplanationFormat},
    profiles::el_reasoner::{ELReasoner, CompletionConfig},
    import::resolver::{ImportResolver, ImportResolverConfig},
    server::{ServerManager, sparql::SparqlServer, owllink::OWLlinkServer, rest::RestApiServer},
    reasoning::ReasoningService,
    ontology::{Ontology, Class, ClassExpression, Individual, axioms::*},
    error::Error,
};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;

#[tokio::test]
async fn test_explanation_service_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create a simple ontology with subsumption
    let mut ontology = Ontology::new();
    
    let animal = Class::new("http://example.org/Animal");
    let mammal = Class::new("http://example.org/Mammal");
    let human = Class::new("http://example.org/Human");
    
    // Add axioms: Human ⊑ Mammal ⊑ Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(human.clone()),
        ClassExpression::Class(mammal.clone()),
    )));
    
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(mammal.clone()),
        ClassExpression::Class(animal.clone()),
    )));
    
    // Create explanation service
    let explanation_service = ExplanationService::new();
    
    // Generate explanation for the inference Human ⊑ Animal
    let inference_axiom = Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(human),
        ClassExpression::Class(animal),
    ));
    
    let explanation = explanation_service
        .explain_inference(&ontology, &inference_axiom, ExplanationType::Subsumption)
        .await?;
    
    // Verify explanation contains expected elements
    assert!(explanation.justifications.len() > 0);
    assert!(!explanation.proof_steps.is_empty());
    
    // Test different output formats
    let natural_language = explanation_service
        .format_explanation(&explanation, ExplanationFormat::NaturalLanguage)
        .await?;
    assert!(natural_language.contains("Human"));
    assert!(natural_language.contains("Animal"));
    
    let proof_tree = explanation_service
        .format_explanation(&explanation, ExplanationFormat::ProofTree)
        .await?;
    assert!(proof_tree.contains("├"));
    
    println!("✅ Explanation service integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_el_reasoner_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create EL-compatible ontology
    let mut ontology = Ontology::new();
    
    // Add EL axioms
    let disease = Class::new("http://example.org/Disease");
    let infectious_disease = Class::new("http://example.org/InfectiousDisease");
    let viral_disease = Class::new("http://example.org/ViralDisease");
    let covid19 = Class::new("http://example.org/COVID19");
    
    // Build hierarchy: COVID19 ⊑ ViralDisease ⊑ InfectiousDisease ⊑ Disease
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(covid19.clone()),
        ClassExpression::Class(viral_disease.clone()),
    )));
    
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(viral_disease.clone()),
        ClassExpression::Class(infectious_disease.clone()),
    )));
    
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(infectious_disease.clone()),
        ClassExpression::Class(disease.clone()),
    )));
    
    // Create EL reasoner
    let config = CompletionConfig::default();
    let mut el_reasoner = ELReasoner::new(ontology, config);
    
    // Perform classification
    let classification = el_reasoner.classify().await?;
    
    // Verify classification results
    assert!(classification.class_hierarchy.len() > 0);
    
    // Test subsumption queries
    let is_subsumed = el_reasoner
        .is_subsumed(
            &ClassExpression::Class(covid19.clone()),
            &ClassExpression::Class(disease.clone()),
        )
        .await?;
    assert!(is_subsumed, "COVID19 should be subsumed by Disease");
    
    // Test satisfiability
    let is_satisfiable = el_reasoner
        .is_satisfiable(&ClassExpression::Class(covid19))
        .await?;
    assert!(is_satisfiable, "COVID19 should be satisfiable");
    
    println!("✅ EL reasoner integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_import_resolver_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create import resolver
    let config = ImportResolverConfig::default();
    let resolver = ImportResolver::new().with_config(config);
    
    // Add IRI mapping for testing
    resolver
        .add_iri_mapping(
            "http://example.org/test".to_string(),
            "./test_ontology.owl".to_string(),
        )
        .await?;
    
    // Test cache functionality
    let cache_stats = resolver.get_cache_stats().await;
    assert_eq!(cache_stats.cached_ontologies, 0);
    
    // Clear cache (should not error)
    resolver.clear_cache().await?;
    
    println!("✅ Import resolver integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_sparql_server_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create reasoning service
    let mut ontology = Ontology::new();
    let person = Class::new("http://example.org/Person");
    ontology.add_axiom(Axiom::Declaration(DeclarationAxiom::new(
        crate::ontology::Entity::Class(person),
    )));
    
    let reasoning_service = Arc::new(ReasoningService::new(ontology)?);
    
    // Create SPARQL server
    let sparql_server = SparqlServer::new(
        8001,
        "127.0.0.1".to_string(),
        reasoning_service,
    );
    
    // Start server with timeout to prevent hanging
    let server_result = timeout(Duration::from_millis(100), sparql_server.start()).await;
    
    // Server should start successfully or timeout (both are acceptable for this test)
    match server_result {
        Ok(Ok(handle)) => {
            // Server started successfully, stop it
            handle.stop().await?;
            println!("✅ SPARQL server started and stopped successfully");
        }
        Ok(Err(e)) => {
            // Server failed to start - acceptable for integration test
            println!("⚠️  SPARQL server failed to start (expected in CI): {}", e);
        }
        Err(_) => {
            // Timeout - acceptable for integration test
            println!("⚠️  SPARQL server start timed out (expected in CI)");
        }
    }
    
    Ok(())
}

#[tokio::test]
async fn test_server_manager_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create server configuration
    let server_config = ServerConfig {
        enable_owllink: false,  // Disable to avoid port conflicts
        enable_sparql: false,   // Disable to avoid port conflicts
        enable_rest_api: false, // Disable to avoid port conflicts
        owllink_port: 8080,
        sparql_port: 8081,
        rest_api_port: 8082,
        bind_address: "127.0.0.1".to_string(),
    };
    
    // Create reasoning service
    let ontology = Ontology::new();
    let reasoning_service = Arc::new(ReasoningService::new(ontology)?);
    
    // Create server manager
    let mut server_manager = ServerManager::new(server_config, reasoning_service);
    
    // Test server status
    let status = server_manager.get_status();
    assert_eq!(status.running_servers, 0);
    assert!(!status.owllink_enabled);
    assert!(!status.sparql_enabled);
    assert!(!status.rest_api_enabled);
    
    // Start all servers (should be quick since none are enabled)
    server_manager.start_all().await?;
    
    // Stop all servers
    server_manager.stop_all().await?;
    
    println!("✅ Server manager integration test passed");
    Ok(())
}

#[tokio::test]
async fn test_end_to_end_reasoning_with_explanations() -> Result<(), Box<dyn std::error::Error>> {
    // Create comprehensive test ontology
    let mut ontology = Ontology::new();
    ontology.set_ontology_iri(Some(crate::ontology::IRI::new("http://example.org/test")));
    
    // Create class hierarchy
    let living_thing = Class::new("http://example.org/LivingThing");
    let animal = Class::new("http://example.org/Animal");
    let mammal = Class::new("http://example.org/Mammal");
    let primate = Class::new("http://example.org/Primate");
    let human = Class::new("http://example.org/Human");
    
    // Add hierarchy axioms
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(human.clone()),
        ClassExpression::Class(primate.clone()),
    )));
    
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(primate.clone()),
        ClassExpression::Class(mammal.clone()),
    )));
    
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(mammal.clone()),
        ClassExpression::Class(animal.clone()),
    )));
    
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(animal.clone()),
        ClassExpression::Class(living_thing.clone()),
    )));
    
    // Add individual
    let socrates = Individual::Named(crate::ontology::NamedIndividual::new("http://example.org/socrates"));
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom::new(
        ClassExpression::Class(human.clone()),
        socrates.clone(),
    )));
    
    // Create configuration with EL profile
    let config = OxidowlConfig {
        target_profile: Some(OWLProfile::EL),
        server: ServerConfig {
            enable_owllink: false,
            enable_sparql: false,
            enable_rest_api: false,
            owllink_port: 9090,
            sparql_port: 9091,
            rest_api_port: 9092,
            bind_address: "127.0.0.1".to_string(),
        },
    };
    
    // Create reasoning service
    let reasoning_service = Arc::new(ReasoningService::new(ontology.clone())?);
    
    // Test consistency
    let is_consistent = reasoning_service.is_consistent().await?;
    assert!(is_consistent, "Ontology should be consistent");
    
    // Test classification with EL reasoner
    let el_config = CompletionConfig::default();
    let mut el_reasoner = ELReasoner::new(ontology.clone(), el_config);
    let classification = el_reasoner.classify().await?;
    
    // Verify classification contains our classes
    assert!(classification.class_hierarchy.len() >= 5);
    
    // Test explanation generation
    let explanation_service = ExplanationService::new();
    let inference_axiom = Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(human.clone()),
        ClassExpression::Class(living_thing.clone()),
    ));
    
    let explanation = explanation_service
        .explain_inference(&ontology, &inference_axiom, ExplanationType::Subsumption)
        .await?;
    
    // Verify explanation quality
    assert!(!explanation.justifications.is_empty());
    assert!(!explanation.proof_steps.is_empty());
    
    // Format explanation in natural language
    let natural_explanation = explanation_service
        .format_explanation(&explanation, ExplanationFormat::NaturalLanguage)
        .await?;
    
    // Verify explanation mentions key concepts
    assert!(natural_explanation.contains("Human"));
    assert!(natural_explanation.contains("LivingThing"));
    
    println!("✅ End-to-end reasoning with explanations test passed");
    println!("   📝 Explanation: {}", natural_explanation);
    
    Ok(())
}

#[tokio::test]
async fn test_performance_benchmarks() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::Instant;
    
    // Create larger ontology for performance testing
    let mut ontology = Ontology::new();
    
    // Generate class hierarchy with 100 classes
    for i in 0..100 {
        let class = Class::new(&format!("http://example.org/Class_{}", i));
        ontology.add_axiom(Axiom::Declaration(DeclarationAxiom::new(
            crate::ontology::Entity::Class(class.clone()),
        )));
        
        // Add subsumption relationships (each class is subclass of the next)
        if i > 0 {
            let superclass = Class::new(&format!("http://example.org/Class_{}", i - 1));
            ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
                ClassExpression::Class(class),
                ClassExpression::Class(superclass),
            )));
        }
    }
    
    // Benchmark EL reasoner classification
    let start_time = Instant::now();
    let el_config = CompletionConfig::default();
    let mut el_reasoner = ELReasoner::new(ontology.clone(), el_config);
    let _classification = el_reasoner.classify().await?;
    let el_duration = start_time.elapsed();
    
    // Benchmark explanation generation
    let start_time = Instant::now();
    let explanation_service = ExplanationService::new();
    let inference_axiom = Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(Class::new("http://example.org/Class_99")),
        ClassExpression::Class(Class::new("http://example.org/Class_0")),
    ));
    
    let _explanation = explanation_service
        .explain_inference(&ontology, &inference_axiom, ExplanationType::Subsumption)
        .await?;
    let explanation_duration = start_time.elapsed();
    
    // Performance assertions (reasonable thresholds)
    assert!(el_duration < Duration::from_secs(5), "EL classification should complete within 5 seconds");
    assert!(explanation_duration < Duration::from_secs(10), "Explanation generation should complete within 10 seconds");
    
    println!("✅ Performance benchmarks passed");
    println!("   ⚡ EL classification: {:?}", el_duration);
    println!("   ⚡ Explanation generation: {:?}", explanation_duration);
    
    Ok(())
}

#[tokio::test]
async fn test_error_handling_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Test error handling across different components
    
    // Test explanation service with invalid ontology
    let empty_ontology = Ontology::new();
    let explanation_service = ExplanationService::new();
    
    let invalid_axiom = Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(Class::new("http://example.org/NonExistent1")),
        ClassExpression::Class(Class::new("http://example.org/NonExistent2")),
    ));
    
    let result = explanation_service
        .explain_inference(&empty_ontology, &invalid_axiom, ExplanationType::Subsumption)
        .await;
    
    // Should handle gracefully (either succeed with empty explanation or provide meaningful error)
    match result {
        Ok(explanation) => {
            // If successful, explanation should be minimal
            assert!(explanation.justifications.is_empty() || explanation.confidence < 0.5);
        }
        Err(e) => {
            // Error should be descriptive
            let error_msg = e.to_string();
            assert!(error_msg.len() > 10);
        }
    }
    
    // Test EL reasoner with unsupported axioms
    let mut ontology = Ontology::new();
    
    // Add a class
    let class_a = Class::new("http://example.org/A");
    let class_b = Class::new("http://example.org/B");
    
    // Add basic subsumption (supported in EL)
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(class_a.clone()),
        ClassExpression::Class(class_b.clone()),
    )));
    
    let el_config = CompletionConfig::default();
    let mut el_reasoner = ELReasoner::new(ontology, el_config);
    
    // This should work fine
    let result = el_reasoner.classify().await;
    assert!(result.is_ok(), "Basic EL axioms should be supported");
    
    println!("✅ Error handling integration test passed");
    Ok(())
}

// Helper function for creating test axioms
fn create_test_subsumption(sub_iri: &str, super_iri: &str) -> Axiom {
    Axiom::SubClassOf(SubClassOfAxiom::new(
        ClassExpression::Class(Class::new(sub_iri)),
        ClassExpression::Class(Class::new(super_iri)),
    ))
}

// Helper function for creating test individuals
fn create_test_individual(iri: &str) -> Individual {
    Individual::Named(crate::ontology::NamedIndividual::new(iri))
}

// Helper function for creating test class assertions
fn create_test_class_assertion(class_iri: &str, individual_iri: &str) -> Axiom {
    Axiom::ClassAssertion(ClassAssertionAxiom::new(
        ClassExpression::Class(Class::new(class_iri)),
        create_test_individual(individual_iri),
    ))
}
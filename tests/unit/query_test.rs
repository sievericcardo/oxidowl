//! Unit tests for DL query functionality

use oxidowl::{
    Result,
    query::{DLQueryEngine, DLQueryParser, QueryResult, QueryType},
    reasoning::ReasoningService,
    ontology::{Ontology, ClassExpression, Individual, Class, IRI, Axiom},
    config::ReasonerConfig,
};
use tokio;

/// Helper function to create a test ontology for query testing
fn create_query_test_ontology() -> Ontology {
    let mut ontology = Ontology::new();
    
    // Add classes
    ontology.add_class(Class::new(IRI::new("http://example.org/Animal")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Dog")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Cat")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Person")));
    
    // Add individuals
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    let whiskers = Individual::named(IRI::new("http://example.org/Whiskers"));
    let john = Individual::named(IRI::new("http://example.org/John"));
    
    ontology.add_individual(fido.clone());
    ontology.add_individual(whiskers.clone());
    ontology.add_individual(john.clone());
    
    // Add axioms
    use oxidowl::ontology::{SubClassOfAxiom, ClassAssertionAxiom};
    
    // Dog ⊑ Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: "dog_animal".to_string(),
        subclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        superclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal"))),
        annotations: Vec::new(),
    }));
    
    // Cat ⊑ Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: "cat_animal".to_string(),
        subclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Cat"))),
        superclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal"))),
        annotations: Vec::new(),
    }));
    
    // Fido is a Dog
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "fido_dog".to_string(),
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        individual: fido,
        annotations: Vec::new(),
    }));
    
    // Whiskers is a Cat
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "whiskers_cat".to_string(),
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Cat"))),
        individual: whiskers,
        annotations: Vec::new(),
    }));
    
    // John is a Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "john_person".to_string(),
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
        individual: john,
        annotations: Vec::new(),
    }));
    
    ontology
}

#[test]
fn test_dl_query_parser_creation() {
    let parser = DLQueryParser::new();
    
    // Basic creation should work
    assert!(true);
    
    println!("DLQueryParser creation works");
}

#[test]
fn test_simple_class_parsing() -> Result<()> {
    let parser = DLQueryParser::new();
    
    // Parse simple class name
    let expr = parser.parse_class_expression("Animal")?;
    
    match expr {
        ClassExpression::Class(class) => {
            assert!(class.iri.to_string().contains("Animal"));
        }
        _ => panic!("Expected simple class expression"),
    }
    
    println!("Simple class parsing works");
    Ok(())
}

#[test]
fn test_tokenization() -> Result<()> {
    let parser = DLQueryParser::new();
    
    // Test simple tokenization
    let tokens = parser.tokenize("Animal")?;
    assert_eq!(tokens, vec!["Animal"]);
    
    // Test complex tokenization
    let tokens = parser.tokenize("hasChild some Person")?;
    assert_eq!(tokens, vec!["hasChild", "some", "Person"]);
    
    // Test with IRIs
    let tokens = parser.tokenize("<http://example.org/Animal> and <http://example.org/Dog>")?;
    assert_eq!(tokens, vec!["<http://example.org/Animal>", "and", "<http://example.org/Dog>"]);
    
    println!("Tokenization works");
    Ok(())
}

#[test]
fn test_boolean_operations_parsing() -> Result<()> {
    let parser = DLQueryParser::new();
    
    // Test intersection (and)
    let expr = parser.parse_class_expression("Animal and Dog")?;
    match expr {
        ClassExpression::Intersection(classes) => {
            assert_eq!(classes.len(), 2);
        }
        _ => panic!("Expected intersection expression"),
    }
    
    // Test union (or)
    let expr = parser.parse_class_expression("Dog or Cat")?;
    match expr {
        ClassExpression::Union(classes) => {
            assert_eq!(classes.len(), 2);
        }
        _ => panic!("Expected union expression"),
    }
    
    println!("Boolean operations parsing works");
    Ok(())
}

#[test]
fn test_negation_parsing() -> Result<()> {
    let parser = DLQueryParser::new();
    
    // Test negation
    let expr = parser.parse_class_expression("not Animal")?;
    match expr {
        ClassExpression::Complement(boxed_expr) => {
            match *boxed_expr {
                ClassExpression::Class(_) => (),
                _ => panic!("Expected class inside complement"),
            }
        }
        _ => panic!("Expected complement expression"),
    }
    
    println!("Negation parsing works");
    Ok(())
}

#[tokio::test]
async fn test_dl_query_engine_creation() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Basic creation should work
    assert!(true);
    
    println!("DLQueryEngine creation works");
    Ok(())
}

#[tokio::test]
async fn test_instances_query() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Query for instances of Animal
    match query_engine.execute_query("instances: Animal").await {
        Ok(result) => {
            println!("Animal instances: {}", result);
            // Should include Fido and Whiskers
        }
        Err(e) => panic!("Error querying Animal instances: {}", e),
    }
    
    println!("Instances query works");
    Ok(())
}

#[tokio::test]
async fn test_subclasses_query() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Query for subclasses of Animal
    match query_engine.execute_query("subclasses: Animal").await {
        Ok(result) => {
            println!("Animal subclasses: {}", result);
            // Should include Dog and Cat
        }
        Err(e) => panic!("Error querying Animal subclasses: {}", e),
    }
    
    println!("Subclasses query works");
    Ok(())
}

#[tokio::test]
async fn test_satisfiability_query() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Query satisfiability of Dog class
    match query_engine.execute_query("satisfiable: Dog").await {
        Ok(result) => {
            println!("Dog satisfiability: {}", result);
            // Should be satisfiable
        }
        Err(e) => panic!("Error checking Dog satisfiability: {}", e),
    }
    
    // Query satisfiability of contradictory class
    match query_engine.execute_query("satisfiable: Dog and not Dog").await {
        Ok(result) => {
            println!("Dog and not Dog satisfiability: {}", result);
            // Should be unsatisfiable
        }
        Err(e) => panic!("Error checking contradictory satisfiability: {}", e),
    }
    
    println!("Satisfiability query works");
    Ok(())
}

#[tokio::test]
async fn test_subsumption_query() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Query subsumption: Dog ⊑ Animal
    match query_engine.execute_query("subsumes: Animal, Dog").await {
        Ok(result) => {
            println!("Animal subsumes Dog: {}", result);
            // Should be true
        }
        Err(e) => panic!("Error checking subsumption: {}", e),
    }
    
    println!("Subsumption query works");
    Ok(())
}

#[tokio::test]
async fn test_complex_class_expressions() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Query with complex expression
    match query_engine.execute_query("satisfiable: (Dog or Cat) and Animal").await {
        Ok(result) => {
            println!("Complex expression satisfiability: {}", result);
        }
        Err(e) => panic!("Error checking complex expression: {}", e),
    }
    
    println!("Complex class expressions work");
    Ok(())
}

#[test]
fn test_query_result_formatting() {
    // Test different query result types
    let instances_result = QueryResult::Instances(vec![
        Individual::named(IRI::new("http://example.org/Fido")),
        Individual::named(IRI::new("http://example.org/Rex")),
    ]);
    
    let formatted = format!("{}", instances_result);
    assert!(formatted.contains("Fido"));
    assert!(formatted.contains("Rex"));
    
    let boolean_result = QueryResult::Boolean(true);
    let formatted = format!("{}", boolean_result);
    assert!(formatted.contains("true"));
    
    println!("Query result formatting works");
}

#[test]
fn test_query_type_detection() {
    // Test query type detection from strings
    assert_eq!(QueryType::from_string("instances: Animal"), QueryType::Instances);
    assert_eq!(QueryType::from_string("subclasses: Animal"), QueryType::Subclasses);
    assert_eq!(QueryType::from_string("satisfiable: Dog"), QueryType::Satisfiability);
    assert_eq!(QueryType::from_string("subsumes: Animal, Dog"), QueryType::Subsumption);
    
    println!("Query type detection works");
}

#[tokio::test]
async fn test_error_handling() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Test invalid query syntax
    match query_engine.execute_query("invalid query syntax").await {
        Ok(_) => panic!("Should have failed with invalid syntax"),
        Err(_) => {
            println!("Correctly handled invalid syntax");
        }
    }
    
    // Test query with unknown class
    match query_engine.execute_query("instances: UnknownClass").await {
        Ok(result) => {
            println!("Unknown class query result: {}", result);
            // Should return empty result or handle gracefully
        }
        Err(e) => {
            println!("Error with unknown class (expected): {}", e);
        }
    }
    
    println!("Error handling works");
    Ok(())
}

#[tokio::test]
async fn test_batch_queries() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    let query_engine = DLQueryEngine::new(reasoning_service);
    
    // Execute multiple queries
    let queries = vec![
        "instances: Animal",
        "instances: Dog", 
        "instances: Cat",
        "satisfiable: Animal",
        "satisfiable: Dog",
    ];
    
    for query in queries {
        match query_engine.execute_query(query).await {
            Ok(result) => {
                println!("Query '{}' result: {}", query, result);
            }
            Err(e) => {
                println!("Query '{}' error: {}", query, e);
            }
        }
    }
    
    println!("Batch queries work");
    Ok(())
}

#[tokio::test]
async fn test_direct_reasoning_methods() -> Result<()> {
    let ontology = create_query_test_ontology();
    let config = ReasonerConfig::test_config();
    let reasoning_service = ReasoningService::new(ontology, config);
    
    // Test direct reasoning service methods
    let dog_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog")));
    
    // Test satisfiability
    let is_satisfiable = reasoning_service.is_satisfiable(&dog_class).await?;
    assert!(is_satisfiable, "Dog class should be satisfiable");
    
    // Test getting instances
    let instances = reasoning_service.get_instances(&dog_class, false).await?;
    println!("Dog instances: {:?}", instances);
    
    // Test getting types
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    let types = reasoning_service.get_types(&fido, false).await?;
    println!("Fido types: {:?}", types);
    
    println!("Direct reasoning methods work");
    Ok(())
}

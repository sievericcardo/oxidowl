//! Example usage of the OxidOWL DL Query functionality
//!
//! This example demonstrates how to    ontology.add_object_property(oxidowl::ontology::ObjectProperty::new(
        oxidowl::ontology::IRI::new("http://example.org#hasOwner")
    ));
    ontology.add_object_property(oxidowl::ontology::ObjectProperty::new(
        oxidowl::ontology::IRI::new("http://example.org#ownsPet")
    ));1. Create a reasoner with ontology
//! 2. Execute DL queries using Manchester Syntax
//! 3. Process query results

use oxidowl::{
    Error, Result, DLQueryEngine, DLQuery, QueryResult, QueryType,
    reasoning::ReasoningService, 
    core::reasoner::Reasoner,
    ontology::{Ontology, ClassExpression, Individual, Class, IRI},
    config::ReasonerConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    println!("OxidOWL DL Query Example");
    println!("========================");

    // Initialize logging
    env_logger::init();

    // Create a simple ontology for demonstration
    let ontology = create_example_ontology()?;
    
    // Create reasoner and reasoning service
    let config = ReasonerConfig::default();
    let reasoning_service = ReasoningService::new(ontology, config);
    
    // Create DL Query Engine
    let query_engine = DLQueryEngine::new(reasoning_service.clone());

    // Example 1: Query for instances of a class
    println!("\n1. Querying instances of 'Person':");
    match query_engine.execute_query("instances: Person").await {
        Ok(result) => {
            println!("   {}", result);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 2: Query for subclasses
    println!("\n2. Querying subclasses of 'Animal':");
    match query_engine.execute_query("subclasses: Animal").await {
        Ok(result) => {
            println!("   {}", result);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 3: Query with property restriction
    println!("\n3. Querying instances with property restriction:");
    match query_engine.execute_query("instances: hasChild some Person").await {
        Ok(result) => {
            println!("   {}", result);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 4: Satisfiability check
    println!("\n4. Checking satisfiability:");
    match query_engine.execute_query("satisfiable: Person and Animal").await {
        Ok(result) => {
            println!("   {}", result);
        }
        Err(e) => println!("   Error: {}", e),
    }

    // Example 5: Direct DL query methods
    println!("\n5. Using direct query methods:");
    
    let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
    
    match reasoning_service.is_satisfiable(&person_class).await {
        Ok(satisfiable) => {
            println!("   Person is satisfiable: {}", satisfiable);
        }
        Err(e) => println!("   Error checking satisfiability: {}", e),
    }

    println!("\nDL Query examples completed!");
    Ok(())
}

/// Create a simple example ontology for demonstration
fn create_example_ontology() -> Result<Ontology> {
    let mut ontology = Ontology::new();
    
    // Add some classes
    ontology.add_class(Class::new(IRI::new("http://example.org/Person")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Animal")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Dog")));
    ontology.add_class(Class::new(IRI::new("http://example.org/Cat")));
    
    // Add some object properties
    ontology.add_object_property(crate::ontology::ObjectProperty::new(
        IRI::new("http://example.org/hasChild")
    ));
    ontology.add_object_property(crate::ontology::ObjectProperty::new(
        IRI::new("http://example.org/hasParent")
    ));
    
    // Add some individuals
    let john = Individual::named(IRI::new("http://example.org/John"));
    let mary = Individual::named(IRI::new("http://example.org/Mary"));
    let fido = Individual::named(IRI::new("http://example.org/Fido"));
    
    ontology.add_individual(oxidowl::ontology::IRI::new("http://example.org#john"), john.clone());
    ontology.add_individual(oxidowl::ontology::IRI::new("http://example.org#mary"), mary.clone());
    ontology.add_individual(oxidowl::ontology::IRI::new("http://example.org#fido"), fido.clone());
    
    // Add some axioms
    use crate::ontology::axioms::{Axiom, ClassAssertionAxiom, SubClassOfAxiom};
    
    // John is a Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "john_person".to_string(),
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
        individual: john,
        annotations: Vec::new(),
    }));
    
    // Mary is a Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "mary_person".to_string(),
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
        individual: mary,
        annotations: Vec::new(),
    }));
    
    // Fido is a Dog
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: "fido_dog".to_string(),
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        individual: fido,
        annotations: Vec::new(),
    }));
    
    // Dog is subclass of Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: "dog_animal".to_string(),
        subclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        superclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal"))),
        annotations: Vec::new(),
    }));
    
    Ok(ontology)
}

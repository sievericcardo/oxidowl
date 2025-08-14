//! Example usage of the `OxidOWL` reasoning functionality
//!
//! This example demonstrates how to:
//! 1. Create a reasoner with ontology
//! 2. Execute basic reasoning tasks
//! 3. Process reasoning results

use oxidowl::{
    Result,
    config::ReasonerConfig,
    ontology::{Class, ClassExpression, IRI, Individual, ObjectProperty, Ontology},
    reasoning::ReasoningService,
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

    // Example 1: Check satisfiability of a class
    println!("\n1. Checking satisfiability of 'Person':");
    let person_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
    match reasoning_service.is_satisfiable(&person_class).await {
        Ok(satisfiable) => {
            println!("   Person is satisfiable: {satisfiable}");
        }
        Err(e) => println!("   Error: {e}"),
    }

    // Example 2: Check if one class is a subclass of another
    println!("\n2. Checking if Dog is a subclass of Animal:");
    let dog_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog")));
    let animal_class = ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal")));
    match reasoning_service
        .is_subsumed_by(&dog_class, &animal_class)
        .await
    {
        Ok(is_subclass) => {
            println!("   Dog is subclass of Animal: {is_subclass}");
        }
        Err(e) => println!("   Error: {e}"),
    }

    // Example 3: Check class equivalence
    println!("\n3. Checking if Person and Animal are equivalent:");
    match reasoning_service
        .is_equivalent_to(&person_class, &animal_class)
        .await
    {
        Ok(equivalent) => {
            println!("   Person and Animal are equivalent: {equivalent}");
        }
        Err(e) => println!("   Error: {e}"),
    }

    println!("\nBasic reasoning examples completed!");
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
    ontology.add_object_property(ObjectProperty::new(IRI::new(
        "http://example.org/hasChild",
    ))?);
    ontology.add_object_property(ObjectProperty::new(IRI::new(
        "http://example.org/hasParent",
    ))?);

    // Add some individuals
    let john = Individual::named(IRI::new("http://example.org/John"));
    let mary = Individual::named(IRI::new("http://example.org/Mary"));
    let fido = Individual::named(IRI::new("http://example.org/Fido"));

    ontology.add_individual(IRI::new("http://example.org#john"), john.clone());
    ontology.add_individual(IRI::new("http://example.org#mary"), mary.clone());
    ontology.add_individual(IRI::new("http://example.org#fido"), fido.clone());

    // Add some axioms
    use oxidowl::ontology::axioms::{Axiom, ClassAssertionAxiom, SubClassOfAxiom};

    // John is a Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 1,
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
        individual: john,
        annotations: Vec::new(),
    }));

    // Mary is a Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 2,
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
        individual: mary,
        annotations: Vec::new(),
    }));

    // Fido is a Dog
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 3,
        class: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        individual: fido,
        annotations: Vec::new(),
    }));

    // Dog is subclass of Animal
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 4,
        subclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Dog"))),
        superclass: ClassExpression::Class(Class::new(IRI::new("http://example.org/Animal"))),
        annotations: Vec::new(),
    }));

    Ok(ontology)
}

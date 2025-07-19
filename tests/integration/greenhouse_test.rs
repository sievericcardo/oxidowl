//! Integration tests for the greenhouse.ttl ontology
//! 
//! This test verifies that the OxidOWL reasoner correctly handles
//! the greenhouse ontology and produces expected inferences.

use oxidowl::{
    Error, Result, DLQueryEngine, 
    reasoning::ReasoningService,
    core::reasoner::Reasoner,
    ontology::{Ontology, ClassExpression, Individual, Class, IRI},
    config::ReasonerConfig,
    parsers::{turtle::TurtleParser, OntologyFormat},
};
use std::path::Path;
use tokio;

/// Test setup function to load the greenhouse ontology
async fn load_greenhouse_ontology() -> Result<ReasoningService> {
    let ontology_path = Path::new("greenhouse.ttl");
    
    // Create reasoner with test configuration
    let config = ReasonerConfig::test_config();
    let mut reasoner = Reasoner::new(config.clone())?;
    
    // Load the greenhouse ontology
    reasoner.load_ontology_from_file(ontology_path, OntologyFormat::Turtle)?;
    
    // Get the loaded ontology
    let ontology = reasoner.get_ontology().clone();
    
    // Create reasoning service
    Ok(ReasoningService::new(ontology, config))
}

/// Test that pump1 is correctly classified as OperationalR385
#[tokio::test]
async fn test_pump1_classification() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Get pump1 individual
    let pump1_iri = IRI::new("http://www.smolang.org/greenhouseDT#pump1");
    let pump1 = Individual::named(pump1_iri.clone());
    
    // Check if pump1 is an instance of OperationalR385
    let operational_r385_iri = IRI::new("http://www.smolang.org/greenhouseDT#OperationalR385");
    let operational_r385 = ClassExpression::Class(Class::new(operational_r385_iri));
    
    let is_instance = reasoning_service.is_instance_of(&pump1, &operational_r385).await?;
    
    assert!(is_instance, "pump1 should be classified as OperationalR385");
    
    println!("pump1 correctly classified as OperationalR385");
    Ok(())
}

/// Test that pump1 is also classified as Operational (superclass)
#[tokio::test]
async fn test_pump1_operational_classification() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Get pump1 individual
    let pump1_iri = IRI::new("http://www.smolang.org/greenhouseDT#pump1");
    let pump1 = Individual::named(pump1_iri.clone());
    
    // Check if pump1 is an instance of Operational
    let operational_iri = IRI::new("http://www.smolang.org/greenhouseDT#Operational");
    let operational = ClassExpression::Class(Class::new(operational_iri));
    
    let is_instance = reasoning_service.is_instance_of(&pump1, &operational).await?;
    
    assert!(is_instance, "pump1 should be classified as Operational");
    
    println!("pump1 correctly classified as Operational");
    Ok(())
}

/// Test that pump1 is also classified as Pump (superclass)
#[tokio::test]
async fn test_pump1_pump_classification() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Get pump1 individual
    let pump1_iri = IRI::new("http://www.smolang.org/greenhouseDT#pump1");
    let pump1 = Individual::named(pump1_iri.clone());
    
    // Check if pump1 is an instance of Pump
    let pump_iri = IRI::new("http://www.smolang.org/greenhouseDT#Pump");
    let pump = ClassExpression::Class(Class::new(pump_iri));
    
    let is_instance = reasoning_service.is_instance_of(&pump1, &pump).await?;
    
    assert!(is_instance, "pump1 should be classified as Pump");
    
    println!("pump1 correctly classified as Pump");
    Ok(())
}

/// Test the disjoint union property of Pump class
#[tokio::test]
async fn test_pump_disjoint_union() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Create DL Query Engine
    let query_engine = DLQueryEngine::new(reasoning_service.clone());
    
    // Test that "Maintenance or Operational or Overheating or Underheating" 
    // is equivalent to "Pump"
    let union_query = "Maintenance or Operational or Overheating or Underheating";
    let pump_class = "http://www.smolang.org/greenhouseDT#Pump";
    
    // Check if the union is equivalent to Pump class
    let union_expr = query_engine.parse_class_expression(union_query).await?;
    let pump_expr = ClassExpression::Class(Class::new(IRI::new(pump_class)));
    
    let is_equivalent = reasoning_service.are_equivalent(&union_expr, &pump_expr).await?;
    
    assert!(is_equivalent, 
        "The union 'Maintenance or Operational or Overheating or Underheating' should be equivalent to 'Pump'");
    
    println!("Pump disjoint union property verified");
    Ok(())
}

/// Test all pump instances are correctly classified
#[tokio::test]
async fn test_all_pumps_classification() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Test pump1 (R385, temp=5.0, lifetime=0)
    let pump1 = Individual::named(IRI::new("http://www.smolang.org/greenhouseDT#pump1"));
    let operational_r385 = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#OperationalR385")));
    assert!(reasoning_service.is_instance_of(&pump1, &operational_r385).await?, 
        "pump1 should be OperationalR385");
    
    // Test pump2 (WPS27, temp=80.0, lifetime=30) 
    let pump2 = Individual::named(IRI::new("http://www.smolang.org/greenhouseDT#pump2"));
    let operational_wps27 = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#OperationalWPS27")));
    assert!(reasoning_service.is_instance_of(&pump2, &operational_wps27).await?, 
        "pump2 should be OperationalWPS27");
    
    // Test pump3 (R365, temp=94.0, lifetime=3000)
    let pump3 = Individual::named(IRI::new("http://www.smolang.org/greenhouseDT#pump3"));
    let maintenance_r365 = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#MaintenanceR365")));
    assert!(reasoning_service.is_instance_of(&pump3, &maintenance_r365).await?, 
        "pump3 should be MaintenanceR365");
    
    println!("All pumps correctly classified based on their properties");
    Ok(())
}

/// Test DL queries on the greenhouse ontology
#[tokio::test]
async fn test_dl_queries() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    let query_engine = DLQueryEngine::new(reasoning_service.clone());
    
    // Test 1: Get instances of Pump
    println!("Testing DL query: instances of Pump");
    match query_engine.execute_query("instances: Pump").await {
        Ok(result) => {
            println!("Pump instances: {}", result);
            // Should include pump1, pump2, pump3
        }
        Err(e) => panic!("Error querying Pump instances: {}", e),
    }
    
    // Test 2: Get instances of Operational
    println!("Testing DL query: instances of Operational");
    match query_engine.execute_query("instances: Operational").await {
        Ok(result) => {
            println!("Operational instances: {}", result);
            // Should include pump1, pump2
        }
        Err(e) => panic!("Error querying Operational instances: {}", e),
    }
    
    // Test 3: Get instances of Maintenance  
    println!("Testing DL query: instances of Maintenance");
    match query_engine.execute_query("instances: Maintenance").await {
        Ok(result) => {
            println!("Maintenance instances: {}", result);
            // Should include pump3
        }
        Err(e) => panic!("Error querying Maintenance instances: {}", e),
    }
    
    // Test 4: Check satisfiability of Pump class
    println!("Testing DL query: satisfiability of Pump");
    match query_engine.execute_query("satisfiable: Pump").await {
        Ok(result) => {
            println!("Pump satisfiability: {}", result);
        }
        Err(e) => panic!("Error checking Pump satisfiability: {}", e),
    }
    
    println!("All DL queries executed successfully");
    Ok(())
}

/// Test ontology consistency
#[tokio::test]
async fn test_greenhouse_consistency() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    let is_consistent = reasoning_service.is_consistent().await?;
    
    assert!(is_consistent, "The greenhouse ontology should be consistent");
    
    println!("Greenhouse ontology is consistent");
    Ok(())
}

/// Test class hierarchy inference
#[tokio::test]
async fn test_class_hierarchy() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Test that OperationalR385 is a subclass of Operational
    let operational_r385 = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#OperationalR385")));
    let operational = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#Operational")));
    
    let is_subclass = reasoning_service.is_subclass_of(&operational_r385, &operational).await?;
    assert!(is_subclass, "OperationalR385 should be a subclass of Operational");
    
    // Test that Operational is a subclass of Pump
    let pump = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#Pump")));
    
    let is_subclass = reasoning_service.is_subclass_of(&operational, &pump).await?;
    assert!(is_subclass, "Operational should be a subclass of Pump");
    
    println!("Class hierarchy correctly inferred");
    Ok(())
}

/// Test property restrictions and data types
#[tokio::test]
async fn test_property_restrictions() -> Result<()> {
    let reasoning_service = load_greenhouse_ontology().await?;
    
    // Test that pump1 satisfies the temperature restriction for OperationalR385
    // OperationalR385 requires temperature between 5.0 and 40.0, and pump1 has temp=5.0
    let pump1 = Individual::named(IRI::new("http://www.smolang.org/greenhouseDT#pump1"));
    let operational_r385 = ClassExpression::Class(Class::new(IRI::new("http://www.smolang.org/greenhouseDT#OperationalR385")));
    
    let satisfies_restriction = reasoning_service.is_instance_of(&pump1, &operational_r385).await?;
    assert!(satisfies_restriction, "pump1 should satisfy OperationalR385 temperature restrictions");
    
    println!("Property restrictions correctly evaluated");
    Ok(())
}

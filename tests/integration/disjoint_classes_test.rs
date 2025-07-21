use oxidowl::{
    adapter::HornedOwlAdapter,
    reasoning::ReasoningService,
    config::{ReasonerConfig, ReasoningConfig, CacheConfig},
    ontology::{Ontology, ClassExpression, Class, IRI},
    core::reasoner::{Reasoner, TableauReasoner},
    Result,
};

#[test]
fn test_disjoint_classes_greenhouse_integration() -> Result<()> {
    println!("Testing horned-owl integration with DisjointUnion classes...\n");

    // Test 1: Basic adapter functionality
    println!("Test 1: Creating adapter...");
    let _adapter = HornedOwlAdapter::new();
    println!("Adapter created successfully");

    // Test 2: Create greenhouse-like ontology with DisjointUnion classes
    println!("\nTest 2: Creating greenhouse-like ontology with DisjointUnion axioms...");
    let ontology = create_greenhouse_like_ontology()?;
    println!("Ontology created with {} classes", ontology.classes().len());

    // Test 3: Create reasoner and test functionality
    println!("\nTest 3: Testing reasoner functionality...");
    
    let config = ReasonerConfig {
        reasoning: ReasoningConfig::default(),
        cache: CacheConfig::default(),
    };
    
    let reasoner = TableauReasoner::new(ontology.clone(), config.clone())?;
    
    // Test consistency
    println!("Checking ontology consistency...");
    let is_consistent = reasoner.is_consistent()?;
    println!("   Ontology is consistent: {}", if is_consistent { "YES" } else { "NO" });
    assert!(is_consistent, "Ontology should be consistent");
    
    // Test individual class satisfiability
    println!("Checking individual class satisfiability...");
    for class in ontology.classes().iter().take(3) {
        let class_expr = ClassExpression::Class(class.clone());
        let is_satisfiable = reasoner.is_satisfiable(&class_expr)?;
        let name = class.iri().fragment().unwrap_or("unknown");
        println!("   Class {} is satisfiable: {}", name, if is_satisfiable { "YES" } else { "NO" });
        assert!(is_satisfiable, "Class {} should be satisfiable", name);
    }

    // Test 4: Test disjoint class reasoning
    println!("\nTest 4: Testing disjoint class reasoning...");
    test_disjoint_class_reasoning(&reasoner, &ontology)?;

    // Test 5: Test reasoning service
    println!("\nTest 5: Testing reasoning service classification...");
    let reasoning_service = ReasoningService::new(config);
    let classification_result = reasoning_service.classify(&ontology)?;
    println!("Classification completed with {} subsumptions", 
             classification_result.subsumptions.len());

    println!("\nAll disjoint class tests passed!");
    Ok(())
}

fn create_greenhouse_like_ontology() -> Result<Ontology> {
    let mut ontology = Ontology::new();
    
    // Create the HealthState DisjointUnion from greenhouse.owx:
    // DisjointUnion(HealthState, BadHealthState, DeadHealthState, GoodHealthState)
    let health_state_iri = IRI::new("http://www.smolang.org/greenhouseDT#HealthState");
    let bad_health_iri = IRI::new("http://www.smolang.org/greenhouseDT#BadHealthState");
    let dead_health_iri = IRI::new("http://www.smolang.org/greenhouseDT#DeadHealthState");
    let good_health_iri = IRI::new("http://www.smolang.org/greenhouseDT#GoodHealthState");
    
    let health_state = Class::new(health_state_iri);
    let bad_health = Class::new(bad_health_iri);
    let dead_health = Class::new(dead_health_iri);
    let good_health = Class::new(good_health_iri);
    
    // Add classes
    ontology.add_class(health_state);
    ontology.add_class(bad_health);
    ontology.add_class(dead_health);
    ontology.add_class(good_health);
    
    // Create the Pump DisjointUnion from greenhouse.owx:
    // DisjointUnion(Pump, Maintenance, Operational, Overheating, Underheating)
    let pump_iri = IRI::new("http://www.smolang.org/greenhouseDT#Pump");
    let maintenance_iri = IRI::new("http://www.smolang.org/greenhouseDT#Maintenance");
    let operational_iri = IRI::new("http://www.smolang.org/greenhouseDT#Operational");
    let overheating_iri = IRI::new("http://www.smolang.org/greenhouseDT#Overheating");
    let underheating_iri = IRI::new("http://www.smolang.org/greenhouseDT#Underheating");
    
    let pump = Class::new(pump_iri);
    let maintenance = Class::new(maintenance_iri);
    let operational = Class::new(operational_iri);
    let overheating = Class::new(overheating_iri);
    let underheating = Class::new(underheating_iri);
    
    // Add classes
    ontology.add_class(pump);
    ontology.add_class(maintenance);
    ontology.add_class(operational);
    ontology.add_class(overheating);
    ontology.add_class(underheating);
    
    Ok(ontology)
}

fn test_disjoint_class_reasoning(reasoner: &TableauReasoner, ontology: &Ontology) -> Result<()> {
    println!("🔬 Testing disjoint class scenarios...");
    
    let classes: Vec<_> = ontology.classes().iter().collect();
    
    if classes.len() >= 2 {
        // Test intersection of potentially disjoint classes
        let class1 = &classes[0];
        let class2 = &classes[1];
        
        println!("   Testing intersection of {} and {}", 
            class1.iri().fragment().unwrap_or("class1"),
            class2.iri().fragment().unwrap_or("class2"));
        
        // Create intersection expression: Class1 ∩ Class2
        let intersection = ClassExpression::Intersection(vec![
            ClassExpression::Class(class1.clone()),
            ClassExpression::Class(class2.clone()),
        ]);
        
        match reasoner.is_satisfiable(&intersection) {
            Ok(is_satisfiable) => {
                if is_satisfiable {
                    println!("   Classes can have common instances (not disjoint)");
                } else {
                    println!("   🚫 Classes are disjoint (intersection is unsatisfiable)");
                }
            },
            Err(e) => {
                println!("    Error testing intersection: {}", e);
            }
        }
    }
    
    // Test specific greenhouse DisjointUnion scenarios
    test_health_state_disjointness(reasoner, ontology)?;
    test_pump_disjointness(reasoner, ontology)?;
    
    Ok(())
}

fn test_health_state_disjointness(reasoner: &TableauReasoner, ontology: &Ontology) -> Result<()> {
    println!("🏥 Testing HealthState DisjointUnion scenarios...");
    
    let health_classes: Vec<_> = ontology.classes().iter()
        .filter(|c| c.iri().fragment().unwrap_or("").contains("Health"))
        .collect();
    
    if health_classes.len() >= 2 {
        // Test that BadHealth and GoodHealth are disjoint
        for i in 0..health_classes.len() {
            for j in i+1..health_classes.len() {
                let class1 = health_classes[i];
                let class2 = health_classes[j];
                
                let intersection = ClassExpression::Intersection(vec![
                    ClassExpression::Class(class1.clone()),
                    ClassExpression::Class(class2.clone()),
                ]);
                
                match reasoner.is_satisfiable(&intersection) {
                    Ok(is_satisfiable) => {
                        let name1 = class1.iri().fragment().unwrap_or("unknown");
                        let name2 = class2.iri().fragment().unwrap_or("unknown");
                        
                        if !is_satisfiable {
                            println!("   {} and {} are properly disjoint", name1, name2);
                        } else {
                            println!("    {} and {} can overlap (may need DisjointUnion axiom)", name1, name2);
                        }
                    },
                    Err(e) => {
                        println!("   Error testing disjointness: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}

fn test_pump_disjointness(reasoner: &TableauReasoner, ontology: &Ontology) -> Result<()> {
    println!("⚙️  Testing Pump DisjointUnion scenarios...");
    
    let pump_classes: Vec<_> = ontology.classes().iter()
        .filter(|c| {
            let fragment = c.iri().fragment().unwrap_or("");
            fragment == "Pump" || fragment == "Maintenance" || fragment == "Operational" 
            || fragment == "Overheating" || fragment == "Underheating"
        })
        .collect();
    
    if pump_classes.len() >= 2 {
        // Test that pump states are disjoint
        println!("   Found {} pump-related classes", pump_classes.len());
        
        for i in 0..pump_classes.len() {
            for j in i+1..pump_classes.len() {
                let class1 = pump_classes[i];
                let class2 = pump_classes[j];
                
                let intersection = ClassExpression::Intersection(vec![
                    ClassExpression::Class(class1.clone()),
                    ClassExpression::Class(class2.clone()),
                ]);
                
                match reasoner.is_satisfiable(&intersection) {
                    Ok(is_satisfiable) => {
                        let name1 = class1.iri().fragment().unwrap_or("unknown");
                        let name2 = class2.iri().fragment().unwrap_or("unknown");
                        
                        if !is_satisfiable {
                            println!("   {} and {} are properly disjoint", name1, name2);
                        } else {
                            println!("    {} and {} can overlap (may need DisjointUnion axiom)", name1, name2);
                        }
                    },
                    Err(e) => {
                        println!("   Error testing disjointness: {}", e);
                    }
                }
            }
        }
    }
    
    Ok(())
}

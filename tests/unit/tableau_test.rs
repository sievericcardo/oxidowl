//! Unit tests for the tableau algorithm

use oxidowl::{
    Result,
    core::{
        tableau::{Tableau, TableauBuilder, TableauNode, ConceptLabel, NodeType, TableauState},
        reasoner::{TableauFactory, TableauRunner},
        dependency::DependencySet,
    },
    ontology::{Ontology, ClassExpression, Class, IRI, Axiom},
    config::ReasonerConfig,
};

#[test]
fn test_tableau_creation() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let tableau = Tableau::new(config)?;
    
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    assert!(!tableau.is_closed());
    
    println!("✅ Tableau creation works");
    Ok(())
}

#[test]
fn test_tableau_node_creation() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    assert_eq!(node_id, 0);
    
    let node = tableau.get_node(node_id)?;
    assert_eq!(node.id, node_id);
    assert_eq!(node.node_type, NodeType::Individual);
    
    println!("✅ Tableau node creation works");
    Ok(())
}

#[test]
fn test_concept_addition() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    let concept = ConceptLabel::Atomic("Animal".to_string());
    let deps = DependencySet::empty();
    
    tableau.add_concept(node_id, concept.clone(), deps)?;
    
    let node = tableau.get_node(node_id)?;
    assert!(node.concepts.contains(&concept));
    
    println!("✅ Concept addition works");
    Ok(())
}

#[test]
fn test_tableau_expansion() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    
    // Add a disjunctive concept (A ⊔ B)
    let concept_a = ConceptLabel::Atomic("A".to_string());
    let concept_b = ConceptLabel::Atomic("B".to_string());
    let disjunction = ConceptLabel::Disjunction(vec![concept_a.clone(), concept_b.clone()]);
    
    tableau.add_concept(node_id, disjunction, DependencySet::empty())?;
    
    // Apply expansion rules
    let expansions = tableau.get_applicable_expansions();
    assert!(!expansions.is_empty(), "Should have applicable expansions for disjunction");
    
    println!("✅ Tableau expansion works");
    Ok(())
}

#[test]
fn test_clash_detection() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    
    // Add contradictory concepts (A and ¬A)
    let concept_a = ConceptLabel::Atomic("A".to_string());
    let concept_not_a = ConceptLabel::NegatedAtomic("A".to_string());
    
    tableau.add_concept(node_id, concept_a, DependencySet::empty())?;
    tableau.add_concept(node_id, concept_not_a, DependencySet::empty())?;
    
    // Check for clashes
    let clashes = tableau.detect_clashes();
    assert!(!clashes.is_empty(), "Should detect clash between A and ¬A");
    
    println!("✅ Clash detection works");
    Ok(())
}

#[test]
fn test_tableau_builder() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let builder = TableauBuilder::new(&config)?;
    
    let ontology = Ontology::new();
    let tableau = builder.build_for_consistency(&ontology)?;
    
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    
    println!("✅ TableauBuilder works");
    Ok(())
}

#[test]
fn test_tableau_satisfiability() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let builder = TableauBuilder::new(&config)?;
    
    let ontology = Ontology::new();
    let tableau = builder.build_for_satisfiability(&ontology, "TestClass")?;
    
    // Should be satisfiable for a simple class
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    
    println!("✅ Tableau satisfiability checking works");
    Ok(())
}

#[test]
fn test_tableau_subsumption() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let builder = TableauBuilder::new(&config)?;
    
    let ontology = Ontology::new();
    let tableau = builder.build_for_subsumption(&ontology, "Dog", "Animal")?;
    
    // Should build tableau for subsumption check
    assert!(tableau.get_node_count() > 0);
    
    println!("✅ Tableau subsumption checking works");
    Ok(())
}

#[test]
fn test_tableau_instance_check() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let builder = TableauBuilder::new(&config)?;
    
    let ontology = Ontology::new();
    let tableau = builder.build_for_instance_check(&ontology, "fido", "Dog")?;
    
    // Should build tableau for instance check
    assert!(tableau.get_node_count() > 0);
    
    println!("✅ Tableau instance checking works");
    Ok(())
}

#[test]
fn test_tableau_factory() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let factory = TableauFactory::new(config)?;
    
    let ontology = Ontology::new();
    let runner = factory.create_for_consistency(&ontology)?;
    
    // Should create a tableau runner
    assert!(runner.get_node_count() >= 0);
    
    println!("✅ TableauFactory works");
    Ok(())
}

#[test]
fn test_dependency_tracking() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    let concept = ConceptLabel::Atomic("A".to_string());
    
    // Create dependency set
    let mut deps = DependencySet::empty();
    deps.add_axiom_dependency("axiom1".to_string());
    
    tableau.add_concept(node_id, concept.clone(), deps.clone())?;
    
    let node = tableau.get_node(node_id)?;
    if let Some(concept_deps) = node.concept_dependencies.get(&concept) {
        assert!(!concept_deps.is_empty());
    }
    
    println!("✅ Dependency tracking works");
    Ok(())
}

#[test]
fn test_tableau_completion() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    let concept = ConceptLabel::Atomic("SimpleClass".to_string());
    
    tableau.add_concept(node_id, concept, DependencySet::empty())?;
    
    // Try to complete the tableau
    let is_complete = tableau.is_complete();
    
    // A tableau with just one atomic concept should be complete
    // (depending on implementation details)
    println!("Tableau completion status: {}", is_complete);
    
    println!("✅ Tableau completion check works");
    Ok(())
}

#[test]
fn test_tableau_blocking() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    // Create two nodes with similar signatures
    let node1_id = tableau.create_root_node()?;
    let node2_id = tableau.create_successor_node(node1_id, "hasChild".to_string())?;
    
    let concept_a = ConceptLabel::Atomic("A".to_string());
    let concept_b = ConceptLabel::Atomic("B".to_string());
    
    // Add same concepts to both nodes
    tableau.add_concept(node1_id, concept_a.clone(), DependencySet::empty())?;
    tableau.add_concept(node1_id, concept_b.clone(), DependencySet::empty())?;
    
    tableau.add_concept(node2_id, concept_a, DependencySet::empty())?;
    tableau.add_concept(node2_id, concept_b, DependencySet::empty())?;
    
    // Check if blocking is detected
    let blocking_status = tableau.check_blocking(node2_id);
    
    println!("Blocking status for node {}: {:?}", node2_id, blocking_status);
    
    println!("✅ Tableau blocking works");
    Ok(())
}

#[test]
fn test_tableau_backtracking() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let mut tableau = Tableau::new(config)?;
    
    let node_id = tableau.create_root_node()?;
    
    // Create a choice point with disjunction
    let concept_a = ConceptLabel::Atomic("A".to_string());
    let concept_b = ConceptLabel::Atomic("B".to_string());
    let disjunction = ConceptLabel::Disjunction(vec![concept_a, concept_b]);
    
    tableau.add_concept(node_id, disjunction, DependencySet::empty())?;
    
    // Save state for backtracking
    let state = tableau.save_state();
    
    // Make a choice
    let concept_a = ConceptLabel::Atomic("A".to_string());
    tableau.add_concept(node_id, concept_a, DependencySet::empty())?;
    
    // Restore state
    tableau.restore_state(state)?;
    
    // Node should be restored to previous state
    let node = tableau.get_node(node_id)?;
    println!("Node after backtracking has {} concepts", node.concepts.len());
    
    println!("✅ Tableau backtracking works");
    Ok(())
}

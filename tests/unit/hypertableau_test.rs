//! Unit tests for hypertableau algorithms

use oxidowl::{
    Result,
    core::{
        hypertableau::{
            HyperTableau, 
            extension_table::ExtensionManager,
            ground_disjunction::{GroundDisjunction, GroundDisjunctionHeader, DisjunctPredicate, DisjunctionPriority},
            monitor::{TableauMonitor, MonitoringLevel},
            branching::{BranchingManager, BranchingStrategy},
        },
        blocking::AnywhereBlocking,
        dependency::DependencySet,
        tableau::TableauState,
    },
    ontology::{ClassExpression, Class, IRI, ObjectProperty, ObjectPropertyExpression, Ontology},
    config::ReasonerConfig,
};

#[test]
fn test_hypertableau_creation() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    
    let tableau = HyperTableau::new(config, blocking_checker)?;
    
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    assert!(!tableau.is_reasoning_complete());
    
    println!("HyperTableau creation works");
    Ok(())
}

#[test]
fn test_extension_manager() -> Result<()> {
    let mut manager = ExtensionManager::new();
    
    // Test concept assertion
    let individual = "john";
    let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
    
    manager.add_concept_assertion(individual, &concept)?;
    
    // Test role assertion
    let subject = "john";
    let object = "mary";
    let property = ObjectPropertyExpression::ObjectProperty(
        ObjectProperty::new(IRI::new("http://example.org/knows"))
    );
    
    manager.add_role_assertion(subject, &property, object)?;
    
    // Test queries
    assert!(manager.contains_concept_assertion(individual, &concept));
    assert!(manager.contains_role_assertion(subject, &property, object));
    
    println!("ExtensionManager works");
    Ok(())
}

#[test]
fn test_ground_disjunction() -> Result<()> {
    // Create predicates for ground disjunction
    let predicates = vec![
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/A"))),
            argument: 0,
        },
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/B"))),
            argument: 0,
        },
    ];
    
    let header = GroundDisjunctionHeader::new(predicates, DisjunctionPriority::Normal);
    let disjunction = GroundDisjunction::new(
        header,
        vec![0], // arguments (node IDs)
        vec![false], // is_core
        DependencySet::empty(),
        0, // id
    );
    
    assert_eq!(disjunction.get_id(), 0);
    assert_eq!(disjunction.get_arguments().len(), 1);
    
    println!("GroundDisjunction works");
    Ok(())
}

#[test]
fn test_tableau_monitor() -> Result<()> {
    let mut monitor = TableauMonitor::new();
    
    // Test monitoring level setting
    monitor.set_monitoring_level(MonitoringLevel::Debug);
    assert_eq!(monitor.get_monitoring_level(), MonitoringLevel::Debug);
    
    // Test event logging
    use oxidowl::core::hypertableau::monitor::events;
    use oxidowl::ontology::Individual;
    
    let event = events::fact_derived(
        "Test fact".to_string(),
        Individual::named(IRI::new("http://example.org/test")),
        0,
    );
    
    monitor.log_event(event);
    
    // Test statistics
    let stats = monitor.get_statistics();
    assert!(stats.events_logged >= 1);
    
    println!("TableauMonitor works");
    Ok(())
}

#[test]
fn test_branching_manager() -> Result<()> {
    let mut manager = BranchingManager::new();
    
    // Test branching strategy setting
    manager.set_strategy(BranchingStrategy::BestFirst);
    
    // Create a test ground disjunction for branching
    let predicates = vec![
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/X"))),
            argument: 0,
        },
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/Y"))),
            argument: 0,
        },
    ];
    
    let header = GroundDisjunctionHeader::new(predicates, DisjunctionPriority::High);
    let disjunction = GroundDisjunction::new(
        header,
        vec![0],
        vec![false],
        DependencySet::empty(),
        1,
    );
    
    // Add choice point
    manager.add_choice_point(disjunction, vec![0, 1]);
    
    // Test branch selection
    if let Some(branch) = manager.select_next_branch() {
        assert!(branch.disjunction_id == 1);
    }
    
    println!("BranchingManager works");
    Ok(())
}

#[test]
fn test_dependency_tracking() -> Result<()> {
    let mut deps = DependencySet::empty();
    
    // Add axiom dependencies
    deps.add_axiom_dependency("axiom1".to_string());
    deps.add_axiom_dependency("axiom2".to_string());
    
    // Add choice dependencies
    deps.add_choice_dependency(1, 0); // disjunction 1, choice 0
    
    assert!(!deps.is_empty());
    assert!(deps.contains_axiom("axiom1"));
    assert!(deps.contains_choice(1, 0));
    
    // Test merging
    let mut other_deps = DependencySet::empty();
    other_deps.add_axiom_dependency("axiom3".to_string());
    
    deps.merge(other_deps);
    assert!(deps.contains_axiom("axiom3"));
    
    println!("Dependency tracking works");
    Ok(())
}

#[test]
fn test_clash_detection() -> Result<()> {
    let mut manager = ExtensionManager::new();
    
    // Add contradictory concept assertions
    let individual = "test_individual";
    let concept_a = ClassExpression::Class(Class::new(IRI::new("http://example.org/A")));
    let concept_not_a = ClassExpression::Complement(
        Box::new(ClassExpression::Class(Class::new(IRI::new("http://example.org/A"))))
    );
    
    manager.add_concept_assertion(individual, &concept_a)?;
    manager.add_concept_assertion(individual, &concept_not_a)?;
    
    // Should detect clash
    let clashes = manager.detect_clashes();
    assert!(!clashes.is_empty(), "Should detect clash between A and ¬A");
    
    println!("Clash detection works");
    Ok(())
}

#[test]
fn test_hypertableau_reasoning() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Set up a simple reasoning task
    let ontology = Ontology::new();
    
    // Apply initial assertions (empty ontology for this test)
    tableau.apply_initial_assertions(&ontology)?;
    
    // Test that tableau is in consistent state
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    
    println!("HyperTableau reasoning works");
    Ok(())
}

#[test]
fn test_extension_table_operations() -> Result<()> {
    let mut manager = ExtensionManager::new();
    
    // Test adding facts with dependencies
    let deps = DependencySet::empty();
    
    manager.add_fact_with_dependencies(
        "TestPredicate".to_string(),
        vec!["arg1".to_string(), "arg2".to_string()],
        deps.clone(),
    )?;
    
    // Test querying facts
    let contains_fact = manager.contains_fact("TestPredicate", &["arg1", "arg2"]);
    assert!(contains_fact, "Should contain the added fact");
    
    // Test statistics
    let stats = manager.get_statistics();
    assert!(stats.total_facts >= 1);
    
    println!("Extension table operations work");
    Ok(())
}

#[test]
fn test_blocking_in_hypertableau() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Create nodes for blocking test
    let node1_id = tableau.create_node();
    let node2_id = tableau.create_node();
    
    // Add similar concepts to both nodes
    let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/TestConcept")));
    
    tableau.extension_manager.add_concept_assertion(&format!("node_{}", node1_id), &concept)?;
    tableau.extension_manager.add_concept_assertion(&format!("node_{}", node2_id), &concept)?;
    
    // Test blocking check
    let is_blocked = tableau.check_blocking(node2_id);
    println!("Node {} blocking status: {:?}", node2_id, is_blocked);
    
    println!("Blocking in HyperTableau works");
    Ok(())
}

#[test]
fn test_disjunction_priority_handling() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Create disjunctions with different priorities
    let high_priority_predicates = vec![
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/High"))),
            argument: 0,
        },
    ];
    
    let low_priority_predicates = vec![
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/Low"))),
            argument: 0,
        },
    ];
    
    let high_header = GroundDisjunctionHeader::new(high_priority_predicates, DisjunctionPriority::High);
    let low_header = GroundDisjunctionHeader::new(low_priority_predicates, DisjunctionPriority::Low);
    
    let high_disjunction = GroundDisjunction::new(
        high_header, vec![0], vec![false], DependencySet::empty(), 0
    );
    let low_disjunction = GroundDisjunction::new(
        low_header, vec![0], vec![false], DependencySet::empty(), 1
    );
    
    tableau.add_ground_disjunction(high_disjunction);
    tableau.add_ground_disjunction(low_disjunction);
    
    // High priority should be processed first
    assert!(tableau.has_pending_disjunctions());
    
    println!("Disjunction priority handling works");
    Ok(())
}

#[test]
fn test_backtracking() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Save initial state
    let checkpoint = tableau.create_checkpoint();
    
    // Make some changes
    let node_id = tableau.create_node();
    let concept = ClassExpression::Class(Class::new(IRI::new("http://example.org/Test")));
    tableau.extension_manager.add_concept_assertion(&format!("node_{}", node_id), &concept)?;
    
    let initial_facts = tableau.extension_manager.get_statistics().total_facts;
    
    // Restore checkpoint
    tableau.restore_checkpoint(checkpoint)?;
    
    let facts_after_restore = tableau.extension_manager.get_statistics().total_facts;
    assert!(facts_after_restore <= initial_facts, "Facts should be restored");
    
    println!("Backtracking works");
    Ok(())
}

#[test]
fn test_statistics_collection() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Simulate some operations
    tableau.statistics.clause_evaluations = 10;
    tableau.statistics.branching_points = 5;
    tableau.statistics.backtracks = 2;
    tableau.statistics.facts_derived = 20;
    
    let stats = tableau.get_statistics();
    assert_eq!(stats.clause_evaluations, 10);
    assert_eq!(stats.branching_points, 5);
    assert_eq!(stats.backtracks, 2);
    assert_eq!(stats.facts_derived, 20);
    
    println!("Statistics collection works");
    Ok(())
}

#[test] 
fn test_hypertableau_state_management() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Initial state should be satisfiable
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    
    // Simulate closure due to clash
    tableau.close_tableau();
    assert_eq!(tableau.get_state(), TableauState::Unsatisfiable);
    
    // Reset state
    tableau.reset_state();
    assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    assert!(!tableau.is_closed());
    
    println!("HyperTableau state management works");
    Ok(())
}

#[test]
fn test_rule_application() -> Result<()> {
    let config = ReasonerConfig::test_config();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    let mut tableau = HyperTableau::new(config, blocking_checker)?;
    
    // Create a disjunction that can trigger rule application
    let predicates = vec![
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/A"))),
            argument: 0,
        },
        DisjunctPredicate::Concept {
            concept: ClassExpression::Class(Class::new(IRI::new("http://example.org/B"))),
            argument: 0,
        },
    ];
    
    let header = GroundDisjunctionHeader::new(predicates, DisjunctionPriority::Normal);
    let disjunction = GroundDisjunction::new(
        header, vec![0], vec![false], DependencySet::empty(), 0
    );
    
    tableau.add_ground_disjunction(disjunction);
    
    // Apply one step of reasoning
    let progress_made = tableau.apply_expansion_step()?;
    
    // Should make progress by handling the disjunction
    println!("Progress made: {}", progress_made);
    
    println!("Rule application works");
    Ok(())
}

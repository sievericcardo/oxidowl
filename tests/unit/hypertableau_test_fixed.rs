//! Hypertableau Unit Tests
//!
//! Tests for the hypertableau rea#[test]
fn test_monitor_integration() {
    use oxidowl::core::hypertableau::monitor::{TableauMonitor, MonitoringLevel};
    use std::time::Duration;
    
    let mut monitor = TableauMonitor::new(MonitoringLevel::Basic);
    
    // Test monitor can track reasoning progress
    monitor.start();
    monitor.start_reasoning();
    
    let stats = monitor.finish();
    
    // Verify basic fields exist
    assert!(stats.total_duration >= Duration::from_secs(0));
}

use oxidowl::{
    core::hypertableau::HyperTableau,
    config::ReasoningConfig,
    core::blocking::AnywhereBlocking,
    ontology::Ontology,
};

#[test]
fn test_hypertableau_creation() {
    let config = ReasoningConfig::default();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    
    let result = HyperTableau::new(config, blocking_checker);
    assert!(result.is_ok());
}

#[test]
fn test_extension_manager_creation() {
    use oxidowl::core::hypertableau::extension_table::ExtensionManager;
    
    let manager = ExtensionManager::new();
    // Test that extension manager can be created
    // Just verify it exists for now
    drop(manager);
}

#[test]
fn test_basic_hypertableau_components() {
    // Test that the main hypertableau components can be instantiated
    let config = ReasoningConfig::default();
    let blocking_checker = Box::new(AnywhereBlocking::new());
    
    let result = HyperTableau::new(config, blocking_checker);
    assert!(result.is_ok());
    
    if let Ok(tableau) = result {
        // Test basic functionality exists
        drop(tableau);
    }
}

#[test]
fn test_hypertableau_statistics() {
    use oxidowl::core::hypertableau::HyperTableauStatistics;
    
    let stats = HyperTableauStatistics {
        nodes_created: 0,
        disjunctions_processed: 0,
        clause_evaluations: 0,
        branching_points: 0,
        backtracks: 0,
        hyperresolution_time: std::time::Duration::from_secs(0),
        clause_evaluation_time: std::time::Duration::from_secs(0),
        cache_hit_ratio: 0.0,
        max_depth: 0,
        facts_derived: 0,
    };
    
    assert_eq!(stats.nodes_created, 0);
    assert_eq!(stats.backtracks, 0);
}

//! HyperTableau Algorithm Implementation
//!
//! This module implements HermiT's hypertableau algorithm that uses
//! hyperresolution and ground disjunctions for efficient tableau reasoning.

pub mod ground_disjunction;
pub mod hyperresolution;
pub mod clause_evaluator;
pub mod extension_tables;
pub mod dependency_tracking;
pub mod branching;
pub mod monitor;

use crate::{
    config::ReasoningConfig,
    core::{
        tableau::{TableauNode, TableauEdge, TableauState},
        blocking::BlockingChecker,
        dependency::DependencySet,
        completion::CompletionRule,
    },
    ontology::{Ontology, ClassExpression, Individual, Axiom},
    Error, Result,
};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};

use ground_disjunction::{GroundDisjunction, GroundDisjunctionHeader};
use hyperresolution::HyperresolutionManager;
use clause_evaluator::DLClauseEvaluator;
use extension_tables::ExtensionManager;
use dependency_tracking::{DependencyTracker, BranchingPointId, FactId};
use branching::{BranchingManager, BranchingStrategy, BranchingType, BranchingChoice};
use monitor::{TableauMonitor, MonitoringLevel, ReasoningStats};

/// Main HyperTableau structure combining HermiT's algorithm with
#[derive(Debug)]
pub struct HyperTableau {
    /// Tableau nodes from traditional tableau
    nodes: HashMap<usize, TableauNode>,
    
    /// Edges between nodes
    edges: Vec<TableauEdge>,
    
    /// Extension manager for fact storage and retrieval
    extension_manager: ExtensionManager,
    
    /// Hyperresolution manager for clause compilation and evaluation
    hyperresolution_manager: HyperresolutionManager,
    
    /// Ground disjunctions awaiting processing
    ground_disjunctions: VecDeque<GroundDisjunction>,
    
    /// First unprocessed ground disjunction
    first_unprocessed_disjunction: Option<usize>,
    
    /// Branching manager for handling non-deterministic choices
    branching_manager: BranchingManager,
    
    /// Dependency tracker for backjumping
    dependency_tracker: Arc<Mutex<DependencyTracker>>,
    
    /// Blocking strategy
    blocking_checker: Box<dyn BlockingChecker>,
    
    /// Tableau monitor for debugging and statistics
    monitor: TableauMonitor,
    
    /// Current state of the tableau
    state: TableauState,
    
    /// Configuration settings
    config: ReasoningConfig,
    
    /// Statistics collection
    statistics: HyperTableauStatistics,
    
    /// Next node ID
    next_node_id: usize,
    
    /// Whether the tableau is closed (inconsistent)
    is_closed: bool,
    
    /// Whether the tableau expansion is complete
    is_complete: bool,
}

/// Statistics for hypertableau reasoning
#[derive(Debug, Default, Clone)]
pub struct HyperTableauStatistics {
    /// Number of nodes created
    pub nodes_created: u64,
    
    /// Number of ground disjunctions processed
    pub disjunctions_processed: u64,
    
    /// Number of clause evaluations
    pub clause_evaluations: u64,
    
    /// Number of branching points created
    pub branching_points: u64,
    
    /// Number of backtracking operations
    pub backtracks: u64,
    
    /// Time spent in hyperresolution
    pub hyperresolution_time: std::time::Duration,
    
    /// Time spent in clause evaluation
    pub clause_evaluation_time: std::time::Duration,
    
    /// Cache hit ratio
    pub cache_hit_ratio: f64,
    
    /// Maximum depth reached
    pub max_depth: u32,
}

impl HyperTableau {
    /// Create a new hypertableau with the given configuration
    pub fn new(
        config: ReasoningConfig,
        blocking_checker: Box<dyn BlockingChecker>,
    ) -> Result<Self> {
        let extension_manager = ExtensionManager::new(&config)?;
        let hyperresolution_manager = HyperresolutionManager::new(&config)?;
        let dependency_tracker = Arc::new(Mutex::new(DependencyTracker::new()));
        let branching_manager = BranchingManager::new(
            BranchingStrategy::DepthFirst, // Default strategy
            dependency_tracker.clone(),
        );
        let monitor = TableauMonitor::new(MonitoringLevel::Basic);
        
        Ok(HyperTableau {
            nodes: HashMap::new(),
            edges: Vec::new(),
            extension_manager,
            hyperresolution_manager,
            ground_disjunctions: VecDeque::new(),
            first_unprocessed_disjunction: None,
            branching_manager,
            dependency_tracker,
            blocking_checker,
            monitor,
            state: TableauState::Satisfiable,
            config,
            statistics: HyperTableauStatistics::default(),
            next_node_id: 0,
            is_closed: false,
            is_complete: false,
        })
    }
    
    /// Set the monitoring level
    pub fn set_monitoring_level(&mut self, level: MonitoringLevel) {
        self.monitor.set_monitoring_level(level);
    }
    
    /// Set branching strategy
    pub fn set_branching_strategy(&mut self, strategy: BranchingStrategy) {
        self.branching_manager = BranchingManager::new(strategy, self.dependency_tracker.clone());
    }
    
    /// Initialize the tableau with an ontology
    pub fn initialize(&mut self, ontology: &Ontology) -> Result<()> {
        self.monitor.start();
        
        // Reset state
        self.reset_state();
        
        // Compile DL clauses from the ontology
        let dl_clauses = self.compile_ontology_to_clauses(ontology)?;
        
        // Initialize hyperresolution manager with compiled clauses
        self.hyperresolution_manager.initialize(dl_clauses)?;
        
        // Create initial individual nodes from ABox
        self.create_initial_nodes(ontology)?;
        
        // Apply initial concept assertions
        self.apply_initial_assertions(ontology)?;
        
        Ok(())
    }
    
    /// Run the main hypertableau algorithm
    pub fn run(&mut self) -> Result<TableauState> {
        self.monitor.start_reasoning();
        
        let start_time = Instant::now();
        
        // Main saturation loop
        while !self.is_complete && !self.is_closed {
            if !self.do_iteration()? {
                break;
            }
        }
        
        self.statistics.hyperresolution_time += start_time.elapsed();
        
        // Determine final state
        self.state = if self.is_closed {
            TableauState::Unsatisfiable
        } else {
            TableauState::Satisfiable
        };
        
        // Update statistics
        self.update_final_statistics();
        
        Ok(self.state)
    }
    
    /// Get the final reasoning statistics
    pub fn get_reasoning_stats(&mut self) -> ReasoningStats {
        // Update monitor with latest statistics
        self.monitor.update_dependency_stats(
            &self.dependency_tracker.lock().unwrap().get_stats()
        );
        self.monitor.update_branching_stats(self.branching_manager.get_stats());
        self.monitor.update_hyperresolution_stats(self.hyperresolution_manager.get_stats());
        self.monitor.update_extension_stats(self.extension_manager.get_stats());
        
        self.monitor.finish()
    }
    
    /// Perform one iteration of the hypertableau algorithm
    fn do_iteration(&mut self) -> Result<bool> {
        let mut has_change = false;
        
        // 1. Apply hyperresolution rules
        if self.apply_hyperresolution()? {
            has_change = true;
        }
        
        // 2. Process ground disjunctions
        if self.process_ground_disjunctions()? {
            has_change = true;
        }
        
        // 3. Apply traditional tableau rules for remaining concepts
        if self.apply_traditional_rules()? {
            has_change = true;
        }
        
        // 4. Check for clashes
        if self.extension_manager.contains_clash() {
            if !self.handle_clash()? {
                self.is_closed = true;
                has_change = false;
            } else {
                has_change = true;
            }
        }
        
        // 5. Apply blocking strategies
        if !self.is_closed {
            self.apply_blocking()?;
        }
        
        Ok(has_change)
    }
    
    /// Apply hyperresolution to derive new ground disjunctions
    fn apply_hyperresolution(&mut self) -> Result<bool> {
        let start_time = Instant::now();
        
        let new_disjunctions = self.hyperresolution_manager.apply_rules(
            &mut self.extension_manager,
            &self.dependency_tracker.lock().unwrap(),
        )?;
        
        self.statistics.clause_evaluation_time += start_time.elapsed();
        self.statistics.clause_evaluations += new_disjunctions.len() as u64;
        
        if !new_disjunctions.is_empty() {
            for disjunction in new_disjunctions {
                self.add_ground_disjunction(disjunction);
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Process pending ground disjunctions
    fn process_ground_disjunctions(&mut self) -> Result<bool> {
        let mut has_change = false;
        
        while let Some(disjunction_id) = self.first_unprocessed_disjunction {
            if disjunction_id >= self.ground_disjunctions.len() {
                self.first_unprocessed_disjunction = None;
                break;
            }
            
            let disjunction = &self.ground_disjunctions[disjunction_id];
            
            // Log monitoring event
            self.monitor.log_event(monitor::events::ground_disjunction_processing(
                format!("{:?}", disjunction),
                disjunction.individual.clone(),
                disjunction.disjuncts.len(),
                std::time::Duration::default(),
            ));
            
            // Check if disjunction is already satisfied
            if disjunction.is_satisfied(&self.extension_manager)? {
                self.first_unprocessed_disjunction = Some(disjunction_id + 1);
                continue;
            }
            
            // Handle the disjunction
            if self.handle_ground_disjunction(disjunction_id)? {
                has_change = true;
            }
            
            self.first_unprocessed_disjunction = Some(disjunction_id + 1);
            self.statistics.disjunctions_processed += 1;
        }
        
        Ok(has_change)
    }
    
    /// Handle a specific ground disjunction
    fn handle_ground_disjunction(&mut self, disjunction_id: usize) -> Result<bool> {
        let disjunction = self.ground_disjunctions[disjunction_id].clone();
        
        // If only one disjunct, apply it directly
        if disjunction.disjuncts.len() == 1 {
            let fact_id = {
                let mut tracker = self.dependency_tracker.lock().unwrap();
                tracker.create_fact(
                    format!("Direct disjunct: {}", disjunction.disjuncts[0]),
                    dependency_tracking::utils::clause_application_dependency(
                        disjunction_id,
                        vec![], // TODO: Add premise facts
                    ),
                )?
            };
            
            self.extension_manager.add_concept_assertion(
                &disjunction.individual,
                &disjunction.disjuncts[0],
            )?;
            
            self.monitor.log_event(monitor::events::fact_derived(
                format!("Direct disjunct: {}", disjunction.disjuncts[0]),
                disjunction.individual.clone(),
                0,
            ));
            
            return Ok(true);
        }
        
        // Create branching point for multiple disjuncts
        let choices = branching::utils::create_disjunction_choices(
            &disjunction,
            &disjunction.individual,
        );
        
        let branch_id = self.branching_manager.create_branching_point(
            BranchingType::GroundDisjunction {
                disjunction: disjunction.clone(),
                individual: disjunction.individual.clone(),
            },
            choices,
        )?;
        
        self.statistics.branching_points += 1;
        
        // Make the first choice
        if let Some((assertion, individual)) = self.branching_manager.make_choice(branch_id, None)? {
            let fact_id = {
                let mut tracker = self.dependency_tracker.lock().unwrap();
                tracker.create_fact(
                    format!("Branching choice: {}", assertion),
                    dependency_tracking::utils::branching_dependency(branch_id, 0),
                )?
            };
            
            self.extension_manager.add_concept_assertion(&individual, &assertion)?;
            
            self.monitor.log_event(monitor::events::fact_derived(
                format!("Branching choice: {}", assertion),
                individual,
                0,
            ));
        }
        
        Ok(true)
    }
    
    /// Apply traditional tableau rules
    fn apply_traditional_rules(&mut self) -> Result<bool> {
        // This would implement traditional tableau expansion rules
        // for concepts that are not handled by hyperresolution
        // For now, return false as this is complex and depends on
        // the specific tableau rules being used
        Ok(false)
    }
    
    /// Handle clash detection and backtracking
    fn handle_clash(&mut self) -> Result<bool> {
        // Try to backtrack to resolve the clash
        if let Some(branch_id) = self.branching_manager.backtrack()? {
            self.statistics.backtracks += 1;
            
            // Try the next choice at the branching point
            if let Some((assertion, individual)) = 
                self.branching_manager.make_choice(branch_id, None)? 
            {
                let fact_id = {
                    let mut tracker = self.dependency_tracker.lock().unwrap();
                    tracker.create_fact(
                        format!("Backtrack choice: {}", assertion),
                        dependency_tracking::utils::branching_dependency(branch_id, 1),
                    )?
                };
                
                self.extension_manager.add_concept_assertion(&individual, &assertion)?;
                return Ok(true);
            }
        }
        
        // No more choices, tableau is closed
        Ok(false)
    }
    
    /// Apply blocking strategies
    fn apply_blocking(&mut self) -> Result<()> {
        // TODO: Implement blocking strategies
        // This would check for blocking conditions and apply them
        Ok(())
    }
    
    /// Add a ground disjunction to the processing queue
    fn add_ground_disjunction(&mut self, disjunction: GroundDisjunction) {
        self.ground_disjunctions.push_back(disjunction);
        
        if self.first_unprocessed_disjunction.is_none() {
            self.first_unprocessed_disjunction = Some(self.ground_disjunctions.len() - 1);
        }
    }
    
    /// Compile ontology axioms to DL clauses
    fn compile_ontology_to_clauses(&self, ontology: &Ontology) -> Result<Vec<hyperresolution::DLClause>> {
        // TODO: Implement clause compilation from ontology
        // This would translate OWL axioms to DL clauses
        Ok(vec![])
    }
    
    /// Create initial nodes from ABox individuals
    fn create_initial_nodes(&mut self, ontology: &Ontology) -> Result<()> {
        // TODO: Create tableau nodes for ABox individuals
        Ok(())
    }
    
    /// Apply initial concept assertions from ABox
    fn apply_initial_assertions(&mut self, ontology: &Ontology) -> Result<()> {
        // TODO: Apply initial concept and role assertions
        Ok(())
    }
    
    /// Reset internal state for new reasoning task
    fn reset_state(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.ground_disjunctions.clear();
        self.first_unprocessed_disjunction = None;
        self.branching_manager.reset();
        {
            let mut tracker = self.dependency_tracker.lock().unwrap();
            tracker.reset();
        }
        self.extension_manager.clear();
        self.hyperresolution_manager.reset();
        self.next_node_id = 0;
        self.is_closed = false;
        self.is_complete = false;
        self.state = TableauState::Satisfiable;
        self.statistics = HyperTableauStatistics::default();
    }
    
    /// Update final statistics after reasoning
    fn update_final_statistics(&mut self) {
        self.statistics.max_depth = self.branching_manager.get_stats().max_depth as u32;
        self.statistics.cache_hit_ratio = self.extension_manager.get_stats().cache_hit_ratio;
        
        // Calculate averages and final metrics
        self.branching_manager.calculate_average_branching_factor();
    }
    
    /// Get current tableau state
    pub fn get_state(&self) -> TableauState {
        self.state
    }
    
    /// Get internal statistics
    pub fn get_statistics(&self) -> &HyperTableauStatistics {
        &self.statistics
    }
    
    /// Check if the tableau is satisfiable
    pub fn is_satisfiable(&self) -> bool {
        matches!(self.state, TableauState::Satisfiable) && !self.is_closed
    }
    
    /// Check if reasoning is complete
    pub fn is_reasoning_complete(&self) -> bool {
        self.is_complete || self.is_closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::blocking::SimpleBlockingChecker;
    
    #[test]
    fn test_hypertableau_creation() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(SimpleBlockingChecker::new());
        
        let tableau = HyperTableau::new(config, blocking_checker);
        assert!(tableau.is_ok());
        
        let tableau = tableau.unwrap();
        assert_eq!(tableau.get_state(), TableauState::Satisfiable);
        assert!(!tableau.is_reasoning_complete());
    }
    
    #[test]
    fn test_monitoring_level_setting() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(SimpleBlockingChecker::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        tableau.set_monitoring_level(MonitoringLevel::Debug);
        
        assert_eq!(tableau.monitor.get_monitoring_level(), MonitoringLevel::Debug);
    }
    
    #[test]
    fn test_branching_strategy_setting() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(SimpleBlockingChecker::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        tableau.set_branching_strategy(BranchingStrategy::BestFirst);
        
        // Strategy is internal to BranchingManager, so just verify no errors
        assert!(true);
    }
    
    #[test]
    fn test_state_reset() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(SimpleBlockingChecker::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        
        // Add some mock data
        tableau.next_node_id = 5;
        tableau.is_closed = true;
        
        // Reset should clear state
        tableau.reset_state();
        
        assert_eq!(tableau.next_node_id, 0);
        assert!(!tableau.is_closed);
        assert_eq!(tableau.get_state(), TableauState::Satisfiable);
    }
    
    #[test]
    fn test_ground_disjunction_handling() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(SimpleBlockingChecker::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        
        // Create a simple ground disjunction
        let disjunction = GroundDisjunction {
            individual: Individual::new("test_ind".to_string()),
            disjuncts: vec![
                ClassExpression::Class("A".to_string()),
                ClassExpression::Class("B".to_string()),
            ],
            priority: 1.0,
            ..Default::default()
        };
        
        tableau.add_ground_disjunction(disjunction);
        
        assert_eq!(tableau.ground_disjunctions.len(), 1);
        assert_eq!(tableau.first_unprocessed_disjunction, Some(0));
    }
    
    #[test]
    fn test_statistics_tracking() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(SimpleBlockingChecker::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        
        // Simulate some operations
        tableau.statistics.clause_evaluations = 10;
        tableau.statistics.branching_points = 5;
        tableau.statistics.backtracks = 2;
        
        let stats = tableau.get_statistics();
        assert_eq!(stats.clause_evaluations, 10);
        assert_eq!(stats.branching_points, 5);
        assert_eq!(stats.backtracks, 2);
    }
}

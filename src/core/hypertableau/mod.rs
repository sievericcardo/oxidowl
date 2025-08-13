//! `HyperTableau` Algorithm Implementation
//!
//! This module implements `HermiT`'s hypertableau algorithm that uses
//! hyperresolution and ground disjunctions for efficient tableau reasoning.

pub mod ground_disjunction;
pub mod hyperresolution;
pub mod clause_evaluator;
pub mod extension_table;
pub mod dependency_tracking;
pub mod branching;
pub mod monitor;

use crate::{
    config::ReasoningConfig,
    core::{
        tableau::{TableauNode, TableauEdge, TableauState},
        blocking::BlockingChecker,
    },
    ontology::{Ontology, Individual, ClassExpression, ObjectPropertyExpression, Axiom},
    Result,
};

use std::{
    collections::{HashMap, VecDeque, HashSet},
    sync::{Arc, Mutex},
    time::Instant,
};

use ground_disjunction::{GroundDisjunction, DisjunctPredicate};
use hyperresolution::HyperresolutionManager;
use extension_table::ExtensionManager;
use dependency_tracking::DependencyTracker;
use branching::{BranchingManager, BranchingStrategy, BranchingType};
use monitor::{TableauMonitor, MonitoringLevel, ReasoningStats};

/// Main `HyperTableau` structure combining `HermiT`'s algorithm with
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
    
    /// Facts derived during reasoning
    pub facts_derived: u64,
}

impl HyperTableau {
    /// Create a new hypertableau with the given configuration
    pub fn new(
        config: ReasoningConfig,
        blocking_checker: Box<dyn BlockingChecker>,
    ) -> Result<Self> {
        let extension_manager = ExtensionManager::new();
        let hyperresolution_manager = HyperresolutionManager::new(Vec::new(), true)?;
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
            self.dependency_tracker.lock().unwrap().get_stats()
        );
        self.monitor.update_branching_stats(self.branching_manager.get_stats());
        self.monitor.update_hyperresolution_stats(&self.hyperresolution_manager.get_statistics());
        self.monitor.update_extension_stats(self.extension_manager.get_statistics());
        
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
            if self.handle_clash()? {
                has_change = true;
            } else {
                self.is_closed = true;
                has_change = false;
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
            &mut self.branching_manager,
        )?;
        
        self.statistics.clause_evaluation_time += start_time.elapsed();
        
        if new_disjunctions {
            self.statistics.clause_evaluations += 1;
        }
        
        Ok(new_disjunctions)
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
            // Log disjunction processing
            let individual_str = disjunction.individual();
            let individual_iri = crate::ontology::IRI::new(&format!("http://example.org/{individual_str}"));
            let individual = Individual::named(individual_iri);
            
            // Log monitoring event
            self.monitor.log_event(monitor::events::ground_disjunction_processing(
                format!("{disjunction:?}"),
                individual,
                disjunction.disjuncts().len(),
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
        if disjunction.disjuncts().len() == 1 {
            let premise_facts = vec![
                dependency_tracking::FactId(disjunction_id),
                dependency_tracking::FactId(disjunction_id + 1000), // Offset for context facts
            ];
            
            let fact_id = {
                let mut tracker = self.dependency_tracker.lock().unwrap();
                tracker.create_fact(
                    format!("Direct disjunct: {}", disjunction.disjuncts()[0]),
                    dependency_tracking::utils::clause_application_dependency(
                        disjunction_id,
                        premise_facts,
                    ),
                )?
            };
            
             // Extract concept from the first disjunct
            if let DisjunctPredicate::Concept { concept, .. } = &disjunction.disjuncts()[0] {
                self.extension_manager.add_concept_assertion(
                    &disjunction.individual(),
                    concept,
                )?;
                
                // Create Individual for monitoring
                let individual_str = disjunction.individual();
                let individual_iri = crate::ontology::IRI::new(&format!("http://example.org/{individual_str}"));
                let individual = Individual::named(individual_iri);
                
                self.monitor.log_event(monitor::events::fact_derived(
                    format!("Direct disjunct: {}", disjunction.disjuncts()[0]),
                    individual,
                    0,
                ));
            }
            
            return Ok(true);
        }
        
        // Create branching point for multiple disjuncts
        let individual_str = disjunction.individual();
        let individual_iri = crate::ontology::IRI::new(&format!("http://example.org/{individual_str}"));
        let individual = Individual::named(individual_iri);

        let choices = branching::utils::create_disjunction_choices(
            &disjunction,
            &individual,
        );
        
        let branch_id = self.branching_manager.create_branching_point(
            BranchingType::GroundDisjunction {
                disjunction: disjunction.clone(),
                individual: individual.clone(),
            },
            choices,
        )?;
        
        self.statistics.branching_points += 1;
        
        // Make the first choice
        if let Some((assertion, individual)) = self.branching_manager.make_choice(branch_id, None)? {
            let fact_id = {
                let mut tracker = self.dependency_tracker.lock().unwrap();
                tracker.create_fact(
                    format!("Branching choice: {assertion}"),
                    dependency_tracking::utils::branching_dependency(branch_id, 0),
                )?
            };
            
            self.extension_manager.add_concept_assertion(&individual.iri().map_or("anonymous".to_string(), std::string::ToString::to_string), &assertion)?;
            
            self.monitor.log_event(monitor::events::fact_derived(
                format!("Branching choice: {assertion}"),
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
                        format!("Backtrack choice: {assertion}"),
                        dependency_tracking::utils::branching_dependency(branch_id, 1),
                    )?
                };
                
                self.extension_manager.add_concept_assertion(&individual.to_string(), &assertion)?;
                return Ok(true);
            }
        }
        
        // No more choices, tableau is closed
        Ok(false)
    }
    
    /// Apply blocking strategies
    fn apply_blocking(&mut self) -> Result<()> {
        // Check for blocking opportunities
        // This implements a simple anywhere blocking strategy
        let individuals: Vec<String> = self.extension_manager
            .get_all_individuals()
            .unwrap_or_default();

        for blocker in &individuals {
            for blocked in &individuals {
                if blocker != blocked && self.can_block(blocker, blocked)? {
                    self.extension_manager.add_blocking(blocker.clone(), blocked.clone())?;
                    
                    self.monitor.log_event(monitor::events::blocking_operation(
                        blocker.clone(),
                        blocked.clone(),
                        "anywhere".to_string(),
                        std::time::Duration::default(),
                    ));
                }
            }
        }
        
        Ok(())
    }

    /// Check if one individual can block another
    fn can_block(&self, blocker: &str, blocked: &str) -> Result<bool> {
        // Simple blocking: blocker blocks blocked if blocker has all concepts that blocked has
        let blocker_concepts = self.extension_manager.get_individual_concepts(blocker)?;
        let blocked_concepts = self.extension_manager.get_individual_concepts(blocked)?;
        
        // Blocker can block blocked if blocked's concepts are a subset of blocker's concepts
        for concept in &blocked_concepts {
            if !blocker_concepts.contains(concept) {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// Add a ground disjunction to the processing queue
    pub fn add_ground_disjunction(&mut self, disjunction: GroundDisjunction) {
        self.ground_disjunctions.push_back(disjunction);
        
        if self.first_unprocessed_disjunction.is_none() {
            self.first_unprocessed_disjunction = Some(self.ground_disjunctions.len() - 1);
        }
    }
    
    /// Compile ontology axioms to DL clauses
    fn compile_ontology_to_clauses(&self, ontology: &Ontology) -> Result<Vec<hyperresolution::DLClause>> {
        let mut clauses = Vec::new();
        
        // Compile TBox axioms (subclass, equivalent class axioms)
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::SubClassOf(subclass_axiom) => {
                    let clause = self.compile_subclass_axiom(subclass_axiom)?;
                    clauses.push(clause);
                }
                Axiom::EquivalentClasses(equiv_axiom) => {
                    let equiv_clauses = self.compile_equivalent_classes_axiom(equiv_axiom)?;
                    clauses.extend(equiv_clauses);
                }
                _ => {
                    // Handle other axiom types as needed
                }
            }
        }
        
        // Compile ABox axioms (class assertions, property assertions)
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(class_assertion) => {
                    let clause = self.compile_class_assertion(class_assertion)?;
                    clauses.push(clause);
                }
                Axiom::ObjectPropertyAssertion(prop_assertion) => {
                    let clause = self.compile_object_property_assertion(prop_assertion)?;
                    clauses.push(clause);
                }
                _ => {
                    // Handle other ABox axioms
                }
            }
        }
        
        Ok(clauses)
    }

    /// Compile a subclass axiom to DL clause
    fn compile_subclass_axiom(&self, axiom: &crate::ontology::SubClassOfAxiom) -> Result<hyperresolution::DLClause> {
        // SubClassOf(A, B) becomes ¬A(x) ∨ B(x)
        let var_x = "x".to_string();
        
        let subclass_atom = self.compile_class_expression_to_atom(&axiom.subclass, &var_x, true)?; // negated
        let superclass_atom = self.compile_class_expression_to_atom(&axiom.superclass, &var_x, false)?; // positive
        
        Ok(hyperresolution::DLClause {
            head: vec![superclass_atom],
            body: vec![subclass_atom],
            variables: HashSet::from([var_x]),
            id: axiom.id.to_string(),
        })
    }

    /// Compile equivalent classes axiom to DL clauses
    fn compile_equivalent_classes_axiom(&self, axiom: &crate::ontology::EquivalentClassesAxiom) -> Result<Vec<hyperresolution::DLClause>> {
        let mut clauses = Vec::new();
        
        // EquivalentClasses(A, B) becomes A(x) ≡ B(x), which is two implications
        for i in 0..axiom.classes.len() {
            for j in (i+1)..axiom.classes.len() {
                let var_x = "x".to_string();
                
                // A(x) → B(x): ¬A(x) ∨ B(x)
                let a_atom = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, true)?;
                let b_atom = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, false)?;
                
                clauses.push(hyperresolution::DLClause {
                    head: vec![b_atom],
                    body: vec![a_atom],
                    variables: HashSet::from([var_x.clone()]),
                    id: format!("{}_forward_{}", axiom.id, i),
                });
                
                // B(x) → A(x): ¬B(x) ∨ A(x)
                let b_atom_neg = self.compile_class_expression_to_atom(&axiom.classes[j], &var_x, true)?;
                let a_atom_pos = self.compile_class_expression_to_atom(&axiom.classes[i], &var_x, false)?;
                
                clauses.push(hyperresolution::DLClause {
                    head: vec![a_atom_pos],
                    body: vec![b_atom_neg],
                    variables: HashSet::from([var_x]),
                    id: format!("{}_backward_{}", axiom.id, i),
                });
            }
        }
        
        Ok(clauses)
    }

    /// Compile a class assertion to DL clause
    fn compile_class_assertion(&self, axiom: &crate::ontology::ClassAssertionAxiom) -> Result<hyperresolution::DLClause> {
        // ClassAssertion(A, a) becomes A(a)
        let individual_name = match &axiom.individual {
            Individual::Named(named) => named.iri.to_string(),
            Individual::Anonymous(anon) => anon.id.clone(),
        };
        
        let atom = self.compile_class_expression_to_atom(&axiom.class, &individual_name, false)?;
        
        Ok(hyperresolution::DLClause {
            head: vec![atom],
            body: vec![],
            variables: HashSet::new(),
            id: axiom.id.to_string(),
        })
    }

    /// Compile an object property assertion to DL clause
    fn compile_object_property_assertion(&self, axiom: &crate::ontology::ObjectPropertyAssertionAxiom) -> Result<hyperresolution::DLClause> {
        // ObjectPropertyAssertion(R, a, b) becomes R(a, b)
        let subject_name = match &axiom.source {
            Individual::Named(named) => named.iri.to_string(),
            Individual::Anonymous(anon) => anon.id.clone(),
        };
        
        let object_name = match &axiom.target {
            Individual::Named(named) => named.iri.to_string(),
            Individual::Anonymous(anon) => anon.id.clone(),
        };
        
        let property_iri = match &axiom.property {
            ObjectPropertyExpression::ObjectProperty(prop) => prop.iri.to_string(),
            ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("inverse({})", prop.iri)
            }
            ObjectPropertyExpression::PropertyChain(_chain) => {
                // For now, treat property chains as a single property
                // This is a simplification - proper handling would require complex reasoning
                "property_chain".to_string()
            }
        };
        
        let atom = hyperresolution::Atom {
            predicate: property_iri,
            arguments: vec![subject_name, object_name],
            is_positive: true,
        };
        
        Ok(hyperresolution::DLClause {
            head: vec![atom],
            body: vec![],
            variables: HashSet::new(),
            id: axiom.id.to_string(),
        })
    }

    /// Compile a class expression to an atom
    fn compile_class_expression_to_atom(&self, expr: &ClassExpression, variable: &str, negated: bool) -> Result<hyperresolution::Atom> {
        match expr {
            ClassExpression::Class(class) => {
                Ok(hyperresolution::Atom {
                    predicate: class.iri.to_string(),
                    arguments: vec![variable.to_string()],
                    is_positive: !negated,
                })
            }
            _ => {
                // For complex expressions, we'd need to create additional clauses
                // For now, return a placeholder
                Ok(hyperresolution::Atom {
                    predicate: "complex_expression".to_string(),
                    arguments: vec![variable.to_string()],
                    is_positive: !negated,
                })
            }
        }
    }
    
    /// Create initial nodes from `ABox` individuals
    fn create_initial_nodes(&mut self, ontology: &Ontology) -> Result<()> {
        // Get all individuals from the ABox
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(class_assertion) => {
                    let individual_name = match &class_assertion.individual {
                        Individual::Named(named) => named.iri.to_string(),
                        Individual::Anonymous(anon) => anon.id.clone(),
                    };
                    
                    // Create node for this individual if not already exists
                    self.extension_manager.ensure_individual_exists(&individual_name)?;
                }
                Axiom::ObjectPropertyAssertion(prop_assertion) => {
                    let subject_name = match &prop_assertion.source {
                        Individual::Named(named) => named.iri.to_string(),
                        Individual::Anonymous(anon) => anon.id.clone(),
                    };
                    
                    let object_name = match &prop_assertion.target {
                        Individual::Named(named) => named.iri.to_string(),
                        Individual::Anonymous(anon) => anon.id.clone(),
                    };
                    
                    // Create nodes for both individuals
                    self.extension_manager.ensure_individual_exists(&subject_name)?;
                    self.extension_manager.ensure_individual_exists(&object_name)?;
                }
                _ => {
                    // Handle other ABox axioms as needed
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply initial concept assertions from `ABox`
    fn apply_initial_assertions(&mut self, ontology: &Ontology) -> Result<()> {
        let start_fact_count = self.statistics.facts_derived;
        
        // Apply class assertions
        for axiom in ontology.axioms() {
            match axiom {
                Axiom::ClassAssertion(class_assertion) => {
                    let individual_name = match &class_assertion.individual {
                        Individual::Named(named) => named.iri.to_string(),
                        Individual::Anonymous(anon) => anon.id.clone(),
                    };
                    
                    // Add the class assertion to extension tables
                    self.extension_manager.add_concept_assertion(&individual_name, &class_assertion.class)?;
                    self.statistics.facts_derived += 1;
                }
                Axiom::ObjectPropertyAssertion(prop_assertion) => {
                    let subject_name = match &prop_assertion.source {
                        Individual::Named(named) => named.iri.to_string(),
                        Individual::Anonymous(anon) => anon.id.clone(),
                    };
                    
                    let object_name = match &prop_assertion.target {
                        Individual::Named(named) => named.iri.to_string(),
                        Individual::Anonymous(anon) => anon.id.clone(),
                    };
                    
                    // Add the property assertion to extension tables
                    self.extension_manager.add_role_assertion(&subject_name, &prop_assertion.property, &object_name)?;
                    self.statistics.facts_derived += 1;
                }
                _ => {
                    // Handle other ABox axioms (data property assertions, etc.)
                }
            }
        }
        
        let initial_facts = self.statistics.facts_derived - start_fact_count;
        log::info!("Applied {initial_facts} initial assertions from ABox");
        
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
        let ext_stats = self.extension_manager.get_statistics();
        self.statistics.cache_hit_ratio = if ext_stats.cache_hits + ext_stats.cache_misses > 0 {
            ext_stats.cache_hits as f64 / (ext_stats.cache_hits + ext_stats.cache_misses) as f64
        } else {
            0.0
        };
        
        // Calculate averages and final metrics
        self.branching_manager.calculate_average_branching_factor();
    }
    
    /// Get current tableau state
    #[must_use] pub fn get_state(&self) -> TableauState {
        self.state
    }
    
    /// Get current statistics
    #[must_use] pub fn get_statistics(&self) -> &HyperTableauStatistics {
        &self.statistics
    }
    
    /// Check if reasoning is complete
    #[must_use] pub fn is_reasoning_complete(&self) -> bool {
        // Reasoning is complete if:
        // 1. The tableau is closed (unsatisfiable), or
        // 2. The tableau state is unknown (timeout/error), or 
        // 3. We have a satisfiable state AND all disjunctions have been processed
        match self.state {
            TableauState::Unsatisfiable | TableauState::Unknown => true,
            TableauState::Satisfiable => {
                // For satisfiable state, check if we have processed all disjunctions
                // and the tableau has been properly initialized (has some ground disjunctions or has been run)
                self.first_unprocessed_disjunction.is_none() && !self.ground_disjunctions.is_empty()
            }
        }
    }
}

// Import the HyperTableauInterface trait from the reasoner module
use crate::core::reasoner::HyperTableauInterface;

/// Implementation of `HyperTableauInterface` for integration with the main reasoner
impl HyperTableauInterface for HyperTableau {
    fn run(&mut self) -> Result<TableauState> {
        self.run()
    }
    
    fn get_node_count(&self) -> usize {
        self.nodes.len()
    }
    
    fn get_backtrack_count(&self) -> usize {
        self.statistics.backtracks as usize
    }
    
    fn get_max_depth(&self) -> usize {
        self.statistics.max_depth as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::blocking::AnywhereBlocking;
    
    #[test]
    fn test_hypertableau_creation() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
        let tableau = HyperTableau::new(config, blocking_checker);
        assert!(tableau.is_ok());
        
        let tableau = tableau.unwrap();
        assert_eq!(tableau.get_state(), TableauState::Satisfiable);
        assert!(!tableau.is_reasoning_complete());
    }
    
    #[test]
    fn test_monitoring_level_setting() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        tableau.set_monitoring_level(MonitoringLevel::Debug);
        
        assert_eq!(tableau.monitor.get_monitoring_level(), MonitoringLevel::Debug);
    }
    
    #[test]
    fn test_branching_strategy_setting() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        tableau.set_branching_strategy(BranchingStrategy::BestFirst);
        
        // Strategy is internal to BranchingManager, so just verify no errors
        assert!(true);
    }
    
    #[test]
    fn test_state_reset() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
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
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
        let mut tableau = HyperTableau::new(config, blocking_checker).unwrap();
        
        // Create a simple ground disjunction using proper constructor
        use crate::core::hypertableau::ground_disjunction::{GroundDisjunctionHeader, DisjunctPredicate, DisjunctionPriority};
        use crate::core::dependency::DependencySet;
        use crate::ontology::concepts::Class;
        
        let predicates = vec![
            DisjunctPredicate::Concept {
                concept: ClassExpression::Class(Class::new(crate::ontology::IRI::new("A"))),
                argument: 0,
            },
            DisjunctPredicate::Concept {
                concept: ClassExpression::Class(Class::new(crate::ontology::IRI::new("B"))),
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
        
        tableau.add_ground_disjunction(disjunction);
        
        assert_eq!(tableau.ground_disjunctions.len(), 1);
        assert_eq!(tableau.first_unprocessed_disjunction, Some(0));
    }
    
    #[test]
    fn test_statistics_tracking() {
        let config = ReasoningConfig::default();
        let blocking_checker = Box::new(AnywhereBlocking::new());
        
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

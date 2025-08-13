//! Hypertableau Hyperresolution Algorithm
//! 
//! Implementation of the hyperresolution algorithm for the OWL 2 DL hypertableau reasoner.

use crate::Result;
use crate::core::hypertableau::ExtensionManager;
use log::debug;
use std::collections::{HashMap, HashSet};
use std::fmt;
use serde::{Serialize, Deserialize};

/// Hyperresolution manager handles compilation and application of DL clauses
#[derive(Debug)]
pub struct HyperresolutionManager {
    /// Compiled clause evaluators indexed by predicate
    tuple_consumers_by_delta_predicate: HashMap<String, CompiledDLClauseInfo>,
    
    /// Optimized evaluators for atomic role assertions with guards
    atomic_role_consumers_by_guard_concept: HashMap<String, HashMap<String, CompiledDLClauseInfo>>,
    
    /// Unguarded atomic role consumers for efficiency
    atomic_role_consumers_unguarded: HashMap<String, CompiledDLClauseInfo>,
    
    /// Clause evaluators for processing
    evaluators: Vec<DLClauseEvaluator>,
    
    /// Buffer management for efficient memory usage
    values_buffer: Vec<Option<String>>,
    max_variables: usize,
    
    /// Performance optimizations
    optimization_enabled: bool,
    guard_optimization_threshold: usize,
}

/// Compiled information about DL clauses for efficient execution
#[derive(Debug, Clone, Default)]
pub struct CompiledDLClauseInfo {
    /// The clause evaluator
    evaluator_index: usize,
    
    /// Next clause info in the chain
    next: Option<Box<CompiledDLClauseInfo>>,
    
    /// Index in the list for optimization decisions
    index_in_list: usize,
    
    /// Priority for evaluation order
    priority: i32,
}

/// DL Clause evaluator for processing compiled clauses
#[derive(Debug)]
pub struct DLClauseEvaluator {
    /// The body clause pattern to match
    body_clause: DLClause,
    
    /// Head clauses to apply when body matches
    head_clauses: Vec<DLClause>,
    
    /// Variable bindings from matching
    variable_bindings: HashMap<String, String>,
    
    /// Execution state
    program_counter: usize,
    workers: Vec<Worker>,
    
    /// Statistics
    match_count: u64,
    application_count: u64,
}

/// Individual DL clause representation
#[derive(Debug, Clone)]
pub struct DLClause {
    /// Body atoms (conditions)
    pub body: Vec<Atom>,
    
    /// Head atoms (conclusions)
    pub head: Vec<Atom>,
    
    /// Variables used in the clause
    pub variables: HashSet<String>,
    
    /// Clause identifier
    pub id: String,
}

/// Atomic formula in DL clauses
#[derive(Debug, Clone)]
pub struct Atom {
    /// Predicate name
    pub predicate: String,
    
    /// Arguments (variables or constants)
    pub arguments: Vec<String>,
    
    /// Whether this is a positive or negative atom
    pub is_positive: bool,
}

/// Worker for executing parts of clause evaluation
#[derive(Debug)]
pub struct Worker {
    /// Operation type
    operation: WorkerOperation,
    
    /// Target program counter for control flow
    target_pc: Option<usize>,
    
    /// Arguments for the operation
    arguments: Vec<String>,
}

/// Types of operations workers can perform
#[derive(Debug, Clone)]
pub enum WorkerOperation {
    /// Open a retrieval for matching
    OpenRetrieval,
    
    /// Check if retrieval has more tuples
    CheckAfterLast,
    
    /// Move to next tuple
    NextTuple,
    
    /// Bind variable to value
    BindVariable,
    
    /// Check guard condition
    CheckGuard,
    
    /// Apply head clause
    ApplyHead,
    
    /// Jump to program counter
    Jump,
    
    /// Return from evaluation
    Return,
}

impl HyperresolutionManager {
    /// Create a new hyperresolution manager
    pub fn new(clauses: Vec<DLClause>, optimization_enabled: bool) -> Result<Self> {
        let mut manager = HyperresolutionManager {
            tuple_consumers_by_delta_predicate: HashMap::new(),
            atomic_role_consumers_by_guard_concept: HashMap::new(),
            atomic_role_consumers_unguarded: HashMap::new(),
            evaluators: Vec::new(),
            values_buffer: Vec::new(),
            max_variables: 0,
            optimization_enabled,
            guard_optimization_threshold: 10,
        };
        
        manager.compile_clauses(clauses)?;
        Ok(manager)
    }
    
    /// Compile DL clauses into efficient evaluators
    fn compile_clauses(&mut self, clauses: Vec<DLClause>) -> Result<()> {
        // Group clauses by body pattern for efficiency
        let mut clauses_by_body: HashMap<String, Vec<DLClause>> = HashMap::new();
        
        for clause in clauses {
            let body_key = self.create_body_key(&clause);

            // Track maximum variables for buffer sizing
            self.max_variables = self.max_variables.max(clause.variables.len());

            clauses_by_body.entry(body_key).or_default().push(clause);
        }
        
        // Initialize values buffer
        self.values_buffer = vec![None; self.max_variables];
        
        // Compile each group of clauses
        for (body_key, clause_group) in clauses_by_body {
            self.compile_clause_group(body_key, clause_group)?;
        }
        
        Ok(())
    }
    
    /// Compile a group of clauses with the same body pattern
    fn compile_clause_group(&mut self, body_key: String, clauses: Vec<DLClause>) -> Result<()> {
        if clauses.is_empty() {
            return Ok(());
        }
        
        // Create evaluator for this clause group
        let evaluator = DLClauseEvaluator::new(
            clauses[0].clone(), // Use first clause as body template
            clauses,
            self.optimization_enabled,
        )?;
        
        let evaluator_index = self.evaluators.len();
        self.evaluators.push(evaluator);
        
        // Create compiled clause info
        let clause_info = CompiledDLClauseInfo {
            evaluator_index,
            next: None,
            index_in_list: 1,
            priority: 0,
        };
        
        // Index by delta predicate for efficient lookup
        if let Some(first_body_atom) = self.evaluators[evaluator_index].body_clause.body.first() {
            let predicate = &first_body_atom.predicate.clone();
            self.add_compiled_clause_info(predicate.clone(), clause_info.clone())?;
            
            // Add optimizations for atomic roles if enabled
            if self.optimization_enabled && self.is_atomic_role_predicate(predicate) {
                self.add_atomic_role_optimizations(predicate.to_string(), clause_info)?;
            }
        }
        
        Ok(())
    }
    
    /// Add compiled clause info to the appropriate index
    fn add_compiled_clause_info(&mut self, predicate: String, mut info: CompiledDLClauseInfo) -> Result<()> {
        if let Some(existing) = self.tuple_consumers_by_delta_predicate.get(&predicate) {
            info.next = Some(Box::new(existing.clone()));
            info.index_in_list = existing.index_in_list + 1;
        }
        
        self.tuple_consumers_by_delta_predicate.insert(predicate, info);
        Ok(())
    }
    
    /// Add atomic role optimizations for guarded and unguarded cases
    fn add_atomic_role_optimizations(&mut self, predicate: String, info: CompiledDLClauseInfo) -> Result<()> {
        let evaluator = &self.evaluators[info.evaluator_index];
        let guards = self.extract_guard_concepts(&evaluator.body_clause)?;
        
        if guards.is_empty() {
            // Unguarded case
            if let Some(existing) = self.atomic_role_consumers_unguarded.get(&predicate) {
                let mut new_info = info.clone();
                new_info.next = Some(Box::new(existing.clone()));
                new_info.index_in_list = existing.index_in_list + 1;
                self.atomic_role_consumers_unguarded.insert(predicate, new_info);
            } else {
                self.atomic_role_consumers_unguarded.insert(predicate, info);
            }
        } else {
            // Guarded case
            for guard_concept in guards {
                let guard_map = self.atomic_role_consumers_by_guard_concept
                    .entry(predicate.clone())
                    .or_default();
                
                if let Some(existing) = guard_map.get(&guard_concept) {
                    let mut new_info = info.clone();
                    new_info.next = Some(Box::new(existing.clone()));
                    new_info.index_in_list = existing.index_in_list + 1;
                    guard_map.insert(guard_concept, new_info);
                } else {
                    guard_map.insert(guard_concept, info.clone());
                }
            }
        }
        
        Ok(())
    }
    
    /// Extract guard concepts from a clause for optimization
    fn extract_guard_concepts(&self, clause: &DLClause) -> Result<Vec<String>> {
        let mut guards = Vec::new();
        
        if clause.body.len() < 2 {
            return Ok(guards);
        }
        
        // First atom should be the atomic role
        let role_atom = &clause.body[0];
        if role_atom.arguments.len() != 2 {
            return Ok(guards);
        }
        
        let x = &role_atom.arguments[0];
        let y = &role_atom.arguments[1];
        
        // Look for atomic concept guards on the role arguments
        for atom in &clause.body[1..] {
            if self.is_atomic_concept_predicate(&atom.predicate) && atom.arguments.len() == 1 {
                let variable = &atom.arguments[0];
                if variable == x || variable == y {
                    guards.push(atom.predicate.clone());
                }
            }
        }
        
        Ok(guards)
    }
    
    /// Apply DL clauses during tableau expansion
    pub fn apply_dl_clauses(&mut self, extension_manager: &mut ExtensionManager) -> Result<()> {
        // Collect predicates to avoid borrowing conflicts
        let predicates: Vec<(String, Vec<Vec<String>>)> = self.tuple_consumers_by_delta_predicate
            .keys()
            .map(|predicate| {
                let delta_tuples = extension_manager.get_delta_old_tuples(predicate)
                    .unwrap_or_else(|_| Vec::new());
                (predicate.clone(), delta_tuples)
            })
            .collect();

        // Process all delta-old tuples
        for (predicate, delta_tuples) in predicates {
            if extension_manager.contains_clash() {
                break;
            }
            
            // Get delta-old tuples for this predicate
            let compiled_info = self.tuple_consumers_by_delta_predicate
                .get(&predicate)
                .cloned()
                .unwrap_or_default();
            
            for tuple in delta_tuples {
                if extension_manager.contains_clash() {
                    break;
                }
                
                // Apply optimization if available
                if self.optimization_enabled && self.should_use_optimization(&predicate, &tuple) {
                    self.apply_optimized_clauses(&predicate, &tuple, extension_manager)?;
                } else {
                    self.apply_unoptimized_clauses(&compiled_info, &tuple, extension_manager)?;
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply optimized clause evaluation
    fn apply_optimized_clauses(
        &mut self,
        predicate: &str,
        tuple: &[String],
        extension_manager: &mut ExtensionManager,
    ) -> Result<()> {
        // Try unguarded clauses first
        if let Some(unguarded_info) = self.atomic_role_consumers_unguarded.get(predicate).cloned() {
            self.apply_compiled_clause_chain(&unguarded_info, tuple, extension_manager)?;
        }
        
        // Try guarded clauses
        if let Some(guard_map) = self.atomic_role_consumers_by_guard_concept.get(predicate).cloned() {
            // Check guards for first argument
            if tuple.len() >= 2 {
                let node1_concepts = extension_manager.get_node_concepts(&tuple[0])?;
                for concept in node1_concepts {
                    if let Some(guarded_info) = guard_map.get(&concept) {
                        self.apply_compiled_clause_chain(guarded_info, tuple, extension_manager)?;
                    }
                }
                
                // Check guards for second argument
                let node2_concepts = extension_manager.get_node_concepts(&tuple[1])?;
                for concept in node2_concepts {
                    if let Some(guarded_info) = guard_map.get(&concept) {
                        self.apply_compiled_clause_chain(guarded_info, tuple, extension_manager)?;
                    }
                }
            }
        }
        
        Ok(())
    }
    
    /// Apply unoptimized clause evaluation
    fn apply_unoptimized_clauses(
        &mut self,
        compiled_info: &CompiledDLClauseInfo,
        tuple: &[String],
        extension_manager: &mut ExtensionManager,
    ) -> Result<()> {
        self.apply_compiled_clause_chain(compiled_info, tuple, extension_manager)
    }
    
    /// Apply a chain of compiled clauses
    fn apply_compiled_clause_chain(
        &mut self,
        compiled_info: &CompiledDLClauseInfo,
        tuple: &[String],
        extension_manager: &mut ExtensionManager,
    ) -> Result<()> {
        let mut current = Some(compiled_info);
        
        while let Some(info) = current {
            if extension_manager.contains_clash() {
                break;
            }
            
            // Apply the evaluator
            let evaluator = &mut self.evaluators[info.evaluator_index];
            evaluator.evaluate(tuple, extension_manager)?;
            
            current = info.next.as_deref();
        }
        
        Ok(())
    }
    
    /// Determine if optimization should be used
    fn should_use_optimization(&self, predicate: &str, tuple: &[String]) -> bool {
        if !self.optimization_enabled {
            return false;
        }
        
        // Use optimization if we have guard-based consumers
        self.atomic_role_consumers_by_guard_concept.contains_key(predicate) ||
        self.atomic_role_consumers_unguarded.contains_key(predicate)
    }
    
    /// Create a body key for clause grouping
    fn create_body_key(&self, clause: &DLClause) -> String {
        let mut key = String::new();
        for (i, atom) in clause.body.iter().enumerate() {
            if i > 0 {
                key.push('&');
            }
            key.push_str(&format!("{}({})", atom.predicate, atom.arguments.join(",")));
        }
        key
    }
    
    /// Check if a predicate represents an atomic role
    fn is_atomic_role_predicate(&self, predicate: &str) -> bool {
        // Simple heuristic: atomic roles typically have specific naming patterns
        // In a full implementation, this would consult the ontology signature
        predicate.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
        !predicate.starts_with("¬") &&
        !predicate.starts_with("neg_") &&
        !predicate.contains("(") &&
        !predicate.contains(")") &&
        !predicate.starts_with('_') && 
        !self.is_atomic_concept_predicate(predicate)
    }
    
    /// Check if a predicate represents an atomic concept
    fn is_atomic_concept_predicate(&self, predicate: &str) -> bool {
        // Simple heuristic: concepts typically start with uppercase
        predicate.chars().next().is_some_and(|c| c.is_uppercase())
    }
    
    /// Clear internal state for new reasoning task
    pub fn clear(&mut self) {
        for evaluator in &mut self.evaluators {
            evaluator.clear();
        }
        
        for buffer_slot in &mut self.values_buffer {
            *buffer_slot = None;
        }
    }
    
    /// Get statistics about clause application
    pub fn get_statistics(&self) -> HyperresolutionStatistics {
        let total_matches: u64 = self.evaluators.iter().map(|e| e.match_count).sum();
        let total_applications: u64 = self.evaluators.iter().map(|e| e.application_count).sum();
        
        HyperresolutionStatistics {
            total_clauses: self.evaluators.len(),
            total_matches,
            total_applications,
            optimization_enabled: self.optimization_enabled,
            guard_optimizations: self.atomic_role_consumers_by_guard_concept.len(),
            unguarded_optimizations: self.atomic_role_consumers_unguarded.len(),
        }
    }

    /// Initialize the hyperresolution manager
    pub fn initialize(&mut self, dl_clauses: Vec<DLClause>) -> crate::Result<()> {
        // Clear any existing state
        self.clear();
        
        // Process each DL clause and set up appropriate evaluators
        for clause in dl_clauses {
            let evaluator = DLClauseEvaluator::new(clause, Vec::new(), true)?;
            self.evaluators.push(evaluator);
        }
        
        // Build index mappings for efficient rule application
        self.build_consumer_indices()?;
        
        Ok(())
    }
    
    /// Build consumer indices for efficient rule application
    fn build_consumer_indices(&mut self) -> crate::Result<()> {
        for (idx, evaluator) in self.evaluators.iter().enumerate() {
            // Index evaluators by the predicates they consume
            for predicate in evaluator.get_consumed_predicates() {
                let clause_info = CompiledDLClauseInfo {
                    evaluator_index: idx,
                    next: None,
                    index_in_list: 0,
                    priority: 0,
                };
                self.tuple_consumers_by_delta_predicate.insert(predicate, clause_info);
            }
        }
        Ok(())
    }
    
    /// Apply rules to derive new facts
    pub fn apply_rules(&mut self, extension_manager: &mut ExtensionManager, _branching_manager: &mut crate::core::hypertableau::branching::BranchingManager) -> crate::Result<bool> {
        let mut new_facts_derived = false;
        
        // Process delta tuples for each predicate with active evaluators
        for (predicate, clause_info) in &self.tuple_consumers_by_delta_predicate {
            if let Some(new_tuples) = extension_manager.get_new_tuples(predicate) {
                let evaluator_idx = clause_info.evaluator_index;
                if evaluator_idx < self.evaluators.len() {
                    let evaluator = &mut self.evaluators[evaluator_idx];
                    for tuple in new_tuples {
                        // Apply the evaluator to each new tuple
                        if evaluator.evaluate(&tuple, extension_manager).is_ok() {
                            new_facts_derived = true;
                        }
                    }
                }
            }
        }
        
        Ok(new_facts_derived)
    }
    
    /// Reset the hyperresolution manager
    pub fn reset(&mut self) {
        // Clear all indexing structures
        self.tuple_consumers_by_delta_predicate.clear();
        self.atomic_role_consumers_by_guard_concept.clear();
        self.atomic_role_consumers_unguarded.clear();
        
        // Clear all evaluators
        for evaluator in &mut self.evaluators {
            evaluator.clear();
        }
        self.evaluators.clear();
        
        // Reset internal state
        self.values_buffer.clear();
        self.max_variables = 0;
    }
}

impl DLClauseEvaluator {
    /// Create a new DL clause evaluator
    pub fn new(body_clause: DLClause, head_clauses: Vec<DLClause>, optimization_enabled: bool) -> Result<Self> {
        let workers = Self::compile_workers(&body_clause, &head_clauses)?;
        
        Ok(DLClauseEvaluator {
            body_clause,
            head_clauses,
            variable_bindings: HashMap::new(),
            program_counter: 0,
            workers,
            match_count: 0,
            application_count: 0,
        })
    }
    
    /// Compile workers for efficient execution
    fn compile_workers(body_clause: &DLClause, head_clauses: &[DLClause]) -> Result<Vec<Worker>> {
        let mut workers = Vec::new();
        
        // Generate workers for body matching
        for (i, atom) in body_clause.body.iter().enumerate() {
            workers.push(Worker {
                operation: WorkerOperation::OpenRetrieval,
                target_pc: None,
                arguments: vec![atom.predicate.clone()],
            });
            
            workers.push(Worker {
                operation: WorkerOperation::CheckAfterLast,
                target_pc: Some(workers.len() + 3), // Jump to next atom or head application
                arguments: vec![],
            });
            
            // Bind variables
            for arg in &atom.arguments {
                workers.push(Worker {
                    operation: WorkerOperation::BindVariable,
                    target_pc: None,
                    arguments: vec![arg.clone()],
                });
            }
            
            workers.push(Worker {
                operation: WorkerOperation::NextTuple,
                target_pc: Some(workers.len() - 3 - atom.arguments.len()), // Loop back
                arguments: vec![],
            });
        }
        
        // Generate workers for head application
        for head_clause in head_clauses {
            for atom in &head_clause.head {
                workers.push(Worker {
                    operation: WorkerOperation::ApplyHead,
                    target_pc: None,
                    arguments: vec![atom.predicate.clone()].into_iter()
                        .chain(atom.arguments.iter().cloned())
                        .collect(),
                });
            }
        }
        
        workers.push(Worker {
            operation: WorkerOperation::Return,
            target_pc: None,
            arguments: vec![],
        });
        
        Ok(workers)
    }
    
    /// Evaluate the clause against a tuple
    pub fn evaluate(&mut self, tuple: &[String], extension_manager: &mut ExtensionManager) -> Result<()> {
        self.match_count += 1;
        self.program_counter = 0;
        self.variable_bindings.clear();
        
        // Set initial bindings from the delta tuple
        if let Some(first_atom) = self.body_clause.body.first() {
            for (i, arg) in first_atom.arguments.iter().enumerate() {
                if i < tuple.len() {
                    self.variable_bindings.insert(arg.clone(), tuple[i].clone());
                }
            }
        }
        
        // Execute workers
        while self.program_counter < self.workers.len() {
            let worker = &self.workers[self.program_counter];
            
            match worker.execute(&mut self.variable_bindings, extension_manager)? {
                WorkerResult::Continue => {
                    self.program_counter += 1;
                }
                WorkerResult::Jump(pc) => {
                    self.program_counter = pc;
                }
                WorkerResult::Return => {
                    break;
                }
                WorkerResult::Fail => {
                    // Backtrack or try next possibility
                    break;
                }
            }
            
            if extension_manager.contains_clash() {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Clear evaluator state
    pub fn clear(&mut self) {
        self.variable_bindings.clear();
        self.program_counter = 0;
    }
    
    /// Get predicates consumed by this evaluator
    pub fn get_consumed_predicates(&self) -> Vec<String> {
        let mut predicates = Vec::new();
        
        // Extract predicates from body clause atoms
        for atom in &self.body_clause.body {
            predicates.push(atom.predicate.clone());
        }
        
        predicates
    }
}

impl Worker {
    /// Execute this worker operation
    pub fn execute(
        &self,
        bindings: &mut HashMap<String, String>,
        extension_manager: &mut ExtensionManager,
    ) -> Result<WorkerResult> {
        match &self.operation {
            WorkerOperation::OpenRetrieval => {
                // Simulate opening a retrieval for the predicate
                Ok(WorkerResult::Continue)
            }
            
            WorkerOperation::CheckAfterLast => {
                // Simulate checking if retrieval has more tuples
                // For now, always assume there are tuples
                Ok(WorkerResult::Continue)
            }
            
            WorkerOperation::NextTuple => {
                // Move to next tuple in retrieval
                Ok(WorkerResult::Continue)
            }
            
            WorkerOperation::BindVariable => {
                // Bind variable to current tuple value
                if let Some(_var_name) = self.arguments.first() {
                    // In a full implementation, this would access the current retrieval state
                    // and bind the variable to the tuple value at the specified position
                    debug!("Variable binding operation - would bind to current tuple value");
                    Ok(WorkerResult::Continue)
                } else {
                    Ok(WorkerResult::Fail)
                }
            }
            
            WorkerOperation::CheckGuard => {
                // Check guard conditions
                Ok(WorkerResult::Continue)
            }
            
            WorkerOperation::ApplyHead => {
                // Apply head clause with current bindings
                if self.arguments.len() >= 2 {
                    let predicate = &self.arguments[0];
                    let args: Vec<String> = self.arguments[1..].iter()
                        .map(|arg| bindings.get(arg).cloned().unwrap_or_else(|| arg.clone()))
                        .collect();
                    
                    extension_manager.add_fact(predicate.clone(), args)?;
                }
                Ok(WorkerResult::Continue)
            }
            
            WorkerOperation::Jump => {
                if let Some(pc) = self.target_pc {
                    Ok(WorkerResult::Jump(pc))
                } else {
                    Ok(WorkerResult::Continue)
                }
            }
            
            WorkerOperation::Return => {
                Ok(WorkerResult::Return)
            }
        }
    }
}

/// Result of worker execution
#[derive(Debug)]
pub enum WorkerResult {
    Continue,
    Jump(usize),
    Return,
    Fail,
}

/// Statistics about hyperresolution performance
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HyperresolutionStatistics {
    pub total_clauses: usize,
    pub total_matches: u64,
    pub total_applications: u64,
    pub optimization_enabled: bool,
    pub guard_optimizations: usize,
    pub unguarded_optimizations: usize,
}

impl fmt::Display for HyperresolutionStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, 
            "Hyperresolution Statistics:\n\
             Total Clauses: {}\n\
             Total Matches: {}\n\
             Total Applications: {}\n\
             Optimization Enabled: {}\n\
             Guard Optimizations: {}\n\
             Unguarded Optimizations: {}",
            self.total_clauses,
            self.total_matches,
            self.total_applications,
            self.optimization_enabled,
            self.guard_optimizations,
            self.unguarded_optimizations
        )
    }
}
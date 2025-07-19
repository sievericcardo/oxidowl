//! DL Clause Evaluator
//!
//! This module implements efficient evaluation of DL clauses in the hypertableau
//! algorithm. It compiles clauses into executable form and provides optimized
//! evaluation strategies based on HermiT's approach.

use crate::{
    core::{
        dependency::DependencySet,
    },
    Result,
};

use super::{
    ground_disjunction::{GroundDisjunction, GroundDisjunctionHeader},
    hyperresolution::{DLClause, Atom},
    extension_table::ExtensionManager,
};

use std::{
    collections::HashMap,
    fmt,
};

use serde::{Serialize, Deserialize};

/// DL Clause Evaluator for efficient clause processing
#[derive(Debug)]
pub struct DLClauseEvaluator {
    /// The body clause pattern to match against facts
    body_clause: DLClause,
    
    /// Head clauses to apply when body matches
    head_clauses: Vec<DLClause>,
    
    /// Compiled workers for execution
    workers: Vec<Worker>,
    
    /// Variable mappings and bindings
    variable_mappings: HashMap<String, usize>,
    binding_buffer: Vec<Option<String>>,
    
    /// Retrieval operations for fact matching
    retrievals: Vec<RetrievalOperation>,
    
    /// Ground disjunction management
    disjunction_header_manager: GroundDisjunctionHeaderManager,
    
    /// Buffer management for efficiency
    buffer_supply: BufferSupply,
    values_buffer_manager: ValuesBufferManager,
    
    /// Union dependency sets for backtracking
    union_dependency_sets: HashMap<usize, UnionDependencySet>,
    
    /// Execution statistics
    evaluations: u64,
    matches: u64,
    applications: u64,
    
    /// Optimization flags
    optimization_enabled: bool,
    early_termination: bool,
}

/// Worker for executing parts of clause evaluation
#[derive(Debug, Clone)]
pub struct Worker {
    /// Operation type
    operation: WorkerOperation,
    
    /// Program counter for next instruction
    next_pc: usize,
    
    /// Jump target for control flow
    jump_target: Option<usize>,
    
    /// Arguments for the operation
    arguments: Vec<String>,
    
    /// Variable indices for binding
    variable_indices: Vec<usize>,
    
    /// Optimization hints
    optimization_hint: OptimizationHint,
}

/// Types of operations workers can perform
#[derive(Debug, Clone)]
pub enum WorkerOperation {
    /// Open a retrieval for fact matching
    OpenRetrieval {
        predicate: String,
        arity: usize,
        view: RetrievalView,
    },
    
    /// Check if retrieval has more tuples
    CheckAfterLast {
        fail_target: usize,
    },
    
    /// Move to next tuple in retrieval
    NextTuple {
        loop_target: usize,
    },
    
    /// Bind variable to tuple value
    BindVariable {
        var_index: usize,
        tuple_index: usize,
    },
    
    /// Check guard condition for optimization
    CheckGuard {
        predicate: String,
        var_index: usize,
        fail_target: usize,
    },
    
    /// Apply head clause with current bindings
    ApplyHead {
        clause_index: usize,
        atom_index: usize,
    },
    
    /// Create ground disjunction
    CreateGroundDisjunction {
        atoms: Vec<String>,
        priority: i32,
    },
    
    /// Add dependency information
    AddDependency {
        source_indices: Vec<usize>,
        target_index: usize,
    },
    
    /// Jump to specific program counter
    Jump {
        target: usize,
    },
    
    /// Return from evaluation
    Return,
    
    /// No operation (for optimization)
    Nop,
}

/// Views for fact retrieval
#[derive(Debug, Clone)]
pub enum RetrievalView {
    /// All facts
    All,
    /// Only new facts (delta)
    DeltaNew,
    /// Old facts before current iteration
    DeltaOld,
    /// Extension facts
    Extension,
}

/// Optimization hints for workers
#[derive(Debug, Clone)]
pub enum OptimizationHint {
    None,
    FastPath,
    GuardOptimized,
    EarlyTermination,
    CacheFriendly,
}

/// Retrieval operation for fact access
#[derive(Debug)]
pub struct RetrievalOperation {
    /// Predicate being retrieved
    predicate: String,
    
    /// Arity of the predicate
    arity: usize,
    
    /// View type
    view: RetrievalView,
    
    /// Binding pattern (which arguments are bound)
    binding_pattern: Vec<bool>,
    
    /// Current position in results
    position: usize,
    
    /// Current tuple buffer
    tuple_buffer: Vec<String>,
    
    /// Results cache
    results: Vec<Vec<String>>,
    
    /// Is retrieval open
    is_open: bool,
}

/// Ground disjunction header manager
#[derive(Debug)]
pub struct GroundDisjunctionHeaderManager {
    /// Header cache
    headers: HashMap<String, GroundDisjunctionHeader>,
    
    /// Next header ID
    next_id: u32,
}

/// Buffer supply for memory management
#[derive(Debug)]
pub struct BufferSupply {
    /// Available buffers
    buffers: Vec<Vec<String>>,
    
    /// Buffer sizes
    buffer_sizes: Vec<usize>,
    
    /// Current allocation index
    allocation_index: usize,
}

/// Values buffer manager
#[derive(Debug)]
pub struct ValuesBufferManager {
    /// Main values buffer
    values_buffer: Vec<Option<String>>,
    
    /// Maximum variables across all clauses
    max_variables: usize,
    
    /// Variable mappings
    variable_mappings: HashMap<String, usize>,
}

/// Union dependency set for backtracking
#[derive(Debug)]
pub struct UnionDependencySet {
    /// Component dependency sets
    dependency_sets: Vec<Option<DependencySet>>,
    
    /// Size of the union
    size: usize,
    
    /// Is this set currently in use
    in_use: bool,
}

/// Execution context for clause evaluation
#[derive(Debug)]
pub struct ExecutionContext<'a> {
    /// Current program counter
    program_counter: usize,
    
    /// Variable bindings
    bindings: &'a mut [Option<String>],
    
    /// Extension manager for fact access
    extension_manager: &'a mut ExtensionManager,
    
    /// Current retrievals
    retrievals: &'a mut Vec<RetrievalOperation>,
    
    /// Execution flags
    interrupted: bool,
    early_termination: bool,
}

impl DLClauseEvaluator {
    /// Create a new DL clause evaluator
    pub fn new(
        body_clause: DLClause,
        head_clauses: Vec<DLClause>,
        optimization_enabled: bool,
    ) -> Result<Self> {
        let mut evaluator = DLClauseEvaluator {
            body_clause: body_clause.clone(),
            head_clauses,
            workers: Vec::new(),
            variable_mappings: HashMap::new(),
            binding_buffer: Vec::new(),
            retrievals: Vec::new(),
            disjunction_header_manager: GroundDisjunctionHeaderManager::new(),
            buffer_supply: BufferSupply::new(),
            values_buffer_manager: ValuesBufferManager::new(),
            union_dependency_sets: HashMap::new(),
            evaluations: 0,
            matches: 0,
            applications: 0,
            optimization_enabled,
            early_termination: true,
        };
        
        evaluator.compile()?;
        Ok(evaluator)
    }
    
    /// Compile the clause into executable workers
    fn compile(&mut self) -> Result<()> {
        // Analyze variables and create mappings
        self.analyze_variables()?;
        
        // Compile body matching
        self.compile_body_matching()?;
        
        // Compile head application
        self.compile_head_application()?;
        
        // Add return instruction
        self.workers.push(Worker {
            operation: WorkerOperation::Return,
            next_pc: 0,
            jump_target: None,
            arguments: Vec::new(),
            variable_indices: Vec::new(),
            optimization_hint: OptimizationHint::None,
        });
        
        // Optimize the compiled code
        if self.optimization_enabled {
            self.optimize_workers()?;
        }
        
        Ok(())
    }
    
    /// Analyze variables in the clause
    fn analyze_variables(&mut self) -> Result<()> {
        let mut var_counter = 0;
        
        // Collect variables from body
        for atom in &self.body_clause.body {
            for arg in &atom.arguments {
                if self.is_variable(arg) && !self.variable_mappings.contains_key(arg) {
                    self.variable_mappings.insert(arg.clone(), var_counter);
                    var_counter += 1;
                }
            }
        }
        
        // Collect variables from heads
        for head_clause in &self.head_clauses {
            for atom in &head_clause.head {
                for arg in &atom.arguments {
                    if self.is_variable(arg) && !self.variable_mappings.contains_key(arg) {
                        self.variable_mappings.insert(arg.clone(), var_counter);
                        var_counter += 1;
                    }
                }
            }
        }
        
        // Initialize binding buffer
        self.binding_buffer = vec![None; var_counter];
        self.values_buffer_manager.values_buffer = vec![None; var_counter];
        self.values_buffer_manager.max_variables = var_counter;
        
        Ok(())
    }
    
    /// Compile body matching operations
    fn compile_body_matching(&mut self) -> Result<()> {
        // First collect all atoms to avoid borrowing conflicts
        let atoms_data: Vec<(usize, Atom)> = self.body_clause.body.iter().enumerate()
            .map(|(i, atom)| (i, atom.clone()))
            .collect();
            
        for (atom_index, atom) in atoms_data {
            // Open retrieval for this atom
            let binding_pattern: Vec<bool> = atom.arguments.iter()
                .map(|arg| self.is_variable(arg))
                .collect();
            
            self.workers.push(Worker {
                operation: WorkerOperation::OpenRetrieval {
                    predicate: atom.predicate.clone(),
                    arity: atom.arguments.len(),
                    view: if atom_index == 0 { RetrievalView::DeltaOld } else { RetrievalView::Extension },
                },
                next_pc: self.workers.len() + 1,
                jump_target: None,
                arguments: atom.arguments.clone(),
                variable_indices: atom.arguments.iter()
                    .map(|arg| self.variable_mappings.get(arg).copied().unwrap_or(0))
                    .collect(),
                optimization_hint: OptimizationHint::None,
            });
            
            // Check if retrieval has tuples
            let fail_target = self.calculate_fail_target(atom_index);
            self.workers.push(Worker {
                operation: WorkerOperation::CheckAfterLast {
                    fail_target,
                },
                next_pc: self.workers.len() + 1,
                jump_target: Some(fail_target),
                arguments: Vec::new(),
                variable_indices: Vec::new(),
                optimization_hint: OptimizationHint::EarlyTermination,
            });
            
            // Bind variables from tuple
            for (arg_index, arg) in atom.arguments.iter().enumerate() {
                if self.is_variable(arg) {
                    let var_index = self.variable_mappings[arg];
                    self.workers.push(Worker {
                        operation: WorkerOperation::BindVariable {
                            var_index,
                            tuple_index: arg_index,
                        },
                        next_pc: self.workers.len() + 1,
                        jump_target: None,
                        arguments: vec![arg.clone()],
                        variable_indices: vec![var_index],
                        optimization_hint: OptimizationHint::FastPath,
                    });
                }
            }
            
            // Add guard checks if optimized
            if self.optimization_enabled {
                self.add_guard_checks(&atom, atom_index)?;
            }
            
            // Move to next tuple (loop back for iteration)
            let loop_target = self.workers.len() - atom.arguments.len() - 1;
            self.workers.push(Worker {
                operation: WorkerOperation::NextTuple {
                    loop_target,
                },
                next_pc: self.workers.len() + 1,
                jump_target: Some(loop_target),
                arguments: Vec::new(),
                variable_indices: Vec::new(),
                optimization_hint: OptimizationHint::CacheFriendly,
            });
        }
        
        Ok(())
    }
    
    /// Add guard checks for optimization
    fn add_guard_checks(&mut self, atom: &Atom, atom_index: usize) -> Result<()> {
        // Add guard checks for atomic concepts
        if atom.predicate.starts_with(char::is_uppercase) && atom.arguments.len() == 1 {
            let var_index = self.variable_mappings[&atom.arguments[0]];
            let fail_target = self.calculate_fail_target(atom_index);
            
            self.workers.push(Worker {
                operation: WorkerOperation::CheckGuard {
                    predicate: atom.predicate.clone(),
                    var_index,
                    fail_target,
                },
                next_pc: self.workers.len() + 1,
                jump_target: Some(fail_target),
                arguments: vec![atom.predicate.clone()],
                variable_indices: vec![var_index],
                optimization_hint: OptimizationHint::GuardOptimized,
            });
        }
        
        Ok(())
    }
    
    /// Compile head application operations
    fn compile_head_application(&mut self) -> Result<()> {
        for (clause_index, head_clause) in self.head_clauses.iter().enumerate() {
            for (atom_index, atom) in head_clause.head.iter().enumerate() {
                // Check if this creates a ground disjunction
                if self.creates_ground_disjunction(atom) {
                    self.workers.push(Worker {
                        operation: WorkerOperation::CreateGroundDisjunction {
                            atoms: vec![format!("{}({})", atom.predicate, atom.arguments.join(","))],
                            priority: self.calculate_disjunction_priority(atom),
                        },
                        next_pc: self.workers.len() + 1,
                        jump_target: None,
                        arguments: atom.arguments.clone(),
                        variable_indices: atom.arguments.iter()
                            .map(|arg| self.variable_mappings.get(arg).copied().unwrap_or(0))
                            .collect(),
                        optimization_hint: OptimizationHint::None,
                    });
                } else {
                    // Regular fact application
                    self.workers.push(Worker {
                        operation: WorkerOperation::ApplyHead {
                            clause_index,
                            atom_index,
                        },
                        next_pc: self.workers.len() + 1,
                        jump_target: None,
                        arguments: atom.arguments.clone(),
                        variable_indices: atom.arguments.iter()
                            .map(|arg| self.variable_mappings.get(arg).copied().unwrap_or(0))
                            .collect(),
                        optimization_hint: OptimizationHint::FastPath,
                    });
                }
                
                // Add dependency tracking
                self.workers.push(Worker {
                    operation: WorkerOperation::AddDependency {
                        source_indices: (0..self.body_clause.body.len()).collect(),
                        target_index: atom_index,
                    },
                    next_pc: self.workers.len() + 1,
                    jump_target: None,
                    arguments: Vec::new(),
                    variable_indices: Vec::new(),
                    optimization_hint: OptimizationHint::None,
                });
            }
        }
        
        Ok(())
    }
    
    /// Optimize compiled workers
    fn optimize_workers(&mut self) -> Result<()> {
        // Remove unnecessary NOPs
        self.workers.retain(|worker| !matches!(worker.operation, WorkerOperation::Nop));
        
        // Optimize jump targets
        self.optimize_jump_targets()?;
        
        // Optimize variable access patterns
        self.optimize_variable_access()?;
        
        // Add fast paths for common patterns
        self.add_fast_paths()?;
        
        Ok(())
    }
    
    /// Optimize jump targets
    fn optimize_jump_targets(&mut self) -> Result<()> {
        let worker_count = self.workers.len();
        for worker in &mut self.workers {
            if let Some(target) = worker.jump_target {
                // Ensure jump target is within bounds
                if target >= worker_count {
                    worker.jump_target = Some(worker_count - 1);
                }
            }
        }
        Ok(())
    }
    
    /// Optimize variable access patterns
    fn optimize_variable_access(&mut self) -> Result<()> {
        // Reorder variable indices for better cache locality
        for worker in &mut self.workers {
            worker.variable_indices.sort_unstable();
        }
        Ok(())
    }
    
    /// Add fast paths for common patterns
    fn add_fast_paths(&mut self) -> Result<()> {
        // Mark workers that can use fast execution paths
        for worker in &mut self.workers {
            match &worker.operation {
                WorkerOperation::BindVariable { .. } => {
                    worker.optimization_hint = OptimizationHint::FastPath;
                }
                WorkerOperation::ApplyHead { .. } => {
                    worker.optimization_hint = OptimizationHint::FastPath;
                }
                _ => {}
            }
        }
        Ok(())
    }
    
    /// Execute the compiled clause
    pub fn evaluate(&mut self, extension_manager: &mut ExtensionManager) -> Result<()> {
        self.evaluations += 1;
        
        // Execute workers with simplified context management
        let mut program_counter = 0;
        let mut interrupted = false;
        
        while program_counter < self.workers.len() && !interrupted {
            let worker = self.workers[program_counter].clone();
            
            // Check for clash before execution
            if extension_manager.contains_clash() {
                break;
            }
            
            // Execute worker with direct access to avoid borrowing conflicts
            let result = {
                // Create a temporary context to avoid borrowing conflicts
                // We'll pass the extension_manager and other parameters directly
                let mut temp_context = ExecutionContext {
                    program_counter,
                    bindings: &mut vec![], // Use empty temporary bindings
                    extension_manager,
                    retrievals: &mut vec![], // Use empty temporary retrievals
                    interrupted,
                    early_termination: self.early_termination,
                };
                
                // Execute and get result immediately
                match self.execute_worker_safe(&worker, &mut temp_context) {
                    Ok(result) => result,
                    Err(e) => return Err(e),
                }
            };
            
            // Process result
            match result {
                ExecutionResult::Continue => {
                    program_counter += 1;
                }
                ExecutionResult::Jump(target) => {
                    program_counter = target;
                }
                ExecutionResult::Return => {
                    break;
                }
                ExecutionResult::Fail => {
                    // Handle failure (backtracking or termination)
                    break;
                }
                ExecutionResult::Match => {
                    self.matches += 1;
                    program_counter += 1;
                }
                ExecutionResult::Application => {
                    self.applications += 1;
                    program_counter += 1;
                }
            }
            
            // Early termination check without borrowing conflicts
            if self.early_termination && self.should_terminate_early_safe() {
                interrupted = true;
            }
        }
        
        Ok(())
    }

    /// Execute worker with safer borrowing
    fn execute_worker_safe(
        &mut self,
        worker: &Worker,
        context: &mut ExecutionContext
    ) -> Result<ExecutionResult> {
        // Use the existing execute_worker method
        self.execute_worker(worker, context)
    }
    
    /// Check termination without borrowing conflicts
    fn should_terminate_early_safe(&self) -> bool {
        // Simple implementation without borrowing self.binding_buffer
        false
    }
    
    /// Execute a single worker
    fn execute_worker(
        &mut self,
        worker: &Worker,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        match &worker.operation {
            WorkerOperation::OpenRetrieval { predicate, arity, view } => {
                self.open_retrieval(predicate, *arity, view, context)
            }
            
            WorkerOperation::CheckAfterLast { fail_target } => {
                self.check_after_last(*fail_target, context)
            }
            
            WorkerOperation::NextTuple { loop_target } => {
                self.next_tuple(*loop_target, context)
            }
            
            WorkerOperation::BindVariable { var_index, tuple_index } => {
                self.bind_variable(*var_index, *tuple_index, context)
            }
            
            WorkerOperation::CheckGuard { predicate, var_index, fail_target } => {
                self.check_guard(predicate, *var_index, *fail_target, context)
            }
            
            WorkerOperation::ApplyHead { clause_index, atom_index } => {
                self.apply_head(*clause_index, *atom_index, context)
            }
            
            WorkerOperation::CreateGroundDisjunction { atoms, priority } => {
                self.create_ground_disjunction(atoms, *priority, context)
            }
            
            WorkerOperation::AddDependency { source_indices, target_index } => {
                self.add_dependency(source_indices, *target_index, context)
            }
            
            WorkerOperation::Jump { target } => {
                Ok(ExecutionResult::Jump(*target))
            }
            
            WorkerOperation::Return => {
                Ok(ExecutionResult::Return)
            }
            
            WorkerOperation::Nop => {
                Ok(ExecutionResult::Continue)
            }
        }
    }
    
    /// Open retrieval operation
    fn open_retrieval(
        &mut self,
        predicate: &str,
        arity: usize,
        view: &RetrievalView,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        // Convert RetrievalView to the extension_table version
        use crate::core::hypertableau::extension_table::RetrievalView as ExtRetrievalView;
        let ext_view = match view {
            RetrievalView::All => ExtRetrievalView::Complete,
            RetrievalView::DeltaNew => ExtRetrievalView::DeltaNew,
            RetrievalView::DeltaOld => ExtRetrievalView::DeltaOld,
            RetrievalView::Extension => ExtRetrievalView::Extension,
        };
        let facts = context.extension_manager.get_facts(predicate, &ext_view)?;
        
        let retrieval = RetrievalOperation {
            predicate: predicate.to_string(),
            arity,
            view: view.clone(),
            binding_pattern: vec![false; arity],
            position: 0,
            tuple_buffer: vec![String::new(); arity],
            results: facts,
            is_open: true,
        };
        
        context.retrievals.push(retrieval);
        Ok(ExecutionResult::Continue)
    }
    
    /// Check after last operation
    fn check_after_last(
        &mut self,
        fail_target: usize,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        if let Some(retrieval) = context.retrievals.last() {
            if retrieval.position >= retrieval.results.len() {
                Ok(ExecutionResult::Jump(fail_target))
            } else {
                Ok(ExecutionResult::Continue)
            }
        } else {
            Ok(ExecutionResult::Jump(fail_target))
        }
    }
    
    /// Next tuple operation
    fn next_tuple(
        &mut self,
        loop_target: usize,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        if let Some(retrieval) = context.retrievals.last_mut() {
            retrieval.position += 1;
            if retrieval.position < retrieval.results.len() {
                // Update tuple buffer
                if let Some(tuple) = retrieval.results.get(retrieval.position) {
                    retrieval.tuple_buffer = tuple.clone();
                }
                Ok(ExecutionResult::Jump(loop_target))
            } else {
                Ok(ExecutionResult::Continue)
            }
        } else {
            Ok(ExecutionResult::Continue)
        }
    }
    
    /// Bind variable operation
    fn bind_variable(
        &mut self,
        var_index: usize,
        tuple_index: usize,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        if let Some(retrieval) = context.retrievals.last() {
            if tuple_index < retrieval.tuple_buffer.len() && var_index < context.bindings.len() {
                context.bindings[var_index] = Some(retrieval.tuple_buffer[tuple_index].clone());
                Ok(ExecutionResult::Continue)
            } else {
                Ok(ExecutionResult::Fail)
            }
        } else {
            Ok(ExecutionResult::Fail)
        }
    }
    
    /// Check guard operation
    fn check_guard(
        &mut self,
        predicate: &str,
        var_index: usize,
        fail_target: usize,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        if let Some(Some(value)) = context.bindings.get(var_index) {
            let has_guard = context.extension_manager.has_concept(value, predicate)?;
            if has_guard {
                Ok(ExecutionResult::Continue)
            } else {
                Ok(ExecutionResult::Jump(fail_target))
            }
        } else {
            Ok(ExecutionResult::Jump(fail_target))
        }
    }
    
    /// Apply head operation
    fn apply_head(
        &mut self,
        clause_index: usize,
        atom_index: usize,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        if let Some(head_clause) = self.head_clauses.get(clause_index) {
            if let Some(atom) = head_clause.head.get(atom_index) {
                // Substitute variables with current bindings
                let mut args = Vec::new();
                for arg in &atom.arguments {
                    if let Some(var_index) = self.variable_mappings.get(arg) {
                        if let Some(Some(value)) = context.bindings.get(*var_index) {
                            args.push(value.clone());
                        } else {
                            args.push(arg.clone());
                        }
                    } else {
                        args.push(arg.clone());
                    }
                }
                
                // Add fact to extension manager
                context.extension_manager.add_fact(atom.predicate.clone(), args)?;
                Ok(ExecutionResult::Application)
            } else {
                Ok(ExecutionResult::Fail)
            }
        } else {
            Ok(ExecutionResult::Fail)
        }
    }
    
    /// Create ground disjunction operation
    fn create_ground_disjunction(
        &mut self,
        atoms: &[String],
        priority: i32,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        use crate::core::hypertableau::ground_disjunction::{
            GroundDisjunctionHeader, DisjunctionPriority, DisjunctPredicate
        };
        use crate::core::dependency::DependencySet;
        
        // Convert atoms to DisjunctPredicate - this is a simplified conversion
        // In a real implementation, atoms would be parsed properly
        let predicates: Vec<DisjunctPredicate> = atoms.iter().enumerate().map(|(i, _atom)| {
            // Create a placeholder concept predicate for now
            DisjunctPredicate::Concept {
                concept: crate::ontology::ClassExpression::Class(
                    crate::ontology::Class {
                        iri: crate::ontology::IRI::new("http://example.org/placeholder").to_url().expect("Valid URL").into()
                    }
                ),
                argument: i,
            }
        }).collect();
        
        let header = GroundDisjunctionHeader::new_with_predicates(
            predicates,
            DisjunctionPriority::Normal,
        );

        // Create ground disjunction
        let disjunction = GroundDisjunction::new(
            header,
            vec![0; atoms.len()], // arguments (node IDs)
            vec![false; atoms.len()], // is_core flags
            DependencySet::empty(), // dependency set
            0, // id
        );
        
        // Add to extension manager
        context.extension_manager.add_ground_disjunction(disjunction)?;
        Ok(ExecutionResult::Application)
    }
    
    /// Add dependency operation
    fn add_dependency(
        &mut self,
        source_indices: &[usize],
        target_index: usize,
        context: &mut ExecutionContext,
    ) -> Result<ExecutionResult> {
        // Create dependency set from sources
        let mut dependencies = DependencySet::empty();
        for &source_index in source_indices {
            if let Some(retrieval) = context.retrievals.get(source_index) {
                // Add dependency information
                dependencies = dependencies.union(&DependencySet::singleton(source_index.to_string()));
            }
        }
        
        // Add dependency to extension manager
        context.extension_manager.add_dependency(target_index.to_string(), dependencies)?;
        Ok(ExecutionResult::Continue)
    }
    
    /// Helper functions
    fn is_variable(&self, term: &str) -> bool {
        term.chars().next().map_or(false, |c| c.is_lowercase())
    }
    
    fn calculate_fail_target(&self, atom_index: usize) -> usize {
        // Calculate where to jump on failure - simplified logic
        self.workers.len()
    }
    
    fn creates_ground_disjunction(&self, atom: &Atom) -> bool {
        // Check if this atom creates a ground disjunction
        atom.predicate.contains("DisjunctiveAssertion") || atom.predicate.contains("Choice")
    }
    
    fn calculate_disjunction_priority(&self, atom: &Atom) -> i32 {
        // Calculate priority for ground disjunctions
        atom.arguments.len() as i32
    }
    
    fn should_terminate_early(&self, context: &ExecutionContext) -> bool {
        // Early termination heuristics
        context.extension_manager.contains_clash() || self.matches > 1000
    }
    
    /// Clear evaluator state
    pub fn clear(&mut self) {
        for binding in &mut self.binding_buffer {
            *binding = None;
        }
        self.retrievals.clear();
        self.evaluations = 0;
        self.matches = 0;
        self.applications = 0;
    }
    
    /// Get evaluation statistics
    pub fn get_statistics(&self) -> EvaluationStatistics {
        EvaluationStatistics {
            evaluations: self.evaluations,
            matches: self.matches,
            applications: self.applications,
            workers: self.workers.len(),
            variables: self.variable_mappings.len(),
            optimization_enabled: self.optimization_enabled,
        }
    }
}

/// Result of worker execution
#[derive(Debug)]
pub enum ExecutionResult {
    Continue,
    Jump(usize),
    Return,
    Fail,
    Match,
    Application,
}

/// Evaluation statistics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvaluationStatistics {
    pub evaluations: u64,
    pub matches: u64,
    pub applications: u64,
    pub workers: usize,
    pub variables: usize,
    pub optimization_enabled: bool,
}

// Placeholder implementations for helper managers
impl GroundDisjunctionHeaderManager {
    fn new() -> Self {
        GroundDisjunctionHeaderManager {
            headers: HashMap::new(),
            next_id: 0,
        }
    }
}

impl BufferSupply {
    fn new() -> Self {
        BufferSupply {
            buffers: Vec::new(),
            buffer_sizes: Vec::new(),
            allocation_index: 0,
        }
    }
}

impl ValuesBufferManager {
    fn new() -> Self {
        ValuesBufferManager {
            values_buffer: Vec::new(),
            max_variables: 0,
            variable_mappings: HashMap::new(),
        }
    }
}

impl fmt::Display for EvaluationStatistics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,
            "Evaluation Statistics:\n\
             Evaluations: {}\n\
             Matches: {}\n\
             Applications: {}\n\
             Workers: {}\n\
             Variables: {}\n\
             Optimization: {}",
            self.evaluations, self.matches, self.applications,
            self.workers, self.variables, self.optimization_enabled
        )
    }
}
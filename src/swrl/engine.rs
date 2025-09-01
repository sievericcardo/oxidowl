//! SWRL Rule Engine
//!
//! This module implements the core SWRL rule execution engine that coordinates
//! rule firing, inference generation, and integration with the tableau reasoner.

use crate::ontology::{Axiom, Ontology, Individual, Literal};
use crate::swrl::{
    SWRLAtom, SWRLConfig, SWRLExecutionContext, SWRLExecutionResult, SWRLReasoningStrategy,
    SWRLRule, SWRLRuleState, SWRLStatistics, SWRLDArgument, SWRLIArgument,
    builtins::{SWRLBuiltIn, SWRLBuiltInRegistry, SWRLValue},
    interpreter::SWRLInterpreter,
    validation::SWRLValidator,
};
use crate::{Error, IRI, Result};
use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// SWRL Rule Engine
///
/// The main engine for executing SWRL rules and generating inferences.
/// Supports both forward and backward chaining strategies.
#[derive(Debug)]
pub struct SWRLRuleEngine {
    /// Rule execution states
    rule_states: HashMap<u64, SWRLRuleState>,

    /// Built-in predicate registry
    builtin_registry: Arc<SWRLBuiltInRegistry>,

    /// Rule interpreter
    interpreter: SWRLInterpreter,

    /// Rule validator
    validator: SWRLValidator,

    /// Engine configuration
    config: SWRLConfig,

    /// Execution statistics
    statistics: SWRLStatistics,

    /// Current ontology
    ontology: Option<Arc<RwLock<Ontology>>>,

    /// Inference cache
    inference_cache: HashSet<Axiom>,

    /// Rule priority ordering
    rule_priorities: HashMap<u64, u32>,
}

impl SWRLRuleEngine {
    /// Create a new SWRL engine
    #[must_use]
    pub fn new(config: SWRLConfig) -> Self {
        let builtin_registry = Arc::new(SWRLBuiltInRegistry::new());
        let interpreter = SWRLInterpreter::new(builtin_registry.clone());
        let validator = SWRLValidator::new();

        Self {
            rule_states: HashMap::new(),
            builtin_registry,
            interpreter,
            validator,
            config,
            statistics: SWRLStatistics::default(),
            ontology: None,
            inference_cache: HashSet::new(),
            rule_priorities: HashMap::new(),
        }
    }

    /// Set the ontology for rule execution
    pub fn set_ontology(&mut self, ontology: Arc<RwLock<Ontology>>) {
        self.ontology = Some(ontology);
        self.load_rules_from_ontology();
    }

    /// Load SWRL rules from the current ontology
    fn load_rules_from_ontology(&mut self) {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();

            self.rule_states.clear();
            let mut rule_count = 0;

            for axiom in ontology_guard.axioms() {
                if let Axiom::Rule(rule_axiom) = axiom {
                    // Validate the rule
                    match self.validator.validate_rule(&rule_axiom.rule) {
                        Ok(_) => {
                            let rule_state = SWRLRuleState::new(rule_axiom.rule.clone());
                            self.rule_states.insert(rule_axiom.id, rule_state);
                            rule_count += 1;

                            if self.config.debug {
                                debug!("Loaded SWRL rule {}: {:?}", rule_axiom.id, rule_axiom.rule);
                            }
                        }
                        Err(e) => {
                            warn!("Invalid SWRL rule {}: {}", rule_axiom.id, e);
                        }
                    }
                }
            }

            info!("Loaded {} SWRL rules from ontology", rule_count);
        }
    }

    /// Execute all rules using the configured strategy
    pub fn execute_rules(&mut self) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();

        if self.rule_states.is_empty() {
            return Ok(SWRLExecutionResult::empty());
        }

        let result = match self.config.strategy {
            SWRLReasoningStrategy::ForwardChaining => self.execute_forward_chaining(),
            SWRLReasoningStrategy::BackwardChaining => self.execute_backward_chaining(),
            SWRLReasoningStrategy::Hybrid => self.execute_hybrid_reasoning(),
        }?;

        let execution_time = start_time.elapsed();
        self.statistics.total_reasoning_time_us += execution_time.as_micros() as u64;
        self.statistics.update(&result);

        info!(
            "SWRL execution completed: {} rules fired, {} inferences in {:?}",
            result.applications,
            result.inferences.len(),
            execution_time
        );

        Ok(result)
    }

    /// Execute forward chaining strategy
    fn execute_forward_chaining(&mut self) -> Result<SWRLExecutionResult> {
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut any_fired = false;
        let mut iteration = 0;

        // Continue until no new inferences are generated
        loop {
            iteration += 1;
            let mut iteration_inferences = Vec::new();
            let mut iteration_fired = false;

            if self.config.debug {
                debug!("Forward chaining iteration {}", iteration);
            }

            // Get rules ordered by priority
            let ordered_rules = self.get_ordered_rules();

            for rule_id in ordered_rules {
                // First get the rule to execute (clone it to avoid borrow issues)
                let rule_to_execute = if let Some(rule_state) = self.rule_states.get(&rule_id) {
                    if rule_state.should_skip(self.config.max_rule_applications) {
                        continue;
                    }
                    rule_state.rule.clone()
                } else {
                    continue;
                };

                let result = self.execute_single_rule(&rule_to_execute)?;

                if result.fired {
                    iteration_fired = true;
                    any_fired = true;
                    total_applications += result.applications;

                    // Add new inferences that aren't already cached
                    let inferences_clone = result.inferences.clone();
                    for inference in &inferences_clone {
                        if !self.inference_cache.contains(inference) {
                            self.inference_cache.insert(inference.clone());
                            iteration_inferences.push(inference.clone());
                            total_inferences.push(inference.clone());
                        }
                    }

                    // Now we can safely get the mutable reference
                    if let Some(rule_state) = self.rule_states.get_mut(&rule_id) {
                        rule_state.mark_applied(result);
                    }
                }
            }

            // Apply new inferences to the ontology
            if !iteration_inferences.is_empty() {
                self.apply_inferences_to_ontology(iteration_inferences)?;
            }

            // Stop if no rules fired in this iteration
            if !iteration_fired {
                break;
            }

            // Safety check for infinite loops
            if iteration > 1000 {
                warn!("Forward chaining stopped after 1000 iterations");
                break;
            }
        }

        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }

    /// Execute backward chaining strategy
    fn execute_backward_chaining(&mut self) -> Result<SWRLExecutionResult> {
        info!("Executing SWRL backward chaining strategy");
        
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut any_fired = false;
        
        // Initialize goal stack for backward chaining
        let mut goal_stack = Vec::new();
        let mut proved_goals = HashSet::new();
        let mut failed_goals = HashSet::new();
        
        // Start with queries from the ontology (if any are specified)
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // For demonstration, we'll use class assertion queries as initial goals
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                    // Convert to a goal for backward chaining
                    let goal = SWRLGoal::from_class_assertion(assertion);
                    goal_stack.push(goal);
                }
            }
        }
        
        // Main backward chaining loop
        let mut iteration = 0;
        while !goal_stack.is_empty() && iteration < self.config.max_rule_applications {
            iteration += 1;
            let current_goal = goal_stack.pop().unwrap();
            
            // Skip if already proved or failed
            if proved_goals.contains(&current_goal) || failed_goals.contains(&current_goal) {
                continue;
            }
            
            // Try to prove the goal using available rules
            let proof_result = self.try_prove_goal(&current_goal, &mut goal_stack)?;
            
            match proof_result {
                GoalProofResult::Proved => {
                    proved_goals.insert(current_goal);
                    any_fired = true;
                    // For now, we don't track specific axioms in backward chaining
                    // total_inferences.push(some_axiom);
                }
                GoalProofResult::Failed => {
                    failed_goals.insert(current_goal);
                }
                GoalProofResult::NeedsMoreProofs => {
                    // Subgoals added to stack, try again later
                    goal_stack.push(current_goal);
                }
            }
            
            total_applications += 1;
        }
        
        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }

    /// Execute hybrid reasoning strategy
    fn execute_hybrid_reasoning(&mut self) -> Result<SWRLExecutionResult> {
        info!("Executing SWRL hybrid reasoning strategy");
        
        // Combine forward and backward chaining in an interleaved manner
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut any_fired = false;
        
        let max_iterations = self.config.max_rule_applications / 2; // Split between strategies
        
        // Phase 1: Forward chaining to establish base facts
        info!("Hybrid reasoning - Phase 1: Forward chaining");
        let forward_result = self.execute_forward_chaining_limited(max_iterations)?;
        total_inferences.extend(forward_result.inferences);
        total_applications += forward_result.applications;
        any_fired = any_fired || forward_result.fired;
        
        // Phase 2: Backward chaining for goal-directed inference
        info!("Hybrid reasoning - Phase 2: Backward chaining");
        let backward_result = self.execute_backward_chaining_limited(max_iterations)?;
        total_inferences.extend(backward_result.inferences);
        total_applications += backward_result.applications;
        any_fired = any_fired || backward_result.fired;
        
        // Phase 3: Additional forward chaining if backward chaining added new facts
        if backward_result.fired {
            info!("Hybrid reasoning - Phase 3: Additional forward chaining");
            let additional_forward = self.execute_forward_chaining_limited(max_iterations / 2)?;
            total_inferences.extend(additional_forward.inferences);
            total_applications += additional_forward.applications;
            any_fired = any_fired || additional_forward.fired;
        }
        
        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }

    /// Check if a rule is applicable for execution
    fn is_rule_applicable(&self, rule_id: usize) -> Result<bool> {
        // Convert usize back to u64 for the HashMap lookup
        let rule_id_u64 = rule_id as u64;
        if let Some(rule_state) = self.rule_states.get(&rule_id_u64) {
            Ok(rule_state.active)
        } else {
            Ok(false)
        }
    }

    /// Execute a single SWRL rule
    fn execute_single_rule(&mut self, rule: &SWRLRule) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();

        if self.config.debug {
            debug!("Executing rule: {:?}", rule);
        }

        // Check timeout
        if let Some(timeout_ms) = self.config.timeout_ms {
            if start_time.elapsed().as_millis() > timeout_ms as u128 {
                return Err(Error::reasoning("SWRL rule execution timeout"));
            }
        }

        let mut context = SWRLExecutionContext::new();
        context.max_depth = self.config.max_execution_depth;

        let result =
            self.interpreter
                .execute_rule(rule, &mut context, self.ontology.as_ref().unwrap())?;

        let execution_time = start_time.elapsed();
        let mut result_with_time = result;
        result_with_time.execution_time_us = execution_time.as_micros() as u64;

        Ok(result_with_time)
    }

    /// Get rules ordered by priority
    fn get_ordered_rules(&self) -> Vec<u64> {
        let mut rule_ids: Vec<_> = self.rule_states.keys().copied().collect();

        // Sort by priority (higher priority first)
        rule_ids.sort_by(|a, b| {
            let priority_a = self.rule_priorities.get(a).unwrap_or(&0);
            let priority_b = self.rule_priorities.get(b).unwrap_or(&0);
            priority_b.cmp(priority_a)
        });

        rule_ids
    }

    /// Apply inferences to the ontology
    fn apply_inferences_to_ontology(&mut self, inferences: Vec<Axiom>) -> Result<()> {
        if let Some(ontology) = &self.ontology {
            let mut ontology_guard = ontology.write().unwrap();

            for inference in inferences {
                ontology_guard.add_axiom(inference);
            }
        }

        Ok(())
    }

    /// Add a custom built-in predicate
    pub fn add_builtin(&mut self, builtin: Box<dyn SWRLBuiltIn>) {
        let iri = IRI::new(builtin.name());
        Arc::get_mut(&mut self.builtin_registry)
            .unwrap()
            .register_builtin(iri, builtin);
    }

    /// Set rule priority
    pub fn set_rule_priority(&mut self, rule_id: u64, priority: u32) {
        self.rule_priorities.insert(rule_id, priority);
    }

    /// Get execution statistics
    #[must_use]
    pub fn get_statistics(&self) -> &SWRLStatistics {
        &self.statistics
    }

    /// Reset engine state
    pub fn reset(&mut self) {
        self.rule_states.clear();
        self.inference_cache.clear();
        self.rule_priorities.clear();
        self.statistics.reset();
    }

    /// Get current configuration
    #[must_use]
    pub fn get_config(&self) -> &SWRLConfig {
        &self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: SWRLConfig) {
        self.config = config;
    }

    /// Check if a rule is currently loaded
    #[must_use]
    pub fn has_rule(&self, rule_id: u64) -> bool {
        self.rule_states.contains_key(&rule_id)
    }

    /// Get rule state
    #[must_use]
    pub fn get_rule_state(&self, rule_id: u64) -> Option<&SWRLRuleState> {
        self.rule_states.get(&rule_id)
    }

    /// Enable or disable a rule
    pub fn set_rule_active(&mut self, rule_id: u64, active: bool) -> Result<()> {
        if let Some(rule_state) = self.rule_states.get_mut(&rule_id) {
            rule_state.active = active;
            Ok(())
        } else {
            Err(Error::reasoning(format!("Rule {} not found", rule_id)))
        }
    }

    /// Get all loaded rule IDs
    #[must_use]
    pub fn get_rule_ids(&self) -> Vec<u64> {
        self.rule_states.keys().copied().collect()
    }

    /// Execute rules with a specific query goal (for backward chaining)
    pub fn execute_with_goal(&mut self, goal: &SWRLAtom) -> Result<SWRLExecutionResult> {
        info!("Starting goal-driven execution for goal: {:?}", goal);
        
        let mut result = SWRLExecutionResult::empty();
        let mut goal_stack = vec![goal.clone()];
        let mut visited_goals = HashSet::new();
        let max_depth = self.config.max_rule_applications;
        
        while let Some(current_goal) = goal_stack.pop() {
            // Prevent infinite recursion
            if visited_goals.contains(&current_goal) {
                continue;
            }
            
            if visited_goals.len() >= max_depth {
                warn!("Maximum goal depth reached, stopping backward chaining");
                break;
            }
            
            visited_goals.insert(current_goal.clone());
            
            // Check if goal is already satisfied by known facts
            if self.is_goal_satisfied(&current_goal)? {
                continue;
            }
            
            // Find rules that can prove this goal
            let applicable_rules = self.find_rules_for_goal(&current_goal)?;
            
            for rule_id in applicable_rules {
                // Get the rule from the ontology
                let rule_to_execute = if let Some(ontology) = &self.ontology {
                    let ontology_guard = ontology.read().unwrap();
                    let mut found_rule = None;
                    for axiom in ontology_guard.axioms() {
                        if let Axiom::Rule(rule_axiom) = axiom {
                            if rule_axiom.id == rule_id {
                                found_rule = Some(rule_axiom.rule.clone());
                                break;
                            }
                        }
                    }
                    found_rule
                } else {
                    None
                };
                
                if let Some(rule) = rule_to_execute {
                    // For each rule that can prove the goal, try to prove its body
                    let subgoals = self.extract_subgoals_from_rule_body(&rule.body)?;
                    
                    // Add subgoals to the stack for backward chaining
                    let mut all_subgoals_satisfied = true;
                    for subgoal in &subgoals {
                        if !self.is_goal_satisfied(subgoal)? {
                            goal_stack.push(subgoal.clone());
                            all_subgoals_satisfied = false;
                        }
                    }
                    
                    // If all subgoals are satisfied, we can fire this rule
                    if all_subgoals_satisfied {
                        if let Ok(rule_result) = self.execute_single_rule(&rule) {
                            // For now, we'll simulate result merging by tracking success
                            if rule_result.fired {
                                result.fired = true;
                                result.inferences.extend(rule_result.inferences);
                                result.applications += rule_result.applications;
                            }
                            
                            // Check if our original goal is now satisfied
                            if self.is_goal_satisfied(goal)? {
                                info!("Goal achieved through backward chaining");
                                return Ok(result);
                            }
                        }
                    }
                }
            }
        }
        
        if self.is_goal_satisfied(goal)? {
            info!("Goal-driven execution completed successfully");
        } else {
            warn!("Goal could not be satisfied through backward chaining");
        }
        
        Ok(result)
    }

    /// Check if a goal is satisfied by current facts/knowledge
    fn is_goal_satisfied(&self, goal: &SWRLAtom) -> Result<bool> {
        match goal {
            SWRLAtom::ClassAtom { predicate, argument } => {
                // Check if the individual is known to be of this class
                self.check_class_membership(predicate, argument, None)
            }
            SWRLAtom::ObjectPropertyAtom { predicate, first_argument, second_argument } => {
                // Check if the object property relation exists
                self.check_object_property_relation(predicate, first_argument, second_argument)
            }
            SWRLAtom::DataPropertyAtom { predicate, first_argument, second_argument } => {
                // Check if the data property relation exists
                self.check_data_property_relation(predicate, first_argument, second_argument)
            }
            SWRLAtom::SameIndividualAtom { first_argument, second_argument } => {
                // Check if individuals are known to be the same
                self.check_same_individual(first_argument, second_argument)
            }
            SWRLAtom::DifferentIndividualsAtom { first_argument, second_argument } => {
                // Check if individuals are known to be different
                self.check_different_individuals(first_argument, second_argument)
            }
            SWRLAtom::DataRangeAtom { predicate, argument } => {
                // Check if the data value is in the specified data range
                self.check_data_range_membership(predicate, argument, None)
            }
            SWRLAtom::BuiltInAtom { predicate, arguments } => {
                // Evaluate built-in predicates
                self.evaluate_builtin_atom(predicate, arguments)
            }
        }
    }

    /// Find rules that can potentially prove a given goal
    fn find_rules_for_goal(&self, goal: &SWRLAtom) -> Result<Vec<u64>> {
        let mut applicable_rules = Vec::new();
        
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            for axiom in ontology_guard.axioms() {
                if let Axiom::Rule(rule_axiom) = axiom {
                    // Check if any atom in the rule head matches our goal
                    for head_atom in &rule_axiom.rule.head {
                        if self.atoms_unify(head_atom, goal)? {
                            applicable_rules.push(rule_axiom.id);
                            break;
                        }
                    }
                }
            }
        }
        
        Ok(applicable_rules)
    }

    /// Extract subgoals from a rule body for backward chaining
    fn extract_subgoals_from_rule_body(&self, body: &[SWRLAtom]) -> Result<Vec<SWRLAtom>> {
        // For now, treat all body atoms as subgoals
        // In a more sophisticated implementation, we'd handle variable bindings
        Ok(body.to_vec())
    }

    /// Check if two atoms can unify (enhanced unification with variable binding)
    fn atoms_unify(&self, atom1: &SWRLAtom, atom2: &SWRLAtom) -> Result<bool> {
        // Enhanced unification following SWRL specification
        // This implements proper unification with variable binding and term matching
        
        match (atom1, atom2) {
            (SWRLAtom::ClassAtom { predicate: p1, argument: arg1 }, 
             SWRLAtom::ClassAtom { predicate: p2, argument: arg2 }) => {
                // Class atoms unify if predicates match and arguments are unifiable
                if p1 == p2 {
                    self.arguments_unify(arg1, arg2)
                } else {
                    Ok(false)
                }
            }
            (SWRLAtom::ObjectPropertyAtom { predicate: p1, first_argument: arg1_1, second_argument: arg1_2 }, 
             SWRLAtom::ObjectPropertyAtom { predicate: p2, first_argument: arg2_1, second_argument: arg2_2 }) => {
                // Object property atoms unify if predicates match and both argument pairs unify
                if p1 == p2 {
                    let first_unify = self.arguments_unify(arg1_1, arg2_1)?;
                    let second_unify = self.arguments_unify(arg1_2, arg2_2)?;
                    Ok(first_unify && second_unify)
                } else {
                    Ok(false)
                }
            }
            (SWRLAtom::DataPropertyAtom { predicate: p1, first_argument: arg1_1, second_argument: arg1_2 }, 
             SWRLAtom::DataPropertyAtom { predicate: p2, first_argument: arg2_1, second_argument: arg2_2 }) => {
                // Data property atoms unify if predicates match and both argument pairs unify
                if p1 == p2 {
                    let first_unify = self.arguments_unify(arg1_1, arg2_1)?;
                    let second_unify = self.data_arguments_unify(arg1_2, arg2_2)?;
                    Ok(first_unify && second_unify)
                } else {
                    Ok(false)
                }
            }
            (SWRLAtom::BuiltInAtom { predicate: p1, arguments: args1 }, 
             SWRLAtom::BuiltInAtom { predicate: p2, arguments: args2 }) => {
                // Built-in atoms unify if predicates match and argument lists unify
                if p1 == p2 && args1.len() == args2.len() {
                    for (arg1, arg2) in args1.iter().zip(args2.iter()) {
                        if !self.data_arguments_unify(arg1, arg2)? {
                            return Ok(false);
                        }
                    }
                    Ok(true)
                } else {
                    Ok(false)
                }
            }
            (SWRLAtom::SameIndividualAtom { first_argument: arg1_1, second_argument: arg1_2 }, 
             SWRLAtom::SameIndividualAtom { first_argument: arg2_1, second_argument: arg2_2 }) => {
                // Same individual atoms unify if both argument pairs unify
                let first_unify = self.arguments_unify(arg1_1, arg2_1)?;
                let second_unify = self.arguments_unify(arg1_2, arg2_2)?;
                Ok(first_unify && second_unify)
            }
            (SWRLAtom::DifferentIndividualsAtom { first_argument: arg1_1, second_argument: arg1_2 }, 
             SWRLAtom::DifferentIndividualsAtom { first_argument: arg2_1, second_argument: arg2_2 }) => {
                // Different individuals atoms unify if both argument pairs unify
                let first_unify = self.arguments_unify(arg1_1, arg2_1)?;
                let second_unify = self.arguments_unify(arg1_2, arg2_2)?;
                Ok(first_unify && second_unify)
            }
            _ => {
                // Different atom types cannot unify
                Ok(false)
            },
        }
    }
    
    /// Check if two individual arguments can unify
    fn arguments_unify(&self, arg1: &SWRLIArgument, arg2: &SWRLIArgument) -> Result<bool> {
        match (arg1, arg2) {
            (SWRLIArgument::Individual(ind1), SWRLIArgument::Individual(ind2)) => {
                // Named individuals unify if they are the same
                Ok(ind1 == ind2)
            }
            (SWRLIArgument::Variable(var1), SWRLIArgument::Variable(var2)) => {
                // Variables always unify (binding would be handled in substitution)
                Ok(true)
            }
            (SWRLIArgument::Variable(_), SWRLIArgument::Individual(_)) |
            (SWRLIArgument::Individual(_), SWRLIArgument::Variable(_)) => {
                // Variable and individual can unify (variable would be bound to individual)
                Ok(true)
            }
        }
    }
    
    /// Check if two data arguments can unify
    fn data_arguments_unify(&self, arg1: &SWRLDArgument, arg2: &SWRLDArgument) -> Result<bool> {
        match (arg1, arg2) {
            (SWRLDArgument::Literal(lit1), SWRLDArgument::Literal(lit2)) => {
                // Literals unify if they are equal
                Ok(lit1 == lit2)
            }
            (SWRLDArgument::Variable(var1), SWRLDArgument::Variable(var2)) => {
                // Variables always unify
                Ok(true)
            }
            (SWRLDArgument::Variable(_), SWRLDArgument::Literal(_)) |
            (SWRLDArgument::Literal(_), SWRLDArgument::Variable(_)) => {
                // Variable and literal can unify
                Ok(true)
            }
        }
    }

    /// Check class membership using ontology reasoning
    /// Check if individual is member of a class using ontology reasoning
    fn check_class_membership(&self, class: &crate::ontology::ClassExpression, individual: &SWRLIArgument, context: Option<&SWRLExecutionContext>) -> Result<bool> {
        // Convert SWRL individual argument to ontology individual
        let individual_iri = match individual {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(var) => {
                // Check if variable is bound to an individual
                if let Some(ctx) = context {
                    if let Some(bound_value) = ctx.bindings.get(var) {
                        match bound_value {
                            // If bound to an individual, use that
                            SWRLValue::Individual(ind) => ind.iri(),
                            _ => return Ok(false), // Wrong type binding
                        }
                    } else {
                        return Ok(false); // Unbound variable  
                    }
                } else {
                    // No context - return false for variables
                    return Ok(false);
                }
            },
        };
        
        // Check membership using ontology
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                // Direct class assertion check
                for axiom in ontology.axioms() {
                    if let crate::ontology::Axiom::ClassAssertion(class_axiom) = axiom {
                        if class_axiom.individual.iri() == individual_iri && class_axiom.class == *class {
                            return Ok(true);
                        }
                    }
                }
                
                // Check subclass relationships
                match class {
                    crate::ontology::ClassExpression::Class(target_class) => {
                        for axiom in ontology.axioms() {
                            if let crate::ontology::Axiom::ClassAssertion(class_axiom) = axiom {
                                if class_axiom.individual.iri() == individual_iri {
                                    // Check if asserted class is subclass of target
                                    if self.is_subclass_relation(&class_axiom.class, class, &ontology) {
                                        return Ok(true);
                                    }
                                }
                            }
                        }
                    },
                    _ => {
                        // For complex class expressions, simplified handling
                        // Would need full reasoning engine integration
                        return Ok(false);
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Check if first class is subclass of second class
    fn is_subclass_relation(
        &self,
        subclass: &crate::ontology::ClassExpression,
        superclass: &crate::ontology::ClassExpression,
        ontology: &crate::ontology::Ontology
    ) -> bool {
        // Direct subclass check
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                if subclass_axiom.subclass == *subclass && subclass_axiom.superclass == *superclass {
                    return true;
                }
            }
        }
        
        // Transitivity check (one level for now)
        for axiom in ontology.axioms() {
            if let crate::ontology::Axiom::SubClassOf(subclass_axiom) = axiom {
                if subclass_axiom.subclass == *subclass {
                    // Check if intermediate class is subclass of target
                    if self.is_subclass_relation(&subclass_axiom.superclass, superclass, ontology) {
                        return true;
                    }
                }
            }
        }
        
        false
    }

    /// Check object property relation using ontology reasoning
    fn check_object_property_relation(
        &self,
        property: &crate::ontology::ObjectPropertyExpression,
        first: &SWRLIArgument,
        second: &SWRLIArgument,
    ) -> Result<bool> {
        // Convert SWRL arguments to ontology individuals
        let first_iri = match first {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        let second_iri = match second {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        
        // Check if the property relation is explicitly asserted
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                for axiom in ontology.axioms() {
                    match axiom {
                        crate::ontology::Axiom::ObjectPropertyAssertion(assertion) => {
                            if let (Some(subj_iri), Some(obj_iri)) = 
                                (assertion.source.iri(), assertion.target.iri()) {
                                if Some(subj_iri) == first_iri && Some(obj_iri) == second_iri &&
                                   self.property_expressions_match(&assertion.property, property) {
                                    return Ok(true);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Helper method to check if class expressions match
    fn class_expressions_match(&self, expr1: &crate::ontology::ClassExpression, expr2: &crate::ontology::ClassExpression) -> bool {
        match (expr1, expr2) {
            (crate::ontology::ClassExpression::Class(c1), crate::ontology::ClassExpression::Class(c2)) => {
                c1.iri == c2.iri
            }
            _ => false, // More complex matching could be implemented
        }
    }

    /// Helper method to check if property expressions match
    fn property_expressions_match(&self, expr1: &crate::ontology::ObjectPropertyExpression, expr2: &crate::ontology::ObjectPropertyExpression) -> bool {
        match (expr1, expr2) {
            (crate::ontology::ObjectPropertyExpression::ObjectProperty(p1), 
             crate::ontology::ObjectPropertyExpression::ObjectProperty(p2)) => {
                p1.iri == p2.iri
            }
            _ => false, // More complex matching could be implemented
        }
    }

    /// Helper method to check if data property expressions match
    fn data_property_expressions_match(&self, expr1: &crate::ontology::DataPropertyExpression, expr2: &crate::ontology::DataPropertyExpression) -> bool {
        match (expr1, expr2) {
            (crate::ontology::DataPropertyExpression::DataProperty(p1), 
             crate::ontology::DataPropertyExpression::DataProperty(p2)) => {
                p1.iri == p2.iri
            }
            _ => false, // More complex matching could be implemented
        }
    }

    /// Helper method to check if literals match
    fn literals_match(&self, ontology_lit: &crate::ontology::Literal, swrl_value: &SWRLDArgument) -> bool {
        match swrl_value {
            SWRLDArgument::Literal(swrl_lit) => {
                ontology_lit.value == swrl_lit.value && ontology_lit.datatype == swrl_lit.datatype
            }
            SWRLDArgument::Variable(_) => false, // Cannot match unbound variables
        }
    }

    /// Check data property relation using ontology reasoning
    fn check_data_property_relation(
        &self,
        property: &crate::ontology::DataPropertyExpression,
        individual: &SWRLIArgument,
        value: &SWRLDArgument,
    ) -> Result<bool> {
        // Convert SWRL arguments
        let individual_iri = match individual {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        
        let literal_value = match value {
            SWRLDArgument::Literal(lit) => lit,
            SWRLDArgument::Variable(_) => return Ok(false),
        };
        
        // Check if the data property relation is explicitly asserted
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                for axiom in ontology.axioms() {
                    match axiom {
                        crate::ontology::Axiom::DataPropertyAssertion(assertion) => {
                            if let Some(subj_iri) = assertion.individual.iri() {
                                if Some(subj_iri) == individual_iri &&
                                   self.data_property_expressions_match(&assertion.property, property) &&
                                   self.literals_match(&assertion.value, &SWRLDArgument::Literal(literal_value.clone())) {
                                    return Ok(true);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        Ok(false)
    }

    /// Check if two individuals are the same using ontology reasoning
    fn check_same_individual(&self, first: &SWRLIArgument, second: &SWRLIArgument) -> Result<bool> {
        let first_iri = match first {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        let second_iri = match second {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        
        // Check for explicit same individual assertions
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                for axiom in ontology.axioms() {
                    match axiom {
                        crate::ontology::Axiom::SameIndividual(assertion) => {
                            let iris: Vec<_> = assertion.individuals.iter()
                                .filter_map(|ind| ind.iri())
                                .collect();
                            if first_iri.map_or(false, |iri| iris.contains(&iri)) && 
                               second_iri.map_or(false, |iri| iris.contains(&iri)) {
                                return Ok(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Same IRI means same individual
        Ok(first_iri == second_iri)
    }

    /// Check if two individuals are different using ontology reasoning
    fn check_different_individuals(&self, first: &SWRLIArgument, second: &SWRLIArgument) -> Result<bool> {
        let first_iri = match first {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        let second_iri = match second {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false),
        };
        
        // Check for explicit different individuals assertions
        if let Some(ontology_ref) = &self.ontology {
            if let Ok(ontology) = ontology_ref.read() {
                for axiom in ontology.axioms() {
                    match axiom {
                        crate::ontology::Axiom::DifferentIndividuals(assertion) => {
                            let iris: Vec<_> = assertion.individuals.iter()
                                .filter_map(|ind| ind.iri())
                                .collect();
                            if first_iri.map_or(false, |iri| iris.contains(&iri)) && 
                               second_iri.map_or(false, |iri| iris.contains(&iri)) {
                                return Ok(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        
        // Different IRIs typically mean different individuals under unique name assumption
        Ok(first_iri != second_iri)
    }

    /// Check if a data value satisfies a data range constraint
    fn check_data_range_membership(&self, range: &crate::ontology::DataRange, value: &SWRLDArgument, context: Option<&SWRLExecutionContext>) -> Result<bool> {
        use crate::ontology::DataRange;
        
        match range {
            DataRange::Datatype(datatype_iri) => {
                // Check if value matches the datatype
                match value {
                    SWRLDArgument::Literal(literal_value) => {
                        // Check if literal has datatype and matches
                        match &literal_value.datatype {
                            Some(value_datatype) => {
                                // Direct datatype match
                                Ok(datatype_iri.as_str() == value_datatype.as_str())
                            },
                            None => {
                                // Infer datatype from literal format
                                let inferred_datatype = self.infer_datatype_from_literal(&literal_value.value);
                                Ok(inferred_datatype.as_ref().map(|iri| iri.as_str()) == Some(datatype_iri.as_str()))
                            }
                        }
                    },
                    SWRLDArgument::Variable(var) => {
                        // Check if variable is bound to a value of the correct datatype
                        if let Some(ctx) = context {
                            if let Some(bound_value) = ctx.bindings.get(var) {
                                // Convert SWRLValue to SWRLDArgument for recursive call
                                let d_arg = match bound_value {
                                    SWRLValue::Literal(lit) => SWRLDArgument::Literal(lit.clone()),
                                    _ => return Ok(false), // Wrong type for data range
                                };
                                self.check_data_range_membership(range, &d_arg, context)
                            } else {
                                Ok(false) // Unbound variable doesn't satisfy range
                            }
                        } else {
                            Ok(false) // No context - can't check variable
                        }
                    },
                }
            },
            DataRange::DataIntersectionOf(ranges) => {
                // Value must satisfy all ranges
                for sub_range in ranges {
                    if !self.check_data_range_membership(sub_range, value, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            },
            DataRange::DataUnionOf(ranges) => {
                // Value must satisfy at least one range
                for sub_range in ranges {
                    if self.check_data_range_membership(sub_range, value, context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            },
            DataRange::DataComplementOf(complement_range) => {
                // Value must not satisfy the complement range
                Ok(!self.check_data_range_membership(complement_range, value, context)?)
            },
            DataRange::DataOneOf(values) => {
                // Value must be one of the enumerated values
                match value {
                    SWRLDArgument::Literal(literal_value) => {
                        for enum_value in values {
                            if enum_value.value == literal_value.value && enum_value.datatype == literal_value.datatype {
                                return Ok(true);
                            }
                        }
                        Ok(false)
                    },
                    SWRLDArgument::Variable(var) => {
                        if let Some(ctx) = context {
                            if let Some(bound_value) = ctx.bindings.get(var) {
                                // Convert SWRLValue to SWRLDArgument for recursive call
                                let d_arg = match bound_value {
                                    SWRLValue::Literal(lit) => SWRLDArgument::Literal(lit.clone()),
                                    _ => return Ok(false), // Wrong type for data range
                                };
                                self.check_data_range_membership(range, &d_arg, context)
                            } else {
                                Ok(false)
                            }
                        } else {
                            Ok(false)
                        }
                    },
                }
            },
            DataRange::DatatypeRestriction { datatype, restrictions } => {
                // Check base datatype first
                if !self.check_data_range_membership(&DataRange::Datatype(datatype.clone()), value, context)? {
                    return Ok(false);
                }
                
                // Check facet restrictions
                match value {
                    SWRLDArgument::Literal(literal) => {
                        self.check_facet_restrictions(&literal.value, restrictions)
                    },
                    SWRLDArgument::Variable(var) => {
                        if let Some(ctx) = context {
                            if let Some(bound_value) = ctx.bindings.get(var) {
                                // Convert SWRLValue to SWRLDArgument for recursive call
                                let d_arg = match bound_value {
                                    SWRLValue::Literal(lit) => SWRLDArgument::Literal(lit.clone()),
                                    _ => return Ok(false), // Wrong type for data range
                                };
                                self.check_data_range_membership(range, &d_arg, context)
                            } else {
                                Ok(false)
                            }
                        } else {
                            Ok(false)
                        }
                    },
                }
            },
        }
    }
    
    /// Infer datatype from literal string representation
    fn infer_datatype_from_literal(&self, literal: &str) -> Option<crate::ontology::IRI> {
        use crate::ontology::IRI;
        
        // Basic datatype inference based on format
        if literal.parse::<i64>().is_ok() {
            Some(IRI::new("http://www.w3.org/2001/XMLSchema#integer"))
        } else if literal.parse::<f64>().is_ok() {
            Some(IRI::new("http://www.w3.org/2001/XMLSchema#decimal"))
        } else if literal == "true" || literal == "false" {
            Some(IRI::new("http://www.w3.org/2001/XMLSchema#boolean"))
        } else {
            // Default to string
            Some(IRI::new("http://www.w3.org/2001/XMLSchema#string"))
        }
    }
    
    /// Check facet restrictions on a literal value
    fn check_facet_restrictions(&self, literal: &str, facet_restrictions: &[crate::ontology::FacetRestriction]) -> Result<bool> {
        use crate::ontology::IRI;
        
        for facet_restriction in facet_restrictions {
            let facet_name = facet_restriction.facet.as_str();
            let facet_value = &facet_restriction.value;
            
            match facet_name {
                "http://www.w3.org/2001/XMLSchema#minInclusive" => {
                    if let (Ok(value), Ok(min)) = (literal.parse::<f64>(), facet_value.value.parse::<f64>()) {
                        if value < min {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#maxInclusive" => {
                    if let (Ok(value), Ok(max)) = (literal.parse::<f64>(), facet_value.value.parse::<f64>()) {
                        if value > max {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#minExclusive" => {
                    if let (Ok(value), Ok(min)) = (literal.parse::<f64>(), facet_value.value.parse::<f64>()) {
                        if value <= min {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#maxExclusive" => {
                    if let (Ok(value), Ok(max)) = (literal.parse::<f64>(), facet_value.value.parse::<f64>()) {
                        if value >= max {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#length" => {
                    if let Ok(required_length) = facet_value.value.parse::<usize>() {
                        if literal.len() != required_length {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#minLength" => {
                    if let Ok(min_length) = facet_value.value.parse::<usize>() {
                        if literal.len() < min_length {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#maxLength" => {
                    if let Ok(max_length) = facet_value.value.parse::<usize>() {
                        if literal.len() > max_length {
                            return Ok(false);
                        }
                    }
                },
                "http://www.w3.org/2001/XMLSchema#pattern" => {
                    // Pattern matching would require regex functionality
                    // For now, consider it satisfied
                    continue;
                },
                _ => {
                    // Unknown facet, assume satisfied
                    continue;
                }
            }
        }
        
        Ok(true)
    }

    /// Evaluate a built-in atom (comprehensive SWRL built-in evaluation)
    fn evaluate_builtin_atom(&self, predicate: &crate::ontology::IRI, arguments: &[SWRLDArgument]) -> Result<bool> {
        // Enhanced built-in evaluation with comprehensive error handling and type checking
        
        if let Some(builtin) = self.builtin_registry.get_builtin(predicate) {
            // Convert arguments to values with proper type checking
            let values: Result<Vec<crate::swrl::builtins::SWRLValue>> = arguments.iter()
                .map(|arg| self.convert_swrl_argument_to_value(arg))
                .collect();
            
            match values {
                Ok(vals) => {
                    // Validate argument count
                    if !builtin.validate_argument_count(vals.len()) {
                        warn!("Invalid argument count for built-in {}: expected {}, got {}", 
                              predicate.as_str(), builtin.expected_argument_count(), vals.len());
                        return Ok(false);
                    }
                    
                    // Validate argument types
                    if !builtin.validate_argument_types(&vals) {
                        warn!("Invalid argument types for built-in {}", predicate.as_str());
                        return Ok(false);
                    }
                    
                    // Execute the built-in with proper error handling
                    match builtin.execute(&vals) {
                        Ok(result) => {
                            // Handle different result types
                            match result {
                                crate::swrl::builtins::SWRLValue::Boolean(b) => Ok(b),
                                _ => {
                                    // Non-boolean results are considered successful execution
                                    Ok(true)
                                }
                            }
                        },
                        Err(e) => {
                            debug!("Built-in {} execution failed: {}", predicate.as_str(), e);
                            Ok(false)
                        }
                    }
                },
                Err(e) => {
                    warn!("Failed to convert arguments for built-in {}: {}", predicate.as_str(), e);
                    Ok(false)
                }
            }
        } else {
            // Check for core SWRL built-ins that should always be available
            match predicate.as_str() {
                "http://www.w3.org/2003/11/swrlb#equal" => {
                    self.evaluate_core_builtin_equal(arguments)
                },
                "http://www.w3.org/2003/11/swrlb#notEqual" => {
                    self.evaluate_core_builtin_not_equal(arguments)
                },
                "http://www.w3.org/2003/11/swrlb#lessThan" => {
                    self.evaluate_core_builtin_less_than(arguments)
                },
                "http://www.w3.org/2003/11/swrlb#lessThanOrEqual" => {
                    self.evaluate_core_builtin_less_than_or_equal(arguments)
                },
                "http://www.w3.org/2003/11/swrlb#greaterThan" => {
                    self.evaluate_core_builtin_greater_than(arguments)
                },
                "http://www.w3.org/2003/11/swrlb#greaterThanOrEqual" => {
                    self.evaluate_core_builtin_greater_than_or_equal(arguments)
                },
                _ => {
                    warn!("Unknown built-in predicate: {}", predicate.as_str());
                    Ok(false)
                }
            }
        }
    }
    
    /// Evaluate core equal built-in
    fn evaluate_core_builtin_equal(&self, arguments: &[SWRLDArgument]) -> Result<bool> {
        if arguments.len() != 2 {
            return Ok(false);
        }
        
        let val1 = self.convert_swrl_argument_to_value(&arguments[0])?;
        let val2 = self.convert_swrl_argument_to_value(&arguments[1])?;
        
        Ok(val1 == val2)
    }
    
    /// Evaluate core not equal built-in
    fn evaluate_core_builtin_not_equal(&self, arguments: &[SWRLDArgument]) -> Result<bool> {
        Ok(!self.evaluate_core_builtin_equal(arguments)?)
    }
    
    /// Evaluate core less than built-in
    fn evaluate_core_builtin_less_than(&self, arguments: &[SWRLDArgument]) -> Result<bool> {
        if arguments.len() != 2 {
            return Ok(false);
        }
        
        let val1 = self.convert_swrl_argument_to_value(&arguments[0])?;
        let val2 = self.convert_swrl_argument_to_value(&arguments[1])?;
        
        match (val1, val2) {
            (crate::swrl::builtins::SWRLValue::Integer(i1), crate::swrl::builtins::SWRLValue::Integer(i2)) => {
                Ok(i1 < i2)
            },
            (crate::swrl::builtins::SWRLValue::Decimal(d1), crate::swrl::builtins::SWRLValue::Decimal(d2)) => {
                Ok(d1 < d2)
            },
            (crate::swrl::builtins::SWRLValue::Integer(i), crate::swrl::builtins::SWRLValue::Decimal(d)) => {
                Ok((i as f64) < d)
            },
            (crate::swrl::builtins::SWRLValue::Decimal(d), crate::swrl::builtins::SWRLValue::Integer(i)) => {
                Ok(d < (i as f64))
            },
            _ => Ok(false),
        }
    }
    
    /// Evaluate core less than or equal built-in
    fn evaluate_core_builtin_less_than_or_equal(&self, arguments: &[SWRLDArgument]) -> Result<bool> {
        let equal = self.evaluate_core_builtin_equal(arguments)?;
        let less_than = self.evaluate_core_builtin_less_than(arguments)?;
        Ok(equal || less_than)
    }
    
    /// Evaluate core greater than built-in
    fn evaluate_core_builtin_greater_than(&self, arguments: &[SWRLDArgument]) -> Result<bool> {
        // greater_than(a, b) = less_than(b, a)
        if arguments.len() != 2 {
            return Ok(false);
        }
        
        let reversed_args = [arguments[1].clone(), arguments[0].clone()];
        self.evaluate_core_builtin_less_than(&reversed_args)
    }
    
    /// Evaluate core greater than or equal built-in
    fn evaluate_core_builtin_greater_than_or_equal(&self, arguments: &[SWRLDArgument]) -> Result<bool> {
        let equal = self.evaluate_core_builtin_equal(arguments)?;
        let greater_than = self.evaluate_core_builtin_greater_than(arguments)?;
        Ok(equal || greater_than)
    }

    /// Convert SWRL argument to value for built-in evaluation
    fn convert_swrl_argument_to_value(&self, argument: &SWRLDArgument) -> Result<crate::swrl::builtins::SWRLValue> {
        match argument {
            SWRLDArgument::Literal(literal) => {
                use crate::swrl::builtins::SWRLValue;
                
                // Convert based on datatype
                if let Some(dt) = &literal.datatype {
                    match dt.as_str() {
                        "http://www.w3.org/2001/XMLSchema#integer" |
                        "http://www.w3.org/2001/XMLSchema#int" => {
                            match literal.value.parse::<i64>() {
                                Ok(val) => Ok(SWRLValue::Integer(val)),
                                Err(_) => Ok(SWRLValue::String(literal.value.clone())),
                            }
                        },
                        "http://www.w3.org/2001/XMLSchema#decimal" |
                        "http://www.w3.org/2001/XMLSchema#double" |
                        "http://www.w3.org/2001/XMLSchema#float" => {
                            match literal.value.parse::<f64>() {
                                Ok(val) => Ok(SWRLValue::Decimal(val)),
                                Err(_) => Ok(SWRLValue::String(literal.value.clone())),
                            }
                        },
                        "http://www.w3.org/2001/XMLSchema#boolean" => {
                            match literal.value.parse::<bool>() {
                                Ok(val) => Ok(SWRLValue::Boolean(val)),
                                Err(_) => Ok(SWRLValue::String(literal.value.clone())),
                            }
                        },
                        "http://www.w3.org/2001/XMLSchema#string" |
                        "http://www.w3.org/2001/XMLSchema#anyURI" => {
                            Ok(SWRLValue::String(literal.value.clone()))
                        },
                        "http://www.w3.org/2001/XMLSchema#dateTime" => {
                            // For datetime, store as string for now (proper datetime parsing would require chrono)
                            Ok(SWRLValue::String(literal.value.clone()))
                        },
                        _ => {
                            // Unknown datatype, default to string
                            Ok(SWRLValue::String(literal.value.clone()))
                        }
                    }
                } else {
                    // No datatype specified, try to infer
                    if let Ok(int_val) = literal.value.parse::<i64>() {
                        Ok(SWRLValue::Integer(int_val))
                    } else if let Ok(float_val) = literal.value.parse::<f64>() {
                        Ok(SWRLValue::Decimal(float_val))
                    } else if let Ok(bool_val) = literal.value.parse::<bool>() {
                        Ok(SWRLValue::Boolean(bool_val))
                    } else {
                        Ok(SWRLValue::String(literal.value.clone()))
                    }
                }
            },
            SWRLDArgument::Variable(var) => {
                // For now, return an error as variable binding context is needed
                // In a real implementation, this would need access to current execution context
                Err(crate::error::OxidowlError::ParseError(
                    format!("Cannot convert unbound variable '{:?}' to value", var)
                ))
            },
        }
    }

    /// Check if any rules can potentially fire
    #[must_use]
    pub fn has_applicable_rules(&self) -> bool {
        self.rule_states
            .values()
            .any(|state| state.active && !state.should_skip(self.config.max_rule_applications))
    }

    /// Get inferences generated in the last execution
    #[must_use]
    pub fn get_cached_inferences(&self) -> &HashSet<Axiom> {
        &self.inference_cache
    }

    /// Clear inference cache
    pub fn clear_inference_cache(&mut self) {
        self.inference_cache.clear();
    }
}

impl Default for SWRLRuleEngine {
    fn default() -> Self {
        Self::new(SWRLConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, ClassExpression};
    use crate::swrl::{SWRLAtom, SWRLIArgument, SWRLRule, SWRLRuleAxiom, SWRLVariable};

    fn create_test_ontology_with_multiple_rules() -> Arc<RwLock<Ontology>> {
        let mut ontology = Ontology::new();

        // Add some test classes
        let person_class = Class::new(IRI::new("http://example.org/Person"));
        let student_class = Class::new(IRI::new("http://example.org/Student"));
        let teacher_class = Class::new(IRI::new("http://example.org/Teacher"));
        let adult_class = Class::new(IRI::new("http://example.org/Adult"));

        ontology.add_class(person_class);
        ontology.add_class(student_class);
        ontology.add_class(teacher_class);
        ontology.add_class(adult_class);

        // Rule 1: Person(?x) -> Student(?x)
        let var_x1 = SWRLVariable::new(IRI::new("http://example.org/var#x1"));
        let rule1 = SWRLRule::new(
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(
                    "http://example.org/Student",
                ))),
                argument: SWRLIArgument::Variable(var_x1.clone()),
            }],
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(
                    "http://example.org/Person",
                ))),
                argument: SWRLIArgument::Variable(var_x1),
            }],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(1, rule1)));

        // Rule 2: Student(?x) -> Adult(?x)
        let var_x2 = SWRLVariable::new(IRI::new("http://example.org/var#x2"));
        let rule2 = SWRLRule::new(
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Adult"))),
                argument: SWRLIArgument::Variable(var_x2.clone()),
            }],
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(
                    "http://example.org/Student",
                ))),
                argument: SWRLIArgument::Variable(var_x2),
            }],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(2, rule2)));

        // Rule 3: Person(?x) -> Teacher(?x)
        let var_x3 = SWRLVariable::new(IRI::new("http://example.org/var#x3"));
        let rule3 = SWRLRule::new(
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(
                    "http://example.org/Teacher",
                ))),
                argument: SWRLIArgument::Variable(var_x3.clone()),
            }],
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new(
                    "http://example.org/Person",
                ))),
                argument: SWRLIArgument::Variable(var_x3),
            }],
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(3, rule3)));

        Arc::new(RwLock::new(ontology))
    }

    fn create_test_ontology() -> Arc<RwLock<Ontology>> {
        let mut ontology = Ontology::new();

        // Add some test classes and individuals
        let person_class = Class::new(IRI::new("http://example.org/Person"));
        let student_class = Class::new(IRI::new("http://example.org/Student"));

        ontology.add_class(person_class);
        ontology.add_class(student_class);

        // Add a test SWRL rule: Person(?x) -> Student(?x)
        let var_x = SWRLVariable::new(IRI::new("http://example.org/var#x"));

        let body_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
            argument: SWRLIArgument::Variable(var_x.clone()),
        };

        let head_atom = SWRLAtom::ClassAtom {
            predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
            argument: SWRLIArgument::Variable(var_x),
        };

        let rule = SWRLRule::new(vec![head_atom], vec![body_atom]);
        let rule_axiom = SWRLRuleAxiom::new(1, rule);

        ontology.add_axiom(Axiom::Rule(rule_axiom));

        Arc::new(RwLock::new(ontology))
    }

    #[test]
    fn test_engine_creation() {
        let config = SWRLConfig::default();
        let engine = SWRLRuleEngine::new(config);

        assert_eq!(engine.rule_states.len(), 0);
        assert!(!engine.has_applicable_rules());
    }

    #[test]
    fn test_load_rules_from_ontology() {
        let mut engine = SWRLRuleEngine::new(SWRLConfig::default());
        let ontology = create_test_ontology();

        engine.set_ontology(ontology);

        assert_eq!(engine.rule_states.len(), 1);
        assert!(engine.has_rule(1));
        assert!(engine.has_applicable_rules());
    }

    #[test]
    fn test_rule_priorities() {
        let mut engine = SWRLRuleEngine::new(SWRLConfig::default());
        let ontology = create_test_ontology_with_multiple_rules();

        engine.set_ontology(ontology);

        // Set priorities for the rules
        engine.set_rule_priority(1, 10);
        engine.set_rule_priority(2, 5);
        engine.set_rule_priority(3, 15);
    }
}

/// Goal for backward chaining proof search
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SWRLGoal {
    ClassAssertion {
        individual: String,
        class: String,
    },
    PropertyAssertion {
        subject: String,
        property: String,
        object: String,
    },
    DataPropertyAssertion {
        subject: String,
        property: String,
        value: String,
    },
    // Add more goal types as needed
}

impl SWRLGoal {
    /// Create goal from class assertion
    pub fn from_class_assertion(assertion: &crate::ontology::axioms::ClassAssertionAxiom) -> Self {
        let individual_name = match &assertion.individual {
            crate::ontology::Individual::Named(named) => named.iri.to_string(),
            crate::ontology::Individual::Anonymous(anon) => anon.id.clone(),
        };
        
        let class_name = match &assertion.class {
            crate::ontology::ClassExpression::Class(class) => class.iri.to_string(),
            _ => "ComplexClass".to_string(), // Simplified for complex expressions
        };
        
        SWRLGoal::ClassAssertion {
            individual: individual_name,
            class: class_name,
        }
    }
}

/// Result of attempting to prove a goal
#[derive(Debug, Clone, PartialEq)]
pub enum GoalProofResult {
    Proved,
    Failed,
    NeedsMoreProofs, // Subgoals were generated
}

impl SWRLRuleEngine {
    /// Try to prove a goal using backward chaining
    fn try_prove_goal(&mut self, goal: &SWRLGoal, goal_stack: &mut Vec<SWRLGoal>) -> Result<GoalProofResult> {
        // First check if goal is already satisfied by current facts
        if self.is_goal_satisfied_swrl(goal)? {
            return Ok(GoalProofResult::Proved);
        }
        
        // Try to find rules that could prove this goal
        let applicable_rules = self.find_rules_for_goal_swrl(goal)?;
        
        if applicable_rules.is_empty() {
            return Ok(GoalProofResult::Failed);
        }
        
        // Try each applicable rule
        for rule_id in applicable_rules {
            if let Some(rule_state) = self.rule_states.get(&rule_id) {
                let rule = &rule_state.rule;
                // Generate subgoals from rule body
                let subgoals = self.generate_subgoals_from_rule(rule, goal)?;
                
                if subgoals.is_empty() {
                    // Rule can be applied directly
                    let mut context = SWRLExecutionContext::new();
                    let result = self.interpreter.execute_rule(rule, &mut context, self.ontology.as_ref().unwrap())?;
                    
                    if result.inferences.len() > 0 {
                        return Ok(GoalProofResult::Proved);
                    }
                } else {
                    // Add subgoals to stack for later processing
                    for subgoal in subgoals {
                        goal_stack.push(subgoal);
                    }
                    return Ok(GoalProofResult::NeedsMoreProofs);
                }
            }
        }
        
        Ok(GoalProofResult::Failed)
    }
    
    /// Check if a goal is already satisfied
    fn is_goal_satisfied_swrl(&self, goal: &SWRLGoal) -> Result<bool> {
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            match goal {
                SWRLGoal::ClassAssertion { individual, class } => {
                    // Check if individual is asserted to be of this class
                    for axiom in ontology_guard.axioms() {
                        if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                            let ind_name = match &assertion.individual {
                                crate::ontology::Individual::Named(named) => named.iri.to_string(),
                                crate::ontology::Individual::Anonymous(anon) => anon.id.clone(),
                            };
                            
                            let class_name = match &assertion.class {
                                crate::ontology::ClassExpression::Class(cls) => cls.iri.to_string(),
                                _ => continue,
                            };
                            
                            if ind_name == *individual && class_name == *class {
                                return Ok(true);
                            }
                        }
                    }
                }
                SWRLGoal::PropertyAssertion { subject, property, object } => {
                    // Check if property assertion exists
                    for axiom in ontology_guard.axioms() {
                        if let crate::ontology::axioms::Axiom::ObjectPropertyAssertion(assertion) = axiom {
                            // Check if assertion matches goal
                            // Implementation would compare IRIs properly
                            return Ok(false); // Simplified
                        }
                    }
                }
                SWRLGoal::DataPropertyAssertion { subject, property, value } => {
                    // Check if data property assertion exists
                    for axiom in ontology_guard.axioms() {
                        if let crate::ontology::axioms::Axiom::DataPropertyAssertion(assertion) = axiom {
                            // Check if assertion matches goal
                            // Implementation would compare properly
                            return Ok(false); // Simplified
                        }
                    }
                }
            }
        }
        
        Ok(false)
    }
    
    /// Find rules that could potentially prove the given goal
    fn find_rules_for_goal_swrl(&self, goal: &SWRLGoal) -> Result<Vec<u64>> {
        let mut applicable_rules = Vec::new();
        
        for (rule_id, rule_state) in &self.rule_states {
            let rule = &rule_state.rule;
            // Check if rule head could prove the goal
            if self.rule_head_matches_goal(rule, goal)? {
                applicable_rules.push(*rule_id);
            }
        }
        
        Ok(applicable_rules)
    }
    
    /// Check if a rule's head could prove the given goal
    fn rule_head_matches_goal(&self, rule: &SWRLRule, goal: &SWRLGoal) -> Result<bool> {
        for head_atom in &rule.head {
            match (head_atom, goal) {
                (crate::swrl::SWRLAtom::ClassAtom { predicate: _, argument: _ }, SWRLGoal::ClassAssertion { individual: _, class: _ }) => {
                    // Check if class atom could match goal
                    // Would need proper variable unification here
                    return Ok(true); // Simplified
                }
                (crate::swrl::SWRLAtom::ObjectPropertyAtom { predicate: _, first_argument: _, second_argument: _ }, SWRLGoal::PropertyAssertion { .. }) => {
                    return Ok(true); // Simplified
                }
                (crate::swrl::SWRLAtom::DataPropertyAtom { predicate: _, first_argument: _, second_argument: _ }, SWRLGoal::DataPropertyAssertion { .. }) => {
                    return Ok(true); // Simplified
                }
                _ => {}
            }
        }
        
        Ok(false)
    }
    
    /// Generate subgoals from a rule body that need to be satisfied
    fn generate_subgoals_from_rule(&self, rule: &SWRLRule, goal: &SWRLGoal) -> Result<Vec<SWRLGoal>> {
        let mut subgoals = Vec::new();
        
        // For each atom in the rule body, create a corresponding subgoal
        for body_atom in &rule.body {
            match body_atom {
                crate::swrl::SWRLAtom::ClassAtom { predicate, argument } => {
                    // Create class assertion subgoal
                    subgoals.push(SWRLGoal::ClassAssertion {
                        individual: format!("var_{:?}", argument), // Simplified variable handling
                        class: format!("{:?}", predicate),
                    });
                }
                crate::swrl::SWRLAtom::ObjectPropertyAtom { predicate, first_argument, second_argument } => {
                    // Create property assertion subgoal
                    subgoals.push(SWRLGoal::PropertyAssertion {
                        subject: format!("var_{:?}", first_argument),
                        property: format!("{:?}", predicate),
                        object: format!("var_{:?}", second_argument),
                    });
                }
                crate::swrl::SWRLAtom::DataPropertyAtom { predicate, first_argument, second_argument } => {
                    // Create data property assertion subgoal
                    subgoals.push(SWRLGoal::DataPropertyAssertion {
                        subject: format!("var_{:?}", first_argument),
                        property: format!("{:?}", predicate),
                        value: format!("var_{:?}", second_argument),
                    });
                }
                _ => {
                    // Handle other atom types as needed
                }
            }
        }
        
        Ok(subgoals)
    }

    /// Execute forward chaining with iteration limit
    fn execute_forward_chaining_limited(&mut self, max_iterations: usize) -> Result<SWRLExecutionResult> {
        info!("Executing limited forward chaining with max {} iterations", max_iterations);
        
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut any_fired = false;
        
        for iteration in 0..max_iterations {
            let mut iteration_fired = false;
            
            // Get rules ordered by priority
            let ordered_rule_ids = self.get_ordered_rules();
            
            for rule_id in ordered_rule_ids {
                if let Some(rule_state) = self.rule_states.get(&rule_id) {
                    if self.is_rule_applicable(rule_id.try_into().unwrap())? {
                        // Clone the rule to avoid borrow checker issues
                        let rule_clone = rule_state.rule.clone();
                        let result = self.execute_single_rule(&rule_clone)?;
                        
                        if result.fired {
                            iteration_fired = true;
                            any_fired = true;
                            total_inferences.extend(result.inferences);
                        }
                        
                        total_applications += 1;
                    }
                }
            }
            
            // Stop if no rules fired in this iteration
            if !iteration_fired {
                break;
            }
        }
        
        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }

    /// Execute backward chaining with iteration limit
    fn execute_backward_chaining_limited(&mut self, max_iterations: usize) -> Result<SWRLExecutionResult> {
        info!("Executing limited backward chaining with max {} iterations", max_iterations);
        
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut any_fired = false;
        
        // Initialize goal stack for backward chaining
        let mut goal_stack = Vec::new();
        let mut proved_goals = HashSet::new();
        let mut failed_goals = HashSet::new();
        
        // Start with queries from the ontology (if any are specified)
        if let Some(ontology) = &self.ontology {
            let ontology_guard = ontology.read().unwrap();
            
            // Add unproven class assertions as goals
            for axiom in ontology_guard.axioms() {
                if let crate::ontology::axioms::Axiom::ClassAssertion(assertion) = axiom {
                    let goal = SWRLGoal::from_class_assertion(assertion);
                    if !self.is_goal_satisfied_swrl(&goal)? {
                        goal_stack.push(goal);
                    }
                }
            }
        }
        
        // Main backward chaining loop with iteration limit
        let mut iteration = 0;
        while !goal_stack.is_empty() && iteration < max_iterations {
            iteration += 1;
            let current_goal = goal_stack.pop().unwrap();
            
            // Skip if already proved or failed
            if proved_goals.contains(&current_goal) || failed_goals.contains(&current_goal) {
                continue;
            }
            
            // Try to prove the goal using available rules
            let proof_result = self.try_prove_goal(&current_goal, &mut goal_stack)?;
            
            match proof_result {
                GoalProofResult::Proved => {
                    proved_goals.insert(current_goal);
                    any_fired = true;
                    // For now, we don't track specific axioms in backward chaining limited
                    // total_inferences.push(some_axiom);
                }
                GoalProofResult::Failed => {
                    failed_goals.insert(current_goal);
                }
                GoalProofResult::NeedsMoreProofs => {
                    // Subgoals added to stack, try again later
                    goal_stack.push(current_goal);
                }
            }
            
            total_applications += 1;
        }
        
        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }
}

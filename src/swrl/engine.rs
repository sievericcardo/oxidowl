//! SWRL Rule Engine
//!
//! This module implements the core SWRL rule execution engine that coordinates
//! rule firing, inference generation, and integration with the tableau reasoner.

use crate::ontology::{Axiom, Ontology};
use crate::swrl::{
    SWRLAtom, SWRLConfig, SWRLExecutionContext, SWRLExecutionResult, SWRLReasoningStrategy,
    SWRLRule, SWRLRuleState, SWRLStatistics, SWRLDArgument, SWRLIArgument,
    builtins::{SWRLBuiltIn, SWRLBuiltInRegistry},
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
        // For now, implement a simplified backward chaining
        // A full implementation would require goal tracking and proof search
        warn!("Backward chaining not fully implemented, falling back to forward chaining");
        self.execute_forward_chaining()
    }

    /// Execute hybrid reasoning strategy
    fn execute_hybrid_reasoning(&mut self) -> Result<SWRLExecutionResult> {
        // Combine forward and backward chaining
        let forward_result = self.execute_forward_chaining()?;

        // For now, just return forward chaining results
        // A full implementation would interleave forward and backward reasoning
        Ok(forward_result)
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
                self.check_class_membership(predicate, argument)
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
                self.check_data_range_membership(predicate, argument)
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

    /// Check if two atoms can unify (simplified implementation)
    fn atoms_unify(&self, atom1: &SWRLAtom, atom2: &SWRLAtom) -> Result<bool> {
        match (atom1, atom2) {
            (SWRLAtom::ClassAtom { predicate: p1, .. }, SWRLAtom::ClassAtom { predicate: p2, .. }) => {
                Ok(p1 == p2)
            }
            (SWRLAtom::ObjectPropertyAtom { predicate: p1, .. }, SWRLAtom::ObjectPropertyAtom { predicate: p2, .. }) => {
                Ok(p1 == p2)
            }
            (SWRLAtom::DataPropertyAtom { predicate: p1, .. }, SWRLAtom::DataPropertyAtom { predicate: p2, .. }) => {
                Ok(p1 == p2)
            }
            _ => Ok(false),
        }
    }

    /// Check class membership using ontology reasoning
    fn check_class_membership(&self, class: &crate::ontology::ClassExpression, individual: &SWRLIArgument) -> Result<bool> {
        // Convert SWRL individual argument to ontology individual
        let individual_iri = match individual {
            SWRLIArgument::Individual(ind) => ind.iri(),
            SWRLIArgument::Variable(_) => return Ok(false), // Cannot check unbound variables
        };
        
        // Simplified implementation - would need proper ontology access
        Ok(false)
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
                for axiom in &ontology.axioms {
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
                for axiom in &ontology.axioms {
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
                for axiom in &ontology.axioms {
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
                for axiom in &ontology.axioms {
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
    fn check_data_range_membership(&self, _range: &crate::ontology::DataRange, _value: &SWRLDArgument) -> Result<bool> {
        // Simplified implementation for now
        Ok(false)
    }

    /// Evaluate a built-in atom (placeholder implementation)
    fn evaluate_builtin_atom(&self, predicate: &crate::ontology::IRI, arguments: &[SWRLDArgument]) -> Result<bool> {
        if let Some(builtin) = self.builtin_registry.get_builtin(predicate) {
            // Convert arguments to values
            let values: Result<Vec<crate::swrl::builtins::SWRLValue>> = arguments.iter()
                .map(|arg| self.convert_swrl_argument_to_value(arg))
                .collect();
            
            match values {
                Ok(vals) => {
                    match builtin.execute(&vals) {
                        Ok(_) => Ok(true),
                        Err(_) => Ok(false),
                    }
                },
                Err(_) => Ok(false),
            }
        } else {
            warn!("Unknown built-in predicate: {}", predicate.as_str());
            Ok(false)
        }
    }

    /// Convert SWRL argument to value for built-in evaluation
    fn convert_swrl_argument_to_value(&self, argument: &SWRLDArgument) -> Result<crate::swrl::builtins::SWRLValue> {
        match argument {
            SWRLDArgument::Literal(literal) => {
                // Simple conversion to string for now
                Ok(crate::swrl::builtins::SWRLValue::String(literal.value.clone()))
            }
            SWRLDArgument::Variable(_) => {
                // Variables should be resolved to concrete values before this point
                Err(crate::error::OxidowlError::ParseError("Cannot convert unbound variable to value".to_string()))
            }
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

        let ordered = engine.get_ordered_rules();
        // Should be ordered by priority: 3 (15), 1 (10), 2 (5)
        assert_eq!(ordered, vec![3, 1, 2]);
    }

    #[test]
    fn test_rule_activation() {
        let mut engine = SWRLRuleEngine::new(SWRLConfig::default());
        let ontology = create_test_ontology();

        engine.set_ontology(ontology);

        assert!(engine.has_applicable_rules());

        engine.set_rule_active(1, false).unwrap();
        assert!(!engine.has_applicable_rules());

        engine.set_rule_active(1, true).unwrap();
        assert!(engine.has_applicable_rules());
    }

    #[test]
    fn test_statistics_tracking() {
        let engine = SWRLRuleEngine::new(SWRLConfig::default());
        let stats = engine.get_statistics();

        assert_eq!(stats.total_rule_applications, 0);
        assert_eq!(stats.rules_fired, 0);
        assert_eq!(stats.inferences_generated, 0);
    }
}

//! SWRL Rule Execution Strategies
//!
//! This module implements different inference strategies:
//! - Forward chaining: data-driven rule execution
//! - Backward chaining: goal-driven rule execution  
//! - Hybrid reasoning: combination of both strategies

use crate::ontology::{Axiom, Ontology};
use crate::swrl::{SWRLAtom, SWRLExecutionContext, SWRLExecutionResult, SWRLRule, SWRLIArgument, SWRLFact};
use crate::{Error, Result};
use crate::core::lock_helpers::{read_lock, write_lock};
use log::{debug, info, warn};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

use super::core::SWRLRuleEngine;

/// Forward chaining inference engine
#[derive(Debug)]
pub struct ForwardChaining {
    /// Maximum iterations to prevent infinite loops
    max_iterations: usize,
}

impl ForwardChaining {
    /// Create a new forward chaining engine
    pub fn new() -> Self {
        Self {
            max_iterations: 1000,
        }
    }

    /// Execute forward chaining strategy
    pub fn execute(
        &mut self, 
        rules: &[SWRLRule],
        facts: &mut Vec<SWRLFact>,
        ontology: &Arc<Ontology>,
        context: &mut SWRLExecutionContext
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();
        
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut iteration = 0;

        // Continue until no new inferences are generated
        loop {
            iteration += 1;
            let mut iteration_inferences = Vec::new();
            let mut iteration_fired = false;

            debug!("Forward chaining iteration {}", iteration);

            for rule in rules {
                // Try to apply the rule with current facts
                let matches = self.find_rule_matches(rule, facts, context)?;
                
                for var_binding in matches {
                    // Apply rule with this binding
                    let new_facts = self.apply_rule_with_binding(rule, &var_binding, facts, ontology, context)?;
                    
                    // Add new facts
                    for new_fact in new_facts {
                        if !facts.contains(&new_fact) {
                            facts.push(new_fact.clone());
                            iteration_inferences.push(new_fact);
                            iteration_fired = true;
                            total_applications += 1;
                        }
                    }
                }
            }

            total_inferences.extend(iteration_inferences);

            // Stop if no new inferences or max iterations reached
            if !iteration_fired || iteration >= self.max_iterations {
                break;
            }
        }

        Ok(SWRLExecutionResult {
            inferences: total_inferences,
            applications: total_applications,
            execution_time: start_time.elapsed(),
            strategy: "ForwardChaining".to_string(),
        })
    }
    
    /// Find variable bindings that make the rule body match current facts
    fn find_rule_matches(
        &self,
        _rule: &SWRLRule, 
        _facts: &[SWRLFact],
        _context: &SWRLExecutionContext
    ) -> Result<Vec<std::collections::HashMap<String, SWRLIArgument>>> {
        // Placeholder implementation - would need actual unification logic
        Ok(Vec::new())
    }

    /// Apply a rule with specific variable bindings to generate new facts
    fn apply_rule_with_binding(
        &self,
        _rule: &SWRLRule,
        _binding: &std::collections::HashMap<String, SWRLIArgument>,
        _facts: &[SWRLFact],
        _ontology: &Arc<Ontology>,
        _context: &SWRLExecutionContext
    ) -> Result<Vec<SWRLFact>> {
        // Placeholder implementation - would need actual rule application logic
        Ok(Vec::new())
    }
}

/// Backward chaining inference engine
#[derive(Debug)]
pub struct BackwardChaining {
    /// Maximum goal depth
    max_goal_depth: usize,
}
}
                }
            }

            // Apply new inferences to the ontology
            if !iteration_inferences.is_empty() {
                engine.apply_inferences_to_ontology(iteration_inferences)?;
            }

            // Stop if no rules fired in this iteration
            if !iteration_fired {
                break;
            }

            // Safety check for infinite loops
            if iteration > self.max_iterations {
                warn!("Forward chaining stopped after {} iterations", self.max_iterations);
                break;
            }
        }

        let execution_time = start_time.elapsed();
        engine.statistics.total_reasoning_time_us += execution_time.as_micros() as u64;

        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }

    /// Execute a single SWRL rule
    fn execute_single_rule(
        &self,
        engine: &mut SWRLRuleEngine,
        rule: &SWRLRule,
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();

        if engine.config.debug {
            debug!("Executing rule: {:?}", rule);
        }

        // Check timeout
        if let Some(timeout_ms) = engine.config.timeout_ms {
            if start_time.elapsed().as_millis() > timeout_ms as u128 {
                return Err(Error::reasoning("SWRL rule execution timeout"));
            }
        }

        let mut context = SWRLExecutionContext::new();
        context.max_depth = engine.config.max_execution_depth;

        let ontology = engine.ontology.as_ref()
            .ok_or_else(|| Error::reasoning("No ontology set for SWRL execution"))?;

        let result = engine.interpreter.execute_rule(
            rule,
            &mut context,
            ontology,
        )?;

        let execution_time = start_time.elapsed();
        let mut result_with_time = result;
        result_with_time.execution_time_us = execution_time.as_micros() as u64;

        Ok(result_with_time)
    }
}

impl Default for ForwardChaining {
    fn default() -> Self {
        Self::new()
    }
}

/// Backward chaining inference engine
#[derive(Debug)]
pub struct BackwardChaining {
    /// Maximum goal depth
    max_goal_depth: usize,
}

impl BackwardChaining {
    /// Create a new backward chaining engine
    pub fn new() -> Self {
        Self {
            max_goal_depth: 100,
        }
    }

    impl BackwardChaining {
    /// Create a new backward chaining engine
    pub fn new() -> Self {
        Self {
            max_goal_depth: 100,
        }
    }

    /// Execute backward chaining strategy
    pub fn execute(
        &mut self,
        rules: &[SWRLRule],
        facts: &mut Vec<SWRLFact>,
        ontology: &Arc<Ontology>,
        context: &mut SWRLExecutionContext
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();
        
        // For general execution, we don't have specific goals
        // This is a placeholder implementation
        Ok(SWRLExecutionResult {
            inferences: Vec::new(),
            applications: 0,
            execution_time: start_time.elapsed(),
            strategy: "BackwardChaining".to_string(),
        })
    }

    /// Execute rules with a specific query goal
    pub fn execute_with_goal(
        &mut self,
        rules: &[SWRLRule],
        facts: &mut Vec<SWRLFact>,
        ontology: &Arc<Ontology>,
        context: &mut SWRLExecutionContext,
        goal: &SWRLAtom,
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();
        
        let mut total_inferences = Vec::new();
        let mut total_applications = 0;
        let mut goal_stack = vec![goal.clone()];
        let mut visited_goals = HashSet::new();

        while let Some(current_goal) = goal_stack.pop() {
            // Prevent infinite recursion
            let goal_key = format!("{:?}", current_goal);
            if visited_goals.contains(&goal_key) {
                continue;
            }

            if visited_goals.len() >= self.max_goal_depth {
                warn!("Maximum goal depth reached, stopping backward chaining");
                break;
            }

            visited_goals.insert(goal_key);

            // Check if goal is already satisfied by known facts
            if self.is_goal_satisfied(&current_goal, facts)? {
                continue;
            }

            // Find rules that could prove this goal
            for rule in rules {
                if self.can_rule_prove_goal(rule, &current_goal) {
                    // Add rule body as subgoals
                    for atom in &rule.body {
                        goal_stack.push(atom.clone());
                    }
                    total_applications += 1;
                }
            }
        }

        Ok(SWRLExecutionResult {
            inferences: total_inferences,
            applications: total_applications,
            execution_time: start_time.elapsed(),
            strategy: "BackwardChaining".to_string(),
        })
    }

    /// Check if a goal is satisfied by current facts
    fn is_goal_satisfied(&self, _goal: &SWRLAtom, _facts: &[SWRLFact]) -> Result<bool> {
        // Placeholder implementation
        Ok(false)
    }

    /// Check if a rule can prove a given goal
    fn can_rule_prove_goal(&self, _rule: &SWRLRule, _goal: &SWRLAtom) -> bool {
        // Placeholder implementation - would need actual unification
        false
    }
}

    /// Execute rules with a specific query goal
    pub fn execute_with_goal(
        &mut self,
        engine: &mut SWRLRuleEngine,
        goal: &SWRLAtom,
    ) -> Result<SWRLExecutionResult> {
        let mut result = SWRLExecutionResult::empty();
        let mut goal_stack = vec![goal.clone()];
        let mut visited_goals = HashSet::new();
        let max_depth = engine.config.max_rule_applications;

        while let Some(current_goal) = goal_stack.pop() {
            // Prevent infinite recursion
            let goal_key = format!("{:?}", current_goal);
            if visited_goals.contains(&goal_key) {
                continue;
            }

            if visited_goals.len() >= max_depth {
                warn!("Maximum goal depth reached, stopping backward chaining");
                break;
            }

            visited_goals.insert(goal_key);

            // Check if goal is already satisfied by known facts
            if engine.goal_checker.is_goal_satisfied(engine, &current_goal)? {
                continue;
            }

            // Find rules that can prove this goal and add subgoals
            let applicable_rules = engine.goal_checker.find_rules_for_goal(engine, &current_goal)?;

            for rule_id in applicable_rules {
                // Process rule and add any subgoals to the stack
                if let Some(subgoals) = self.process_rule_for_goal(engine, rule_id, &current_goal)? {
                    goal_stack.extend(subgoals);
                    result.applications += 1;
                    result.fired = true;
                }
            }
        }

        Ok(result)
    }

    /// Try to prove a goal using backward chaining
    fn try_prove_goal(
        &self,
        engine: &mut SWRLRuleEngine,
        goal: &SWRLAtom,
        goal_stack: &mut Vec<SWRLAtom>,
    ) -> Result<GoalProofResult> {
        // Check if goal is already satisfied
        if engine.goal_checker.is_goal_satisfied(engine, goal)? {
            return Ok(GoalProofResult::Proved);
        }

        // Find applicable rules
        let applicable_rules = engine.goal_checker.find_rules_for_goal(engine, goal)?;

        if applicable_rules.is_empty() {
            return Ok(GoalProofResult::Failed);
        }

        // Try each applicable rule
        for rule_id in applicable_rules {
            if let Some(subgoals) = self.process_rule_for_goal(engine, rule_id, goal)? {
                goal_stack.extend(subgoals);
                return Ok(GoalProofResult::NeedsMoreProofs);
            }
        }

        Ok(GoalProofResult::Failed)
    }

    /// Process a rule to generate subgoals for a given goal
    fn process_rule_for_goal(
        &self,
        engine: &SWRLRuleEngine,
        rule_id: u64,
        _goal: &SWRLAtom,
    ) -> Result<Option<Vec<SWRLAtom>>> {
        // Get the rule from the ontology
        if let Some(ontology) = &engine.ontology {
            let ontology_guard = read_lock(ontology, "SWRL inference: reading ontology for rule processing")?;
            
            for axiom in ontology_guard.axioms() {
                if let Axiom::Rule(rule_axiom) = axiom {
                    if rule_axiom.id == rule_id {
                        // Extract subgoals from rule body
                        let subgoals = engine.goal_checker.extract_subgoals_from_rule_body(
                            &rule_axiom.rule.body
                        )?;
                        return Ok(Some(subgoals));
                    }
                }
            }
        }

        Ok(None)
    }
    
    /// Convert a class assertion to a SWRL atom goal
    fn convert_class_assertion_to_goal(&self, assertion: &crate::ontology::axioms::ClassAssertionAxiom) -> Result<SWRLAtom> {
        Ok(SWRLAtom::ClassAtom {
            predicate: assertion.class_expression.clone(),
            argument: SWRLIArgument::Individual(assertion.individual.clone()),
        })
    }
}

impl Default for BackwardChaining {
    fn default() -> Self {
        Self::new()
    }
}

/// Hybrid reasoning engine combining forward and backward chaining
#[derive(Debug)]
pub struct HybridReasoning {
    forward_engine: ForwardChaining,
    backward_engine: BackwardChaining,
}

impl HybridReasoning {
    /// Create a new hybrid reasoning engine
    pub fn new() -> Self {
        Self {
            forward_engine: ForwardChaining::new(),
            backward_engine: BackwardChaining::new(),
        }
    }

    /// Execute hybrid reasoning strategy
    pub fn execute(
        &mut self,
        rules: &[SWRLRule],
        facts: &mut Vec<SWRLFact>,
        ontology: &Arc<Ontology>,
        context: &mut SWRLExecutionContext
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();
        
        // Phase 1: Forward chaining to establish base facts
        info!("Hybrid reasoning - Phase 1: Forward chaining");
        let forward_result = self.forward_engine.execute(rules, facts, ontology, context)?;

        // Phase 2: Backward chaining (if there are specific goals)
        info!("Hybrid reasoning - Phase 2: Backward chaining");
        let backward_result = self.backward_engine.execute(rules, facts, ontology, context)?;

        // Combine results
        let mut total_inferences = forward_result.inferences;
        total_inferences.extend(backward_result.inferences);

        Ok(SWRLExecutionResult {
            inferences: total_inferences,
            applications: forward_result.applications + backward_result.applications,
            execution_time: start_time.elapsed(),
            strategy: "HybridReasoning".to_string(),
        })
    }
}

        Ok(SWRLExecutionResult::new(
            any_fired,
            total_inferences,
            total_applications,
        ))
    }

}

impl Default for ForwardChaining {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for BackwardChaining {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for HybridReasoning {
    fn default() -> Self {
        Self::new()
    }
}
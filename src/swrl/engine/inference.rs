//! SWRL Rule Execution Strategies
//!
//! This module implements different inference strategies:
//! - Forward chaining: data-driven rule execution
//! - Backward chaining: goal-driven rule execution  
//! - Hybrid reasoning: combination of both strategies

use crate::ontology::{Axiom, Ontology};
use crate::swrl::{SWRLAtom, SWRLExecutionContext, SWRLExecutionResult, SWRLRule, SWRLIArgument};
use crate::{Error, Result};
use log::{debug, info, warn};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Instant;

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
        known_facts: &mut Vec<SWRLAtom>,
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
                let matches = self.find_rule_matches(rule, known_facts, context)?;
                
                for var_binding in matches {
                    // Apply rule with this binding
                    let new_facts = self.apply_rule_with_binding(rule, &var_binding, known_facts, ontology, context)?;
                    
                    // Add new facts
                    for new_fact in new_facts {
                        if !known_facts.contains(&new_fact) {
                            known_facts.push(new_fact.clone());
                            iteration_inferences.push(new_fact);
                            iteration_fired = true;
                            total_applications += 1;
                        }
                    }
                }
            }

            // Convert SWRLAtom to Axiom for the inferences
            let axiom_inferences: Vec<Axiom> = iteration_inferences.into_iter()
                .map(|_atom| {
                    // For now, create a simple fact assertion from the atom
                    // In a more sophisticated implementation, this would convert
                    // SWRL atoms to appropriate OWL axioms
                    Axiom::Declaration(crate::ontology::axioms::DeclarationAxiom {
                        id: 0, // Generate proper ID
                        entity: crate::ontology::axioms::Entity::NamedIndividual(
                            crate::ontology::IRI::new("http://example.org/swrl_inferred_fact")
                        ),
                    })
                })
                .collect();
            
            total_inferences.extend(axiom_inferences);

            // Stop if no new inferences or max iterations reached
            if !iteration_fired || iteration >= self.max_iterations {
                break;
            }
        }

        Ok(SWRLExecutionResult {
            fired: total_applications > 0,
            inferences: total_inferences,
            applications: total_applications,
            execution_time_us: start_time.elapsed().as_micros() as u64,
        })
    }
    
    /// Find variable bindings that make the rule body match current facts
    fn find_rule_matches(
        &self,
        _rule: &SWRLRule, 
        _known_facts: &[SWRLAtom],
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
        _known_facts: &[SWRLAtom],
        _ontology: &Arc<Ontology>,
        _context: &SWRLExecutionContext
    ) -> Result<Vec<SWRLAtom>> {
        // Placeholder implementation - would need actual rule application logic
        Ok(Vec::new())
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

    /// Execute backward chaining strategy
    pub fn execute(
        &mut self,
        rules: &[SWRLRule],
        known_facts: &mut Vec<SWRLAtom>,
        ontology: &Arc<Ontology>,
        context: &mut SWRLExecutionContext
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();
        
        // For general execution, we don't have specific goals
        // This is a placeholder implementation
        Ok(SWRLExecutionResult {
            fired: false,
            inferences: Vec::new(),
            applications: 0,
            execution_time_us: start_time.elapsed().as_micros() as u64,
        })
    }

    /// Execute rules with a specific query goal
    pub fn execute_with_goal(
        &mut self,
        rules: &[SWRLRule],
        known_facts: &mut Vec<SWRLAtom>,
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
            if self.is_goal_satisfied(&current_goal, known_facts)? {
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
            fired: total_applications > 0,
            inferences: total_inferences,
            applications: total_applications,
            execution_time_us: start_time.elapsed().as_micros() as u64,
        })
    }

    /// Check if a goal is satisfied by current facts
    fn is_goal_satisfied(&self, _goal: &SWRLAtom, _known_facts: &[SWRLAtom]) -> Result<bool> {
        // Placeholder implementation
        Ok(false)
    }

    /// Check if a rule can prove a given goal
    fn can_rule_prove_goal(&self, _rule: &SWRLRule, _goal: &SWRLAtom) -> bool {
        // Placeholder implementation - would need actual unification
        false
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
        known_facts: &mut Vec<SWRLAtom>,
        ontology: &Arc<Ontology>,
        context: &mut SWRLExecutionContext
    ) -> Result<SWRLExecutionResult> {
        let start_time = Instant::now();
        
        // Phase 1: Forward chaining to establish base facts
        info!("Hybrid reasoning - Phase 1: Forward chaining");
        let forward_result = self.forward_engine.execute(rules, known_facts, ontology, context)?;

        // Phase 2: Backward chaining (if there are specific goals)
        info!("Hybrid reasoning - Phase 2: Backward chaining");
        let backward_result = self.backward_engine.execute(rules, known_facts, ontology, context)?;

        // Combine results
        let mut total_inferences = forward_result.inferences;
        total_inferences.extend(backward_result.inferences);

        Ok(SWRLExecutionResult {
            fired: forward_result.applications > 0 || backward_result.applications > 0,
            inferences: total_inferences,
            applications: forward_result.applications + backward_result.applications,
            execution_time_us: start_time.elapsed().as_micros() as u64,
        })
    }
}

impl Default for HybridReasoning {
    fn default() -> Self {
        Self::new()
    }
}
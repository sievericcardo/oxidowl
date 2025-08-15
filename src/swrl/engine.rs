//! SWRL Rule Engine
//!
//! This module implements the core SWRL rule execution engine that coordinates
//! rule firing, inference generation, and integration with the tableau reasoner.

use crate::{Error, Result, IRI};
use crate::ontology::{Axiom, Ontology};
use crate::swrl::{
    SWRLExecutionContext, SWRLExecutionResult, SWRLConfig, SWRLStatistics, SWRLRuleState, SWRLReasoningStrategy,
    SWRLRule, SWRLAtom,
    builtins::{SWRLBuiltInRegistry, SWRLBuiltIn},
    interpreter::SWRLInterpreter,
    validation::SWRLValidator,
};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::Instant;
use log::{debug, info, warn};

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

        Ok(SWRLExecutionResult::new(any_fired, total_inferences, total_applications))
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
        
        let result = self.interpreter.execute_rule(
            rule, 
            &mut context, 
            self.ontology.as_ref().unwrap()
        )?;

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
    pub fn execute_with_goal(&mut self, _goal: &SWRLAtom) -> Result<SWRLExecutionResult> {
        // TODO: Implement goal-driven backward chaining
        warn!("Goal-driven execution not yet implemented");
        Ok(SWRLExecutionResult::empty())
    }

    /// Check if any rules can potentially fire
    #[must_use]
    pub fn has_applicable_rules(&self) -> bool {
        self.rule_states.values().any(|state| 
            state.active && !state.should_skip(self.config.max_rule_applications)
        )
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
    use crate::swrl::{SWRLVariable, SWRLIArgument, SWRLAtom, SWRLRule, SWRLRuleAxiom};

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
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
                argument: SWRLIArgument::Variable(var_x1.clone()),
            }],
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
                argument: SWRLIArgument::Variable(var_x1),
            }]
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
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Student"))),
                argument: SWRLIArgument::Variable(var_x2),
            }]
        );
        ontology.add_axiom(Axiom::Rule(SWRLRuleAxiom::new(2, rule2)));
        
        // Rule 3: Person(?x) -> Teacher(?x)
        let var_x3 = SWRLVariable::new(IRI::new("http://example.org/var#x3"));
        let rule3 = SWRLRule::new(
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Teacher"))),
                argument: SWRLIArgument::Variable(var_x3.clone()),
            }],
            vec![SWRLAtom::ClassAtom {
                predicate: ClassExpression::Class(Class::new(IRI::new("http://example.org/Person"))),
                argument: SWRLIArgument::Variable(var_x3),
            }]
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

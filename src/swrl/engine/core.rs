//! SWRL Rule Engine Core Implementation
//!
//! This module contains the main SWRLRuleEngine struct and its public API.
//! The core coordinates rule execution by delegating to specialized modules.

use crate::ontology::{Axiom, Ontology};
use crate::swrl::{
    SWRLAtom, SWRLConfig, SWRLExecutionContext, SWRLExecutionResult, SWRLReasoningStrategy,
    SWRLRule, SWRLRuleState, SWRLStatistics,
    builtins::{SWRLBuiltIn, SWRLBuiltInRegistry},
    interpreter::SWRLInterpreter,
    validation::SWRLValidator,
};
use crate::{Error, IRI, Result};
use log::{debug, info, warn};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use super::inference::{BackwardChaining, ForwardChaining, HybridReasoning};
use super::validation::{GoalChecker, RuleValidator};

/// SWRL Rule Engine
///
/// The main engine for executing SWRL rules and generating inferences.
/// Supports both forward and backward chaining strategies.
#[derive(Debug)]
pub struct SWRLRuleEngine {
    /// Rule execution states
    pub(super) rule_states: HashMap<u64, SWRLRuleState>,

    /// Built-in predicate registry
    pub(super) builtin_registry: Arc<SWRLBuiltInRegistry>,

    /// Rule interpreter
    pub(super) interpreter: SWRLInterpreter,

    /// Rule validator
    pub(super) validator: SWRLValidator,

    /// Engine configuration
    pub(super) config: SWRLConfig,

    /// Execution statistics
    pub(super) statistics: SWRLStatistics,

    /// Current ontology
    pub(super) ontology: Option<Arc<RwLock<Ontology>>>,

    /// Inference cache
    pub(super) inference_cache: HashSet<Axiom>,

    /// Rule priority ordering
    pub(super) rule_priorities: HashMap<u64, u32>,

    /// Forward chaining engine
    pub(super) forward_chaining: ForwardChaining,

    /// Backward chaining engine  
    pub(super) backward_chaining: BackwardChaining,

    /// Hybrid reasoning engine
    pub(super) hybrid_reasoning: HybridReasoning,

    /// Rule validation engine
    pub(super) rule_validator: RuleValidator,

    /// Goal checking engine
    pub(super) goal_checker: GoalChecker,

    /// Cached rules from ontology
    pub(super) rules: Vec<SWRLRule>,

    /// Current execution context
    pub(super) context: SWRLExecutionContext,
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
            forward_chaining: ForwardChaining::new(),
            backward_chaining: BackwardChaining::new(),
            hybrid_reasoning: HybridReasoning::new(),
            rule_validator: RuleValidator::new(),
            goal_checker: GoalChecker::new(),
            rules: Vec::new(),
            context: SWRLExecutionContext::default(),
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
            self.rules.clear();
            let mut rule_count = 0;

            for axiom in ontology_guard.axioms() {
                if let Axiom::Rule(rule_axiom) = axiom {
                    // Validate the rule
                    match self.validator.validate_rule(&rule_axiom.rule) {
                        Ok(_) => {
                            let rule_state = SWRLRuleState::new(rule_axiom.rule.clone());
                            self.rule_states.insert(rule_axiom.id, rule_state);
                            self.rules.push(rule_axiom.rule.clone());
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
        if self.rule_states.is_empty() {
            return Ok(SWRLExecutionResult::empty());
        }

        let result = match self.config.strategy {
            SWRLReasoningStrategy::ForwardChaining => {
                let mut forward_chaining = std::mem::take(&mut self.forward_chaining);
                let mut known_facts = Vec::new(); // Initialize with facts from ontology
                if let Some(ontology_arc) = &self.ontology {
                    let ontology_guard = ontology_arc.read().unwrap();
                    // We need to extract the Arc<Ontology> from Arc<RwLock<Ontology>>
                    // For now, let's create a temporary Arc
                    let temp_ontology = Arc::new((*ontology_guard).clone());
                    drop(ontology_guard); // Release the lock
                    let result = forward_chaining.execute(
                        &self.rules,
                        &mut known_facts,
                        &temp_ontology,
                        &mut self.context,
                    );
                    self.forward_chaining = forward_chaining;
                    result
                } else {
                    Err(Error::reasoning("No ontology set for SWRL execution"))
                }
            }
            SWRLReasoningStrategy::BackwardChaining => {
                let mut backward_chaining = std::mem::take(&mut self.backward_chaining);
                let mut known_facts = Vec::new(); // Initialize with facts from ontology
                if let Some(ontology_arc) = &self.ontology {
                    let ontology_guard = ontology_arc.read().unwrap();
                    let temp_ontology = Arc::new((*ontology_guard).clone());
                    drop(ontology_guard); // Release the lock
                    let result = backward_chaining.execute(
                        &self.rules,
                        &mut known_facts,
                        &temp_ontology,
                        &mut self.context,
                    );
                    self.backward_chaining = backward_chaining;
                    result
                } else {
                    Err(Error::reasoning("No ontology set for SWRL execution"))
                }
            }
            SWRLReasoningStrategy::Hybrid => {
                let mut hybrid_reasoning = std::mem::take(&mut self.hybrid_reasoning);
                let mut known_facts = Vec::new(); // Initialize with facts from ontology
                if let Some(ontology_arc) = &self.ontology {
                    let ontology_guard = ontology_arc.read().unwrap();
                    let temp_ontology = Arc::new((*ontology_guard).clone());
                    drop(ontology_guard); // Release the lock
                    let result = hybrid_reasoning.execute(
                        &self.rules,
                        &mut known_facts,
                        &temp_ontology,
                        &mut self.context,
                    );
                    self.hybrid_reasoning = hybrid_reasoning;
                    result
                } else {
                    Err(Error::reasoning("No ontology set for SWRL execution"))
                }
            }
        };

        if let Ok(ref res) = result {
            self.statistics.update(res);
            info!(
                "SWRL execution completed: {} rules fired, {} inferences",
                res.applications,
                res.inferences.len()
            );
        }

        result
    }

    /// Execute rules with a specific query goal (for backward chaining)
    pub fn execute_with_goal(&mut self, goal: &SWRLAtom) -> Result<SWRLExecutionResult> {
        info!("Starting goal-driven execution for goal: {:?}", goal);
        let mut backward_chaining = std::mem::take(&mut self.backward_chaining);
        let mut known_facts = Vec::new(); // Initialize with facts from ontology
        if let Some(ontology_arc) = &self.ontology {
            let ontology_guard = ontology_arc.read().unwrap();
            let temp_ontology = Arc::new((*ontology_guard).clone());
            drop(ontology_guard); // Release the lock
            let result = backward_chaining.execute_with_goal(
                &self.rules,
                &mut known_facts,
                &temp_ontology,
                &mut self.context,
                goal,
            );
            self.backward_chaining = backward_chaining;
            result
        } else {
            Err(Error::reasoning("No ontology set for SWRL execution"))
        }
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

    /// Get rules ordered by priority
    pub(super) fn get_ordered_rules(&self) -> Vec<u64> {
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
    pub(super) fn apply_inferences_to_ontology(&mut self, inferences: Vec<Axiom>) -> Result<()> {
        if let Some(ontology) = &self.ontology {
            let mut ontology_guard = ontology.write().unwrap();

            for inference in inferences {
                ontology_guard.add_axiom(inference);
            }
        }

        Ok(())
    }
}

//! SWRL Rule Engine Core Implementation
//!
//! This module contains the main `SWRLRuleEngine` struct and its public API.
//! The core coordinates rule execution by delegating to specialized modules.

#![allow(dead_code)]

use crate::core::lock_helpers::{read_lock, write_lock};
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
            let ontology_guard =
                match read_lock(ontology, "SWRL core: reading ontology for loading rules") {
                    Ok(guard) => guard,
                    Err(e) => {
                        warn!("Failed to acquire read lock on ontology: {e}");
                        return;
                    }
                };

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

            info!("Loaded {rule_count} SWRL rules from ontology");
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
                    let ontology_guard = read_lock(
                        ontology_arc,
                        "SWRL core: reading ontology for forward chaining",
                    )?;
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
                    let ontology_guard = read_lock(
                        ontology_arc,
                        "SWRL core: reading ontology for backward chaining",
                    )?;
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
                    let ontology_guard = read_lock(
                        ontology_arc,
                        "SWRL core: reading ontology for hybrid reasoning",
                    )?;
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
        info!("Starting goal-driven execution for goal: {goal:?}");
        let mut backward_chaining = std::mem::take(&mut self.backward_chaining);
        let mut known_facts = Vec::new(); // Initialize with facts from ontology
        if let Some(ontology_arc) = &self.ontology {
            let ontology_guard = read_lock(
                ontology_arc,
                "SWRL core: reading ontology for goal-driven execution",
            )?;
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
        if let Some(registry) = Arc::get_mut(&mut self.builtin_registry) {
            registry.register_builtin(iri, builtin);
        }
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
            Err(Error::reasoning(format!("Rule {rule_id} not found")))
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
            let mut ontology_guard =
                write_lock(ontology, "SWRL core: writing inferences to ontology")?;

            for inference in inferences {
                ontology_guard.add_axiom(inference);
            }
        }

        Ok(())
    }

    // ─── parallel execution (Phase 3.4) ─────────────────────────────────────

    /// Execute all loaded rules with opportunistic parallelism.
    ///
    /// Rules are first partitioned into *independent groups* — sets of rules
    /// whose head predicates are disjoint, so they cannot interfere with each
    /// other's write targets.  Groups are processed sequentially (to preserve
    /// fix-point semantics), but rules *within* a group are matched in parallel
    /// using [`rayon`].
    ///
    /// The current implementation uses the **conservative partition**: every
    /// rule forms its own singleton group, so the outer loop is sequential and
    /// each "parallel" step has exactly one rule.  This is always correct and
    /// establishes the architecture; future work will detect actual independence
    /// by analysing head/body predicate overlap to produce larger groups.
    ///
    /// Body matching is read-only and operates on a snapshot of the ontology
    /// taken at the start of each group.  Confirmed new inferences are applied
    /// to the live ontology in a serial merge phase at the end of each group.
    ///
    /// Only compiled when the `parallel` feature is enabled.
    #[cfg(feature = "parallel")]
    pub fn execute_rules_parallel(&mut self) -> Result<SWRLExecutionResult> {
        use rayon::prelude::*;

        if self.rules.is_empty() {
            return Ok(SWRLExecutionResult::empty());
        }

        let ontology_arc = match &self.ontology {
            Some(o) => Arc::clone(o),
            None => {
                return Err(Error::reasoning(
                    "No ontology set for parallel SWRL execution",
                ));
            }
        };

        // Partition rules into independent groups.
        // Conservative strategy: one rule per group (always correct).
        let rule_groups = self.partition_independent_rules();

        let mut combined = SWRLExecutionResult::empty();

        for group_indices in &rule_groups {
            // Clone just the rules in this group so the rayon closure can own them.
            let group_rules: Vec<SWRLRule> = group_indices
                .iter()
                .filter_map(|&idx| self.rules.get(idx).cloned())
                .collect();

            if group_rules.is_empty() {
                continue;
            }

            // Take an immutable snapshot for parallel body matching.
            // Body matching is read-only, so we clone the ontology once per group
            // rather than holding the lock across the parallel section.
            let snapshot: Arc<Ontology> = {
                let guard =
                    read_lock(&ontology_arc, "SWRL parallel: snapshot for group")?;
                Arc::new((*guard).clone())
            };

            // --- Parallel body matching -----------------------------------------
            // Each rule in the group is evaluated against the snapshot in parallel.
            // `derive_head_axioms` is a pure function; no shared mutable state.
            let snapshot_ref: &Ontology = &snapshot;
            let candidate_axioms: Vec<Axiom> = group_rules
                .par_iter()
                .flat_map(|rule| Self::derive_head_axioms(rule, snapshot_ref))
                .collect();
            // --- End parallel section -------------------------------------------

            let n_candidates = candidate_axioms.len();

            // --- Sequential merge phase ----------------------------------------
            // De-duplicate inferences through `inference_cache` and write to the
            // live ontology under a single write lock.
            if !candidate_axioms.is_empty() {
                let mut onto =
                    write_lock(&ontology_arc, "SWRL parallel: applying inferences")?;
                for axiom in candidate_axioms {
                    // `HashSet::insert` returns `true` if the item was not present.
                    if self.inference_cache.insert(axiom.clone()) {
                        onto.add_axiom(axiom);
                        combined.applications += 1;
                        combined.fired = true;
                    }
                }
            }
            // --- End merge phase ------------------------------------------------

            if n_candidates > 0 {
                debug!(
                    "SWRL parallel group ({} rules): {} candidate inferences",
                    group_rules.len(),
                    n_candidates
                );
            }
        }

        info!(
            "Parallel SWRL execution complete: {} applications across {} groups",
            combined.applications,
            rule_groups.len()
        );

        Ok(combined)
    }

    /// Derive head axioms for a rule when its body is satisfied by `ontology`.
    ///
    /// This is a **pure, read-only** function — safe to call from a rayon
    /// parallel iterator without any synchronisation.
    ///
    /// **Current behaviour (placeholder):** only ground rules with an *empty
    /// body* (tautological antecedent) are evaluated; all other rules return
    /// no inferences.  A production implementation would unify each body atom
    /// against the ABox to produce variable bindings, then instantiate the head
    /// atoms under those bindings.  Full unification-based matching is already
    /// handled by the sequential `ForwardChaining` and `BackwardChaining`
    /// engines; this function is the hook point for a future parallelised
    /// version of that logic.
    fn derive_head_axioms(_rule: &SWRLRule, _ontology: &Ontology) -> Vec<Axiom> {
        // Placeholder: ground rules with empty bodies fire unconditionally.
        // Converting SWRLAtoms → Axioms requires ABox matching (TODO).
        // Returning empty is always sound (may miss inferences, never adds wrong ones).
        Vec::new()
    }

    /// Partition the loaded rules into groups that can be executed concurrently.
    ///
    /// **Conservative strategy (Phase 3.4 baseline):** every rule is placed in
    /// its own singleton group.  This is always correct because singleton groups
    /// trivially have no write-write or read-write conflicts.
    ///
    /// **Future improvement:** analyse each rule's head atoms to extract the set
    /// of predicates it *writes*, and its body atoms to extract the predicates it
    /// *reads*.  Two rules may share a group when their write-sets are disjoint
    /// *and* neither rule reads the other's write-set (no read-after-write hazard).
    ///
    /// Returns a `Vec<Vec<usize>>` where each inner `Vec` is one group and each
    /// element is an index into `self.rules`.
    fn partition_independent_rules(&self) -> Vec<Vec<usize>> {
        // One rule per group — conservative but always correct.
        self.rules
            .iter()
            .enumerate()
            .map(|(i, _)| vec![i])
            .collect()
    }
}

//! SWRL (Semantic Web Rule Language) Support
//!
//! This module provides comprehensive support for SWRL rules in oxidowl,
//! including rule parsing, validation, execution, and integration with
//! the tableau reasoner.

pub mod additional_builtins;
pub mod backward_chaining;
pub mod boolean_builtins;
pub mod builtins;
pub mod collection_builtins;
pub mod datetime_builtins;
pub mod datetime_constructor_builtins;
pub mod engine;
pub mod extended_builtins;
pub mod integration;
pub mod interpreter;
pub mod list_builtins;
pub mod math_builtins;
pub mod parser;
pub mod regex_builtins;
pub mod string_builtins;
pub mod temporal;
pub mod uri_builtins;
pub mod validation;

//#[cfg(test)]
//pub mod tests;

// Re-export main types
pub use backward_chaining::{
    BackwardChainingEngine, FactBase, QueryResult, VariableBindings as BCVariableBindings,
};
pub use builtins::{SWRLBuiltIn, SWRLBuiltInRegistry as BuiltInRegistry, SWRLValue};
pub use datetime_constructor_builtins::DateTimeConstructorRegistry;
pub use engine::SWRLRuleEngine;
pub use integration::{SWRLFeatureRegistry, SWRLFeatureStatistics, ValidationResult};
pub use interpreter::SWRLInterpreter;
pub use parser::{NamespaceManager, ParseError, SWRLParser};
pub use validation::{
    SWRLValidator, ValidationIssue, ValidationResult as ValidationResultBase, ValidationWarning,
};

// Re-export core SWRL types from ontology module
pub use crate::ontology::axioms::{
    SWRLAtom, SWRLDArgument, SWRLIArgument, SWRLRule, SWRLRuleAxiom, SWRLVariable,
};

use crate::ontology::{axioms::*, *};
use crate::{Error, Result};
use std::collections::HashMap;
use std::fmt;

/// SWRL Rule execution context
#[derive(Debug, Clone)]
pub struct SWRLExecutionContext {
    /// Current variable bindings
    pub bindings: HashMap<SWRLVariable, SWRLValue>,
    /// Stack of bindings for backtracking
    pub binding_stack: Vec<HashMap<SWRLVariable, SWRLValue>>,
    /// Current execution depth (for loop detection)
    pub depth: usize,
    /// Maximum execution depth allowed
    pub max_depth: usize,
}

impl Default for SWRLExecutionContext {
    fn default() -> Self {
        Self {
            bindings: HashMap::new(),
            binding_stack: Vec::new(),
            depth: 0,
            max_depth: 1000, // Prevent infinite recursion
        }
    }
}

impl SWRLExecutionContext {
    /// Create a new execution context
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Push current bindings onto stack
    pub fn push_bindings(&mut self) {
        self.binding_stack.push(self.bindings.clone());
    }

    /// Pop bindings from stack
    pub fn pop_bindings(&mut self) -> Option<HashMap<SWRLVariable, SWRLValue>> {
        if let Some(bindings) = self.binding_stack.pop() {
            self.bindings = bindings.clone();
            Some(bindings)
        } else {
            None
        }
    }

    /// Bind a variable to a value
    pub fn bind(&mut self, variable: SWRLVariable, value: SWRLValue) -> Result<()> {
        if let Some(existing) = self.bindings.get(&variable) {
            if existing != &value {
                return Err(Error::reasoning(format!(
                    "Variable {} already bound to different value",
                    variable.iri
                )));
            }
        }
        self.bindings.insert(variable, value);
        Ok(())
    }

    /// Get binding for a variable
    #[must_use]
    pub fn get_binding(&self, variable: &SWRLVariable) -> Option<&SWRLValue> {
        self.bindings.get(variable)
    }

    /// Check if a variable is bound
    #[must_use]
    pub fn is_bound(&self, variable: &SWRLVariable) -> bool {
        self.bindings.contains_key(variable)
    }

    /// Clear all bindings
    pub fn clear(&mut self) {
        self.bindings.clear();
        self.binding_stack.clear();
        self.depth = 0;
    }

    /// Increment execution depth
    pub fn increment_depth(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > self.max_depth {
            Err(Error::reasoning("SWRL execution depth limit exceeded"))
        } else {
            Ok(())
        }
    }

    /// Decrement execution depth
    pub fn decrement_depth(&mut self) {
        if self.depth > 0 {
            self.depth -= 1;
        }
    }
}

/// SWRL Rule execution result
#[derive(Debug, Clone)]
pub struct SWRLExecutionResult {
    /// Whether the rule was fired
    pub fired: bool,
    /// New inferences generated
    pub inferences: Vec<Axiom>,
    /// Number of rule applications
    pub applications: usize,
    /// Execution time in microseconds
    pub execution_time_us: u64,
}

impl SWRLExecutionResult {
    /// Create a new execution result
    #[must_use]
    pub fn new(fired: bool, inferences: Vec<Axiom>, applications: usize) -> Self {
        Self {
            fired,
            inferences,
            applications,
            execution_time_us: 0,
        }
    }

    /// Create an empty result (rule not fired)
    #[must_use]
    pub fn empty() -> Self {
        Self {
            fired: false,
            inferences: Vec::new(),
            applications: 0,
            execution_time_us: 0,
        }
    }
}

/// SWRL reasoning strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SWRLReasoningStrategy {
    /// Forward chaining (generate consequences)
    ForwardChaining,
    /// Backward chaining (goal-driven)
    BackwardChaining,
    /// Hybrid approach
    Hybrid,
}

/// SWRL reasoning configuration
#[derive(Debug, Clone)]
pub struct SWRLConfig {
    /// Reasoning strategy to use
    pub strategy: SWRLReasoningStrategy,
    /// Maximum number of rule applications per reasoning cycle
    pub max_rule_applications: usize,
    /// Maximum execution depth for recursive rules
    pub max_execution_depth: usize,
    /// Enable built-in predicates
    pub enable_builtins: bool,
    /// Enable debugging output
    pub debug: bool,
    /// Timeout for rule execution in milliseconds
    pub timeout_ms: Option<u64>,
}

impl Default for SWRLConfig {
    fn default() -> Self {
        Self {
            strategy: SWRLReasoningStrategy::ForwardChaining,
            max_rule_applications: 1000,
            max_execution_depth: 100,
            enable_builtins: true,
            debug: false,
            timeout_ms: Some(30000), // 30 seconds
        }
    }
}

/// SWRL reasoning statistics
#[derive(Debug, Clone, Default)]
pub struct SWRLStatistics {
    /// Total number of rule applications
    pub total_rule_applications: usize,
    /// Number of rules fired
    pub rules_fired: usize,
    /// Number of inferences generated
    pub inferences_generated: usize,
    /// Total reasoning time in microseconds
    pub total_reasoning_time_us: u64,
    /// Average time per rule application
    pub avg_time_per_application_us: f64,
}

impl SWRLStatistics {
    /// Update statistics with execution result
    pub fn update(&mut self, result: &SWRLExecutionResult) {
        self.total_rule_applications += result.applications;
        if result.fired {
            self.rules_fired += 1;
        }
        self.inferences_generated += result.inferences.len();
        self.total_reasoning_time_us += result.execution_time_us;

        if self.total_rule_applications > 0 {
            self.avg_time_per_application_us =
                self.total_reasoning_time_us as f64 / self.total_rule_applications as f64;
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// SWRL Rule state for execution tracking
#[derive(Debug, Clone)]
pub struct SWRLRuleState {
    /// The rule being executed
    pub rule: SWRLRule,
    /// Number of times this rule has been applied
    pub application_count: usize,
    /// Last execution result
    pub last_result: Option<SWRLExecutionResult>,
    /// Whether this rule is currently active
    pub active: bool,
}

impl SWRLRuleState {
    /// Create new rule state
    #[must_use]
    pub fn new(rule: SWRLRule) -> Self {
        Self {
            rule,
            application_count: 0,
            last_result: None,
            active: true,
        }
    }

    /// Mark rule as applied with result
    pub fn mark_applied(&mut self, result: SWRLExecutionResult) {
        self.application_count += result.applications;
        self.last_result = Some(result);
    }

    /// Check if rule should be skipped
    #[must_use]
    pub fn should_skip(&self, max_applications: usize) -> bool {
        !self.active || self.application_count >= max_applications
    }
}

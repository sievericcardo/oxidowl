//! Completion rule system for tableau expansion
//!
//! This module implements the core completion rules for SROIQV(D) tableau
//! reasoning, based on the rule systems from Konclude, HermiT, and Pellet.

use crate::{
    core::dependency::{DependencySet, DependencyTracker, DependencyType},
    ontology::{ClassExpression, Individual, Role, DataProperty, ObjectPropertyExpression},
    Error, Result
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    fmt,
};

/// Completion rule types for tableau expansion
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompletionRule {
    /// Conjunction rule: A ⊓ B → A, B
    And,
    
    /// Disjunction rule: A ⊔ B → A | B (creates branching)
    Or,
    
    /// Existential rule: ∃R.C → create new individual with R-edge and C
    Some,
    
    /// Universal rule: ∀R.C with R-edge to y → C on y
    All,
    
    /// At-least cardinality: ≥n R.C → create at least n R-successors with C
    AtLeast,
    
    /// At-most cardinality: ≤n R.C → merge or block excess successors
    AtMost,
    
    /// Nominal rule: {a} → merge with individual a
    Nominal,
    
    /// Self rule: ∀R.Self → R(x,x)
    Self_,
    
    /// Choose rule: handle non-deterministic cardinality choices
    Choose,
    
    /// Datatype rule: handle datatype restrictions
    Datatype,
    
    /// Unfolding rule: unfold concept definitions
    Unfold,
    
    /// Property chain rule: R1 ∘ R2 ∘ ... ∘ Rn ⊑ S → propagate S edges
    PropertyChain,
    
    /// Guess rule: generate individuals for functionality/cardinality
    Guess,
}

/// Priority levels for rule application ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RulePriority {
    /// Critical rules (deterministic, no choice)
    Highest = 0,

    /// High priority (propagation, essential for completeness)
    High = 1,
    
    /// Normal priority (existential, universal, etc.)
    Normal = 2,
    
    /// Low priority (cardinality, non-critical)
    Low = 3,

    /// Lowest priority (cleanup, optimisation)
    Lowest = 4,
}

/// Completion rule application context
#[derive(Debug, Clone)]
pub struct RuleApplication {
    /// Rule to apply
    pub rule: CompletionRule,

    /// Target individual or concept
    pub node: String,

    /// Rule-specific context
    pub context: RuleContext,

    /// Priority for application
    pub priority: RulePriority,

    /// Dependencies for this rule application
    pub dependencies: DependencySet,
}

/// Context specific to each rule type
#[derive(Debug, Clone)]
pub enum RuleContext {
    /// Context for concept-based rules (AND, OR, etc.)
    Concept {
        concept: ClassExpression,
        dependencies: DependencySet,
    },

    /// Context for role-based rules (SOME, ALL, etc.)
    Role {
        role: Role,
        source: String
        target: String,
        concept: ClassExpression,
    },

    /// Context for cardinality rules
    Cardinality {
        cardinality: u32,
        role: Role,
        filler: Option<ClassExpression>,
        existing_successors: Vec<String>,
    },

    /// Context for nominal rules
    Nominal {
        nominal: Individual,
        current_node: String,
    },

    /// Context for datatype rules
    Datatype {
        property: DataProperty,
        restriction: String,
        value: Option<String>,
    },

    /// Context for merge rules
    Merge {
        source: String,
        target: String,
        reason: String,
    },

    /// Context for at-most cardinality rules
    AtMost {
        node_id: String,
        cardinality: u32,
        property: Role,
        filler: ClassExpression,
    },

    /// Context for property chain rules
    PropertyChain {
        chain: Vec<ObjectPropertyExpression>,
        target: String,
        source: String,
        super_property: ObjectPropertyExpression,
    },
}    
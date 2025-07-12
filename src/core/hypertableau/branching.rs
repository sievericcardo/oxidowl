//! Branching and Backtracking for HyperTableau
//!
//! This module implements branching points and backtracking mechanisms for
//! handling non-deterministic disjunctions in the hypertableau algorithm.

use crate::{
    ontology::{Individual, ClassExpression, Axiom},
    Error, Result,
};

use super::{
    dependency_tracking::{DependencyTracker, BranchingPointId, FactId},
    ground_disjunction::GroundDisjunction,
};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    fmt,
};

/// Different types of branching decisions
#[derive(Debug, Clone, PartialEq)]
pub enum BranchingType {
    /// Branching on a ground disjunction
    GroundDisjunction {
        disjunction: GroundDisjunction,
        individual: Individual,
    },
    /// Branching on an existential restriction
    ExistentialRestriction {
        property: String,
        filler: ClassExpression,
        individual: Individual,
    },
    /// Branching on a universal restriction clash
    UniversalRestriction {
        property: String,
        filler: ClassExpression,
        individual: Individual,
    },
    /// Branching on number restrictions
    NumberRestriction {
        property: String,
        cardinality: usize,
        filler: Option<ClassExpression>,
        individual: Individual,
    },
}

/// Represents a choice within a branching point
#[derive(Debug, Clone)]
pub struct BranchingChoice {
    /// Index of this choice within the branching point
    pub choice_index: usize,
    
    /// Description of what this choice represents
    pub description: String,
    
    /// The assertion that would be made if this choice is selected
    pub assertion: ClassExpression,
    
    /// The individual this assertion applies to
    pub individual: Individual,
    
    /// Whether this choice has been explored
    pub explored: bool,
    
    /// Whether this choice led to a clash
    pub caused_clash: bool,
    
    /// Cost estimate for this choice (for heuristics)
    pub cost_estimate: f64,
}
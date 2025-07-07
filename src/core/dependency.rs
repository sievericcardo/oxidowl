//! Dependency tracking and management
//!
//! This module provides dependency tracking capabilities for backtracking
//! and maintaining the reasoning dependency graph.

use crate::{Error, Result};
use std::{
    collections::{HashMap, HashSet, BTreeSet},
    ftm,
    sync::{Arc, Weak},
}

/// Identifier for the dependency nodes
pub type DependencyId = u64;

/// Identifier for branching points in the dependency graph
pub type BranchingPoint = u64;

/// Dependency set tracking concept derivations and branching dependencies
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepepdencySet {
    /// Branching points the dependency set is associated with
    branching_points: BTreeSet<BranchingPoint>,

    /// Deterministic dependencies
    deterministic_deps: HashSet<DependencyId>,

    /// Non-deterministic dependencies
    nondeterministic_deps: HashSet<DependencyId>,

    /// Reference count for the dependency set
    ref_count: usize,
}

/// Dependency node representing a reasoning step or choice point
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Unique identifier for the dependency node
    id: DependencyId,

    /// Type of dependency node (e.g., concept, role, data property)
    node_type: DependencyType,

    /// Dependencies that led to this node
    dependencies: DependencySet,

    /// Nodes that depend on this node
    dependents: HashSet<DependencyId>,

    /// Branching point this node is associated with
    branching_point: Option<BranchingPoint>,

    /// Status of the dependency node (active, inactive, etc.)
    status: DependencyStatus,
}

/// Type of dependency node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Deterministic dependency (e.g. AND, SOME)
    Deterministic {
        rule: String,
        source_concept: String,
    },

    /// Non-deterministic dependency (e.g. OR, cardinality choice)
    NonDeterministic {
        rule: String,
        choices: Vec<String>,
        chosen_index: Option<usize>,
    },

    /// Merging operation (e.g. merging two branches)
    Merge {
        source_node: String,
        target_node: String,
    },

    /// Concept implication
    Implication {
        antecedent: String,
        consequent: String,
    },

    /// Functionality restriction
    Functional {
        role: String,
        individual: String,
    },

    /// Distinctness constraint
    Distinct {
        individuals: Vec<String>,
    },

    /// Nominal handling
    Nominal {
        nominal: String,
        individual: String,
    },

    /// Expanded existential
    Expanded {
        existential: String,
        witness: String,
    },
    
    /// Datatype constraint
    Datatype {
        constraint: String,
        value: String,
    },
}
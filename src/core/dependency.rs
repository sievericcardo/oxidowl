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
pub struct DependencySet {
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

/// Status of the dependency node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyStatus {
    /// Active dependency node
    Active,

    /// Inactive dependency node (e.g. backtracked)
    Backtracked,

    /// Suspended dependency node (part of a merge operation)
    Suspended,

    /// Removed dependency node (e.g. due to inconsistency)
    Removed,
}

/// Dependency tracker managing the dependency graph and operations
#[derive(Debug)]
pub struct DependencyTracker {
    /// Nodes
    nodes: HashMap<DependencyId, DependencyNode>,

    /// Next available dependency ID
    next_id: DependencyId,

    /// Current branching point
    current_branching_level: BranchingPoint,

    /// Stack of branching points
    branching_stack: Vec<BranchingPoint>,

    /// Dependency sets for efficient memory management
    set_factory: DependencySetFactory,

    /// Active dependencies at each branching point
    active_dependencies: HashMap<BranchingPoint, HashSet<DependencyId>>,
}

/// Factory for creating and managing dependency sets\
#[derive(Debug)]
pub struct DependencySetFactory {
    /// Cache of dependency sets
    set_cache: HashMap<DependencyId, Arc<DependencySet>>,

    /// Empty dependency set singleton
    empty_set: Arc<DependencySet>,

    /// Usage counters for garbage collection
    usage_counters: HashMap<DependencySetKey, usize>,
}

/// Key for identifying dependency sets in the factory
#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencySetKey {
    branching_points: BTreeSet<BranchingPoint>,
    deterministic_deps: HashSet<DependencyId>,
    nondeterministic_deps: HashSet<DependencyId>,
}

impl std::hash::Hash for DependencySetKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.branching_points.hash(state);
        self.deterministic_deps.hash(state);
        self.nondeterministic_deps.hash(state);
    }
}

/// Track point for dependency management
#[derive(Debug, Clone)]
pub struct DependencyTrackPoint {
    /// Branching point identifier
    branching_point: BranchingPoint,

    /// Dependencies at this point
    active_dependencies: HashSet<DependencyId>,

    /// Timestamp for ordering
    timestamp: std::time::Instant,
}

impl DependencySet {
    /// Create an empty dependency set
    pub fn new() -> Self {
        Self {
            branching_points: BTreeSet::new(),
            deterministic_deps: HashSet::new(),
            nondeterministic_deps: HashSet::new(),
            ref_count: 0,
        }
    }

    /// Create a dependency set with a single branching point
    pub fn with_branching_point(branching_point: BranchingPoint) -> Self {
        let mut set = Self::new();
        set.branching_points.insert(branching_point);
        set
    }

    /// Create a dependency set with a single dependency
    pub fn with_dependency(dep_id: DependencyId, is_deterministic: bool) -> Self {
        let mut set = Self::new();
        if is_deterministic {
            set.deterministic_deps.insert(dep_id);
        } else {
            set.nondeterministic_deps.insert(dep_id);
        }
        set
    }

    /// Union of two dependency sets
    pub fn union(&self, other: &DependencySet) -> Self {
        Self {
            branching_points: self.branching_points.union(&other.branching_points).cloned().collect(),
            deterministic_deps: self.deterministic_deps.union(&other.deterministic_deps).cloned().collect(),
            nondeterministic_deps: self.nondeterministic_deps.union(&other.nondeterministic_deps).cloned().collect(),
            ref_count: 0, // Ref count is managed externally
        }
    }

    /// Add a branching point to the dependency set
    pub fn add_branching_point(&mut self, branching_point: BranchingPoint) {
        self.branching_points.insert(branching_point);
    }

    /// Add a dependency to the set
    pub fn add_dependency(&mut self, dep_id: DependencyId, is_deterministic: bool) {
        if is_deterministic {
            self.deterministic_deps.insert(dep_id);
        } else {
            self.nondeterministic_deps.insert(dep_id);
        }
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.branching_points.is_empty()
            && self.deterministic_deps.is_empty()
            && self.nondeterministic_deps.is_empty()
    }

    /// Get all branching points
    pub fn branching_points(&self) -> &BTreeSet<BranchingPoint> {
        &self.branching_points
    }

    /// Get maximum branching point
    pub fn max_branching_point(&self) -> Option<BranchingPoint> {
        self.branching_points.iter().max().copied()
    }

    /// Check if the set is valid at a given branching point
    pub fn is_valid_at(&self, branching_point: BranchingPoint) -> bool {
        self.branching_points.iter().all(|&bp| bp <= branching_point)
    }

    /// Check if the set conflicts with another set at a given branching point
    pub fn conflicts_with(&self, other: &DependencySet, branching_point: BranchingPoint) -> bool {
        self.branching_points.contains(&branch_point) && other.branching_points.contains(&branch_point)
    }
}
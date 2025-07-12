//! Dependency Tracking for HyperTableau
//!
//! This module implements dependency tracking for supporting backtracking and
//! justification in the hypertableau algorithm. It tracks causal relationships
//! between derived facts and their supporting evidence.

use crate::{
    ontology::{Individual, ClassExpression, Axiom},
    Error, Result,
};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    hash::{Hash, Hasher},
    fmt,
};

/// Unique identifier for dependency sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DependencySetId(pub usize);

/// Unique identifier for branching points  
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BranchingPointId(pub usize);

/// Unique identifier for facts/assertions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactId(pub usize);

/// Types of dependencies that can exist
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    /// Dependency from a clause application
    ClauseApplication {
        clause_id: usize,
        premise_facts: Vec<FactId>,
    },
    /// Dependency from a branching decision
    BranchingDecision {
        branch_id: BranchingPointId,
        choice_index: usize,
    },
    /// Dependency from an initial assertion
    InitialAssertion {
        axiom_id: usize,
    },
    /// Dependency from a blocking operation
    Blocking {
        blocker_node: usize,
        blocked_node: usize,
    },
    /// Dependency from unfolding a concept
    ConceptUnfolding {
        concept: ClassExpression,
        individual: Individual,
    },
}

/// A dependency set tracks the causal history of a derived fact
#[derive(Debug, Clone)]
pub struct DependencySet {
    /// Unique identifier for this dependency set
    pub id: DependencySetId,
    
    /// The type of dependency
    pub dependency_type: DependencyType,
    
    /// Parent dependency sets that this one depends on
    pub parents: Vec<DependencySetId>,
    
    /// Level in the dependency hierarchy (for optimization)
    pub level: usize,
    
    /// Whether this dependency set is currently active
    pub is_active: bool,
    
    /// Timestamp when this dependency was created
    pub timestamp: std::time::Instant,
}
//! Ground Disjunction Implementation
//!
//! This module implements ground disjunctions as used in HermiT's hypertableau algorithm.
//! Ground disjunctions represent disjunctive facts that must hold in the tableau.

use crate::{
    core::{
        dependency::DependencySet,
        hypertableau::extension_tables::ExtensionManager,
    },
    ontology::{ClassExpression, ObjectProperty, Individual},
    Error, Result,
};

use std::{
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
};

/// A ground disjunction represents a disjunctive clause where all variables are bound
#[derive(Debug, Clone)]
pub struct GroundDisjunction {
    /// Header containing the disjunctive structure
    header: GroundDisjunctionHeader,
    
    /// Arguments (nodes/individuals) for this ground disjunction
    arguments: Vec<usize>, // Node IDs
    
    /// Core flags for each argument (used for blocking)
    is_core: Vec<bool>,
    
    /// Dependency set for backtracking
    dependency_set: DependencySet,
    
    /// Unique identifier
    id: usize,
}

/// Header structure for ground disjunctions containing the disjunctive predicates
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroundDisjunctionHeader {
    /// The predicates in this disjunction
    predicates: Vec<DisjunctPredicate>,
    
    /// Sorted indices for disjunct processing order
    sorted_disjunct_indices: Vec<usize>,
    
    /// Priority for processing
    priority: DisjunctionPriority,
}

/// A single predicate in a disjunction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DisjunctPredicate {
    /// Concept assertion: C(x)
    Concept {
        concept: ClassExpression,
        argument: usize, // position in arguments array
    },
    
    /// Role assertion: R(x, y)
    Role {
        property: ObjectProperty,
        subject: usize,  // position in arguments array
        object: usize,   // position in arguments array
    },
    
    /// Equality: x = y
    Equality {
        left: usize,
        right: usize,
    },
    
    /// Inequality: x ≠ y
    Inequality {
        left: usize,
        right: usize,
    },
}

/// Priority levels for ground disjunction processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisjunctionPriority {
    /// Highest priority - process immediately
    Critical = 0,
    /// High priority - process soon
    High = 1,
    /// Normal priority - standard processing
    Normal = 2,
    /// Low priority - process when convenient
    Low = 3,
}
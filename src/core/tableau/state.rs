//! Tableau state management
//!
//! This module handles the overall state of tableau expansion,
//! including statistics, clash detection, and completion tracking.

use super::node::{NodeId, RoleLabel};
use crate::core::dependency::DependencySet;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

/// Current state of tableau expansion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableauState {
    /// Tableau is satisfiable (open)
    Satisfiable,
    /// Tableau is unsatisfiable (closed)
    Unsatisfiable,
    /// State is unknown (timeout, resource limit, etc.)
    Unknown,
}

/// Priority levels for rule applications
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Highest priority (deterministic rules)
    Highest = 0,

    /// High priority (propagation rules)
    High = 1,

    /// Normal priority (expansion rules)
    Normal = 2,

    /// Low priority (non-deterministic rules)
    Low = 3,

    /// Lowest priority (optimization rules)
    Lowest = 4,
}

/// Clash detection and management
#[derive(Debug)]
pub struct ClashDetector {
    /// Currently detected clashes
    clashes: Vec<Clash>,
}

/// Representation of a clash in the tableau
#[derive(Debug, Clone)]
pub struct Clash {
    /// Type of clash
    pub clash_type: ClashType,

    /// Nodes involved in the clash
    pub nodes: Vec<NodeId>,

    /// Dependencies that led to this clash
    pub dependencies: Arc<DependencySet>,

    /// Explanation for the clash
    pub explanation: String,
}

/// Types of clashes that can occur
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClashType {
    /// Concept clash (C and ¬C)
    Concept { concept: String, node: NodeId },

    /// Cardinality clash
    Cardinality {
        role: RoleLabel,
        node: NodeId,
        min_cardinality: u32,
        max_cardinality: u32,
    },

    /// Functionality clash
    Functionality {
        role: RoleLabel,
        node: NodeId,
        individuals: Vec<NodeId>,
    },

    /// Nominal clash
    Nominal {
        individual: String,
        nodes: Vec<NodeId>,
    },

    /// DL Clause violation detected during tableau expansion
    /// This indicates a clause from the ontology was violated by the tableau state
    ClauseViolation {
        /// The ID of the violated clause
        clause_id: String,
        /// The node where the violation was detected
        node: NodeId,
        /// Description of the violation
        description: String,
    },
}

/// Statistics about tableau construction and expansion
#[derive(Debug, Clone, Default)]
pub struct TableauStatistics {
    /// Total number of nodes created
    pub total_nodes: usize,

    /// Total number of edges created
    pub total_edges: usize,

    /// Number of rule applications
    pub rule_applications: usize,

    /// Number of clashes detected
    pub clashes_detected: usize,

    /// Number of backtracks performed
    pub backtracks: usize,

    /// Start time of tableau construction
    pub start_time: Option<Instant>,

    /// Total construction time
    pub construction_time: Duration,

    /// Peak memory usage (if available)
    pub peak_memory: Option<usize>,
}

impl Default for ClashDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl ClashDetector {
    /// Create a new clash detector
    #[must_use]
    pub fn new() -> Self {
        Self {
            clashes: Vec::new(),
        }
    }

    /// Add a new clash
    pub fn add_clash(&mut self, clash: Clash) {
        self.clashes.push(clash);
    }

    /// Check if any clashes have been detected
    #[must_use]
    pub fn has_clashes(&self) -> bool {
        !self.clashes.is_empty()
    }

    /// Get all detected clashes
    #[must_use]
    pub fn clashes(&self) -> &[Clash] {
        &self.clashes
    }

    /// Clear all clashes
    pub fn clear(&mut self) {
        self.clashes.clear();
    }

    /// Get the number of clashes
    #[must_use]
    pub fn clash_count(&self) -> usize {
        self.clashes.len()
    }

    /// Find clashes involving a specific node
    #[must_use]
    pub fn clashes_for_node(&self, node_id: NodeId) -> Vec<&Clash> {
        self.clashes
            .iter()
            .filter(|clash| clash.nodes.contains(&node_id))
            .collect()
    }
}

impl TableauStatistics {
    /// Create new statistics with start time
    #[must_use]
    pub fn new() -> Self {
        Self {
            start_time: Some(Instant::now()),
            ..Default::default()
        }
    }

    /// Increment node count
    pub fn increment_nodes(&mut self) {
        self.total_nodes += 1;
    }

    /// Increment edge count
    pub fn increment_edges(&mut self) {
        self.total_edges += 1;
    }

    /// Increment rule application count
    pub fn increment_rule_applications(&mut self) {
        self.rule_applications += 1;
    }

    /// Increment clash count
    pub fn increment_clashes(&mut self) {
        self.clashes_detected += 1;
    }

    /// Increment backtrack count
    pub fn increment_backtracks(&mut self) {
        self.backtracks += 1;
    }

    /// Finalize statistics (called when tableau construction completes)
    pub fn finalize(&mut self) {
        if let Some(start) = self.start_time {
            self.construction_time = start.elapsed();
        }
    }

    /// Get construction time as milliseconds
    #[must_use]
    pub fn construction_time_ms(&self) -> u128 {
        self.construction_time.as_millis()
    }

    /// Calculate nodes per second
    #[must_use]
    pub fn nodes_per_second(&self) -> f64 {
        if self.construction_time.as_secs_f64() > 0.0 {
            self.total_nodes as f64 / self.construction_time.as_secs_f64()
        } else {
            0.0
        }
    }
}

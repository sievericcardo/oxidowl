//! Monitoring and Statistics for HyperTableau
//!
//! This module provides comprehensive monitoring, statistics collection,
//! and debugging support for the hypertableau algorithm implementation.

use crate::{
    ontology::{Individual, ClassExpression, Axiom},
    Error, Result,
};

use super::{
    dependency_tracking::{DependencyTracker, DependencyStats},
    branching::{BranchingManager, BranchingStats, BranchingPointId},
    ground_disjunction::GroundDisjunction,
    hyperresolution::HyperresolutionStats,
    clause_evaluator::ClauseEvaluationStats,
    extension_tables::ExtensionStats,
};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
    fmt,
};

use serde::{Serialize, Deserialize};

/// Different levels of monitoring detail
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonitoringLevel {
    /// No monitoring (best performance)
    None,
    /// Basic statistics only
    Basic,
    /// Detailed statistics and timing
    Detailed,
    /// Full debugging information
    Debug,
}

/// Types of events that can be monitored
#[derive(Debug, Clone, PartialEq)]
pub enum MonitoredEvent {
    /// Clause application
    ClauseApplication {
        clause_id: usize,
        premises: Vec<String>,
        conclusion: String,
        duration: Duration,
    },
    /// Ground disjunction processing
    GroundDisjunctionProcessing {
        disjunction: String,
        individual: Individual,
        choice_count: usize,
        duration: Duration,
    },
    /// Branching point creation
    BranchingPointCreated {
        branch_id: BranchingPointId,
        branching_type: String,
        choice_count: usize,
    },
    /// Branching choice made
    BranchingChoiceMade {
        branch_id: BranchingPointId,
        choice_index: usize,
        choice_description: String,
    },
    /// Backtracking operation
    Backtracking {
        branch_id: BranchingPointId,
        retracted_facts: usize,
        duration: Duration,
    },
    /// Clash detection
    ClashDetected {
        individual: Individual,
        conflicting_concepts: Vec<String>,
        duration: Duration,
    },
    /// Fact derivation
    FactDerived {
        fact_description: String,
        individual: Individual,
        dependency_level: usize,
    },
    /// Extension table operation
    ExtensionOperation {
        operation_type: String,
        concept: String,
        individual: Individual,
        duration: Duration,
    },
    /// Blocking operation
    BlockingOperation {
        blocker: Individual,
        blocked: Individual,
        blocking_type: String,
        duration: Duration,
    },
}

/// Comprehensive statistics for the entire reasoning process
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStats {
    /// Overall timing information
    pub total_duration: Duration,
    pub startup_duration: Duration,
    pub reasoning_duration: Duration,
    pub cleanup_duration: Duration,
    
    /// Dependency tracking statistics
    pub dependency_stats: DependencyStats,
    
    /// Branching statistics
    pub branching_stats: BranchingStats,
    
    /// Hyperresolution statistics
    pub hyperresolution_stats: HyperresolutionStats,
    
    /// Clause evaluation statistics
    pub clause_evaluation_stats: ClauseEvaluationStats,
    
    /// Extension table statistics
    pub extension_stats: ExtensionStats,
    
    /// Memory usage information
    pub memory_usage: MemoryUsage,
    
    /// Performance metrics
    pub performance_metrics: PerformanceMetrics,
    
    /// Error and warning counts
    pub error_counts: ErrorCounts,
}

/// Memory usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub peak_memory_bytes: usize,
    pub current_memory_bytes: usize,
    pub dependency_sets_memory: usize,
    pub extension_tables_memory: usize,
    pub branching_points_memory: usize,
    pub clause_index_memory: usize,
}

/// Performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub facts_per_second: f64,
    pub clauses_per_second: f64,
    pub branching_points_per_second: f64,
    pub cache_hit_ratio: f64,
    pub average_reasoning_depth: f64,
    pub parallelization_efficiency: f64,
}

/// Error and warning counts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorCounts {
    pub total_errors: usize,
    pub total_warnings: usize,
    pub clash_count: usize,
    pub timeout_count: usize,
    pub memory_limit_exceeded: usize,
    pub infinite_loop_detected: usize,
}
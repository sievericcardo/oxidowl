//! Statistics and performance metrics for reasoning operations
//!
//! This module contains structures and utilities for tracking and reporting
//! reasoning performance statistics.

use std::time::Duration;

/// Statistics about reasoning operations
#[derive(Debug, Default, Clone)]
pub struct ReasoningStatistics {
    /// Number of consistency checks performed
    pub consistency_checks: u64,

    /// Number of satisfiability checks performed
    pub satisfiability_checks: u64,

    /// Number of subsumption checks performed
    pub subsumption_checks: u64,

    /// Total reasoning time
    pub total_reasoning_time: Duration,

    /// Cache hit ratio
    pub cache_hit_ratio: f64,

    /// Number of tableau nodes created
    pub tableau_nodes_created: u64,

    /// Number of backtracking operations
    pub backtracking_operations: u64,

    /// Maximum tableau depth reached
    pub max_tableau_depth: usize,
}

impl ReasoningStatistics {
    /// Create new empty statistics
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Reset all statistics to zero
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Update tableau statistics from a completed tableau run
    pub fn update_tableau_stats(&mut self, nodes: u64, backtracks: u64, max_depth: usize) {
        self.tableau_nodes_created += nodes;
        self.backtracking_operations += backtracks;
        self.max_tableau_depth = self.max_tableau_depth.max(max_depth);
    }

    /// Add reasoning time to the total
    pub fn add_reasoning_time(&mut self, duration: Duration) {
        self.total_reasoning_time += duration;
    }

    /// Increment consistency check counter
    pub fn increment_consistency_checks(&mut self) {
        self.consistency_checks += 1;
    }

    /// Increment satisfiability check counter
    pub fn increment_satisfiability_checks(&mut self) {
        self.satisfiability_checks += 1;
    }

    /// Increment subsumption check counter
    pub fn increment_subsumption_checks(&mut self) {
        self.subsumption_checks += 1;
    }

    /// Update cache hit ratio
    pub fn update_cache_hit_ratio(&mut self, ratio: f64) {
        self.cache_hit_ratio = ratio;
    }

    /// Get total number of reasoning operations
    #[must_use]
    pub fn total_operations(&self) -> u64 {
        self.consistency_checks + self.satisfiability_checks + self.subsumption_checks
    }

    /// Get average reasoning time per operation
    #[must_use]
    pub fn average_reasoning_time(&self) -> Duration {
        let total_ops = self.total_operations();
        if total_ops > 0 {
            self.total_reasoning_time / total_ops as u32
        } else {
            Duration::ZERO
        }
    }

    /// Generate a human-readable report
    #[must_use]
    pub fn report(&self) -> String {
        format!(
            "Reasoning Statistics:\n\
             - Consistency checks: {}\n\
             - Satisfiability checks: {}\n\
             - Subsumption checks: {}\n\
             - Total operations: {}\n\
             - Total reasoning time: {:?}\n\
             - Average time per operation: {:?}\n\
             - Cache hit ratio: {:.2}%\n\
             - Tableau nodes created: {}\n\
             - Backtracking operations: {}\n\
             - Maximum tableau depth: {}",
            self.consistency_checks,
            self.satisfiability_checks,
            self.subsumption_checks,
            self.total_operations(),
            self.total_reasoning_time,
            self.average_reasoning_time(),
            self.cache_hit_ratio * 100.0,
            self.tableau_nodes_created,
            self.backtracking_operations,
            self.max_tableau_depth
        )
    }
}

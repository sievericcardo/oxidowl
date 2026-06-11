//! Incremental Reasoning Framework
//!
//! This module provides incremental reasoning capabilities that build upon
//! the core reasoning infrastructure to enable efficient handling of ontology
//! changes without full re-reasoning.
//!
//! ## Key Components
//! - [`ChangeTracker`]: Monitors ontology modifications and dependencies
//! - [`DeltaComputer`]: Computes minimal reasoning updates
//! - [`IncrementalCacheManager`]: Manages selective cache invalidation
//! - [`IncrementalReasoningService`]: Main service interface

pub mod cache_management;
pub mod change_tracking;
pub mod delta_computation;
pub mod service;

pub use cache_management::{IncrementalCacheManager, InvalidationTracker};
pub use change_tracking::{ABoxChange, ChangeTracker, DependencyGraph, TBoxChange};
pub use delta_computation::{DeltaComputer, QueryDelta, ReasoningDelta};
pub use service::IncrementalReasoningService;

use std::time::Instant;

/// Represents a change event in the ontology with timestamp information
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeEvent {
    /// `TBox` (terminological) change
    TBox(TBoxChange),
    /// `ABox` (assertional) change  
    ABox(ABoxChange),
}

impl ChangeEvent {
    /// Get the timestamp when this change occurred
    #[must_use]
    pub fn timestamp(&self) -> Instant {
        match self {
            ChangeEvent::TBox(change) => change.timestamp(),
            ChangeEvent::ABox(change) => change.timestamp(),
        }
    }

    /// Get a human-readable description of the change
    #[must_use]
    pub fn description(&self) -> String {
        match self {
            ChangeEvent::TBox(change) => format!("TBox: {}", change.description()),
            ChangeEvent::ABox(change) => format!("ABox: {}", change.description()),
        }
    }
}

/// Configuration for incremental reasoning behavior
#[derive(Debug, Clone)]
pub struct IncrementalConfig {
    /// Maximum number of changes to track before compacting history
    pub max_change_history: usize,
    /// Whether to enable aggressive cache invalidation for consistency
    pub aggressive_invalidation: bool,
    /// Batch size for processing multiple changes efficiently
    pub change_batch_size: usize,
    /// Enable performance monitoring and statistics collection
    pub enable_profiling: bool,
}

impl Default for IncrementalConfig {
    fn default() -> Self {
        Self {
            max_change_history: 10000,
            aggressive_invalidation: false,
            change_batch_size: 100,
            enable_profiling: false,
        }
    }
}

/// Statistics for incremental reasoning performance monitoring
#[derive(Debug, Default, Clone)]
pub struct IncrementalStatistics {
    /// Number of incremental updates performed
    pub incremental_updates: usize,
    /// Number of full reasoning operations avoided
    pub avoided_full_reasoning: usize,
    /// Total time saved by incremental reasoning (in milliseconds)
    pub time_saved_ms: u64,
    /// Number of cache entries invalidated
    pub cache_invalidations: usize,
    /// Number of successful cache reuses
    pub cache_hits: usize,
    /// Memory usage for incremental data structures (in bytes)
    pub memory_usage_bytes: usize,
}

impl IncrementalStatistics {
    /// Calculate the cache hit ratio
    #[must_use]
    pub fn cache_hit_ratio(&self) -> f64 {
        if self.cache_hits + self.cache_invalidations == 0 {
            0.0
        } else {
            self.cache_hits as f64 / (self.cache_hits + self.cache_invalidations) as f64
        }
    }

    /// Get average time saved per incremental update
    #[must_use]
    pub fn average_time_saved_ms(&self) -> f64 {
        if self.incremental_updates == 0 {
            0.0
        } else {
            self.time_saved_ms as f64 / self.incremental_updates as f64
        }
    }
}

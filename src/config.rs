//! Configuration management for Oxidowl
//!
//! This module handles loading and managing configuration settings that control
//! the behavior of the reasoning engine, servers, and other components.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};

/// Main configuration structure for Oxidowl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonerConfig {
    /// General settings for the reasoning engine
    pub reasoning: ReasoningConfig,
    /// Cache settings for optimizing reasoning performance
    pub cache: CacheConfig,
    /// Server configuration for the reasoning service
    pub server: ServerConfig,
    /// Logging configuration for debugging and monitoring
    pub logging: LoggingConfig,
    /// Performance tuning parameters
    pub performance: PerformanceConfig,
}

/// Configuration for core reasoning tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningConfig {
    /// Blocking strategy for the tableau algorithm
    pub blocking_strategy: BlockingStrategy,
    /// Strategy for expanding the tableau
    pub expansion_strategy: ExpansionStrategy,
    /// Enable or disable optimisation
    pub enable_optimisations: bool,
    /// EMaximum response time for reasoning tasks
    pub timeout: Option<Duration>,
    /// Maximum memory usage in MB for the reasoner
    pub max_memory_mb: Option<u64>,
    /// Enable incremental reasoning
    pub incremental_reasoning: bool,
    /// Enable explanation generation
    pub enable_explanations: bool,
    /// Maximum tableau expansion depth
    pub max_expansion_depth: u32,
    /// Enable clash detection
    pub enable_clash_detection: bool,
}


/// Blocking strategy for the tableau algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockingStrategy {
    /// Anywhere blocking
    Anywhere,
    /// Ancestor blocking
    Ancestor,
    /// Pairwise blocking
    Pairwise,
    /// Dynamic blocking (adaptive)
    Dynamic,
}

/// Existential expansion strategy for the tableau
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ExpansionStrategy {
    /// Creation order strategy
    CreationOrder,
    /// Individual reuse strategy
    IndividualReuse,
    /// Priority-based expansion
    Priority,
}

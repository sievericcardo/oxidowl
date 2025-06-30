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

/// Configuration for caching mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable satisfiability cache
    pub enable_satisfiability_cache: bool,
    /// Enable completion graph caching
    pub enable_completion_graph_cache: bool,
    /// Enable unsatisfiability cache
    pub enable_unsatisfiability_cache: bool,
    /// Maximum size of the cache in MB
    pub max_cache_size_mb: u64,
    /// Time to live for cache entries
    pub cache_ttl: Option<std::time::Duration>,
    /// Cache eviction strategy
    pub eviction_strategy: CacheEvictionStrategy,
    /// Cache persistence settings
    pub persistence: bool,
}

/// Cache eviction strategies
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CacheEvictionStrategy {
    /// Least Recently Used (LRU) eviction
    LRU,
    /// Least Frequently Used (LFU) eviction
    LFU,
    /// Time-based eviction
    TimeToLive,
    /// Random eviction
    Random,
}

/// Server configuration for the reasoning service
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Default port for the reasoning server
    pub port: u16,
    /// Default bind address for the server
    pub bind_address: String,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Request timeout duration
    pub request_timeout: Duration,
    /// Enable CORS (Cross-Origin Resource Sharing) headers
    pub enable_cors: bool,
    /// Maximum request size in bytes
    pub max_request_size: usize,
}


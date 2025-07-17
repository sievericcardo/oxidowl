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
    /// Tableau algorithm type to use
    pub tableau_algorithm: TableauAlgorithm,
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

/// Tableau algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableauAlgorithm {
    /// Traditional tableau algorithm
    Traditional,
    /// HyperTableau algorithm (HermiT-style)
    HyperTableau,
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

/// Logging configuration for the reasoner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Logging level
    pub level: LogLevel,
    /// Enable or disable file logging
    pub enable_file_logging: bool,
    /// Path to the log file
    pub log_file_path: Option<String>,
    /// Log rotation settings in MB
    pub log_rotation_size_mb: u64,
    /// Maximum number of log files to keep
    pub max_log_files: u32,
}

/// Logging levels
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum LogLevel {
    /// Debug level logging
    Debug,
    /// Info level logging
    Info,
    /// Warning level logging
    Warning,
    /// Error level logging
    Error,
    /// Trace level logging
    Trace,
}

/// Performance configuration for the reasoner
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Number of threads for parallel reasoning
    pub worker_threads: Option<usize>,
    /// Enable or disable tableau expansion
    pub enable_parallel_expansion: bool,
    /// Enable or disable optimisations
    pub enable_simd: bool,
    /// Memory pool in MB
    pub memory_pool_size_mb: u64,
    /// Garbage collection threshold
    pub gc_threshold: f64,
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        Self {
            reasoning: ReasoningConfig {
                tableau_algorithm: TableauAlgorithm::Traditional,
                blocking_strategy: BlockingStrategy::Anywhere,
                expansion_strategy: ExpansionStrategy::CreationOrder,
                enable_optimisations: true,
                timeout: Some(Duration::from_secs(300)), // 5 minutes
                max_memory_mb: Some(4096), // 4 GB
                incremental_reasoning: false,
                enable_explanations: false,
                max_expansion_depth: 100,
                enable_clash_detection: true,
            },
            cache: CacheConfig {
                enable_satisfiability_cache: true,
                enable_completion_graph_cache: true,
                enable_unsatisfiability_cache: false,
                max_cache_size_mb: 1024, // 1 GB
                cache_ttl: Some(Duration::from_secs(3600)),
                eviction_strategy: CacheEvictionStrategy::LRU,
                persistence: false,
            },
            server: ServerConfig {
                port: 8080,
                bind_address: "127.0.01".to_string(),
                max_connections: 100,
                request_timeout: Duration::from_secs(30),
                enable_cors: true,
                max_request_size: 50 * 1024 * 1024, // 50MB
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                enable_file_logging: false,
                log_file_path: None,
                log_rotation_size_mb: 100, // 100 MB
                max_log_files: 5,
            },
            performance: PerformanceConfig {
                worker_threads: Some(4), // Default to 4 threads
                enable_parallel_expansion: true,
                enable_simd: true,
                memory_pool_size_mb: 512, // 512 MB
                gc_threshold: 0.75, // 75% threshold for GC
            },
        }
    }
}

impl ReasonerConfig {
    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ReasonerConfig = toml::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse TOML config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string(self)
            .map_err(|e| Error::config(format!("Failed to serialize TOML config: {}", e)))?;

        fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from JSON file
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ReasonerConfig = serde_json::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse JSON config: {}", e)))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::config(format!("Failed to serialize JSON config: {}", e)))?;

        fs::write(path, content)?;
        Ok(())
    }

    /// Validate the configuration settings
    pub fn validate(&self) -> Result<()> {
        // Validate timeouts
        if let Some(timeout) = self.reasoning.timeout {
            if timeout.as_secs() == 0 {
                return Err(Error::config("Timeout cannot be zero".to_string()));
            }
        }

        // Validate memory limits
        if let Some(max_memory) = self.reasoning.max_memory_mb {
            if max_memory == 0 {
                return Err(Error::config("Maximum memory cannot be zero".to_string()));
            }
        }

        // Validate cache settings
        if self.cache.max_cache_size_mb == 0 {
            return Err(Error::config("Maximum cache size cannot be zero".to_string()));
        }

        // Validate server settings
        if self.server.max_connections == 0 {
            return Err(Error::config("Maximum connections cannot be zero".to_string()));
        }

        if self.server.request_timeout.as_secs() == 0 {
            return Err(Error::config("Request timeout cannot be zero".to_string()));
        }

        Ok(())
    }

    /// Get the worker threads to use
    pub fn worker_thread_count(&self) -> usize {
        self.performance.worker_threads.unwrap_or_else(|| num_cpus::get())
    }

    /// Check if parallel processing is enabled
    pub fn is_parallel_processing_enabled(&self) -> bool {
        self.performance.enable_parallel_expansion && self.worker_thread_count() > 1
    }
}

/// Create a configuration for specific use cases
impl ReasonerConfig {
    /// Configuration for large ontologies
    pub fn large_ontology_config() -> Self {
        let mut config = Self::default();
        config.reasoning.max_memory_mb = Some(8192); // 8 GB
        config.cache.max_cache_size_mb = 2048; // 2 GB
        config.performance.memory_pool_size_mb = 1024; // 1 GB
        config.reasoning.timeout = Some(Duration::from_secs(1800)); // 30 minutes
        config.reasoning.max_expansion_depth = 200; // Increase depth for large ontologies
        config.reasoning.enable_optimisations = true; // Enable optimisations
        config
    }

    /// Configuration for web services
    pub fn web_service_config() -> Self {
        let mut config = Self::default();
        config.server.max_connections = 500; // Increase for web service
        config.server.request_timeout = Duration::from_secs(60); // 1 minute timeout
        config.reasoning.timeout = Some(Duration::from_secs(120)); // 2 minutes
        config.cache.persistence = true; // Enable cache persistence
        config
    }

    /// Configuration for debugging and development
    pub fn debug_config() -> Self {
        let mut config = Self::default();
        config.logging.level = LogLevel::Debug; // Set logging to debug level
        config.reasoning.enable_explanations = true; // Enable explanations
        config.reasoning.enable_clash_detection = false; // Disable clash detection for debugging
        config.cache.enable_satisfiability_cache = false; // Disable satisfiability cache for debugging
        config.performance.worker_threads = Some(1); // Use single thread for debugging
        config
    }

    /// Configuration for production environments
    pub fn production_config() -> Self {
        let mut config = Self::default();
        config.logging.level = LogLevel::Info; // Set logging to info level
        config.reasoning.enable_explanations = false; // Disable explanations in production
        config.reasoning.enable_clash_detection = true; // Enable clash detection
        config.cache.enable_satisfiability_cache = true; // Enable satisfiability cache
        config.performance.worker_threads = Some(8); // Use multiple threads for production
        config.performance.enable_parallel_expansion = true; // Enable parallel expansion
        config
    }

    /// Configuration for testing purposes
    pub fn test_config() -> Self {
        let mut config = Self::default();
        config.logging.level = LogLevel::Debug; // Set logging to debug level for tests
        config.reasoning.enable_explanations = true; // Enable explanations for tests
        config.reasoning.enable_clash_detection = false; // Disable clash detection for tests
        config.cache.enable_satisfiability_cache = true; // Enable satisfiability cache for tests
        config.performance.worker_threads = Some(2); // Use 2 threads for testing
        config.performance.enable_parallel_expansion = false; // Disable parallel expansion for tests
        config
    }
}

/// Network service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub enable_owllink: bool,
    pub enable_sparql: bool,
    pub enable_http: bool,
    pub enable_websocket: bool,
    pub owllink_address: std::net::SocketAddr,
    pub sparql_address: std::net::SocketAddr,
    pub http_address: std::net::SocketAddr,
    pub websocket_address: std::net::SocketAddr,
    pub request_timeout_seconds: u64,
}

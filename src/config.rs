//! Configuration management for Oxidowl
//!
//! This module handles loading and managing configuration settings that control
//! the behavior of the reasoning engine, servers, and other components.

use crate::{Error, Result};
use enumset::{EnumSet, EnumSetType};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path, time::Duration};

/// Features that can be enabled in reasoning
#[derive(EnumSetType, Debug, Serialize, Deserialize)]
pub enum ReasoningFeature {
    /// Enable optimizations
    Optimizations,
    /// Enable explanation generation
    Explanations,
    /// Enable clash detection
    ClashDetection,
    /// Enable blockers cache (HermiT-style)
    BlockersCache,
}

/// Features that can be enabled in the cache
#[derive(EnumSetType, Debug, Serialize, Deserialize)]
pub enum CacheFeature {
    /// Enable satisfiability cache
    Satisfiability,
    /// Enable completion graph cache
    CompletionGraph,
    /// Enable unsatisfiability cache
    Unsatisfiability,
}

/// Features that can be enabled on the server
#[derive(EnumSetType, Debug, Serialize, Deserialize)]
pub enum ServerFeature {
    /// Enable server on startup
    Server,
    /// Enable CORS headers
    Cors,
    /// Enable `OWLlink` server
    OWLlink,
    /// Enable SPARQL endpoint
    SPARQL,
    /// Enable REST API
    RestAPI,
}

/// Performance features that can be enabled
#[derive(EnumSetType, Debug, Serialize, Deserialize)]
pub enum PerformanceFeature {
    /// Enable parallel tableau expansion
    ParallelExpansion,
    /// Enable SIMD optimizations
    SIMD,
    /// Enable NUMA-aware allocation
    NumaAwareness,
    /// Enable lock-free concurrent data structures
    LockFree,
    /// Enable persistent data structures for structural sharing
    PersistentCollections,
}

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
    /// Enabled reasoning features
    pub features: EnumSet<ReasoningFeature>,
    /// Maximum response time for reasoning tasks
    pub timeout: Option<Duration>,
    /// Maximum memory usage in MB for the reasoner
    pub max_memory_mb: Option<u64>,
    /// Enable incremental reasoning
    pub incremental_reasoning: bool,
    /// Maximum tableau expansion depth
    pub max_expansion_depth: u32,
    /// Ignore unsupported datatypes
    pub ignore_unsupported_datatypes: bool,
    /// Dump DL clauses for debugging
    pub dump_clauses: bool,
    /// Clause dump file path
    pub clause_dump_file: Option<String>,
    /// Target OWL profile for optimization
    pub target_profile: OWLProfile,
}

impl ReasoningConfig {
    /// Check if a specific feature is enabled
    #[must_use]
    pub fn is_enabled(&self, feature: ReasoningFeature) -> bool {
        self.features.contains(feature)
    }

    /// Enable a specific feature
    pub fn enable(&mut self, feature: ReasoningFeature) {
        self.features.insert(feature);
    }

    /// Disable a specific feature
    pub fn disable(&mut self, feature: ReasoningFeature) {
        self.features.remove(feature);
    }
}

/// Tableau algorithm types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableauAlgorithm {
    /// Traditional tableau algorithm
    Traditional,
    /// Hypertableau algorithm (2-5x faster, uses structural sharing)
    Hypertableau,
    /// Profile-optimized algorithms
    ProfileOptimized,
}

/// OWL 2 Profile types for optimization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OWLProfile {
    /// Full OWL 2 DL
    DL,
    /// OWL 2 EL (Existential Language)
    EL,
    /// OWL 2 QL (Query Language)
    QL,
    /// OWL 2 RL (Rule Language)
    RL,
    /// Auto-detect profile
    Auto,
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
    /// Single blocking (HermiT-style)
    Single,
    /// Core blocking (HermiT-style)
    Core,
    /// Optimal blocking (HermiT-style)
    Optimal,
    /// Indexed anywhere blocking (O(1) lookup with hash-based index)
    Indexed,
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
    /// EL expansion strategy (HermiT-style)
    EL,
    /// Optimal expansion strategy (HermiT-style)
    Optimal,
}

/// Configuration for caching mechanisms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enabled cache features
    pub features: EnumSet<CacheFeature>,
    /// Maximum size of the cache in MB
    pub max_cache_size_mb: u64,
    /// Time to live for cache entries
    pub cache_ttl: Option<std::time::Duration>,
    /// Cache eviction strategy
    pub eviction_strategy: CacheEvictionStrategy,
    /// Cache persistence settings
    pub persistence: bool,
}

impl CacheConfig {
    /// Check if a specific feature is enabled
    #[must_use]
    pub fn is_enabled(&self, feature: CacheFeature) -> bool {
        self.features.contains(feature)
    }

    /// Enable a specific feature
    pub fn enable(&mut self, feature: CacheFeature) {
        self.features.insert(feature);
    }

    /// Disable a specific feature
    pub fn disable(&mut self, feature: CacheFeature) {
        self.features.remove(feature);
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        let mut features = EnumSet::new();
        features.insert(CacheFeature::Satisfiability);
        features.insert(CacheFeature::CompletionGraph);

        Self {
            features,
            max_cache_size_mb: 100,
            cache_ttl: None,
            eviction_strategy: CacheEvictionStrategy::LRU,
            persistence: false,
        }
    }
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
    /// Enabled server features
    pub features: EnumSet<ServerFeature>,
    /// Default port for the reasoning server
    pub port: u16,
    /// Default bind address for the server
    pub bind_address: String,
    /// Maximum number of concurrent connections
    pub max_connections: usize,
    /// Request timeout duration
    pub request_timeout: Duration,
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// `OWLlink` server port
    pub owllink_port: u16,
    /// SPARQL endpoint port
    pub sparql_port: u16,
    /// REST API port
    pub rest_api_port: u16,
}

impl ServerConfig {
    /// Check if a specific feature is enabled
    #[must_use]
    pub fn is_enabled(&self, feature: ServerFeature) -> bool {
        self.features.contains(feature)
    }

    /// Enable a specific feature
    pub fn enable(&mut self, feature: ServerFeature) {
        self.features.insert(feature);
    }

    /// Disable a specific feature
    pub fn disable(&mut self, feature: ServerFeature) {
        self.features.remove(feature);
    }
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
    /// Performance profile (Low/Medium/High/Ultra)
    pub profile: PerformanceProfile,
    /// Number of threads for parallel reasoning (overrides profile default)
    pub worker_threads: Option<usize>,
    /// Enabled performance features
    pub features: EnumSet<PerformanceFeature>,
    /// Memory pool in MB
    pub memory_pool_size_mb: u64,
    /// Garbage collection threshold
    pub gc_threshold: f64,
    /// Maximum classification parallelism (concurrent subsumption tests)
    pub max_parallel_classification_tasks: Option<usize>,
}

impl PerformanceConfig {
    /// Check if a specific feature is enabled
    #[must_use]
    pub fn is_enabled(&self, feature: PerformanceFeature) -> bool {
        self.features.contains(feature)
    }

    /// Enable a specific feature
    pub fn enable(&mut self, feature: PerformanceFeature) {
        self.features.insert(feature);
    }

    /// Disable a specific feature
    pub fn disable(&mut self, feature: PerformanceFeature) {
        self.features.remove(feature);
    }
}

/// Performance profile presets for different resource levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PerformanceProfile {
    /// Low: Minimal resources (1-2 cores, basic caching, minimal memory)
    /// Good for: Embedded systems, resource-constrained environments
    Low,
    /// Medium: Balanced resources (4-8 cores, standard caching, moderate memory)
    /// Good for: Desktop applications, typical servers
    Medium,
    /// High: Performance-focused (16+ cores, aggressive caching, SIMD, optimized memory)
    /// Good for: High-performance servers, classification workloads [DEFAULT]
    #[default]
    High,
    /// Ultra: Maximum performance (all cores, NUMA-aware, maximum caching, maximum memory)
    /// Good for: Multi-socket servers, very large ontologies (100K+ concepts)
    Ultra,
}

impl PerformanceProfile {
    /// Get the recommended number of worker threads for this profile
    #[must_use]
    pub fn worker_threads(&self) -> usize {
        match self {
            Self::Low => 2,
            Self::Medium => num_cpus::get().min(8),
            Self::High => num_cpus::get().min(32),
            Self::Ultra => num_cpus::get(),
        }
    }

    /// Get the recommended cache size in MB for this profile
    #[must_use]
    pub fn cache_size_mb(&self) -> u64 {
        match self {
            Self::Low => 50,
            Self::Medium => 200,
            Self::High => 1024,
            Self::Ultra => 4096,
        }
    }

    /// Get the memory pool size in MB for this profile
    #[must_use]
    pub fn memory_pool_size_mb(&self) -> u64 {
        match self {
            Self::Low => 128,
            Self::Medium => 512,
            Self::High => 2048,
            Self::Ultra => 8192,
        }
    }

    /// Whether SIMD should be enabled for this profile
    #[must_use]
    pub fn enable_simd(&self) -> bool {
        matches!(self, Self::High | Self::Ultra)
    }

    /// Whether NUMA awareness should be enabled for this profile
    #[must_use]
    pub fn enable_numa_awareness(&self) -> bool {
        matches!(self, Self::Ultra)
    }

    /// Whether lock-free data structures should be enabled
    #[must_use]
    pub fn enable_lock_free(&self) -> bool {
        matches!(self, Self::High | Self::Ultra)
    }

    /// Whether persistent collections should be enabled
    #[must_use]
    pub fn enable_persistent_collections(&self) -> bool {
        matches!(self, Self::High | Self::Ultra)
    }

    /// Get the recommended performance features for this profile
    #[must_use]
    pub fn features(&self) -> EnumSet<PerformanceFeature> {
        let mut features = EnumSet::new();
        features.insert(PerformanceFeature::ParallelExpansion);

        if self.enable_simd() {
            features.insert(PerformanceFeature::SIMD);
        }
        if self.enable_numa_awareness() {
            features.insert(PerformanceFeature::NumaAwareness);
        }
        if self.enable_lock_free() {
            features.insert(PerformanceFeature::LockFree);
        }
        if self.enable_persistent_collections() {
            features.insert(PerformanceFeature::PersistentCollections);
        }

        features
    }

    /// Maximum parallel classification tasks
    #[must_use]
    pub fn max_parallel_classification_tasks(&self) -> usize {
        match self {
            Self::Low => 4,
            Self::Medium => 64,
            Self::High => 1024,
            Self::Ultra => 10000,
        }
    }
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        let profile = PerformanceProfile::default();
        Self {
            profile,
            worker_threads: None, // Will use profile default
            features: profile.features(),
            memory_pool_size_mb: profile.memory_pool_size_mb(),
            gc_threshold: 0.75,
            max_parallel_classification_tasks: Some(profile.max_parallel_classification_tasks()),
        }
    }
}

impl PerformanceConfig {
    /// Create a configuration from a performance profile
    #[must_use]
    pub fn from_profile(profile: PerformanceProfile) -> Self {
        Self {
            profile,
            worker_threads: None,
            features: profile.features(),
            memory_pool_size_mb: profile.memory_pool_size_mb(),
            gc_threshold: 0.75,
            max_parallel_classification_tasks: Some(profile.max_parallel_classification_tasks()),
        }
    }

    /// Get the effective number of worker threads
    #[must_use]
    pub fn get_worker_threads(&self) -> usize {
        self.worker_threads
            .unwrap_or_else(|| self.profile.worker_threads())
    }

    /// Set performance profile from environment variable
    #[must_use]
    pub fn from_env() -> Self {
        let profile = std::env::var("OXIDOWL_PERFORMANCE_PROFILE")
            .ok()
            .and_then(|s| match s.to_lowercase().as_str() {
                "low" => Some(PerformanceProfile::Low),
                "medium" => Some(PerformanceProfile::Medium),
                "high" => Some(PerformanceProfile::High),
                "ultra" => Some(PerformanceProfile::Ultra),
                _ => None,
            })
            .unwrap_or_default();

        Self::from_profile(profile)
    }
}

/// Configuration specific to tableau reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableauConfig {
    /// Maximum tableau expansion depth
    pub max_depth: u32,
    /// Timeout for tableau expansion
    pub timeout: Option<Duration>,
    /// Enable/disable blocking optimization
    pub blocking_enabled: bool,
    /// Enable/disable general optimizations
    pub optimization_enabled: bool,
    /// RDF 1.1 compatibility mode (disables RDF-star features)
    pub rdf11_mode: bool,
    /// Maximum depth for quoted triple reasoning (0 = disabled)
    pub quoted_triple_reasoning_depth: usize,
}

impl Default for TableauConfig {
    fn default() -> Self {
        Self {
            max_depth: 100,
            timeout: Some(Duration::from_secs(300)),
            blocking_enabled: true,
            optimization_enabled: true,
            rdf11_mode: false,                // RDF-star enabled by default
            quoted_triple_reasoning_depth: 2, // Allow 2 levels of nesting
        }
    }
}

impl Default for ReasoningConfig {
    fn default() -> Self {
        let mut features = EnumSet::new();
        features.insert(ReasoningFeature::Optimizations);
        features.insert(ReasoningFeature::ClashDetection);

        Self {
            tableau_algorithm: TableauAlgorithm::Traditional,
            blocking_strategy: BlockingStrategy::Anywhere,
            expansion_strategy: ExpansionStrategy::CreationOrder,
            features,
            timeout: Some(Duration::from_secs(300)), // 5 minutes
            max_memory_mb: Some(4096),               // 4 GB
            incremental_reasoning: false,
            max_expansion_depth: 100,
            ignore_unsupported_datatypes: false,
            dump_clauses: false,
            clause_dump_file: None,
            target_profile: OWLProfile::Auto,
        }
    }
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        let mut reasoning_features = EnumSet::new();
        reasoning_features.insert(ReasoningFeature::Optimizations);
        reasoning_features.insert(ReasoningFeature::ClashDetection);

        let mut cache_features = EnumSet::new();
        cache_features.insert(CacheFeature::Satisfiability);
        cache_features.insert(CacheFeature::CompletionGraph);

        let mut server_features = EnumSet::new();
        server_features.insert(ServerFeature::Cors);
        server_features.insert(ServerFeature::RestAPI);

        Self {
            reasoning: ReasoningConfig {
                tableau_algorithm: TableauAlgorithm::Traditional,
                blocking_strategy: BlockingStrategy::Anywhere,
                expansion_strategy: ExpansionStrategy::CreationOrder,
                features: reasoning_features,
                timeout: Some(Duration::from_secs(300)), // 5 minutes
                max_memory_mb: Some(4096),               // 4 GB
                incremental_reasoning: false,
                max_expansion_depth: 100,
                ignore_unsupported_datatypes: false,
                dump_clauses: false,
                clause_dump_file: None,
                target_profile: OWLProfile::Auto,
            },
            cache: CacheConfig {
                features: cache_features,
                max_cache_size_mb: 1024, // 1 GB
                cache_ttl: Some(Duration::from_secs(3600)),
                eviction_strategy: CacheEvictionStrategy::LRU,
                persistence: false,
            },
            server: ServerConfig {
                features: server_features,
                port: 8080,
                bind_address: "127.0.0.1".to_string(),
                max_connections: 100,
                request_timeout: Duration::from_secs(30),
                max_request_size: 50 * 1024 * 1024, // 50MB
                owllink_port: 8081,
                sparql_port: 8082,
                rest_api_port: 8080,
            },
            logging: LoggingConfig {
                level: LogLevel::Info,
                enable_file_logging: false,
                log_file_path: None,
                log_rotation_size_mb: 100, // 100 MB
                max_log_files: 5,
            },
            performance: PerformanceConfig::default(),
        }
    }
}

impl ReasonerConfig {
    /// Load configuration from TOML file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ReasonerConfig = toml::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse TOML config: {e}")))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to TOML file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string(self)
            .map_err(|e| Error::config(format!("Failed to serialize TOML config: {e}")))?;

        fs::write(path, content)?;
        Ok(())
    }

    /// Load configuration from JSON file
    pub fn load_from_json<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: ReasonerConfig = serde_json::from_str(&content)
            .map_err(|e| Error::config(format!("Failed to parse JSON config: {e}")))?;

        config.validate()?;
        Ok(config)
    }

    /// Save configuration to JSON file
    pub fn save_to_json<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| Error::config(format!("Failed to serialize JSON config: {e}")))?;

        fs::write(path, content)?;
        Ok(())
    }

    /// Validate the configuration settings
    pub fn validate(&self) -> Result<()> {
        // Validate timeouts
        if let Some(timeout) = self.reasoning.timeout
            && timeout.as_secs() == 0
        {
            return Err(Error::config("Timeout cannot be zero".to_string()));
        }

        // Validate memory limits
        if let Some(max_memory) = self.reasoning.max_memory_mb
            && max_memory == 0
        {
            return Err(Error::config("Maximum memory cannot be zero".to_string()));
        }

        // Validate cache settings
        if self.cache.max_cache_size_mb == 0 {
            return Err(Error::config(
                "Maximum cache size cannot be zero".to_string(),
            ));
        }

        // Validate server settings
        if self.server.max_connections == 0 {
            return Err(Error::config(
                "Maximum connections cannot be zero".to_string(),
            ));
        }

        if self.server.request_timeout.as_secs() == 0 {
            return Err(Error::config("Request timeout cannot be zero".to_string()));
        }

        Ok(())
    }

    /// Get the worker threads to use
    pub fn worker_thread_count(&self) -> usize {
        self.performance
            .worker_threads
            .unwrap_or_else(num_cpus::get)
    }

    /// Check if parallel processing is enabled
    #[must_use]
    pub fn is_parallel_processing_enabled(&self) -> bool {
        self.performance
            .is_enabled(PerformanceFeature::ParallelExpansion)
            && self.worker_thread_count() > 1
    }
}

/// Create a configuration for specific use cases
impl ReasonerConfig {
    /// Configuration for large ontologies
    #[must_use]
    pub fn large_ontology_config() -> Self {
        let mut config = Self::default();
        config.reasoning.max_memory_mb = Some(8192); // 8 GB
        config.cache.max_cache_size_mb = 2048; // 2 GB
        config.performance.memory_pool_size_mb = 1024; // 1 GB
        config.reasoning.timeout = Some(Duration::from_secs(1800)); // 30 minutes
        config.reasoning.max_expansion_depth = 200; // Increase depth for large ontologies
        config.reasoning.enable(ReasoningFeature::Optimizations); // Enable optimisations
        config
    }

    /// Configuration for web services
    #[must_use]
    pub fn web_service_config() -> Self {
        let mut config = Self::default();
        config.server.max_connections = 500; // Increase for web service
        config.server.request_timeout = Duration::from_secs(60); // 1 minute timeout
        config.reasoning.timeout = Some(Duration::from_secs(120)); // 2 minutes
        config.cache.persistence = true; // Enable cache persistence
        config
    }

    /// Configuration for debugging and development
    #[must_use]
    pub fn debug_config() -> Self {
        let mut config = Self::default();
        config.logging.level = LogLevel::Debug; // Set logging to debug level
        config.reasoning.enable(ReasoningFeature::Explanations); // Enable explanations
        config.reasoning.disable(ReasoningFeature::ClashDetection); // Disable clash detection for debugging
        config.cache.disable(CacheFeature::Satisfiability); // Disable satisfiability cache for debugging
        config.performance.worker_threads = Some(1); // Use single thread for debugging
        config
    }

    /// Configuration for production environments
    #[must_use]
    pub fn production_config() -> Self {
        let mut config = Self::default();
        config.logging.level = LogLevel::Info; // Set logging to info level
        config.reasoning.disable(ReasoningFeature::Explanations); // Disable explanations in production
        config.reasoning.enable(ReasoningFeature::ClashDetection); // Enable clash detection
        config.cache.enable(CacheFeature::Satisfiability); // Enable satisfiability cache
        config.performance.worker_threads = Some(8); // Use multiple threads for production
        config
            .performance
            .enable(PerformanceFeature::ParallelExpansion); // Enable parallel expansion
        config
    }

    /// Configuration for testing purposes
    #[must_use]
    pub fn test_config() -> Self {
        let mut config = Self::default();
        config.logging.level = LogLevel::Debug; // Set logging to debug level for tests
        config.reasoning.enable(ReasoningFeature::Explanations); // Enable explanations for tests
        config.reasoning.disable(ReasoningFeature::ClashDetection); // Disable clash detection for tests
        config.cache.enable(CacheFeature::Satisfiability); // Enable satisfiability cache for tests
        config.performance.worker_threads = Some(2); // Use 2 threads for testing
        config
            .performance
            .disable(PerformanceFeature::ParallelExpansion); // Disable parallel expansion for tests
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

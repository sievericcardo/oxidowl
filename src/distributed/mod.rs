//! Distributed Query Processing Module
//!
//! Phase 2.2 implementation providing distributed reasoning capabilities across multiple nodes.
//! This module enables horizontal scaling of OxidOWL for large-scale ontology processing.
//!
//! # Architecture Overview
//!
//! The distributed system consists of several key components:
//! - **Cluster Manager**: Node discovery, health monitoring, and lifecycle management
//! - **Query Distributor**: Intelligent query splitting and task distribution
//! - **Result Aggregator**: Parallel result collection and merging
//! - **Fault Tolerance**: Failure detection, recovery, and re-execution strategies
//! - **Load Balancer**: Dynamic workload distribution based on node capacity
//!
//! # Features
//!
//! - **Automatic Node Discovery**: Dynamic cluster formation and scaling
//! - **Query Parallelization**: Intelligent decomposition of complex queries
//! - **Fault Tolerance**: Automatic recovery from node failures
//! - **Load Balancing**: Optimal resource utilization across the cluster
//! - **Result Consistency**: Guaranteed correctness of distributed computations

pub mod cluster;
pub mod coordination;
pub mod fault_tolerance;
pub mod load_balancing;
pub mod query_distribution;
pub mod result_aggregation;

// Core distributed processing types
pub use cluster::{ClusterManager, ClusterState, NodeHealth, NodeInfo};
pub use coordination::{ClusterCoordinator, ConsensusProtocol, DistributedLock};
pub use fault_tolerance::{FailureDetector, FaultTolerance, RecoveryStrategy};
pub use load_balancing::{LoadBalancer, NodeLoad, WorkloadDistribution};
pub use query_distribution::{DistributedQuery, QueryDistributor, QueryPartition};
pub use result_aggregation::{AggregatedResult, PartialResult, ResultAggregator};

use crate::prelude::*;
use std::net::SocketAddr;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for cluster nodes
pub type NodeId = Uuid;

/// Address information for cluster nodes
pub type NodeAddress = SocketAddr;

/// Configuration for distributed query processing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributedConfig {
    /// Local node configuration
    pub node_config: NodeConfig,

    /// Cluster discovery and communication settings
    pub cluster_config: ClusterConfig,

    /// Query distribution parameters
    pub query_config: QueryDistributionConfig,

    /// Fault tolerance settings
    pub fault_tolerance_config: FaultToleranceConfig,

    /// Load balancing configuration
    pub load_balancing_config: LoadBalancingConfig,
}

/// Configuration for individual nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Unique node identifier
    pub node_id: NodeId,

    /// Node's network address
    pub address: NodeAddress,

    /// Node capabilities and resources
    pub capabilities: NodeCapabilities,

    /// Node-specific settings
    pub settings: NodeSettings,
}

/// Node capabilities and resource specifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeCapabilities {
    /// Available CPU cores
    pub cpu_cores: usize,

    /// Available memory in MB
    pub memory_mb: usize,

    /// Storage capacity in GB
    pub storage_gb: usize,

    /// Network bandwidth in Mbps
    pub network_bandwidth_mbps: usize,

    /// Specialized reasoning capabilities
    pub reasoning_features: Vec<String>,
}

/// Node-specific configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSettings {
    /// Maximum concurrent queries
    pub max_concurrent_queries: usize,

    /// Query timeout in seconds
    pub query_timeout_seconds: u64,

    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,

    /// Enable performance monitoring
    pub enable_monitoring: bool,
}

/// Cluster-level configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Cluster name/identifier
    pub cluster_name: String,

    /// Discovery mechanism settings
    pub discovery: DiscoveryConfig,

    /// Communication protocols
    pub communication: CommunicationConfig,

    /// Consensus protocol settings
    pub consensus: ConsensusConfig,
}

/// Node discovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Discovery method (multicast, zookeeper, static, etc.)
    pub method: DiscoveryMethod,

    /// Discovery timeout in seconds
    pub timeout_seconds: u64,

    /// Retry attempts for discovery
    pub retry_attempts: usize,

    /// Discovery-specific parameters
    pub parameters: HashMap<String, String>,
}

/// Available discovery methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiscoveryMethod {
    /// Multicast discovery
    Multicast { address: String, port: u16 },

    /// Static node list
    Static { nodes: Vec<NodeAddress> },

    /// External service discovery
    External { service_url: String },

    /// Consul-based discovery
    Consul { consul_address: String },
}

/// Communication protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunicationConfig {
    /// Protocol type (TCP, gRPC, etc.)
    pub protocol: CommunicationProtocol,

    /// Connection timeout in seconds
    pub connection_timeout_seconds: u64,

    /// Keep-alive settings
    pub keep_alive: bool,

    /// Compression settings
    pub compression: CompressionConfig,
}

/// Available communication protocols
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommunicationProtocol {
    /// TCP-based communication
    Tcp,

    /// gRPC communication
    Grpc,

    /// HTTP-based RESTful API
    Http,

    /// WebSocket communication
    WebSocket,
}

/// Compression configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Enable compression
    pub enabled: bool,

    /// Compression algorithm
    pub algorithm: CompressionAlgorithm,

    /// Compression level
    pub level: u8,
}

/// Available compression algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// No compression
    None,

    /// GZIP compression
    Gzip,

    /// LZ4 compression
    Lz4,

    /// Zstandard compression
    Zstd,
}

/// Consensus protocol configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusConfig {
    /// Consensus algorithm
    pub algorithm: ConsensusAlgorithm,

    /// Leader election timeout
    pub election_timeout_ms: u64,

    /// Heartbeat interval
    pub heartbeat_interval_ms: u64,

    /// Minimum cluster size for consensus
    pub min_cluster_size: usize,
}

/// Available consensus algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConsensusAlgorithm {
    /// Raft consensus algorithm
    Raft,

    /// Simple leader election
    LeaderElection,

    /// No consensus (single node)
    None,
}

/// Query distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryDistributionConfig {
    /// Distribution strategy
    pub strategy: DistributionStrategy,

    /// Minimum partition size
    pub min_partition_size: usize,

    /// Maximum partitions per query
    pub max_partitions: usize,

    /// Load balancing parameters
    pub load_balancing: LoadBalancingConfig,
}

/// Query distribution strategies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DistributionStrategy {
    /// Distribute by ontology concepts
    ConceptBased,

    /// Distribute by query complexity
    ComplexityBased,

    /// Round-robin distribution
    RoundRobin,

    /// Load-aware distribution
    LoadAware,

    /// Hybrid strategy combining multiple approaches
    Hybrid {
        strategies: Vec<DistributionStrategy>,
    },
}

/// Fault tolerance configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaultToleranceConfig {
    /// Failure detection timeout
    pub failure_detection_timeout_ms: u64,

    /// Maximum retry attempts
    pub max_retry_attempts: usize,

    /// Recovery strategy
    pub recovery_strategy: RecoveryStrategyConfig,

    /// Enable automatic recovery
    pub auto_recovery: bool,
}

/// Recovery strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RecoveryStrategyConfig {
    /// Re-execute failed tasks on other nodes
    Reexecution,

    /// Use cached results if available
    Caching,

    /// Degraded mode with partial results
    DegradedMode,

    /// Hybrid recovery approach
    Hybrid,
}

/// Load balancing configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancingConfig {
    /// Load balancing algorithm
    pub algorithm: LoadBalancingAlgorithm,

    /// Load threshold for rebalancing
    pub load_threshold: f64,

    /// Rebalancing interval
    pub rebalancing_interval_seconds: u64,

    /// Enable predictive load balancing
    pub predictive: bool,
}

/// Available load balancing algorithms
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    /// Round-robin assignment
    RoundRobin,

    /// Least connections algorithm
    LeastConnections,

    /// Weighted round-robin
    WeightedRoundRobin,

    /// CPU usage based
    CpuBased,

    /// Memory usage based
    MemoryBased,

    /// Custom load metric
    Custom { metric: String },
}

/// Default configuration for distributed processing
impl Default for DistributedConfig {
    fn default() -> Self {
        Self {
            node_config: NodeConfig::default(),
            cluster_config: ClusterConfig::default(),
            query_config: QueryDistributionConfig::default(),
            fault_tolerance_config: FaultToleranceConfig::default(),
            load_balancing_config: LoadBalancingConfig::default(),
        }
    }
}

/// Default node configuration
impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            node_id: Uuid::new_v4(),
            address: "127.0.0.1:8080"
                .parse()
                .expect("Failed to parse socket address for cluster node"),
            capabilities: NodeCapabilities::default(),
            settings: NodeSettings::default(),
        }
    }
}

/// Default node capabilities
impl Default for NodeCapabilities {
    fn default() -> Self {
        Self {
            cpu_cores: num_cpus::get(),
            memory_mb: 8192, // 8GB default
            storage_gb: 100,
            network_bandwidth_mbps: 1000, // 1Gbps
            reasoning_features: vec![
                "SROIQ".to_string(),
                "SROIQV".to_string(),
                "DL-queries".to_string(),
                "SWRL".to_string(),
            ],
        }
    }
}

/// Default node settings
impl Default for NodeSettings {
    fn default() -> Self {
        Self {
            max_concurrent_queries: 10,
            query_timeout_seconds: 300, // 5 minutes
            health_check_interval_seconds: 30,
            enable_monitoring: true,
        }
    }
}

/// Default cluster configuration
impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            cluster_name: "oxidowl-cluster".to_string(),
            discovery: DiscoveryConfig::default(),
            communication: CommunicationConfig::default(),
            consensus: ConsensusConfig::default(),
        }
    }
}

/// Default discovery configuration
impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            method: DiscoveryMethod::Multicast {
                address: "224.0.0.1".to_string(),
                port: 8090,
            },
            timeout_seconds: 30,
            retry_attempts: 3,
            parameters: HashMap::new(),
        }
    }
}

/// Default communication configuration
impl Default for CommunicationConfig {
    fn default() -> Self {
        Self {
            protocol: CommunicationProtocol::Tcp,
            connection_timeout_seconds: 10,
            keep_alive: true,
            compression: CompressionConfig::default(),
        }
    }
}

/// Default compression configuration
impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithm: CompressionAlgorithm::Gzip,
            level: 6,
        }
    }
}

/// Default consensus configuration
impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            algorithm: ConsensusAlgorithm::Raft,
            election_timeout_ms: 5000,
            heartbeat_interval_ms: 1000,
            min_cluster_size: 3,
        }
    }
}

/// Default query distribution configuration
impl Default for QueryDistributionConfig {
    fn default() -> Self {
        Self {
            strategy: DistributionStrategy::LoadAware,
            min_partition_size: 100,
            max_partitions: 10,
            load_balancing: LoadBalancingConfig::default(),
        }
    }
}

/// Default fault tolerance configuration
impl Default for FaultToleranceConfig {
    fn default() -> Self {
        Self {
            failure_detection_timeout_ms: 5000,
            max_retry_attempts: 3,
            recovery_strategy: RecoveryStrategyConfig::Hybrid,
            auto_recovery: true,
        }
    }
}

/// Default load balancing configuration
impl Default for LoadBalancingConfig {
    fn default() -> Self {
        Self {
            algorithm: LoadBalancingAlgorithm::WeightedRoundRobin,
            load_threshold: 0.8, // 80% load threshold
            rebalancing_interval_seconds: 60,
            predictive: true,
        }
    }
}

/// Main distributed query processing service
pub struct DistributedQueryProcessor {
    /// Configuration for the distributed system
    config: DistributedConfig,

    /// Cluster management component
    cluster_manager: Arc<RwLock<ClusterManager>>,

    /// Query distribution component
    query_distributor: Arc<RwLock<QueryDistributor>>,

    /// Result aggregation component
    result_aggregator: Arc<RwLock<ResultAggregator>>,

    /// Fault tolerance component
    fault_tolerance: Arc<RwLock<FaultTolerance>>,

    /// Load balancing component
    load_balancer: Arc<RwLock<LoadBalancer>>,

    /// Cluster coordination component
    coordinator: Arc<RwLock<ClusterCoordinator>>,
}

impl DistributedQueryProcessor {
    /// Create a new distributed query processor
    pub async fn new(config: DistributedConfig) -> Result<Self> {
        // Initialize cluster manager
        let cluster_manager = Arc::new(RwLock::new(
            ClusterManager::new(config.cluster_config.clone()).await?,
        ));

        // Initialize query distributor
        let query_distributor = Arc::new(RwLock::new(
            QueryDistributor::new(config.query_config.clone()).await?,
        ));

        // Initialize result aggregator
        let result_aggregator = Arc::new(RwLock::new(ResultAggregator::new().await?));

        // Initialize fault tolerance
        let fault_tolerance = Arc::new(RwLock::new(
            FaultTolerance::new(config.fault_tolerance_config.clone()).await?,
        ));

        // Initialize load balancer
        let load_balancer = Arc::new(RwLock::new(
            LoadBalancer::new(config.load_balancing_config.clone()).await?,
        ));

        // Initialize cluster coordinator
        let coordinator = Arc::new(RwLock::new(
            ClusterCoordinator::new(config.cluster_config.clone()).await?,
        ));

        Ok(Self {
            config,
            cluster_manager,
            query_distributor,
            result_aggregator,
            fault_tolerance,
            load_balancer,
            coordinator,
        })
    }

    /// Start the distributed query processor
    pub async fn start(&self) -> Result<()> {
        info!("Starting distributed query processor...");

        // Start cluster manager
        let cluster_manager = self.cluster_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = cluster_manager.write().await.start().await {
                error!("Cluster manager failed: {}", e);
            }
        });

        // Start coordinator
        let coordinator = self.coordinator.clone();
        tokio::spawn(async move {
            if let Err(e) = coordinator.write().await.start().await {
                error!("Cluster coordinator failed: {}", e);
            }
        });

        // Start fault tolerance monitoring
        let fault_tolerance = self.fault_tolerance.clone();
        tokio::spawn(async move {
            if let Err(e) = fault_tolerance.write().await.start_monitoring().await {
                error!("Fault tolerance monitoring failed: {}", e);
            }
        });

        // Start load balancer
        let load_balancer = self.load_balancer.clone();
        tokio::spawn(async move {
            if let Err(e) = load_balancer.write().await.start().await {
                error!("Load balancer failed: {}", e);
            }
        });

        info!("Distributed query processor started successfully");
        Ok(())
    }

    /// Execute a distributed query
    pub async fn execute_distributed_query(
        &self,
        query: ConjunctiveQuery,
    ) -> Result<AggregatedResult> {
        info!(
            "Executing distributed query with {} atoms",
            query.body_atoms.len()
        );

        // Distribute the query across available nodes
        let distributed_query = {
            let distributor = self.query_distributor.read().await;
            let cluster = self.cluster_manager.read().await;
            distributor.distribute_query(&query, &cluster).await?
        };

        // Execute query partitions in parallel
        let partial_results = self
            .execute_query_partitions(distributed_query.partitions)
            .await?;

        // Aggregate results
        let aggregated_result = {
            let aggregator = self.result_aggregator.read().await;
            aggregator.aggregate_results(partial_results).await?
        };

        info!("Distributed query execution completed successfully");
        Ok(aggregated_result)
    }

    /// Execute query partitions on different nodes
    async fn execute_query_partitions(
        &self,
        partitions: Vec<QueryPartition>,
    ) -> Result<Vec<PartialResult>> {
        let mut tasks = Vec::new();

        for partition in partitions {
            let cluster_manager = self.cluster_manager.clone();
            let fault_tolerance = self.fault_tolerance.clone();

            let task = tokio::spawn(async move {
                // Execute partition with fault tolerance
                let ft = fault_tolerance.read().await;
                ft.execute_with_retry(&partition, &cluster_manager).await
            });

            tasks.push(task);
        }

        // Wait for all tasks to complete
        let mut results = Vec::new();
        for task in tasks {
            match task.await {
                Ok(Ok(result)) => results.push(result),
                Ok(Err(e)) => return Err(e),
                Err(e) => {
                    return Err(Error::Internal {
                        message: format!("Task execution failed: {}", e),
                    });
                }
            }
        }

        Ok(results)
    }

    /// Get cluster status information
    pub async fn get_cluster_status(&self) -> Result<ClusterState> {
        let cluster_manager = self.cluster_manager.read().await;
        cluster_manager.get_cluster_state().await
    }

    /// Add a new node to the cluster
    pub async fn add_node(&self, node_info: NodeInfo) -> Result<()> {
        let mut cluster_manager = self.cluster_manager.write().await;
        cluster_manager.add_node(node_info).await
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: NodeId) -> Result<()> {
        let mut cluster_manager = self.cluster_manager.write().await;
        cluster_manager.remove_node(node_id).await
    }

    /// Stop the distributed query processor
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping distributed query processor...");

        // Stop all components
        {
            let mut cluster_manager = self.cluster_manager.write().await;
            cluster_manager.stop().await?;
        }

        {
            let mut coordinator = self.coordinator.write().await;
            coordinator.stop().await?;
        }

        {
            let fault_tolerance = self.fault_tolerance.write().await;
            fault_tolerance.stop().await?;
        }

        {
            let mut load_balancer = self.load_balancer.write().await;
            load_balancer.stop().await?;
        }

        info!("Distributed query processor stopped successfully");
        Ok(())
    }
}

/// Error types for distributed processing
#[derive(Debug, thiserror::Error)]
pub enum DistributedError {
    #[error("Cluster error: {0}")]
    Cluster(String),

    #[error("Node communication error: {0}")]
    Communication(String),

    #[error("Query distribution error: {0}")]
    Distribution(String),

    #[error("Result aggregation error: {0}")]
    Aggregation(String),

    #[error("Fault tolerance error: {0}")]
    FaultTolerance(String),

    #[error("Load balancing error: {0}")]
    LoadBalancing(String),

    #[error("Coordination error: {0}")]
    Coordination(String),

    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Convert distributed errors to the main Error type
impl From<DistributedError> for Error {
    fn from(err: DistributedError) -> Self {
        Error::Internal {
            message: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DistributedConfig::default();
        assert_eq!(config.cluster_config.cluster_name, "oxidowl-cluster");
        assert_eq!(config.node_config.capabilities.cpu_cores, num_cpus::get());
        assert!(config.fault_tolerance_config.auto_recovery);
    }

    #[test]
    fn test_node_capabilities() {
        let capabilities = NodeCapabilities::default();
        assert!(
            capabilities
                .reasoning_features
                .contains(&"SROIQ".to_string())
        );
        assert!(capabilities.memory_mb > 0);
        assert!(capabilities.cpu_cores > 0);
    }

    #[tokio::test]
    async fn test_distributed_processor_creation() {
        let config = DistributedConfig::default();
        let processor = DistributedQueryProcessor::new(config).await;
        assert!(processor.is_ok());
    }
}

//! Cluster Management Module
//!
//! Provides comprehensive cluster lifecycle management including node discovery,
//! health monitoring, and dynamic scaling capabilities.

use crate::distributed::{DistributedError, NodeAddress, NodeId};
use crate::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, Instant, interval, timeout};
use uuid::Uuid;

/// Information about a cluster node
#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: NodeId,

    /// Node network address
    pub address: NodeAddress,

    /// Node capabilities and resources
    pub capabilities: crate::distributed::NodeCapabilities,

    /// Current node status
    pub status: NodeStatus,

    /// Node metadata
    pub metadata: HashMap<String, String>,

    /// Last seen timestamp
    #[serde(skip)]
    pub last_seen: Instant,

    /// Node version information
    pub version: String,
}

/// Node status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeStatus {
    /// Node is active and healthy
    Active,

    /// Node is starting up
    Starting,

    /// Node is shutting down gracefully
    Stopping,

    /// Node is temporarily unavailable
    Unavailable,

    /// Node has failed and is not responding
    Failed,

    /// Node is in maintenance mode
    Maintenance,
}

/// Node health information
#[derive(Debug, Clone, Serialize)]
pub struct NodeHealth {
    /// Node identifier
    pub node_id: NodeId,

    /// Health status
    pub status: HealthStatus,

    /// CPU usage percentage (0-100)
    pub cpu_usage: f32,

    /// Memory usage percentage (0-100)
    pub memory_usage: f32,

    /// Network latency in milliseconds
    pub network_latency_ms: f64,

    /// Active query count
    pub active_queries: usize,

    /// Last health check timestamp
    #[serde(skip)]
    pub last_check: Instant,

    /// Health metrics history
    pub metrics_history: Vec<HealthMetric>,
}

/// Health status enumeration
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Node is healthy and operating normally
    Healthy,

    /// Node is experiencing some issues but still functional
    Degraded,

    /// Node is unhealthy and should not receive new work
    Unhealthy,

    /// Node health is unknown (no recent health checks)
    Unknown,
}

/// Historical health metric
#[derive(Debug, Clone, Serialize)]
pub struct HealthMetric {
    /// Timestamp of the metric
    #[serde(skip)]
    pub timestamp: Instant,

    /// Metric type
    pub metric_type: HealthMetricType,

    /// Metric value
    pub value: f64,
}

/// Types of health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthMetricType {
    CpuUsage,
    MemoryUsage,
    NetworkLatency,
    QueryThroughput,
    ErrorRate,
    ResponseTime,
}

/// Overall cluster state
#[derive(Debug, Clone, Serialize)]
pub struct ClusterState {
    /// Cluster identifier
    pub cluster_id: String,

    /// All nodes in the cluster
    pub nodes: HashMap<NodeId, NodeInfo>,

    /// Current cluster topology
    pub topology: ClusterTopology,

    /// Cluster health summary
    pub health: ClusterHealth,

    /// Cluster statistics
    pub statistics: ClusterStatistics,

    /// Last update timestamp
    #[serde(skip)]
    pub last_updated: Instant,
}

/// Cluster topology information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTopology {
    /// Leader node (if using leader-follower pattern)
    pub leader: Option<NodeId>,

    /// Node groups by role or region
    pub groups: HashMap<String, Vec<NodeId>>,

    /// Network partitions (if any)
    pub partitions: Vec<Vec<NodeId>>,

    /// Connection graph between nodes
    pub connections: HashMap<NodeId, Vec<NodeId>>,
}

/// Cluster-wide health information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealth {
    /// Overall cluster status
    pub status: ClusterHealthStatus,

    /// Number of healthy nodes
    pub healthy_nodes: usize,

    /// Number of degraded nodes
    pub degraded_nodes: usize,

    /// Number of unhealthy nodes
    pub unhealthy_nodes: usize,

    /// Total number of nodes
    pub total_nodes: usize,

    /// Cluster capacity utilization
    pub capacity_utilization: f32,

    /// Average response time across cluster
    pub average_response_time_ms: f64,
}

/// Cluster health status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusterHealthStatus {
    /// All nodes healthy, full capacity available
    Healthy,

    /// Some nodes degraded but cluster operational
    Degraded,

    /// Significant issues, reduced capacity
    Unhealthy,

    /// Critical issues, minimal capacity
    Critical,

    /// Cluster is down or unreachable
    Down,
}

/// Cluster statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatistics {
    /// Total queries processed
    pub total_queries: u64,

    /// Queries processed per second
    pub queries_per_second: f64,

    /// Average query response time
    pub avg_response_time_ms: f64,

    /// Total data transferred
    pub data_transferred_bytes: u64,

    /// Uptime since cluster formation
    pub uptime_seconds: u64,

    /// Node join/leave events
    pub node_events: u64,
}

// ─── Actor message type ─────────────────────────────────────────────────────

enum ClusterQueryMsg {
    GetClusterState {
        tx: oneshot::Sender<ClusterState>,
    },
    GetNodeInfo {
        node_id: NodeId,
        tx: oneshot::Sender<Option<NodeInfo>>,
    },
    GetActiveNodes {
        tx: oneshot::Sender<Vec<NodeInfo>>,
    },
    GetClusterHealth {
        tx: oneshot::Sender<ClusterHealth>,
    },
}

/// Main cluster manager implementation — actor handle
#[derive(Clone)]
pub struct ClusterManager {
    /// Event channel for cluster events
    event_sender: mpsc::UnboundedSender<ClusterEvent>,
    /// State query channel
    query_tx: mpsc::Sender<ClusterQueryMsg>,
    /// Shutdown signal
    shutdown_tx: mpsc::Sender<()>,
}

/// Cluster events
#[derive(Debug, Clone)]
pub enum ClusterEvent {
    /// Node joined the cluster
    NodeJoined(NodeInfo),

    /// Node left the cluster
    NodeLeft(NodeId),

    /// Node health changed
    NodeHealthChanged(NodeId, HealthStatus),

    /// Leader elected
    LeaderElected(NodeId),

    /// Network partition detected
    NetworkPartition(Vec<NodeId>),

    /// Cluster topology changed
    TopologyChanged,
}

impl ClusterManager {
    /// Create a new cluster manager — spawns the actor task immediately
    pub async fn new(config: crate::distributed::ClusterConfig) -> Result<Self> {
        let cluster_state = ClusterState {
            cluster_id: config.cluster_name.clone(),
            nodes: HashMap::new(),
            topology: ClusterTopology {
                leader: None,
                groups: HashMap::new(),
                partitions: Vec::new(),
                connections: HashMap::new(),
            },
            health: ClusterHealth {
                status: ClusterHealthStatus::Healthy,
                healthy_nodes: 0,
                degraded_nodes: 0,
                unhealthy_nodes: 0,
                total_nodes: 0,
                capacity_utilization: 0.0,
                average_response_time_ms: 0.0,
            },
            statistics: ClusterStatistics {
                total_queries: 0,
                queries_per_second: 0.0,
                avg_response_time_ms: 0.0,
                data_transferred_bytes: 0,
                uptime_seconds: 0,
                node_events: 0,
            },
            last_updated: Instant::now(),
        };

        let (event_sender, mut event_receiver) = mpsc::unbounded_channel();
        let (query_tx, mut query_rx) = mpsc::channel::<ClusterQueryMsg>(64);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);

        let health_monitor = HealthMonitor::new(config.clone()).await?;
        let discovery_service = DiscoveryService::new(config.clone()).await?;
        let event_sender_clone = event_sender.clone();

        tokio::spawn(async move {
            let mut state = cluster_state;
            let mut health_monitor = health_monitor;
            let mut discovery_service = discovery_service;
            let mut health_interval = interval(Duration::from_secs(30));
            let mut discovery_interval = interval(Duration::from_secs(1 * 60));
            // Consume the immediate first ticks so monitoring starts after the first period
            health_interval.tick().await;
            discovery_interval.tick().await;

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Cluster manager shutdown signal received");
                        if let Err(e) = health_monitor.stop().await {
                            error!("Health monitor stop error: {e}");
                        }
                        if let Err(e) = discovery_service.stop().await {
                            error!("Discovery service stop error: {e}");
                        }
                        break;
                    }
                    Some(event) = event_receiver.recv() => {
                        if let Err(e) = Self::process_cluster_event(&mut state, event).await {
                            error!("Error processing cluster event: {e}");
                        }
                    }
                    Some(msg) = query_rx.recv() => {
                        Self::handle_cluster_query(&state, msg);
                    }
                    _ = health_interval.tick() => {
                        if let Err(e) = HealthMonitor::perform_health_checks(&state, &event_sender_clone).await {
                            error!("Health check failed: {e}");
                        }
                    }
                    _ = discovery_interval.tick() => {
                        if let Err(e) = DiscoveryService::discover_nodes(&config, &event_sender_clone).await {
                            error!("Node discovery failed: {e}");
                        }
                    }
                }
            }
        });

        Ok(Self {
            event_sender,
            query_tx,
            shutdown_tx,
        })
    }

    /// Handle a state query from the query channel
    fn handle_cluster_query(state: &ClusterState, msg: ClusterQueryMsg) {
        match msg {
            ClusterQueryMsg::GetClusterState { tx } => {
                let _ = tx.send(state.clone());
            }
            ClusterQueryMsg::GetNodeInfo { node_id, tx } => {
                let _ = tx.send(state.nodes.get(&node_id).cloned());
            }
            ClusterQueryMsg::GetActiveNodes { tx } => {
                let active: Vec<NodeInfo> = state
                    .nodes
                    .values()
                    .filter(|n| n.status == NodeStatus::Active)
                    .cloned()
                    .collect();
                let _ = tx.send(active);
            }
            ClusterQueryMsg::GetClusterHealth { tx } => {
                let _ = tx.send(state.health.clone());
            }
        }
    }

    /// Start cluster manager services (no-op — actor starts in `new`)
    pub async fn start(&self) -> Result<()> {
        info!("Cluster manager started");
        Ok(())
    }

    /// Process cluster events
    async fn process_cluster_event(
        cluster_state: &mut ClusterState,
        event: ClusterEvent,
    ) -> Result<()> {
        match event {
            ClusterEvent::NodeJoined(node_info) => {
                info!("Node joined cluster: {}", node_info.id);
                cluster_state.nodes.insert(node_info.id, node_info);
                cluster_state.statistics.node_events += 1;
            }

            ClusterEvent::NodeLeft(node_id) => {
                info!("Node left cluster: {node_id}");
                cluster_state.nodes.remove(&node_id);
                cluster_state.statistics.node_events += 1;

                // Update topology
                if cluster_state.topology.leader == Some(node_id) {
                    cluster_state.topology.leader = None;
                    // Trigger leader election if needed
                }
            }

            ClusterEvent::NodeHealthChanged(node_id, health_status) => {
                if let Some(node) = cluster_state.nodes.get_mut(&node_id) {
                    match health_status {
                        HealthStatus::Healthy => node.status = NodeStatus::Active,
                        HealthStatus::Degraded => node.status = NodeStatus::Unavailable,
                        HealthStatus::Unhealthy => node.status = NodeStatus::Failed,
                        HealthStatus::Unknown => node.status = NodeStatus::Unavailable,
                    }
                    debug!("Node {node_id} health changed to {health_status:?}");
                }
            }

            ClusterEvent::LeaderElected(node_id) => {
                info!("New leader elected: {node_id}");
                cluster_state.topology.leader = Some(node_id);
            }

            ClusterEvent::NetworkPartition(partition) => {
                warn!("Network partition detected: {partition:?}");
                cluster_state.topology.partitions.push(partition);
            }

            ClusterEvent::TopologyChanged => {
                info!("Cluster topology changed");
                // Recalculate cluster health and statistics
                Self::update_cluster_health(cluster_state);
            }
        }

        cluster_state.last_updated = Instant::now();
        Ok(())
    }

    /// Update cluster health metrics
    pub(crate) fn update_cluster_health(state: &mut ClusterState) {
        let mut healthy = 0;
        let mut degraded = 0;
        let mut unhealthy = 0;

        for node in state.nodes.values() {
            match node.status {
                NodeStatus::Active => healthy += 1,
                NodeStatus::Unavailable => degraded += 1,
                NodeStatus::Failed => unhealthy += 1,
                _ => {} // Starting, stopping, maintenance don't count in health
            }
        }

        state.health.healthy_nodes = healthy;
        state.health.degraded_nodes = degraded;
        state.health.unhealthy_nodes = unhealthy;
        state.health.total_nodes = state.nodes.len();

        // Determine overall cluster health
        let total_operational = healthy + degraded;
        let health_ratio = if state.health.total_nodes > 0 {
            total_operational as f32 / state.health.total_nodes as f32
        } else {
            0.0
        };

        state.health.status = match health_ratio {
            r if r >= 0.9 => ClusterHealthStatus::Healthy,
            r if r >= 0.7 => ClusterHealthStatus::Degraded,
            r if r >= 0.5 => ClusterHealthStatus::Unhealthy,
            r if r > 0.0 => ClusterHealthStatus::Critical,
            _ => ClusterHealthStatus::Down,
        };

        // Calculate capacity utilization
        let total_capacity: usize = state
            .nodes
            .values()
            .filter(|n| matches!(n.status, NodeStatus::Active | NodeStatus::Unavailable))
            .map(|n| n.capabilities.cpu_cores)
            .sum();

        let used_capacity: usize = state
            .nodes
            .values()
            .filter(|n| n.status == NodeStatus::Active)
            .map(|n| n.capabilities.cpu_cores)
            .sum();

        state.health.capacity_utilization = if total_capacity > 0 {
            used_capacity as f32 / total_capacity as f32
        } else {
            0.0
        };
    }

    /// Add a node to the cluster
    pub async fn add_node(&self, node_info: NodeInfo) -> Result<()> {
        let event = ClusterEvent::NodeJoined(node_info);
        self.event_sender.send(event).map_err(|e| {
            DistributedError::Cluster(format!("Failed to send node join event: {e}"))
        })?;
        Ok(())
    }

    /// Remove a node from the cluster
    pub async fn remove_node(&self, node_id: NodeId) -> Result<()> {
        let event = ClusterEvent::NodeLeft(node_id);
        self.event_sender.send(event).map_err(|e| {
            DistributedError::Cluster(format!("Failed to send node leave event: {e}"))
        })?;
        Ok(())
    }

    /// Get current cluster state
    pub async fn get_cluster_state(&self) -> Result<ClusterState> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.query_tx
            .send(ClusterQueryMsg::GetClusterState { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "ClusterManager actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "ClusterManager did not respond".to_string(),
        })
    }

    /// Get node information by ID
    pub async fn get_node_info(&self, node_id: NodeId) -> Result<Option<NodeInfo>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.query_tx
            .send(ClusterQueryMsg::GetNodeInfo {
                node_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "ClusterManager actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "ClusterManager did not respond".to_string(),
        })
    }

    /// Get all active nodes
    pub async fn get_active_nodes(&self) -> Result<Vec<NodeInfo>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.query_tx
            .send(ClusterQueryMsg::GetActiveNodes { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "ClusterManager actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "ClusterManager did not respond".to_string(),
        })
    }

    /// Get cluster health information
    pub async fn get_cluster_health(&self) -> Result<ClusterHealth> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.query_tx
            .send(ClusterQueryMsg::GetClusterHealth { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "ClusterManager actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "ClusterManager did not respond".to_string(),
        })
    }

    /// Stop cluster manager
    pub async fn stop(&self) -> Result<()> {
        info!("Stopping cluster manager...");
        let _ = self.shutdown_tx.send(()).await;
        info!("Cluster manager stopped successfully");
        Ok(())
    }
}

/// Health monitoring service
pub struct HealthMonitor {
    #[allow(dead_code)]
    config: crate::distributed::ClusterConfig,
    monitoring_tasks: HashMap<NodeId, tokio::task::JoinHandle<()>>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub async fn new(config: crate::distributed::ClusterConfig) -> Result<Self> {
        Ok(Self {
            config,
            monitoring_tasks: HashMap::new(),
            shutdown_tx: None,
        })
    }

    /// Start health monitoring (no-op — managed by ClusterManager actor)
    pub async fn start(
        &mut self,
        _event_sender: mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        info!("Starting cluster health monitoring");
        Ok(())
    }

    /// Perform health checks on all nodes
    pub(crate) async fn perform_health_checks(
        state: &ClusterState,
        event_sender: &mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        let nodes = state.nodes.clone();

        for (node_id, node_info) in nodes {
            // Skip health check for failed nodes
            if node_info.status == NodeStatus::Failed {
                continue;
            }

            // Perform health check
            let health_status = Self::check_node_health(&node_info).await?;

            // Send health change event if status changed
            let current_health = match node_info.status {
                NodeStatus::Active => HealthStatus::Healthy,
                NodeStatus::Unavailable => HealthStatus::Degraded,
                NodeStatus::Failed => HealthStatus::Unhealthy,
                _ => HealthStatus::Unknown,
            };

            if health_status != current_health {
                let event = ClusterEvent::NodeHealthChanged(node_id, health_status);
                let _ = event_sender.send(event);
            }
        }

        Ok(())
    }

    /// Check individual node health
    async fn check_node_health(node_info: &NodeInfo) -> Result<HealthStatus> {
        // Simple ping-based health check
        // In a real implementation, this would include:
        // - Network connectivity test
        // - Resource utilization check
        // - Service responsiveness test

        let health_check_timeout = Duration::from_secs(5);

        match timeout(health_check_timeout, Self::ping_node(&node_info.address)).await {
            Ok(Ok(())) => {
                // Additional checks could go here
                Ok(HealthStatus::Healthy)
            }
            Ok(Err(_)) => Ok(HealthStatus::Degraded),
            Err(_) => Ok(HealthStatus::Unhealthy), // Timeout
        }
    }

    /// Ping a node to check basic connectivity
    async fn ping_node(address: &NodeAddress) -> Result<()> {
        // Simplified ping implementation
        // In practice, this would be a proper health check endpoint
        match tokio::net::TcpStream::connect(address).await {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::Network {
                message: format!("Node ping failed: {e}"),
            }),
        }
    }

    /// Stop health monitoring
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }

        // Wait for monitoring tasks to complete
        for (_, task) in self.monitoring_tasks.drain() {
            task.abort();
        }

        Ok(())
    }
}

/// Node discovery service
pub struct DiscoveryService {
    config: crate::distributed::ClusterConfig,
    #[allow(dead_code)]
    discovered_nodes: HashMap<NodeId, NodeInfo>,
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub async fn new(config: crate::distributed::ClusterConfig) -> Result<Self> {
        Ok(Self {
            config,
            discovered_nodes: HashMap::new(),
            shutdown_tx: None,
        })
    }

    /// Start node discovery
    pub async fn start(&mut self, event_sender: mpsc::UnboundedSender<ClusterEvent>) -> Result<()> {
        info!("Starting node discovery service");

        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        let config = self.config.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(1 * 60)); // Discovery every minute

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if let Err(e) = Self::discover_nodes(&config, &event_sender).await {
                            error!("Node discovery failed: {e}");
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Discovery service shutdown");
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Discover nodes in the cluster
    async fn discover_nodes(
        config: &crate::distributed::ClusterConfig,
        event_sender: &mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        match &config.discovery.method {
            crate::distributed::DiscoveryMethod::Multicast { address, port } => {
                Self::discover_multicast(address, *port, event_sender).await
            }
            crate::distributed::DiscoveryMethod::Static { nodes } => {
                Self::discover_static(nodes, event_sender).await
            }
            crate::distributed::DiscoveryMethod::External { service_url } => {
                Self::discover_external(service_url, event_sender).await
            }
            crate::distributed::DiscoveryMethod::Consul { consul_address } => {
                Self::discover_consul(consul_address, event_sender).await
            }
        }
    }

    /// Discover nodes via multicast
    async fn discover_multicast(
        _address: &str,
        _port: u16,
        _event_sender: &mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        // Multicast discovery implementation would go here
        // For now, this is a placeholder
        debug!("Multicast discovery not yet implemented");
        Ok(())
    }

    /// Discover nodes from static configuration
    async fn discover_static(
        nodes: &[NodeAddress],
        event_sender: &mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        for address in nodes {
            // Try to connect to each static node
            match tokio::net::TcpStream::connect(address).await {
                Ok(_) => {
                    let node_info = NodeInfo {
                        id: Uuid::new_v4(), // Generate ID for static node
                        address: *address,
                        capabilities: crate::distributed::NodeCapabilities::default(),
                        status: NodeStatus::Active,
                        metadata: HashMap::new(),
                        last_seen: Instant::now(),
                        version: "1.0.0".to_string(),
                    };

                    let event = ClusterEvent::NodeJoined(node_info);
                    let _ = event_sender.send(event);
                }
                Err(e) => {
                    debug!("Failed to connect to static node {address}: {e}");
                }
            }
        }
        Ok(())
    }

    /// Discover nodes via external service
    async fn discover_external(
        _service_url: &str,
        _event_sender: &mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        // External service discovery implementation would go here
        debug!("External discovery not yet implemented");
        Ok(())
    }

    /// Discover nodes via Consul
    async fn discover_consul(
        _consul_address: &str,
        _event_sender: &mpsc::UnboundedSender<ClusterEvent>,
    ) -> Result<()> {
        // Consul discovery implementation would go here
        debug!("Consul discovery not yet implemented");
        Ok(())
    }

    /// Stop discovery service
    pub async fn stop(&mut self) -> Result<()> {
        if let Some(shutdown_tx) = self.shutdown_tx.take() {
            let _ = shutdown_tx.send(()).await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cluster_manager_creation() {
        let config = crate::distributed::ClusterConfig::default();
        let manager = ClusterManager::new(config).await;
        assert!(manager.is_ok());
    }

    #[test]
    fn test_cluster_health_calculation() {
        let mut state = ClusterState {
            cluster_id: "test".to_string(),
            nodes: HashMap::new(),
            topology: ClusterTopology {
                leader: None,
                groups: HashMap::new(),
                partitions: Vec::new(),
                connections: HashMap::new(),
            },
            health: ClusterHealth {
                status: ClusterHealthStatus::Healthy,
                healthy_nodes: 0,
                degraded_nodes: 0,
                unhealthy_nodes: 0,
                total_nodes: 0,
                capacity_utilization: 0.0,
                average_response_time_ms: 0.0,
            },
            statistics: ClusterStatistics {
                total_queries: 0,
                queries_per_second: 0.0,
                avg_response_time_ms: 0.0,
                data_transferred_bytes: 0,
                uptime_seconds: 0,
                node_events: 0,
            },
            last_updated: Instant::now(),
        };

        // Add some test nodes
        for i in 0..5 {
            let node_info = NodeInfo {
                id: Uuid::new_v4(),
                address: format!("127.0.0.1:808{}", i)
                    .parse()
                    .expect("Failed to parse socket address for cluster node"),
                capabilities: crate::distributed::NodeCapabilities::default(),
                status: if i < 4 {
                    NodeStatus::Active
                } else {
                    NodeStatus::Failed
                },
                metadata: HashMap::new(),
                last_seen: Instant::now(),
                version: "1.0.0".to_string(),
            };
            state.nodes.insert(node_info.id, node_info);
        }

        ClusterManager::update_cluster_health(&mut state);

        assert_eq!(state.health.healthy_nodes, 4);
        assert_eq!(state.health.unhealthy_nodes, 1);
        assert_eq!(state.health.total_nodes, 5);
        // With 4/5 nodes active (80%), the cluster is Degraded (requires 90% for Healthy)
        assert_eq!(state.health.status, ClusterHealthStatus::Degraded);
    }
}

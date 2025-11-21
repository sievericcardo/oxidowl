//! Load Balancing Module for Distributed Query Processing
//!
//! This module implements load balancing strategies for distributing queries
//! across cluster nodes based on their current load and capabilities.

use super::{LoadBalancingAlgorithm, LoadBalancingConfig, NodeId};
use crate::prelude::*;
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Load balancer for distributing work across cluster nodes
pub struct LoadBalancer {
    config: LoadBalancingConfig,
    node_loads: RwLock<HashMap<NodeId, NodeLoad>>,
}

/// Current load information for a node
#[derive(Debug, Clone)]
pub struct NodeLoad {
    /// Node identifier
    pub node_id: NodeId,

    /// Current CPU usage (0.0 - 1.0)
    pub cpu_usage: f64,

    /// Current memory usage (0.0 - 1.0)
    pub memory_usage: f64,

    /// Number of active queries
    pub active_queries: usize,

    /// Network bandwidth usage (0.0 - 1.0)
    pub network_usage: f64,
}

/// Workload distribution across nodes
#[derive(Debug, Clone)]
pub struct WorkloadDistribution {
    /// Map of node IDs to assigned workload weights
    pub assignments: HashMap<NodeId, f64>,

    /// Load balancing score (0.0 - 1.0, higher is better)
    pub balance_score: f64,
}

impl LoadBalancer {
    /// Create a new load balancer with the given configuration
    pub async fn new(config: LoadBalancingConfig) -> Result<Self> {
        Ok(Self {
            config,
            node_loads: RwLock::new(HashMap::new()),
        })
    }

    /// Start the load balancer
    pub async fn start(&mut self) -> Result<()> {
        info!(
            "Load balancer started with algorithm: {:?}",
            self.config.algorithm
        );
        Ok(())
    }

    /// Stop the load balancer
    pub async fn stop(&mut self) -> Result<()> {
        info!("Load balancer stopped");
        Ok(())
    }

    /// Update load information for a node
    pub async fn update_node_load(&self, node_id: NodeId, load: NodeLoad) -> Result<()> {
        let mut loads = self.node_loads.write().await;
        loads.insert(node_id, load);
        Ok(())
    }

    /// Get current load for a node
    pub async fn get_node_load(&self, node_id: &NodeId) -> Result<Option<NodeLoad>> {
        let loads = self.node_loads.read().await;
        Ok(loads.get(node_id).cloned())
    }

    /// Select best node for new workload based on current loads
    pub async fn select_node(&self, available_nodes: &[NodeId]) -> Result<Option<NodeId>> {
        if available_nodes.is_empty() {
            return Ok(None);
        }

        let loads = self.node_loads.read().await;

        // Select based on algorithm
        let selected = match self.config.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                // Simple round-robin selection
                available_nodes.first().copied()
            }
            LoadBalancingAlgorithm::LeastConnections => {
                // Select node with fewest active queries
                available_nodes
                    .iter()
                    .min_by_key(|id| loads.get(id).map(|l| l.active_queries).unwrap_or(0))
                    .copied()
            }
            LoadBalancingAlgorithm::CpuBased => {
                // Select node with lowest CPU usage
                available_nodes
                    .iter()
                    .min_by(|a, b| {
                        let cpu_a = loads.get(a).map(|l| l.cpu_usage).unwrap_or(1.0);
                        let cpu_b = loads.get(b).map(|l| l.cpu_usage).unwrap_or(1.0);
                        cpu_a
                            .partial_cmp(&cpu_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            }
            LoadBalancingAlgorithm::MemoryBased => {
                // Select node with lowest memory usage
                available_nodes
                    .iter()
                    .min_by(|a, b| {
                        let mem_a = loads.get(a).map(|l| l.memory_usage).unwrap_or(1.0);
                        let mem_b = loads.get(b).map(|l| l.memory_usage).unwrap_or(1.0);
                        mem_a
                            .partial_cmp(&mem_b)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .copied()
            }
            LoadBalancingAlgorithm::WeightedRoundRobin => {
                // Use weighted selection based on combined metrics
                self.select_weighted_node(available_nodes, &loads).await
            }
            LoadBalancingAlgorithm::Custom { .. } => {
                // Fallback to least connections for custom metrics
                available_nodes
                    .iter()
                    .min_by_key(|id| loads.get(id).map(|l| l.active_queries).unwrap_or(0))
                    .copied()
            }
        };

        Ok(selected)
    }

    /// Select node using weighted algorithm
    async fn select_weighted_node(
        &self,
        available_nodes: &[NodeId],
        loads: &HashMap<NodeId, NodeLoad>,
    ) -> Option<NodeId> {
        // Calculate composite load score for each node
        available_nodes
            .iter()
            .min_by(|a, b| {
                let score_a = self.calculate_load_score(loads.get(a));
                let score_b = self.calculate_load_score(loads.get(b));
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    /// Calculate composite load score (lower is better)
    fn calculate_load_score(&self, load: Option<&NodeLoad>) -> f64 {
        match load {
            Some(l) => {
                // Weighted average of CPU, memory, and query count
                let cpu_weight = 0.4;
                let mem_weight = 0.3;
                let query_weight = 0.2;
                let network_weight = 0.1;

                cpu_weight * l.cpu_usage
                    + mem_weight * l.memory_usage
                    + query_weight * (l.active_queries as f64 / 10.0) // Normalize to 0-1
                    + network_weight * l.network_usage
            }
            None => 0.0, // Prefer nodes with no load data (likely available)
        }
    }

    /// Calculate workload distribution across nodes
    pub async fn calculate_distribution(
        &self,
        available_nodes: &[NodeId],
        total_workload: f64,
    ) -> Result<WorkloadDistribution> {
        let loads = self.node_loads.read().await;
        let mut assignments = HashMap::new();

        if available_nodes.is_empty() {
            return Ok(WorkloadDistribution {
                assignments,
                balance_score: 0.0,
            });
        }

        // Calculate total available capacity
        let mut total_capacity = 0.0;
        let mut node_capacities = HashMap::new();

        for node_id in available_nodes {
            let capacity = 1.0 - self.calculate_load_score(loads.get(node_id));
            node_capacities.insert(*node_id, capacity);
            total_capacity += capacity;
        }

        // Distribute workload proportionally to capacity
        for (node_id, capacity) in node_capacities {
            let assignment = if total_capacity > 0.0 {
                (capacity / total_capacity) * total_workload
            } else {
                total_workload / available_nodes.len() as f64
            };
            assignments.insert(node_id, assignment);
        }

        // Calculate balance score (variance from ideal distribution)
        let ideal_load = total_workload / available_nodes.len() as f64;
        let variance: f64 = assignments
            .values()
            .map(|&load| (load - ideal_load).powi(2))
            .sum::<f64>()
            / available_nodes.len() as f64;

        let balance_score = 1.0 - (variance / (ideal_load.powi(2) + 1.0));

        Ok(WorkloadDistribution {
            assignments,
            balance_score,
        })
    }

    /// Check if rebalancing is needed based on current loads
    pub async fn needs_rebalancing(&self, available_nodes: &[NodeId]) -> Result<bool> {
        if available_nodes.len() < 2 {
            return Ok(false);
        }

        let loads = self.node_loads.read().await;

        // Calculate load variance
        let load_scores: Vec<f64> = available_nodes
            .iter()
            .map(|id| self.calculate_load_score(loads.get(id)))
            .collect();

        let mean = load_scores.iter().sum::<f64>() / load_scores.len() as f64;
        let variance = load_scores
            .iter()
            .map(|&score| (score - mean).powi(2))
            .sum::<f64>()
            / load_scores.len() as f64;

        // Trigger rebalancing if variance exceeds threshold
        Ok(variance > self.config.load_threshold)
    }
}

impl Default for NodeLoad {
    fn default() -> Self {
        Self {
            node_id: uuid::Uuid::new_v4(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            active_queries: 0,
            network_usage: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_load_balancer_creation() {
        let config = LoadBalancingConfig::default();
        let balancer = LoadBalancer::new(config).await;
        assert!(balancer.is_ok());
    }

    #[tokio::test]
    async fn test_node_selection() {
        let config = LoadBalancingConfig::default();
        let balancer = LoadBalancer::new(config).await.expect("Failed to create load balancer with given configuration");

        let node_id = uuid::Uuid::new_v4();
        let nodes = vec![node_id];

        let selected = balancer.select_node(&nodes).await.expect("Failed to select node using load balancer");
        assert_eq!(selected, Some(node_id));
    }

    #[tokio::test]
    async fn test_load_score_calculation() {
        let config = LoadBalancingConfig::default();
        let balancer = LoadBalancer::new(config).await.expect("Failed to create load balancer with given configuration");

        let low_load = NodeLoad {
            node_id: uuid::Uuid::new_v4(),
            cpu_usage: 0.2,
            memory_usage: 0.3,
            active_queries: 1,
            network_usage: 0.1,
        };

        let high_load = NodeLoad {
            node_id: uuid::Uuid::new_v4(),
            cpu_usage: 0.9,
            memory_usage: 0.8,
            active_queries: 10,
            network_usage: 0.7,
        };

        let low_score = balancer.calculate_load_score(Some(&low_load));
        let high_score = balancer.calculate_load_score(Some(&high_load));

        assert!(low_score < high_score);
    }
}

//! Load Balancing Module for Distributed Query Processing
//!
//! This module implements load balancing strategies for distributing queries
//! across cluster nodes based on their current load and capabilities.

use super::{LoadBalancingAlgorithm, LoadBalancingConfig, NodeId};
use crate::prelude::*;
use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};

// ─── Actor kernel helpers ─────────────────────────────────────────────────

fn lb_load_score(load: Option<&NodeLoad>) -> f64 {
    match load {
        Some(l) => {
            0.4 * l.cpu_usage
                + 0.3 * l.memory_usage
                + 0.2 * (l.active_queries as f64 / 10.0)
                + 0.1 * l.network_usage
        }
        None => 0.0,
    }
}

fn lb_select_node(
    config: &LoadBalancingConfig,
    nodes: &[NodeId],
    loads: &HashMap<NodeId, NodeLoad>,
) -> Option<NodeId> {
    if nodes.is_empty() {
        return None;
    }
    match config.algorithm {
        LoadBalancingAlgorithm::RoundRobin => nodes.first().copied(),
        LoadBalancingAlgorithm::LeastConnections => nodes
            .iter()
            .min_by_key(|id| loads.get(*id).map(|l| l.active_queries).unwrap_or(0))
            .copied(),
        LoadBalancingAlgorithm::CpuBased => nodes
            .iter()
            .min_by(|a, b| {
                let ca = loads.get(*a).map(|l| l.cpu_usage).unwrap_or(1.0);
                let cb = loads.get(*b).map(|l| l.cpu_usage).unwrap_or(1.0);
                ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied(),
        LoadBalancingAlgorithm::MemoryBased => nodes
            .iter()
            .min_by(|a, b| {
                let ma = loads.get(*a).map(|l| l.memory_usage).unwrap_or(1.0);
                let mb = loads.get(*b).map(|l| l.memory_usage).unwrap_or(1.0);
                ma.partial_cmp(&mb).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied(),
        LoadBalancingAlgorithm::WeightedRoundRobin | LoadBalancingAlgorithm::Custom { .. } => nodes
            .iter()
            .min_by(|a, b| {
                lb_load_score(loads.get(*a))
                    .partial_cmp(&lb_load_score(loads.get(*b)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied(),
    }
}

fn lb_calculate_distribution(
    nodes: &[NodeId],
    total_workload: f64,
    loads: &HashMap<NodeId, NodeLoad>,
) -> WorkloadDistribution {
    let mut assignments = HashMap::new();
    if nodes.is_empty() {
        return WorkloadDistribution {
            assignments,
            balance_score: 0.0,
        };
    }
    let mut total_capacity = 0.0_f64;
    let mut node_capacities: HashMap<NodeId, f64> = HashMap::new();
    for &node_id in nodes {
        let capacity = 1.0 - lb_load_score(loads.get(&node_id));
        node_capacities.insert(node_id, capacity);
        total_capacity += capacity;
    }
    for (node_id, capacity) in node_capacities {
        let assignment = if total_capacity > 0.0 {
            (capacity / total_capacity) * total_workload
        } else {
            total_workload / nodes.len() as f64
        };
        assignments.insert(node_id, assignment);
    }
    let ideal_load = total_workload / nodes.len() as f64;
    let variance: f64 = assignments
        .values()
        .map(|&load| (load - ideal_load).powi(2))
        .sum::<f64>()
        / nodes.len() as f64;
    let balance_score = 1.0 - (variance / (ideal_load.powi(2) + 1.0));
    WorkloadDistribution {
        assignments,
        balance_score,
    }
}

fn lb_needs_rebalancing(
    load_threshold: f64,
    nodes: &[NodeId],
    loads: &HashMap<NodeId, NodeLoad>,
) -> bool {
    if nodes.len() < 2 {
        return false;
    }
    let scores: Vec<f64> = nodes
        .iter()
        .map(|id| lb_load_score(loads.get(id)))
        .collect();
    let mean = scores.iter().sum::<f64>() / scores.len() as f64;
    let variance = scores.iter().map(|&s| (s - mean).powi(2)).sum::<f64>() / scores.len() as f64;
    variance > load_threshold
}

// ─── Actor message type ───────────────────────────────────────────────────

enum LoadBalancerMsg {
    UpdateNodeLoad {
        node_id: NodeId,
        load: NodeLoad,
    },
    GetNodeLoad {
        node_id: NodeId,
        tx: oneshot::Sender<Option<NodeLoad>>,
    },
    SelectNode {
        nodes: Vec<NodeId>,
        tx: oneshot::Sender<Option<NodeId>>,
    },
    CalculateDistribution {
        nodes: Vec<NodeId>,
        total_workload: f64,
        tx: oneshot::Sender<WorkloadDistribution>,
    },
    NeedsRebalancing {
        nodes: Vec<NodeId>,
        tx: oneshot::Sender<bool>,
    },
    Shutdown,
}

/// Load balancer for distributing work across cluster nodes — actor handle
#[derive(Clone)]
pub struct LoadBalancer {
    tx: mpsc::Sender<LoadBalancerMsg>,
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
    /// Create a new load balancer — spawns the actor task immediately
    pub async fn new(config: LoadBalancingConfig) -> Result<Self> {
        let (tx, mut rx) = mpsc::channel::<LoadBalancerMsg>(64);
        tokio::spawn(async move {
            let mut node_loads: HashMap<NodeId, NodeLoad> = HashMap::new();
            loop {
                match rx.recv().await {
                    Some(LoadBalancerMsg::UpdateNodeLoad { node_id, load }) => {
                        node_loads.insert(node_id, load);
                    }
                    Some(LoadBalancerMsg::GetNodeLoad { node_id, tx }) => {
                        let _ = tx.send(node_loads.get(&node_id).cloned());
                    }
                    Some(LoadBalancerMsg::SelectNode { nodes, tx }) => {
                        let _ = tx.send(lb_select_node(&config, &nodes, &node_loads));
                    }
                    Some(LoadBalancerMsg::CalculateDistribution {
                        nodes,
                        total_workload,
                        tx,
                    }) => {
                        let _ = tx.send(lb_calculate_distribution(
                            &nodes,
                            total_workload,
                            &node_loads,
                        ));
                    }
                    Some(LoadBalancerMsg::NeedsRebalancing { nodes, tx }) => {
                        let _ = tx.send(lb_needs_rebalancing(
                            config.load_threshold,
                            &nodes,
                            &node_loads,
                        ));
                    }
                    Some(LoadBalancerMsg::Shutdown) | None => break,
                }
            }
        });
        Ok(Self { tx })
    }

    /// Start the load balancer (no-op — actor starts in `new`)
    pub async fn start(&self) -> Result<()> {
        info!("Load balancer started");
        Ok(())
    }

    /// Stop the load balancer
    pub async fn stop(&self) -> Result<()> {
        let _ = self.tx.send(LoadBalancerMsg::Shutdown).await;
        info!("Load balancer stopped");
        Ok(())
    }

    /// Update load information for a node
    pub async fn update_node_load(&self, node_id: NodeId, load: NodeLoad) -> Result<()> {
        let _ = self
            .tx
            .send(LoadBalancerMsg::UpdateNodeLoad { node_id, load })
            .await;
        Ok(())
    }

    /// Get current load for a node
    pub async fn get_node_load(&self, node_id: &NodeId) -> Result<Option<NodeLoad>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(LoadBalancerMsg::GetNodeLoad {
                node_id: *node_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "LoadBalancer actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "LoadBalancer did not respond".to_string(),
        })
    }

    /// Select best node for new workload based on current loads
    pub async fn select_node(&self, available_nodes: &[NodeId]) -> Result<Option<NodeId>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(LoadBalancerMsg::SelectNode {
                nodes: available_nodes.to_vec(),
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "LoadBalancer actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "LoadBalancer did not respond".to_string(),
        })
    }

    /// Calculate workload distribution across nodes
    pub async fn calculate_distribution(
        &self,
        available_nodes: &[NodeId],
        total_workload: f64,
    ) -> Result<WorkloadDistribution> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(LoadBalancerMsg::CalculateDistribution {
                nodes: available_nodes.to_vec(),
                total_workload,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "LoadBalancer actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "LoadBalancer did not respond".to_string(),
        })
    }

    /// Check if rebalancing is needed based on current loads
    pub async fn needs_rebalancing(&self, available_nodes: &[NodeId]) -> Result<bool> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(LoadBalancerMsg::NeedsRebalancing {
                nodes: available_nodes.to_vec(),
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "LoadBalancer actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "LoadBalancer did not respond".to_string(),
        })
    }

    /// Calculate composite load score (lower is better) — kept for external use
    pub fn calculate_load_score(&self, load: Option<&NodeLoad>) -> f64 {
        lb_load_score(load)
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
        let balancer = LoadBalancer::new(config)
            .await
            .expect("Failed to create load balancer with given configuration");

        let node_id = uuid::Uuid::new_v4();
        let nodes = vec![node_id];

        let selected = balancer
            .select_node(&nodes)
            .await
            .expect("Failed to select node using load balancer");
        assert_eq!(selected, Some(node_id));
    }

    #[tokio::test]
    async fn test_load_score_calculation() {
        let config = LoadBalancingConfig::default();
        let balancer = LoadBalancer::new(config)
            .await
            .expect("Failed to create load balancer with given configuration");

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

//! Cluster Coordination Module
//!
//! This module provides coordination primitives for distributed query processing,
//! including distributed locks, consensus protocols, and cluster state management.

use super::{ClusterConfig, ConsensusAlgorithm, NodeId};
use crate::prelude::*;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot};

// ─── Actor message type ──────────────────────────────────────────────────────────

enum CoordinatorMsg {
    AcquireLock {
        lock_id: String,
        node_id: NodeId,
        timeout: Duration,
        tx: oneshot::Sender<Result<bool>>,
    },
    ReleaseLock {
        lock_id: String,
        node_id: NodeId,
        tx: oneshot::Sender<Result<bool>>,
    },
    GetLeader {
        tx: oneshot::Sender<Result<Option<NodeId>>>,
    },
    SetLeader {
        leader_id: Option<NodeId>,
        tx: oneshot::Sender<Result<()>>,
    },
    IsLeader {
        node_id: NodeId,
        tx: oneshot::Sender<Result<bool>>,
    },
    GetConsensusState {
        tx: oneshot::Sender<Result<ConsensusProtocol>>,
    },
    UpdateConsensusState {
        state: ConsensusProtocol,
        tx: oneshot::Sender<Result<()>>,
    },
    AddParticipant {
        node_id: NodeId,
        tx: oneshot::Sender<Result<()>>,
    },
    RemoveParticipant {
        node_id: NodeId,
        tx: oneshot::Sender<Result<()>>,
    },
    HasQuorum {
        tx: oneshot::Sender<Result<bool>>,
    },
    IncrementTerm {
        tx: oneshot::Sender<Result<u64>>,
    },
    Shutdown,
}

/// Cluster coordinator for managing distributed consensus and synchronization — actor handle
#[derive(Clone)]
pub struct ClusterCoordinator {
    tx: mpsc::Sender<CoordinatorMsg>,
}

/// Distributed lock for coordinating access to shared resources
#[derive(Debug, Clone)]
pub struct DistributedLock {
    /// Lock identifier
    pub lock_id: String,

    /// Node holding the lock
    pub holder: Option<NodeId>,

    /// Lock acquisition timestamp
    pub acquired_at: std::time::Instant,

    /// Lock timeout duration
    pub timeout: std::time::Duration,
}

/// Consensus protocol state and operations
#[derive(Debug, Clone)]
pub struct ConsensusProtocol {
    /// Current protocol algorithm
    pub algorithm: ConsensusAlgorithm,

    /// Current leader node (if applicable)
    pub leader: Option<NodeId>,

    /// Current term/epoch number
    pub term: u64,

    /// Participating nodes
    pub participants: Vec<NodeId>,

    /// Quorum size required for decisions
    pub quorum_size: usize,
}

impl ClusterCoordinator {
    /// Create a new cluster coordinator — spawns the actor task immediately
    pub async fn new(config: ClusterConfig) -> Result<Self> {
        let consensus = ConsensusProtocol {
            algorithm: config.consensus.algorithm.clone(),
            leader: None,
            term: 0,
            participants: Vec::new(),
            quorum_size: Self::calculate_quorum_size(&config),
        };

        // Log initial consensus setup
        match config.consensus.algorithm {
            ConsensusAlgorithm::Raft => info!("Starting Raft consensus protocol"),
            ConsensusAlgorithm::LeaderElection => info!("Starting leader election"),
            ConsensusAlgorithm::None => {
                info!("Running in single-node mode, no consensus protocol");
            }
        }

        let (tx, mut rx) = mpsc::channel::<CoordinatorMsg>(64);
        tokio::spawn(async move {
            let mut leader: Option<NodeId> = None;
            let mut locks: HashMap<String, DistributedLock> = HashMap::new();
            let mut consensus = consensus;

            loop {
                match rx.recv().await {
                    Some(CoordinatorMsg::AcquireLock {
                        lock_id,
                        node_id,
                        timeout,
                        tx,
                    }) => {
                        let result = {
                            if let Some(existing) = locks.get(&lock_id) {
                                if let Some(holder) = existing.holder {
                                    if existing.acquired_at.elapsed() < existing.timeout {
                                        Ok(false) // still held
                                    } else {
                                        debug!("Lock {lock_id} expired for holder {holder:?}");
                                        locks.insert(
                                            lock_id.clone(),
                                            DistributedLock {
                                                lock_id: lock_id.clone(),
                                                holder: Some(node_id),
                                                acquired_at: std::time::Instant::now(),
                                                timeout,
                                            },
                                        );
                                        info!("Lock {lock_id} acquired by node {node_id:?}");
                                        Ok(true)
                                    }
                                } else {
                                    locks.insert(
                                        lock_id.clone(),
                                        DistributedLock {
                                            lock_id: lock_id.clone(),
                                            holder: Some(node_id),
                                            acquired_at: std::time::Instant::now(),
                                            timeout,
                                        },
                                    );
                                    info!("Lock {lock_id} acquired by node {node_id:?}");
                                    Ok(true)
                                }
                            } else {
                                locks.insert(
                                    lock_id.clone(),
                                    DistributedLock {
                                        lock_id: lock_id.clone(),
                                        holder: Some(node_id),
                                        acquired_at: std::time::Instant::now(),
                                        timeout,
                                    },
                                );
                                info!("Lock {lock_id} acquired by node {node_id:?}");
                                Ok(true)
                            }
                        };
                        let _ = tx.send(result);
                    }
                    Some(CoordinatorMsg::ReleaseLock {
                        lock_id,
                        node_id,
                        tx,
                    }) => {
                        let result = if let Some(lock) = locks.get(&lock_id) {
                            if lock.holder == Some(node_id) {
                                locks.remove(&lock_id);
                                info!("Lock {lock_id} released by node {node_id:?}");
                                Ok(true)
                            } else {
                                warn!(
                                    "Node {:?} attempted to release lock {} held by {:?}",
                                    node_id, lock_id, lock.holder
                                );
                                Ok(false)
                            }
                        } else {
                            Ok(false)
                        };
                        let _ = tx.send(result);
                    }
                    Some(CoordinatorMsg::GetLeader { tx }) => {
                        let _ = tx.send(Ok(leader));
                    }
                    Some(CoordinatorMsg::SetLeader { leader_id, tx }) => {
                        leader = leader_id;
                        if let Some(id) = leader_id {
                            info!("Leader set to node {id:?}");
                        } else {
                            info!("Leader cleared (no leader)");
                        }
                        let _ = tx.send(Ok(()));
                    }
                    Some(CoordinatorMsg::IsLeader { node_id, tx }) => {
                        let _ = tx.send(Ok(leader == Some(node_id)));
                    }
                    Some(CoordinatorMsg::GetConsensusState { tx }) => {
                        let _ = tx.send(Ok(consensus.clone()));
                    }
                    Some(CoordinatorMsg::UpdateConsensusState { state, tx }) => {
                        consensus = state;
                        let _ = tx.send(Ok(()));
                    }
                    Some(CoordinatorMsg::AddParticipant { node_id, tx }) => {
                        if !consensus.participants.contains(&node_id) {
                            consensus.participants.push(node_id);
                            info!("Added node {node_id:?} to consensus participants");
                        }
                        let _ = tx.send(Ok(()));
                    }
                    Some(CoordinatorMsg::RemoveParticipant { node_id, tx }) => {
                        consensus.participants.retain(|&id| id != node_id);
                        info!("Removed node {node_id:?} from consensus participants");
                        let _ = tx.send(Ok(()));
                    }
                    Some(CoordinatorMsg::HasQuorum { tx }) => {
                        let _ = tx.send(Ok(consensus.participants.len() >= consensus.quorum_size));
                    }
                    Some(CoordinatorMsg::IncrementTerm { tx }) => {
                        consensus.term += 1;
                        info!("Consensus term incremented to {}", consensus.term);
                        let _ = tx.send(Ok(consensus.term));
                    }
                    Some(CoordinatorMsg::Shutdown) | None => break,
                }
            }
        });

        Ok(Self { tx })
    }

    /// Calculate quorum size based on cluster configuration
    fn calculate_quorum_size(config: &ClusterConfig) -> usize {
        match config.consensus.algorithm {
            ConsensusAlgorithm::Raft => {
                let n = config.consensus.min_cluster_size;
                (n / 2) + 1
            }
            ConsensusAlgorithm::LeaderElection => 1,
            ConsensusAlgorithm::None => 1,
        }
    }

    /// Start the cluster coordinator (no-op — actor starts in `new`)
    pub async fn start(&self) -> Result<()> {
        info!("Cluster coordinator started");
        Ok(())
    }

    /// Stop the cluster coordinator
    pub async fn stop(&self) -> Result<()> {
        let _ = self.tx.send(CoordinatorMsg::Shutdown).await;
        info!("Cluster coordinator stopped");
        Ok(())
    }

    /// Acquire a distributed lock
    pub async fn acquire_lock(
        &self,
        lock_id: String,
        node_id: NodeId,
        timeout: std::time::Duration,
    ) -> Result<bool> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::AcquireLock {
                lock_id,
                node_id,
                timeout,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Release a distributed lock
    pub async fn release_lock(&self, lock_id: String, node_id: NodeId) -> Result<bool> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::ReleaseLock {
                lock_id,
                node_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Get current leader node
    pub async fn get_leader(&self) -> Result<Option<NodeId>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::GetLeader { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Set leader node
    pub async fn set_leader(&self, leader_id: Option<NodeId>) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::SetLeader {
                leader_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Check if this node is the leader
    pub async fn is_leader(&self, node_id: NodeId) -> Result<bool> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::IsLeader {
                node_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Get consensus state
    pub async fn get_consensus_state(&self) -> Result<ConsensusProtocol> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::GetConsensusState { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Update consensus state
    pub async fn update_consensus_state(&self, new_state: ConsensusProtocol) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::UpdateConsensusState {
                state: new_state,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Add node to consensus participants
    pub async fn add_participant(&self, node_id: NodeId) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::AddParticipant {
                node_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Remove node from consensus participants
    pub async fn remove_participant(&self, node_id: NodeId) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::RemoveParticipant {
                node_id,
                tx: resp_tx,
            })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Check if quorum is available
    pub async fn has_quorum(&self) -> Result<bool> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::HasQuorum { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }

    /// Increment consensus term
    pub async fn increment_term(&self) -> Result<u64> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx
            .send(CoordinatorMsg::IncrementTerm { tx: resp_tx })
            .await
            .map_err(|_| Error::Internal {
                message: "Coordinator actor is down".to_string(),
            })?;
        resp_rx.await.map_err(|_| Error::Internal {
            message: "Coordinator did not respond".to_string(),
        })?
    }
}

impl DistributedLock {
    /// Check if lock has expired
    #[must_use]
    pub fn is_expired(&self) -> bool {
        self.acquired_at.elapsed() >= self.timeout
    }

    /// Get remaining time before lock expires
    #[must_use]
    pub fn time_remaining(&self) -> Option<std::time::Duration> {
        self.timeout.checked_sub(self.acquired_at.elapsed())
    }
}

impl ConsensusProtocol {
    /// Create a new consensus protocol state
    #[must_use]
    pub fn new(algorithm: ConsensusAlgorithm, min_cluster_size: usize) -> Self {
        let quorum_size = match algorithm {
            ConsensusAlgorithm::Raft => (min_cluster_size / 2) + 1,
            _ => 1,
        };

        Self {
            algorithm,
            leader: None,
            term: 0,
            participants: Vec::new(),
            quorum_size,
        }
    }

    /// Check if a quorum can be reached with current participants
    #[must_use]
    pub fn can_reach_quorum(&self) -> bool {
        self.participants.len() >= self.quorum_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_coordinator_creation() {
        let config = ClusterConfig::default();
        let coordinator = ClusterCoordinator::new(config).await;
        assert!(coordinator.is_ok());
    }

    #[tokio::test]
    async fn test_lock_acquisition() -> Result<()> {
        let config = ClusterConfig::default();
        let coordinator = ClusterCoordinator::new(config)
            .await
            .expect("Failed to create cluster coordinator with given configuration");

        let lock_id = "test_lock".to_string();
        let node_id = uuid::Uuid::new_v4();
        let timeout = std::time::Duration::from_secs(60);

        let acquired = coordinator
            .acquire_lock(lock_id.clone(), node_id, timeout)
            .await?;
        assert!(acquired);

        // Try to acquire same lock again (should fail)
        let acquired2 = coordinator
            .acquire_lock(lock_id.clone(), uuid::Uuid::new_v4(), timeout)
            .await?;
        assert!(!acquired2);
        Ok(())
    }

    #[tokio::test]
    async fn test_lock_release() -> Result<()> {
        let config = ClusterConfig::default();
        let coordinator = ClusterCoordinator::new(config)
            .await
            .expect("Failed to create cluster coordinator with given configuration");

        let lock_id = "test_lock".to_string();
        let node_id = uuid::Uuid::new_v4();
        let timeout = std::time::Duration::from_secs(60);

        coordinator
            .acquire_lock(lock_id.clone(), node_id, timeout)
            .await?;
        let released = coordinator.release_lock(lock_id.clone(), node_id).await?;
        assert!(released);

        // Should be able to acquire again after release
        let acquired = coordinator.acquire_lock(lock_id, node_id, timeout).await?;
        assert!(acquired);
        Ok(())
    }

    #[tokio::test]
    async fn test_leader_management() {
        let config = ClusterConfig::default();
        let coordinator = ClusterCoordinator::new(config)
            .await
            .expect("Failed to create cluster coordinator with given configuration");

        let node_id = uuid::Uuid::new_v4();

        // Initially no leader
        let leader = coordinator
            .get_leader()
            .await
            .expect("Failed to get cluster leader node");
        assert!(leader.is_none());

        // Set leader
        coordinator
            .set_leader(Some(node_id))
            .await
            .expect("Failed to set cluster leader node");
        let leader = coordinator
            .get_leader()
            .await
            .expect("Failed to get cluster leader node");
        assert_eq!(leader, Some(node_id));

        // Check if node is leader
        let is_leader = coordinator
            .is_leader(node_id)
            .await
            .expect("Failed to check if node is cluster leader");
        assert!(is_leader);
    }

    #[tokio::test]
    async fn test_consensus_participants() {
        let config = ClusterConfig::default();
        let coordinator = ClusterCoordinator::new(config)
            .await
            .expect("Failed to create cluster coordinator with given configuration");

        let node1 = uuid::Uuid::new_v4();
        let node2 = uuid::Uuid::new_v4();

        coordinator
            .add_participant(node1)
            .await
            .expect("Failed to add participant node to cluster");
        coordinator
            .add_participant(node2)
            .await
            .expect("Failed to add participant node to cluster");

        let state = coordinator
            .get_consensus_state()
            .await
            .expect("Failed to get consensus state from cluster");
        assert_eq!(state.participants.len(), 2);
        assert!(state.participants.contains(&node1));
        assert!(state.participants.contains(&node2));
    }

    #[test]
    fn test_lock_expiration() {
        let lock = DistributedLock {
            lock_id: "test".to_string(),
            holder: Some(uuid::Uuid::new_v4()),
            acquired_at: std::time::Instant::now() - std::time::Duration::from_secs(120),
            timeout: std::time::Duration::from_secs(60),
        };

        assert!(lock.is_expired());
        assert!(lock.time_remaining().is_none());
    }
}

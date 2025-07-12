//! Blocking strategies for tableau reasoning
//!
//! This module implements various blocking techniques used to ensure termination
//! of tableau algorithms while preserving correctness and completeness.

use crate::{
    config::{BlockingStrategy as ConfigBlockingStrategy, ReasoningConfig},
    core::tableau::{TableauNode, ConceptLabel, NodeId},
    Error, Result,
};
use std::collections::{HashMap, HashSet};

/// Trait for blocking checkers
pub trait BlockingChecker: Send + Sync + std::fmt::Debug {
    /// Check if a node is blocked by another node
    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId>;
    
    /// Update blocking information after node changes
    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()>;
    
    /// Get the blocking signature for a node
    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel>;
}

/// Factory for creating blocking checkers
pub struct BlockingStrategy;

impl BlockingStrategy {
    /// Create a blocking checker based on configuration
    pub fn create_checker(config: &ReasoningConfig) -> Result<Box<dyn BlockingChecker>> {
        match config.blocking_strategy {
            ConfigBlockingStrategy::Anywhere => Ok(Box::new(AnywhereBlocking::new())),
            ConfigBlockingStrategy::Ancestor => Ok(Box::new(AncestorBlocking::new())),
            ConfigBlockingStrategy::Pairwise => Ok(Box::new(PairwiseBlocking::new())),
            ConfigBlockingStrategy::Dynamic => Ok(Box::new(DynamicBlocking::new())),
        }
    }
}

/// Anywhere blocking strategy
#[derive(Debug)]
pub struct AnywhereBlocking {
    /// Cache of node signatures
    signature_cache: HashMap<NodeId, Vec<ConceptLabel>>,
}

impl AnywhereBlocking {
    pub fn new() -> Self {
        Self {
            signature_cache: HashMap::new(),
        }
    }
}

impl BlockingChecker for AnywhereBlocking {
    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        let node_signature = self.get_signature(node);
        
        // Check if any previous node has the same signature
        for other_node in nodes {
            if other_node.id != node.id && other_node.id < node.id {
                let other_signature = self.get_signature(other_node);
                if signatures_subsume(&other_signature, &node_signature) {
                    return Some(other_node.id);
                }
            }
        }
        
        None
    }
    
    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        // Clear cache
        self.signature_cache.clear();
        
        // Update blocking information for all nodes
        for i in 0..nodes.len() {
            let blocker = self.is_blocked(&nodes[i], nodes);
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }
        
        Ok(())
    }
    
    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        if let Some(cached) = self.signature_cache.get(&node.id) {
            return cached.clone();
        }
        
        let mut signature: Vec<_> = node.concepts.iter().cloned().collect();
        signature.sort_by(|a, b| format!("{:?}", a).cmp(&format!("{:?}", b)));
        signature
    }
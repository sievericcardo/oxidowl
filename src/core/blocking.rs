//! Blocking strategies for tableau reasoning
//!
//! This module implements various blocking techniques used to ensure termination
//! of tableau algorithms while preserving correctness and completeness.

use crate::{
    config::{BlockingStrategy as ConfigBlockingStrategy, ReasoningConfig},
    core::tableau::{TableauNode, ConceptLabel, NodeId},
    Result,
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

impl Default for AnywhereBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl AnywhereBlocking {
    #[must_use] pub fn new() -> Self {
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
        signature.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        signature
    }
}

/// Ancestor blocking strategy
#[derive(Debug)]
pub struct AncestorBlocking {
    /// Parent relationships between nodes
    parent_map: HashMap<NodeId, NodeId>,
}

impl Default for AncestorBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl AncestorBlocking {
    #[must_use] pub fn new() -> Self {
        Self {
            parent_map: HashMap::new(),
        }
    }
    
    /// Check if one node is an ancestor of another
    fn is_ancestor(&self, potential_ancestor: NodeId, node: NodeId) -> bool {
        let mut current = node;
        while let Some(&parent) = self.parent_map.get(&current) {
            if parent == potential_ancestor {
                return true;
            }
            current = parent;
        }
        false
    }
}

impl BlockingChecker for AncestorBlocking {
    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        let node_signature = self.get_signature(node);
        
        // Check only ancestor nodes
        for other_node in nodes {
            if other_node.id != node.id && self.is_ancestor(other_node.id, node.id) {
                let other_signature = self.get_signature(other_node);
                if signatures_subsume(&other_signature, &node_signature) {
                    return Some(other_node.id);
                }
            }
        }
        
        None
    }
    
    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        // Update blocking information
        for i in 0..nodes.len() {
            let blocker = self.is_blocked(&nodes[i], nodes);
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }
        
        Ok(())
    }
    
    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        let mut signature: Vec<_> = node.concepts.iter().cloned().collect();
        signature.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        signature
    }
}

/// Pairwise blocking strategy
#[derive(Debug)]
pub struct PairwiseBlocking {
    /// Pairs of nodes that have been compared
    compared_pairs: HashSet<(NodeId, NodeId)>,
}

impl Default for PairwiseBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl PairwiseBlocking {
    #[must_use] pub fn new() -> Self {
        Self {
            compared_pairs: HashSet::new(),
        }
    }
}

impl BlockingChecker for PairwiseBlocking {
    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        let node_signature = self.get_signature(node);
        
        // Check pairwise relationships
        for other_node in nodes {
            if other_node.id != node.id && 
               !self.compared_pairs.contains(&(node.id, other_node.id)) {
                let other_signature = self.get_signature(other_node);
                if signatures_subsume(&other_signature, &node_signature) {
                    return Some(other_node.id);
                }
            }
        }
        
        None
    }
    
    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        // Update blocking information
        for i in 0..nodes.len() {
            let blocker = self.is_blocked(&nodes[i], nodes);
            nodes[i].blocking_info.is_blocked = blocker.is_some();
            nodes[i].blocking_info.blocker = blocker;
        }
        
        Ok(())
    }
    
    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        let mut signature: Vec<_> = node.concepts.iter().cloned().collect();
        signature.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
        signature
    }
}

/// Dynamic blocking strategy that adapts based on tableau characteristics
#[derive(Debug)]
pub struct DynamicBlocking {
    /// Current strategy being used
    current_strategy: Box<dyn BlockingChecker>,
    
    /// Statistics for adaptation
    stats: BlockingStatistics,
}

#[derive(Debug, Default)]
struct BlockingStatistics {
    blocked_nodes: usize,
    total_comparisons: usize,
    false_positives: usize,
}

impl Default for DynamicBlocking {
    fn default() -> Self {
        Self::new()
    }
}

impl DynamicBlocking {
    #[must_use] pub fn new() -> Self {
        Self {
            current_strategy: Box::new(AnywhereBlocking::new()),
            stats: BlockingStatistics::default(),
        }
    }
    
    /// Adapt the blocking strategy based on performance
    fn adapt_strategy(&mut self) {
        // Simple adaptation logic - in practice this would be more sophisticated
        if self.stats.false_positives > self.stats.blocked_nodes / 2 {
            // Too many false positives, switch to more precise strategy
            if !self.is_ancestor_blocking() {
                self.current_strategy = Box::new(AncestorBlocking::new());
            }
        }
    }
    
    fn is_ancestor_blocking(&self) -> bool {
        // Check if current strategy is ancestor blocking
        format!("{:?}", self.current_strategy).contains("AncestorBlocking")
    }
}

impl BlockingChecker for DynamicBlocking {
    fn is_blocked(&self, node: &TableauNode, nodes: &[TableauNode]) -> Option<NodeId> {
        self.current_strategy.is_blocked(node, nodes)
    }
    
    fn update_blocking(&mut self, nodes: &mut [TableauNode]) -> Result<()> {
        let result = self.current_strategy.update_blocking(nodes);
        
        // Update statistics
        self.stats.blocked_nodes = nodes.iter().filter(|n| n.blocking_info.is_blocked).count();
        
        // Adapt strategy if needed
        self.adapt_strategy();
        
        result
    }
    
    fn get_signature(&self, node: &TableauNode) -> Vec<ConceptLabel> {
        self.current_strategy.get_signature(node)
    }
}

/// Check if one signature subsumes another
fn signatures_subsume(sig1: &[ConceptLabel], sig2: &[ConceptLabel]) -> bool {
    // Simple subsumption check - sig1 subsumes sig2 if all concepts in sig1 are in sig2
    sig1.iter().all(|concept| sig2.contains(concept))
}

/// More sophisticated signature comparison
fn signatures_equal(sig1: &[ConceptLabel], sig2: &[ConceptLabel]) -> bool {
    if sig1.len() != sig2.len() {
        return false;
    }
    
    sig1.iter().all(|concept| sig2.contains(concept))
}

/// Compute signature similarity (for dynamic blocking)
fn signature_similarity(sig1: &[ConceptLabel], sig2: &[ConceptLabel]) -> f64 {
    if sig1.is_empty() && sig2.is_empty() {
        return 1.0;
    }
    
    let intersection_size = sig1.iter().filter(|concept| sig2.contains(concept)).count();
    let union_size = sig1.len() + sig2.len() - intersection_size;
    
    if union_size == 0 {
        1.0
    } else {
        intersection_size as f64 / union_size as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tableau::{NodeType, BlockingInfo, NodeStatus};
    use std::collections::HashMap;

    fn create_test_node(id: NodeId, concepts: Vec<&str>) -> TableauNode {
        let concept_set: HashSet<ConceptLabel> = concepts
            .into_iter()
            .map(|c| ConceptLabel::Atomic(c.to_string()))
            .collect();
        
        TableauNode {
            id,
            concepts: concept_set,
            node_type: NodeType::Individual,
            blocking_info: BlockingInfo::default(),
            concept_dependencies: HashMap::new(),
            status: NodeStatus::default(),
        }
    }

    #[test]
    fn test_anywhere_blocking() {
        let blocking = AnywhereBlocking::new();
        
        let node1 = create_test_node(0, vec!["A", "B"]);
        let node2 = create_test_node(1, vec!["A", "B", "C"]);
        let node3 = create_test_node(2, vec!["A", "B"]);
        
        let nodes = vec![node1, node2, node3];
        
        // Node 3 should be blocked by node 1
        let blocked = blocking.is_blocked(&nodes[2], &nodes);
        assert_eq!(blocked, Some(0));
    }

    #[test]
    fn test_signature_subsumption() {
        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];
        
        let sig2 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
            ConceptLabel::Atomic("C".to_string()),
        ];
        
        assert!(signatures_subsume(&sig1, &sig2));
        assert!(!signatures_subsume(&sig2, &sig1));
    }

    #[test]
    fn test_signature_equality() {
        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];
        
        let sig2 = vec![
            ConceptLabel::Atomic("B".to_string()),
            ConceptLabel::Atomic("A".to_string()),
        ];
        
        assert!(signatures_equal(&sig1, &sig2));
    }

    #[test]
    fn test_signature_similarity() {
        let sig1 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("B".to_string()),
        ];
        
        let sig2 = vec![
            ConceptLabel::Atomic("A".to_string()),
            ConceptLabel::Atomic("C".to_string()),
        ];
        
        let similarity = signature_similarity(&sig1, &sig2);
        assert!((similarity - 0.333).abs() < 0.01); // 1/3 similarity
    }
}
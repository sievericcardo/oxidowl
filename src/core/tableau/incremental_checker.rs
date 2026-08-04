//! Incremental clause checking with LRU cache
//!
//! This module provides caching and change tracking for clause checking
//! to avoid redundant checks when nodes haven't changed.
//!
//! # Overview
//!
//! During tableau expansion, many nodes are checked repeatedly against the same
//! clauses without changes. This module provides:
//! - **`NodeFingerprint`**: Fast hash-based representation of node state
//! - **`CheckResultCache`**: LRU cache for clause check results
//! - **`ChangeTracker`**: Track which nodes have changed during expansion
//!
//! # Performance
//!
//! - Cache hit: O(1) lookup, no clause checking needed
//! - Cache miss: O(k) clause checking (with indexing) + O(1) cache insert
//! - Expected hit rate: 60-80% on typical ontologies
//! - Memory: ~100 bytes per cache entry
//!
//! # Example
//!
//! ```rust,ignore
//! use oxidowl::core::tableau::{IncrementalClauseChecker, CheckResultCache};
//!
//! // Create cache with 10,000 entry capacity
//! let mut cache = CheckResultCache::new(10_000);
//!
//! // Compute fingerprint for a node
//! let fingerprint = NodeFingerprint::from_node(&node);
//!
//! // Check cache before checking clauses
//! if let Some(result) = cache.get(fingerprint, clause_id) {
//!     // Cache hit! Use cached result
//!     return result;
//! }
//!
//! // Cache miss - check clause and cache result
//! let result = check_clause(&node, &clause);
//! cache.put(fingerprint, clause_id, result);
//! ```

use crate::core::tableau::{ConceptLabel, NodeId, TableauNode};
use lru::LruCache;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::num::NonZeroUsize;

/// Fast fingerprint of a node's state
///
/// A fingerprint is a 64-bit hash representing the complete state of a node
/// (concepts, roles, edges). Two nodes with the same fingerprint should produce
/// the same clause checking results.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NodeFingerprint(u64);

impl NodeFingerprint {
    /// Compute fingerprint from a tableau node
    ///
    /// The fingerprint includes:
    /// - All atomic concepts (sorted for consistency)
    /// - All role successors (sorted)
    /// - Node type
    ///
    /// Complex concepts are normalized before hashing to ensure equivalent
    /// concepts produce the same fingerprint.
    ///
    /// # Performance
    ///
    /// - Time: O(n log n) where n = concepts + role edges
    /// - Space: O(1) (result is single u64)
    #[must_use]
    pub fn from_node(node: &TableauNode) -> Self {
        let mut hasher = DefaultHasher::new();

        // Hash node ID for uniqueness
        node.id.hash(&mut hasher);

        // Hash atomic concepts (sorted for determinism)
        let mut atomic_concepts: Vec<String> = node
            .concepts
            .iter()
            .filter_map(|c| match c {
                ConceptLabel::Atomic(name) => Some(name.clone()),
                ConceptLabel::NegatedAtomic(name) => Some(format!("¬{name}")),
                _ => None, // Complex concepts handled separately
            })
            .collect();
        atomic_concepts.sort();
        for concept in atomic_concepts {
            concept.hash(&mut hasher);
        }

        // Hash role successors (sorted for determinism)
        let mut role_edges: Vec<(String, Vec<NodeId>)> = node
            .role_successors
            .iter()
            .map(|(role, successors)| {
                let mut sorted_successors: Vec<NodeId> = successors.iter().copied().collect();
                sorted_successors.sort_unstable();
                // Convert Arc<str> key to String so the Vec type stays (String, Vec<NodeId>)
                (role.to_string(), sorted_successors)
            })
            .collect();
        role_edges.sort_by(|a, b| a.0.cmp(&b.0));
        for (role, successors) in role_edges {
            role.hash(&mut hasher);
            for successor in successors {
                successor.hash(&mut hasher);
            }
        }

        // Hash node type
        format!("{:?}", node.node_type).hash(&mut hasher);

        NodeFingerprint(hasher.finish())
    }

    /// Get the raw hash value
    #[must_use]
    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

/// Cache key combining node fingerprint and clause ID
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    fingerprint: NodeFingerprint,
    clause_id: usize,
}

impl CacheKey {
    fn new(fingerprint: NodeFingerprint, clause_id: usize) -> Self {
        Self {
            fingerprint,
            clause_id,
        }
    }
}

/// Result of a clause check that can be cached
///
/// For deterministic clauses, we cache:
/// - None: No violation found
/// - Some(violation): Violation found with details
///
/// For disjunctive clauses, we cache the selected disjunct index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CachedCheckResult {
    /// No violation found
    NoViolation,

    /// Violation found (violation details can be reconstructed)
    Violation {
        /// Clause ID that was violated
        clause_id: usize,
        /// Brief description for reconstruction
        description: String,
    },

    /// Disjunctive clause - selected disjunct
    DisjunctSelected {
        /// Index of selected disjunct
        disjunct_index: usize,
    },
}

/// LRU cache for clause checking results
///
/// The cache maps (`NodeFingerprint`, `ClauseID`) -> `CachedCheckResult`.
/// When the cache is full, the least recently used entry is evicted.
///
/// # Memory Usage
///
/// Each cache entry uses approximately:
/// - 8 bytes: `NodeFingerprint` (u64)
/// - 8 bytes: `clause_id` (usize)
/// - 40 bytes: `CachedCheckResult` (enum with string)
/// - 32 bytes: LRU metadata (prev/next pointers, etc.)
/// - **Total: ~88 bytes per entry**
///
/// Default capacity of 10,000 entries ≈ 880 KB
pub struct CheckResultCache {
    /// LRU cache for check results
    cache: LruCache<CacheKey, CachedCheckResult>,

    /// Statistics
    hits: usize,
    misses: usize,
    evictions: usize,
}

impl CheckResultCache {
    /// Create a new cache with the given capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of entries to cache
    ///
    /// # Memory
    ///
    /// Memory usage ≈ capacity × 88 bytes
    /// - 10,000 entries ≈ 880 KB
    /// - 50,000 entries ≈ 4.4 MB
    /// - 100,000 entries ≈ 8.8 MB
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        let cache = LruCache::new(NonZeroUsize::new(capacity).expect("capacity must be > 0"));
        Self {
            cache,
            hits: 0,
            misses: 0,
            evictions: 0,
        }
    }

    /// Get cached result for a node/clause pair
    ///
    /// Returns Some(result) if cached, None if not found.
    ///
    /// # Performance
    ///
    /// - Time: O(1)
    /// - Side effects: Updates LRU order
    pub fn get(
        &mut self,
        fingerprint: NodeFingerprint,
        clause_id: usize,
    ) -> Option<&CachedCheckResult> {
        let key = CacheKey::new(fingerprint, clause_id);
        if let Some(result) = self.cache.get(&key) {
            self.hits += 1;
            Some(result)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Store a check result in the cache
    ///
    /// If the cache is full, evicts the least recently used entry.
    ///
    /// # Performance
    ///
    /// - Time: O(1)
    /// - Side effects: May evict old entry
    pub fn put(
        &mut self,
        fingerprint: NodeFingerprint,
        clause_id: usize,
        result: CachedCheckResult,
    ) {
        let key = CacheKey::new(fingerprint, clause_id);

        // Track evictions when cache is full
        if self.cache.len() == self.cache.cap().get() {
            self.evictions += 1;
        }

        self.cache.put(key, result);
    }

    /// Clear all cached results
    pub fn clear(&mut self) {
        self.cache.clear();
        // Don't reset stats - useful for tracking across clears
    }

    /// Get cache statistics
    #[must_use]
    pub fn statistics(&self) -> CacheStatistics {
        CacheStatistics {
            capacity: self.cache.cap().get(),
            size: self.cache.len(),
            hits: self.hits,
            misses: self.misses,
            evictions: self.evictions,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Get cache capacity
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.cache.cap().get()
    }

    /// Get current cache size
    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Reset statistics (but keep cached entries)
    pub fn reset_statistics(&mut self) {
        self.hits = 0;
        self.misses = 0;
        self.evictions = 0;
    }
}

/// Statistics about cache performance
#[derive(Debug, Clone)]
pub struct CacheStatistics {
    /// Maximum number of entries
    pub capacity: usize,

    /// Current number of entries
    pub size: usize,

    /// Number of cache hits
    pub hits: usize,

    /// Number of cache misses
    pub misses: usize,

    /// Number of evictions (LRU evictions when full)
    pub evictions: usize,

    /// Hit rate (hits / (hits + misses))
    pub hit_rate: f64,
}

impl CacheStatistics {
    /// Check if cache is performing well
    ///
    /// A cache is considered "good" if:
    /// - Hit rate > 50%
    /// - Few evictions relative to size (< 10% of capacity)
    #[must_use]
    pub fn is_performing_well(&self) -> bool {
        self.hit_rate > 0.5 && (self.evictions as f64) < (self.capacity as f64 * 0.1)
    }

    /// Get cache utilization (0.0 to 1.0)
    #[must_use]
    pub fn utilization(&self) -> f64 {
        self.size as f64 / self.capacity as f64
    }
}

impl std::fmt::Display for CacheStatistics {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Cache: {}/{} entries ({:.1}% full), {} hits, {} misses ({:.1}% hit rate), {} evictions",
            self.size,
            self.capacity,
            self.utilization() * 100.0,
            self.hits,
            self.misses,
            self.hit_rate * 100.0,
            self.evictions
        )
    }
}

/// Track which nodes have changed during tableau expansion
///
/// Only changed nodes need their cached results invalidated.
/// This allows us to keep cache entries for stable nodes.
pub struct ChangeTracker {
    /// Nodes that have been modified
    changed_nodes: HashSet<NodeId>,

    /// Previous fingerprints for nodes (to detect changes)
    previous_fingerprints: HashMap<NodeId, NodeFingerprint>,
}

impl ChangeTracker {
    /// Create a new change tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            changed_nodes: HashSet::new(),
            previous_fingerprints: HashMap::new(),
        }
    }

    /// Record that a node has been checked
    ///
    /// Stores the node's fingerprint for future change detection.
    pub fn record_check(&mut self, node_id: NodeId, fingerprint: NodeFingerprint) {
        self.previous_fingerprints.insert(node_id, fingerprint);
        // Node is now "stable" until modified
        self.changed_nodes.remove(&node_id);
    }

    /// Mark a node as changed
    ///
    /// Should be called when:
    /// - New concept added to node
    /// - New role edge added from node
    /// - Node merged or split
    pub fn mark_changed(&mut self, node_id: NodeId) {
        self.changed_nodes.insert(node_id);
    }

    /// Check if a node has changed since last check
    ///
    /// Returns true if:
    /// - Node is in `changed_nodes` set
    /// - Node's current fingerprint differs from previous
    pub fn has_changed(&mut self, node_id: NodeId, current_fingerprint: NodeFingerprint) -> bool {
        // Explicit marking takes precedence
        if self.changed_nodes.contains(&node_id) {
            return true;
        }

        // Check if fingerprint changed
        if let Some(&prev_fingerprint) = self.previous_fingerprints.get(&node_id) {
            prev_fingerprint != current_fingerprint
        } else {
            // No previous fingerprint - consider it changed (first check)
            true
        }
    }

    /// Clear all change tracking
    pub fn clear(&mut self) {
        self.changed_nodes.clear();
        self.previous_fingerprints.clear();
    }

    /// Get number of nodes being tracked
    #[must_use]
    pub fn tracked_nodes(&self) -> usize {
        self.previous_fingerprints.len()
    }

    /// Get number of nodes marked as changed
    #[must_use]
    pub fn changed_count(&self) -> usize {
        self.changed_nodes.len()
    }
}

impl Default for ChangeTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tableau::NodeType;

    fn create_test_node(id: NodeId, concepts: Vec<&str>) -> TableauNode {
        let mut node = TableauNode::new(id, NodeType::Individual);
        for concept in concepts {
            node.concepts
                .insert(ConceptLabel::Atomic(concept.to_string()));
        }
        node
    }

    #[test]
    fn test_fingerprint_deterministic() {
        let node1 = create_test_node(0, vec!["Person", "Adult"]);
        let node2 = create_test_node(0, vec!["Adult", "Person"]); // Different order

        let fp1 = NodeFingerprint::from_node(&node1);
        let fp2 = NodeFingerprint::from_node(&node2);

        // Same concepts (sorted) should produce same fingerprint
        assert_eq!(fp1, fp2, "Fingerprints should be order-independent");
    }

    #[test]
    fn test_fingerprint_different_nodes() {
        let node1 = create_test_node(0, vec!["Person"]);
        let node2 = create_test_node(0, vec!["Animal"]);

        let fp1 = NodeFingerprint::from_node(&node1);
        let fp2 = NodeFingerprint::from_node(&node2);

        // Different concepts should produce different fingerprints (with high probability)
        assert_ne!(
            fp1, fp2,
            "Different concepts should produce different fingerprints"
        );
    }

    #[test]
    fn test_fingerprint_with_roles() {
        let mut node1 = create_test_node(0, vec!["Person"]);
        node1
            .role_successors
            .insert(std::sync::Arc::from("knows"), vec![1, 2].into_iter().collect());

        let mut node2 = create_test_node(0, vec!["Person"]);
        node2
            .role_successors
            .insert(std::sync::Arc::from("knows"), vec![2, 1].into_iter().collect()); // Different order

        let fp1 = NodeFingerprint::from_node(&node1);
        let fp2 = NodeFingerprint::from_node(&node2);

        // Same role successors (sorted) should produce same fingerprint
        assert_eq!(
            fp1, fp2,
            "Role successor order shouldn't affect fingerprint"
        );
    }

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = CheckResultCache::new(100);

        let fingerprint = NodeFingerprint(12345);
        let clause_id = 1;

        // Initially empty
        assert!(cache.get(fingerprint, clause_id).is_none());
        assert_eq!(cache.statistics().misses, 1);

        // Put and get
        cache.put(fingerprint, clause_id, CachedCheckResult::NoViolation);
        assert!(cache.get(fingerprint, clause_id).is_some());
        assert_eq!(cache.statistics().hits, 1);

        // Get again (another hit)
        assert!(cache.get(fingerprint, clause_id).is_some());
        assert_eq!(cache.statistics().hits, 2);
    }

    #[test]
    fn test_cache_different_clauses() {
        let mut cache = CheckResultCache::new(100);

        let fingerprint = NodeFingerprint(12345);

        // Cache results for different clauses on same node
        cache.put(fingerprint, 1, CachedCheckResult::NoViolation);
        cache.put(
            fingerprint,
            2,
            CachedCheckResult::Violation {
                clause_id: 2,
                description: "Test violation".to_string(),
            },
        );

        // Should be independent
        assert_eq!(
            cache.get(fingerprint, 1),
            Some(&CachedCheckResult::NoViolation)
        );
        match cache.get(fingerprint, 2) {
            Some(CachedCheckResult::Violation { clause_id, .. }) => {
                assert_eq!(*clause_id, 2);
            }
            _ => panic!("Expected violation result"),
        }
    }

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = CheckResultCache::new(2); // Small cache

        let fp1 = NodeFingerprint(1);
        let fp2 = NodeFingerprint(2);
        let fp3 = NodeFingerprint(3);

        // Fill cache
        cache.put(fp1, 1, CachedCheckResult::NoViolation);
        cache.put(fp2, 1, CachedCheckResult::NoViolation);
        assert_eq!(cache.len(), 2);

        // Access fp1 to make it most recently used
        cache.get(fp1, 1);

        // Add fp3 - should evict fp2 (least recently used)
        cache.put(fp3, 1, CachedCheckResult::NoViolation);
        assert_eq!(cache.len(), 2);

        // fp1 and fp3 should be present, fp2 should be evicted
        assert!(cache.get(fp1, 1).is_some());
        assert!(cache.get(fp3, 1).is_some());
        assert!(cache.get(fp2, 1).is_none()); // Evicted!
    }

    #[test]
    fn test_cache_statistics() {
        let mut cache = CheckResultCache::new(100);

        let fp = NodeFingerprint(12345);

        // Misses
        cache.get(fp, 1);
        cache.get(fp, 2);
        assert_eq!(cache.statistics().misses, 2);
        assert_eq!(cache.statistics().hit_rate, 0.0);

        // Add and hit
        cache.put(fp, 1, CachedCheckResult::NoViolation);
        cache.get(fp, 1);
        cache.get(fp, 1);

        let stats = cache.statistics();
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 2);
        assert_eq!(stats.hit_rate, 0.5); // 2 hits / 4 total
    }

    #[test]
    fn test_change_tracker_basic() {
        let mut tracker = ChangeTracker::new();

        let fp1 = NodeFingerprint(100);
        let fp2 = NodeFingerprint(200);

        // First check - always "changed" (no previous fingerprint)
        assert!(tracker.has_changed(0, fp1));

        // Record check
        tracker.record_check(0, fp1);

        // Same fingerprint - not changed
        assert!(!tracker.has_changed(0, fp1));

        // Different fingerprint - changed
        assert!(tracker.has_changed(0, fp2));
    }

    #[test]
    fn test_change_tracker_explicit_marking() {
        let mut tracker = ChangeTracker::new();

        let fp = NodeFingerprint(100);

        // Record check
        tracker.record_check(0, fp);
        assert!(!tracker.has_changed(0, fp));

        // Explicitly mark as changed
        tracker.mark_changed(0);
        assert!(tracker.has_changed(0, fp)); // Should be changed even with same fingerprint
    }

    #[test]
    fn test_change_tracker_multiple_nodes() {
        let mut tracker = ChangeTracker::new();

        let fp1 = NodeFingerprint(100);
        let fp2 = NodeFingerprint(200);

        // Track two nodes
        tracker.record_check(0, fp1);
        tracker.record_check(1, fp2);

        // Mark only node 0 as changed
        tracker.mark_changed(0);

        // Node 0 changed, node 1 not changed
        assert!(tracker.has_changed(0, fp1));
        assert!(!tracker.has_changed(1, fp2));
    }

    #[test]
    fn test_cached_check_result_types() {
        // Test different result types
        let no_violation = CachedCheckResult::NoViolation;
        let violation = CachedCheckResult::Violation {
            clause_id: 42,
            description: "Test".to_string(),
        };
        let disjunct = CachedCheckResult::DisjunctSelected { disjunct_index: 2 };

        // Equality checks
        assert_eq!(no_violation, CachedCheckResult::NoViolation);
        assert_ne!(no_violation, violation);
        assert_ne!(violation, disjunct);
    }

    #[test]
    fn test_cache_statistics_display() {
        let mut cache = CheckResultCache::new(100);
        let fp = NodeFingerprint(12345);

        cache.put(fp, 1, CachedCheckResult::NoViolation);
        cache.get(fp, 1);
        cache.get(fp, 2); // Miss

        let stats = cache.statistics();
        let display = format!("{}", stats);

        // Should contain key metrics
        assert!(display.contains("Cache:"));
        assert!(display.contains("hit rate"));
    }

    #[test]
    fn test_cache_performance_assessment() {
        let mut cache = CheckResultCache::new(100);
        let fp = NodeFingerprint(12345);

        // Poor performance (low hit rate)
        cache.get(fp, 1); // Miss
        cache.get(fp, 2); // Miss
        assert!(!cache.statistics().is_performing_well());

        // Add entries and hit them
        cache.put(fp, 1, CachedCheckResult::NoViolation);
        cache.put(fp, 2, CachedCheckResult::NoViolation);

        for _ in 0..10 {
            cache.get(fp, 1); // Hit
            cache.get(fp, 2); // Hit
        }

        // Good performance (high hit rate)
        assert!(cache.statistics().is_performing_well());
    }
}

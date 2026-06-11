//! Completion Graph Reuse Cache
//!
//! Caches entire completion graph summaries so that repeated satisfiability
//! tests with the same input can reuse prior work rather than rebuilding from
//! scratch.
//!
//! A "summary" is a compact serializable representation of the key structural
//! features of a completed tableau: node count, edge count, representative
//! concept labels per node, and the satisfiability verdict.

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// Compact representation of a completion graph outcome.
#[derive(Debug, Clone)]
pub struct CompletionGraphSummary {
    /// Whether the graph is open (satisfiable) or closed (unsatisfiable).
    pub satisfiable: bool,
    /// Number of nodes in the final graph.
    pub node_count: usize,
    /// Number of edges in the final graph.
    pub edge_count: usize,
    /// Representative concept labels of root node (sorted).
    pub root_labels: Vec<String>,
    /// Cached at this instant.
    pub cached_at: Instant,
}

impl CompletionGraphSummary {
    #[must_use]
    pub fn new(
        satisfiable: bool,
        node_count: usize,
        edge_count: usize,
        root_labels: Vec<String>,
    ) -> Self {
        Self {
            satisfiable,
            node_count,
            edge_count,
            root_labels,
            cached_at: Instant::now(),
        }
    }
}

/// Key: the initial concept set (sorted and dedupled).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GraphCacheKey(Vec<String>);

impl GraphCacheKey {
    #[must_use]
    pub fn new(concepts: impl IntoIterator<Item = String>) -> Self {
        let mut v: Vec<String> = concepts.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        Self(v)
    }
}

/// Thread-safe completion-graph reuse cache.
#[derive(Debug, Clone)]
pub struct CompletionGraphCache {
    cache: Arc<DashMap<GraphCacheKey, CompletionGraphSummary>>,
    max_size: usize,
    ttl: Duration,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl CompletionGraphCache {
    #[must_use]
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_size,
            ttl,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Look up the cached summary for an initial concept set.
    pub fn get(
        &self,
        concepts: impl IntoIterator<Item = String>,
    ) -> Option<CompletionGraphSummary> {
        let key = GraphCacheKey::new(concepts);
        if let Some(summary) = self.cache.get(&key) {
            if summary.cached_at.elapsed() < self.ttl {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(summary.clone());
            }
            drop(summary);
            self.cache.remove(&key);
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Store a completion graph summary.
    pub fn insert(
        &self,
        concepts: impl IntoIterator<Item = String>,
        summary: CompletionGraphSummary,
    ) {
        if self.cache.len() >= self.max_size {
            // Simple FIFO-ish eviction.
            if let Some(key) = self.cache.iter().next().map(|e| e.key().clone()) {
                self.cache.remove(&key);
            }
        }
        let key = GraphCacheKey::new(concepts);
        self.cache.insert(key, summary);
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        if h + m == 0.0 { 0.0 } else { h / (h + m) }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

impl Default for CompletionGraphCache {
    fn default() -> Self {
        Self::new(10_000, Duration::from_secs(1 * 3600))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let cache = CompletionGraphCache::default();
        let summary = CompletionGraphSummary::new(true, 3, 2, vec!["A".to_string()]);
        cache.insert(["A".to_string(), "B".to_string()], summary);
        let found = cache.get(["B".to_string(), "A".to_string()]);
        assert!(found.is_some());
        assert!(found.unwrap().satisfiable);
    }
}

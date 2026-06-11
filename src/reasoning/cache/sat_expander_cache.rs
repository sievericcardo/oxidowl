//! Satisfiable Expander Cache
//!
//! Caches the result of node expansion — when a tableau node with a known
//! concept-label signature has been successfully expanded before, the expander
//! can reuse the result directly.
//!
//! Inspired by Konclude's satisfiable expander cache mechanism.
//!
//! Thread-safe via `DashMap`.

use dashmap::DashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// The expansion result for a node that was successfully satisfied.
#[derive(Debug, Clone)]
pub struct ExpanderCacheEntry {
    /// The set of additional concepts added to the node during expansion.
    pub added_concepts: Vec<String>,
    /// Successor signatures created (role, successor-concept-set pairs).
    pub successors: Vec<(String, Vec<String>)>,
    /// When this entry was computed.
    pub computed_at: Instant,
    /// How many times this entry has been used.
    pub hit_count: u64,
}

impl ExpanderCacheEntry {
    #[must_use]
    pub fn new(added_concepts: Vec<String>, successors: Vec<(String, Vec<String>)>) -> Self {
        Self {
            added_concepts,
            successors,
            computed_at: Instant::now(),
            hit_count: 0,
        }
    }
}

/// Signature used as cache key — sorted concepts at a node.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExpansionSignature(Vec<String>);

impl ExpansionSignature {
    #[must_use]
    pub fn new(concepts: impl IntoIterator<Item = String>) -> Self {
        let mut v: Vec<String> = concepts.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        Self(v)
    }
}

/// Thread-safe satisfiable expander cache.
#[derive(Debug, Clone)]
pub struct SatExpanderCache {
    cache: Arc<DashMap<ExpansionSignature, ExpanderCacheEntry>>,
    max_entries: usize,
    ttl: Duration,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl SatExpanderCache {
    #[must_use]
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            max_entries,
            ttl,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Look up expansion result for a node's concept set.
    pub fn get(&self, concepts: impl IntoIterator<Item = String>) -> Option<ExpanderCacheEntry> {
        let sig = ExpansionSignature::new(concepts);
        if let Some(mut entry) = self.cache.get_mut(&sig) {
            if entry.computed_at.elapsed() < self.ttl {
                entry.hit_count += 1;
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.clone());
            }
            // Expired — remove.
            drop(entry);
            self.cache.remove(&sig);
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Store an expansion result.
    pub fn insert(
        &self,
        concepts: impl IntoIterator<Item = String>,
        added: Vec<String>,
        successors: Vec<(String, Vec<String>)>,
    ) {
        if self.cache.len() >= self.max_entries {
            // Simple eviction: remove a random entry.
            if let Some(key) = self.cache.iter().next().map(|e| e.key().clone()) {
                self.cache.remove(&key);
            }
        }
        let sig = ExpansionSignature::new(concepts);
        self.cache
            .insert(sig, ExpanderCacheEntry::new(added, successors));
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

impl Default for SatExpanderCache {
    fn default() -> Self {
        Self::new(50_000, Duration::from_secs(1 * 3600))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_retrieve() {
        let cache = SatExpanderCache::default();
        cache.insert(
            ["A".to_string(), "B".to_string()],
            vec!["C".to_string()],
            vec![],
        );
        let result = cache.get(["B".to_string(), "A".to_string()]); // order shouldn't matter
        assert!(result.is_some());
        assert_eq!(result.unwrap().added_concepts, vec!["C".to_string()]);
    }
}

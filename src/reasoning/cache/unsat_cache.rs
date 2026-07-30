//! Unsatisfiability Cache
//!
//! Caches concept-set signatures known to be unsatisfiable.  When the tableau
//! encounters a node whose concept label matches a cached signature it can
//! immediately close that branch without expansion, dramatically pruning the
//! search space.
//!
//! Inspired by Konclude's `CUnsatisfiableCacheHandler`.
//!
//! # Key design choices
//! - Signatures are computed as a sorted `Vec<String>` of concept names,
//!   then hashed via `FxHasher` for fast lookup.
//! - Thread-safe via `DashMap` (fine-grained bucket locking, no global mutex).
//! - Signatures are **subsumption-closed**: if `{A, B}` is unsatisfiable and
//!   `{A, B, C}` is encountered, it should also be a cache hit because a superset
//!   of unsatisfiable concepts is always unsatisfiable.  To leverage this we
//!   also store minimal signatures (no superfluous concepts) — the lookup does a
//!   subset check against stored minimal signatures.

use dashmap::DashMap;
use std::hash::Hash;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

/// A concept-set signature — a canonically-sorted set of concept names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConceptSignature(Vec<String>);

impl ConceptSignature {
    /// Create a canonical signature from an unordered set of concept names.
    #[must_use]
    pub fn new(concepts: impl IntoIterator<Item = String>) -> Self {
        let mut v: Vec<String> = concepts.into_iter().collect();
        v.sort_unstable();
        v.dedup();
        Self(v)
    }

    /// The concept names in sorted order.
    #[must_use]
    pub fn concepts(&self) -> &[String] {
        &self.0
    }

    /// Check whether `other` is a superset of this signature.
    /// If `self` is unsatisfiable, any superset is also unsatisfiable.
    #[must_use]
    pub fn is_subset_of(&self, other: &ConceptSignature) -> bool {
        if self.0.len() > other.0.len() {
            return false;
        }
        // Both are sorted, so we can do a merge-style check.
        let mut j = 0;
        'outer: for concept in &self.0 {
            while j < other.0.len() {
                match other.0[j].as_str().cmp(concept.as_str()) {
                    std::cmp::Ordering::Equal => {
                        j += 1;
                        continue 'outer;
                    }
                    std::cmp::Ordering::Less => j += 1,
                    std::cmp::Ordering::Greater => return false,
                }
            }
            return false;
        }
        true
    }
}

/// Thread-safe unsatisfiability cache.
#[derive(Debug, Clone)]
pub struct UnsatCache {
    /// Minimal unsatisfiable signatures stored for subset-check lookup.
    cache: Arc<DashMap<ConceptSignature, ()>>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
    entries: Arc<AtomicU64>,
}

impl UnsatCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
            entries: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record that the given concept set is unsatisfiable.
    pub fn record_unsat(&self, concepts: impl IntoIterator<Item = String>) {
        let sig = ConceptSignature::new(concepts);
        if self.cache.insert(sig, ()).is_none() {
            self.entries.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Check whether `concepts` is known to be unsatisfiable.
    ///
    /// Returns `true` if the set — or any of its subsets — is in the cache.
    #[must_use]
    pub fn is_known_unsat(&self, concepts: impl IntoIterator<Item = String>) -> bool {
        let query = ConceptSignature::new(concepts);

        // Exact hit (fast path).
        if self.cache.contains_key(&query) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return true;
        }

        // Subset check: any cached sig that is a subset of the query implies unsat.
        let found = self
            .cache
            .iter()
            .any(|entry| entry.key().is_subset_of(&query));
        if found {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        found
    }

    /// Number of entries in the cache.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.entries.load(Ordering::Relaxed)
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Cache hit rate.
    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        if h + m == 0.0 { 0.0 } else { h / (h + m) }
    }

    /// Reset counters (for benchmarking).
    pub fn reset_stats(&self) {
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Clear all cached entries.
    pub fn clear(&self) {
        self.cache.clear();
        self.entries.store(0, Ordering::Relaxed);
        self.reset_stats();
    }
}

impl Default for UnsatCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_hit() {
        let cache = UnsatCache::new();
        cache.record_unsat(["A".to_string(), "B".to_string()]);
        assert!(cache.is_known_unsat(["A".to_string(), "B".to_string()]));
    }

    #[test]
    fn test_superset_hit() {
        let cache = UnsatCache::new();
        cache.record_unsat(["A".to_string(), "B".to_string()]);
        // {A, B, C} is a superset — should still be unsat.
        assert!(cache.is_known_unsat(["A".to_string(), "B".to_string(), "C".to_string()]));
    }

    #[test]
    fn test_miss() {
        let cache = UnsatCache::new();
        cache.record_unsat(["X".to_string()]);
        assert!(!cache.is_known_unsat(["A".to_string(), "B".to_string()]));
    }

    #[test]
    fn test_subset_check() {
        let s1 = ConceptSignature::new(["A".to_string(), "B".to_string()]);
        let s2 = ConceptSignature::new(["A".to_string(), "B".to_string(), "C".to_string()]);
        assert!(s1.is_subset_of(&s2));
        assert!(!s2.is_subset_of(&s1));
    }
}

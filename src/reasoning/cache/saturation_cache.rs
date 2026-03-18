//! Saturation-Tableau Cache Bridge
//!
//! Links the saturation engine's precomputed subsumptions directly into tableau
//! cache entries.  When the saturation engine has already determined that
//! concept `A ⊑ B`, the tableau can query this cache instead of building a new
//! branch to verify it.

use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A subsumption fact: `sub ⊑ super`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SubsumptionFact {
    pub sub_concept: String,
    pub super_concept: String,
}

/// Thread-safe cache of saturation-derived subsumption facts.
#[derive(Debug, Clone)]
pub struct SaturationCache {
    /// Map from sub-concept to set of known super-concepts.
    cache: Arc<DashMap<String, HashSet<String>>>,
    /// Map from super-concept to set of known sub-concepts (inverted index).
    inverted: Arc<DashMap<String, HashSet<String>>>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl SaturationCache {
    #[must_use]
    pub fn new() -> Self {
        Self {
            cache: Arc::new(DashMap::new()),
            inverted: Arc::new(DashMap::new()),
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record a subsumption `sub ⊑ super` derived by saturation.
    pub fn record_subsumption(&self, sub: String, sup: String) {
        self.cache.entry(sub.clone()).or_default().insert(sup.clone());
        self.inverted.entry(sup).or_default().insert(sub);
    }

    /// Record multiple subsumptions at once (e.g. a full saturation result).
    pub fn record_many(&self, facts: impl IntoIterator<Item = SubsumptionFact>) {
        for fact in facts {
            self.record_subsumption(fact.sub_concept, fact.super_concept);
        }
    }

    /// Check if `sub ⊑ super` is known from saturation.
    #[must_use]
    pub fn is_subsumed(&self, sub: &str, sup: &str) -> bool {
        let result = self
            .cache
            .get(sub)
            .map_or(false, |supers| supers.contains(sup));
        if result {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        result
    }

    /// Get all known super-concepts of `sub`.
    #[must_use]
    pub fn super_concepts(&self, sub: &str) -> Option<HashSet<String>> {
        self.cache.get(sub).map(|v| v.clone())
    }

    /// Get all known sub-concepts of `sup`.
    #[must_use]
    pub fn sub_concepts(&self, sup: &str) -> Option<HashSet<String>> {
        self.inverted.get(sup).map(|v| v.clone())
    }

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        if h + m == 0.0 { 0.0 } else { h / (h + m) }
    }

    #[must_use]
    pub fn total_facts(&self) -> usize {
        self.cache.iter().map(|e| e.value().len()).sum()
    }
}

impl Default for SaturationCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsumption_hit() {
        let cache = SaturationCache::new();
        cache.record_subsumption("Dog".to_string(), "Animal".to_string());
        assert!(cache.is_subsumed("Dog", "Animal"));
        assert!(!cache.is_subsumed("Animal", "Dog"));
    }

    #[test]
    fn test_super_concepts() {
        let cache = SaturationCache::new();
        cache.record_subsumption("Dog".to_string(), "Animal".to_string());
        cache.record_subsumption("Dog".to_string(), "Pet".to_string());
        let supers = cache.super_concepts("Dog").unwrap();
        assert!(supers.contains("Animal"));
        assert!(supers.contains("Pet"));
    }
}

//! Computed Consequences Cache
//!
//! Caches computed reasoning consequences — subsumptions, instances, and
//! equivalent classes — across reasoning tasks so that repeated queries
//! return immediately without re-running the tableau.

use dashmap::DashMap;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

/// A cached set of class expressions (stored as IRIs / Manchester strings).
#[derive(Debug, Clone)]
pub struct ClassSet {
    pub classes: HashSet<String>,
    pub computed_at: Instant,
}

/// A cached set of individual IRIs.
#[derive(Debug, Clone)]
pub struct IndividualSet {
    pub individuals: HashSet<String>,
    pub computed_at: Instant,
}

/// Thread-safe consequences cache with per-entry TTL.
#[derive(Debug, Clone)]
pub struct ConsequencesCache {
    /// subclasses: concept_iri × direct → set of subclass IRIs.
    subclasses: Arc<DashMap<(String, bool), ClassSet>>,
    /// superclasses: concept_iri × direct → set.
    superclasses: Arc<DashMap<(String, bool), ClassSet>>,
    /// equivalent classes: concept_iri → set.
    equivalent: Arc<DashMap<String, ClassSet>>,
    /// instances: concept_iri × direct → set.
    instances: Arc<DashMap<(String, bool), IndividualSet>>,
    /// types of an individual: individual_iri × direct → set.
    types: Arc<DashMap<(String, bool), ClassSet>>,
    ttl: Duration,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl ConsequencesCache {
    #[must_use]
    pub fn new(ttl: Duration) -> Self {
        Self {
            subclasses: Arc::new(DashMap::new()),
            superclasses: Arc::new(DashMap::new()),
            equivalent: Arc::new(DashMap::new()),
            instances: Arc::new(DashMap::new()),
            types: Arc::new(DashMap::new()),
            ttl,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    // ── Subclasses ────────────────────────────────────────────────────────

    pub fn cache_subclasses(&self, class: &str, direct: bool, subs: HashSet<String>) {
        self.subclasses.insert(
            (class.to_string(), direct),
            ClassSet {
                classes: subs,
                computed_at: Instant::now(),
            },
        );
    }

    #[must_use]
    pub fn get_subclasses(&self, class: &str, direct: bool) -> Option<HashSet<String>> {
        self.get_class_set(&self.subclasses, &(class.to_string(), direct))
    }

    // ── Superclasses ──────────────────────────────────────────────────────

    pub fn cache_superclasses(&self, class: &str, direct: bool, supers: HashSet<String>) {
        self.superclasses.insert(
            (class.to_string(), direct),
            ClassSet {
                classes: supers,
                computed_at: Instant::now(),
            },
        );
    }

    #[must_use]
    pub fn get_superclasses(&self, class: &str, direct: bool) -> Option<HashSet<String>> {
        self.get_class_set(&self.superclasses, &(class.to_string(), direct))
    }

    // ── Equivalent classes ────────────────────────────────────────────────

    pub fn cache_equivalent(&self, class: &str, equivs: HashSet<String>) {
        self.equivalent.insert(
            class.to_string(),
            ClassSet {
                classes: equivs,
                computed_at: Instant::now(),
            },
        );
    }

    #[must_use]
    pub fn get_equivalent(&self, class: &str) -> Option<HashSet<String>> {
        let key = class.to_string();
        match self.equivalent.get(&key) {
            Some(entry) if entry.computed_at.elapsed() < self.ttl => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.classes.clone())
            }
            Some(_) => {
                drop(self.equivalent.remove(&key));
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    // ── Instances ─────────────────────────────────────────────────────────

    pub fn cache_instances(&self, class: &str, direct: bool, inds: HashSet<String>) {
        self.instances.insert(
            (class.to_string(), direct),
            IndividualSet {
                individuals: inds,
                computed_at: Instant::now(),
            },
        );
    }

    #[must_use]
    pub fn get_instances(&self, class: &str, direct: bool) -> Option<HashSet<String>> {
        let key = (class.to_string(), direct);
        match self.instances.get(&key) {
            Some(entry) if entry.computed_at.elapsed() < self.ttl => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.individuals.clone())
            }
            Some(_) => {
                drop(self.instances.remove(&key));
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }

    // ── Types ─────────────────────────────────────────────────────────────

    pub fn cache_types(&self, individual: &str, direct: bool, types: HashSet<String>) {
        self.types.insert(
            (individual.to_string(), direct),
            ClassSet {
                classes: types,
                computed_at: Instant::now(),
            },
        );
    }

    #[must_use]
    pub fn get_types(&self, individual: &str, direct: bool) -> Option<HashSet<String>> {
        self.get_class_set(&self.types, &(individual.to_string(), direct))
    }

    // ── Invalidation ──────────────────────────────────────────────────────

    /// Invalidate all cached consequences (e.g. after axiom changes).
    pub fn invalidate_all(&self) {
        self.subclasses.clear();
        self.superclasses.clear();
        self.equivalent.clear();
        self.instances.clear();
        self.types.clear();
        self.hits.store(0, Ordering::Relaxed);
        self.misses.store(0, Ordering::Relaxed);
    }

    /// Invalidate consequences involving the given class.
    pub fn invalidate_class(&self, class: &str) {
        self.subclasses.retain(|(k, _), _| k != class);
        self.superclasses.retain(|(k, _), _| k != class);
        self.equivalent.remove(class);
        self.instances.retain(|(k, _), _| k != class);
        // Note: types are keyed by individual, leave them for now.
    }

    // ── Statistics ────────────────────────────────────────────────────────

    #[must_use]
    pub fn hit_rate(&self) -> f64 {
        let h = self.hits.load(Ordering::Relaxed) as f64;
        let m = self.misses.load(Ordering::Relaxed) as f64;
        if h + m == 0.0 { 0.0 } else { h / (h + m) }
    }

    // ── Private helpers ───────────────────────────────────────────────────

    fn get_class_set(
        &self,
        map: &DashMap<(String, bool), ClassSet>,
        key: &(String, bool),
    ) -> Option<HashSet<String>> {
        match map.get(key) {
            Some(entry) if entry.computed_at.elapsed() < self.ttl => {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Some(entry.classes.clone())
            }
            Some(_) => {
                drop(map.remove(key));
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
            None => {
                self.misses.fetch_add(1, Ordering::Relaxed);
                None
            }
        }
    }
}

impl Default for ConsequencesCache {
    fn default() -> Self {
        Self::new(Duration::from_secs(3600))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subclasses_roundtrip() {
        let cache = ConsequencesCache::default();
        let mut subs = HashSet::new();
        subs.insert("Dog".to_string());
        subs.insert("Cat".to_string());
        cache.cache_subclasses("Animal", false, subs.clone());
        let retrieved = cache.get_subclasses("Animal", false);
        assert_eq!(retrieved.unwrap(), subs);
    }

    #[test]
    fn test_invalidation() {
        let cache = ConsequencesCache::default();
        let mut subs = HashSet::new();
        subs.insert("Dog".to_string());
        cache.cache_subclasses("Animal", true, subs);
        cache.invalidate_class("Animal");
        assert!(cache.get_subclasses("Animal", true).is_none());
    }
}

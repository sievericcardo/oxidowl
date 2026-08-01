//! Multi-Level Tableau Caching
//!
//! This module aggregates all tableau-level cache layers described in Phase 2
//! of the Konclude-inspired feature implementation plan:
//!
//! | Layer | File | Purpose |
//! |---|---|---|
//! | `UnsatCache` | `unsat_cache.rs` | Prune branches hitting known-unsat signatures |
//! | `SatExpanderCache` | `sat_expander_cache.rs` | Reuse node expansion results |
//! | `CompletionGraphCache` | `completion_graph_cache.rs` | Reuse full completion graph summaries |
//! | `SaturationCache` | `saturation_cache.rs` | Bridge saturation engine → tableau |
//! | `ConsequencesCache` | `consequences_cache.rs` | Cross-task consequence caching |

pub mod completion_graph_cache;
pub mod consequences_cache;
pub mod sat_expander_cache;
pub mod saturation_cache;
pub mod unsat_cache;

pub use completion_graph_cache::{CompletionGraphCache, CompletionGraphSummary, GraphCacheKey};
pub use consequences_cache::ConsequencesCache;
pub use sat_expander_cache::{ExpanderCacheEntry, ExpansionSignature, SatExpanderCache};
pub use saturation_cache::{SaturationCache, SubsumptionFact};
pub use unsat_cache::{ConceptSignature, UnsatCache};

use std::time::Duration;

/// Aggregated statistics across all cache layers.
#[derive(Debug, Clone, Default)]
pub struct CacheLayerStats {
    pub unsat_cache_hit_rate: f64,
    pub expander_cache_hit_rate: f64,
    pub graph_cache_hit_rate: f64,
    pub saturation_cache_hit_rate: f64,
    pub consequences_cache_hit_rate: f64,
    pub unsat_entries: u64,
    pub expander_entries: usize,
    pub graph_entries: usize,
    pub saturation_facts: usize,
}

/// Unified multi-level cache manager holding all cache layers.
///
/// All layers are reference-counted so the manager is cheaply cloneable.
#[derive(Debug, Clone)]
pub struct MultiLevelCacheManager {
    pub unsat: UnsatCache,
    pub expander: SatExpanderCache,
    pub completion_graph: CompletionGraphCache,
    pub saturation: SaturationCache,
    pub consequences: ConsequencesCache,
}

impl MultiLevelCacheManager {
    /// Create a new manager with reasonable defaults.
    #[must_use]
    pub fn new() -> Self {
        Self {
            unsat: UnsatCache::new(),
            expander: SatExpanderCache::new(50_000, Duration::from_hours(1)),
            completion_graph: CompletionGraphCache::new(10_000, Duration::from_hours(1)),
            saturation: SaturationCache::new(),
            consequences: ConsequencesCache::new(Duration::from_hours(1)),
        }
    }

    /// Collect aggregated statistics.
    #[must_use]
    pub fn stats(&self) -> CacheLayerStats {
        CacheLayerStats {
            unsat_cache_hit_rate: self.unsat.hit_rate(),
            expander_cache_hit_rate: self.expander.hit_rate(),
            graph_cache_hit_rate: self.completion_graph.hit_rate(),
            saturation_cache_hit_rate: self.saturation.hit_rate(),
            consequences_cache_hit_rate: self.consequences.hit_rate(),
            unsat_entries: self.unsat.len(),
            expander_entries: self.expander.len(),
            graph_entries: self.completion_graph.len(),
            saturation_facts: self.saturation.total_facts(),
        }
    }

    /// Invalidate all caches (e.g. on ontology change).
    pub fn invalidate_all(&self) {
        // UnsatCache and SaturationCache entries remain valid across minor ontology
        // changes (they only grow more conservative), but we clear them on major changes.
        // ConsequencesCache and expander/graph caches are sensitive to new axioms.
        self.consequences.invalidate_all();
        // Note: unsat/saturation caches are additive — do not clear by default.
    }

    /// Full reset of every cache layer (used after axiom removal).
    pub fn reset_all(&self) {
        self.unsat.clear();
        self.expander.clear();
        self.completion_graph.clear();
        self.saturation.clear();
        self.consequences.invalidate_all();
    }
}

impl Default for MultiLevelCacheManager {
    fn default() -> Self {
        Self::new()
    }
}

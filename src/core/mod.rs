//! Core reasoning components
//!
//! This module contains the main reasoning engine, including the tableau algorithm
//! implementation, reasoning tasks, and result management.

pub mod blocking;
pub mod completion;
pub mod completion_cache;
pub mod dependency;
pub mod expansion;
pub mod fast_hashing;
pub mod hypergraph;
pub mod incremental;
pub mod inverted_index;
pub mod lock_helpers;
pub mod persistent_collections;
pub mod reasoner;
pub mod saturation;
pub mod tableau;

pub use blocking::{BlockingChecker, BlockingStrategy};
pub use completion::{CompletionRule, RuleApplication};
pub use completion_cache::{
    CacheStatistics as CompletionCacheStats, CompletionGraph, CompletionGraphCache,
};
pub use dependency::{DependencySet, DependencyTracker};
pub use expansion::{
    BreadthFirstExpansionStrategy, ComplexityStrategy, CreationOrderStrategy,
    DepthFirstExpansionStrategy, ExpansionManager, ExpansionStrategy, HeuristicExpansionStrategy,
    PriorityBasedExpansionStrategy,
};
pub use fast_hashing::{FastConceptHasher, compute_fast_signature, hash_concept};
pub use hypergraph::{
    EdgeType, HyperEdge, HyperNode, Hypergraph, NodeId, NodeSignature,
    expansion::{ExpansionState, ExpansionStatistics, HypertableauExpansion},
};
pub use incremental::{
    DependencyTracker as ConceptDependencyTracker, IncrementalClassifier, IncrementalStatistics,
};
pub use inverted_index::{ConceptIndex, IndexStatistics};
pub use lock_helpers::{mutex_lock, read_lock, write_lock};
pub use persistent_collections::{ConceptSet, ConceptSetPool, ConceptSetPoolStats};
pub use reasoner::Reasoner;
pub use saturation::{
    SaturationConfig, SaturationEngine, SaturationNode, SaturationResult, SaturationStatus,
};
pub use tableau::{Tableau, TableauState};

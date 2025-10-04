//! Core reasoning components
//!
//! This module contains the main reasoning engine, including the tableau algorithm
//! implementation, reasoning tasks, and result management.

pub mod blocking;
pub mod completion;
pub mod dependency;
pub mod expansion;
pub mod hypergraph;
pub mod reasoner;
pub mod tableau;

pub use blocking::{BlockingChecker, BlockingStrategy};
pub use completion::{CompletionRule, RuleApplication};
pub use dependency::{DependencySet, DependencyTracker};
pub use expansion::{
    BreadthFirstExpansionStrategy, ComplexityStrategy, CreationOrderStrategy,
    DepthFirstExpansionStrategy, ExpansionManager, ExpansionStrategy, HeuristicExpansionStrategy,
    PriorityBasedExpansionStrategy,
};
pub use hypergraph::{
    Hypergraph, HyperNode, HyperEdge, NodeId, EdgeType, NodeSignature,
    expansion::{HypertableauExpansion, ExpansionState, ExpansionStatistics},
};
pub use reasoner::Reasoner;
pub use tableau::{Tableau, TableauState};

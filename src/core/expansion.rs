//! Existential expansion strategies and management
//!
//! This module implements expansion strategies for managing how existential
//! concepts are expanded in the tableau, based on the sophisticated expansion
//! management systems from Konclude, HermiT, and Pellet.

use crate::{
    core::{
        completion::{CompletionRule, RuleApplication, RuleContext, RulePriority},
        dependency::{DependencySet, DependencyTracker, DependencyType},
    },
    ontology::{ClassExpression, Individual, Role, ObjectPropertyExpression},
    Error, Result,
};
use std::{
    collections::{HashMap, HashSet, VecDeque, BinaryHeap},
    cmp::Ordering,
    fmt,
};

/// Strategy for expanding existential concepts in the tableau.
pub trait ExpansionStrategy: fmt::Debug + Send + Sync {
    /// Initialise the strategy with tableau context
    fn initialise(&mut self, context: &ExpansionContext) -> Result<()>;

    /// Select the next existential concept to expand
    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate>;

    /// Determine expansion order for multiple existentials
    fn order_expansions(
        &mut self,
        existentials: &[ExistentialCandidate],
    );

    /// Check if expansion should be delayed
    fn should_delay_expansion(
        &self,
        candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> bool;

    /// Get expansion priority for an existential candidate
    fn get_expansion_priority(
        &self,
        candidate: &ExistentialCandidate,
    );

    /// Notify about completed expansion
    fn expansion_completed(
        &mut self,
        candidate: &ExistentialCandidate,
        result: &ExpansionResult,
    );

    /// Clear the strategy state
    fn clear(&mut self);
}

/// Manager for coordinating existential expansions
#[derive(Debug)]
pub struct ExpansionManager {
    /// Current expansion strategy
    strategy: Box<dyn ExpansionStrategy>,

    /// Queue of pending existential candidates
    pending_queue: BinaryHeap<PrioritisedCandidate>,

    /// Currently expanding candidates
    expanding: HashSet<String>,

    /// Expansion history for optimization
    expansion_history: HashMap<String, ExpansionRecord>,

    /// Dependency tracker
    dependency_tracker: DependencyTracker,

    /// Configuration options
    config: ExistentialConfig,

    /// Statistics
    statistics: ExpansionStatistics,
}

/// Context information for expansion decisions
#[derive(Debug, Clone)]
pub struct ExpansionContext {
    /// Current tableau size
    pub tableau_size: usize,

    /// Current branching depth
    pub branching_depth: u32,

    /// Available memory
    pub available_memory: usize,

    /// Time elapsed since start
    pub elapsed_time: std::time::Duration,

    /// Active individuals
    pub active_individuals: HashSet<String>,

    /// Role hierarchy
    pub role_hierarchy: HashMap<Role, Vec<Role>>,

    /// Concept hierarchy
    pub concept_hierarchy: HashMap<String, Vec<String>>,
}

/// Candidate existential for expansion
#[derive(Debug, Clone)]
pub struct ExistentialCandidate {
    /// Node containing the existential
    pub node: String,

    /// The existential concept to expand
    pub concept: ClassExpression,

    /// Associated role for the existential
    pub role: ObjectPropertyExpression,

    /// Dependencies for this candidate
    pub dependencies: DependencySet,

    /// Potential witness
    pub potential_witness: Vec<String>,

    /// Expansion complexity estimate
    pub complexity: ExpansionComplexity,

    /// Creation timestamp
    pub created_at: std::time::Instant,
}

/// Expansion complexity metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionComplexity {
    /// Complexity of filler
    pub syntactic_complexity: u32,

    /// Number of role successors needed
    pub role_successors: u32,

    /// Estimated branching factor
    pub branching_factor: u32,

    /// Memory requirement estimate
    pub memory_estimate: usize,
}

/// Priority for expansion ordering
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExpansionPriority{
    /// Immediate expansion required
    Immediate = 0,

    /// High priority expansion
    High = 1,

    /// Normal priority expansion
    Normal = 2,

    /// Low priority expansion
    Low = 3,

    /// Delayed expansion
    Delayed = 4,
}

/// Wrapper for priority-based ordering
#[derive(Debug, Clone)]
struct PrioritzedCandidate {
    candidate: ExistentialCandidate,
    priority: ExpansionPriority,
    insertion_order: u64,
}

/// Result of an expansion operation
#[derive(Debug, Clone)]
pub struct ExpansionResult {
    /// New individuals created
    pub new_individuals: Vec<String>,

    /// New edges created
    pub new_edges: Vec<(String, String, Role)>,

    /// New concepts added
    pub new_concepts: Vec<(String, ClassExpression)>,

    /// Rule applications generated
    pub rule_applications: Vec<RuleApplication>,

    /// Expansion successful
    pub success: bool,

    /// Dependencies generated during expansion
    pub dependencies: DependencySet,
}
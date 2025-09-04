//! Existential expansion strategies and management
//!
//! This module implements expansion strategies for managing how existential
//! concepts are expanded in the tableau, based on the sophisticated expansion
//! management systems from Konclude, `HermiT`, and Pellet.

use crate::{
    Error, Result,
    core::{
        completion::{CompletionRule, RuleApplication},
        dependency::{DependencySet, DependencyTracker},
    },
    ontology::{ClassExpression, ObjectPropertyExpression, Role},
};

use std::{
    cmp::Ordering,
    collections::{BinaryHeap, HashMap, HashSet},
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
    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]);

    /// Check if expansion should be delayed
    fn should_delay_expansion(
        &self,
        candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> bool;

    /// Get expansion priority for an existential candidate
    fn get_expansion_priority(&self, candidate: &ExistentialCandidate) -> ExpansionPriority;

    /// Notify about completed expansion
    fn expansion_completed(&mut self, candidate: &ExistentialCandidate, result: &ExpansionResult);

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
    config: ExpansionConfig,

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
    pub existential: ClassExpression,

    /// Associated role for the existential
    pub role: ObjectPropertyExpression,

    /// The filler concept
    pub filler: ClassExpression,

    /// Dependencies for this candidate
    pub dependencies: DependencySet,

    /// Potential witness
    pub potential_witnesses: Vec<String>,

    /// Expansion complexity estimate
    pub complexity: ExpansionComplexity,

    /// Creation timestamp
    pub created_at: std::time::Instant,
}

/// Expansion complexity metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionComplexity {
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
pub enum ExpansionPriority {
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
struct PrioritisedCandidate {
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

/// Record of a completed expansion
#[derive(Debug, Clone)]
pub struct ExpansionRecord {
    /// Candidate that was expanded
    pub candidate: ExistentialCandidate,

    /// Result of the expansion
    pub result: ExpansionResult,

    /// Timestamp when expansion was completed
    pub timestamp: std::time::Instant,

    /// Expansion strategy used
    pub strategy_used: String,
}

/// Configuration for expansion management
#[derive(Debug, Clone)]
pub struct ExpansionConfig {
    /// Maximum pending queue size
    pub max_queue_size: usize,

    /// Enable expansion caching
    pub enable_caching: bool,

    /// Delay complex expansions
    pub delay_complex: bool,

    /// Complexity threshold for delaying
    pub complexity_threshold: u32,

    /// Maximum expansion depth
    pub max_expansion_depth: u32,

    /// Prefer witnesses over new individuals
    pub prefer_witnesses: bool,
}

/// Statistics about expansion operations
#[derive(Debug, Default, Clone)]
pub struct ExpansionStatistics {
    /// Total expansions performed
    pub total_expansions: u64,

    /// Expansion using witnesses
    pub witness_expansions: u64,

    /// New individuals creations
    pub new_individuals_creations: u64,

    /// Expansions delayed
    pub delayed_expansions: u64,

    /// Average expansion time
    pub average_expansion_time: std::time::Duration,

    /// Total expansion time
    pub total_expansion_time: std::time::Duration,
}

/// Creation order strategy
#[derive(Debug)]
pub struct CreationOrderStrategy {
    /// Current insertion order for candidates
    insertion_order: u64,
}

/// Complexity-based strategy
#[derive(Debug)]
pub struct ComplexityStrategy {
    /// Weight factors for complexity metrics
    weights: ComplexityWeights,
}

/// Role depth strategy
#[derive(Debug)]
pub struct RoleDepthStrategy {
    /// Role depth cache
    role_depths: HashMap<Role, u32>,

    /// Prefer shallow or deep roles
    prefer_shallow: bool,
}

/// Hybrid strategy combining multiple approaches
#[derive(Debug)]
pub struct HybridStrategy {
    /// Primary strategy
    primary: Box<dyn ExpansionStrategy>,

    /// Fallback strategy
    fallbacks: Vec<Box<dyn ExpansionStrategy>>,

    /// Strategy selection criteria
    selection_criteria: StrategySelectionCriteria,
}

/// Weights for complexity metrics
#[derive(Debug, Clone)]
pub struct ComplexityWeights {
    /// Weight for syntactic complexity
    pub syntactic: f64,

    /// Weight for role successors
    pub role_successors: f64,

    /// Weight for branching factor
    pub branching_factor: f64,

    /// Weight for memory estimate
    pub memory_estimate: f64,
}

/// Criteria for selecting expansion strategies
#[derive(Debug, Clone)]
pub struct StrategySelectionCriteria {
    /// Tableau size thresholds
    pub size_thresholds: Vec<usize>,

    /// Complexity thresholds
    pub complexity_thresholds: Vec<u32>,

    /// Time-based switching
    pub time_based_switching: bool,
}

impl ExpansionManager {
    /// Create a new expansion manager with the given strategy
    #[must_use]
    pub fn new(strategy: Box<dyn ExpansionStrategy>) -> Self {
        Self {
            strategy,
            pending_queue: BinaryHeap::new(),
            expanding: HashSet::new(),
            expansion_history: HashMap::new(),
            dependency_tracker: DependencyTracker::new(),
            config: ExpansionConfig::default(),
            statistics: ExpansionStatistics::default(),
        }
    }

    /// Add an existential candidate to the pending queue
    pub fn add_candidate(&mut self, candidate: ExistentialCandidate) -> Result<()> {
        if self.pending_queue.len() >= self.config.max_queue_size {
            return Err(Error::QueueFull);
        }

        let priority = self.strategy.get_expansion_priority(&candidate);
        let prioritised_candidate = PrioritisedCandidate {
            candidate,
            priority,
            insertion_order: self.next_insertion_order(),
        };

        self.pending_queue.push(prioritised_candidate);

        Ok(())
    }

    /// Get the next existential to expand
    pub fn next_expansion(&mut self, context: &ExpansionContext) -> Option<ExistentialCandidate> {
        while let Some(prioritised) = self.pending_queue.pop() {
            if self.expanding.contains(&prioritised.candidate.node) {
                continue; // Already expanding this candidate
            }

            if self
                .strategy
                .should_delay_expansion(&prioritised.candidate, context)
            {
                // Re-queue with lower priority
                let delayed = PrioritisedCandidate {
                    candidate: prioritised.candidate,
                    priority: ExpansionPriority::Delayed,
                    insertion_order: self.next_insertion_order(),
                };
                self.pending_queue.push(delayed); // Reinsert for later
                self.statistics.delayed_expansions += 1;
                continue;
            }

            self.expanding.insert(prioritised.candidate.node.clone());
            return Some(prioritised.candidate);
        }

        None // No candidates available
    }

    /// Expand an existential candidate
    pub fn expand_candidate(
        &mut self,
        candidate: ExistentialCandidate,
        context: &ExpansionContext,
    ) -> Result<ExpansionResult> {
        let start_time = std::time::Instant::now();

        let result = if self.config.prefer_witnesses && !candidate.potential_witnesses.is_empty() {
            self.expand_with_witness(&candidate, context)?
        } else {
            self.expand_with_new_individual(&candidate, context)?
        };

        // Update statistics
        let expansion_time = start_time.elapsed();
        self.statistics.total_expansions += 1;
        self.statistics.total_expansion_time += expansion_time;
        self.statistics.average_expansion_time =
            self.statistics.total_expansion_time / self.statistics.total_expansions as u32;

        if result.new_individuals.is_empty() {
            self.statistics.witness_expansions += 1;
        } else {
            self.statistics.new_individuals_creations += 1;
        }

        // Record expansion
        let record = ExpansionRecord {
            candidate: candidate.clone(),
            result: result.clone(),
            timestamp: start_time,
            strategy_used: self.strategy_name(),
        };
        self.expansion_history
            .insert(candidate.node.clone(), record);

        // Notify strategy
        self.strategy.expansion_completed(&candidate, &result);

        // Remove from expanding set
        self.expanding.remove(&candidate.node);

        Ok(result)
    }

    /// Expand using an existing witness
    fn expand_with_witness(
        &self,
        candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> Result<ExpansionResult> {
        let witness = candidate.potential_witnesses[0].clone(); // Use first witness

        let mut result = ExpansionResult {
            new_individuals: Vec::new(),
            new_edges: vec![(
                candidate.node.clone(),
                witness.clone(),
                Role::ObjectProperty(candidate.role.clone()),
            )],
            new_concepts: vec![(witness, candidate.filler.clone())],
            rule_applications: Vec::new(),
            success: true,
            dependencies: candidate.dependencies.clone(),
        };

        // Create rule application for adding filler concept
        let rule_app = RuleApplication::concept(
            CompletionRule::Some,
            candidate.node.clone(),
            candidate.existential.clone(),
            candidate.dependencies.clone(),
        );
        result.rule_applications.push(rule_app);

        Ok(result)
    }

    /// Expand by creating a new individual
    fn expand_with_new_individual(
        &self,
        candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> Result<ExpansionResult> {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let new_individual = format!("_exist_{}_{}", candidate.node, &uuid_str[..8]);

        let mut result = ExpansionResult {
            new_individuals: vec![new_individual.clone()],
            new_edges: vec![(
                candidate.node.clone(),
                new_individual.clone(),
                Role::ObjectProperty(candidate.role.clone()),
            )],
            new_concepts: vec![(new_individual, candidate.filler.clone())],
            rule_applications: Vec::new(),
            success: true,
            dependencies: candidate.dependencies.clone(),
        };

        // Create rule application
        let rule_app = RuleApplication::concept(
            CompletionRule::Some,
            candidate.node.clone(),
            candidate.existential.clone(),
            candidate.dependencies.clone(),
        );
        result.rule_applications.push(rule_app);

        Ok(result)
    }

    /// Check if there are pending expansions
    #[must_use]
    pub fn has_pending_expansions(&self) -> bool {
        !self.pending_queue.is_empty() || !self.expanding.is_empty()
    }

    /// Get pending expansion count
    #[must_use]
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
    }

    /// Clear all pending expansions
    pub fn clear_pending(&mut self) {
        self.pending_queue.clear();
        self.expanding.clear();
        self.expansion_history.clear();
        self.statistics = ExpansionStatistics::default();
    }

    /// Get expansion statistics
    #[must_use]
    pub fn statistics(&self) -> &ExpansionStatistics {
        &self.statistics
    }

    /// Set expansion strategy
    pub fn set_strategy(&mut self, strategy: Box<dyn ExpansionStrategy>) {
        self.strategy = strategy;
    }

    /// Get next insertion order number
    fn next_insertion_order(&mut self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64
    }

    /// Get strategy name for recording
    fn strategy_name(&self) -> String {
        format!("{:?}", self.strategy)
    }
}

impl CreationOrderStrategy {
    /// Create a new creation order strategy
    #[must_use]
    pub fn new() -> Self {
        Self { insertion_order: 0 }
    }
}

impl ExpansionStrategy for CreationOrderStrategy {
    fn initialise(&mut self, _context: &ExpansionContext) -> Result<()> {
        self.insertion_order = 0;
        Ok(())
    }

    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate> {
        candidates.first().cloned()
    }

    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]) {
        existentials.sort_by_key(|e| e.created_at);
    }

    fn should_delay_expansion(
        &self,
        _candidate: &ExistentialCandidate,
        _context: &ExpansionContext,
    ) -> bool {
        false // Never delay in creation order strategy
    }

    fn get_expansion_priority(&self, _candidate: &ExistentialCandidate) -> ExpansionPriority {
        ExpansionPriority::Normal // Default priority
    }

    fn expansion_completed(
        &mut self,
        _candidate: &ExistentialCandidate,
        _result: &ExpansionResult,
    ) {
        self.insertion_order += 1; // Increment insertion order on completion
    }

    fn clear(&mut self) {
        self.insertion_order = 0; // Reset insertion order
    }
}

impl ComplexityStrategy {
    /// Create a new complexity-based strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            weights: ComplexityWeights::default(),
        }
    }

    /// Calculate complexity score
    fn calculate_complexity_score(&self, candidate: &ExistentialCandidate) -> f64 {
        let complexity = &candidate.complexity;

        self.weights.syntactic * f64::from(complexity.syntactic_complexity)
            + self.weights.role_successors * f64::from(complexity.role_successors)
            + self.weights.branching_factor * f64::from(complexity.branching_factor)
            + self.weights.memory_estimate * complexity.memory_estimate as f64
    }
}

impl ExpansionStrategy for ComplexityStrategy {
    fn initialise(&mut self, _context: &ExpansionContext) -> Result<()> {
        Ok(())
    }

    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate> {
        candidates
            .iter()
            .min_by(|a, b| {
                self.calculate_complexity_score(a)
                    .partial_cmp(&self.calculate_complexity_score(b))
                    .unwrap_or(Ordering::Equal)
            })
            .cloned()
    }

    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]) {
        existentials.sort_by(|a, b| {
            self.calculate_complexity_score(b)
                .partial_cmp(&self.calculate_complexity_score(a))
                .unwrap_or(Ordering::Equal)
        });
    }

    fn should_delay_expansion(
        &self,
        candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> bool {
        candidate.complexity.syntactic_complexity > 10 // reasonable default threshold
            && context.branching_depth > 100 // reasonable default max depth
    }

    fn get_expansion_priority(&self, candidate: &ExistentialCandidate) -> ExpansionPriority {
        let score = self.calculate_complexity_score(candidate);

        if score < 5.0 {
            ExpansionPriority::High
        } else if score < 15.0 {
            ExpansionPriority::Normal
        } else {
            ExpansionPriority::Low
        }
    }

    fn expansion_completed(
        &mut self,
        _candidate: &ExistentialCandidate,
        _result: &ExpansionResult,
    ) {
        // No specific action needed on completion
    }

    fn clear(&mut self) {
        // No state to clear in this strategy
    }
}

impl ExistentialCandidate {
    /// Create a new existential candidate
    pub fn new(
        node: String,
        existential: ClassExpression,
        dependencies: DependencySet,
    ) -> Result<Self> {
        let (role, filler) = match &existential {
            ClassExpression::ObjectSomeValuesFrom {
                property,
                filler: f,
            } => (property.clone(), (**f).clone()),
            _ => return Err(Error::internal("Not an existential concept")),
        };

        let complexity = Self::calculate_complexity(&existential);

        Ok(Self {
            node,
            existential,
            role,
            filler,
            dependencies,
            potential_witnesses: Vec::new(),
            complexity,
            created_at: std::time::Instant::now(),
        })
    }

    /// Calculate expansion complexity
    fn calculate_complexity(existential: &ClassExpression) -> ExpansionComplexity {
        let syntactic_complexity = Self::syntactic_complexity(existential);

        ExpansionComplexity {
            syntactic_complexity,
            role_successors: 1, // Basic estimation
            branching_factor: if syntactic_complexity > 5 { 2 } else { 1 },
            memory_estimate: syntactic_complexity as usize * 100,
        }
    }

    /// Calculate syntactic complexity of a concept
    fn syntactic_complexity(concept: &ClassExpression) -> u32 {
        match concept {
            ClassExpression::Class(_) => 1,
            ClassExpression::ObjectIntersectionOf(operands) => {
                1 + operands.iter().map(Self::syntactic_complexity).sum::<u32>()
            }
            ClassExpression::ObjectUnionOf(operands) => {
                2 + operands.iter().map(Self::syntactic_complexity).sum::<u32>()
            }
            ClassExpression::ObjectSomeValuesFrom { filler, .. } => {
                2 + Self::syntactic_complexity(filler)
            }
            ClassExpression::ObjectAllValuesFrom { filler, .. } => {
                2 + Self::syntactic_complexity(filler)
            }
            ClassExpression::ObjectMinCardinality { filler, .. } => {
                3 + Self::syntactic_complexity(filler)
            }
            ClassExpression::ObjectMaxCardinality { filler, .. } => {
                3 + Self::syntactic_complexity(filler)
            }
            ClassExpression::ObjectComplementOf(inner) => 1 + Self::syntactic_complexity(inner),
            _ => 1,
        }
    }

    /// Add a potential witness to this candidate
    pub fn add_witness(&mut self, witness: String) {
        if !self.potential_witnesses.contains(&witness) {
            self.potential_witnesses.push(witness);
        }
    }
}

impl Ord for PrioritisedCandidate {
    fn cmp(&self, other: &Self) -> Ordering {
        // Lower priority values are higher priority (reverse order)
        other
            .priority
            .cmp(&self.priority)
            .then_with(|| self.insertion_order.cmp(&other.insertion_order))
    }
}

impl PartialOrd for PrioritisedCandidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for PrioritisedCandidate {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.insertion_order == other.insertion_order
    }
}

impl Eq for PrioritisedCandidate {}

impl Default for ExpansionConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 10000,
            enable_caching: true,
            delay_complex: true,
            complexity_threshold: 10,
            max_expansion_depth: 100,
            prefer_witnesses: true,
        }
    }
}

impl Default for ComplexityWeights {
    fn default() -> Self {
        Self {
            syntactic: 1.0,
            role_successors: 2.0,
            branching_factor: 3.0,
            memory_estimate: 0.1,
        }
    }
}

impl Default for CreationOrderStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for ComplexityStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Breadth-first expansion strategy
#[derive(Debug)]
pub struct BreadthFirstExpansionStrategy {
    /// Depth tracking for breadth-first ordering
    depth_map: HashMap<String, u32>,
}

impl BreadthFirstExpansionStrategy {
    /// Create a new breadth-first strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            depth_map: HashMap::new(),
        }
    }
}

impl ExpansionStrategy for BreadthFirstExpansionStrategy {
    fn initialise(&mut self, _context: &ExpansionContext) -> Result<()> {
        self.depth_map.clear();
        Ok(())
    }

    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate> {
        candidates
            .iter()
            .min_by_key(|c| self.depth_map.get(&c.node).unwrap_or(&0))
            .cloned()
    }

    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]) {
        existentials.sort_by_key(|e| self.depth_map.get(&e.node).unwrap_or(&0));
    }

    fn should_delay_expansion(
        &self,
        _candidate: &ExistentialCandidate,
        _context: &ExpansionContext,
    ) -> bool {
        false
    }

    fn get_expansion_priority(&self, candidate: &ExistentialCandidate) -> ExpansionPriority {
        let depth = self.depth_map.get(&candidate.node).unwrap_or(&0);
        if *depth < 5 {
            ExpansionPriority::High
        } else if *depth < 10 {
            ExpansionPriority::Normal
        } else {
            ExpansionPriority::Low
        }
    }

    fn expansion_completed(&mut self, candidate: &ExistentialCandidate, result: &ExpansionResult) {
        // Update depth for new individuals
        let current_depth = *self.depth_map.get(&candidate.node).unwrap_or(&0);
        for individual in &result.new_individuals {
            self.depth_map.insert(individual.clone(), current_depth + 1);
        }
    }

    fn clear(&mut self) {
        self.depth_map.clear();
    }
}

/// Depth-first expansion strategy
#[derive(Debug)]
pub struct DepthFirstExpansionStrategy {
    /// Current expansion path
    expansion_path: Vec<String>,
}

impl DepthFirstExpansionStrategy {
    /// Create a new depth-first strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            expansion_path: Vec::new(),
        }
    }
}

impl ExpansionStrategy for DepthFirstExpansionStrategy {
    fn initialise(&mut self, _context: &ExpansionContext) -> Result<()> {
        self.expansion_path.clear();
        Ok(())
    }

    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate> {
        // Prefer candidates from current expansion path
        if let Some(last_node) = self.expansion_path.last() {
            if let Some(candidate) = candidates.iter().find(|c| &c.node == last_node) {
                return Some(candidate.clone());
            }
        }
        candidates.first().cloned()
    }

    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]) {
        // Reverse order for depth-first behavior
        existentials.reverse();
    }

    fn should_delay_expansion(
        &self,
        _candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> bool {
        context.branching_depth > 50 // Limit depth to prevent infinite expansion
    }

    fn get_expansion_priority(&self, candidate: &ExistentialCandidate) -> ExpansionPriority {
        if self.expansion_path.contains(&candidate.node) {
            ExpansionPriority::High
        } else {
            ExpansionPriority::Normal
        }
    }

    fn expansion_completed(&mut self, candidate: &ExistentialCandidate, result: &ExpansionResult) {
        self.expansion_path.push(candidate.node.clone());
        // Add new individuals to expansion path
        for individual in &result.new_individuals {
            self.expansion_path.push(individual.clone());
        }
    }

    fn clear(&mut self) {
        self.expansion_path.clear();
    }
}

/// Priority-based expansion strategy
#[derive(Debug)]
pub struct PriorityBasedExpansionStrategy {
    /// Priority assignments for nodes
    node_priorities: HashMap<String, ExpansionPriority>,
    /// Default priority for new nodes
    default_priority: ExpansionPriority,
}

impl PriorityBasedExpansionStrategy {
    /// Create a new priority-based strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            node_priorities: HashMap::new(),
            default_priority: ExpansionPriority::Normal,
        }
    }

    /// Set priority for a specific node
    pub fn set_node_priority(&mut self, node: String, priority: ExpansionPriority) {
        self.node_priorities.insert(node, priority);
    }
}

impl ExpansionStrategy for PriorityBasedExpansionStrategy {
    fn initialise(&mut self, _context: &ExpansionContext) -> Result<()> {
        Ok(())
    }

    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate> {
        candidates
            .iter()
            .min_by_key(|c| {
                self.node_priorities
                    .get(&c.node)
                    .unwrap_or(&self.default_priority)
            })
            .cloned()
    }

    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]) {
        existentials.sort_by_key(|e| {
            self.node_priorities
                .get(&e.node)
                .unwrap_or(&self.default_priority)
        });
    }

    fn should_delay_expansion(
        &self,
        candidate: &ExistentialCandidate,
        _context: &ExpansionContext,
    ) -> bool {
        matches!(
            self.node_priorities
                .get(&candidate.node)
                .unwrap_or(&self.default_priority),
            ExpansionPriority::Delayed
        )
    }

    fn get_expansion_priority(&self, candidate: &ExistentialCandidate) -> ExpansionPriority {
        self.node_priorities
            .get(&candidate.node)
            .unwrap_or(&self.default_priority)
            .clone()
    }

    fn expansion_completed(
        &mut self,
        _candidate: &ExistentialCandidate,
        _result: &ExpansionResult,
    ) {
        // No specific action needed
    }

    fn clear(&mut self) {
        self.node_priorities.clear();
    }
}

/// Heuristic expansion strategy using multiple heuristics
#[derive(Debug)]
pub struct HeuristicExpansionStrategy {
    /// Complexity weights
    complexity_weights: ComplexityWeights,
    /// Depth preference
    depth_preference: f64,
    /// Role preference mapping
    role_preferences: HashMap<String, f64>,
}

impl HeuristicExpansionStrategy {
    /// Create a new heuristic strategy
    #[must_use]
    pub fn new() -> Self {
        Self {
            complexity_weights: ComplexityWeights::default(),
            depth_preference: 0.5,
            role_preferences: HashMap::new(),
        }
    }

    /// Calculate heuristic score for a candidate
    fn calculate_heuristic_score(&self, candidate: &ExistentialCandidate) -> f64 {
        let complexity = &candidate.complexity;

        // Base complexity score
        let complexity_score = self.complexity_weights.syntactic
            * f64::from(complexity.syntactic_complexity)
            + self.complexity_weights.role_successors * f64::from(complexity.role_successors)
            + self.complexity_weights.branching_factor * f64::from(complexity.branching_factor);

        // Role preference adjustment
        let role_key = format!("{:?}", candidate.role);
        let role_adjustment = self.role_preferences.get(&role_key).unwrap_or(&1.0);

        complexity_score * role_adjustment
    }
}

impl ExpansionStrategy for HeuristicExpansionStrategy {
    fn initialise(&mut self, _context: &ExpansionContext) -> Result<()> {
        Ok(())
    }

    fn select_next_existential(
        &mut self,
        candidates: &[ExistentialCandidate],
    ) -> Option<ExistentialCandidate> {
        candidates
            .iter()
            .min_by(|a, b| {
                self.calculate_heuristic_score(a)
                    .partial_cmp(&self.calculate_heuristic_score(b))
                    .unwrap_or(Ordering::Equal)
            })
            .cloned()
    }

    fn order_expansions(&mut self, existentials: &mut [ExistentialCandidate]) {
        existentials.sort_by(|a, b| {
            self.calculate_heuristic_score(a)
                .partial_cmp(&self.calculate_heuristic_score(b))
                .unwrap_or(Ordering::Equal)
        });
    }

    fn should_delay_expansion(
        &self,
        candidate: &ExistentialCandidate,
        context: &ExpansionContext,
    ) -> bool {
        self.calculate_heuristic_score(candidate) > 20.0 && context.branching_depth > 20
    }

    fn get_expansion_priority(&self, candidate: &ExistentialCandidate) -> ExpansionPriority {
        let score = self.calculate_heuristic_score(candidate);

        if score < 5.0 {
            ExpansionPriority::High
        } else if score < 15.0 {
            ExpansionPriority::Normal
        } else if score < 25.0 {
            ExpansionPriority::Low
        } else {
            ExpansionPriority::Delayed
        }
    }

    fn expansion_completed(
        &mut self,
        _candidate: &ExistentialCandidate,
        _result: &ExpansionResult,
    ) {
        // Could update heuristics based on expansion results
    }

    fn clear(&mut self) {
        // No persistent state to clear
    }
}

impl Default for BreadthFirstExpansionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for DepthFirstExpansionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PriorityBasedExpansionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for HeuristicExpansionStrategy {
    fn default() -> Self {
        Self::new()
    }
}

/// Uuid generation for unique identifiers
mod uuid {
    pub struct Uuid;

    impl Uuid {
        pub fn new_v4() -> Self {
            Self
        }

        pub fn to_string(&self) -> String {
            format!(
                "{:016x}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
                    % (1u128 << 64)
            )
        }
    }
}

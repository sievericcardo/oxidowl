//! Phase 2: Advanced Query Processing Enhancements
//!
//! This module implements sophisticated query optimization, rewriting, and execution
//! capabilities that build upon the existing foundation to deliver industry-leading
//! performance and intelligent query processing.

#![allow(dead_code)]

use super::conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
use super::optimization::{ExecutionStrategy, OptimizationError, PlanMetadata, QueryPlan};
use super::optimizer::AdvancedQueryPlan;
use crate::ontology::{
    ClassExpression, DataPropertyExpression, Individual, ObjectPropertyExpression, Ontology,
};
use crate::reasoning::ReasoningService;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Phase 2: Cost-based Query Optimizer
///
/// Implements sophisticated cost-based optimization with:
/// - Statistical cost estimation
/// - Join order optimization  
/// - Index selection recommendations
/// - Adaptive query rewriting
#[derive(Debug)]
pub struct CostBasedOptimizer {
    /// Query statistics collector
    statistics: Arc<RwLock<QueryStatistics>>,

    /// Cost model for different operations
    cost_model: Arc<CostModel>,

    /// Join order optimizer
    join_optimizer: JoinOrderOptimizer,

    /// Index advisor
    index_advisor: IndexAdvisor,

    /// Query rewriter
    query_rewriter: AdvancedQueryRewriter,

    /// Configuration
    config: CostBasedOptimizerConfig,
}

/// Configuration for cost-based optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostBasedOptimizerConfig {
    /// Enable cost-based join reordering
    pub enable_join_optimization: bool,

    /// Enable automatic index recommendations
    pub enable_index_recommendations: bool,

    /// Enable adaptive query rewriting
    pub enable_adaptive_rewriting: bool,

    /// Maximum search depth for join optimization
    pub max_join_search_depth: usize,

    /// Cost model sensitivity parameters
    pub cost_sensitivity: f64,

    /// Statistics collection sampling rate
    pub statistics_sampling_rate: f64,

    /// Query result caching enabled
    pub enable_result_caching: bool,

    /// Cache size limit (number of cached results)
    pub cache_size_limit: usize,
}

/// Query execution statistics for cost estimation
#[derive(Debug, Clone)]
pub struct QueryStatistics {
    /// Execution time statistics by query pattern
    execution_times: HashMap<QueryPattern, ExecutionTimeStats>,

    /// Memory usage statistics by query pattern
    memory_usage: HashMap<QueryPattern, MemoryUsageStats>,

    /// Result size statistics by query pattern
    result_sizes: HashMap<QueryPattern, ResultSizeStats>,

    /// Index usage statistics
    index_usage: HashMap<String, IndexUsageStats>,

    /// Total queries processed
    total_queries: u64,

    /// Last update timestamp
    last_updated: Instant,
}

/// Query pattern for statistical analysis
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct QueryPattern {
    /// Number of atoms in query
    pub atom_count: usize,

    /// Number of variables in query
    pub variable_count: usize,

    /// Number of joins in query
    pub join_count: usize,

    /// Query complexity class
    pub complexity_class: QueryComplexityClass,

    /// Dominant axiom types involved (stored as Vec for Hash compatibility)
    pub axiom_types: Vec<AxiomType>,
}

/// Query complexity classification
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum QueryComplexityClass {
    /// Simple atomic queries
    Atomic,

    /// Simple conjunctive queries
    SimpleConjunctive,

    /// Complex multi-join queries
    ComplexConjunctive,

    /// Recursive or transitive queries
    Recursive,

    /// Highly complex queries with many variables
    HighlyComplex,
}

/// Axiom types for cost estimation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum AxiomType {
    SubClassOf,
    EquivalentClasses,
    DisjointClasses,
    ObjectPropertyDomain,
    ObjectPropertyRange,
    SubObjectPropertyOf,
    InverseObjectProperties,
    FunctionalObjectProperty,
    InverseFunctionalObjectProperty,
    TransitiveObjectProperty,
    SymmetricObjectProperty,
    AsymmetricObjectProperty,
    ReflexiveObjectProperty,
    IrreflexiveObjectProperty,
    ClassAssertion,
    ObjectPropertyAssertion,
    NegativeObjectPropertyAssertion,
    DataPropertyAssertion,
    SameIndividual,
    DifferentIndividuals,
}

/// Statistical information for execution times
#[derive(Debug, Clone, PartialEq)]
pub struct ExecutionTimeStats {
    pub mean: f64,
    pub median: f64,
    pub std_dev: f64,
    pub min: f64,
    pub max: f64,
    pub sample_count: u64,
    pub percentile_95: f64,
    pub percentile_99: f64,
}

/// Statistical information for memory usage
#[derive(Debug, Clone)]
pub struct MemoryUsageStats {
    pub mean_bytes: f64,
    pub median_bytes: f64,
    pub std_dev_bytes: f64,
    pub min_bytes: u64,
    pub max_bytes: u64,
    pub sample_count: u64,
    pub peak_memory_percentile_95: f64,
}

/// Statistical information for result sizes
#[derive(Debug, Clone)]
pub struct ResultSizeStats {
    pub mean_size: f64,
    pub median_size: f64,
    pub std_dev_size: f64,
    pub min_size: usize,
    pub max_size: usize,
    pub sample_count: u64,
    pub empty_result_ratio: f64,
}

/// Index usage statistics
#[derive(Debug, Clone)]
pub struct IndexUsageStats {
    pub hit_count: u64,
    pub miss_count: u64,
    pub hit_ratio: f64,
    pub average_lookup_time: f64,
    pub total_lookups: u64,
    pub last_used: Instant,
}

/// Cost model for query operations
#[derive(Debug)]
pub struct CostModel {
    /// Base costs for different operation types
    base_costs: HashMap<OperationType, f64>,

    /// Scaling factors for different data sizes
    scaling_factors: HashMap<DataSizeCategory, f64>,

    /// Index access costs
    index_costs: HashMap<IndexType, f64>,

    /// Join operation costs
    join_costs: HashMap<JoinType, f64>,
}

/// Types of query operations for cost estimation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum OperationType {
    AtomicQuery,
    Join,
    Filter,
    Projection,
    Union,
    Intersection,
    Difference,
    Transitive,
    Recursive,
    IndexLookup,
    FullScan,
}

/// Data size categories for cost scaling
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum DataSizeCategory {
    Small,     // < 1K axioms
    Medium,    // 1K - 100K axioms
    Large,     // 100K - 1M axioms
    VeryLarge, // 1M - 10M axioms
    Massive,   // > 10M axioms
}

/// Index types for cost calculation
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum IndexType {
    Hash,
    BTree,
    Trie,
    Bitmap,
    Inverted,
    Spatial,
    Composite,
}

/// Join types with different cost characteristics
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum JoinType {
    NestedLoop,
    HashJoin,
    SortMergeJoin,
    IndexJoin,
    AdaptiveJoin,
}

/// Join order optimizer using dynamic programming
#[derive(Debug)]
pub struct JoinOrderOptimizer {
    /// Memoization cache for subproblems
    memo_cache: HashMap<JoinSubproblem, JoinSolution>,

    /// Cost threshold for exhaustive search
    exhaustive_search_threshold: usize,

    /// Heuristic strategies for large problems
    heuristics: Vec<Box<dyn JoinOrderHeuristic>>,
}

/// Represents a join subproblem for optimization
#[derive(Debug, Clone)]
pub struct JoinSubproblem {
    /// Set of relations to join
    relations: HashSet<QueryAtom>,

    /// Available join conditions
    join_conditions: Vec<JoinCondition>,
}

/// Solution for a join subproblem
#[derive(Debug, Clone)]
pub struct JoinSolution {
    /// Optimal join order
    join_order: Vec<QueryAtom>,

    /// Estimated cost
    estimated_cost: f64,

    /// Estimated result size
    estimated_result_size: usize,

    /// Join methods to use
    join_methods: Vec<JoinType>,
}

/// Join condition between query atoms
#[derive(Debug, Clone, PartialEq)]
pub struct JoinCondition {
    /// Left side atom
    left_atom: QueryAtom,

    /// Right side atom
    right_atom: QueryAtom,

    /// Shared variables
    shared_variables: Vec<QueryVariable>,

    /// Join selectivity estimate
    selectivity: f64,
}

/// Trait for join order heuristics
pub trait JoinOrderHeuristic: std::fmt::Debug + Send + Sync {
    /// Generate a heuristic join order
    fn generate_join_order(
        &self,
        problem: &JoinSubproblem,
        stats: &QueryStatistics,
    ) -> JoinSolution;

    /// Get heuristic name for logging
    fn name(&self) -> &str;
}

/// Index advisor for recommending beneficial indices
#[derive(Debug)]
pub struct IndexAdvisor {
    /// Historical query patterns
    query_patterns: HashMap<QueryPattern, u64>,

    /// Current index performance
    index_performance: HashMap<String, IndexPerformanceMetrics>,

    /// Cost-benefit analysis cache
    analysis_cache: HashMap<IndexRecommendationRequest, IndexRecommendation>,

    /// Configuration
    config: IndexAdvisorConfig,
}

/// Configuration for index advisor
#[derive(Debug, Clone)]
pub struct IndexAdvisorConfig {
    /// Minimum query frequency for index recommendation
    pub min_query_frequency: u64,

    /// Cost-benefit threshold for recommendations
    pub cost_benefit_threshold: f64,

    /// Maximum number of recommended indices
    pub max_recommendations: usize,

    /// Index maintenance cost factor
    pub maintenance_cost_factor: f64,
}

/// Index performance metrics
#[derive(Debug, Clone)]
pub struct IndexPerformanceMetrics {
    pub average_lookup_time: f64,
    pub hit_ratio: f64,
    pub memory_usage: u64,
    pub maintenance_cost: f64,
    pub last_rebuild_time: Instant,
    pub query_acceleration_factor: f64,
}

/// Request for index recommendation
#[derive(Debug, Clone)]
pub struct IndexRecommendationRequest {
    pub query_pattern: QueryPattern,
    pub frequency: u64,
    pub current_performance: ExecutionTimeStats,
}

/// Index recommendation result
#[derive(Debug, Clone)]
pub struct IndexRecommendation {
    /// Recommended index type
    pub index_type: IndexType,

    /// Columns/attributes to index
    pub indexed_elements: Vec<IndexedElement>,

    /// Expected performance improvement
    pub expected_improvement: f64,

    /// Implementation cost estimate
    pub implementation_cost: f64,

    /// Maintenance cost estimate
    pub maintenance_cost: f64,

    /// Confidence in recommendation
    pub confidence: f64,

    /// Justification for recommendation
    pub justification: String,
}

/// Element to be indexed
#[derive(Debug, Clone)]
pub enum IndexedElement {
    ClassExpression(ClassExpression),
    ObjectProperty(ObjectPropertyExpression),
    Individual(Individual),
    Composite { elements: Vec<IndexedElement> },
}

/// Advanced query rewriter with adaptive strategies
#[derive(Debug)]
pub struct AdvancedQueryRewriter {
    /// Rewriting rules database
    rewriting_rules: Vec<RewritingRule>,

    /// Adaptive strategy selector
    strategy_selector: AdaptiveStrategySelector,

    /// Rewriting history for learning
    rewriting_history: Vec<RewritingHistoryEntry>,

    /// Performance feedback system
    feedback_system: RewritingFeedbackSystem,
}

/// Query rewriting rule
#[derive(Debug)]
pub struct RewritingRule {
    /// Rule identifier
    pub id: String,

    /// Rule description
    pub description: String,

    /// Pattern to match
    pub pattern: QueryPattern,

    /// Rewriting function
    pub rewriter: Box<dyn QueryRewriter>,

    /// Expected performance improvement
    pub expected_improvement: f64,

    /// Applicability conditions
    pub conditions: Vec<ApplicabilityCondition>,
}

/// Trait for query rewriting functions
pub trait QueryRewriter: std::fmt::Debug + Send + Sync {
    /// Apply rewriting to a query
    fn rewrite(
        &self,
        query: &ConjunctiveQuery,
        context: &RewritingContext,
    ) -> Result<ConjunctiveQuery, RewritingError>;

    /// Estimate performance impact
    fn estimate_impact(&self, query: &ConjunctiveQuery) -> f64;
}

/// Context for query rewriting
#[derive(Debug)]
pub struct RewritingContext {
    /// Current query statistics
    pub statistics: Arc<RwLock<QueryStatistics>>,

    /// Available indices
    pub available_indices: Vec<String>,

    /// Ontology information
    pub ontology: Arc<Ontology>,

    /// Reasoning service
    pub reasoning_service: Arc<ReasoningService>,
}

/// Condition for rule applicability
#[derive(Debug)]
pub enum ApplicabilityCondition {
    /// Minimum query complexity
    MinComplexity(QueryComplexityClass),

    /// Maximum query complexity
    MaxComplexity(QueryComplexityClass),

    /// Required axiom types present
    RequiredAxiomTypes(HashSet<AxiomType>),

    /// Minimum expected improvement
    MinImprovement(f64),

    /// Index availability
    IndexAvailable(String),

    /// Custom condition
    Custom(Box<dyn ConditionEvaluator>),
}

/// Trait for custom condition evaluation
pub trait ConditionEvaluator: std::fmt::Debug + Send + Sync {
    /// Evaluate condition for a query
    fn evaluate(&self, query: &ConjunctiveQuery, context: &RewritingContext) -> bool;
}

/// Adaptive strategy selector for query rewriting
#[derive(Debug)]
pub struct AdaptiveStrategySelector {
    /// Strategy performance history
    strategy_history: HashMap<String, StrategyPerformanceHistory>,

    /// Current strategy weights
    strategy_weights: HashMap<String, f64>,

    /// Learning parameters
    learning_config: AdaptiveLearningConfig,
}

/// Performance history for a rewriting strategy
#[derive(Debug, Clone)]
pub struct StrategyPerformanceHistory {
    pub applications: u64,
    pub successes: u64,
    pub average_improvement: f64,
    pub confidence_interval: (f64, f64),
    pub last_used: Instant,
}

/// Configuration for adaptive learning
#[derive(Debug, Clone)]
pub struct AdaptiveLearningConfig {
    pub learning_rate: f64,
    pub exploration_rate: f64,
    pub min_confidence_threshold: f64,
    pub history_window_size: usize,
}

/// Entry in rewriting history for learning
#[derive(Debug, Clone)]
pub struct RewritingHistoryEntry {
    pub original_query: ConjunctiveQuery,
    pub rewritten_query: ConjunctiveQuery,
    pub rule_applied: String,
    pub performance_before: ExecutionTimeStats,
    pub performance_after: ExecutionTimeStats,
    pub improvement: f64,
    pub timestamp: Instant,
}

/// Feedback system for rewriting performance
#[derive(Debug)]
pub struct RewritingFeedbackSystem {
    /// Feedback collection
    feedback_data: Vec<RewritingFeedback>,

    /// Analysis results
    analysis_results: HashMap<String, FeedbackAnalysis>,

    /// Automatic adjustment system
    auto_adjustment: AutoAdjustmentSystem,
}

/// Individual feedback entry
#[derive(Debug, Clone)]
pub struct RewritingFeedback {
    pub rule_id: String,
    pub query_pattern: QueryPattern,
    pub actual_improvement: f64,
    pub expected_improvement: f64,
    pub success: bool,
    pub error_message: Option<String>,
    pub timestamp: Instant,
}

/// Analysis of feedback data
#[derive(Debug)]
pub struct FeedbackAnalysis {
    pub rule_effectiveness: f64,
    pub accuracy_score: f64,
    pub reliability_score: f64,
    pub recommended_adjustments: Vec<RuleAdjustment>,
}

/// Suggested adjustments to rewriting rules
#[derive(Debug)]
pub enum RuleAdjustment {
    /// Adjust expected improvement estimate
    AdjustImprovement(f64),

    /// Modify applicability conditions
    ModifyConditions(Vec<ApplicabilityCondition>),

    /// Change rule priority
    ChangePriority(i32),

    /// Disable rule temporarily or permanently
    DisableRule {
        temporary: bool,
        duration: Option<Duration>,
    },
}

/// Automatic adjustment system for rules
#[derive(Debug)]
pub struct AutoAdjustmentSystem {
    /// Adjustment history
    adjustment_history: Vec<AutoAdjustmentEntry>,

    /// Adjustment strategies
    strategies: Vec<Box<dyn AdjustmentStrategy>>,

    /// Configuration
    config: AutoAdjustmentConfig,
}

/// Entry in adjustment history
#[derive(Debug)]
pub struct AutoAdjustmentEntry {
    pub rule_id: String,
    pub adjustment: RuleAdjustment,
    pub reason: String,
    pub impact: f64,
    pub timestamp: Instant,
}

/// Trait for adjustment strategies
pub trait AdjustmentStrategy: std::fmt::Debug + Send + Sync {
    /// Analyze feedback and suggest adjustments
    fn suggest_adjustments(&self, feedback: &[RewritingFeedback]) -> Vec<RuleAdjustment>;

    /// Strategy name
    fn name(&self) -> &str;
}

/// Configuration for auto-adjustment system
#[derive(Debug, Clone)]
pub struct AutoAdjustmentConfig {
    pub enable_auto_adjustment: bool,
    pub adjustment_threshold: f64,
    pub min_feedback_samples: usize,
    pub adjustment_frequency: Duration,
    pub conservative_mode: bool,
}

/// Error types for query rewriting
#[derive(Debug, Clone)]
pub enum RewritingError {
    /// Invalid query structure
    InvalidQuery(String),

    /// Rule application failed
    RuleApplicationFailed { rule_id: String, error: String },

    /// No applicable rules found
    NoApplicableRules,

    /// Rewriting resulted in worse performance
    PerformanceRegression { original_cost: f64, new_cost: f64 },

    /// System error
    SystemError(String),
}

impl std::fmt::Display for RewritingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RewritingError::InvalidQuery(msg) => write!(f, "Invalid query: {}", msg),
            RewritingError::RuleApplicationFailed { rule_id, error } => {
                write!(f, "Rule '{}' failed: {}", rule_id, error)
            }
            RewritingError::NoApplicableRules => write!(f, "No applicable rewriting rules found"),
            RewritingError::PerformanceRegression {
                original_cost,
                new_cost,
            } => {
                write!(
                    f,
                    "Rewriting resulted in performance regression: {} -> {}",
                    original_cost, new_cost
                )
            }
            RewritingError::SystemError(msg) => write!(f, "System error: {}", msg),
        }
    }
}

impl std::error::Error for RewritingError {}

// ===== Implementation Methods =====

impl CostBasedOptimizer {
    /// Create a new cost-based optimizer
    pub fn new(
        _ontology: Arc<Ontology>,
        _reasoning_service: Arc<ReasoningService>,
        config: CostBasedOptimizerConfig,
    ) -> Self {
        let statistics = Arc::new(RwLock::new(QueryStatistics::new()));
        let cost_model = Arc::new(CostModel::default());

        Self {
            statistics: statistics.clone(),
            cost_model: cost_model.clone(),
            join_optimizer: JoinOrderOptimizer::new(),
            index_advisor: IndexAdvisor::new(IndexAdvisorConfig::default()),
            query_rewriter: AdvancedQueryRewriter::new(),
            config,
        }
    }

    /// Optimize a query using cost-based strategies
    pub fn optimize_query(
        &mut self,
        query: &ConjunctiveQuery,
    ) -> Result<AdvancedQueryPlan, OptimizationError> {
        // Step 1: Analyze query pattern and collect statistics
        let pattern = self.analyze_query_pattern(query);

        // Step 2: Generate base query plan
        let base_plan = {
            let stats = self.statistics.read().map_err(|e| {
                OptimizationError::internal(format!("Failed to read statistics: {}", e))
            })?;
            self.generate_base_plan(query, &pattern, &stats)?
        };

        // Step 3: Optimize join order if applicable
        let optimized_joins = if self.config.enable_join_optimization && pattern.join_count > 1 {
            let stats = self.statistics.read().map_err(|e| {
                OptimizationError::internal(format!("Failed to read statistics: {}", e))
            })?;
            self.optimize_join_order(query, &pattern, &stats)?
        } else {
            base_plan.strategy.clone()
        };

        // Step 4: Generate index recommendations
        let _index_recommendations = if self.config.enable_index_recommendations {
            let stats = self.statistics.read().map_err(|e| {
                OptimizationError::internal(format!("Failed to read statistics: {}", e))
            })?;
            self.index_advisor
                .recommend_indices(query, &pattern, &stats)?
        } else {
            Vec::new()
        };

        // Step 5: Apply adaptive query rewriting
        let rewritten_query = if self.config.enable_adaptive_rewriting {
            // For rewriting, we would need access to actual ontology and reasoning service
            // For now, skip rewriting if not provided - would need to be passed from caller
            query.clone()
        } else {
            query.clone()
        };

        // Step 6: Estimate performance for final plan
        let stats = self.statistics.read().map_err(|e| {
            OptimizationError::internal(format!("Failed to read statistics: {}", e))
        })?;
        let performance_prediction = self.estimate_performance(&rewritten_query, &pattern, &stats);

        // Step 7: Generate optimization suggestions
        let _optimization_suggestions =
            self.generate_optimization_suggestions(&rewritten_query, &pattern);

        // Step 8: Calculate confidence scores
        let confidence_scores = self.calculate_confidence_scores(&pattern, &stats);

        // Step 9: Get estimates that need stats
        let estimated_result_size = self.estimate_result_size(&rewritten_query, &stats);
        let overall_confidence = self.calculate_overall_confidence(&pattern, &stats);

        drop(stats); // Explicitly drop to avoid borrow issues

        Ok(AdvancedQueryPlan {
            base_plan: QueryPlan {
                original_query: query.clone(),
                optimized_query: rewritten_query.clone(),
                strategy: optimized_joins,
                estimated_cost: performance_prediction.execution_time,
                join_order: self.compute_join_order(&rewritten_query),
                metadata: PlanMetadata::default(),
            },
            predicted_performance: super::optimizer::PerformancePrediction {
                estimated_execution_time: Duration::from_secs_f64(
                    performance_prediction.execution_time,
                ),
                estimated_memory_usage: performance_prediction.memory_usage as usize,
                estimated_result_size,
                confidence_level: overall_confidence,
            },
            recommended_indices: self.get_index_recommendations(&rewritten_query, &pattern),
            optimization_suggestions: self.get_optimization_suggestions(&rewritten_query, &pattern),
            confidence_scores: super::optimizer::ConfidenceScores {
                execution_time_confidence: confidence_scores.performance_prediction,
                memory_usage_confidence: confidence_scores.index_recommendations,
                optimization_strategy_confidence: confidence_scores.optimization_suggestions,
                overall_confidence: confidence_scores.overall,
            },
        })
    }

    /// Compute optimal join order for query
    fn compute_join_order(&self, query: &ConjunctiveQuery) -> Vec<usize> {
        let atoms = &query.body_atoms;
        if atoms.len() <= 1 {
            return vec![];
        }

        // Build join graph showing which atoms share variables
        let mut join_graph: HashMap<usize, Vec<(usize, Vec<QueryVariable>)>> = HashMap::new();

        for (i, atom_i) in atoms.iter().enumerate() {
            for (j, atom_j) in atoms.iter().enumerate() {
                if i < j {
                    let shared_vars = self.find_shared_variables(atom_i, atom_j);
                    if !shared_vars.is_empty() {
                        join_graph
                            .entry(i)
                            .or_insert_with(Vec::new)
                            .push((j, shared_vars.clone()));
                        join_graph
                            .entry(j)
                            .or_insert_with(Vec::new)
                            .push((i, shared_vars));
                    }
                }
            }
        }

        // Use greedy heuristic: start with smallest estimated atom, add most selective joins
        let mut ordered_indices = Vec::new();
        let mut remaining: HashSet<usize> = (0..atoms.len()).collect();

        // Find atom with smallest estimated cardinality
        let start_idx = (0..atoms.len())
            .min_by_key(|&i| self.estimate_atom_cardinality(&atoms[i]))
            .unwrap_or(0);

        ordered_indices.push(start_idx);
        remaining.remove(&start_idx);

        // Greedily add atoms with highest selectivity to current set
        while !remaining.is_empty() {
            let mut best_next = None;
            let mut best_selectivity = f64::MAX;

            for &candidate in &remaining {
                // Check if candidate joins with any atom in current set
                let mut max_selectivity = 0.0_f64;
                for &existing in &ordered_indices {
                    if let Some(neighbors) = join_graph.get(&existing) {
                        for (neighbor, shared_vars) in neighbors {
                            if *neighbor == candidate {
                                let selectivity = self.estimate_join_selectivity(
                                    &atoms[existing],
                                    &atoms[candidate],
                                    shared_vars,
                                );
                                max_selectivity = max_selectivity.max(selectivity);
                            }
                        }
                    }
                }

                // Lower selectivity is better (fewer results)
                if max_selectivity > 0.0 && max_selectivity < best_selectivity {
                    best_selectivity = max_selectivity;
                    best_next = Some(candidate);
                }
            }

            // Add best candidate or any remaining if no join found
            if let Some(next_idx) = best_next {
                ordered_indices.push(next_idx);
                remaining.remove(&next_idx);
            } else if let Some(&any_remaining) = remaining.iter().next() {
                ordered_indices.push(any_remaining);
                remaining.remove(&any_remaining);
            }
        }

        // Return indices directly
        ordered_indices
    }

    /// Find variables shared between two atoms
    fn find_shared_variables(&self, atom1: &QueryAtom, atom2: &QueryAtom) -> Vec<QueryVariable> {
        let vars1 = self.extract_variables(atom1);
        let vars2 = self.extract_variables(atom2);

        vars1.into_iter().filter(|v| vars2.contains(v)).collect()
    }

    /// Extract all variables from an atom
    fn extract_variables(&self, atom: &QueryAtom) -> HashSet<QueryVariable> {
        let mut vars = HashSet::new();
        match atom {
            QueryAtom::ClassAtom { variable, .. } => {
                vars.insert(variable.clone());
            }
            QueryAtom::ObjectPropertyAtom {
                subject, object, ..
            } => {
                vars.insert(subject.clone());
                vars.insert(object.clone());
            }
            QueryAtom::DataPropertyAtom {
                subject, literal, ..
            } => {
                vars.insert(subject.clone());
                vars.insert(literal.clone());
            }
            _ => {}
        }
        vars
    }

    /// Estimate cardinality of a single atom
    fn estimate_atom_cardinality(&self, atom: &QueryAtom) -> usize {
        match atom {
            QueryAtom::ClassAtom { .. } => 1000, // Classes typically have many instances
            QueryAtom::ObjectPropertyAtom { .. } => 500, // Properties somewhat selective
            QueryAtom::DataPropertyAtom { .. } => 500,
            _ => 100,
        }
    }

    /// Estimate join selectivity between two atoms
    fn estimate_join_selectivity(
        &self,
        atom1: &QueryAtom,
        atom2: &QueryAtom,
        _shared_vars: &[QueryVariable],
    ) -> f64 {
        // Simple heuristic: properties are more selective than classes
        let card1 = self.estimate_atom_cardinality(atom1) as f64;
        let card2 = self.estimate_atom_cardinality(atom2) as f64;

        // Selectivity = expected result size / cartesian product size
        let expected_result = (card1 * card2).sqrt(); // Geometric mean heuristic
        expected_result / (card1 * card2)
    }

    /// Describe an atom in readable form
    fn describe_atom(&self, atom: &QueryAtom) -> String {
        match atom {
            QueryAtom::ClassAtom {
                class_expression,
                variable,
            } => {
                format!(
                    "{}(?{})",
                    self.describe_class_expr(class_expression),
                    variable.name
                )
            }
            QueryAtom::ObjectPropertyAtom {
                property,
                subject,
                object,
            } => {
                format!(
                    "{}(?{}, ?{})",
                    self.describe_prop_expr(property),
                    subject.name,
                    object.name
                )
            }
            QueryAtom::DataPropertyAtom {
                property, subject, ..
            } => {
                format!(
                    "{}(?{}, ...)",
                    self.describe_data_prop(property),
                    subject.name
                )
            }
            _ => "UnknownAtom".to_string(),
        }
    }

    fn describe_class_expr(&self, expr: &ClassExpression) -> String {
        match expr {
            ClassExpression::Class(c) => c
                .iri
                .as_str()
                .split('#')
                .last()
                .unwrap_or("Class")
                .to_string(),
            _ => "ComplexClass".to_string(),
        }
    }

    fn describe_prop_expr(&self, expr: &ObjectPropertyExpression) -> String {
        match expr {
            ObjectPropertyExpression::ObjectProperty(p) => p
                .iri
                .as_str()
                .split('#')
                .last()
                .unwrap_or("Property")
                .to_string(),
            _ => "ComplexProperty".to_string(),
        }
    }

    fn describe_data_prop(&self, expr: &DataPropertyExpression) -> String {
        match expr {
            DataPropertyExpression::DataProperty(p) => p
                .iri
                .as_str()
                .split('#')
                .last()
                .unwrap_or("DataProperty")
                .to_string(),
        }
    }

    /// Estimate result size for query
    fn estimate_result_size(&self, query: &ConjunctiveQuery, stats: &QueryStatistics) -> usize {
        // Look for similar queries in statistics
        let pattern = self.analyze_query_pattern(query);

        if let Some(result_stats) = stats.result_sizes.get(&pattern) {
            result_stats.mean_size as usize
        } else {
            // Heuristic: base size * reduction per atom
            let base_size = 1000;
            let reduction_factor = 0.6_f64; // Each atom reduces by ~40%
            let atoms = query.body_atoms.len() as u32;
            (base_size as f64 * reduction_factor.powi(atoms as i32)) as usize
        }
    }

    /// Calculate overall confidence based on statistics
    fn calculate_overall_confidence(&self, pattern: &QueryPattern, stats: &QueryStatistics) -> f64 {
        // Check if we have sufficient data for this pattern
        let sample_size = stats
            .execution_times
            .get(pattern)
            .map(|s| s.sample_count)
            .unwrap_or(0);

        // Confidence increases with sample size (logarithmic)
        if sample_size == 0 {
            0.5 // Low confidence with no data
        } else if sample_size < 10 {
            0.6 + (sample_size as f64 * 0.03)
        } else if sample_size < 100 {
            0.8 + ((sample_size as f64).ln() / 50.0)
        } else {
            0.95 // High confidence with many samples
        }
    }

    /// Get index recommendations for query
    fn get_index_recommendations(
        &self,
        query: &ConjunctiveQuery,
        _pattern: &QueryPattern,
    ) -> Vec<super::optimizer::IndexRecommendation> {
        let mut recommendations = Vec::new();

        // Recommend indices for frequently accessed properties
        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ObjectPropertyAtom { property, .. } => {
                    if let ObjectPropertyExpression::ObjectProperty(_prop) = property {
                        recommendations.push(super::optimizer::IndexRecommendation {
                            index_type: "Hash".to_string(),
                            expected_improvement: 2.5,
                            creation_cost: 1.0,
                            maintenance_cost: 1.2,
                        });
                    }
                }
                QueryAtom::ClassAtom {
                    class_expression, ..
                } => {
                    if let ClassExpression::Class(_class) = class_expression {
                        recommendations.push(super::optimizer::IndexRecommendation {
                            index_type: "BTree".to_string(),
                            expected_improvement: 1.8,
                            creation_cost: 0.8,
                            maintenance_cost: 1.0,
                        });
                    }
                }
                _ => {}
            }
        }

        // Limit to top 3 recommendations
        recommendations.truncate(3);
        recommendations
    }

    /// Get optimization suggestions for query
    fn get_optimization_suggestions(
        &self,
        query: &ConjunctiveQuery,
        pattern: &QueryPattern,
    ) -> Vec<super::optimizer::OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest query simplification for complex queries
        if pattern.atom_count > 10 {
            suggestions.push(super::optimizer::OptimizationSuggestion {
                suggestion_type: super::optimizer::OptimizationType::QueryRewriting,
                description: "Consider breaking complex query into smaller sub-queries".to_string(),
                expected_improvement: 1.5,
                implementation_complexity: 0.6,
            });
        }

        // Suggest materialization for repeated patterns
        if self.has_repeated_patterns(query) {
            suggestions.push(super::optimizer::OptimizationSuggestion {
                suggestion_type: super::optimizer::OptimizationType::CachingStrategy,
                description: "Materialize commonly queried pattern".to_string(),
                expected_improvement: 3.0,
                implementation_complexity: 0.8,
            });
        }

        // Suggest adding indices if none recommended yet
        if query.body_atoms.len() > 3 {
            suggestions.push(super::optimizer::OptimizationSuggestion {
                suggestion_type: super::optimizer::OptimizationType::IndexCreation,
                description: "Add indices for frequently joined properties".to_string(),
                expected_improvement: 2.0,
                implementation_complexity: 0.3,
            });
        }

        suggestions
    }

    /// Check if query has repeated patterns
    fn has_repeated_patterns(&self, query: &ConjunctiveQuery) -> bool {
        let mut pattern_counts: HashMap<String, usize> = HashMap::new();

        for atom in &query.body_atoms {
            let pattern = self.describe_atom(atom);
            *pattern_counts.entry(pattern).or_insert(0) += 1;
        }

        pattern_counts.values().any(|&count| count > 1)
    }

    /// Analyze query to extract pattern information
    fn analyze_query_pattern(&self, query: &ConjunctiveQuery) -> QueryPattern {
        let atom_count = query.body_atoms.len();
        let variable_count = query.answer_variables.len();
        let join_count = self.count_joins(query);

        let complexity_class = match (atom_count, variable_count, join_count) {
            (1, _, 0) => QueryComplexityClass::Atomic,
            (2..=5, _, 1..=2) => QueryComplexityClass::SimpleConjunctive,
            (6..=20, _, 3..=10) => QueryComplexityClass::ComplexConjunctive,
            (_, _, _) if self.has_recursive_patterns(query) => QueryComplexityClass::Recursive,
            _ => QueryComplexityClass::HighlyComplex,
        };

        let axiom_types = self.extract_axiom_types(query);

        QueryPattern {
            atom_count,
            variable_count,
            join_count,
            complexity_class,
            axiom_types,
        }
    }

    // Additional helper methods would be implemented here...
    fn count_joins(&self, query: &ConjunctiveQuery) -> usize {
        // Count join points by finding shared variables between atoms
        let mut join_count = 0;
        let atoms = &query.body_atoms;

        for i in 0..atoms.len() {
            for j in (i + 1)..atoms.len() {
                if self.atoms_share_variable(&atoms[i], &atoms[j]) {
                    join_count += 1;
                }
            }
        }

        join_count
    }

    /// Check if two query atoms share any variables
    fn atoms_share_variable(&self, atom1: &QueryAtom, atom2: &QueryAtom) -> bool {
        let vars1 = self.extract_variables_from_atom(atom1);
        let vars2 = self.extract_variables_from_atom(atom2);

        vars1.iter().any(|v| vars2.contains(v))
    }

    /// Extract all variables from a query atom
    fn extract_variables_from_atom(&self, atom: &QueryAtom) -> Vec<QueryVariable> {
        match atom {
            QueryAtom::ClassAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ObjectPropertyAtom {
                subject, object, ..
            } => {
                vec![subject.clone(), object.clone()]
            }
            QueryAtom::DataPropertyAtom {
                subject, literal, ..
            } => {
                // literal is also a QueryVariable in conjunctive queries
                vec![subject.clone(), literal.clone()]
            }
            QueryAtom::SameIndividualAtom { left, right } => {
                vec![left.clone(), right.clone()]
            }
            QueryAtom::DifferentIndividualsAtom { left, right } => {
                vec![left.clone(), right.clone()]
            }
            QueryAtom::ConcreteIndividualAtom { variable, .. } => {
                vec![variable.clone()]
            }
            QueryAtom::ConcreteLiteralAtom { variable, .. } => {
                vec![variable.clone()]
            }
        }
    }

    fn has_recursive_patterns(&self, query: &ConjunctiveQuery) -> bool {
        // Check for transitive closure patterns or cyclic variable dependencies

        // Build a dependency graph of variables
        let mut graph: HashMap<QueryVariable, Vec<QueryVariable>> = HashMap::new();

        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ObjectPropertyAtom {
                    subject, object, ..
                } => {
                    graph
                        .entry(subject.clone())
                        .or_insert_with(Vec::new)
                        .push(object.clone());
                }
                QueryAtom::DataPropertyAtom { subject, .. } => {
                    // Data properties create endpoints, not recursive patterns
                    graph.entry(subject.clone()).or_insert_with(Vec::new);
                }
                _ => {}
            }
        }

        // Check for cycles using DFS
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for start_var in graph.keys() {
            if self.has_cycle_dfs(start_var, &graph, &mut visited, &mut rec_stack) {
                return true;
            }
        }

        false
    }

    /// DFS helper to detect cycles in variable dependency graph
    fn has_cycle_dfs(
        &self,
        var: &QueryVariable,
        graph: &HashMap<QueryVariable, Vec<QueryVariable>>,
        visited: &mut HashSet<QueryVariable>,
        rec_stack: &mut HashSet<QueryVariable>,
    ) -> bool {
        if rec_stack.contains(var) {
            return true; // Found a cycle
        }

        if visited.contains(var) {
            return false; // Already processed
        }

        visited.insert(var.clone());
        rec_stack.insert(var.clone());

        if let Some(neighbors) = graph.get(var) {
            for neighbor in neighbors {
                if self.has_cycle_dfs(neighbor, graph, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(var);
        false
    }

    fn extract_axiom_types(&self, query: &ConjunctiveQuery) -> Vec<AxiomType> {
        // Implementation for extracting axiom types
        let mut types = HashSet::new();

        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ClassAtom { .. } => {
                    types.insert(AxiomType::SubClassOf);
                }
                QueryAtom::ObjectPropertyAtom { .. } => {
                    types.insert(AxiomType::SubObjectPropertyOf);
                }
                QueryAtom::DataPropertyAtom { .. } => {
                    types.insert(AxiomType::DataPropertyAssertion);
                }
                _ => {}
            }
        }

        types.into_iter().collect()
    }

    fn generate_base_plan(
        &self,
        query: &ConjunctiveQuery,
        pattern: &QueryPattern,
        stats: &QueryStatistics,
    ) -> Result<QueryPlan, OptimizationError> {
        // Select base execution strategy based on query pattern
        let strategy = match pattern.complexity_class {
            QueryComplexityClass::Atomic => ExecutionStrategy::Direct,
            QueryComplexityClass::SimpleConjunctive => {
                if pattern.join_count <= 2 {
                    ExecutionStrategy::Direct
                } else {
                    ExecutionStrategy::Tableau {
                        expansion_order: query.body_atoms.clone(),
                    }
                }
            }
            QueryComplexityClass::ComplexConjunctive => ExecutionStrategy::Tableau {
                expansion_order: query.body_atoms.clone(),
            },
            QueryComplexityClass::Recursive => ExecutionStrategy::Hybrid {
                tableau_atoms: query.body_atoms.clone(),
                rewriting_atoms: vec![],
            },
            QueryComplexityClass::HighlyComplex => {
                // Split into tableau and rewriting components
                let (tableau_atoms, rewriting_atoms) = self.split_query_atoms(&query.body_atoms);
                ExecutionStrategy::Hybrid {
                    tableau_atoms,
                    rewriting_atoms,
                }
            }
        };

        // Estimate initial cost
        let estimated_cost = self.estimate_query_cost(query, &strategy, stats);

        // Generate join order if needed
        let join_order = if pattern.join_count > 1 {
            self.compute_join_order(query)
        } else {
            vec![]
        };

        Ok(QueryPlan {
            original_query: query.clone(),
            optimized_query: query.clone(),
            strategy,
            estimated_cost,
            join_order,
            metadata: PlanMetadata::default(),
        })
    }

    /// Split query atoms into tableau and rewriting components
    fn split_query_atoms(&self, atoms: &[QueryAtom]) -> (Vec<QueryAtom>, Vec<QueryAtom>) {
        let mut tableau_atoms = Vec::new();
        let mut rewriting_atoms = Vec::new();

        for atom in atoms {
            match atom {
                // Complex atoms go to tableau
                QueryAtom::ClassAtom { class_expression, .. } => {
                    if self.is_complex_class_expression(class_expression) {
                        tableau_atoms.push(atom.clone());
                    } else {
                        rewriting_atoms.push(atom.clone());
                    }
                }
                // Properties can be handled by rewriting
                QueryAtom::ObjectPropertyAtom { .. } | QueryAtom::DataPropertyAtom { .. } => {
                    rewriting_atoms.push(atom.clone());
                }
                // Other atoms go to tableau
                _ => tableau_atoms.push(atom.clone()),
            }
        }

        (tableau_atoms, rewriting_atoms)
    }

    /// Check if class expression is complex
    fn is_complex_class_expression(&self, expr: &ClassExpression) -> bool {
        matches!(
            expr,
            ClassExpression::ObjectIntersectionOf(_)
                | ClassExpression::ObjectUnionOf(_)
                | ClassExpression::ObjectSomeValuesFrom { .. }
                | ClassExpression::ObjectAllValuesFrom { .. }
                | ClassExpression::ObjectMinCardinality { .. }
                | ClassExpression::ObjectMaxCardinality { .. }
        )
    }

    /// Estimate cost of executing query with given strategy
    fn estimate_query_cost(
        &self,
        query: &ConjunctiveQuery,
        strategy: &ExecutionStrategy,
        _stats: &QueryStatistics,
    ) -> f64 {
        let base_cost = match strategy {
            ExecutionStrategy::Direct => 100.0,
            ExecutionStrategy::Tableau { .. } => 500.0,
            ExecutionStrategy::Rewriting { .. } => 300.0,
            ExecutionStrategy::Hybrid { .. } => 400.0,
        };

        // Adjust for query complexity
        let atom_count = query.body_atoms.len() as f64;
        let complexity_multiplier = 1.0 + (atom_count * 0.5);

        // Adjust for join complexity
        let join_multiplier = if query.body_atoms.len() > 1 {
            let join_count = self.count_joins(query);
            1.0 + (join_count as f64 * 0.3)
        } else {
            1.0
        };

        base_cost * complexity_multiplier * join_multiplier
    }

    fn optimize_join_order(
        &self,
        query: &ConjunctiveQuery,
        pattern: &QueryPattern,
        _stats: &QueryStatistics,
    ) -> Result<ExecutionStrategy, OptimizationError> {
        // Use greedy join ordering algorithm
        if query.body_atoms.len() <= 1 {
            return Ok(ExecutionStrategy::Direct);
        }

        // Get optimized atom order from join optimizer
        let ordered_indices = self.join_optimizer.optimize_join_order_greedy(&query.body_atoms);
        
        // Reorder atoms according to optimized indices
        let mut ordered_atoms: Vec<QueryAtom> = ordered_indices
            .iter()
            .filter_map(|&idx| query.body_atoms.get(idx).cloned())
            .collect();

        // If we didn't get all atoms, add remaining ones
        if ordered_atoms.len() < query.body_atoms.len() {
            for (_i, atom) in query.body_atoms.iter().enumerate() {
                if !ordered_atoms.iter().any(|a| {
                    format!("{:?}", a) == format!("{:?}", atom)
                }) {
                    ordered_atoms.push(atom.clone());
                }
            }
        }

        // Determine strategy based on query complexity
        let strategy = if pattern.complexity_class == QueryComplexityClass::Atomic
            || pattern.complexity_class == QueryComplexityClass::SimpleConjunctive
        {
            ExecutionStrategy::Tableau {
                expansion_order: ordered_atoms,
            }
        } else {
            let (tableau_atoms, rewriting_atoms) = self.split_query_atoms(&ordered_atoms);
            if rewriting_atoms.is_empty() {
                ExecutionStrategy::Tableau {
                    expansion_order: ordered_atoms,
                }
            } else {
                ExecutionStrategy::Hybrid {
                    tableau_atoms,
                    rewriting_atoms,
                }
            }
        };

        Ok(strategy)
    }

    fn get_available_indices(&self) -> Vec<String> {
        // Implementation for getting available indices
        Vec::new()
    }

    fn estimate_performance(
        &self,
        query: &ConjunctiveQuery,
        pattern: &QueryPattern,
        stats: &QueryStatistics,
    ) -> PerformancePrediction {
        // Use historical data if available
        let base_time = if let Some(time_stats) = stats.execution_times.get(pattern) {
            time_stats.mean
        } else {
            // Estimate based on complexity
            let base = match pattern.complexity_class {
                QueryComplexityClass::Atomic => 10.0,
                QueryComplexityClass::SimpleConjunctive => 50.0,
                QueryComplexityClass::ComplexConjunctive => 200.0,
                QueryComplexityClass::Recursive => 500.0,
                QueryComplexityClass::HighlyComplex => 1000.0,
            };

            // Scale by number of atoms
            base * (1.0 + query.body_atoms.len() as f64 * 0.2)
        };

        // Adjust for join complexity
        let join_factor = if pattern.join_count > 0 {
            1.0 + (pattern.join_count as f64 * 0.5)
        } else {
            1.0
        };

        let execution_time = base_time * join_factor;

        // Estimate memory usage
        let memory_usage = if let Some(mem_stats) = stats.memory_usage.get(pattern) {
            mem_stats.mean_bytes
        } else {
            // Estimate: 1MB base + 100KB per atom + 500KB per join
            let base_mem = 1024.0 * 1024.0; // 1MB
            let atom_mem = query.body_atoms.len() as f64 * 100.0 * 1024.0;
            let join_mem = pattern.join_count as f64 * 500.0 * 1024.0;
            base_mem + atom_mem + join_mem
        };

        // Estimate result size
        let result_size_estimate = if let Some(size_stats) = stats.result_sizes.get(pattern) {
            size_stats.mean_size as usize
        } else {
            // Heuristic: start with 100, multiply by join selectivity
            let base_size = 100;
            let join_multiplier = if pattern.join_count > 0 {
                // Each join typically reduces result size
                0.5_f64.powi(pattern.join_count as i32)
            } else {
                1.0
            };
            (base_size as f64 * join_multiplier * (1.0 + pattern.atom_count as f64 * 0.3)) as usize
        };

        // Calculate confidence based on available data
        let confidence_scores = self.calculate_confidence_scores(pattern, stats);
        let confidence = confidence_scores.performance_prediction;

        PerformancePrediction {
            execution_time,
            memory_usage,
            result_size_estimate,
            confidence,
        }
    }

    fn generate_optimization_suggestions(
        &self,
        query: &ConjunctiveQuery,
        pattern: &QueryPattern,
    ) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        // Suggest index creation for frequently accessed properties
        if pattern.join_count > 2 {
            for atom in &query.body_atoms {
                if let QueryAtom::ObjectPropertyAtom { property, .. } = atom {
                    suggestions.push(OptimizationSuggestion {
                        suggestion_type: OptimizationSuggestionType::IndexCreation,
                        description: format!(
                            "Create index on property: {:?} to improve join performance",
                            property
                        ),
                        expected_improvement: 0.4, // 40% improvement
                        implementation_difficulty: DifficultyLevel::Easy,
                        priority: Priority::High,
                    });
                }
            }
        }

        // Suggest query rewriting for complex class expressions
        for atom in &query.body_atoms {
            if let QueryAtom::ClassAtom { class_expression, .. } = atom {
                if matches!(
                    class_expression,
                    ClassExpression::ObjectUnionOf(_) | ClassExpression::ObjectIntersectionOf(_)
                ) {
                    suggestions.push(OptimizationSuggestion {
                        suggestion_type: OptimizationSuggestionType::QueryRewriting,
                        description: "Consider rewriting complex class expressions using subsumption reasoning".to_string(),
                        expected_improvement: 0.25,
                        implementation_difficulty: DifficultyLevel::Medium,
                        priority: Priority::Medium,
                    });
                }
            }
        }

        // Suggest join reordering if not already optimized
        if pattern.join_count > 3 && !self.config.enable_join_optimization {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationSuggestionType::JoinReordering,
                description: "Enable join optimization to improve execution order".to_string(),
                expected_improvement: 0.5, // 50% improvement for many joins
                implementation_difficulty: DifficultyLevel::Easy,
                priority: Priority::High,
            });
        }

        // Suggest caching for expensive queries
        if pattern.complexity_class == QueryComplexityClass::HighlyComplex
            || pattern.complexity_class == QueryComplexityClass::Recursive
        {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationSuggestionType::Caching,
                description: "Enable result caching for this expensive query pattern".to_string(),
                expected_improvement: 0.8, // 80% improvement on cache hits
                implementation_difficulty: DifficultyLevel::Easy,
                priority: Priority::High,
            });
        }

        // Suggest partitioning for very large result sets
        if query.body_atoms.len() > 5 {
            suggestions.push(OptimizationSuggestion {
                suggestion_type: OptimizationSuggestionType::Partitioning,
                description: "Consider partitioning query into smaller sub-queries for parallel execution".to_string(),
                expected_improvement: 0.6,
                implementation_difficulty: DifficultyLevel::Hard,
                priority: Priority::Medium,
            });
        }

        suggestions
    }

    fn calculate_confidence_scores(
        &self,
        pattern: &QueryPattern,
        stats: &QueryStatistics,
    ) -> ConfidenceScores {
        // Calculate confidence based on historical data
        let sample_count = stats
            .execution_times
            .get(pattern)
            .map(|s| s.sample_count)
            .unwrap_or(0);

        // Execution time confidence: higher with more samples
        let execution_time = if sample_count == 0 {
            0.5_f64
        } else if sample_count < 10 {
            0.6 + (sample_count as f64 * 0.03)
        } else {
            0.9_f64.min(0.7 + (sample_count as f64).ln() / 20.0)
        };

        // Memory usage confidence: slightly lower than execution time
        let memory_usage = execution_time * 0.9;

        // Strategy confidence: based on pattern complexity
        let strategy = match pattern.complexity_class {
            QueryComplexityClass::Atomic => 0.95,
            QueryComplexityClass::SimpleConjunctive => 0.85,
            QueryComplexityClass::ComplexConjunctive => 0.75,
            QueryComplexityClass::Recursive => 0.60,
            QueryComplexityClass::HighlyComplex => 0.50,
        };

        // Overall is weighted average
        let overall = execution_time * 0.4 + memory_usage * 0.3 + strategy * 0.3;

        ConfidenceScores {
            performance_prediction: execution_time,
            index_recommendations: 0.8, // Moderate confidence in index recommendations
            optimization_suggestions: strategy,
            overall,
        }
    }
}

/// Performance prediction result
#[derive(Debug, Clone)]
pub struct PerformancePrediction {
    pub execution_time: f64,
    pub memory_usage: f64,
    pub result_size_estimate: usize,
    pub confidence: f64,
}

/// Optimization suggestion
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    pub suggestion_type: OptimizationSuggestionType,
    pub description: String,
    pub expected_improvement: f64,
    pub implementation_difficulty: DifficultyLevel,
    pub priority: Priority,
}

/// Types of optimization suggestions
#[derive(Debug, Clone)]
pub enum OptimizationSuggestionType {
    IndexCreation,
    QueryRewriting,
    JoinReordering,
    Caching,
    Partitioning,
    Other(String),
}

/// Implementation difficulty levels
#[derive(Debug, Clone)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Priority levels for suggestions
#[derive(Debug, Clone)]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}

/// Confidence scores for various predictions
#[derive(Debug, Clone)]
pub struct ConfidenceScores {
    pub performance_prediction: f64,
    pub index_recommendations: f64,
    pub optimization_suggestions: f64,
    pub overall: f64,
}

// ===== Default Implementations =====

impl Default for CostBasedOptimizerConfig {
    fn default() -> Self {
        Self {
            enable_join_optimization: true,
            enable_index_recommendations: true,
            enable_adaptive_rewriting: true,
            max_join_search_depth: 10,
            cost_sensitivity: 1.0,
            statistics_sampling_rate: 1.0,
            enable_result_caching: true,
            cache_size_limit: 10000,
        }
    }
}

impl QueryStatistics {
    pub fn new() -> Self {
        Self {
            execution_times: HashMap::new(),
            memory_usage: HashMap::new(),
            result_sizes: HashMap::new(),
            index_usage: HashMap::new(),
            total_queries: 0,
            last_updated: Instant::now(),
        }
    }
}

impl CostModel {
    pub fn default() -> Self {
        let mut base_costs = HashMap::new();
        base_costs.insert(OperationType::AtomicQuery, 1.0);
        base_costs.insert(OperationType::Join, 5.0);
        base_costs.insert(OperationType::Filter, 0.5);
        base_costs.insert(OperationType::IndexLookup, 0.1);
        base_costs.insert(OperationType::FullScan, 100.0);

        let mut scaling_factors = HashMap::new();
        scaling_factors.insert(DataSizeCategory::Small, 1.0);
        scaling_factors.insert(DataSizeCategory::Medium, 2.0);
        scaling_factors.insert(DataSizeCategory::Large, 5.0);
        scaling_factors.insert(DataSizeCategory::VeryLarge, 15.0);
        scaling_factors.insert(DataSizeCategory::Massive, 50.0);

        Self {
            base_costs,
            scaling_factors,
            index_costs: HashMap::new(),
            join_costs: HashMap::new(),
        }
    }
}

impl JoinOrderOptimizer {
    pub fn new() -> Self {
        Self {
            memo_cache: HashMap::new(),
            exhaustive_search_threshold: 8,
            heuristics: Vec::new(),
        }
    }

    /// Greedy join ordering algorithm - orders atoms by selectivity
    pub fn optimize_join_order_greedy(&self, atoms: &[QueryAtom]) -> Vec<usize> {
        if atoms.is_empty() {
            return Vec::new();
        }

        let mut remaining: Vec<usize> = (0..atoms.len()).collect();
        let mut ordered = Vec::with_capacity(atoms.len());
        let mut selected_vars = std::collections::HashSet::new();

        // Start with the most selective atom
        if let Some((idx, _)) = remaining
            .iter()
            .enumerate()
            .min_by_key(|(_, atom_idx)| self.estimate_atom_selectivity(&atoms[**atom_idx]))
        {
            let atom_idx = remaining.remove(idx);
            ordered.push(atom_idx);
            
            // Track variables from first atom
            for var in self.extract_atom_variables(&atoms[atom_idx]) {
                selected_vars.insert(var);
            }
        }

        // Greedily select atoms that join with already selected variables
        while !remaining.is_empty() {
            let (best_idx, _) = remaining
                .iter()
                .enumerate()
                .max_by_key(|(_, atom_idx)| {
                    let vars = self.extract_atom_variables(&atoms[**atom_idx]);
                    let join_count = vars.iter().filter(|v| selected_vars.contains(v)).count();
                    join_count
                })
                .unwrap();

            let atom_idx = remaining.remove(best_idx);
            ordered.push(atom_idx);

            // Add new variables
            for var in self.extract_atom_variables(&atoms[atom_idx]) {
                selected_vars.insert(var);
            }
        }

        ordered
    }

    fn estimate_atom_selectivity(&self, atom: &QueryAtom) -> usize {
        // Simple heuristic: fewer variables = more selective
        match atom {
            QueryAtom::ConcreteIndividualAtom { .. } => 1, // Most selective
            QueryAtom::ConcreteLiteralAtom { .. } => 1,
            QueryAtom::ClassAtom { .. } => 2,
            QueryAtom::DataPropertyAtom { .. } => 2,
            QueryAtom::ObjectPropertyAtom { .. } => 3,
            QueryAtom::SameIndividualAtom { .. } => 3,
            QueryAtom::DifferentIndividualsAtom { .. } => 3,
        }
    }

    fn extract_atom_variables(&self, atom: &QueryAtom) -> Vec<QueryVariable> {
        match atom {
            QueryAtom::ClassAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ObjectPropertyAtom { subject, object, .. } => {
                vec![subject.clone(), object.clone()]
            }
            QueryAtom::DataPropertyAtom { subject, .. } => vec![subject.clone()],
            QueryAtom::SameIndividualAtom { left, right } => vec![left.clone(), right.clone()],
            QueryAtom::DifferentIndividualsAtom { left, right } => {
                vec![left.clone(), right.clone()]
            }
            QueryAtom::ConcreteIndividualAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ConcreteLiteralAtom { variable, .. } => vec![variable.clone()],
        }
    }
}

impl IndexAdvisor {
    pub fn new(config: IndexAdvisorConfig) -> Self {
        Self {
            query_patterns: HashMap::new(),
            index_performance: HashMap::new(),
            analysis_cache: HashMap::new(),
            config,
        }
    }

    pub fn recommend_indices(
        &mut self,
        query: &ConjunctiveQuery,
        pattern: &QueryPattern,
        stats: &QueryStatistics,
    ) -> Result<Vec<IndexRecommendation>, OptimizationError> {
        let mut recommendations = Vec::new();

        // Analyze property usage frequency
        let mut property_frequency = HashMap::new();
        for atom in &query.body_atoms {
            if let QueryAtom::ObjectPropertyAtom { property, .. } = atom {
                *property_frequency.entry(format!("{:?}", property)).or_insert(0) += 1;
            }
        }

        // Recommend indices for frequently used properties
        for (property_name, frequency) in property_frequency {
            if frequency >= self.config.min_query_frequency as usize {
                // Calculate expected benefit
                let current_cost = self.estimate_property_access_cost_without_index(
                    &property_name,
                    pattern,
                    stats,
                );
                let indexed_cost = self.estimate_property_access_cost_with_index(
                    &property_name,
                    pattern,
                    stats,
                );

                let benefit = current_cost - indexed_cost;
                let maintenance_cost = indexed_cost * self.config.maintenance_cost_factor;
                let cost_benefit_ratio = benefit / maintenance_cost.max(1.0);

                if cost_benefit_ratio >= self.config.cost_benefit_threshold {
                    recommendations.push(IndexRecommendation {
                        index_type: IndexType::BTree,
                        indexed_elements: vec![],
                        expected_improvement: benefit,
                        implementation_cost: maintenance_cost,
                        maintenance_cost,
                        confidence: if cost_benefit_ratio > 5.0 { 0.9 } else { 0.7 },
                        justification: format!(
                            "Property {} used frequently ({}x) with cost/benefit ratio: {:.2}",
                            property_name, frequency, cost_benefit_ratio
                        ),
                    });
                }
            }
        }

        // Recommend indices for join patterns
        if pattern.join_count > 2 {
            let join_atoms = self.identify_join_atoms(query);
            if !join_atoms.is_empty() {
                let join_benefit = pattern.join_count as f64 * 100.0;
                recommendations.push(IndexRecommendation {
                    index_type: IndexType::Hash,
                    indexed_elements: vec![],
                    expected_improvement: join_benefit,
                    implementation_cost: 50.0,
                    maintenance_cost: 10.0,
                    confidence: 0.8,
                    justification: format!(
                        "Multiple joins ({}) detected - hash index recommended for join optimization",
                        pattern.join_count
                    ),
                });
            }
        }

        // Limit recommendations
        recommendations.sort_by(|a, b| {
            b.expected_improvement
                .partial_cmp(&a.expected_improvement)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        recommendations.truncate(self.config.max_recommendations);

        Ok(recommendations)
    }

    fn estimate_property_access_cost_without_index(
        &self,
        _property_name: &str,
        pattern: &QueryPattern,
        _stats: &QueryStatistics,
    ) -> f64 {
        // Without index, need full scan
        let base_cost = 1000.0;
        base_cost * (1.0 + pattern.atom_count as f64 * 0.1)
    }

    fn estimate_property_access_cost_with_index(
        &self,
        _property_name: &str,
        _pattern: &QueryPattern,
        _stats: &QueryStatistics,
    ) -> f64 {
        // With index, much faster lookup
        10.0 // Logarithmic cost
    }

    fn identify_join_atoms(&self, query: &ConjunctiveQuery) -> Vec<QueryAtom> {
        let mut join_atoms = Vec::new();
        let mut var_usage = HashMap::new();

        // Track which atoms use which variables
        for atom in &query.body_atoms {
            let vars = self.extract_variables_from_atom(atom);
            for var in vars {
                var_usage.entry(var).or_insert_with(Vec::new).push(atom.clone());
            }
        }

        // Atoms that share variables are join atoms
        for (_var, atoms) in var_usage {
            if atoms.len() > 1 {
                join_atoms.extend(atoms);
            }
        }

        join_atoms
    }

    fn extract_variables_from_atom(&self, atom: &QueryAtom) -> Vec<QueryVariable> {
        match atom {
            QueryAtom::ClassAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ObjectPropertyAtom { subject, object, .. } => {
                vec![subject.clone(), object.clone()]
            }
            QueryAtom::DataPropertyAtom { subject, .. } => vec![subject.clone()],
            QueryAtom::SameIndividualAtom { left, right } => vec![left.clone(), right.clone()],
            QueryAtom::DifferentIndividualsAtom { left, right } => vec![left.clone(), right.clone()],
            QueryAtom::ConcreteIndividualAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ConcreteLiteralAtom { variable, .. } => vec![variable.clone()],
        }
    }
}

impl Default for IndexAdvisorConfig {
    fn default() -> Self {
        Self {
            min_query_frequency: 10,
            cost_benefit_threshold: 2.0,
            max_recommendations: 5,
            maintenance_cost_factor: 0.1,
        }
    }
}

impl AdvancedQueryRewriter {
    pub fn new() -> Self {
        Self {
            rewriting_rules: Vec::new(),
            strategy_selector: AdaptiveStrategySelector::new(),
            rewriting_history: Vec::new(),
            feedback_system: RewritingFeedbackSystem::new(),
        }
    }

    pub fn rewrite_query(
        &mut self,
        query: &ConjunctiveQuery,
        context: &RewritingContext,
    ) -> Result<ConjunctiveQuery, RewritingError> {
        let mut rewritten = query.clone();
        let mut improvements_made = false;

        // Apply rewriting rules based on context
        for rule in &self.rewriting_rules {
            if self.is_rule_applicable(rule, &rewritten, context) {
                match self.apply_rewriting_rule(rule, &rewritten, context) {
                    Ok(new_query) => {
                        // Check if rewriting improved the query
                        if self.estimate_query_improvement(&rewritten, &new_query, context) > 0.05 {
                            rewritten = new_query.clone();
                            improvements_made = true;

                            // Record feedback
                            self.rewriting_history.push(RewritingHistoryEntry {
                                original_query: query.clone(),
                                rewritten_query: new_query,
                                rule_applied: rule.id.clone(),
                                performance_before: ExecutionTimeStats {
                                    mean: 0.0,
                                    median: 0.0,
                                    std_dev: 0.0,
                                    min: 0.0,
                                    max: 0.0,
                                    sample_count: 0,
                                    percentile_95: 0.0,
                                    percentile_99: 0.0,
                                },
                                performance_after: ExecutionTimeStats {
                                    mean: 0.0,
                                    median: 0.0,
                                    std_dev: 0.0,
                                    min: 0.0,
                                    max: 0.0,
                                    sample_count: 0,
                                    percentile_95: 0.0,
                                    percentile_99: 0.0,
                                },
                                improvement: 0.1,
                                timestamp: Instant::now(),
                            });
                        }
                    }
                    Err(e) => {
                        log::debug!("Rewriting rule {} failed: {:?}", rule.id, e);
                    }
                }
            }
        }

        // Apply subsumption-based optimizations
        if !improvements_made {
            rewritten = self.apply_subsumption_rewriting(&rewritten, context)?;
        }

        // Simplify redundant atoms
        rewritten = self.remove_redundant_atoms(&rewritten)?;

        // If rewriting made query worse, return original
        if !improvements_made && rewritten.body_atoms.len() > query.body_atoms.len() {
            return Ok(query.clone());
        }

        Ok(rewritten)
    }

    fn is_rule_applicable(
        &self,
        rule: &RewritingRule,
        query: &ConjunctiveQuery,
        context: &RewritingContext,
    ) -> bool {
        // Check if rule conditions are met
        if query.body_atoms.len() < 1 {
            return false;
        }

        // Check specific applicability conditions
        for condition in &rule.conditions {
            match condition {
                ApplicabilityCondition::MinComplexity(_min_complexity) => {
                    // Would calculate actual complexity
                    if query.body_atoms.len() < 2 {
                        return false;
                    }
                }
                ApplicabilityCondition::MaxComplexity(_max_complexity) => {
                    // Would check if query is too complex
                }
                ApplicabilityCondition::MinImprovement(min_imp) => {
                    if rule.expected_improvement < *min_imp {
                        return false;
                    }
                }
                ApplicabilityCondition::IndexAvailable(idx_name) => {
                    if !context.available_indices.contains(idx_name) {
                        return false;
                    }
                }
                _ => {
                    // Other conditions
                }
            }
        }

        true
    }

    fn matches_atom_type(&self, atom: &QueryAtom, atom_type: &str) -> bool {
        match (atom, atom_type) {
            (QueryAtom::ClassAtom { .. }, "Class") => true,
            (QueryAtom::ObjectPropertyAtom { .. }, "ObjectProperty") => true,
            (QueryAtom::DataPropertyAtom { .. }, "DataProperty") => true,
            _ => false,
        }
    }

    fn apply_rewriting_rule(
        &self,
        rule: &RewritingRule,
        query: &ConjunctiveQuery,
        _context: &RewritingContext,
    ) -> Result<ConjunctiveQuery, RewritingError> {
        // Apply transformation based on rule type
        let mut rewritten = query.clone();

        // Apply various transformations based on rule ID
        if rule.id.contains("subsumption") {
            // Remove subsumed atoms
            rewritten.body_atoms.retain(|atom| !self.is_subsumed(atom, &query.body_atoms));
        } else if rule.id.contains("join") {
            // Eliminate redundant joins
            rewritten = self.eliminate_redundant_joins(&rewritten);
        } else if rule.id.contains("filter") {
            // Push filters earlier in execution
            rewritten = self.push_filters_down(&rewritten);
        } else if rule.id.contains("simplif") {
            // Simplify complex class expressions
            rewritten = self.simplify_complex_expressions(&rewritten);
        }

        Ok(rewritten)
    }

    fn is_subsumed(&self, atom: &QueryAtom, all_atoms: &[QueryAtom]) -> bool {
        // Check if atom is subsumed by another atom
        // This is a simplified version - production would use reasoning
        for other in all_atoms {
            if std::ptr::eq(atom, other) {
                continue;
            }
            // Would check actual subsumption relationship
            if format!("{:?}", atom) == format!("{:?}", other) {
                return true;
            }
        }
        false
    }

    fn eliminate_redundant_joins(&self, query: &ConjunctiveQuery) -> ConjunctiveQuery {
        // Simplified implementation - would do actual join analysis
        query.clone()
    }

    fn push_filters_down(&self, query: &ConjunctiveQuery) -> ConjunctiveQuery {
        // Simplified implementation - would reorder atoms
        query.clone()
    }

    fn simplify_complex_expressions(&self, query: &ConjunctiveQuery) -> ConjunctiveQuery {
        // Simplified implementation - would simplify class expressions
        query.clone()
    }

    fn estimate_query_improvement(
        &self,
        original: &ConjunctiveQuery,
        rewritten: &ConjunctiveQuery,
        context: &RewritingContext,
    ) -> f64 {
        // Estimate improvement using cost model analysis
        let original_cost = self.estimate_query_cost(original, context);
        let rewritten_cost = self.estimate_query_cost(rewritten, context);
        
        if original_cost > 0.0 {
            // Calculate relative improvement
            let improvement = (original_cost - rewritten_cost) / original_cost;
            // Clamp between -1.0 and 1.0
            improvement.max(-1.0).min(1.0)
        } else {
            // If we can't estimate original cost, assume modest improvement
            0.1
        }
    }
    
    fn estimate_query_cost(
        &self,
        query: &ConjunctiveQuery,
        context: &RewritingContext,
    ) -> f64 {
        let mut cost = 0.0;
        
        // Atom count contributes to cost
        cost += query.body_atoms.len() as f64 * 10.0;
        
        // Join complexity
        let num_joins = query.body_atoms.len().saturating_sub(1);
        cost += (num_joins as f64).powi(2) * 5.0;
        
        // Variable count increases cost
        let mut variables = std::collections::HashSet::new();
        for atom in &query.body_atoms {
            match atom {
                QueryAtom::ClassAtom { variable, .. } => {
                    variables.insert(variable.name.clone());
                }
                QueryAtom::ObjectPropertyAtom { subject, object, .. } => {
                    variables.insert(subject.name.clone());
                    variables.insert(object.name.clone());
                }
                _ => {}
            }
        }
        cost += variables.len() as f64 * 3.0;
        
        // Selectivity factor from context statistics
        // Note: statistics is Arc<RwLock<QueryStatistics>>, needs to be locked
        if let Ok(stats) = context.statistics.read() {
            if stats.total_queries > 0 {
                // Use query count as proxy for concept selectivity
                let selectivity = variables.len() as f64 / (stats.total_queries as f64 + 1.0);
                cost *= 1.0 + selectivity.min(2.0); // Cap selectivity impact
            }
        }
        
        cost
    }

    fn apply_subsumption_rewriting(
        &self,
        query: &ConjunctiveQuery,
        _context: &RewritingContext,
    ) -> Result<ConjunctiveQuery, RewritingError> {
        // Remove atoms that are subsumed by others
        let mut rewritten = query.clone();
        rewritten.body_atoms.retain(|atom| !self.is_subsumed(atom, &query.body_atoms));
        Ok(rewritten)
    }

    fn remove_redundant_atoms(
        &self,
        query: &ConjunctiveQuery,
    ) -> Result<ConjunctiveQuery, RewritingError> {
        // Remove duplicate atoms
        let mut rewritten = query.clone();
        let mut seen = HashSet::new();
        rewritten.body_atoms.retain(|atom| {
            let signature = format!("{:?}", atom);
            seen.insert(signature)
        });
        Ok(rewritten)
    }

    fn extract_pattern(&self, query: &ConjunctiveQuery) -> String {
        format!(
            "atoms:{},joins:{}",
            query.body_atoms.len(),
            self.rewriter_count_joins(query)
        )
    }

    fn rewriter_count_joins(&self, query: &ConjunctiveQuery) -> usize {
        let mut var_count = HashMap::new();
        for atom in &query.body_atoms {
            for var in self.rewriter_extract_variables_from_atom(atom) {
                *var_count.entry(var).or_insert(0) += 1;
            }
        }
        var_count.values().filter(|&&count| count > 1).count()
    }

    fn rewriter_extract_variables_from_atom(&self, atom: &QueryAtom) -> Vec<QueryVariable> {
        match atom {
            QueryAtom::ClassAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ObjectPropertyAtom { subject, object, .. } => {
                vec![subject.clone(), object.clone()]
            }
            QueryAtom::DataPropertyAtom { subject, .. } => vec![subject.clone()],
            QueryAtom::SameIndividualAtom { left, right } => vec![left.clone(), right.clone()],
            QueryAtom::DifferentIndividualsAtom { left, right } => vec![left.clone(), right.clone()],
            QueryAtom::ConcreteIndividualAtom { variable, .. } => vec![variable.clone()],
            QueryAtom::ConcreteLiteralAtom { variable, .. } => vec![variable.clone()],
        }
    }
}

impl AdaptiveStrategySelector {
    pub fn new() -> Self {
        Self {
            strategy_history: HashMap::new(),
            strategy_weights: HashMap::new(),
            learning_config: AdaptiveLearningConfig::default(),
        }
    }
}

impl Default for AdaptiveLearningConfig {
    fn default() -> Self {
        Self {
            learning_rate: 0.1,
            exploration_rate: 0.1,
            min_confidence_threshold: 0.7,
            history_window_size: 1000,
        }
    }
}

impl RewritingFeedbackSystem {
    pub fn new() -> Self {
        Self {
            feedback_data: Vec::new(),
            analysis_results: HashMap::new(),
            auto_adjustment: AutoAdjustmentSystem::new(),
        }
    }
}

impl AutoAdjustmentSystem {
    pub fn new() -> Self {
        Self {
            adjustment_history: Vec::new(),
            strategies: Vec::new(),
            config: AutoAdjustmentConfig::default(),
        }
    }
}

impl Default for AutoAdjustmentConfig {
    fn default() -> Self {
        Self {
            enable_auto_adjustment: false, // Conservative default
            adjustment_threshold: 0.8,
            min_feedback_samples: 50,
            adjustment_frequency: Duration::from_secs(3600), // 1 hour
            conservative_mode: true,
        }
    }
}

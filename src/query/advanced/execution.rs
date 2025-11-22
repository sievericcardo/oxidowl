//! Query execution engine for high-performance conjunctive query processing
//!
//! This module implements the actual execution of optimized conjunctive queries
//! using various strategies including tableau reasoning, query rewriting, and
//! direct evaluation.

use super::conjunctive::{ConjunctiveQuery, QueryAtom, QueryConstraints, QueryVariable};
use super::optimization::{ExecutionStrategy, QueryOptimizer};
use super::rewriting::QueryRewriter;
use crate::ontology::{Individual, Literal, Ontology};
use crate::reasoning::ReasoningService;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// High-performance query execution engine
pub struct QueryEngine {
    ontology: Arc<Ontology>,
    reasoning_service: Arc<ReasoningService>,
    optimizer: QueryOptimizer,
    rewriter: QueryRewriter,
    cache: QueryCache,
    config: ExecutionConfig,
}

/// Configuration for query execution
#[derive(Debug, Clone)]
pub struct ExecutionConfig {
    /// Enable result caching
    pub enable_caching: bool,
    /// Maximum time for query execution (ms)
    pub max_execution_time: Option<u64>,
    /// Maximum number of results to return
    pub result_limit: Option<usize>,
    /// Enable parallel execution
    pub enable_parallel_execution: bool,
    /// Number of worker threads for parallel execution
    pub worker_threads: usize,
}

/// Query result cache
#[derive(Debug)]
struct QueryCache {
    results: lru::LruCache<u64, ConjunctiveQueryResult>, // query hash -> result
    statistics: CacheStatistics,
}

/// Cache performance statistics
#[derive(Debug, Default)]
struct CacheStatistics {
    hits: u64,
    misses: u64,
    evictions: u64,
}

/// Result of executing a conjunctive query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConjunctiveQueryResult {
    /// Query bindings (solutions)
    pub bindings: Vec<QueryBinding>,
    /// Query execution metadata
    #[serde(skip)]
    pub metadata: ExecutionMetadata,
    /// Whether the result set is complete or truncated
    pub complete: bool,
}

/// A single binding (solution) for a conjunctive query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryBinding {
    /// Variable bindings: variable -> bound value
    pub variable_bindings: HashMap<QueryVariable, BoundValue>,
}

/// A value that a query variable can be bound to
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BoundValue {
    /// Individual IRI
    Individual(Individual),
    /// Literal value
    Literal(Literal),
    /// Class IRI (for class variables)
    Class(String),
    /// Property IRI
    Property(String),
}

/// Metadata about query execution
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetadata {
    /// Total execution time
    pub execution_time: Duration,
    /// Time spent on optimization
    pub optimization_time: Duration,
    /// Execution strategy used
    pub strategy_used: String,
    /// Number of intermediate results processed
    pub intermediate_results: usize,
    /// Cache hit/miss information
    pub cache_hit: bool,
    /// Number of reasoning calls made
    pub reasoning_calls: usize,
    /// Memory usage statistics
    pub memory_usage: MemoryUsage,
}

/// Memory usage statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryUsage {
    /// Peak memory usage during execution (bytes)
    pub peak_memory: usize,
    /// Number of temporary data structures created
    pub temp_structures: usize,
}

/// Errors that can occur during query execution
#[derive(Debug, thiserror::Error)]
pub enum AdvancedQueryError {
    #[error("Query execution timeout after {0}ms")]
    ExecutionTimeout(u64),
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
    #[error("Reasoning error: {0}")]
    ReasoningError(String),
    #[error("Optimization error: {0}")]
    OptimizationError(#[from] super::optimization::OptimizationError),
    #[error("Rewriting error: {0}")]
    RewritingError(#[from] super::rewriting::RewritingError),
    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
    #[error("Internal error: {0}")]
    InternalError(String),
}

impl AdvancedQueryError {
    /// Create an internal error
    pub fn internal<S: Into<String>>(message: S) -> Self {
        Self::InternalError(message.into())
    }
}

impl QueryEngine {
    /// Create a new query execution engine
    pub fn new(
        ontology: Arc<Ontology>,
        reasoning_service: Arc<ReasoningService>,
    ) -> Result<Self, AdvancedQueryError> {
        let optimizer = QueryOptimizer::new(ontology.clone(), reasoning_service.clone());
        let rewriter = QueryRewriter::new(ontology.clone())
            .map_err(|e| AdvancedQueryError::RewritingError(e))?;

        Ok(Self {
            ontology: ontology.clone(),
            reasoning_service,
            optimizer,
            rewriter,
            cache: QueryCache::new(),
            config: ExecutionConfig::default(),
        })
    }

    /// Create engine with custom configuration
    pub fn with_config(
        ontology: Arc<Ontology>,
        reasoning_service: Arc<ReasoningService>,
        config: ExecutionConfig,
    ) -> Result<Self, AdvancedQueryError> {
        let mut engine = Self::new(ontology, reasoning_service)?;
        engine.config = config;
        Ok(engine)
    }

    /// Execute a conjunctive query
    pub fn execute_query(
        &mut self,
        query: &ConjunctiveQuery,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let start_time = Instant::now();

        // Check cache first
        if self.config.enable_caching {
            let query_hash = self.compute_query_hash(query);
            if let Some(cached_result) = self.cache.get(query_hash) {
                return Ok(cached_result);
            }
        }

        // Validate query
        self.validate_query(query)?;

        // Optimize query
        let optimization_start = Instant::now();
        let plan = self.optimizer.optimize(query)?;
        let optimization_time = optimization_start.elapsed();

        // Execute query according to plan
        let mut result = match plan.strategy {
            ExecutionStrategy::Direct => self.execute_direct(&plan.optimized_query)?,
            ExecutionStrategy::Tableau {
                ref expansion_order,
            } => self.execute_tableau(&plan.optimized_query, expansion_order)?,
            ExecutionStrategy::Rewriting {
                ref rewritten_queries,
            } => self.execute_rewriting(&plan.optimized_query, rewritten_queries)?,
            ExecutionStrategy::Hybrid {
                ref tableau_atoms,
                ref rewriting_atoms,
            } => self.execute_hybrid(&plan.optimized_query, tableau_atoms, rewriting_atoms)?,
        };

        // Update metadata
        result.metadata.execution_time = start_time.elapsed();
        result.metadata.optimization_time = optimization_time;
        result.metadata.strategy_used = format!("{:?}", plan.strategy);

        // Cache result if enabled
        if self.config.enable_caching {
            let query_hash = self.compute_query_hash(query);
            self.cache.put(query_hash, result.clone());
        }

        Ok(result)
    }

    /// Execute query using direct evaluation (for simple queries)
    fn execute_direct(
        &mut self,
        query: &ConjunctiveQuery,
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let mut bindings = Vec::new();
        let mut reasoning_calls = 0;

        // For simple queries, we can evaluate directly
        if query.body_atoms.len() == 1 {
            match &query.body_atoms[0] {
                QueryAtom::ClassAtom {
                    variable,
                    class_expression,
                } => {
                    // Find all instances of the class
                    if let Ok(instances) =
                        self.reasoning_service.get_instances_sync(class_expression)
                    {
                        reasoning_calls += 1;
                        for instance in instances {
                            let mut binding = QueryBinding::new();
                            binding
                                .bind_variable(variable.clone(), BoundValue::Individual(instance));
                            bindings.push(binding);
                        }
                    }
                }
                QueryAtom::ObjectPropertyAtom {
                    subject,
                    property,
                    object,
                } => {
                    // Find all property assertions
                    if let Ok(assertions) = self
                        .reasoning_service
                        .get_object_property_assertions_sync(property)
                    {
                        reasoning_calls += 1;
                        for (subj, obj) in assertions {
                            let mut binding = QueryBinding::new();
                            binding.bind_variable(subject.clone(), BoundValue::Individual(subj));
                            binding.bind_variable(object.clone(), BoundValue::Individual(obj));
                            bindings.push(binding);
                        }
                    }
                }
                _ => {
                    // Fall back to tableau execution for other atom types
                    return self.execute_tableau(query, &query.body_atoms);
                }
            }
        } else {
            // Multiple atoms require more complex evaluation
            return self.execute_tableau(query, &query.body_atoms);
        }

        // Apply result limit
        let complete = if let Some(limit) = self.config.result_limit {
            if bindings.len() > limit {
                bindings.truncate(limit);
                false
            } else {
                true
            }
        } else {
            true
        };

        Ok(ConjunctiveQueryResult {
            bindings,
            metadata: ExecutionMetadata {
                execution_time: Duration::from_millis(0), // Will be set by caller
                optimization_time: Duration::from_millis(0),
                strategy_used: "Direct".to_string(),
                intermediate_results: 0,
                cache_hit: false,
                reasoning_calls,
                memory_usage: MemoryUsage::default(),
            },
            complete,
        })
    }

    /// Execute query using tableau reasoning
    fn execute_tableau(
        &mut self,
        query: &ConjunctiveQuery,
        expansion_order: &[QueryAtom],
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let mut bindings = Vec::new();
        let mut reasoning_calls = 0;

        // Create initial binding with free variables
        let mut current_bindings = vec![QueryBinding::new()];

        // Process atoms in the specified order
        for atom in expansion_order {
            let mut next_bindings = Vec::new();

            for current_binding in current_bindings {
                let atom_bindings = self.evaluate_atom_with_binding(atom, &current_binding)?;
                reasoning_calls += atom_bindings.len();

                for atom_binding in atom_bindings {
                    if let Some(combined_binding) = current_binding.combine(&atom_binding) {
                        next_bindings.push(combined_binding);
                    }
                }
            }

            current_bindings = next_bindings;

            // Check timeout
            if let Some(timeout) = self.config.max_execution_time {
                // Simplified timeout check - in practice would track elapsed time
                if current_bindings.len() > 10000 {
                    return Err(AdvancedQueryError::ExecutionTimeout(timeout));
                }
            }
        }

        // Filter to answer variables only
        for binding in current_bindings {
            let answer_binding = binding.project(&query.answer_variables);
            bindings.push(answer_binding);
        }

        // Remove duplicates and apply constraints
        bindings = self.apply_constraints(&bindings, &query.constraints)?;
        bindings.dedup();

        // Apply result limit
        let complete = if let Some(limit) = self.config.result_limit {
            if bindings.len() > limit {
                bindings.truncate(limit);
                false
            } else {
                true
            }
        } else {
            true
        };

        Ok(ConjunctiveQueryResult {
            bindings,
            metadata: ExecutionMetadata {
                execution_time: Duration::from_millis(0), // Will be set by caller
                optimization_time: Duration::from_millis(0),
                strategy_used: "Tableau".to_string(),
                intermediate_results: reasoning_calls,
                cache_hit: false,
                reasoning_calls,
                memory_usage: MemoryUsage::default(),
            },
            complete,
        })
    }

    /// Execute query using OWL 2 QL rewriting
    fn execute_rewriting(
        &mut self,
        _query: &ConjunctiveQuery,
        rewritten_queries: &[ConjunctiveQuery],
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        let mut all_bindings = Vec::new();
        let mut total_reasoning_calls = 0;

        // Execute each rewritten query
        for rewritten_query in rewritten_queries {
            let result = self.execute_tableau(rewritten_query, &rewritten_query.body_atoms)?;
            all_bindings.extend(result.bindings);
            total_reasoning_calls += result.metadata.reasoning_calls;
        }

        // Remove duplicates
        all_bindings.dedup();

        // Apply result limit
        let complete = if let Some(limit) = self.config.result_limit {
            if all_bindings.len() > limit {
                all_bindings.truncate(limit);
                false
            } else {
                true
            }
        } else {
            true
        };

        Ok(ConjunctiveQueryResult {
            bindings: all_bindings,
            metadata: ExecutionMetadata {
                execution_time: Duration::from_millis(0), // Will be set by caller
                optimization_time: Duration::from_millis(0),
                strategy_used: "Rewriting".to_string(),
                intermediate_results: rewritten_queries.len(),
                cache_hit: false,
                reasoning_calls: total_reasoning_calls,
                memory_usage: MemoryUsage::default(),
            },
            complete,
        })
    }

    /// Execute query using hybrid strategy
    fn execute_hybrid(
        &mut self,
        query: &ConjunctiveQuery,
        tableau_atoms: &[QueryAtom],
        _rewriting_atoms: &[QueryAtom],
    ) -> Result<ConjunctiveQueryResult, AdvancedQueryError> {
        // For now, fall back to tableau execution
        // In practice, this would intelligently combine both approaches
        self.execute_tableau(query, tableau_atoms)
    }

    /// Evaluate a single atom in the context of existing bindings
    fn evaluate_atom_with_binding(
        &mut self,
        atom: &QueryAtom,
        binding: &QueryBinding,
    ) -> Result<Vec<QueryBinding>, AdvancedQueryError> {
        match atom {
            QueryAtom::ClassAtom {
                variable,
                class_expression,
            } => {
                if let Some(bound_value) = binding.get_binding(variable) {
                    // Variable is already bound - check if it satisfies the class
                    if let BoundValue::Individual(individual) = bound_value {
                        if self
                            .reasoning_service
                            .is_instance_of_sync(individual, class_expression)
                            .unwrap_or(false)
                        {
                            Ok(vec![QueryBinding::new()])
                        } else {
                            Ok(vec![])
                        }
                    } else {
                        Ok(vec![])
                    }
                } else {
                    // Variable is free - find all instances
                    if let Ok(instances) =
                        self.reasoning_service.get_instances_sync(class_expression)
                    {
                        Ok(instances
                            .into_iter()
                            .map(|instance| {
                                let mut new_binding = QueryBinding::new();
                                new_binding.bind_variable(
                                    variable.clone(),
                                    BoundValue::Individual(instance),
                                );
                                new_binding
                            })
                            .collect())
                    } else {
                        Ok(vec![])
                    }
                }
            }
            _ => {
                // Simplified - other atom types would be handled similarly
                Ok(vec![QueryBinding::new()])
            }
        }
    }

    /// Apply query constraints to filter bindings
    fn apply_constraints(
        &self,
        bindings: &[QueryBinding],
        _constraints: &QueryConstraints,
    ) -> Result<Vec<QueryBinding>, AdvancedQueryError> {
        // Simplified - in practice would apply distinct, type, and value constraints
        Ok(bindings.to_vec())
    }

    /// Validate that a query is well-formed and executable
    fn validate_query(&self, query: &ConjunctiveQuery) -> Result<(), AdvancedQueryError> {
        if query.body_atoms.is_empty() {
            return Err(AdvancedQueryError::InvalidQuery(
                "Query has no body atoms".to_string(),
            ));
        }

        if !query.is_well_formed() {
            return Err(AdvancedQueryError::InvalidQuery(
                "Query is not well-formed".to_string(),
            ));
        }

        Ok(())
    }

    /// Compute hash for a query (for caching)
    fn compute_query_hash(&self, query: &ConjunctiveQuery) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // Hash answer variables
        for var in &query.answer_variables {
            var.hash(&mut hasher);
        }

        // Hash body atoms
        for atom in &query.body_atoms {
            // Simple structural hash
            std::mem::discriminant(atom).hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Get cache statistics
    pub fn cache_statistics(&self) -> &CacheStatistics {
        &self.cache.statistics
    }
}

impl QueryBinding {
    /// Create a new empty binding
    pub fn new() -> Self {
        Self {
            variable_bindings: HashMap::new(),
        }
    }

    /// Bind a variable to a value
    pub fn bind_variable(&mut self, variable: QueryVariable, value: BoundValue) {
        self.variable_bindings.insert(variable, value);
    }

    /// Get the binding for a variable
    pub fn get_binding(&self, variable: &QueryVariable) -> Option<&BoundValue> {
        self.variable_bindings.get(variable)
    }

    /// Combine two bindings if compatible
    pub fn combine(&self, other: &QueryBinding) -> Option<QueryBinding> {
        let mut combined = self.clone();

        for (var, value) in &other.variable_bindings {
            if let Some(existing_value) = combined.variable_bindings.get(var) {
                if existing_value != value {
                    return None; // Incompatible bindings
                }
            } else {
                combined
                    .variable_bindings
                    .insert(var.clone(), value.clone());
            }
        }

        Some(combined)
    }

    /// Project binding to only include specified variables
    pub fn project(&self, variables: &[QueryVariable]) -> QueryBinding {
        let mut projected = QueryBinding::new();

        for var in variables {
            if let Some(value) = self.variable_bindings.get(var) {
                projected
                    .variable_bindings
                    .insert(var.clone(), value.clone());
            }
        }

        projected
    }
}

impl QueryCache {
    fn new() -> Self {
        Self {
            results: lru::LruCache::new(
                std::num::NonZeroUsize::new(100).expect("Hardcoded non-zero value for cache size"),
            ),
            statistics: CacheStatistics::default(),
        }
    }

    fn get(&mut self, key: u64) -> Option<ConjunctiveQueryResult> {
        if let Some(result) = self.results.get(&key) {
            self.statistics.hits += 1;
            Some(result.clone())
        } else {
            self.statistics.misses += 1;
            None
        }
    }

    fn put(&mut self, key: u64, result: ConjunctiveQueryResult) {
        if self.results.put(key, result).is_some() {
            self.statistics.evictions += 1;
        }
    }
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            max_execution_time: Some(30000), // 30 seconds
            result_limit: Some(10000),
            enable_parallel_execution: false, // Disabled by default for safety
            worker_threads: 4,
        }
    }
}

impl Default for QueryBinding {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for BoundValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundValue::Individual(individual) => write!(f, "{}", individual),
            BoundValue::Literal(literal) => write!(f, "{}", literal),
            BoundValue::Class(class) => write!(f, "{}", class),
            BoundValue::Property(property) => write!(f, "{}", property),
        }
    }
}

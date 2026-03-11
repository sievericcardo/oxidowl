//! Advanced query processing module
//!
//! This module implements high-performance conjunctive query answering with:
//! - SPARQL-like query capabilities
//! - OWL 2 QL query rewriting optimization
//! - Efficient query execution strategies
//! - Advanced optimization with ML-driven strategies
//! - Industrial-strength optimizations and ML-enhanced heuristics

pub mod conjunctive;
pub mod execution;
pub mod feature_extraction;
pub mod ml_models;
pub mod optimization;
pub mod optimizer; // Advanced optimizer
pub mod rewriting;

// Core advanced components (using existing enhanced modules)
pub mod actors; // Phase 3: execution engine actors
pub mod cost_optimizer; // Enhanced with cost-based optimization
pub mod execution_engine; // Enhanced with advanced execution

// Industrial-strength components
pub mod industrial;
pub mod ml_heuristics;
pub mod performance_benchmarking;

// ML-Enhanced Query Optimization
pub mod ml_core; // ML infrastructure and models

// Test modules
#[cfg(test)]
pub mod extended_integration_tests;

// Additional test modules (temporarily disabled - waiting for full implementation)
// #[cfg(test)]
// pub mod industrial_tests;
// #[cfg(test)]
// pub mod ml_heuristics_tests;
// #[cfg(test)]
// pub mod benchmarking_tests;
// #[cfg(test)]
// pub mod integration_tests;

pub use conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
pub use execution::{AdvancedQueryError, ConjunctiveQueryResult, QueryEngine};
pub use feature_extraction::{DLQueryFeatureExtractor, FeatureExtractionConfig};
pub use ml_models::{EnsembleModel, LinearRegressionModel, NeuralNetworkModel};
pub use optimization::QueryOptimizer;
pub use optimizer::{
    AdvancedOptimizerConfig, AdvancedQueryOptimizer, AdvancedQueryPlan, IntelligentIndexingSystem,
    PerformanceMonitor, PerformancePredictor,
};
pub use rewriting::QueryRewriter;

// Advanced exports from enhanced modules
pub use cost_optimizer::{
    AdvancedQueryRewriter, CostBasedOptimizer, CostBasedOptimizerConfig, CostModel, IndexAdvisor,
    JoinOrderOptimizer, QueryStatistics,
};
pub use execution_engine::{
    AdvancedExecutionConfig, AdvancedExecutionEngine, CacheConfig, ExecutionConstraints,
    ExecutionId, ExecutionPerformanceMonitor, ExecutionPriority, ExecutionStrategySelector,
    ParallelExecutionConfig, ParallelExecutionCoordinator, QueryResultCache,
};

// Industrial-strength exports
pub use industrial::{
    IndustrialClassificationResult, IndustrialOptimizer, IndustrialQueryOptimizer,
    LargeOntologyConfig, LargeScaleStrategy,
};
pub use ml_heuristics::{MLError, MLHeuristicsConfig, MLHeuristicsEngine, ReasoningStrategy};
pub use performance_benchmarking::{
    BenchmarkingConfig, CompetitiveAnalysisReport, IndustrialBenchmarkReport,
    PerformanceBenchmarkingSystem,
};

// ML-Enhanced Query Optimization exports
pub use ml_core::{
    CostPrediction, MLHeuristicsConfig as MLConfig, MLHeuristicsEngine as MLEngine, QueryExecution,
    QueryFeatureExtractor, QueryFeatures, TrainingMetrics,
};

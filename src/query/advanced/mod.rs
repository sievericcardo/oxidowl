//! Advanced query processing module
//! 
//! This module implements high-performance conjunctive query answering with:
//! - SPARQL-like query capabilities
//! - OWL 2 QL query rewriting optimization
//! - Efficient query execution strategies
//! - Advanced optimization with ML-driven strategies
//! - Industrial-strength optimizations and ML-enhanced heuristics

pub mod conjunctive;
pub mod rewriting;
pub mod optimization;
pub mod execution;
pub mod optimizer; // Advanced optimizer
pub mod feature_extraction;
pub mod ml_models;

// Core advanced components (using existing enhanced modules)
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

// Test modules (TODO: Fix compilation errors in these test files)
// #[cfg(test)]
// pub mod industrial_tests;
// #[cfg(test)]
// pub mod ml_heuristics_tests;
// #[cfg(test)]
// pub mod benchmarking_tests;
// #[cfg(test)]
// pub mod integration_tests;

pub use conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
pub use rewriting::QueryRewriter;
pub use optimization::QueryOptimizer;
pub use execution::{QueryEngine, ConjunctiveQueryResult, AdvancedQueryError};
pub use optimizer::{
    AdvancedQueryOptimizer, AdvancedQueryPlan, AdvancedOptimizerConfig,
    PerformancePredictor, IntelligentIndexingSystem, PerformanceMonitor
};
pub use feature_extraction::{DLQueryFeatureExtractor, FeatureExtractionConfig};
pub use ml_models::{LinearRegressionModel, NeuralNetworkModel, EnsembleModel};

// Advanced exports from enhanced modules
pub use cost_optimizer::{
    CostBasedOptimizer, CostBasedOptimizerConfig, QueryStatistics, CostModel, 
    JoinOrderOptimizer, IndexAdvisor, AdvancedQueryRewriter
};
pub use execution_engine::{
    AdvancedExecutionEngine, AdvancedExecutionConfig, QueryResultCache,
    ExecutionStrategySelector, ExecutionPerformanceMonitor, ParallelExecutionCoordinator,
    ExecutionConstraints, ExecutionPriority, ExecutionId, CacheConfig, ParallelExecutionConfig
};

// Industrial-strength exports
pub use industrial::{
    IndustrialOptimizer, LargeOntologyConfig, IndustrialClassificationResult,
    LargeScaleStrategy, IndustrialQueryOptimizer
};
pub use ml_heuristics::{
    MLHeuristicsEngine, MLHeuristicsConfig, ReasoningStrategy, MLError
};
pub use performance_benchmarking::{
    PerformanceBenchmarkingSystem, BenchmarkingConfig, IndustrialBenchmarkReport, 
    CompetitiveAnalysisReport
};

// ML-Enhanced Query Optimization exports
pub use ml_core::{
    MLHeuristicsEngine as MLEngine, 
    MLHeuristicsConfig as MLConfig,
    QueryFeatures, 
    QueryFeatureExtractor,
    CostPrediction,
    QueryExecution,
    TrainingMetrics,
};
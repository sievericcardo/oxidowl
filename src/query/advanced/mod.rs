//! Advanced query processing module
//! 
//! This module implements high-performance conjunctive query answering with:
//! - SPARQL-like query capabilities
//! - OWL 2 QL query rewriting optimization
//! - Efficient query execution strategies
//! - Phase 2: Advanced optimization with ML-driven strategies
//! - Phase 3: Industrial-strength optimizations and ML-enhanced heuristics

pub mod conjunctive;
pub mod rewriting;
pub mod optimization;
pub mod execution;
pub mod optimizer; // Phase 2: Advanced optimizer
pub mod feature_extraction;
pub mod ml_models;

// Phase 3: Industrial-strength components
pub mod industrial;
pub mod ml_heuristics;
pub mod performance_benchmarking;

// Phase 3: Test modules
#[cfg(test)]
pub mod industrial_tests;
#[cfg(test)]
pub mod ml_heuristics_tests;
#[cfg(test)]
pub mod benchmarking_tests;
#[cfg(test)]
pub mod integration_tests;

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

// Phase 3: Industrial-strength exports
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
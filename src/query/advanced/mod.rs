//! Advanced query processing module
//! 
//! This module implements high-performance conjunctive query answering with:
//! - SPARQL-like query capabilities
//! - OWL 2 QL query rewriting optimization
//! - Efficient query execution strategies
//! - Phase 2: Advanced optimization with ML-driven strategies

pub mod conjunctive;
pub mod rewriting;
pub mod optimization;
pub mod execution;
pub mod phase2_optimization;
pub mod feature_extraction;
pub mod ml_models;

pub use conjunctive::{ConjunctiveQuery, QueryAtom, QueryVariable};
pub use rewriting::QueryRewriter;
pub use optimization::QueryOptimizer;
pub use execution::{QueryEngine, ConjunctiveQueryResult, AdvancedQueryError};
pub use phase2_optimization::{
    AdvancedQueryOptimizer, AdvancedQueryPlan, AdvancedOptimizerConfig,
    PerformancePredictor, IntelligentIndexingSystem, PerformanceMonitor
};
pub use feature_extraction::{DLQueryFeatureExtractor, FeatureExtractionConfig};
pub use ml_models::{LinearRegressionModel, NeuralNetworkModel, EnsembleModel};
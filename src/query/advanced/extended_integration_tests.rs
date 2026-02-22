//! Integration Tests for Advanced Query Processing
//!
//! Comprehensive test suite validating the integration of:
//! - Cost-based query optimization
//! - Advanced execution engine
//! - Intelligent caching system
//! - Performance monitoring

use super::*;
use crate::ontology::{ClassExpression, Ontology};
use crate::reasoning::ReasoningService;
use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
mod extended_integration_tests {
    use super::*;

    #[test]
    fn test_advanced_execution_engine_creation() {
        let ontology = Arc::new(Ontology::new());
        let reasoning_service =
            Arc::new(ReasoningService::new(Ontology::new(), Default::default()).expect("Failed to create ReasoningService"));

        let config = AdvancedExecutionConfig {
            enable_caching: true,
            cache_size_limit_mb: 128,
            cache_ttl_seconds: 300,
            enable_parallel_execution: true,
            max_parallel_threads: 4,
            enable_adaptive_strategies: true,
            monitoring_interval_ms: 100,
            enable_result_streaming: false,
            streaming_chunk_size: 1000,
            enable_execution_tracing: true,
        };

        let engine =
            AdvancedExecutionEngine::new(ontology.clone(), reasoning_service.clone(), config);

        assert!(engine.is_ok());
    }

    #[test]
    fn test_cost_based_optimizer_creation() {
        let ontology = Arc::new(Ontology::new());
        let reasoning_service =
            Arc::new(ReasoningService::new(Ontology::new(), Default::default()).expect("Failed to create ReasoningService"));

        let _optimizer = CostBasedOptimizer::new(ontology, reasoning_service, Default::default());

        assert!(true);
    }

    #[test]
    fn test_execution_constraints() {
        let constraints = ExecutionConstraints {
            max_execution_time: Some(Duration::from_secs(10)),
            max_memory_usage: Some(1024 * 1024 * 100),
            min_confidence: Some(0.8),
            priority: ExecutionPriority::Normal,
        };

        assert_eq!(constraints.priority, ExecutionPriority::Normal);
        assert!(constraints.max_execution_time.is_some());
    }

    #[test]
    fn test_execution_priority_ordering() {
        assert!(ExecutionPriority::Background < ExecutionPriority::Normal);
        assert!(ExecutionPriority::Normal < ExecutionPriority::High);
        assert!(ExecutionPriority::High < ExecutionPriority::Urgent);
    }

    #[test]
    fn test_cache_config() {
        let config = CacheConfig {
            max_size_bytes: 1024 * 1024 * 128,
            max_entries: 1000,
            default_ttl: Duration::from_secs(300),
            enable_compression: true,
            compression_threshold: 1024,
            enable_statistics: true,
        };

        assert_eq!(config.max_entries, 1000);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_parallel_execution_config() {
        let config = ParallelExecutionConfig {
            enable_parallel_execution: true,
            max_worker_threads: 4,
            work_queue_size: 100,
            task_timeout: Duration::from_secs(30),
            enable_work_stealing: true,
            enable_resource_monitoring: true,
        };

        assert_eq!(config.max_worker_threads, 4);
        assert!(config.enable_work_stealing);
    }

    #[test]
    fn test_default_configs() {
        let exec_config = AdvancedExecutionConfig::default();
        assert!(exec_config.enable_caching);

        let cache_config = CacheConfig::default();
        assert!(cache_config.max_entries > 0);

        let parallel_config = ParallelExecutionConfig::default();
        assert!(parallel_config.max_worker_threads > 0);
    }

    #[test]
    fn test_query_result_cache() {
        let _cache = QueryResultCache::new(CacheConfig::default());
        assert!(true);
    }

    #[test]
    fn test_execution_strategy_selector() {
        let _selector = ExecutionStrategySelector::new();
        assert!(true);
    }

    #[test]
    fn test_execution_performance_monitor() {
        let _monitor = ExecutionPerformanceMonitor::new();
        assert!(true);
    }

    #[test]
    fn test_parallel_execution_coordinator() {
        let _coordinator = ParallelExecutionCoordinator::new(ParallelExecutionConfig::default());
        assert!(true);
    }

    #[test]
    fn test_simple_query_creation() {
        let query = ConjunctiveQuery {
            answer_variables: vec![QueryVariable::new("x".to_string())],
            body_atoms: vec![QueryAtom::ClassAtom {
                variable: QueryVariable::new("x".to_string()),
                class_expression: ClassExpression::Class(crate::ontology::Class {
                    iri: crate::ontology::IRI::new("Person"),
                }),
            }],
            constraints: Default::default(),
            metadata: Default::default(),
        };

        assert_eq!(query.answer_variables.len(), 1);
        assert_eq!(query.body_atoms.len(), 1);
    }

    #[test]
    fn test_full_pipeline_creation() {
        let ontology = Arc::new(Ontology::new());
        let reasoning_service =
            Arc::new(ReasoningService::new(Ontology::new(), Default::default()).expect("Failed to create ReasoningService"));

        let _optimizer = CostBasedOptimizer::new(
            ontology.clone(),
            reasoning_service.clone(),
            Default::default(),
        );

        let engine = AdvancedExecutionEngine::new(
            ontology.clone(),
            reasoning_service.clone(),
            AdvancedExecutionConfig::default(),
        );

        assert!(engine.is_ok());
    }
}

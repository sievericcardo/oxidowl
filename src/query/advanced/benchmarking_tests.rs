//! Tests for the performance benchmarking components.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ontology::Ontology;
    use crate::reasoning::ReasoningService;
    use std::sync::Arc;

    fn make_components() -> (
        AdvancedQueryOptimizer,
        IndustrialOptimizer,
        MLHeuristicsEngine,
        PerformanceBenchmarkingSystem,
    ) {
        let ontology = Ontology::new();
        let ontology_arc = Arc::new(ontology.clone());
        let reasoning = Arc::new(
            ReasoningService::new(ontology, Default::default())
                .expect("Failed to create ReasoningService"),
        );
        let optimizer = AdvancedQueryOptimizer::new(
            ontology_arc,
            reasoning,
            AdvancedOptimizerConfig::default(),
        );
        let industrial = IndustrialOptimizer::new(LargeOntologyConfig::default());
        let ml_heuristics = MLHeuristicsEngine::new(MLHeuristicsConfig::default());
        let system = PerformanceBenchmarkingSystem::new(BenchmarkingConfig::default());

        (optimizer, industrial, ml_heuristics, system)
    }

    #[tokio::test]
    async fn test_synthetic_benchmarks_produce_aggregate_metrics() {
        let (mut optimizer, mut industrial, mut ml, mut system) = make_components();

        let report = system
            .run_synthetic_benchmarks(&mut optimizer, &mut industrial, &mut ml)
            .await
            .expect("Synthetic benchmarks should succeed");

        assert!(report.aggregate_metrics.total_ontologies_tested > 0);
        assert!((0.0..=1.0).contains(&report.aggregate_metrics.overall_success_rate));
    }

    #[tokio::test]
    async fn test_competitive_analysis_produces_comparisons() {
        let (mut optimizer, mut industrial, mut ml, mut system) = make_components();

        let report = system
            .run_synthetic_benchmarks(&mut optimizer, &mut industrial, &mut ml)
            .await
            .expect("Synthetic benchmarks should succeed");

        let analysis = system
            .run_competitive_analysis(&report)
            .expect("Competitive analysis should succeed");

        assert!(
            !analysis.baseline_comparisons.is_empty(),
            "Competitive analysis should produce baseline comparisons"
        );
    }
}

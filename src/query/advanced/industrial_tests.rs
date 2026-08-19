//! Tests for the industrial-scale ontology optimization components.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ontology::{Class, IRI, Ontology};
    use crate::reasoning::ReasoningService;
    use std::sync::Arc;

    fn build_ontology(class_count: usize) -> Ontology {
        let mut ontology = Ontology::new();
        for i in 0..class_count {
            ontology.add_class(Class::new(IRI::new(&format!("http://ex.org/C{i}"))));
        }
        ontology
    }

    #[test]
    fn test_large_ontology_config_defaults() {
        let config = LargeOntologyConfig::default();
        assert!(config.large_ontology_threshold > 0);
        assert!(config.memory_limit_gb > 0.0);
        assert!(config.concept_chunk_size > 0);
    }

    #[tokio::test]
    async fn test_small_ontology_uses_standard_optimization() {
        let ontology = build_ontology(10);
        let ontology_arc = Arc::new(ontology.clone());
        let reasoning = Arc::new(
            ReasoningService::new(ontology.clone(), Default::default())
                .expect("Failed to create ReasoningService"),
        );
        let mut optimizer = AdvancedQueryOptimizer::new(
            ontology_arc,
            reasoning,
            AdvancedOptimizerConfig::default(),
        );
        let mut industrial = IndustrialOptimizer::new(LargeOntologyConfig::default());

        let result = industrial
            .optimize_large_ontology_classification(&ontology, &mut optimizer)
            .expect("Industrial optimization should succeed");

        match result {
            IndustrialClassificationResult::StandardOptimization {
                concept_count,
                reason,
            } => {
                assert_eq!(concept_count, 10);
                assert!(
                    reason.contains("threshold"),
                    "Reason should mention the size threshold, got: {reason}"
                );
            }
            other => panic!("Expected StandardOptimization for a small ontology, got: {other:?}"),
        }
    }

    #[test]
    fn test_large_scale_strategy_variants() {
        // The strategy enum must expose the four strategies used by the
        // large-ontology classification pipeline.
        let strategies = [
            LargeScaleStrategy::Hierarchical,
            LargeScaleStrategy::Modular,
            LargeScaleStrategy::Distributed,
            LargeScaleStrategy::Hybrid(vec![LargeScaleStrategy::Hierarchical]),
        ];
        assert_eq!(strategies.len(), 4);
    }
}

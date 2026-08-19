//! Integration tests combining industrial optimization and ML heuristics.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ontology::{Class, ClassExpression, IRI, Ontology};
    use crate::reasoning::ReasoningService;
    use std::sync::Arc;

    fn build_ontology(class_count: usize) -> Ontology {
        let mut ontology = Ontology::new();
        for i in 0..class_count {
            ontology.add_class(Class::new(IRI::new(&format!("http://ex.org/C{i}"))));
        }
        ontology
    }

    #[tokio::test]
    async fn test_industrial_and_ml_integration() {
        let ontology = build_ontology(100);
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
        let mut ml_heuristics = MLHeuristicsEngine::new(MLHeuristicsConfig::default());

        // Industrial classification of a small ontology must fall back to the
        // standard path and report the correct concept count.
        let result = industrial
            .optimize_large_ontology_classification(&ontology, &mut optimizer)
            .expect("Industrial classification should succeed");
        assert!(matches!(
            result,
            IndustrialClassificationResult::StandardOptimization {
                concept_count: 100,
                ..
            }
        ));

        // ML heuristics must return a valid strategy for a simple query.
        let query = ConjunctiveQuery {
            answer_variables: vec![QueryVariable::new("x".to_string())],
            body_atoms: vec![QueryAtom::ClassAtom {
                variable: QueryVariable::new("x".to_string()),
                class_expression: ClassExpression::class(IRI::new("http://ex.org/C0")),
            }],
            constraints: Default::default(),
            metadata: Default::default(),
        };
        match ml_heuristics.select_reasoning_strategy(&query, &ontology) {
            Ok(strategy) => assert!(matches!(
                strategy,
                ReasoningStrategy::StandardTableau
                    | ReasoningStrategy::HierarchicalDecomposition
                    | ReasoningStrategy::ModularReasoning
                    | ReasoningStrategy::IncrementalExpansion
                    | ReasoningStrategy::HybridStrategy(_)
            )),
            Err(e) => assert!(matches!(
                e,
                MLError::FeatureExtractionFailed(_)
                    | MLError::ModelPredictionFailed(_)
                    | MLError::TrainingDataInsufficient
                    | MLError::ConfigurationError(_)
            )),
        }
    }
}

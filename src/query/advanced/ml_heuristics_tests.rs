//! Tests for ML-driven reasoning heuristics.

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::ontology::{Class, ClassExpression, IRI, Ontology};

    fn build_ontology() -> Ontology {
        let mut ontology = Ontology::new();
        ontology.add_class(Class::new(IRI::new("http://ex.org/A")));
        ontology.add_class(Class::new(IRI::new("http://ex.org/B")));
        ontology
    }

    fn simple_query() -> ConjunctiveQuery {
        ConjunctiveQuery {
            answer_variables: vec![QueryVariable::new("x".to_string())],
            body_atoms: vec![QueryAtom::ClassAtom {
                variable: QueryVariable::new("x".to_string()),
                class_expression: ClassExpression::class(IRI::new("http://ex.org/A")),
            }],
            constraints: Default::default(),
            metadata: Default::default(),
        }
    }

    #[test]
    fn test_strategy_selection_disabled_returns_standard_tableau() {
        let mut config = MLHeuristicsConfig::default();
        config.enable_strategy_selection = false;
        let mut engine = MLHeuristicsEngine::new(config);

        let strategy = engine
            .select_reasoning_strategy(&simple_query(), &build_ontology())
            .expect("Strategy selection should succeed when disabled");
        assert!(matches!(strategy, ReasoningStrategy::StandardTableau));
    }

    #[test]
    fn test_strategy_selection_enabled_returns_valid_strategy() {
        let mut engine = MLHeuristicsEngine::new(MLHeuristicsConfig::default());

        match engine.select_reasoning_strategy(&simple_query(), &build_ontology()) {
            Ok(strategy) => assert!(matches!(
                strategy,
                ReasoningStrategy::StandardTableau
                    | ReasoningStrategy::HierarchicalDecomposition
                    | ReasoningStrategy::ModularReasoning
                    | ReasoningStrategy::IncrementalExpansion
                    | ReasoningStrategy::HybridStrategy(_)
            )),
            // An untrained model may fail with a specific ML error, but never a
            // silent fallback or an unrelated error.
            Err(e) => assert!(matches!(
                e,
                MLError::FeatureExtractionFailed(_)
                    | MLError::ModelPredictionFailed(_)
                    | MLError::TrainingDataInsufficient
                    | MLError::ConfigurationError(_)
            )),
        }
    }

    #[test]
    fn test_predict_expansion_order_empty() {
        let mut engine = MLHeuristicsEngine::new(MLHeuristicsConfig::default());

        let priorities = engine
            .predict_expansion_order(&[], &build_ontology())
            .expect("Expansion prediction should succeed for empty input");
        assert!(priorities.is_empty());
    }

    #[test]
    fn test_heuristics_report_initial_state() {
        let engine = MLHeuristicsEngine::new(MLHeuristicsConfig::default());
        let report = engine.generate_heuristics_report();

        assert_eq!(report.total_sessions, 0);
        assert!(report.strategy_selection_accuracy.is_finite());
        assert!(report.expansion_prediction_accuracy.is_finite());
    }
}

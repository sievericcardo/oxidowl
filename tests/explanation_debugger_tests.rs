#[cfg(test)]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::debug::{BlackBoxOWLDebugger, DebuggerConfig, OWLDebugger};
use oxidowl::explanation::blackbox::{BlackBoxConfig, BlackBoxExplanation};
use oxidowl::explanation::converter::SatisfiabilityConverter;
use oxidowl::explanation::generator::{Explanation as Justification, ExplanationGenerator};
use oxidowl::explanation::hst::{HSTConfig, HSTExplanationGenerator};
use oxidowl::explanation::ordering::{
    CompositeExplanationOrderer, ExplanationOrderer, ExplanationProgressMonitor,
    JustificationSizeOrderer, SilentExplanationProgressMonitor,
};
use oxidowl::explanation::renderer::{ConciseExplanationRenderer, ExplanationRenderer};
#[allow(unused_imports)]
use oxidowl::inference::InferredAxiomGenerator;
use oxidowl::inference::metrics::{
    NumberOfAxioms, NumberOfClasses, NumberOfSubClassAxioms, OntologyMetrics, OwlMetric,
};
use oxidowl::inference::{
    InferredClassAssertionAxiomGenerator, InferredDisjointClassesAxiomGenerator,
    InferredEquivalentClassAxiomGenerator, InferredSubClassOfAxiomGenerator,
};
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::shortform::SimpleShortFormProvider;
use oxidowl::ontology::*;
use oxidowl::reasoner_api::structural::StructuralReasonerFactory;
use oxidowl::reasoner_api::{OWLReasonerConfiguration, ReasonerFactory};
use std::sync::Arc;

// ══════════════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════════════

fn make_onto_ref(axioms: Vec<Axiom>) -> OntologyRef {
    let df = DF::new();
    let mut ont = df.build_ontology(axioms);
    df.auto_declare(&mut ont);
    Arc::new(std::sync::RwLock::new(ont))
}

fn structural_factory() -> Arc<dyn ReasonerFactory> {
    Arc::new(StructuralReasonerFactory)
}

fn simple_onto_ref() -> OntologyRef {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let mut ont = df.build_ontology(vec![ax]);
    df.auto_declare(&mut ont);
    Arc::new(std::sync::RwLock::new(ont))
}

// ══════════════════════════════════════════════════════════════════════════════
// Explanation Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_blackbox_explanation_creation() {
    let factory = structural_factory();
    let config = BlackBoxConfig::default();
    let explanation = BlackBoxExplanation::new(factory.clone(), config);

    let onto = simple_onto_ref();

    let explanation_with_onto =
        BlackBoxExplanation::new_with_ontology(onto.clone(), factory, BlackBoxConfig::default());

    let _ = explanation;
    let _ = explanation_with_onto;
}

#[test]
fn test_hst_explanation_generator_creation() {
    let factory = structural_factory();
    let config = HSTConfig::default();
    let hst = HSTExplanationGenerator::new(factory.clone(), config);

    let onto = simple_onto_ref();
    let hst_with_onto =
        HSTExplanationGenerator::new_with_ontology(onto.clone(), factory, HSTConfig::default());

    let _ = hst;
    let _ = hst_with_onto;
}

#[test]
fn test_hst_config_custom() {
    let config = HSTConfig {
        max_depth: 25,
        max_justifications: 30,
    };
    assert_eq!(config.max_depth, 25);
    assert_eq!(config.max_justifications, 30);
}

#[test]
fn test_satisfiability_converter_basic() {
    let _converter = SatisfiabilityConverter;
    let onto = simple_onto_ref();

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let entailment = df.sub_class_of(a, b);

    let (temp_onto, changes) = SatisfiabilityConverter::convert(&onto, &entailment);
    let _ = temp_onto;
    let _ = changes;
}

#[test]
fn test_blackbox_debugger_creation() {
    let factory = structural_factory();
    let onto = simple_onto_ref();
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory.clone(), config);

    let _ = debugger.get_definition_tracker();
    let _ = debugger.get_ontology();
    let _ = debugger.get_reasoner_factory();
}

#[test]
fn test_blackbox_debugger_consistency() {
    let factory = structural_factory();
    let onto = simple_onto_ref();
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);
    let result = debugger.is_consistent();
    assert!(result.is_ok());
    assert!(result.unwrap());
}

#[test]
fn test_blackbox_debugger_minimal_unsatisfiable() {
    let factory = structural_factory();
    let onto = simple_onto_ref();
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);
    let result = debugger.find_minimal_unsatisfiable_set();
    assert!(result.is_ok());
}

#[test]
fn test_blackbox_debugger_unsatisfiable_classes() {
    let factory = structural_factory();
    let onto = simple_onto_ref();
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);
    let result = debugger.get_unsatisfiable_classes();
    assert!(result.is_ok());
}

#[test]
fn test_explanation_ordering() {
    let df = DF::new();
    let ax = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );

    let j1 = Justification {
        entailment: ax.clone(),
        justification: vec![ax.clone()],
        is_minimal: true,
        computation_time: std::time::Duration::default(),
    };
    let j2 = Justification {
        entailment: ax.clone(),
        justification: vec![ax.clone(), ax.clone()],
        is_minimal: true,
        computation_time: std::time::Duration::default(),
    };

    let orderer = JustificationSizeOrderer;
    let ordered = orderer.order(vec![j2.clone(), j1.clone()]);
    assert_eq!(ordered.len(), 2);
    assert!(ordered[0].justification.len() <= ordered[1].justification.len());

    let composite = CompositeExplanationOrderer::new(vec![Box::new(JustificationSizeOrderer)]);
    let ordered2 = composite.order(vec![j2, j1]);
    assert_eq!(ordered2.len(), 2);
}

#[test]
fn test_silent_progress_monitor() {
    let monitor = SilentExplanationProgressMonitor;
    let df = DF::new();
    let ax = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );
    let justification = Justification {
        entailment: ax.clone(),
        justification: vec![ax],
        is_minimal: true,
        computation_time: std::time::Duration::default(),
    };
    monitor.found_explanation(0, &justification);
    monitor.progress_update(1, Some(10));
    assert!(!monitor.is_cancelled());
}

#[test]
fn test_concise_explanation_renderer() {
    let provider: Box<dyn oxidowl::ontology::shortform::ShortFormProvider> =
        Box::new(SimpleShortFormProvider);
    let renderer = ConciseExplanationRenderer::new(provider);

    let df = DF::new();
    let ax = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );
    let justification = Justification {
        entailment: ax.clone(),
        justification: vec![ax],
        is_minimal: true,
        computation_time: std::time::Duration::default(),
    };

    let rendered = renderer.render(&justification);
    assert!(!rendered.is_empty());
    assert!(rendered.contains("Explanation"));
}

#[test]
fn test_blackbox_config_custom() {
    let config = BlackBoxConfig {
        timeout: Some(std::time::Duration::from_secs(30)),
        max_explanations: 25,
    };
    assert_eq!(config.max_explanations, 25);
    assert!(config.timeout.is_some());
}

// ══════════════════════════════════════════════════════════════════════════════
// Inferred Axiom Generator Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_inferred_subclass_generator() {
    let generator = InferredSubClassOfAxiomGenerator;
    assert_eq!(generator.get_label(), "Inferred SubClassOf Axioms");

    let onto = simple_onto_ref();
    let guard = onto.read().unwrap();
    let factory = StructuralReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    let axioms = generator.create_axioms(&guard, reasoner.as_ref());
    let _ = axioms;
}

#[test]
fn test_inferred_equivalent_class_generator() {
    let generator = InferredEquivalentClassAxiomGenerator;
    assert_eq!(generator.get_label(), "Inferred EquivalentClasses Axioms");

    let onto = simple_onto_ref();
    let guard = onto.read().unwrap();
    let factory = StructuralReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    let axioms = generator.create_axioms(&guard, reasoner.as_ref());
    let _ = axioms;
}

#[test]
fn test_inferred_disjoint_classes_generator() {
    let generator = InferredDisjointClassesAxiomGenerator;
    assert_eq!(generator.get_label(), "Inferred DisjointClasses Axioms");

    let onto = simple_onto_ref();
    let guard = onto.read().unwrap();
    let factory = StructuralReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    let axioms = generator.create_axioms(&guard, reasoner.as_ref());
    let _ = axioms;
}

#[test]
fn test_inferred_class_assertion_generator() {
    let generator = InferredClassAssertionAxiomGenerator;
    assert_eq!(generator.get_label(), "Inferred ClassAssertion Axioms");

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let i = df.named("http://ex.org/ind");
    let ax = df.class_assertion(a, i);
    let onto = make_onto_ref(vec![ax]);
    let guard = onto.read().unwrap();
    let factory = StructuralReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    let axioms = generator.create_axioms(&guard, reasoner.as_ref());
    let _ = axioms;
}

// ══════════════════════════════════════════════════════════════════════════════
// Ontology Metrics Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_ontology_metrics_class_count() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let ax1 = df.sub_class_of(a.clone(), b.clone());
    let ax2 = df.sub_class_of(b, c);
    let onto = make_onto_ref(vec![ax1, ax2]);
    let guard = onto.read().unwrap();

    let num_classes = NumberOfClasses;
    let count = num_classes.get_value(&guard);
    assert!(count >= 3.0, "Expected at least 3 classes, got {count}");

    let metrics = OntologyMetrics::compute(&guard);
    assert!(metrics.contains_key("NumberOfClasses"));
}

#[test]
fn test_ontology_metrics_axiom_count() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let onto = make_onto_ref(vec![ax]);
    let guard = onto.read().unwrap();

    let num_axioms = NumberOfAxioms;
    let count = num_axioms.get_value(&guard);
    assert!(count >= 1.0, "Expected at least 1 axiom, got {count}");
}

#[test]
fn test_ontology_metrics_subclass_count() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let ax1 = df.sub_class_of(a, b.clone());
    let ax2 = df.sub_class_of(b, c);
    let onto = make_onto_ref(vec![ax1, ax2]);
    let guard = onto.read().unwrap();

    let num_sub = NumberOfSubClassAxioms;
    let count = num_sub.get_value(&guard);
    assert_eq!(count, 2.0);
}

#[test]
fn test_ontology_metrics_empty() {
    let df = DF::new();
    let ont = df.new_ontology();

    let metrics = OntologyMetrics::compute(&ont);
    assert!(metrics.contains_key("NumberOfClasses"));
    assert_eq!(metrics["NumberOfClasses"], 0.0);
    assert_eq!(metrics["NumberOfAxioms"], 0.0);
}

#[test]
fn test_ontology_metrics_all_metrics() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let onto = make_onto_ref(vec![ax]);
    let guard = onto.read().unwrap();

    let metrics = OntologyMetrics::compute(&guard);
    assert!(metrics.contains_key("NumberOfClasses"));
    assert!(metrics.contains_key("NumberOfSubClassAxioms"));
    assert!(metrics.contains_key("NumberOfAxioms"));
    assert!(metrics.contains_key("NumberOfLogicalAxioms"));
    assert!(metrics.contains_key("NumberOfGCI"));
}

// ══════════════════════════════════════════════════════════════════════════════
// Debugger Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_debugger_config() {
    let config = DebuggerConfig {
        timeout: Some(std::time::Duration::from_secs(60)),
        max_justifications_per_entailment: 20,
    };
    assert_eq!(config.max_justifications_per_entailment, 20);
    assert!(config.timeout.is_some());
}

#[test]
fn test_debugger_default_config() {
    let config = DebuggerConfig::default();
    assert_eq!(config.max_justifications_per_entailment, 10);
    assert!(config.timeout.is_none());
}

#[test]
fn test_debugger_find_justifications() {
    let factory = structural_factory();
    let onto = simple_onto_ref();
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let entailment = df.sub_class_of(a, b);

    let result = debugger.find_justifications(&entailment);
    assert!(result.is_ok());
}

#[test]
fn test_debugger_unsatisfiability_explanation() {
    let factory = structural_factory();
    let onto = simple_onto_ref();
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");

    let result = debugger.get_unsatisfiability_explanation(&a);
    assert!(result.is_ok());
}

#[test]
fn test_debugger_empty_ontology() {
    let factory = structural_factory();
    let df = DF::new();
    let ont = df.new_ontology();
    let onto = Arc::new(std::sync::RwLock::new(ont));
    let config = DebuggerConfig::default();

    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);

    let mus = debugger.find_minimal_unsatisfiable_set().unwrap();
    assert!(mus.is_empty());

    let consistent = debugger.is_consistent().unwrap();
    assert!(consistent);
}

#[test]
fn test_explanation_generator_trait_object() {
    let factory = structural_factory();
    let config = BlackBoxConfig::default();
    let bb = BlackBoxExplanation::new(factory.clone(), config);
    let explanation_gen: Box<dyn ExplanationGenerator> = Box::new(bb);
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let entail = df.sub_class_of(a, b);
    let _ = explanation_gen.get_all_explanations(&entail);
}

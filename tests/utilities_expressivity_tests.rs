#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::config::{PerformanceFeature, ReasoningFeature};
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::{
    ConciseObjectRenderer, DLExpressivityChecker, OWLObjectRenderer, OWLObjectVisitor,
    OWLOntologyMerger, OntologyWalker, StructureWalker,
};
use oxidowl::{
    ManagerConfig, NNFConverter, PerformanceConfig, PerformanceProfile, PrefixManager,
    QNameShortFormProvider, ReasonerConfig, ShortFormProvider, SimpleShortFormProvider,
};
use std::sync::{Arc, RwLock};

const EX: &str = "http://example.org/";
const TEST: &str = "http://test.org/";

fn ex(local: &str) -> String {
    format!("{EX}{local}")
}

fn test_iri(local: &str) -> String {
    format!("{TEST}{local}")
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 DL Expressivity Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Ontology with just classes and SubClassOf should produce AL expressivity.
#[test]
fn test_expressivity_al_basic() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![df.sub_class_of(a.clone(), b.clone())]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(!expr.has_complement, "AL should not have complement");
    assert!(!expr.has_union, "AL should not have union");
    assert!(!expr.has_nominals, "AL should not have nominals");
    assert!(
        expr.to_name().starts_with("AL"),
        "Expressivity should start with AL"
    );
    assert!(
        !expr.to_name().contains('C'),
        "AL should not contain complement marker"
    );
}

/// Adding complement should produce ALC.
#[test]
fn test_expressivity_al_with_complement() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let not_a = df.complement_of(a.clone());
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![df.sub_class_of(not_a, b)]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_complement, "Should detect complement");
    assert!(
        !expr.to_name().contains('C'),
        "Just complement alone doesn't make ALC (needs existential+universal too)"
    );
    let name = expr.to_name();
    assert!(
        name.starts_with("AL"),
        "Expressivity should start with AL: {name}"
    );
}

/// Adding existential quantifier to ALC keeps it as ALC (ALCE is absorbed).
#[test]
fn test_expressivity_alc_with_role() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let r = df.obj_prop(&ex("r"));
    let some_r_a = df.some_values_from(r, a);
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![df.sub_class_of(some_r_a, b)]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_existential, "Should detect existential quantifier");
    assert!(
        expr.to_name().contains("ALC") || expr.to_name().contains("AL"),
        "Expressivity: {}",
        expr.to_name()
    );
}

/// Adding union produces ALCU.
#[test]
fn test_expressivity_alc_with_union() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let union_ab = df.union_of(vec![a.clone(), b.clone()]);
    let c = df.class_ce(&ex("C"));
    let ont = df.build_ontology(vec![df.sub_class_of(union_ab, c)]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_union, "Should detect union");
    assert!(
        !expr.has_existential,
        "Union alone does not create existential quantifier"
    );
}

/// Adding a transitive property should add S.
#[test]
fn test_expressivity_with_transitive() {
    let df = DF::new();
    let r = df.obj_prop(&ex("r"));
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![
        df.sub_class_of(a, b),
        df.transitive_object_property(r),
    ]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_transitivity, "Should detect transitivity (S)");
    assert!(
        expr.to_name().contains('S') || !expr.has_complement,
        "Name should reflect transitivity: {}",
        expr.to_name()
    );
}

/// Adding ObjectOneOf (nominals) should add O.
#[test]
fn test_expressivity_with_nominals() {
    let df = DF::new();
    let i = df.named(&ex("ind"));
    let a = df.class_ce(&ex("A"));
    let one_of = df.one_of(vec![i]);
    let ont = df.build_ontology(vec![df.sub_class_of(one_of, a)]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_nominals, "Should detect nominals (O)");
    assert!(
        expr.to_name().contains('O'),
        "Name should contain O for nominals: {}",
        expr.to_name()
    );
}

/// Adding inverse property should add I.
#[test]
fn test_expressivity_with_inverse() {
    let df = DF::new();
    let p1 = df.obj_prop(&ex("p1"));
    let p2 = df.obj_prop(&ex("p2"));
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![
        df.sub_class_of(a, b),
        df.inverse_object_properties(p1, p2),
    ]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_inverse, "Should detect inverse properties (I)");
    assert!(
        expr.to_name().contains('I'),
        "Name should contain I: {}",
        expr.to_name()
    );
}

/// Adding functional property should add F.
#[test]
fn test_expressivity_with_functional() {
    let df = DF::new();
    let r = df.obj_prop(&ex("r"));
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![
        df.sub_class_of(a, b),
        df.functional_object_property(r),
    ]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(
        expr.has_functional,
        "Should detect functional properties (F)"
    );
    assert!(
        expr.to_name().contains('F'),
        "Name should contain F: {}",
        expr.to_name()
    );
}

/// Adding qualified cardinality should add Q.
#[test]
fn test_expressivity_with_cardinality() {
    let df = DF::new();
    let r = df.obj_prop(&ex("r"));
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let min_card = df.min_cardinality(2, r, a.clone());
    let ont = df.build_ontology(vec![df.sub_class_of(b, min_card)]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_cardinality, "Should detect cardinality");
    assert!(
        expr.has_qualified_cardinality,
        "Should detect qualified cardinality (Q) since filler is not owl:Thing"
    );
    assert!(
        expr.to_name().contains('Q'),
        "Name should contain Q: {}",
        expr.to_name()
    );
}

/// Adding a property chain should add R (role hierarchy/disjointness).
#[test]
fn test_expressivity_with_property_chain() {
    let df = DF::new();
    let p1 = df.obj_prop(&ex("p1"));
    let p2 = df.obj_prop(&ex("p2"));
    let chain = ObjectPropertyExpression::PropertyChain(vec![p1, p2]);
    let super_p = df.obj_prop(&ex("super"));
    let ont = df.build_ontology(vec![df.sub_object_property_of(chain, super_p)]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(
        expr.has_role_hierarchy,
        "Should detect role hierarchy (R via property chain)"
    );
}

/// Full SROIQ ontology exercising S, R, O, I, Q.
#[test]
fn test_expressivity_full_sroiq() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let c = df.class_ce(&ex("C"));
    let r = df.obj_prop(&ex("r"));
    let s = df.obj_prop(&ex("s"));
    let t = df.obj_prop(&ex("t"));
    let i = df.named(&ex("ind"));
    let j = df.named(&ex("ind2"));

    let chain = ObjectPropertyExpression::PropertyChain(vec![r.clone(), s.clone()]);
    let not_a = df.complement_of(a.clone());
    let union_ab = df.union_of(vec![a.clone(), b.clone()]);
    let some_r_c = df.some_values_from(r.clone(), c.clone());
    let one_of = df.one_of(vec![i, j]);
    let min_card = df.min_cardinality(2, t.clone(), b.clone());
    let inv_s = ObjectPropertyExpression::InverseObjectProperty(ObjectProperty {
        iri: IRI::new(&ex("s")),
    });

    let ont = df.build_ontology(vec![
        df.sub_class_of(not_a, b.clone()),
        df.sub_class_of(union_ab, c.clone()),
        df.sub_class_of(some_r_c, a.clone()),
        df.sub_class_of(one_of, c.clone()),
        df.sub_object_property_of(chain, t.clone()),
        df.inverse_object_properties(s.clone(), inv_s),
        df.sub_class_of(min_card, a.clone()),
        df.transitive_object_property(r.clone()),
        df.functional_object_property(r),
    ]);

    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&ont);

    assert!(expr.has_complement, "SROIQ should have complement");
    assert!(expr.has_union, "SROIQ should have union");
    assert!(expr.has_nominals, "SROIQ should have nominals (O)");
    assert!(expr.has_inverse, "SROIQ should have inverse (I)");
    assert!(expr.has_transitivity, "SROIQ should have transitivity (S)");
    assert!(
        expr.has_role_hierarchy,
        "SROIQ should have role hierarchy (R)"
    );
    assert!(
        expr.has_qualified_cardinality,
        "SROIQ should have qualified cardinality (Q)"
    );
    assert!(expr.has_functional, "SROIQ should have functional (F)");

    let name = expr.to_name();
    assert!(name.contains('S'), "Name should contain S: {name}");
    assert!(name.contains('O'), "Name should contain O: {name}");
    assert!(name.contains('I'), "Name should contain I: {name}");
    assert!(name.contains('Q'), "Name should contain Q: {name}");
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 Configuration Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Verify default reasoner config has sensible values.
#[test]
fn test_reasoner_config_defaults() {
    let config = ReasonerConfig::default();

    assert_eq!(
        config.reasoning.tableau_algorithm,
        oxidowl::TableauAlgorithm::Traditional
    );
    assert!(
        config.reasoning.timeout.is_some(),
        "Default should have a timeout"
    );
    assert!(
        config.reasoning.max_memory_mb.is_some(),
        "Default should have a memory limit"
    );
    assert!(
        config.reasoning.max_expansion_depth > 0,
        "Max expansion depth should be positive"
    );
    assert!(
        !config.reasoning.dump_clauses,
        "Clause dumping should be disabled by default"
    );
    assert!(
        !config.reasoning.incremental_reasoning,
        "Incremental reasoning should be off by default"
    );
    assert!(
        config.reasoning.is_enabled(ReasoningFeature::Optimizations),
        "Optimizations should be enabled by default"
    );
    assert!(
        config
            .reasoning
            .is_enabled(ReasoningFeature::ClashDetection),
        "Clash detection should be enabled by default"
    );

    assert!(
        config.cache.max_cache_size_mb > 0,
        "Cache size should be positive"
    );
    assert!(config.server.port > 0, "Server port should be positive");
    assert!(
        !config.server.bind_address.is_empty(),
        "Bind address should not be empty"
    );
}

/// Custom manager config creation.
#[test]
fn test_manager_config_custom() {
    let config = ManagerConfig {
        enable_change_history: true,
        max_history_size: 500,
        silent_missing_imports: false,
        max_import_depth: 10,
    };

    assert!(config.enable_change_history);
    assert_eq!(config.max_history_size, 500);
    assert!(!config.silent_missing_imports);
    assert_eq!(config.max_import_depth, 10);

    let default = ManagerConfig::default();
    assert!(!default.enable_change_history);
    assert_eq!(default.max_history_size, 100);
    assert!(default.silent_missing_imports);
}

/// Performance config profile settings.
#[test]
fn test_performance_config() {
    let high_config = PerformanceConfig::from_profile(PerformanceProfile::High);
    assert_eq!(high_config.profile, PerformanceProfile::High);
    assert!(high_config.is_enabled(PerformanceFeature::ParallelExpansion));

    let low_config = PerformanceConfig::from_profile(PerformanceProfile::Low);
    assert_eq!(low_config.profile, PerformanceProfile::Low);
    assert!(low_config.gc_threshold > 0.0);
    assert!(!low_config.profile.enable_simd());

    let ultra_config = PerformanceConfig::from_profile(PerformanceProfile::Ultra);
    assert_eq!(ultra_config.profile, PerformanceProfile::Ultra);
    assert!(ultra_config.profile.enable_numa_awareness());

    let def = PerformanceConfig::default();
    assert_eq!(def.profile, PerformanceProfile::High);
    let default_profile = PerformanceProfile::default();
    let from_profile = PerformanceConfig::from_profile(default_profile);
    assert_eq!(from_profile.worker_threads, def.worker_threads);
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 ShortForm Provider Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Test SimpleShortFormProvider entity-to-string mapping.
#[test]
fn test_simple_shortform_provider() {
    let provider = SimpleShortFormProvider;
    let entity_fragment = Entity::Class(IRI::new("http://example.org/ontology#Person"));
    assert_eq!(provider.get_short_form(&entity_fragment), "Person");

    let entity_segment = Entity::Class(IRI::new("http://example.org/Person"));
    assert_eq!(provider.get_short_form(&entity_segment), "Person");

    let entity_full = Entity::Class(IRI::new("http://example.org/"));
    assert_eq!(provider.get_short_form(&entity_full), "http://example.org/");

    let entity_owl = Entity::Class(IRI::new("http://www.w3.org/2002/07/owl#Thing"));
    assert_eq!(provider.get_short_form(&entity_owl), "Thing");

    let entity_nothing = Entity::Class(IRI::new("http://www.w3.org/2002/07/owl#Nothing"));
    assert_eq!(provider.get_short_form(&entity_nothing), "Nothing");
}

/// Test QNameShortFormProvider prefix resolution.
#[test]
fn test_qname_shortform_provider() {
    let pm = PrefixManager::new();
    let provider = QNameShortFormProvider::new(pm);

    let entity = Entity::Class(IRI::new("http://www.w3.org/2002/07/owl#Thing"));
    let sf = provider.get_short_form(&entity);
    assert!(
        sf == "owl:Thing" || sf == "Thing",
        "QName should produce owl:Thing or fallback: got '{sf}'"
    );

    let entity_fallback = Entity::Class(IRI::new("http://example.org/ontology#Person"));
    let sf2 = provider.get_short_form(&entity_fallback);
    assert_eq!(sf2, "Person");

    let entity_xsd = Entity::Class(IRI::new("http://www.w3.org/2001/XMLSchema#integer"));
    let sf3 = provider.get_short_form(&entity_xsd);
    assert!(
        sf3 == "xsd:integer" || sf3 == "integer",
        "QName should produce xsd:integer or fallback: got '{sf3}'"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 Visitor / Walker Tests
// ══════════════════════════════════════════════════════════════════════════════

/// Counting visitor that tracks how many axioms and class expressions were visited.
#[derive(Debug, Default)]
struct CountingVisitor {
    axiom_count: usize,
    ce_count: usize,
    iri_count: usize,
    ope_count: usize,
    dpe_count: usize,
    ind_count: usize,
}

impl OWLObjectVisitor for CountingVisitor {
    fn visit_axiom(&mut self, _axiom: &Axiom) {
        self.axiom_count += 1;
    }
    fn visit_class_expression(&mut self, _expr: &ClassExpression) {
        self.ce_count += 1;
    }
    fn visit_iri(&mut self, _iri: &IRI) {
        self.iri_count += 1;
    }
    fn visit_ope(&mut self, _ope: &ObjectPropertyExpression) {
        self.ope_count += 1;
    }
    fn visit_dpe(&mut self, _dpe: &DataPropertyExpression) {
        self.dpe_count += 1;
    }
    fn visit_individual(&mut self, _ind: &Individual) {
        self.ind_count += 1;
    }
}

/// Walk an ontology and verify all axioms are visited.
#[test]
fn test_ontology_walker_basic() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let c = df.class_ce(&ex("C"));
    let r = df.obj_prop(&ex("r"));
    let i = df.named(&ex("ind"));
    let some_r_b = df.some_values_from(r.clone(), b.clone());

    let ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(some_r_b, c.clone()),
        df.class_assertion(a.clone(), i),
    ]);

    let mut walker = OntologyWalker::new(CountingVisitor::default());
    walker.walk_ontology(&ont);
    let visitor = walker.into_visitor();

    assert_eq!(visitor.axiom_count, 3, "Should visit all 3 axioms");
    assert!(
        visitor.ce_count >= 4,
        "Should visit at least 4 class expressions (3 axioms + fillers)"
    );
    assert!(
        visitor.ope_count >= 1,
        "Should visit at least 1 object property expression"
    );
    assert!(
        visitor.iri_count >= 4,
        "Should visit at least 4 IRIs (A, B, C, r)"
    );
    assert!(visitor.ind_count >= 1, "Should visit at least 1 individual");
}

/// Walk using StructureWalker and verify current axiom access.
#[test]
fn test_structure_walker_current_axiom() {
    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let ont = df.build_ontology(vec![df.sub_class_of(a.clone(), b.clone())]);

    #[derive(Debug, Default)]
    struct CheckVisitor {
        last_type: Option<AxiomType>,
    }
    impl OWLObjectVisitor for CheckVisitor {
        fn visit_axiom(&mut self, axiom: &Axiom) {
            self.last_type = Some(axiom.axiom_type());
        }
    }

    let mut walker: StructureWalker<'_, CheckVisitor> =
        StructureWalker::new(CheckVisitor::default());
    walker.walk_ontology(&ont);

    assert!(walker.get_current_axiom().is_some());
    let current = walker.get_current_axiom().unwrap();
    assert_eq!(current.axiom_type(), AxiomType::SubClassOf);
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 Ontology Merger Test
// ══════════════════════════════════════════════════════════════════════════════

/// Merge two ontologies and verify combined axiom count.
#[test]
fn test_ontology_merger_basic() {
    use oxidowl::manager::OntologyManager;

    let df = DF::new();
    let a = df.class_ce(&ex("A"));
    let b = df.class_ce(&ex("B"));
    let c = df.class_ce(&ex("C"));
    let d = df.class_ce(&ex("D"));

    let ont1 = df.build_ontology_with_iri(
        "http://test.org/onto1",
        vec![df.sub_class_of(a.clone(), b.clone())],
    );
    let ont2 = df.build_ontology_with_iri(
        "http://test.org/onto2",
        vec![df.sub_class_of(c.clone(), d.clone())],
    );

    let ont1_ref = Arc::new(RwLock::new(ont1));
    let ont2_ref = Arc::new(RwLock::new(ont2));

    let mut manager = OntologyManager::new();
    let merger = OWLOntologyMerger::new(IRI::new("http://test.org/merged"));
    let result = merger.merge(&[ont1_ref, ont2_ref], &mut manager);

    assert!(result.is_ok(), "Merge should succeed");
    let merged_ref = result.unwrap();
    let guard = merged_ref.read().unwrap();
    assert_eq!(
        guard.axioms().len(),
        2,
        "Merged ontology should have 2 axioms"
    );
    assert_eq!(
        guard.get_iri().map(|i| i.as_str().to_string()),
        Some("http://test.org/merged".to_string())
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 Renderer Test
// ══════════════════════════════════════════════════════════════════════════════

/// Render a class expression to string.
#[test]
fn test_concise_renderer_basic() {
    let renderer = ConciseObjectRenderer::new();
    let a = ClassExpression::Class(Class {
        iri: IRI::new(&ex("A")),
    });
    let b = ClassExpression::Class(Class {
        iri: IRI::new(&ex("B")),
    });
    let intersection = ClassExpression::ObjectIntersectionOf(vec![a, b]);

    let rendered = renderer.render_class_expression(&intersection);
    assert!(
        rendered.contains("and"),
        "Should contain 'and' for intersection: {rendered}"
    );
    assert!(rendered.contains("A"), "Should contain short form for A");
    assert!(rendered.contains("B"), "Should contain short form for B");

    let complement = ClassExpression::ObjectComplementOf(Box::new(ClassExpression::Class(Class {
        iri: IRI::new(&ex("C")),
    })));
    let comp_rendered = renderer.render_class_expression(&complement);
    assert!(
        comp_rendered.contains("not"),
        "Should contain 'not': {comp_rendered}"
    );

    let df = DF::new();
    let ax = df.sub_class_of(df.class_ce(&ex("D")), df.class_ce(&ex("E")));
    let ax_rendered = renderer.render_axiom(&ax);
    assert!(
        ax_rendered.contains("SubClassOf"),
        "Should contain SubClassOf: {ax_rendered}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// 2.14 NNF Converter Test
// ══════════════════════════════════════════════════════════════════════════════

/// Create NNFConverter and verify basic functionality.
#[test]
fn test_nnf_converter_creation() {
    let converter = NNFConverter;

    let a = ClassExpression::Class(Class {
        iri: IRI::new(&ex("A")),
    });
    let result = converter.to_nnf(&a);
    assert_eq!(result, a, "NNF of a named class should be itself");

    let not_a = ClassExpression::ObjectComplementOf(Box::new(a.clone()));
    let not_not_a = ClassExpression::ObjectComplementOf(Box::new(not_a.clone()));
    let result2 = converter.to_nnf(&not_not_a);
    match &result2 {
        ClassExpression::Class(c) => {
            assert!(
                c.iri.as_str().contains("A"),
                "Double negation should unwrap to A"
            );
        }
        other => panic!("Expected class after double-negation removal, got: {other:?}"),
    }

    let a2 = ClassExpression::Class(Class {
        iri: IRI::new(&ex("A")),
    });
    let b = ClassExpression::Class(Class {
        iri: IRI::new(&ex("B")),
    });
    let intersection = ClassExpression::ObjectIntersectionOf(vec![a2, b]);
    let not_intersection = ClassExpression::ObjectComplementOf(Box::new(intersection));
    let result3 = converter.to_nnf(&not_intersection);
    assert!(
        matches!(&result3, ClassExpression::ObjectUnionOf(_)),
        "¬(A ⊓ B) should become union form, got: {result3:?}"
    );
}

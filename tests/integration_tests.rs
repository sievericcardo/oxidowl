//! Comprehensive Integration Tests for Oxidowl Porting Features.
//!
//! Covers all 7 phases of the OWL API → Oxidowl porting effort.
//! Tests exercise the full stack: ontology model → manager → parsing →
//! reasoning → explanation → modularity → serialization.

use oxidowl::factory::providers::AxiomCreationProvider;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::*;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn simple_ontology() -> Ontology {
    let mut o = Ontology::new();
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let b = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/B"),
    });
    let c = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/C"),
    });
    o.set_iri(IRI::new("http://ex.org/TestOnt"));
    o.add_axiom(Axiom::Declaration(DeclarationAxiom {
        id: 1,
        entity: Entity::Class(IRI::new("http://ex.org/A")),
    }));
    o.add_axiom(Axiom::Declaration(DeclarationAxiom {
        id: 2,
        entity: Entity::Class(IRI::new("http://ex.org/B")),
    }));
    o.add_axiom(Axiom::Declaration(DeclarationAxiom {
        id: 3,
        entity: Entity::Class(IRI::new("http://ex.org/C")),
    }));
    o.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 4,
        subclass: a.clone(),
        superclass: b.clone(),
        annotations: vec![],
    }));
    o.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 5,
        subclass: b.clone(),
        superclass: c.clone(),
        annotations: vec![],
    }));
    o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 6,
        class: a.clone(),
        individual: Individual::Named(NamedIndividual {
            iri: IRI::new("http://ex.org/ind"),
        }),
        annotations: vec![],
    }));
    o
}

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(RwLock::new(o))
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 1 — Core Model API (Manager, DataFactory, Changes, IRI Mapper)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase1_manager_create_and_register() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont1");
    let _ont = manager.create_ontology(iri.clone());
    assert!(manager.contains_ontology(&iri));
    assert_eq!(manager.ontology_count(), 1);

    let exported = manager.get_ontology(&iri).unwrap();
    assert_eq!(
        exported.read().unwrap().get_iri().cloned(),
        Some(iri.clone())
    );
}

#[test]
fn phase1_factory_entity_deduplication() {
    let factory = DataFactory::new();
    let iri = IRI::new("http://ex.org/A");
    let c1 = factory.get_class(&iri);
    let c2 = factory.get_class(&iri);
    assert_eq!(c1.iri, c2.iri);

    let p1 = factory.get_object_property(&iri);
    let i1 = factory.get_named_individual(&iri);
    assert_eq!(p1.iri, iri);
    assert_eq!(i1.iri, iri);
}

#[test]
fn phase1_change_history_undo_redo() {
    let mut history = ChangeHistory::new(5);
    let factory = DataFactory::new();
    let ax = factory.make_sub_class_of_axiom(
        ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        }),
        ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        }),
        vec![],
    );
    let change = OntologyChange::AddAxiom {
        ontology_iri: IRI::new("http://ex.org/ont"),
        axiom: Axiom::SubClassOf(ax),
    };
    history.record(vec![change]);
    assert!(history.can_undo());
    let undone = history.undo(1);
    assert!(!undone.is_empty());
    assert!(history.can_redo());
}

#[test]
fn phase1_iri_mapper_resolution() {
    use crate::manager::iri_mapper::OntologyIRIMapper;
    use crate::manager::iri_mapper::SimpleIRIMapper;
    let mapper = SimpleIRIMapper::new(
        IRI::new("http://ex.org/ont"),
        IRI::new("file:///tmp/ont.owl"),
    );
    let result = mapper.get_document_iri(&IRI::new("http://ex.org/ont"));
    assert!(result.is_some());
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 2 — Parsers & Serializers (Manchester, LaTeX, DL, KRSS)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase2_manchester_renderer_roundtrip() {
    use crate::parsers::manchester_renderer::ManchesterRenderer;
    let o = simple_ontology();
    let renderer = ManchesterRenderer::new();
    let output = renderer.serialize(&o).unwrap();
    assert!(output.contains("SubClassOf:"));
    assert!(output.contains("http://ex.org/A"));
}

#[test]
fn phase2_latex_render() {
    use crate::parsers::latex::LatexRenderer;
    let o = simple_ontology();
    let config = parsers::latex::LatexConfig::default();
    let renderer = LatexRenderer::new();
    let doc = renderer.render_document(&o, &config).unwrap();
    assert!(doc.contains("\\documentclass"));
    assert!(doc.contains("\\sqsubseteq"));
    assert!(doc.contains("\\end{document}"));
}

#[test]
fn phase2_dl_syntax_render_and_parse() {
    use crate::parsers::dl_syntax::*;
    let o = simple_ontology();
    let renderer = DLSyntaxRenderer::new(true);
    let _output = renderer.serialize(&o).unwrap();

    let mut parser = DLSyntaxParser::new();
    let result = parser.parse("A \u{2291} B");
    assert!(result.is_ok());
}

#[test]
fn phase2_krss_render_and_parse() {
    use crate::parsers::krss::*;
    let o = simple_ontology();
    let renderer = KRSSRenderer::new(KRSSVariant::KRSS);
    let output = renderer.serialize(&o).unwrap();
    assert!(output.contains("implies"));

    let mut parser = KRSSParser::new(KRSSVariant::KRSS);
    let result = parser.parse("(implies A B)");
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 3 — Reasoner API (OWLReasoner, StructuralReasoner)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase3_structural_sub_and_super_classes() {
    let o = simple_ontology();
    let reasoner = StructuralReasoner::new(onto_ref(o));
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let b = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/B"),
    });

    let sups = reasoner.get_super_classes(&a, false).unwrap();
    assert!(!sups.is_empty());
    let flat = sups.get_flattened();
    assert!(flat.contains(&b));

    let subs = reasoner.get_sub_classes(&b, false).unwrap();
    assert!(!subs.is_empty());
    assert!(subs.get_flattened().contains(&a));
}

#[test]
fn phase3_structural_instances_and_types() {
    let o = simple_ontology();
    let reasoner = StructuralReasoner::new(onto_ref(o));
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let ind = Individual::Named(NamedIndividual {
        iri: IRI::new("http://ex.org/ind"),
    });

    let instances = reasoner.get_instances(&a, false).unwrap();
    assert!(instances.contains_entity(&ind));

    let types = reasoner.get_types(&ind, false).unwrap();
    assert!(types.contains_entity(&a));
}

#[test]
fn phase3_tableau_reasoner_factory() {
    let o = simple_ontology();
    let onto = onto_ref(o);
    let factory = TableauReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    assert!(reasoner.is_consistent().unwrap());
}

#[test]
fn phase3_node_and_nodeset() {
    let n = Node::singleton("X");
    assert!(n.is_singleton());
    assert_eq!(n.get_size(), 1);

    let mut set = HashSet::new();
    set.insert(Node::singleton("A"));
    set.insert(Node::singleton("B"));
    let ns = NodeSet::new(set);
    assert!(!ns.is_empty());
    assert_eq!(ns.get_flattened().len(), 2);
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 4 — Tools & Utilities (EntitySearcher, Transformer, NNF, Expressivity)
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase4_entity_searcher() {
    let o = simple_ontology();
    let index = EntityIndex::from_ontology(&o);
    let searcher = EntitySearcher::new(&o, &index);

    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let axioms = searcher.get_sub_class_axioms_for_lhs(&a);
    assert!(!axioms.is_empty());
}

#[test]
fn phase4_entity_renamer() {
    let mut renamer = OWLEntityRenamer::new();
    renamer.add_rename(
        IRI::new("http://ex.org/A"),
        IRI::new("http://ex.org/X"),
        EntityType::Class,
    );
    let o = simple_ontology();
    let onto = onto_ref(o);
    let changes = renamer.rename_ontology(&onto);
    assert!(changes.is_ok());
}

#[test]
fn phase4_entity_remover() {
    let mut remover = OWLEntityRemover::new();
    remover.add_entity(IRI::new("http://ex.org/A"), EntityType::Class);
    let o = simple_ontology();
    let onto = onto_ref(o);
    let changes = remover.remove_entities(&onto);
    assert!(changes.is_ok());
}

#[test]
fn phase4_nnf_converter() {
    use crate::transform::nnf::NNFConverter;
    let c1 = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/C1"),
    });
    let c2 = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/C2"),
    });
    let original =
        ClassExpression::ObjectComplementOf(Box::new(ClassExpression::ObjectIntersectionOf(vec![
            c1.clone(),
            c2.clone(),
        ])));
    let nnf = NNFConverter.to_nnf(&original);
    // ¬(C1 ⊓ C2) → ¬C1 ⊔ ¬C2
    assert!(matches!(nnf, ClassExpression::ObjectUnionOf(_)));
}

#[test]
fn phase4_dl_expressivity() {
    use crate::transform::expressivity::DLExpressivityChecker;
    let o = simple_ontology();
    let checker = DLExpressivityChecker;
    let expr = checker.analyze(&o);
    let name = expr.to_name();
    assert!(!name.is_empty());
}

#[test]
fn phase4_ontology_walker() {
    use crate::walk::{OWLObjectVisitor, OntologyWalker};
    struct Counter {
        count: usize,
    }
    impl OWLObjectVisitor for Counter {
        fn visit_class_expression(&mut self, _: &ClassExpression) {
            self.count += 1;
        }
    }
    let o = simple_ontology();
    let mut walker = OntologyWalker::new(Counter { count: 0 });
    walker.walk_ontology(&o);
    assert!(walker.into_visitor().count > 0);
}

#[test]
fn phase4_ontology_metrics() {
    let o = simple_ontology();
    let metrics = OntologyMetrics::compute(&o);
    assert!(metrics.contains_key("NumberOfAxioms"));
    assert!(metrics.contains_key("NumberOfClasses"));
}

#[test]
fn phase4_ontology_merger() {
    let mut manager = OntologyManager::new();
    let o1 = simple_ontology();
    let o2 = simple_ontology();
    let r1 = onto_ref(o1);
    let r2 = onto_ref(o2);
    let merger = crate::walk::merge::OWLOntologyMerger::new(IRI::new("http://ex.org/Merged"));
    let merged = merger.merge(&[r1, r2], &mut manager);
    assert!(merged.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 5 — Explanation & Modularity
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase5_blackbox_explanation() {
    use crate::explanation::blackbox::BlackBoxConfig;
    let factory = Arc::new(TableauReasonerFactory);
    let config = BlackBoxConfig::default();
    let _bb = BlackBoxExplanation::new(factory, config);
    // Exercise the struct creation; full justifications require a reasoner
}

#[test]
fn phase5_hst_generator() {
    use crate::explanation::hst::{HSTConfig, HSTExplanationGenerator};
    let factory = Arc::new(TableauReasonerFactory);
    let config = HSTConfig::default();
    let _hst = HSTExplanationGenerator::new(factory, config);
}

#[test]
fn phase5_satisfiability_converter() {
    use crate::explanation::converter::SatisfiabilityConverter;
    let o = simple_ontology();
    let onto = onto_ref(o);
    let ax = Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        }),
        superclass: ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        }),
        annotations: vec![],
    });
    let (_temp, _cleanup) = SatisfiabilityConverter::convert(&onto, &ax);
}

#[test]
fn phase5_debugger() {
    let o = simple_ontology();
    let onto = onto_ref(o);
    let factory = Arc::new(TableauReasonerFactory);
    let config = DebuggerConfig::default();
    let debugger = BlackBoxOWLDebugger::new(onto.clone(), factory, config);
    let _consistent = debugger.is_consistent();
}

#[test]
fn phase5_atomic_decomposition() {
    use crate::modularity::decomposition::compute_atomic_decomposition;
    let o = simple_ontology();
    let decomp = compute_atomic_decomposition(&o);
    assert!(decomp.atom_count() > 0);
    assert!(decomp.axiom_count() > 0);
}

#[test]
fn phase5_module_extractor() {
    use crate::modularity::extractor::{ModuleExtractor, ModuleExtractorConfig};
    use crate::modularity::locality::LocalityClass;
    let o = simple_ontology();
    let config = ModuleExtractorConfig::default();
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Star, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&o, &sig);
    assert!(!module.axioms().is_empty());
}

#[test]
fn phase5_syntactic_locality() {
    use crate::modularity::locality::{
        LocalityClass, LocalityEvaluator, SyntacticLocalityEvaluator,
    };
    let evaluator = SyntacticLocalityEvaluator::new(LocalityClass::Star);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/B"));
    let ax = Axiom::SubClassOf(SubClassOfAxiom {
        id: 0,
        subclass: ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/A"),
        }),
        superclass: ClassExpression::Class(Class {
            iri: IRI::new("http://ex.org/B"),
        }),
        annotations: vec![],
    });
    let is_local = evaluator.is_local(&ax, &sig);
    // B is in signature, so the axiom is NOT local (provides info about B)
    assert!(!is_local);
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 6 — OBO & RIO Formats
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase6_obo_parse_and_serialize() {
    use parsers::obo::*;
    let obo_content = "[Term]\nid: TEST:0001\nname: test term\nis_a: TEST:0000\n";
    let ontology = parse(obo_content);
    assert!(ontology.is_ok());

    let o = simple_ontology();
    let writer = OBOWriter::default();
    let output = writer.serialize(&o);
    assert!(!output.is_empty());
}

#[test]
fn phase6_obo_converter() {
    use parsers::obo::converter::{Obo2Owl, Owl2Obo};
    let converter = Obo2Owl::new();
    assert!(converter.convert_stanzas(&[]).is_ok());

    let back = Owl2Obo::new();
    let o = simple_ontology();
    let result = back.serialize(&o);
    assert!(!result.is_empty());
}

#[test]
fn phase6_rio_jsonld() {
    use parsers::rio::jsonld::*;
    let o = simple_ontology();
    let renderer = JsonLdRenderer::new();
    let output = renderer.serialize(&o);
    assert!(output.is_ok());
    assert!(output.unwrap().contains("@graph"));
}

#[test]
fn phase6_rio_trig() {
    use parsers::rio::trig::*;
    let o = simple_ontology();
    let renderer = TriGRenderer::new();
    let output = renderer.serialize(&o);
    assert!(output.is_ok());
}

#[test]
fn phase6_rio_n3_and_nquads() {
    use parsers::rio::n3::*;
    use parsers::rio::nquads::*;
    let o = simple_ontology();

    let n3r = N3Renderer::new();
    assert!(n3r.serialize(&o).is_ok());

    let nqr = NQuadsRenderer::new();
    assert!(nqr.serialize(&o).is_ok());
}

#[test]
fn phase6_rio_trix_and_rdfa() {
    use parsers::rio::rdfa::*;
    use parsers::rio::trix::*;
    let o = simple_ontology();

    let tr = TriXRenderer::new();
    assert!(tr.serialize(&o).is_ok());

    let parser = RDFaParser::new();
    let html = r#"<div typeof="http://ex.org/A" about="http://ex.org/ind"></div>"#;
    assert!(parser.parse(html).is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// PHASE 7 — Vocabularies & Datatypes
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn phase7_datatype_categories() {
    use crate::ontology::datatypes::OWL2Datatype;
    let dt = OWL2Datatype::Integer;
    assert!(dt.is_numeric());
    assert_eq!(dt.short_name(), "integer");
    assert_eq!(dt.category(), DatatypeCategory::Numeric);
    assert!(dt.is_built_in());
}

#[test]
fn phase7_datatype_facets() {
    use crate::ontology::datatypes::OWL2Datatype;
    let dt = OWL2Datatype::String;
    let facets = dt.facets();
    assert!(!facets.is_empty());
    assert!(facets.iter().any(|f| f.short_name() == "length"));
}

#[test]
fn phase7_datatype_subtype() {
    use crate::ontology::datatypes::OWL2Datatype;
    assert!(OWL2Datatype::Int.is_subtype_of(&OWL2Datatype::Integer));
    assert!(OWL2Datatype::Integer.is_subtype_of(&OWL2Datatype::Decimal));
    assert!(!OWL2Datatype::String.is_subtype_of(&OWL2Datatype::Integer));
}

#[test]
fn phase7_datatype_validation() {
    use crate::ontology::datatypes::OWL2Datatype;
    assert!(OWL2Datatype::Integer.validate_lexical_form("42").is_ok());
    assert!(
        OWL2Datatype::Integer
            .validate_lexical_form("not-a-number")
            .is_err()
    );
    assert!(OWL2Datatype::Boolean.validate_lexical_form("true").is_ok());
}

#[test]
fn phase7_datatype_from_iri() {
    use crate::ontology::datatypes::OWL2Datatype;
    let iri = IRI::new("http://www.w3.org/2001/XMLSchema#string");
    let dt = OWL2Datatype::from_iri(&iri);
    assert!(dt.is_some());
    assert_eq!(dt.unwrap(), OWL2Datatype::String);
}

#[test]
fn phase7_namespaces_all() {
    let all = Namespaces::all();
    assert!(all.len() >= 25);
}

#[test]
fn phase7_prefix_manager() {
    let mut pm = PrefixManager::new();
    pm.add_prefix("ex", "http://example.org/");
    let expanded = pm.expand("ex:Test");
    assert_eq!(expanded, Some("http://example.org/Test".to_string()));
    let shortened = pm.shorten("http://example.org/Test");
    assert_eq!(shortened, Some("ex:Test".to_string()));
}

#[test]
fn phase7_prefix_manager_well_known() {
    let pm = PrefixManager::new();
    assert_eq!(
        pm.expand("owl:Thing"),
        Some("http://www.w3.org/2002/07/owl#Thing".to_string())
    );
    assert_eq!(
        pm.expand("rdf:type"),
        Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string())
    );
}

#[test]
fn phase7_vocabulary_constants() {
    use crate::ontology::vocabulary::owl;
    assert_eq!(owl::THING, "http://www.w3.org/2002/07/owl#Thing");
    assert_eq!(
        owl::SUB_CLASS_OF,
        "http://www.w3.org/2002/07/owl#subClassOf"
    );
    assert_eq!(owl::IMPORTS, "http://www.w3.org/2002/07/owl#imports");
    assert_eq!(owl::DEPRECATED, "http://www.w3.org/2002/07/owl#deprecated");
}

#[test]
fn phase7_skos_prov_constants() {
    use crate::ontology::vocabulary::{dc, skos};
    assert_eq!(
        skos::PREF_LABEL,
        "http://www.w3.org/2004/02/skos/core#prefLabel"
    );
    assert_eq!(dc::TITLE, "http://purl.org/dc/elements/1.1/title");
}

// ══════════════════════════════════════════════════════════════════════════════
// Cross-Phase Integration Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn cross_phase_load_and_reason() {
    // Phase 1: Manager
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/cross");
    let onto = manager.create_ontology(iri.clone());

    // Phase 1: Add axioms via changes
    let (a, b, ind, ax1, ax2) = {
        let factory = manager.get_data_factory();
        let a = ClassExpression::Class(factory.get_class(&IRI::new("http://ex.org/A")));
        let b = ClassExpression::Class(factory.get_class(&IRI::new("http://ex.org/B")));
        let ind = Individual::Named(NamedIndividual {
            iri: IRI::new("http://ex.org/i"),
        });
        let ax1 = Axiom::SubClassOf(factory.make_sub_class_of_axiom(a.clone(), b.clone(), vec![]));
        let ax2 = Axiom::ClassAssertion(factory.make_class_assertion_axiom(
            a.clone(),
            ind.clone(),
            vec![],
        ));
        (a, b, ind, ax1, ax2)
    };
    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax1,
    });
    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax2,
    });

    // Phase 3: Create reasoner (clone onto ref for later reuse)
    let onto_clone = onto.clone();
    let reasoner = StructuralReasoner::new(onto);

    // Verify hierarchy
    let subs = reasoner.get_sub_classes(&b, false).unwrap();
    assert!(subs.get_flattened().contains(&a));

    let types = reasoner.get_types(&ind, false).unwrap();
    assert!(types.contains_entity(&a));

    // Phase 4: Entity search on reasoned results
    let guard = onto_clone.read().unwrap();
    let index = EntityIndex::from_ontology(&guard);
    let searcher = EntitySearcher::new(&guard, &index);
    let axioms = searcher.get_class_assertion_axioms(&ind);
    assert!(!axioms.is_empty());
    drop(guard);

    // Phase 7: Expressivity — a simple A ⊑ B ontology has no complex
    // constructors, so the checker must report none of them as present.
    let guard = onto_clone.read().unwrap();
    let expr = DLExpressivityChecker.analyze(&guard);
    assert!(!expr.has_existential);
    assert!(!expr.has_union);
    assert!(!expr.has_complement);
    assert!(!expr.has_cardinality);
    assert!(!expr.has_nominals);
    drop(guard);
}

#[test]
fn cross_phase_serialize_pipeline() {
    let o = simple_ontology();
    let _onto = onto_ref(o.clone());

    // Phase 2: Serialize in multiple formats
    let _manchester = parsers::manchester_renderer::ManchesterRenderer::new()
        .serialize(&o)
        .unwrap();
    let _latex = parsers::latex::LatexRenderer::new()
        .render_document(&o, &parsers::latex::LatexConfig::default())
        .unwrap();
    let _dl = parsers::dl_syntax::DLSyntaxRenderer::new(true)
        .serialize(&o)
        .unwrap();

    // Phase 6: Serialize in OBO and RIO formats
    let _obo = parsers::obo::OBOWriter::default().serialize(&o);
    assert!(
        parsers::rio::trig::TriGRenderer::new()
            .serialize(&o)
            .is_ok()
    );

    // Phase 4: NNF transform
    let a = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/A"),
    });
    let b = ClassExpression::Class(Class {
        iri: IRI::new("http://ex.org/B"),
    });
    let neg =
        ClassExpression::ObjectComplementOf(Box::new(ClassExpression::ObjectIntersectionOf(vec![
            a, b,
        ])));
    let nnf = NNFConverter.to_nnf(&neg);
    assert!(matches!(nnf, ClassExpression::ObjectUnionOf(_)));
}

#[test]
fn cross_phase_modularity_and_metrics() {
    let o = simple_ontology();

    // Phase 4: Metrics
    let metrics = OntologyMetrics::compute(&o);
    assert!(metrics["NumberOfAxioms"] > 0.0);

    // Phase 5: Decomposition
    let decomp = crate::modularity::decomposition::compute_atomic_decomposition(&o);
    assert!(decomp.atom_count() > 0);

    // Phase 5: Module extraction
    use crate::modularity::extractor::{ModuleExtractor, ModuleExtractorConfig};
    use crate::modularity::locality::LocalityClass;
    let config = ModuleExtractorConfig::default();
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Star, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&o, &sig);
    assert!(!module.axioms().is_empty());
}

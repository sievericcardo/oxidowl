//! Comprehensive Example: OWL API v5 Features in Oxidowl
//!
//! Demonstrates all major OWL API v5 feature areas: DataFactory,
//! OntologyManager, parsers, reasoners, profiles, SWRL, SHACL,
//! explanation, modularity, transforms, DL clauses, distributed
//! config, shortform providers, vocabularies, and datatypes.
//!
//! Run: cargo run --example comprehensive_owlapi_usage

#![allow(unused_imports, unused_variables, dead_code)]

use oxidowl::{
    Annotation, AnnotationProperty, AnnotationValue, Class, ClassExpression, DataFactory,
    DataProperty, DataPropertyExpression, EntityType, IRI, ImportsDeclaration, Individual,
    Literal, NamedIndividual, ObjectProperty, ObjectPropertyExpression, Ontology,
    OntologyFormat, OntologyID, OntologyRef, Signature,
    // Manager
    ChangeApplied, ChangeBroadcastStrategy, ManagerConfig, OntologyChange,
    OntologyManager, OntologyManagerRef, Snapshot,
    OntologyLoader, LoaderConfig, MissingImportStrategy,
    // IRI Mappers
    AutoIRIMapper, CompositeIRIMapper, NonMappingOntologyIRIMapper,
    OntologyIRIMapper, SimpleIRIMapper,
    // Listeners
    LoggingChangeListener, NoOpChangeListener, OntologyChangeListener,
    // Reasoner
    Reasoner, ReasonerConfig, ReasoningTask,
    // Reasoner API
    Node, NodeSet, OWLReasoner, OWLReasonerConfiguration, StructuralReasoner,
    // Parsers
    FunctionalParser, OwlXmlSerializer, RdfXmlSerializer, TurtleSerializer,
    ParserFactory, save_file, save_file_gzip, save_to_string,
    // RIO
    NQuadsRenderer, N3Renderer, TriGRenderer, TriXRenderer,
    JsonLdRenderer, RdfJsonRenderer,
    // OBO
    OBOOutputConfig, OBOWriter,
    // Queries
    DLQueryEngine, DLQueryParser, QueryResult, QueryService, QueryType,
    // SWRL
    SWRLAtom, SWRLIArgument, SWRLRule, SWRLRuleEngine, SWRLVariable,
    // Profiles
    ProfileDetectionResult, ProfileValidationReport, ProfileViolation,
    ProfileViolationType, ProfileValidator, OWL2ProfileValidator,
    OWL2Profile as Profile,
    // SHACL
    ShaclConfig, ShaclSeverity, ShaclValidationReport, ShaclValidator,
    // Explanation
    BlackBoxConfig, BlackBoxExplanation, ExplanationGenerator,
    HSTConfig, HSTExplanationGenerator, BlackBoxOWLDebugger,
    DebuggerConfig, OWLDebugger,
    // Inference
    InferredClassAssertionAxiomGenerator, InferredDisjointClassesAxiomGenerator,
    InferredEquivalentClassAxiomGenerator, InferredSubClassOfAxiomGenerator,
    InferredSubDataPropertyAxiomGenerator, InferredSubObjectPropertyAxiomGenerator,
    InferredAxiomGenerator, OntologyMetrics,
    // Modularity
    AtomicDecomposer, AtomicDecomposition, DecomposerConfig,
    ModuleExtractor, ModuleExtractorConfig, ModuleType, LocalityClass,
    SyntacticLocalityEvaluator,
    // Transforms
    DLExpressivity, DLExpressivityChecker, NNFConverter,
    OWLEntityRemover, OWLEntityRenamer, OWLObjectTransformer,
    // Walkers
    OntologyWalker, StructureWalker, OWLOntologyMerger,
    // DL Clauses
    DLClause, DLClauseGenerator, DLClauseSet,
    // RDF-Star
    HornedOwlAdapter, RdfStarCapable,
    // Distributed
    DistributedConfig, NodeConfig, NodeCapabilities, NodeSettings,
    ClusterConfig,
    // Entity Search
    EntityIndex, EntitySearcher,
    // Vocabularies
    Namespaces, PrefixManager,
    // Datatypes
    ConstrainingFacet, DatatypeCategory, OWL2Datatype, OWLFacet,
    // ShortForm
    AnnotationValueShortFormProvider, OntologyIRIShortFormProvider,
    QNameShortFormProvider, ShortFormProvider, SimpleShortFormProvider,
    // Import
    ImportDeclaration, ImportManager,
    // Reasoning Service
    ReasoningService,
    // Axioms
    ontology::axioms::{
        Axiom, DeclarationAxiom, Entity, SubClassOfAxiom, DisjointClassesAxiom,
        ClassAssertionAxiom, ObjectPropertyAssertionAxiom, DataPropertyAssertionAxiom,
    },
    // Parsers trait
    parsers::OntologySerializer,
    // Error
    Error, Result,
};

use std::sync::{Arc, RwLock};

const NS: &str = "http://example.org/demo#";

fn ce(iri: &str) -> ClassExpression { ClassExpression::Class(Class{iri:IRI::new(iri)}) }
fn ind(iri: &str) -> Individual { Individual::Named(NamedIndividual{iri:IRI::new(iri)}) }
fn ope(iri: &str) -> ObjectPropertyExpression { ObjectPropertyExpression::ObjectProperty(ObjectProperty{iri:IRI::new(iri)}) }

fn demo_ontology() -> Ontology {
    let mut o = Ontology::new();
    o.add_axiom(Axiom::Declaration(DeclarationAxiom{id:1,entity:Entity::Class(IRI::new(&format!("{NS}Person")))}));
    o.add_axiom(Axiom::Declaration(DeclarationAxiom{id:2,entity:Entity::Class(IRI::new(&format!("{NS}Employee")))}));
    o.add_axiom(Axiom::SubClassOf(SubClassOfAxiom{id:10,subclass:ce(&format!("{NS}Employee")),superclass:ce(&format!("{NS}Person")),annotations:vec![]}));
    o.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom{id:30,individual:ind(&format!("{NS}alice")),class:ce(&format!("{NS}Person")),annotations:vec![]}));
    o.annotations.push(Annotation{property:AnnotationProperty{iri:IRI::new("http://purl.org/dc/elements/1.1/title")},value:AnnotationValue::Literal(Literal::new("Demo".into())),annotations:vec![]});
    o
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║  Oxidowl — OWL API v5 Comprehensive Feature Demo    ║");
    println!("╚══════════════════════════════════════════════════════╝\n");

    demo_data_factory()?;
    demo_ontology_manager()?;
    demo_parsers()?;
    demo_reasoner().await?;
    demo_entity_searcher()?;
    demo_query_system().await?;
    demo_profiles()?;
    demo_swrl()?;
    demo_shacl()?;
    demo_explanation()?;
    demo_inference()?;
    demo_modularity()?;
    demo_transforms()?;
    demo_merger()?;
    demo_shortform()?;
    demo_dl_clauses()?;
    demo_change_system()?;
    demo_iri_mappers()?;
    demo_import()?;
    demo_metrics()?;
    demo_prefixes()?;
    demo_datatypes()?;
    demo_distributed()?;

    println!("\n╔══════════════════════════════════════════════════════╗");
    println!("║  All 23 feature demonstrations completed.           ║");
    println!("╚══════════════════════════════════════════════════════╝\n");
    Ok(())
}

// 1 ── DataFactory ────────────────────────────────────────────────────────────

fn demo_data_factory() -> Result<()> {
    println!("── 1. DataFactory ──");
    let df = DataFactory::new();
    let c1 = df.get_class(&IRI::new(&format!("{NS}Person")));
    let c2 = df.get_class(&IRI::new(&format!("{NS}Person")));
    println!("   Class interning: {}", c1 == c2);
    println!("   ObjectProperty: {}", df.get_object_property(&IRI::new("http://ex.org#p")).iri);
    println!("   Datatype: {}", df.get_owl_datatype(&IRI::new(oxidowl::ontology::vocabulary::xsd::INTEGER)).iri);
    println!("   AnonymousIndividual: {}", df.get_anonymous_individual().id);
    println!("   Literals: int={}, bool={}, dt={}",
        df.get_integer_literal(42), df.get_boolean_literal(true),
        df.get_date_time_literal("2025-01-15T10:30:00"));
    Ok(())
}

// 2 ── OntologyManager ────────────────────────────────────────────────────────

fn demo_ontology_manager() -> Result<()> {
    println!("\n── 2. OntologyManager ──");
    let mut mgr = OntologyManager::new();
    let iri = IRI::new("http://ex.org/demo");
    let ont_ref = mgr.create_ontology(iri.clone());
    { let mut o = ont_ref.write().unwrap(); *o = demo_ontology(); o.set_iri(iri.clone()); }
    println!("   contains={}, count={}", mgr.contains_ontology(&iri), mgr.ontology_count());
    Ok(())
}

// 3 ── Parsers ────────────────────────────────────────────────────────────────

fn demo_parsers() -> Result<()> {
    println!("\n── 3. Parsers ──");
    let ont = demo_ontology();
    println!("   OwlXml: {} chars", OwlXmlSerializer::new().serialize(&ont)?.len());
    println!("   RdfXml: {} chars", RdfXmlSerializer::new().serialize(&ont)?.len());
    println!("   Turtle: {} chars", TurtleSerializer::new().serialize(&ont)?.len());
    let s = save_to_string(&ont, OntologyFormat::Functional)?;
    let parsed = FunctionalParser::new().parse_string(&s)?;
    println!("   Functional roundtrip: {} axioms", parsed.axioms().len());
    let _p = ParserFactory::create_parser(OntologyFormat::OwlXml)?;
    println!("   ParserFactory: created");
    Ok(())
}

// 4 ── Reasoner ────────────────────────────────────────────────────────────────

async fn demo_reasoner() -> Result<()> {
    println!("\n── 4. Reasoner ──");
    let mut r = Reasoner::new(ReasonerConfig::default())?;
    let ont = demo_ontology();
    r.load_ontology(ont.clone())?;
    println!("   Consistent: {}", r.is_consistent()?);
    let h = r.classify()?;
    println!("   Classified: {} classes", h.hierarchy.len());
    let person = ce(&format!("{NS}Person"));
    println!("   Person satisfiable: {}", r.is_class_satisfiable(&person)?);
    let sr = StructuralReasoner::new(Arc::new(RwLock::new(ont)));
    let subs = sr.get_sub_classes(&person, false)?;
    println!("   StructuralReasoner sub-classes: {}", subs.get_nodes().len());
    Ok(())
}

// 5 ── EntitySearcher ─────────────────────────────────────────────────────────

fn demo_entity_searcher() -> Result<()> {
    println!("\n── 5. EntitySearcher ──");
    let ont = demo_ontology();
    let idx = EntityIndex::from_ontology(&ont);
    let searcher = EntitySearcher::new(&ont, &idx);
    println!("   SubClass axioms: {}", searcher.get_sub_class_axioms_for_lhs(&ce(&format!("{NS}Person"))).len());
    println!("   EquivalentClass: {}", searcher.get_equivalent_classes_axioms(&ce(&format!("{NS}Person"))).len());
    Ok(())
}

// 6 ── Query System ───────────────────────────────────────────────────────────

async fn demo_query_system() -> Result<()> {
    println!("\n── 6. Query System ──");
    let ont = demo_ontology();
    let svc = Arc::new(ReasoningService::new(ont.clone(), ReasonerConfig::default())?);
    let engine = DLQueryEngine::new_with_namespace(svc.clone(), NS.into());
    let result = engine.execute_query("subclasses: Person").await?;
    println!("   DLQuery 'subclasses: Person': {:?}", result);
    let parser = DLQueryParser::new();
    let parsed = parser.parse("Employee and Manager")?;
    println!("   DLQueryParser: {:?}", parsed);
    Ok(())
}

// 7 ── Profiles ──────────────────────────────────────────────────────────────

fn demo_profiles() -> Result<()> {
    println!("\n── 7. Profiles ──");
    let ont = demo_ontology();
    let v = OWL2ProfileValidator::new();
    let report = v.validate_profile(&ont, oxidowl::profiles::OWL2Profile::DL)?;
    println!("   DL conforms: {}", report.conforms);
    let mut det = ProfileDetectionResult::new();
    det.add_analysis(oxidowl::profiles::OWL2Profile::DL, report);
    println!("   Recommended: {}", det.recommended_profile().name());
    Ok(())
}

// 8 ── SWRL ───────────────────────────────────────────────────────────────────

fn demo_swrl() -> Result<()> {
    println!("\n── 8. SWRL ──");
    let rule = SWRLRule::new(
        vec![SWRLAtom::ClassAtom{predicate:ce(&format!("{NS}Person")),argument:SWRLIArgument::Variable(SWRLVariable{iri:IRI::new("urn:swrl#x")})}],
        vec![]);
    println!("   Rule: {} head atoms", rule.head.len());
    let _engine = SWRLRuleEngine::new(oxidowl::swrl::SWRLConfig::default());
    println!("   SWRLRuleEngine: created");
    Ok(())
}

// 9 ── SHACL ─────────────────────────────────────────────────────────────────

fn demo_shacl() -> Result<()> {
    println!("\n── 9. SHACL ──");
    let _v = ShaclValidator::with_config("", "", ShaclConfig::default())?;
    println!("   ShaclSeverity::Violation: {:?}", ShaclSeverity::Violation);
    println!("   ShaclSeverity::Warning: {:?}", ShaclSeverity::Warning);
    Ok(())
}

// 10 ── Explanation ───────────────────────────────────────────────────────────

fn demo_explanation() -> Result<()> {
    println!("\n── 10. Explanation ──");
    let bb = BlackBoxExplanation::new(Arc::new(oxidowl::reasoner_api::TableauReasonerFactory), BlackBoxConfig::default());
    println!("   BlackBoxExplanation: created");
    let _hst = HSTExplanationGenerator::new(Arc::new(oxidowl::reasoner_api::TableauReasonerFactory), HSTConfig::default());
    println!("   HSTExplanationGenerator: created");
    let ont_ref = Arc::new(RwLock::new(demo_ontology()));
    let _dbg = BlackBoxOWLDebugger::new(
        ont_ref,
        Arc::new(oxidowl::reasoner_api::TableauReasonerFactory),
        DebuggerConfig::default(),
    );
    println!("   BlackBoxOWLDebugger: created");
    let _gen: &dyn ExplanationGenerator = &bb;
    println!("   ExplanationGenerator trait: satisfied");
    Ok(())
}

// 11 ── Inference Generators ─────────────────────────────────────────────────

fn demo_inference() -> Result<()> {
    println!("\n── 11. Inference Generators ──");
    let _sub = InferredSubClassOfAxiomGenerator;
    let _eq = InferredEquivalentClassAxiomGenerator;
    let _dis = InferredDisjointClassesAxiomGenerator;
    let _ca = InferredClassAssertionAxiomGenerator;
    println!("   All 4 inference generators: unit structs available");
    Ok(())
}

// 12 ── Modularity ────────────────────────────────────────────────────────────

fn demo_modularity() -> Result<()> {
    println!("\n── 12. Modularity ──");
    let ont = demo_ontology();
    let _eval = SyntacticLocalityEvaluator::new(LocalityClass::Star);
    println!("   SyntacticLocalityEvaluator: created");
    let cfg = ModuleExtractorConfig{module_type:ModuleType::Star,..Default::default()};
    let _ext = ModuleExtractor::new_syntactic(LocalityClass::Star, cfg);
    println!("   ModuleExtractor (Star): created");
    let deco = AtomicDecomposer::new(DecomposerConfig::default());
    let decomp: AtomicDecomposition = deco.decompose(&ont);
    println!("   AtomicDecomposition: {} atoms", decomp.atoms.len());
    Ok(())
}

// 13 ── Transforms ────────────────────────────────────────────────────────────

fn demo_transforms() -> Result<()> {
    println!("\n── 13. Transforms ──");
    let ont = demo_ontology();
    let ont_ref = Arc::new(RwLock::new(ont.clone()));
    let renamer = OWLEntityRenamer::new();
    println!("   OWLEntityRenamer: {} changes", renamer.rename_ontology(&ont_ref)?.len());
    let remover = OWLEntityRemover::new();
    println!("   OWLEntityRemover: {} changes", remover.remove_entities(&ont_ref)?.len());
    let _t = OWLObjectTransformer::new_ce(|ce| Some(ce.clone()));
    println!("   OWLObjectTransformer: created");
    let nnf = NNFConverter;
    let _ = nnf.to_nnf(&ce(&format!("{NS}Person")));
    println!("   NNFConverter: converted");
    let checker = DLExpressivityChecker;
    println!("   DLExpressivity: {:?}", checker.analyze(&ont));
    Ok(())
}

// 14 ── OWLOntologyMerger ────────────────────────────────────────────────────

fn demo_merger() -> Result<()> {
    println!("\n── 14. OWLOntologyMerger ──");
    let mut mgr = OntologyManager::new();
    let m = OWLOntologyMerger::new(IRI::new("http://ex.org/merged"));
    let ont1 = Arc::new(RwLock::new(demo_ontology()));
    let ont2 = Arc::new(RwLock::new(demo_ontology()));
    let merged = m.merge(&[ont1, ont2], &mut mgr)?;
    {
        let guard = merged.read().unwrap();
        println!("   Merged: {} axioms", guard.axioms().len());
    }
    Ok(())
}

// 15 ── ShortForm ─────────────────────────────────────────────────────────────

fn demo_shortform() -> Result<()> {
    println!("\n── 15. ShortForm ──");
    let entity = oxidowl::ontology::axioms::Entity::Class(IRI::new(&format!("{NS}Person")));
    let sf = SimpleShortFormProvider.get_short_form(&entity);
    println!("   SimpleShortForm: Person → '{sf}'");
    let ont_ref = Arc::new(RwLock::new(demo_ontology()));
    let _asp = AnnotationValueShortFormProvider::new(ont_ref, Box::new(SimpleShortFormProvider));
    println!("   AnnotationValueShortFormProvider: created");
    Ok(())
}

// 16 ── DL Clauses ────────────────────────────────────────────────────────────

fn demo_dl_clauses() -> Result<()> {
    println!("\n── 16. DL Clauses ──");
    let mut generator = DLClauseGenerator::new();
    let clauses: DLClauseSet = generator.generate_clauses(&demo_ontology())?;
    println!("   DLClauseGenerator: {} clauses", clauses.total_clauses());
    Ok(())
}

// 17 ── Change System ─────────────────────────────────────────────────────────

fn demo_change_system() -> Result<()> {
    println!("\n── 17. Change System ──");
    let mut mgr = OntologyManager::new_with_config(ManagerConfig{enable_change_history:true,max_history_size:100,silent_missing_imports:true,max_import_depth:20});
    let iri = IRI::new("http://ex.org/changes");
    let ont_ref = mgr.create_ontology(iri.clone());
    mgr.add_change_listener(Box::new(LoggingChangeListener::new(log::Level::Info)));
    mgr.add_change_listener(Box::new(NoOpChangeListener));
    mgr.set_broadcast_strategy(ChangeBroadcastStrategy::Immediate);
    println!("   Broadcast strategy: Immediate");
    let snap = mgr.snapshot_ontology(&iri);
    println!("   Snapshot: {} axioms", snap.map(|s|s.axioms.len()).unwrap_or(0));
    Ok(())
}

// 18 ── IRI Mappers ───────────────────────────────────────────────────────────

fn demo_iri_mappers() -> Result<()> {
    println!("\n── 18. IRI Mappers ──");
    let sm = SimpleIRIMapper::new(IRI::new("http://ex.org/o"),IRI::new("file:///o.owl"));
    println!("   SimpleIRIMapper: {:?}", sm.get_document_iri(&IRI::new("http://ex.org/o")));
    let _cm = CompositeIRIMapper::new(vec![Box::new(sm)]);
    println!("   CompositeIRIMapper: created");
    Ok(())
}

// 19 ── ImportManager ────────────────────────────────────────────────────────

fn demo_import() -> Result<()> {
    println!("\n── 19. ImportManager ──");
    let _im = ImportManager::new(oxidowl::import::ImportManagerConfig::default());
    println!("   ImportManager: created");
    let decl = ImportDeclaration::new(IRI::new("http://ex.org/imported"));
    println!("   ImportDeclaration: {decl:?}");
    Ok(())
}

// 20 ── OntologyMetrics ──────────────────────────────────────────────────────

fn demo_metrics() -> Result<()> {
    println!("\n── 20. OntologyMetrics ──");
    let _ = OntologyMetrics::compute(&demo_ontology());
    println!("   OntologyMetrics: computed");
    Ok(())
}

// 21 ── Prefixes ──────────────────────────────────────────────────────────────

fn demo_prefixes() -> Result<()> {
    println!("\n── 21. Namespaces ──");
    println!("   Standard prefixes: {}", Namespaces::all().len());
    Ok(())
}

// 22 ── Datatypes ─────────────────────────────────────────────────────────────

fn demo_datatypes() -> Result<()> {
    println!("\n── 22. Datatypes ──");
    println!("   OWL2Datatype::Integer: {:?}", OWL2Datatype::Integer);
    println!("   OWL2Datatype::String: {:?}", OWL2Datatype::String);
    println!("   DatatypeCategory::String: {:?}", DatatypeCategory::String);
    println!("   DatatypeCategory::Numeric: {:?}", DatatypeCategory::Numeric);
    println!("   OWLFacet::XsdLength: {:?}", OWLFacet::XsdLength);
    println!("   OWLFacet::XsdMinInclusive: {:?}", OWLFacet::XsdMinInclusive);
    println!("   ConstrainingFacet::MinInclusive: {:?}", ConstrainingFacet::MinInclusive);
    Ok(())
}

// 23 ── Distributed ───────────────────────────────────────────────────────────

fn demo_distributed() -> Result<()> {
    println!("\n── 23. Distributed ──");
    let n = NodeConfig{
        node_id:uuid::Uuid::new_v4(),
        address:"127.0.0.1:9090".parse().unwrap(),
        capabilities:NodeCapabilities{cpu_cores:8,memory_mb:16384,storage_gb:100,network_bandwidth_mbps:1000,reasoning_features:vec!["tableau".into()]},
        settings:NodeSettings::default()};
    println!("   NodeConfig: id={} addr={}", n.node_id, n.address);
    let dist = DistributedConfig::default();
    println!("   DistributedConfig: cluster_name={}", dist.cluster_config.cluster_name);
    Ok(())
}

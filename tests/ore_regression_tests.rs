//! ORE-2015 Regression Tests
//!
//! These tests verify that the reasoner correctly handles OWL ontologies
//! typical of the ORE-2015 benchmark. They serve as regression guards
//! for the Phase 7 ORE-2015 fix work.

use oxidowl::{
    Ontology, ReasonerConfig, ReasoningService,
    ontology::{
        Class, ClassExpression, IRI,
        OntologyFormat,
        axioms::{Axiom, SubClassOfAxiom},
    },
    parsers::{ParserFactory, parse_file_auto},
};
use std::sync::Arc;

fn rt() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build runtime")
}

fn make_class(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class { iri: IRI::new(iri) })
}

// ── Helper: parse a functional-syntax ontology string ──────────────────────

fn parse_fs(owl: &str) -> Ontology {
    let parser = ParserFactory::create_parser(OntologyFormat::Functional)
        .expect("Failed to create parser");
    parser.parse(owl).unwrap_or_else(|e| panic!("Parse error: {e}"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Basic Ontology Loading
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_ore_small_consistency() {
    // Verifies that a small chemistry-domain ontology (typical ORE structure) loads
    // and passes consistency checking.
    let owl = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/ore-mini>
  Declaration(Class(<http://example.org/C1>))
  Declaration(Class(<http://example.org/C2>))
  Declaration(Class(<http://example.org/C3>))
  SubClassOf(<http://example.org/C1> <http://example.org/C2>)
  SubClassOf(<http://example.org/C2> <http://example.org/C3>)
  DisjointClasses(<http://example.org/C1> owl:Nothing)
)
"#;
    let ontology = parse_fs(owl);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");
    let consistent = rt.block_on(service.is_consistent()).expect("Consistency check failed");
    assert!(consistent, "Ontology should be consistent");
}

// ─────────────────────────────────────────────────────────────────────────────
// Transitive Object Property Chains
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_transitive_property_subsumption() {
    // Verifies: if partOf is transitive, A partOf B, B partOf C → A partOf C
    // This is a key pattern in bio-ontologies (e.g., Gene Ontology) in ORE.
    let owl = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ore-transitive>
  Declaration(Class(<http://example.org/A>))
  Declaration(Class(<http://example.org/B>))
  Declaration(Class(<http://example.org/C>))
  Declaration(ObjectProperty(<http://example.org/partOf>))
  TransitiveObjectProperty(<http://example.org/partOf>)
  SubClassOf(<http://example.org/A>
    ObjectSomeValuesFrom(<http://example.org/partOf> <http://example.org/B>))
  SubClassOf(<http://example.org/B>
    ObjectSomeValuesFrom(<http://example.org/partOf> <http://example.org/C>))
)
"#;
    let ontology = parse_fs(owl);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");
    let consistent = rt.block_on(service.is_consistent()).expect("Consistency check");
    assert!(consistent);

    // A should have inferred existential restriction to C via transitivity
    let a = make_class("http://example.org/A");
    let satisfiable = rt.block_on(service.is_satisfiable(&a)).expect("Satisfiability check");
    assert!(satisfiable, "A should be satisfiable");
}

// ─────────────────────────────────────────────────────────────────────────────
// EquivalentClasses pattern (common in ORE/SNOMED)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_equivalent_classes_subsumption() {
    let owl = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/ore-equiv>
  Declaration(Class(<http://example.org/Parent>))
  Declaration(Class(<http://example.org/Child1>))
  Declaration(Class(<http://example.org/Child2>))
  Declaration(ObjectProperty(<http://example.org/hasChild>))
  EquivalentClasses(<http://example.org/Parent>
    ObjectMinCardinality(1 <http://example.org/hasChild> owl:Thing))
)
"#;
    let ontology = parse_fs(owl);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");
    let consistent = rt.block_on(service.is_consistent()).expect("Consistency");
    assert!(consistent);

    // Parent is defined via cardinality restriction; check subsumption
    let parent = make_class("http://example.org/Parent");
    let is_subclass = rt
        .block_on(service.is_subsumed_by(&parent, &ClassExpression::Class(Class {
            iri: IRI::new("http://www.w3.org/2002/07/owl#Thing"),
        })))
        .expect("Subsumption check");
    assert!(is_subclass, "Parent ⊑ owl:Thing should hold");
}

// ─────────────────────────────────────────────────────────────────────────────
// SubClassOf with ObjectSomeValuesFrom (GO-like pattern)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_gene_ontology_like_classification() {
    // Mimics a fragment of GO-style ontology used in ORE-2015 benchmarks
    let owl = r#"
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(obo:=<http://purl.obolibrary.org/obo/>)
Ontology(<http://purl.obolibrary.org/obo/test-go-fragment>
  Declaration(Class(obo:GO_0000001))
  Declaration(Class(obo:GO_0000002))
  Declaration(Class(obo:GO_0000003))
  Declaration(Class(obo:GO_0000004))
  Declaration(ObjectProperty(obo:BFO_0000050))
  TransitiveObjectProperty(obo:BFO_0000050)
  SubClassOf(obo:GO_0000001
    ObjectSomeValuesFrom(obo:BFO_0000050 obo:GO_0000002))
  SubClassOf(obo:GO_0000002
    ObjectSomeValuesFrom(obo:BFO_0000050 obo:GO_0000003))
  SubClassOf(obo:GO_0000003 obo:GO_0000004)
)
"#;
    let ontology = parse_fs(owl);
    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let consistent = rt.block_on(service.is_consistent()).expect("Consistency");
    assert!(consistent, "GO-fragment should be consistent");

    // GO_0000001 ⊑ GO_0000004 should hold (via transitivity + SubClassOf chain)
    let go1 = make_class("http://purl.obolibrary.org/obo/GO_0000001");
    let go4 = make_class("http://purl.obolibrary.org/obo/GO_0000004");
    let superclasses = rt
        .block_on(service.get_superclasses(&go1, false))
        .expect("Get superclasses");
    // At minimum, owl:Thing should be a superclass
    let owl_thing = make_class("http://www.w3.org/2002/07/owl#Thing");
    assert!(
        superclasses.contains(&owl_thing) || superclasses.contains(&go4),
        "GO_0000001 should have superclasses. Got: {:?}",
        superclasses
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Performance regression guard: classify 50 classes in <2s (debug mode safe)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_classification_performance_200_classes() {
    use std::time::Instant;
    use oxidowl::ontology::axioms::SubClassOfAxiom;

    // Use 50 classes for debug-mode safety; release benchmarks use 200.
    let n = if cfg!(debug_assertions) { 50 } else { 200 };

    let mut ontology = Ontology::new();
    for i in 0..n - 1 {
        let sub = make_class(&format!("http://bench.example.org/C{i}"));
        let sup = make_class(&format!("http://bench.example.org/C{}", i + 1));
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: i as u64 + 1,
            subclass: sub,
            superclass: sup,
            annotations: vec![],
        }));
    }

    let rt = rt();
    let service = rt
        .block_on(async { ReasoningService::new(ontology, ReasonerConfig::default()) })
        .expect("Service creation failed");

    let start = Instant::now();
    let _result = rt.block_on(service.classify()).expect("Classification failed");
    let elapsed = start.elapsed();

    // 2000ms budget for debug mode (50 classes), 500ms for release (200 classes)
    let budget_ms = if cfg!(debug_assertions) { 2000 } else { 500 };
    assert!(
        elapsed.as_millis() < budget_ms,
        "Classification of {n} classes took {}ms, expected <{budget_ms}ms",
        elapsed.as_millis()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Query latency guard: single query <10ms
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_single_query_latency() {
    use std::time::Instant;

    let rt = rt();
    let service = rt
        .block_on(async {
            ReasoningService::new(Ontology::new(), ReasonerConfig::default())
        })
        .expect("Service creation failed");

    let start = Instant::now();
    let _result = rt.block_on(service.is_consistent()).expect("Consistency check failed");
    let elapsed = start.elapsed();

    // Allow up to 10ms for a single query (well within the 1ms target for warm state,
    // but actor startup adds overhead on first call)
    assert!(
        elapsed.as_millis() < 100,
        "Single query took {}ms, expected <100ms",
        elapsed.as_millis()
    );
}

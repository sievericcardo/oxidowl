#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::manager::ManagerConfig;
use oxidowl::ontology::*;
use oxidowl::OntologyManager;
use oxidowl::PrefixManager;

fn onto_ref(o: Ontology) -> OntologyRef {
    std::sync::Arc::new(std::sync::RwLock::new(o))
}

// ══════════════════════════════════════════════════════════════════════════════
// PrefixManager Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn prefix_manager_well_known_owl() {
    let pm = PrefixManager::new();
    assert_eq!(
        pm.expand("owl:Thing"),
        Some("http://www.w3.org/2002/07/owl#Thing".to_string())
    );
    assert_eq!(
        pm.expand("rdf:type"),
        Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string())
    );
    assert_eq!(
        pm.expand("rdfs:label"),
        Some("http://www.w3.org/2000/01/rdf-schema#label".to_string())
    );
    assert_eq!(
        pm.expand("xsd:string"),
        Some("http://www.w3.org/2001/XMLSchema#string".to_string())
    );
}

#[test]
fn prefix_manager_well_known_shorten() {
    let pm = PrefixManager::new();
    assert_eq!(
        pm.shorten("http://www.w3.org/2002/07/owl#Thing"),
        Some("owl:Thing".to_string())
    );
    assert_eq!(
        pm.shorten("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
        Some("rdf:type".to_string())
    );
}

#[test]
fn prefix_manager_custom_prefix() {
    let mut pm = PrefixManager::new();
    pm.add_prefix("ex", "http://example.org/");
    let expanded = pm.expand("ex:Test");
    assert_eq!(expanded, Some("http://example.org/Test".to_string()));
    let shortened = pm.shorten("http://example.org/Test");
    assert_eq!(shortened, Some("ex:Test".to_string()));
}

#[test]
fn prefix_manager_remove_prefix() {
    let mut pm = PrefixManager::new();
    pm.add_prefix("ex", "http://example.org/");
    assert!(pm.expand("ex:Test").is_some());
    pm.remove_prefix("ex");
    assert!(pm.expand("ex:Test").is_none());
}

#[test]
fn prefix_manager_unknown_prefix() {
    let pm = PrefixManager::new();
    assert_eq!(pm.expand("unknown:Thing"), None);
    assert_eq!(pm.shorten("http://unknown.org/thing"), None);
}

#[test]
fn prefix_manager_no_colon() {
    let pm = PrefixManager::new();
    assert_eq!(pm.expand("NoPrefixHere"), None);
}

// ══════════════════════════════════════════════════════════════════════════════
// IRI Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn iri_basic_creation() {
    let iri = IRI::new("http://example.org/Class");
    assert_eq!(iri.as_str(), "http://example.org/Class");
}

#[test]
fn iri_display() {
    let iri = IRI::new("http://example.org/Class");
    assert_eq!(format!("{iri}"), "http://example.org/Class");
}

#[test]
fn iri_equality() {
    let i1 = IRI::new("http://ex.org/A");
    let i2 = IRI::new("http://ex.org/A");
    let i3 = IRI::new("http://ex.org/B");
    assert_eq!(i1, i2);
    assert_ne!(i1, i3);
}

#[test]
fn iri_owl_thing() {
    let thing = IRI::owl_thing();
    assert!(thing.is_owl_thing());
    assert!(thing.is_reserved_vocabulary());
}

#[test]
fn iri_owl_nothing() {
    let nothing = IRI::owl_nothing();
    assert!(nothing.is_owl_nothing());
    assert!(nothing.is_reserved_vocabulary());
}

// ══════════════════════════════════════════════════════════════════════════════
// Ontology Metrics Tests
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::inference::metrics::OntologyMetrics;

#[test]
fn ontology_metrics_axiom_counts() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let metrics = OntologyMetrics::compute(&onto);
    assert!(*metrics.get("NumberOfAxioms").unwrap() > 0.0);
    assert!(*metrics.get("NumberOfClasses").unwrap() >= 3.0);
    assert!(*metrics.get("NumberOfSubClassAxioms").unwrap() >= 2.0);
}

#[test]
fn ontology_metrics_empty() {
    let o = Ontology::new();
    let metrics = OntologyMetrics::compute(&o);
    assert_eq!(*metrics.get("NumberOfAxioms").unwrap(), 0.0);
}

// ══════════════════════════════════════════════════════════════════════════════
// OntologyWalker Tests
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::walk::{OWLObjectVisitor, OntologyWalker};

struct CountingVisitor {
    class_count: usize,
    axiom_count: usize,
}

impl OWLObjectVisitor for CountingVisitor {
    fn visit_class_expression(&mut self, _ce: &ClassExpression) {
        self.class_count += 1;
    }
    fn visit_axiom(&mut self, _axiom: &Axiom) {
        self.axiom_count += 1;
    }
}

#[test]
fn ontology_walker_counts() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let mut walker = OntologyWalker::new(CountingVisitor {
        class_count: 0,
        axiom_count: 0,
    });
    walker.walk_ontology(&onto);
    let visitor = walker.into_visitor();
    assert!(visitor.class_count > 0);
    assert!(visitor.axiom_count > 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// OWLOntologyMerger Tests
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::walk::merge::OWLOntologyMerger;

#[test]
fn ontology_merger_basic() {
    let df = DF::new();
    let o1 = onto_ref(df.new_ontology_with_iri("http://ex.org/O1"));
    let o2 = onto_ref(df.new_ontology_with_iri("http://ex.org/O2"));
    let mut manager = OntologyManager::new();
    let merger = OWLOntologyMerger::new(IRI::new("http://ex.org/Merged"));
    let result = merger.merge(&[o1, o2], &mut manager);
    assert!(result.is_ok());
}

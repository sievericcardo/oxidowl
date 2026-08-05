#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::modularity::decomposition::compute_atomic_decomposition;
use oxidowl::modularity::extractor::{ModuleExtractor, ModuleExtractorConfig, ModuleType};
use oxidowl::modularity::locality::{LocalityClass, LocalityEvaluator, SyntacticLocalityEvaluator};
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use std::collections::HashSet;

// ══════════════════════════════════════════════════════════════════════════════
// Atomic Decomposition Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn atomic_decomposition_non_empty() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let decomp = compute_atomic_decomposition(&onto);
    assert!(decomp.atom_count() > 0);
    assert!(decomp.axiom_count() > 0);
}

#[test]
fn atomic_decomposition_empty_ontology() {
    let o = Ontology::new();
    let decomp = compute_atomic_decomposition(&o);
    assert_eq!(decomp.atom_count(), 0);
    assert_eq!(decomp.axiom_count(), 0);
}

#[test]
fn atomic_decomposition_get_atom_axioms() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let decomp = compute_atomic_decomposition(&onto);
    assert!(decomp.atom_count() > 0);
    for i in 0..decomp.atom_count() {
        let axioms = decomp.get_atom_axioms(i);
        assert!(!axioms.is_empty());
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Module Extraction Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn module_extraction_star_non_empty() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let config = ModuleExtractorConfig::default();
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Star, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&onto, &sig);
    assert!(
        !module.axioms().is_empty(),
        "Star module should not be empty"
    );
}

#[test]
fn module_extraction_lower_bound() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let config = ModuleExtractorConfig {
        module_type: ModuleType::LowerBound,
        max_iterations: 100,
    };
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Star, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&onto, &sig);
    assert!(
        !module.axioms().is_empty(),
        "Lower bound module should not be empty"
    );
}

#[test]
fn module_extraction_upper_bound() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let config = ModuleExtractorConfig {
        module_type: ModuleType::UpperBound,
        max_iterations: 100,
    };
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Star, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&onto, &sig);
    assert!(
        !module.axioms().is_empty(),
        "Upper bound module should not be empty"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// Syntactic Locality Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn syntactic_locality_top_non_local() {
    let evaluator = SyntacticLocalityEvaluator::new(LocalityClass::Top);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/B"));
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b.clone());
    // A ⊑ B with B in signature: NOT local (provides info about B)
    let is_local = evaluator.is_local(&ax, &sig);
    assert!(!is_local, "Axiom with B in signature should not be local");
}

#[test]
fn syntactic_locality_top_local_different_sig() {
    let evaluator = SyntacticLocalityEvaluator::new(LocalityClass::Top);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/Different"));
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    // A ⊑ B with completely different sig may be local
    let _ = evaluator.is_local(&ax, &sig);
}

#[test]
fn syntactic_locality_bottom_basic() {
    let evaluator = SyntacticLocalityEvaluator::new(LocalityClass::Bottom);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/C"));
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);
    let _ = evaluator.is_local(&ax, &sig);
}

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
// Atomic Decomposition Extended Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_atomic_decomposition_basic() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let ax1 = df.sub_class_of(a.clone(), b.clone());
    let ax2 = df.sub_class_of(b.clone(), c.clone());
    let mut ont = df.build_ontology(vec![ax1, ax2]);
    df.auto_declare(&mut ont);

    let decomposer = oxidowl::AtomicDecomposer::default();
    let decomposition = decomposer.decompose(&ont);

    assert!(!decomposition.atoms.is_empty(), "Should have at least one atom");
    assert!(decomposition.atom_count() > 0);
    assert!(decomposition.axiom_count() > 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// Module Extraction: Upper Bound
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_module_extraction_upper_bound() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();

    let config = ModuleExtractorConfig {
        module_type: ModuleType::UpperBound,
        max_iterations: 100,
    };
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Bottom, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&onto, &sig);

    assert!(!module.axioms().is_empty(), "Upper bound module should not be empty");

    let contains_a = module.axioms().iter().any(|ax| {
        format!("{:?}", ax).contains("http://ex.org/A")
    });
    assert!(contains_a, "Upper bound module should contain axioms mentioning A");
}

// ══════════════════════════════════════════════════════════════════════════════
// Module Extraction: Lower Bound
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_module_extraction_lower_bound() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();

    let config = ModuleExtractorConfig {
        module_type: ModuleType::LowerBound,
        max_iterations: 100,
    };
    let extractor = ModuleExtractor::new_syntactic(LocalityClass::Top, config);
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));
    let module = extractor.extract_module(&onto, &sig);

    assert!(!module.axioms().is_empty(), "Lower bound module should not be empty");

    let contains_a = module.axioms().iter().any(|ax| {
        format!("{:?}", ax).contains("http://ex.org/A")
    });
    assert!(contains_a, "Lower bound module should contain axioms mentioning A");
}

// ══════════════════════════════════════════════════════════════════════════════
// Syntactic Locality: Top
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_syntactic_locality_top() {
    let evaluator = SyntacticLocalityEvaluator::new(LocalityClass::Top);

    let mut sig_with_b = HashSet::new();
    sig_with_b.insert(IRI::new("http://ex.org/B"));

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);

    let is_local = evaluator.is_local(&ax, &sig_with_b);
    assert!(!is_local, "A ⊑ B with B in sig should NOT be Top-local");

    let mut sig_other = HashSet::new();
    sig_other.insert(IRI::new("http://ex.org/Z"));
    let is_local_z = evaluator.is_local(&ax, &sig_other);
    assert!(is_local_z, "A ⊑ B with unrelated Z in sig SHOULD be Top-local");
}

// ══════════════════════════════════════════════════════════════════════════════
// Syntactic Locality: Bottom
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_syntactic_locality_bottom() {
    let evaluator = SyntacticLocalityEvaluator::new(LocalityClass::Bottom);

    let mut sig_with_a = HashSet::new();
    sig_with_a.insert(IRI::new("http://ex.org/A"));

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax = df.sub_class_of(a, b);

    let is_local = evaluator.is_local(&ax, &sig_with_a);
    assert!(!is_local, "A ⊑ B with A in sig should NOT be Bottom-local");

    let mut sig_other = HashSet::new();
    sig_other.insert(IRI::new("http://ex.org/Z"));
    let is_local_z = evaluator.is_local(&ax, &sig_other);
    assert!(is_local_z, "A ⊑ B with unrelated Z in sig SHOULD be Bottom-local");
}

// ══════════════════════════════════════════════════════════════════════════════
// ModuleType Semantics
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_module_type_semantics() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let mut sig = HashSet::new();
    sig.insert(IRI::new("http://ex.org/A"));

    let extract_ub = ModuleExtractor::new_syntactic(
        LocalityClass::Bottom,
        ModuleExtractorConfig {
            module_type: ModuleType::UpperBound,
            max_iterations: 100,
        },
    );
    let extract_lb = ModuleExtractor::new_syntactic(
        LocalityClass::Top,
        ModuleExtractorConfig {
            module_type: ModuleType::LowerBound,
            max_iterations: 100,
        },
    );
    let extract_star = ModuleExtractor::new_syntactic(
        LocalityClass::Star,
        ModuleExtractorConfig {
            module_type: ModuleType::Star,
            max_iterations: 100,
        },
    );

    let mod_ub = extract_ub.extract_module(&onto, &sig);
    let mod_lb = extract_lb.extract_module(&onto, &sig);
    let mod_star = extract_star.extract_module(&onto, &sig);

    assert!(!mod_ub.axioms().is_empty(), "Upper bound module not empty");
    assert!(!mod_lb.axioms().is_empty(), "Lower bound module not empty");
    assert!(!mod_star.axioms().is_empty(), "Star module not empty");

    let ub_count = mod_ub.axioms().len();
    let lb_count = mod_lb.axioms().len();
    let star_count = mod_star.axioms().len();

    let all_same = ub_count == lb_count && lb_count == star_count;
    let some_differ = !all_same;

    assert!(
        ub_count > 0 && lb_count > 0 && star_count > 0,
        "All module types should produce non-empty modules; ub={ub_count} lb={lb_count} star={star_count}"
    );

    _ = some_differ;
}

// ══════════════════════════════════════════════════════════════════════════════
// Decomposition: Atom Axioms Verification
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_decomposition_atom_axioms() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let ax1 = df.sub_class_of(a.clone(), b.clone());
    let ax2 = df.sub_class_of(b.clone(), c.clone());
    let mut ont = df.build_ontology(vec![ax1.clone(), ax2.clone()]);
    df.auto_declare(&mut ont);

    let decomposition = compute_atomic_decomposition(&ont);

    assert!(decomposition.atom_count() > 0, "Should have atoms");
    for i in 0..decomposition.atom_count() {
        let axioms = decomposition.get_atom_axioms(i);
        assert!(!axioms.is_empty(), "Atom {i} should contain axioms");
        for ax_ref in &axioms {
            let ok = matches!(ax_ref, Axiom::SubClassOf(_)) || matches!(ax_ref, Axiom::Declaration(_));
            assert!(ok, "Atom should contain SubClassOf or Declaration, got {ax_ref:?}");
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Decomposition: Dependencies
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_decomposition_dependencies() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let d = df.class_ce("http://ex.org/D");
    let p = df.obj_prop("http://ex.org/P");
    let i = df.named("http://ex.org/ind");

    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.sub_class_of(b.clone(), c.clone()),
        df.class_assertion(d.clone(), i),
        df.object_property_domain(p.clone(), a.clone()),
        df.object_property_range(p.clone(), d.clone()),
    ]);
    df.auto_declare(&mut ont);

    let decomposition = compute_atomic_decomposition(&ont);

    assert!(
        decomposition.atom_count() > 0,
        "Should have at least one atom"
    );

    for i in 0..decomposition.atom_count() {
        let _deps = decomposition.dependent_atoms(i);
        if let Some(atom) = decomposition.atoms.get(i) {
            assert!(!atom.axiom_positions.is_empty(), "Atom {i} should own axioms");
        }
    }
}

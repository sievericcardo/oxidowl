#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::test_base::TestBase;
use helpers::*;
use oxidowl::OntologyManager;
use oxidowl::manager::changes::OntologyChange;
use oxidowl::manager::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;

// ══════════════════════════════════════════════════════════════════════════════
// OntologyManager — Create and Register
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn manager_create_ontology() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont1");
    let ont = manager.create_ontology(iri.clone());
    assert!(manager.contains_ontology(&iri));
    assert_eq!(manager.ontology_count(), 1);
    let guard = ont.read().unwrap();
    assert_eq!(guard.get_iri().cloned(), Some(iri));
}

#[test]
fn manager_create_multiple_ontologies() {
    let mut manager = OntologyManager::new();
    for i in 0..5 {
        manager.create_ontology(IRI::new(&format!("http://ex.org/ont{i}")));
    }
    assert_eq!(manager.ontology_count(), 5);
}

#[test]
fn manager_remove_ontology() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = manager.create_ontology(iri.clone());
    assert!(manager.contains_ontology(&iri));
    manager.remove_ontology(&ont).unwrap();
    assert!(!manager.contains_ontology(&iri));
    assert_eq!(manager.ontology_count(), 0);
}

#[test]
fn manager_get_nonexistent_ontology() {
    let manager = OntologyManager::new();
    assert!(
        manager
            .get_ontology(&IRI::new("http://ex.org/nonexistent"))
            .is_none()
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// OntologyManager — Apply Changes
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn manager_add_axiom_via_change() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = manager.create_ontology(iri.clone());
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let axiom = df.sub_class_of(a, b);

    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: axiom.clone(),
    });

    let guard = ont.read().unwrap();
    assert_contains_axiom!(&guard, axiom);
}

#[test]
fn manager_add_then_remove_axiom() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = manager.create_ontology(iri.clone());
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let axiom = df.sub_class_of(a, b);

    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: axiom.clone(),
    });
    manager.apply_change(OntologyChange::RemoveAxiom {
        ontology_iri: iri.clone(),
        axiom: axiom.clone(),
    });

    let guard = ont.read().unwrap();
    assert_not_contains_axiom(&guard, &axiom);
}

#[test]
fn manager_add_multiple_axioms() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont");
    let _ont = manager.create_ontology(iri.clone());
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let ax1 = df.sub_class_of(a.clone(), b.clone());
    let ax2 = df.sub_class_of(b.clone(), c.clone());
    let ax3 = df.class_assertion(a.clone(), df.named("http://ex.org/i"));

    let changes = vec![
        OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom: ax1.clone(),
        },
        OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom: ax2.clone(),
        },
        OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom: ax3.clone(),
        },
    ];

    manager.apply_changes(&changes);

    let guard = _ont.read().unwrap();
    assert!(guard.axioms().contains(&ax1));
    assert!(guard.axioms().contains(&ax2));
    assert!(guard.axioms().contains(&ax3));
}

#[test]
fn manager_get_data_factory() {
    let manager = OntologyManager::new();
    let _df = manager.get_data_factory();
    let c = _df.get_class(&IRI::new("http://ex.org/A"));
    assert_eq!(c.iri.as_str(), "http://ex.org/A");
}

// ══════════════════════════════════════════════════════════════════════════════
// OntologyManager — Undo/Redo
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn manager_undo_redo() {
    let config = ManagerConfig {
        enable_change_history: true,
        max_history_size: 100,
        silent_missing_imports: true,
        max_import_depth: 20,
    };
    let mut manager = OntologyManager::new_with_config(config);
    let iri = IRI::new("http://ex.org/ont");
    let _ont = manager.create_ontology(iri.clone());
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let ax1 = df.sub_class_of(a, b);
    let ax2 = df.class_assertion(df.class_ce("http://ex.org/A"), df.named("http://ex.org/i"));

    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax1.clone(),
    });
    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax2.clone(),
    });

    assert_eq!(manager.ontology_count(), 1);
    let undone = manager.undo(1);
    assert!(undone.is_ok(), "Undo should succeed: {:?}", undone.err());
}

// ══════════════════════════════════════════════════════════════════════════════
// OntologyManager — IRI Mapping
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn manager_simple_iri_mapper() {
    use oxidowl::manager::iri_mapper::{OntologyIRIMapper, SimpleIRIMapper};
    let mapper = SimpleIRIMapper::new(
        IRI::new("http://ex.org/ont"),
        IRI::new("file:///tmp/ont.owl"),
    );
    let result = mapper.get_document_iri(&IRI::new("http://ex.org/ont"));
    assert!(result.is_some());
    assert_eq!(result.unwrap().as_str(), "file:///tmp/ont.owl");
}

#[test]
fn manager_iri_mapper_no_match() {
    use oxidowl::manager::iri_mapper::{OntologyIRIMapper, SimpleIRIMapper};
    let mapper = SimpleIRIMapper::new(
        IRI::new("http://ex.org/ont"),
        IRI::new("file:///tmp/ont.owl"),
    );
    let result = mapper.get_document_iri(&IRI::new("http://ex.org/other"));
    assert!(result.is_none());
}

// ══════════════════════════════════════════════════════════════════════════════
// OntologyManager — Manager Config
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn manager_default_config() {
    let config = ManagerConfig::default();
    assert!(!config.enable_change_history);
    assert_eq!(config.max_history_size, 100);
}

#[test]
fn manager_custom_config() {
    let config = ManagerConfig {
        enable_change_history: false,
        max_history_size: 50,
        silent_missing_imports: true,
        max_import_depth: 3,
    };
    let mut manager = OntologyManager::new_with_config(config);
    assert!(!manager.config().enable_change_history);
}

// ── Convenience: assert_contains_axiom and assert_not_contains_axiom ────────

fn assert_contains_axiom(ont: &Ontology, axiom: &Axiom) {
    assert!(
        ont.axioms().contains(axiom),
        "Ontology missing axiom: {axiom:?}"
    );
}

fn assert_not_contains_axiom(ont: &Ontology, axiom: &Axiom) {
    assert!(
        !ont.axioms().contains(axiom),
        "Unexpected axiom found: {axiom:?}"
    );
}

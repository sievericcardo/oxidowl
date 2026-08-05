#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::test_base::TestBase;
use oxidowl::OntologyChangeListener;
use oxidowl::OntologyManager;
use oxidowl::manager::changes::OntologyChange;
use oxidowl::manager::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use std::sync::{Arc, Mutex};

type AddedVec = Arc<Mutex<Vec<Axiom>>>;

struct TrackingListener {
    added: AddedVec,
}

impl OntologyChangeListener for TrackingListener {
    fn on_changes(&self, changes: &[OntologyChange]) {
        let mut lock = self.added.lock().unwrap();
        for c in changes {
            if let OntologyChange::AddAxiom { axiom, .. } = c {
                lock.push(axiom.clone());
            }
        }
    }
}

struct RemovalTrackingListener {
    removed: Arc<Mutex<Vec<Axiom>>>,
}

impl OntologyChangeListener for RemovalTrackingListener {
    fn on_changes(&self, changes: &[OntologyChange]) {
        let mut lock = self.removed.lock().unwrap();
        for c in changes {
            if let OntologyChange::RemoveAxiom { axiom, .. } = c {
                lock.push(axiom.clone());
            }
        }
    }
}

struct CountingListener {
    count: Arc<Mutex<usize>>,
}

impl CountingListener {
    fn new(count: Arc<Mutex<usize>>) -> Self {
        Self { count }
    }

    fn get_count(&self) -> usize {
        *self.count.lock().unwrap()
    }
}

impl OntologyChangeListener for CountingListener {
    fn on_changes(&self, _changes: &[OntologyChange]) {
        *self.count.lock().unwrap() += _changes.len();
    }
}

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

// ══════════════════════════════════════════════════════════════════════════════
// Listener Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_change_listener_receives_add_axiom() {
    let mut tb = TestBase::new();
    let added: AddedVec = Arc::new(Mutex::new(vec![]));
    tb.manager.add_change_listener(Box::new(TrackingListener {
        added: added.clone(),
    }));

    let iri = IRI::new("http://ex.org/ont");
    tb.manager.create_ontology(iri.clone());
    let a = tb.df.class_ce("http://ex.org/A");
    let b = tb.df.class_ce("http://ex.org/B");
    let axiom = tb.df.sub_class_of(a, b);

    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: axiom.clone(),
    });

    let recorded = added.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(&recorded[0], &axiom);
}

#[test]
fn test_change_listener_receives_remove_axiom() {
    let mut tb = TestBase::new();
    let removed: Arc<Mutex<Vec<Axiom>>> = Arc::new(Mutex::new(vec![]));
    tb.manager
        .add_change_listener(Box::new(RemovalTrackingListener {
            removed: removed.clone(),
        }));

    let iri = IRI::new("http://ex.org/ont");
    let ont = tb.manager.create_ontology(iri.clone());
    let a = tb.df.class_ce("http://ex.org/A");
    let b = tb.df.class_ce("http://ex.org/B");
    let axiom = tb.df.sub_class_of(a.clone(), b.clone());

    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(axiom.clone());
    }

    tb.manager.apply_change(OntologyChange::RemoveAxiom {
        ontology_iri: iri.clone(),
        axiom: axiom.clone(),
    });

    let recorded = removed.lock().unwrap();
    assert_eq!(recorded.len(), 1);
    assert_eq!(&recorded[0], &axiom);
}

#[test]
fn test_per_ontology_listener() {
    let mut tb = TestBase::new();
    let added: AddedVec = Arc::new(Mutex::new(vec![]));

    let iri_a = IRI::new("http://ex.org/ontA");
    let iri_b = IRI::new("http://ex.org/ontB");
    tb.manager.create_ontology(iri_a.clone());
    tb.manager.create_ontology(iri_b.clone());

    tb.manager.add_listener_for_ontology(
        &iri_a,
        Box::new(TrackingListener {
            added: added.clone(),
        }),
    );

    let ax_b = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/X"),
        tb.df.class_ce("http://ex.org/Y"),
    );
    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri_b.clone(),
        axiom: ax_b.clone(),
    });
    assert_eq!(added.lock().unwrap().len(), 0);

    let ax_a = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/A"),
        tb.df.class_ce("http://ex.org/B"),
    );
    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri_a.clone(),
        axiom: ax_a.clone(),
    });
    assert_eq!(added.lock().unwrap().len(), 1);
}

#[test]
fn test_clear_listeners() {
    let mut tb = TestBase::new();
    let added: AddedVec = Arc::new(Mutex::new(vec![]));
    tb.manager.add_change_listener(Box::new(TrackingListener {
        added: added.clone(),
    }));

    let iri = IRI::new("http://ex.org/ont");
    tb.manager.create_ontology(iri.clone());

    let ax1 = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/A"),
        tb.df.class_ce("http://ex.org/B"),
    );
    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax1.clone(),
    });
    assert_eq!(added.lock().unwrap().len(), 1);

    tb.manager.clear_listeners();

    let ax2 = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/C"),
        tb.df.class_ce("http://ex.org/D"),
    );
    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax2,
    });
    assert_eq!(added.lock().unwrap().len(), 1);
}

// ══════════════════════════════════════════════════════════════════════════════
// Change / Broadcast Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_apply_changes_batch() {
    let mut tb = TestBase::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = tb.manager.create_ontology(iri.clone());

    let a = tb.df.class_ce("http://ex.org/A");
    let b = tb.df.class_ce("http://ex.org/B");
    let c = tb.df.class_ce("http://ex.org/C");
    let i = tb.df.named("http://ex.org/i");

    let ax1 = tb.df.sub_class_of(a.clone(), b.clone());
    let ax2 = tb.df.sub_class_of(b.clone(), c.clone());
    let ax3 = tb.df.class_assertion(a.clone(), i);

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

    let result = tb.manager.apply_changes(&changes);
    assert_eq!(result, ChangeApplied::Successfully);

    let guard = ont.read().unwrap();
    assert_contains_axiom(&guard, &ax1);
    assert_contains_axiom(&guard, &ax2);
    assert_contains_axiom(&guard, &ax3);
}

#[test]
fn test_try_apply_changes_rollback() {
    let mut tb = TestBase::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = tb.manager.create_ontology(iri.clone());

    let ax1 = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/A"),
        tb.df.class_ce("http://ex.org/B"),
    );
    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax1.clone(),
    });

    let ax2 = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/C"),
        tb.df.class_ce("http://ex.org/D"),
    );

    let changes = vec![
        OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom: ax2.clone(),
        },
        OntologyChange::AddAxiom {
            ontology_iri: IRI::new("http://ex.org/nonexistent"),
            axiom: tb.df.sub_class_of(
                tb.df.class_ce("http://ex.org/E"),
                tb.df.class_ce("http://ex.org/F"),
            ),
        },
    ];

    let result = tb.manager.try_apply_changes(&changes);
    assert_eq!(result, ChangeApplied::UnSuccessfully);

    let guard = ont.read().unwrap();
    assert_contains_axiom(&guard, &ax1);
    assert_not_contains_axiom(&guard, &ax2);
}

#[test]
fn test_broadcast_strategy_buffered() {
    let mut tb = TestBase::new();
    let count = Arc::new(Mutex::new(0usize));
    tb.manager
        .add_change_listener(Box::new(CountingListener::new(count.clone())));

    tb.manager
        .set_broadcast_strategy(ChangeBroadcastStrategy::Buffered(5));

    let iri = IRI::new("http://ex.org/ont");
    tb.manager.create_ontology(iri.clone());

    let a = tb.df.class_ce("http://ex.org/A");
    let b = tb.df.class_ce("http://ex.org/B");
    let c = tb.df.class_ce("http://ex.org/C");

    for class in [a, b, c] {
        let axiom = tb.df.sub_class_of(class, tb.df.class_ce("http://ex.org/D"));
        tb.manager.apply_change(OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom,
        });
    }

    assert_eq!(*count.lock().unwrap(), 0);

    tb.manager.flush_changes();

    assert_eq!(*count.lock().unwrap(), 3);
}

#[test]
fn test_broadcast_strategy_suppressed() {
    let mut tb = TestBase::new();
    let count = Arc::new(Mutex::new(0usize));
    tb.manager
        .add_change_listener(Box::new(CountingListener::new(count.clone())));

    tb.manager
        .set_broadcast_strategy(ChangeBroadcastStrategy::Suppressed);

    let iri = IRI::new("http://ex.org/ont");
    tb.manager.create_ontology(iri.clone());

    let axiom = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/A"),
        tb.df.class_ce("http://ex.org/B"),
    );
    tb.manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom,
    });

    assert_eq!(*count.lock().unwrap(), 0);
}

#[test]
fn test_undo_redo() {
    let config = ManagerConfig {
        enable_change_history: true,
        max_history_size: 100,
        silent_missing_imports: true,
        max_import_depth: 20,
    };
    let mut manager = OntologyManager::new_with_config(config);

    let iri = IRI::new("http://ex.org/ont");
    let ont = manager.create_ontology(iri.clone());
    let df = DF::new();
    let axiom = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );

    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: axiom.clone(),
    });

    {
        let guard = ont.read().unwrap();
        assert_contains_axiom(&guard, &axiom);
    }

    let inverted = manager.undo(1).expect("Undo should succeed");
    assert_eq!(inverted.len(), 1);
    assert!(matches!(inverted[0], OntologyChange::RemoveAxiom { .. }));

    {
        let mut guard = ont.write().unwrap();
        guard.remove_axiom(&axiom);
    }
    {
        let guard = ont.read().unwrap();
        assert_not_contains_axiom(&guard, &axiom);
    }

    let redo_changes = manager.redo(1).expect("Redo should succeed");
    assert_eq!(redo_changes.len(), 1);
    assert!(matches!(redo_changes[0], OntologyChange::AddAxiom { .. }));

    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(axiom.clone());
    }
    {
        let guard = ont.read().unwrap();
        assert_contains_axiom(&guard, &axiom);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Copy / Move Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_copy_ontology_between_managers() {
    let mut mgr1 = OntologyManager::new();
    let mut mgr2 = OntologyManager::new();
    let df = DF::new();

    let iri = IRI::new("http://ex.org/ont");
    let ont = mgr1.create_ontology(iri.clone());

    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let ax1 = df.sub_class_of(a.clone(), b.clone());
    let ax2 = df.sub_class_of(b.clone(), c.clone());
    let ax3 = df.class_assertion(a.clone(), df.named("http://ex.org/i"));

    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(ax1.clone());
        guard.add_axiom(ax2.clone());
        guard.add_axiom(ax3.clone());
    }

    let copied = mgr2
        .copy_ontology(&mgr1, &iri, None)
        .expect("Copy should succeed");

    assert!(mgr1.contains_ontology(&iri));
    assert!(mgr2.contains_ontology(&iri));

    {
        let guard = copied.read().unwrap();
        assert_contains_axiom(&guard, &ax1);
        assert_contains_axiom(&guard, &ax2);
        assert_contains_axiom(&guard, &ax3);
    }

    {
        let guard = ont.read().unwrap();
        assert_contains_axiom(&guard, &ax1);
        assert_contains_axiom(&guard, &ax2);
        assert_contains_axiom(&guard, &ax3);
    }
}

#[test]
fn test_move_ontology_between_managers() {
    let mut mgr1 = OntologyManager::new();
    let mut mgr2 = OntologyManager::new();
    let df = DF::new();

    let iri = IRI::new("http://ex.org/ont");
    let ont = mgr1.create_ontology(iri.clone());

    let ax = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );
    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(ax.clone());
    }

    let moved_ref = mgr2
        .move_ontology(&mut mgr1, &iri, None)
        .expect("Move should succeed");

    assert!(!mgr1.contains_ontology(&iri));
    assert!(mgr2.contains_ontology(&iri));

    let guard = moved_ref.read().unwrap();
    assert_contains_axiom(&guard, &ax);
}

#[test]
fn test_remove_ontology() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = manager.create_ontology(iri.clone());

    assert!(manager.contains_ontology(&iri));
    assert_eq!(manager.ontology_count(), 1);

    manager.remove_ontology(&ont).unwrap();

    assert!(!manager.contains_ontology(&iri));
    assert_eq!(manager.ontology_count(), 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// Save / Load Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_save_ontology_to_string_and_reload() {
    let mut tb = TestBase::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = tb.manager.create_ontology(iri.clone());

    let a = tb.df.class_ce("http://ex.org/A");
    let b = tb.df.class_ce("http://ex.org/B");
    let c = tb.df.class_ce("http://ex.org/C");
    let ax1 = tb.df.sub_class_of(a.clone(), b.clone());
    let ax2 = tb.df.sub_class_of(b.clone(), c.clone());
    let ax3 = tb
        .df
        .class_assertion(a.clone(), tb.df.named("http://ex.org/i"));

    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(ax1.clone());
        guard.add_axiom(ax2.clone());
        guard.add_axiom(ax3.clone());
    }

    let original = ont.read().unwrap().clone();
    assert_eq!(original.axioms().len(), 3);

    let serialized = tb
        .manager
        .save_ontology_to_string(&iri, OntologyFormat::Functional)
        .expect("Save to string should succeed");

    let mut tb2 = TestBase::new();
    let reloaded = tb2
        .load_and_get_ontology(&serialized, OntologyFormat::Functional)
        .expect("Reload should succeed");

    assert_eq!(reloaded.axioms().len(), 3);
    helpers::assertions::assert_ontologies_axiom_equal(&original, &reloaded);
}

#[test]
fn test_save_ontology_to_file() {
    let mut tb = TestBase::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = tb.manager.create_ontology(iri.clone());

    let ax = tb.df.sub_class_of(
        tb.df.class_ce("http://ex.org/A"),
        tb.df.class_ce("http://ex.org/B"),
    );
    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(ax.clone());
    }

    let original = ont.read().unwrap().clone();

    let file_path = tb.temp_dir.path().join("test_save.ofn");
    tb.manager
        .save_ontology(&iri, &file_path, OntologyFormat::Functional)
        .expect("Save to file should succeed");

    assert!(file_path.exists());

    let mut tb2 = TestBase::new();
    let content = std::fs::read_to_string(&file_path).expect("Should read saved file");
    let reloaded = tb2
        .load_and_get_ontology(&content, OntologyFormat::Functional)
        .expect("Reload should succeed");

    helpers::assertions::assert_ontologies_axiom_equal(&original, &reloaded);
}

#[test]
fn test_snapshot_and_restore() {
    let mut manager = OntologyManager::new();
    let iri = IRI::new("http://ex.org/ont");
    let ont = manager.create_ontology(iri.clone());
    let df = DF::new();

    let ax1 = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );
    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(ax1.clone());
    }

    let snapshot = manager
        .snapshot_ontology(&iri)
        .expect("Snapshot should succeed");
    assert_eq!(snapshot.axioms.len(), 1);

    let ax2 = df.sub_class_of(
        df.class_ce("http://ex.org/C"),
        df.class_ce("http://ex.org/D"),
    );
    manager.apply_change(OntologyChange::AddAxiom {
        ontology_iri: iri.clone(),
        axiom: ax2.clone(),
    });

    {
        let guard = ont.read().unwrap();
        assert_eq!(guard.axioms().len(), 2);
    }

    manager.restore_snapshot(snapshot);

    {
        let guard = ont.read().unwrap();
        assert_eq!(guard.axioms().len(), 1);
        assert_contains_axiom(&guard, &ax1);
        assert_not_contains_axiom(&guard, &ax2);
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Config Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_manager_config_history_depth() {
    let config = ManagerConfig {
        enable_change_history: true,
        max_history_size: 2,
        silent_missing_imports: true,
        max_import_depth: 20,
    };
    let mut manager = OntologyManager::new_with_config(config);
    let iri = IRI::new("http://ex.org/ont");
    manager.create_ontology(iri.clone());
    let df = DF::new();

    for i in 0..5 {
        let axiom = df.sub_class_of(
            df.class_ce(&format!("http://ex.org/C{i}")),
            df.class_ce(&format!("http://ex.org/C{}", i + 1)),
        );
        manager.apply_change(OntologyChange::AddAxiom {
            ontology_iri: iri.clone(),
            axiom,
        });
    }

    let inverted = manager.undo(5).expect("Undo should succeed");
    assert_eq!(inverted.len(), 2);
}

#[test]
fn test_silent_missing_imports_config() {
    let config = ManagerConfig {
        enable_change_history: false,
        max_history_size: 100,
        silent_missing_imports: true,
        max_import_depth: 20,
    };
    let manager = OntologyManager::new_with_config(config);

    assert!(manager.config().silent_missing_imports);
    assert_eq!(manager.config().max_history_size, 100);
    assert!(!manager.config().enable_change_history);
    assert_eq!(manager.config().max_import_depth, 20);
}

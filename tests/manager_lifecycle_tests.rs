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
use oxidowl::{LoaderConfig, MissingImportStrategy};
use std::sync::{Arc, Mutex, RwLock};

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

    // Undo now auto-applies the inverted (RemoveAxiom) change.
    let inverted = manager.undo(1).expect("Undo should succeed");
    assert_eq!(inverted.len(), 1);
    assert!(matches!(inverted[0], OntologyChange::RemoveAxiom { .. }));

    {
        let guard = ont.read().unwrap();
        assert_not_contains_axiom(&guard, &axiom);
    }

    // Redo now auto-applies the original (AddAxiom) change.
    let redo_changes = manager.redo(1).expect("Redo should succeed");
    assert_eq!(redo_changes.len(), 1);
    assert!(matches!(redo_changes[0], OntologyChange::AddAxiom { .. }));

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

/// CR-011 re-review NR-1: `copy_ontology` must rebuild the target's
/// incremental axiom index (via `Ontology::set_axioms`), not just clone the
/// public `axioms` field.  Since M1, `is_subclass_of` and
/// `get_types(direct = true)` answer from `axiom_index()` — a copy that
/// leaves the index empty under-answers subsumption and leaks supertypes
/// into the direct-type filter.
#[test]
fn test_copy_ontology_rebuilds_axiom_index() {
    use oxidowl::{Reasoner, ReasonerConfig};

    let mut mgr1 = OntologyManager::new();
    let mut mgr2 = OntologyManager::new();
    let df = DF::new();

    let iri = IRI::new("http://ex.org/ont");
    let ont = mgr1.create_ontology(iri.clone());

    // A ⊑ B ⊑ C; i : A and i : B → direct types of i must be {A} only.
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let c = df.class_ce("http://ex.org/C");
    let i = df.named("http://ex.org/i");
    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(df.sub_class_of(a.clone(), b.clone()));
        guard.add_axiom(df.sub_class_of(b.clone(), c.clone()));
        guard.add_axiom(df.class_assertion(a.clone(), i.clone()));
        guard.add_axiom(df.class_assertion(b.clone(), i.clone()));
    }

    let copied = mgr2
        .copy_ontology(&mgr1, &iri, None)
        .expect("copy should succeed");

    // The copy's index must expose the SubClassOf edges (empty before the fix).
    {
        let guard = copied.read().unwrap();
        let index = guard.axiom_index();
        assert_eq!(
            index.direct_superclasses(&a).to_vec(),
            vec![b.clone()],
            "copied ontology's axiom_index must carry A ⊑ B (CR-011 NR-1)"
        );
        assert_eq!(
            index.direct_superclasses(&b).to_vec(),
            vec![c.clone()],
            "copied ontology's axiom_index must carry B ⊑ C (CR-011 NR-1)"
        );
    }

    // Index-backed reasoner queries over the copy must still answer.
    let copied_ontology = {
        let guard = copied.read().unwrap();
        Ontology::clone(&guard)
    };
    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    reasoner
        .load_ontology(copied_ontology)
        .expect("load copied ontology");

    assert!(
        reasoner.is_subclass_of(&a, &c).expect("is_subclass_of"),
        "A ⊑ C must hold over the copied ontology (axiom_index-backed)"
    );
    assert!(
        !reasoner.is_subclass_of(&c, &a).expect("is_subclass_of"),
        "C ⊑ A must not hold over the copied ontology"
    );

    let direct = reasoner
        .get_types(&i, true)
        .expect("get_types(direct = true)");
    assert_eq!(
        direct,
        vec![a.clone()],
        "only A is a direct type of i; supertypes B/C must be filtered out \
         through the rebuilt axiom_index (CR-011 NR-1)"
    );
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

// ══════════════════════════════════════════════════════════════════════════════
// Loader — Recursive Import Resolution
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_loader_resolves_imports_recursively() {
    use oxidowl::OntologyLoader;
    use oxidowl::SimpleIRIMapper;

    let base = "http://test.org/loaderimports";
    let imported_iri = IRI::new(&format!("{base}/B"));

    // Write the imported ontology to a temp file.
    let mut tb = TestBase::new();
    let b_path = tb.temp_dir.path().join("B.ofn");
    let b_content = format!(
        "Ontology(<{imported_iri}>\n  ClassAssertion(<http://test.org/C> <http://test.org/i>)\n)\n"
    );
    std::fs::write(&b_path, &b_content).expect("write imported ontology");

    // Write the main ontology that imports B.
    let a_iri = IRI::new(&format!("{base}/A"));
    let a_path = tb.temp_dir.path().join("A.ofn");
    let a_content = format!(
        "Ontology(<{a_iri}>\n  Import(<{imported_iri}>)\n  SubClassOf(<http://test.org/A> <http://test.org/B>)\n)\n"
    );
    std::fs::write(&a_path, &a_content).expect("write main ontology");

    // Map the imported logical IRI to its document.
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    {
        let mut mgr = manager.write().unwrap();
        let doc_iri = IRI::new(&format!("file://{}", b_path.display()));
        mgr.add_iri_mapper(Box::new(SimpleIRIMapper::new(
            imported_iri.clone(),
            doc_iri,
        )));
    }

    let mut config = LoaderConfig::default();
    config.resolve_imports = true;
    let loader = OntologyLoader::new_with_config(manager.clone(), config);

    let main = loader
        .load_file(&a_path)
        .expect("load main ontology with import resolution");

    assert_eq!(
        main.read().unwrap().get_iri().cloned(),
        Some(a_iri.clone()),
        "Main ontology IRI should be preserved"
    );

    // The imported ontology must have been pulled in and registered.
    {
        let mgr = manager.read().unwrap();
        assert!(
            mgr.contains_ontology(&imported_iri),
            "Imported ontology should be registered on the manager"
        );
        assert!(
            mgr.contains_ontology(&a_iri),
            "Main ontology should be registered on the manager"
        );
        // Imports closure should include both.
        let closure = mgr.get_imports_closure(&main).expect("imports closure");
        assert_eq!(closure.len(), 2, "Closure should contain A and B");
    }
}

#[test]
fn test_loader_skips_unresolved_imports_in_silent_mode() {
    use oxidowl::OntologyLoader;

    let mut tb = TestBase::new();
    let a_iri = IRI::new("http://test.org/missingimport/A");
    let missing = IRI::new("urn:test:does-not-exist");
    let a_path = tb.temp_dir.path().join("missing.ofn");
    let a_content = format!("Ontology(<{a_iri}>\n  Import(<{missing}>)\n)\n");
    std::fs::write(&a_path, &a_content).expect("write main ontology");

    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let mut config = LoaderConfig::default();
    config.resolve_imports = true;
    config.missing_import_strategy = MissingImportStrategy::Silent;
    let loader = OntologyLoader::new_with_config(manager.clone(), config);

    // Silent mode: unresolved imports are skipped, load still succeeds.
    let main = loader.load_file(&a_path).expect("load should succeed");
    let mgr = manager.read().unwrap();
    assert!(mgr.contains_ontology(&a_iri));
    assert!(!mgr.contains_ontology(&missing));
    let _ = main;
}

#[test]
fn test_loader_errors_on_unresolved_imports_in_strict_mode() {
    use oxidowl::OntologyLoader;

    let mut tb = TestBase::new();
    let a_iri = IRI::new("http://test.org/strictmissing/A");
    let missing = IRI::new("urn:test:does-not-exist");
    let a_path = tb.temp_dir.path().join("strict.ofn");
    let a_content = format!("Ontology(<{a_iri}>\n  Import(<{missing}>)\n)\n");
    std::fs::write(&a_path, &a_content).expect("write main ontology");

    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let mut config = LoaderConfig::default();
    config.resolve_imports = true;
    config.missing_import_strategy = MissingImportStrategy::ThrowException;
    let loader = OntologyLoader::new_with_config(manager.clone(), config);

    assert!(
        loader.load_file(&a_path).is_err(),
        "Strict mode should fail on an unresolved import"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// XZ Compression Roundtrip
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_xz_save_and_load_roundtrip() {
    use oxidowl::OntologyLoader;
    use oxidowl::parsers::{load_xz, save_file_xz};

    let mut tb = TestBase::new();
    let iri = IRI::new("http://ex.org/xzont");
    let ont = tb.manager.create_ontology(iri.clone());
    let df = DF::new();

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
    let original = ont.read().unwrap().clone();

    let xz_path = tb.temp_dir.path().join("ont.ofn.xz");
    save_file_xz(&original, &xz_path, OntologyFormat::Functional).expect("save xz should succeed");
    assert!(xz_path.exists(), "xz file should exist");

    // Decompress + parse back.
    let reloaded = load_xz(&xz_path).expect("load xz should succeed");
    helpers::assertions::assert_ontologies_axiom_equal(&original, &reloaded);

    // Also load via the manager-backed loader.
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let loader = OntologyLoader::new(manager.clone());
    let loaded_ref = loader
        .load_xz(&xz_path)
        .expect("loader load_xz should succeed");
    let loaded = loaded_ref.read().unwrap().clone();
    helpers::assertions::assert_ontologies_axiom_equal(&original, &loaded);
}

#[test]
fn test_manager_save_ontology_xz_extension() {
    let mut tb = TestBase::new();
    let iri = IRI::new("http://ex.org/xzmanager");
    let ont = tb.manager.create_ontology(iri.clone());
    let df = DF::new();
    let ax = df.sub_class_of(
        df.class_ce("http://ex.org/A"),
        df.class_ce("http://ex.org/B"),
    );
    {
        let mut guard = ont.write().unwrap();
        guard.add_axiom(ax.clone());
    }
    let original = ont.read().unwrap().clone();

    let xz_path = tb.temp_dir.path().join("ont2.ofn.xz");
    tb.manager
        .save_ontology(&iri, &xz_path, OntologyFormat::Functional)
        .expect("manager save with .xz extension should succeed");

    let reloaded = oxidowl::parsers::load_xz(&xz_path).expect("reload xz");
    helpers::assertions::assert_ontologies_axiom_equal(&original, &reloaded);
}

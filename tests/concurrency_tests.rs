#[cfg(test)]
mod helpers;

use helpers::df::DF;
use oxidowl::factory::DataFactory;
use oxidowl::manager::OntologyManager;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use std::sync::{Arc, Barrier, RwLock};
use std::thread;

// ══════════════════════════════════════════════════════════════════════════════
// Helpers
// ══════════════════════════════════════════════════════════════════════════════

fn make_axiom(i: usize) -> Axiom {
    let df = DF::new();
    let a = df.class_ce(&format!("http://ex.org/A{i}"));
    let b = df.class_ce(&format!("http://ex.org/B{i}"));
    df.sub_class_of(a, b)
}

fn make_onto_with_axiom(i: usize) -> Ontology {
    let df = DF::new();
    let mut ont = df.new_ontology();
    ont.set_iri(IRI::new(&format!("http://ex.org/ont_{i}")));
    ont.add_axiom(make_axiom(i));
    ont
}

// ══════════════════════════════════════════════════════════════════════════════
// Concurrent Manager Access
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn test_concurrent_manager_access() {
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for i in 0..num_threads {
        let mgr = manager.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            let iri = IRI::new(&format!("http://ex.org/thread{i}"));
            bar.wait();
            {
                let mut m = mgr.write().unwrap();
                let ont_ref = m.create_ontology(iri.clone());
                assert!(m.contains_ontology(&iri));
                drop(ont_ref);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let guard = manager.read().unwrap();
    assert!(guard.ontology_count() >= num_threads);
    for i in 0..num_threads {
        let iri = IRI::new(&format!("http://ex.org/thread{i}"));
        assert!(guard.contains_ontology(&iri));
    }
}

#[test]
fn test_concurrent_ontology_mutation() {
    let iri = IRI::new("http://ex.org/shared");
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let ont_ref = {
        let mut m = manager.write().unwrap();
        m.create_ontology(iri.clone())
    };

    let num_threads = 4;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for i in 0..num_threads {
        let ont = ont_ref.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            let axiom = make_axiom(i);
            bar.wait();
            let mut guard = ont.write().unwrap();
            guard.add_axiom(axiom);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let guard = ont_ref.read().unwrap();
    let axiom_count = guard.axioms().len();
    assert!(axiom_count >= num_threads);
}

#[test]
fn test_concurrent_read_access() {
    let df = DF::new();
    let mut ont = df.new_ontology();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    ont.add_axiom(df.sub_class_of(a, b));

    let ont_ref = Arc::new(RwLock::new(ont));
    let num_threads = 8;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for _ in 0..num_threads {
        let ont = ont_ref.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            bar.wait();
            let guard = ont.read().unwrap();
            let axioms = guard.axioms();
            assert!(!axioms.is_empty());
            drop(guard);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_manager_thread_safety() {
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let num_threads = 6;
    let barrier = Arc::new(Barrier::new(num_threads));
    let iri_a = IRI::new("http://ex.org/managed");

    // First create the shared ontology
    {
        let mut m = manager.write().unwrap();
        m.create_ontology(iri_a.clone());
    }

    let mut handles = vec![];
    for i in 0..num_threads {
        let mgr = manager.clone();
        let bar = barrier.clone();
        let iri = iri_a.clone();
        handles.push(thread::spawn(move || {
            bar.wait();
            // Even threads read, odd threads add axiom
            if i % 2 == 0 {
                let m = mgr.read().unwrap();
                assert!(m.contains_ontology(&iri));
            } else {
                let m = mgr.read().unwrap();
                if let Some(ont_ref) = m.get_ontology(&iri) {
                    let mut guard = ont_ref.write().unwrap();
                    let ax = make_axiom(i);
                    guard.add_axiom(ax);
                }
            }
            drop(mgr);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_data_factory_thread_safety() {
    let factory = Arc::new(DataFactory::new());
    let num_threads = 8;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for i in 0..num_threads {
        let df = factory.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            bar.wait();
            let iri = IRI::new(&format!("http://ex.org/class{i}"));
            let class = df.get_class(&iri);
            assert_eq!(class.iri, iri);

            let op = df.get_object_property(&IRI::new(&format!("http://ex.org/prop{i}")));
            let dp = df.get_data_property(&IRI::new(&format!("http://ex.org/dprop{i}")));
            let ni = df.get_named_individual(&IRI::new(&format!("http://ex.org/ind{i}")));

            let ce = ClassExpression::Class(class);
            let ope = ObjectPropertyExpression::ObjectProperty(op);
            let ind = Individual::Named(ni);
            let ax = df.get_sub_class_of_axiom(
                ce,
                ClassExpression::Class(df.get_class(&IRI::new(&format!("http://ex.org/super{i}")))),
            );

            let axiom = Axiom::SubClassOf(ax);
            let _ = axiom.axiom_id();
            drop(dp);
            drop(ind);
            drop(ope);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

#[test]
fn test_ontology_ref_shared_access() {
    let df = DF::new();

    // Pre-create all axioms before sharing
    let axioms: Vec<Axiom> = (0..10)
        .map(|i| {
            let a = df.class_ce(&format!("http://ex.org/A{i}"));
            let b = df.class_ce(&format!("http://ex.org/B{i}"));
            df.sub_class_of(a, b)
        })
        .collect();

    let mut ont = df.new_ontology();
    ont.set_iri(IRI::new("http://ex.org/shared_ref"));
    for ax in &axioms {
        ont.add_axiom(ax.clone());
    }
    df.auto_declare(&mut ont);

    let ont_ref = Arc::new(RwLock::new(ont));
    let num_threads = 6;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for i in 0..num_threads {
        let ont = ont_ref.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            bar.wait();
            let guard = ont.read().unwrap();
            let all_axioms = guard.axioms();
            let logical_count = all_axioms.iter().filter(|a| a.is_logical()).count();
            assert!(logical_count >= 10, "Thread {i}: expected >=10 logical axioms, got {logical_count}");
            drop(guard);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let guard = ont_ref.read().unwrap();
    assert!(guard.axioms().len() >= 10);
}

#[test]
fn test_concurrent_manager_create_and_drop() {
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let num_threads = 6;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for i in 0..num_threads {
        let mgr = manager.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            bar.wait();
            let iri = IRI::new(&format!("http://ex.org/create_drop/ont{i}"));
            {
                let mut m = mgr.write().unwrap();
                let ont_ref = m.create_ontology(iri.clone());
                assert!(m.contains_ontology(&iri));
                drop(ont_ref);
            }
            // Give other threads a chance to run
            thread::yield_now();
            {
                let m = mgr.read().unwrap();
                assert!(m.contains_ontology(&iri));
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let guard = manager.read().unwrap();
    assert_eq!(guard.ontology_count(), num_threads);
}

#[test]
fn test_concurrent_axiom_addition_and_count() {
    let ont_ref = Arc::new(RwLock::new(Ontology::new()));
    let num_threads = 8;
    let barrier = Arc::new(Barrier::new(num_threads));

    let mut handles = vec![];
    for i in 0..num_threads {
        let ont = ont_ref.clone();
        let bar = barrier.clone();
        handles.push(thread::spawn(move || {
            bar.wait();
            let ax = make_axiom(i);
            let mut guard = ont.write().unwrap();
            guard.add_axiom(ax);
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    let guard = ont_ref.read().unwrap();
    assert_eq!(guard.axioms().len(), num_threads);
}

#[test]
fn test_concurrent_ontology_ref_send_sync() {
    let ont = Arc::new(RwLock::new(Ontology::new()));
    let ont_clone = ont.clone();

    let handle = thread::spawn(move || {
        let guard = ont_clone.read().unwrap();
        assert!(guard.axioms().is_empty());
        drop(guard);
    });

    handle.join().unwrap();
    let guard = ont.read().unwrap();
    assert!(guard.axioms().is_empty());
}

#[test]
fn test_concurrent_shared_iri_access() {
    let shared_iri = Arc::new(IRI::new("http://ex.org/concurrent/shared"));
    let num_threads = 4;

    let mut handles = vec![];
    for _ in 0..num_threads {
        let iri = shared_iri.clone();
        handles.push(thread::spawn(move || {
            let class = Class {
                iri: iri.as_ref().clone(),
            };
            let ce = ClassExpression::Class(class);
            let _ = ce;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

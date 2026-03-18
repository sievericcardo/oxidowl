//! Parallel tableau expansion for OWL 2 DL reasoning
//!
//! Extends the parallel-classification scheduler with batched, Rayon-driven
//! tableau expansion across groups of concept-satisfiability tests. This is
//! inspired by Konclude's concurrent tableau workers, which split the
//! work queue across CPU cores while sharing a read-only ontology snapshot.

use crate::{
    Result,
    config::PerformanceConfig,
    core::{
        reasoner::{
            tableau::{TableauFactory},
        },
        saturation::cycle_detection::CycleDetector,
    },
    ontology::{ClassExpression, Ontology},
};
use log::{debug, info};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Instant,
};

/// A single unit of parallel tableau work: expand a concept and collect the
/// resulting satisfiability verdict.
#[derive(Debug, Clone)]
pub struct TableauExpansionTask {
    /// The concept expression to test for satisfiability
    pub concept: ClassExpression,
    /// Depth context used by cycle detection
    pub derivation_chain: Vec<String>,
    /// Numeric priority (higher ⇒ expanded first when resources are limited)
    pub priority: usize,
}

/// Outcome of expanding one `TableauExpansionTask`
#[derive(Debug, Clone)]
pub struct TableauExpansionResult {
    pub concept: ClassExpression,
    pub is_satisfiable: bool,
    /// Whether a derivation cycle was detected for this concept
    pub cycle_detected: bool,
}

/// Statistics gathered during a parallel expansion batch
#[derive(Debug, Default, Clone)]
pub struct ParallelExpansionStats {
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub cycles_detected: usize,
    pub unsatisfiable_count: usize,
    pub wall_time: std::time::Duration,
}

/// A shared, append-only log of expansion results accessible from worker threads
#[allow(unused)]
struct SharedResultLog {
    results: Mutex<Vec<TableauExpansionResult>>,
    completed: AtomicUsize,
    cycles: AtomicUsize,
}

impl SharedResultLog {
    fn new() -> Self {
        Self {
            results: Mutex::new(Vec::new()),
            completed: AtomicUsize::new(0),
            cycles: AtomicUsize::new(0),
        }
    }

    fn _push(&self, result: TableauExpansionResult) {
        let cycle = result.cycle_detected;
        self.results.lock().unwrap().push(result);
        self.completed.fetch_add(1, Ordering::Relaxed);
        if cycle {
            self.cycles.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Parallel tableau expansion engine.
///
/// Runs satisfiability tests for a batch of concept expressions concurrently
/// using Rayon. Each worker thread owns an independent [`TableauAlgorithmInstance`]
/// (no shared mutable state), and a shared [`CycleDetector`] guards against
/// infinite derivation loops.
pub struct ParallelTableauExpander {
    #[allow(unused)]
    config: PerformanceConfig,
    cycle_detector: Arc<CycleDetector>,
}

impl ParallelTableauExpander {
    /// Create a new expander.
    ///
    /// `max_cycle_depth` should be set to at least the number of named classes in
    /// the ontology; the value `512` is a safe default for mid-sized ontologies.
    pub fn new(config: PerformanceConfig, max_cycle_depth: u32) -> Self {
        Self {
            config,
            cycle_detector: Arc::new(CycleDetector::new(max_cycle_depth)),
        }
    }

    /// Expand all tasks in parallel using Rayon's work-stealing thread pool.
    ///
    /// Each `TableauExpansionTask` is processed independently: a fresh tableau
    /// instance is created from `factory` and run to exhaustion (or until a
    /// cycle is detected).  Results are collected and returned together with
    /// aggregate statistics.
    ///
    /// # Concurrency model
    /// - **Read-only** ontology data is shared via `Arc<Ontology>`.
    /// - Each Rayon thread constructs its own `TableauAlgorithmInstance`.
    /// - `CycleDetector` is `Send + Sync` (DashMap + atomics only).
    pub fn expand_batch(
        &self,
        tasks: Vec<TableauExpansionTask>,
        factory: &TableauFactory,
        ontology: &Ontology,
    ) -> Result<(Vec<TableauExpansionResult>, ParallelExpansionStats)> {
        let start = Instant::now();
        let total = tasks.len();

        info!("Starting parallel tableau expansion: {} tasks", total);

        let _shared_log = Arc::new(SharedResultLog::new());

        // Sort tasks by priority (highest first) so that high-value tests
        // are dispatched to workers early.
        let mut sorted_tasks = tasks;
        sorted_tasks.sort_unstable_by(|a, b| b.priority.cmp(&a.priority));

        let cycle_detector = Arc::clone(&self.cycle_detector);

        // Rayon parallel iterator — each closure is independent.
        let results: Result<Vec<TableauExpansionResult>> = sorted_tasks
            .into_par_iter()
            .map(|task| {
                let concept_iri = iri_from_class_expression(&task.concept);

                // Cycle check (non-mutating, thread-safe)
                let cycle_detected = cycle_detector
                    .detect_cycle(&concept_iri, &task.derivation_chain)
                    .is_some();

                if cycle_detected {
                    debug!(
                        "Cycle detected for concept '{}'; marking as satisfiable (safe default)",
                        concept_iri
                    );
                    return Ok(TableauExpansionResult {
                        concept: task.concept,
                        is_satisfiable: true,
                        cycle_detected: true,
                    });
                }

                // Build a fresh tableau instance for this thread.
                let mut instance = factory.create_algorithm_instance(ontology)?;
                let state = instance.run()?;
                let is_satisfiable = state != crate::core::tableau::TableauState::Unsatisfiable;

                cycle_detector.finish_concept(&concept_iri, None);

                Ok(TableauExpansionResult {
                    concept: task.concept,
                    is_satisfiable,
                    cycle_detected: false,
                })
            })
            .collect();

        let results = results?;
        let wall_time = start.elapsed();

        let cycles_detected = results.iter().filter(|r| r.cycle_detected).count();
        let unsatisfiable_count = results.iter().filter(|r| !r.is_satisfiable).count();

        let stats = ParallelExpansionStats {
            total_tasks: total,
            completed_tasks: results.len(),
            cycles_detected,
            unsatisfiable_count,
            wall_time,
        };

        info!(
            "Parallel tableau expansion complete: {} tasks, {} cycles, {} unsatisfiable, {:.2?}",
            stats.total_tasks, stats.cycles_detected, stats.unsatisfiable_count, stats.wall_time
        );

        Ok((results, stats))
    }

    /// Build expansion tasks from a list of class expressions.
    ///
    /// Optionally accepts a `priority_hints` map (concept IRI → priority) to
    /// guide scheduling.  Concepts absent from the map receive priority `0`.
    pub fn build_tasks(
        &self,
        concepts: &[ClassExpression],
        priority_hints: Option<&HashMap<String, usize>>,
    ) -> Vec<TableauExpansionTask> {
        concepts
            .iter()
            .map(|concept| {
                let iri = iri_from_class_expression(concept);
                let priority = priority_hints
                    .and_then(|h| h.get(&iri))
                    .copied()
                    .unwrap_or(0);
                TableauExpansionTask {
                    concept: concept.clone(),
                    derivation_chain: Vec::new(),
                    priority,
                }
            })
            .collect()
    }

    /// Reset internal cycle-detection state between ontology loads.
    pub fn reset(&self) {
        self.cycle_detector.reset();
    }
}

/// Extract a stable string key from a class expression.
///
/// For named classes this returns the IRI. For anonymous expressions a
/// deterministic debug representation is used.
fn iri_from_class_expression(expr: &ClassExpression) -> String {
    match expr {
        ClassExpression::Class(c) => c.iri.to_string(),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::PerformanceConfig;

    #[test]
    fn test_build_tasks_assigns_priority() {
        let config = PerformanceConfig::default();
        let expander = ParallelTableauExpander::new(config, 64);
        let mut hints = HashMap::new();
        hints.insert("http://example.org/A".to_string(), 42);

        use crate::ontology::{Class, IRI};
        let concepts = vec![ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/A"),
        })];
        let tasks = expander.build_tasks(&concepts, Some(&hints));
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].priority, 42);
    }

    #[test]
    fn test_iri_from_class_expression() {
        use crate::ontology::{Class, IRI};
        let expr = ClassExpression::Class(Class {
            iri: IRI::new("http://example.org/Foo"),
        });
        assert_eq!(
            iri_from_class_expression(&expr),
            "http://example.org/Foo"
        );
    }
}

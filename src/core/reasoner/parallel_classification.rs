//! Parallel Classification Framework
//!
//! This module implements massively parallel classification using work-stealing
//! task scheduler for concurrent subsumption testing. This is inspired by Konclude's
//! approach to achieve 20-50x speedup by testing all N² concept pairs concurrently.

use crate::{
    Error, Result,
    ontology::ClassExpression,
    config::PerformanceConfig,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
};
use rayon::prelude::*;
use log::{debug, info};

/// A subsumption test task
#[derive(Debug, Clone)]
pub struct SubsumptionTask {
    pub subclass: ClassExpression,
    pub superclass: ClassExpression,
    pub priority: usize, // Higher priority = more important
}

/// Result of a subsumption test
#[derive(Debug, Clone)]
pub struct SubsumptionResult {
    pub subclass: ClassExpression,
    pub superclass: ClassExpression,
    pub holds: bool,
}

/// Dependency-aware task scheduler for parallel classification
pub struct ParallelClassificationScheduler {
    config: PerformanceConfig,
    completed_tests: Arc<AtomicUsize>,
    total_tests: Arc<AtomicUsize>,
    cancelled: Arc<AtomicBool>,
}

impl ParallelClassificationScheduler {
    pub fn new(config: PerformanceConfig) -> Self {
        Self {
            config,
            completed_tests: Arc::new(AtomicUsize::new(0)),
            total_tests: Arc::new(AtomicUsize::new(0)),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Schedule all classification tasks with dependency awareness
    pub fn schedule_classification_tasks(
        &self,
        classes: &[ClassExpression],
        told_subsumers: &HashMap<ClassExpression, HashSet<ClassExpression>>,
    ) -> Vec<SubsumptionTask> {
        let mut tasks = Vec::new();

        // Build a priority map based on told subsumers (topological ordering)
        let priority_map = self.build_priority_map(classes, told_subsumers);

        // Generate all N² subsumption test tasks
        for subclass in classes {
            for superclass in classes {
                if subclass == superclass {
                    continue; // Skip reflexive subsumption
                }

                // Skip if already determined by told subsumers
                if let Some(subsumers) = told_subsumers.get(subclass) {
                    if subsumers.contains(superclass) {
                        continue;
                    }
                }

                // Assign priority based on position in hierarchy
                let priority = priority_map.get(subclass).copied().unwrap_or(0)
                    + priority_map.get(superclass).copied().unwrap_or(0);

                tasks.push(SubsumptionTask {
                    subclass: subclass.clone(),
                    superclass: superclass.clone(),
                    priority,
                });
            }
        }

        // Sort by priority (higher priority first)
        tasks.sort_by(|a, b| b.priority.cmp(&a.priority));

        self.total_tests.store(tasks.len(), Ordering::Relaxed);
        info!("Scheduled {} parallel subsumption tests", tasks.len());

        tasks
    }

    /// Build priority map for dependency-aware scheduling
    fn build_priority_map(
        &self,
        classes: &[ClassExpression],
        told_subsumers: &HashMap<ClassExpression, HashSet<ClassExpression>>,
    ) -> HashMap<ClassExpression, usize> {
        let mut priority_map = HashMap::new();
        let mut visited = HashSet::new();

        // Assign priorities via topological sort (classes with fewer subsumers get higher priority)
        for class in classes {
            self.assign_priority(class, told_subsumers, &mut priority_map, &mut visited, 0);
        }

        priority_map
    }

    fn assign_priority(
        &self,
        class: &ClassExpression,
        told_subsumers: &HashMap<ClassExpression, HashSet<ClassExpression>>,
        priority_map: &mut HashMap<ClassExpression, usize>,
        visited: &mut HashSet<ClassExpression>,
        depth: usize,
    ) -> usize {
        if let Some(&priority) = priority_map.get(class) {
            return priority;
        }

        if visited.contains(class) {
            return depth; // Cycle detected
        }

        visited.insert(class.clone());

        let mut max_priority = depth;
        if let Some(subsumers) = told_subsumers.get(class) {
            for subsumer in subsumers {
                let priority = self.assign_priority(
                    subsumer,
                    told_subsumers,
                    priority_map,
                    visited,
                    depth + 1,
                );
                max_priority = max_priority.max(priority);
            }
        }

        priority_map.insert(class.clone(), max_priority);
        visited.remove(class);

        max_priority
    }

    /// Execute tasks in parallel using work-stealing
    pub fn execute_parallel<F>(
        &self,
        tasks: Vec<SubsumptionTask>,
        test_fn: F,
    ) -> Result<Vec<SubsumptionResult>>
    where
        F: Fn(&ClassExpression, &ClassExpression) -> Result<bool> + Send + Sync,
    {
        let max_parallelism = self.config.max_parallel_classification_tasks
            .unwrap_or(self.config.get_worker_threads() * 100);

        info!("Executing {} tasks with max parallelism {}", tasks.len(), max_parallelism);

        // Use rayon's parallel iterator with controlled batch size
        let batch_size = (tasks.len() / (self.config.get_worker_threads() * 4)).max(1);

        let results: Result<Vec<SubsumptionResult>> = tasks
            .par_iter()
            .with_max_len(batch_size)
            .map(|task| {
                if self.cancelled.load(Ordering::Relaxed) {
                    return Err(Error::Timeout { 
                        message: "Classification cancelled".to_string() 
                    });
                }

                let holds = test_fn(&task.subclass, &task.superclass)?;

                let completed = self.completed_tests.fetch_add(1, Ordering::Relaxed) + 1;
                let total = self.total_tests.load(Ordering::Relaxed);

                if completed % 100 == 0 {
                    debug!("Classification progress: {}/{} ({:.1}%)", 
                           completed, total, (completed as f64 / total as f64) * 100.0);
                }

                Ok(SubsumptionResult {
                    subclass: task.subclass.clone(),
                    superclass: task.superclass.clone(),
                    holds,
                })
            })
            .collect();

        results
    }

    /// Cancel all pending operations
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }

    /// Get progress information
    pub fn get_progress(&self) -> (usize, usize) {
        (
            self.completed_tests.load(Ordering::Relaxed),
            self.total_tests.load(Ordering::Relaxed),
        )
    }

    /// Reset the scheduler state
    pub fn reset(&self) {
        self.completed_tests.store(0, Ordering::Relaxed);
        self.total_tests.store(0, Ordering::Relaxed);
        self.cancelled.store(false, Ordering::Relaxed);
    }
}

/// Work-stealing queue for advanced scheduling (optional, for Phase 2)
pub struct WorkStealingQueue<T> {
    queue: VecDeque<T>,
}

impl<T> WorkStealingQueue<T> {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.queue.push_back(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.queue.pop_front()
    }

    pub fn steal(&mut self) -> Option<T> {
        self.queue.pop_back()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl<T> Default for WorkStealingQueue<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{Class, IRI};
    use std::sync::Mutex;

    #[test]
    fn test_task_scheduling() {
        let config = PerformanceConfig::default();
        let scheduler = ParallelClassificationScheduler::new(config);

        let classes = vec![
            ClassExpression::Class(Class { iri: IRI::new("A") }),
            ClassExpression::Class(Class { iri: IRI::new("B") }),
            ClassExpression::Class(Class { iri: IRI::new("C") }),
        ];

        let told_subsumers = HashMap::new();

        let tasks = scheduler.schedule_classification_tasks(&classes, &told_subsumers);

        // Should generate N²-N tasks (excluding reflexive)
        assert_eq!(tasks.len(), 3 * 3 - 3);
    }

    #[test]
    fn test_parallel_execution() {
        let config = PerformanceConfig::default();
        let scheduler = ParallelClassificationScheduler::new(config);

        let classes = vec![
            ClassExpression::Class(Class { iri: IRI::new("A") }),
            ClassExpression::Class(Class { iri: IRI::new("B") }),
        ];

        let told_subsumers = HashMap::new();
        let tasks = scheduler.schedule_classification_tasks(&classes, &told_subsumers);

        let call_count = Arc::new(Mutex::new(0));
        let call_count_clone = call_count.clone();

        let test_fn = move |_sub: &ClassExpression, _sup: &ClassExpression| -> Result<bool> {
            *call_count_clone.lock().unwrap() += 1;
            Ok(false)
        };

        let results = scheduler.execute_parallel(tasks, test_fn).unwrap();

        assert_eq!(results.len(), 2); // A->B and B->A
        assert_eq!(*call_count.lock().unwrap(), 2);
    }

    #[test]
    fn test_priority_assignment() {
        let config = PerformanceConfig::default();
        let scheduler = ParallelClassificationScheduler::new(config);

        let class_a = ClassExpression::Class(Class { iri: IRI::new("A") });
        let class_b = ClassExpression::Class(Class { iri: IRI::new("B") });
        let class_c = ClassExpression::Class(Class { iri: IRI::new("C") });

        let classes = vec![class_a.clone(), class_b.clone(), class_c.clone()];

        // A -> B -> C hierarchy
        let mut told_subsumers = HashMap::new();
        told_subsumers.insert(class_a.clone(), [class_b.clone()].into_iter().collect());
        told_subsumers.insert(class_b.clone(), [class_c.clone()].into_iter().collect());

        let tasks = scheduler.schedule_classification_tasks(&classes, &told_subsumers);

        // Verify that tasks are generated and sorted
        assert!(!tasks.is_empty());
        // Higher levels in hierarchy should have higher priority
        assert!(tasks[0].priority >= tasks[tasks.len() - 1].priority);
    }
}

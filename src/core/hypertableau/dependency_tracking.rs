//! Dependency Tracking for `HyperTableau`
//!
//! This module implements dependency tracking for supporting backtracking and
//! justification in the hypertableau algorithm. It tracks causal relationships
//! between derived facts and their supporting evidence.

use crate::{
    Result,
    ontology::{ClassExpression, Individual},
};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};

use serde::{Deserialize, Serialize};

/// Unique identifier for dependency sets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DependencySetId(pub usize);

/// Unique identifier for branching points  
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BranchingPointId(pub usize);

/// Unique identifier for facts/assertions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FactId(pub usize);

/// Types of dependencies that can exist
#[derive(Debug, Clone, PartialEq)]
pub enum DependencyType {
    /// Dependency from a clause application
    ClauseApplication {
        clause_id: usize,
        premise_facts: Vec<FactId>,
    },
    /// Dependency from a branching decision
    BranchingDecision {
        branch_id: BranchingPointId,
        choice_index: usize,
    },
    /// Dependency from an initial assertion
    InitialAssertion { axiom_id: usize },
    /// Dependency from a blocking operation
    Blocking {
        blocker_node: usize,
        blocked_node: usize,
    },
    /// Dependency from unfolding a concept
    ConceptUnfolding {
        concept: ClassExpression,
        individual: Individual,
    },
}

/// A dependency set tracks the causal history of a derived fact
#[derive(Debug, Clone)]
pub struct DependencySet {
    /// Unique identifier for this dependency set
    pub id: DependencySetId,

    /// The type of dependency
    pub dependency_type: DependencyType,

    /// Parent dependency sets that this one depends on
    pub parents: Vec<DependencySetId>,

    /// Level in the dependency hierarchy (for optimization)
    pub level: usize,

    /// Whether this dependency set is currently active
    pub is_active: bool,

    /// Timestamp when this dependency was created
    pub timestamp: std::time::Instant,
}

impl DependencySet {
    /// Create a new dependency set
    #[must_use]
    pub fn new(
        id: DependencySetId,
        dependency_type: DependencyType,
        parents: Vec<DependencySetId>,
    ) -> Self {
        let level = usize::from(!parents.is_empty()); // Will be updated by tracker

        Self {
            id,
            dependency_type,
            parents,
            level,
            is_active: true,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Check if this dependency set is independent (no parents)
    #[must_use]
    pub fn is_independent(&self) -> bool {
        self.parents.is_empty()
    }

    /// Check if this dependency set depends on a specific branching point
    #[must_use]
    pub fn depends_on_branch(&self, branch_id: BranchingPointId) -> bool {
        matches!(&self.dependency_type,
            DependencyType::BranchingDecision { branch_id: bid, .. } if *bid == branch_id)
    }
}

/// Information about a derived fact and its dependencies
#[derive(Debug, Clone)]
pub struct FactDependency {
    /// The fact identifier
    pub fact_id: FactId,

    /// Dependency set explaining how this fact was derived
    pub dependency_set: DependencySetId,

    /// The actual fact content (for debugging/justification)
    pub fact_description: String,

    /// Whether this fact is currently asserted
    pub is_asserted: bool,
}

/// Tracks dependencies between facts for backtracking and justification
#[derive(Debug)]
pub struct DependencyTracker {
    /// All dependency sets indexed by ID
    dependency_sets: HashMap<DependencySetId, DependencySet>,

    /// Facts and their dependency information
    fact_dependencies: HashMap<FactId, FactDependency>,

    /// Reverse mapping: dependency set -> facts it supports
    dependencies_to_facts: HashMap<DependencySetId, HashSet<FactId>>,

    /// Current active branching points
    active_branches: HashSet<BranchingPointId>,

    /// Facts that depend on each branching point
    branch_dependencies: HashMap<BranchingPointId, HashSet<FactId>>,

    /// Counter for generating unique IDs
    next_dependency_id: usize,
    next_fact_id: usize,

    /// Statistics
    stats: DependencyStats,
}

/// Statistics for dependency tracking
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct DependencyStats {
    pub total_dependency_sets: usize,
    pub total_facts: usize,
    pub max_dependency_level: usize,
    pub backtrack_count: usize,
    pub justification_queries: usize,
}

impl DependencyTracker {
    /// Create a new dependency tracker
    #[must_use]
    pub fn new() -> Self {
        Self {
            dependency_sets: HashMap::new(),
            fact_dependencies: HashMap::new(),
            dependencies_to_facts: HashMap::new(),
            active_branches: HashSet::new(),
            branch_dependencies: HashMap::new(),
            next_dependency_id: 1,
            next_fact_id: 1,
            stats: DependencyStats::default(),
        }
    }

    /// Create a new fact with dependency information
    pub fn create_fact(
        &mut self,
        description: String,
        dependency_type: DependencyType,
    ) -> Result<FactId> {
        let fact_id = FactId(self.next_fact_id);
        self.next_fact_id += 1;

        // Create dependency set
        let dep_id = DependencySetId(self.next_dependency_id);
        self.next_dependency_id += 1;

        let parents = self.extract_parent_dependencies(&dependency_type)?;
        let mut dependency_set = DependencySet::new(dep_id, dependency_type.clone(), parents);

        // Calculate level
        dependency_set.level = self.calculate_dependency_level(&dependency_set.parents);
        self.stats.max_dependency_level = self.stats.max_dependency_level.max(dependency_set.level);

        // Store dependency set
        self.dependency_sets.insert(dep_id, dependency_set);

        // Create fact dependency
        let fact_dependency = FactDependency {
            fact_id,
            dependency_set: dep_id,
            fact_description: description,
            is_asserted: true,
        };

        // Store mappings
        self.fact_dependencies.insert(fact_id, fact_dependency);
        self.dependencies_to_facts
            .entry(dep_id)
            .or_default()
            .insert(fact_id);

        // Track branch dependencies
        if let DependencyType::BranchingDecision { branch_id, .. } = &dependency_type {
            self.branch_dependencies
                .entry(*branch_id)
                .or_default()
                .insert(fact_id);
            self.active_branches.insert(*branch_id);
        }

        self.stats.total_facts += 1;
        self.stats.total_dependency_sets += 1;

        Ok(fact_id)
    }

    /// Extract parent dependencies from a dependency type
    fn extract_parent_dependencies(
        &self,
        dep_type: &DependencyType,
    ) -> Result<Vec<DependencySetId>> {
        match dep_type {
            DependencyType::ClauseApplication { premise_facts, .. } => {
                let mut parents = Vec::new();
                for fact_id in premise_facts {
                    if let Some(fact_dep) = self.fact_dependencies.get(fact_id) {
                        parents.push(fact_dep.dependency_set);
                    }
                }
                Ok(parents)
            }
            DependencyType::BranchingDecision { .. } => Ok(vec![]),
            DependencyType::InitialAssertion { .. } => Ok(vec![]),
            DependencyType::Blocking { .. } => Ok(vec![]),
            DependencyType::ConceptUnfolding { .. } => Ok(vec![]),
        }
    }

    /// Calculate the dependency level (maximum distance from independent facts)
    fn calculate_dependency_level(&self, parents: &[DependencySetId]) -> usize {
        if parents.is_empty() {
            return 0;
        }

        parents
            .iter()
            .filter_map(|parent_id| self.dependency_sets.get(parent_id))
            .map(|parent| parent.level)
            .max()
            .unwrap_or(0)
            + 1
    }

    /// Backtrack by removing facts that depend on a specific branching point
    pub fn backtrack_branch(&mut self, branch_id: BranchingPointId) -> Result<Vec<FactId>> {
        let mut retracted_facts = Vec::new();

        // Find all facts that depend on this branch
        if let Some(dependent_facts) = self.branch_dependencies.get(&branch_id).cloned() {
            for fact_id in dependent_facts {
                if let Some(fact_dep) = self.fact_dependencies.get_mut(&fact_id) {
                    if fact_dep.is_asserted {
                        fact_dep.is_asserted = false;
                        retracted_facts.push(fact_id);
                    }
                }
            }
        }

        // Remove branch from active set
        self.active_branches.remove(&branch_id);
        self.stats.backtrack_count += 1;

        Ok(retracted_facts)
    }

    /// Get justification for a fact (trace back its dependencies)
    pub fn get_justification(&mut self, fact_id: FactId) -> Result<Vec<DependencySetId>> {
        self.stats.justification_queries += 1;

        let mut justification = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        if let Some(fact_dep) = self.fact_dependencies.get(&fact_id) {
            queue.push_back(fact_dep.dependency_set);
        }

        while let Some(dep_id) = queue.pop_front() {
            if visited.contains(&dep_id) {
                continue;
            }
            visited.insert(dep_id);

            if let Some(dep_set) = self.dependency_sets.get(&dep_id) {
                justification.push(dep_id);

                // Add parents to queue
                for parent_id in &dep_set.parents {
                    if !visited.contains(parent_id) {
                        queue.push_back(*parent_id);
                    }
                }
            }
        }

        Ok(justification)
    }

    /// Check if a fact is currently asserted (not retracted)
    #[must_use]
    pub fn is_fact_asserted(&self, fact_id: FactId) -> bool {
        self.fact_dependencies
            .get(&fact_id)
            .is_some_and(|dep| dep.is_asserted)
    }

    /// Get all currently asserted facts
    #[must_use]
    pub fn get_asserted_facts(&self) -> Vec<FactId> {
        self.fact_dependencies
            .values()
            .filter(|dep| dep.is_asserted)
            .map(|dep| dep.fact_id)
            .collect()
    }

    /// Get dependency information for a fact
    #[must_use]
    pub fn get_fact_dependency(&self, fact_id: FactId) -> Option<&FactDependency> {
        self.fact_dependencies.get(&fact_id)
    }

    /// Get dependency set information
    #[must_use]
    pub fn get_dependency_set(&self, dep_id: DependencySetId) -> Option<&DependencySet> {
        self.dependency_sets.get(&dep_id)
    }

    /// Check if there are any active branches
    #[must_use]
    pub fn has_active_branches(&self) -> bool {
        !self.active_branches.is_empty()
    }

    /// Get statistics
    #[must_use]
    pub fn get_stats(&self) -> &DependencyStats {
        &self.stats
    }

    /// Clean up retracted facts and unused dependency sets
    pub fn garbage_collect(&mut self) -> Result<usize> {
        let mut removed_count = 0;

        // Find dependency sets that support only retracted facts
        let mut unused_deps = HashSet::new();

        for (dep_id, fact_ids) in &self.dependencies_to_facts {
            let all_retracted = fact_ids.iter().all(|fact_id| {
                self.fact_dependencies
                    .get(fact_id)
                    .is_none_or(|dep| !dep.is_asserted)
            });

            if all_retracted {
                unused_deps.insert(*dep_id);
            }
        }

        // Remove unused dependency sets (but keep independent ones for potential reuse)
        for dep_id in unused_deps {
            if let Some(dep_set) = self.dependency_sets.get(&dep_id) {
                if !dep_set.is_independent() {
                    self.dependency_sets.remove(&dep_id);
                    self.dependencies_to_facts.remove(&dep_id);
                    removed_count += 1;
                }
            }
        }

        Ok(removed_count)
    }

    /// Reset the tracker (clear all data)
    pub fn reset(&mut self) {
        self.dependency_sets.clear();
        self.fact_dependencies.clear();
        self.dependencies_to_facts.clear();
        self.active_branches.clear();
        self.branch_dependencies.clear();
        self.next_dependency_id = 1;
        self.next_fact_id = 1;
        self.stats = DependencyStats::default();
    }
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for working with dependencies
pub mod utils {
    use super::{BranchingPointId, DependencyType, FactId};

    /// Create a dependency type for clause application
    #[must_use]
    pub fn clause_application_dependency(
        clause_id: usize,
        premise_facts: Vec<FactId>,
    ) -> DependencyType {
        DependencyType::ClauseApplication {
            clause_id,
            premise_facts,
        }
    }

    /// Create a dependency type for branching decision
    #[must_use]
    pub fn branching_dependency(
        branch_id: BranchingPointId,
        choice_index: usize,
    ) -> DependencyType {
        DependencyType::BranchingDecision {
            branch_id,
            choice_index,
        }
    }

    /// Create a dependency type for initial assertion
    #[must_use]
    pub fn initial_assertion_dependency(axiom_id: usize) -> DependencyType {
        DependencyType::InitialAssertion { axiom_id }
    }
}

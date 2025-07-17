//! Dependency tracking and management
//!
//! This module provides dependency tracking capabilities for backtracking
//! and maintaining the reasoning dependency graph.

use crate::{Error, Result};
use std::{
    collections::{HashMap, HashSet, BTreeSet},
    fmt,
    hash::{Hash, Hasher},
    sync::{Arc, Weak},
};

/// Identifier for the dependency nodes
pub type DependencyId = u64;

/// Identifier for branching points in the dependency graph
pub type BranchingPoint = u64;

/// Dependency set tracking concept derivations and branching dependencies
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencySet {
    /// Branching points the dependency set is associated with
    branching_points: BTreeSet<BranchingPoint>,

    /// Deterministic dependencies
    deterministic_deps: HashSet<DependencyId>,

    /// Non-deterministic dependencies
    nondeterministic_deps: HashSet<DependencyId>,

    /// Reference count for the dependency set
    ref_count: usize,
}

/// Dependency node representing a reasoning step or choice point
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Unique identifier for the dependency node
    id: DependencyId,

    /// Type of dependency node (e.g., concept, role, data property)
    node_type: DependencyType,

    /// Dependencies that led to this node
    dependencies: DependencySet,

    /// Nodes that depend on this node
    dependents: HashSet<DependencyId>,

    /// Branching point this node is associated with
    branching_point: Option<BranchingPoint>,

    /// Status of the dependency node (active, inactive, etc.)
    status: DependencyStatus,
}

/// Type of dependency node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Deterministic dependency (e.g. AND, SOME)
    Deterministic {
        rule: String,
        source_concept: String,
    },

    /// Non-deterministic dependency (e.g. OR, cardinality choice)
    NonDeterministic {
        rule: String,
        choices: Vec<String>,
        chosen_index: Option<usize>,
    },

    /// Merging operation (e.g. merging two branches)
    Merge {
        source_node: String,
        target_node: String,
    },

    /// Concept implication
    Implication {
        antecedent: String,
        consequent: String,
    },

    /// Functionality restriction
    Functional {
        role: String,
        individual: String,
    },

    /// Distinctness constraint
    Distinct {
        individuals: Vec<String>,
    },

    /// Nominal handling
    Nominal {
        nominal: String,
        individual: String,
    },

    /// Expanded existential
    Expanded {
        existential: String,
        witness: String,
    },
    
    /// Datatype constraint
    Datatype {
        constraint: String,
        value: String,
    },
}

/// Status of the dependency node
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyStatus {
    /// Active dependency node
    Active,

    /// Inactive dependency node (e.g. backtracked)
    Backtracked,

    /// Suspended dependency node (part of a merge operation)
    Suspended,

    /// Removed dependency node (e.g. due to inconsistency)
    Removed,
}

/// Dependency tracker managing the dependency graph and operations
#[derive(Debug)]
pub struct DependencyTracker {
    /// Nodes
    nodes: HashMap<DependencyId, DependencyNode>,

    /// Next available dependency ID
    next_id: DependencyId,

    /// Current branching point
    current_branching_level: BranchingPoint,

    /// Stack of branching points
    branching_stack: Vec<BranchingPoint>,

    /// Dependency sets for efficient memory management
    set_factory: DependencySetFactory,

    /// Active dependencies at each branching point
    active_dependencies: HashMap<BranchingPoint, HashSet<DependencyId>>,
}

/// Factory for creating and managing dependency sets\
#[derive(Debug)]
pub struct DependencySetFactory {
    /// Cache of dependency sets
    set_cache: HashMap<DependencyId, Arc<DependencySet>>,

    /// Empty dependency set singleton
    empty_set: Arc<DependencySet>,

    /// Usage counters for garbage collection
    usage_counters: HashMap<DependencySetKey, usize>,
}

/// Key for identifying dependency sets in the factory
#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencySetKey {
    branching_points: BTreeSet<BranchingPoint>,
    deterministic_deps: HashSet<DependencyId>,
    nondeterministic_deps: HashSet<DependencyId>,
}

impl std::hash::Hash for DependencySetKey {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.branching_points.hash(state);
        self.deterministic_deps.hash(state);
        self.nondeterministic_deps.hash(state);
    }
}

/// Track point for dependency management
#[derive(Debug, Clone)]
pub struct DependencyTrackPoint {
    /// Branching point identifier
    branching_point: BranchingPoint,

    /// Dependencies at this point
    active_dependencies: HashSet<DependencyId>,

    /// Timestamp for ordering
    timestamp: std::time::Instant,
}

impl DependencySet {
    /// Create an empty dependency set
    pub fn new() -> Self {
        Self {
            branching_points: BTreeSet::new(),
            deterministic_deps: HashSet::new(),
            nondeterministic_deps: HashSet::new(),
            ref_count: 0,
        }
    }

    /// Create a dependency set with a single branching point
    pub fn with_branching_point(branching_point: BranchingPoint) -> Self {
        let mut set = Self::new();
        set.branching_points.insert(branching_point);
        set
    }

    /// Create a dependency set with a single dependency
    pub fn with_dependency(dep_id: DependencyId, is_deterministic: bool) -> Self {
        let mut set = Self::new();
        if is_deterministic {
            set.deterministic_deps.insert(dep_id);
        } else {
            set.nondeterministic_deps.insert(dep_id);
        }
        set
    }

    /// Create a singleton dependency set (alias for with_dependency)
    pub fn singleton(source: String) -> Self {
        // For now, just create an empty set - this is a placeholder
        // TODO: Implement proper singleton creation based on source
        Self::empty()
    }

    /// Union of two dependency sets
    pub fn union(&self, other: &DependencySet) -> Self {
        Self {
            branching_points: self.branching_points.union(&other.branching_points).cloned().collect(),
            deterministic_deps: self.deterministic_deps.union(&other.deterministic_deps).cloned().collect(),
            nondeterministic_deps: self.nondeterministic_deps.union(&other.nondeterministic_deps).cloned().collect(),
            ref_count: 0, // Ref count is managed externally
        }
    }

    /// Add a branching point to the dependency set
    pub fn add_branching_point(&mut self, branching_point: BranchingPoint) {
        self.branching_points.insert(branching_point);
    }

    /// Add a dependency to the set
    pub fn add_dependency(&mut self, dep_id: DependencyId, is_deterministic: bool) {
        if is_deterministic {
            self.deterministic_deps.insert(dep_id);
        } else {
            self.nondeterministic_deps.insert(dep_id);
        }
    }

    /// Check if the set is empty
    pub fn is_empty(&self) -> bool {
        self.branching_points.is_empty()
            && self.deterministic_deps.is_empty()
            && self.nondeterministic_deps.is_empty()
    }

    /// Get all branching points
    pub fn branching_points(&self) -> &BTreeSet<BranchingPoint> {
        &self.branching_points
    }

    /// Get maximum branching point
    pub fn max_branching_point(&self) -> Option<BranchingPoint> {
        self.branching_points.iter().max().copied()
    }

    /// Check if the set is valid at a given branching point
    pub fn is_valid_at(&self, branching_point: BranchingPoint) -> bool {
        self.branching_points.iter().all(|&bp| bp <= branching_point)
    }

    /// Check if the set conflicts with another set at a given branching point
    pub fn conflicts_with(&self, other: &DependencySet, branching_point: BranchingPoint) -> bool {
        self.branching_points.contains(&branch_point) && other.branching_points.contains(&branch_point)
    }
}

impl DependencyNode {
    /// Create a new dependency node
    pub fn new(id: DependencyId, node_type: DependencyType) -> Self {
        Self {
            id,
            node_type,
            dependencies: DependencySet::new(),
            dependents: HashSet::new(),
            branching_point: None,
            status: DependencyStatus::Active,
        }
    }

    /// Get the ID of the dependency node
    pub fn id(&self) -> DependencyId {
        self.id
    }

    /// Get the type of the dependency node
    pub fn node_type(&self) -> &DependencyType {
        &self.node_type
    }

    /// Get the dependencies of this node
    pub fn dependencies(&self) -> &DependencySet {
        &self.dependencies
    }

    /// Add a dependency to this node
    pub fn add_dependency(&mut self, dep_set: DependencySet) {
        self.dependencies = self.dependencies.union(&dep_set);
    }

    /// Get the dependents of this node
    pub fn dependents(&self) -> &HashSet<DependencyId> {
        &self.dependents
    }

    /// Add a dependent node
    pub fn add_dependent(&mut self, dep_id: DependencyId) {
        self.dependents.insert(dep_id);
    }

    /// Remove a dependent node
    pub fn remove_dependent(&mut self, dep_id: DependencyId) {
        self.dependents.remove(&dep_id);
    }

    /// Get the status of the dependency node
    pub fn status(&self) -> &DependencyStatus {
        &self.status
    }

    /// Set the status of the dependency node
    pub fn set_status(&mut self, status: DependencyStatus) {
        self.status = status;
    }

    /// Set the branching point for this node
    pub fn set_branching_point(&mut self, branching_point: BranchingPoint) {
        self.branching_point = Some(branching_point);
    }

    /// Check if the node is active
    pub fn is_active(&self) -> bool {
        matches!(self.status, DependencyStatus::Active)
    }
}

impl DependencyTracker {
    /// Create a new dependency tracker
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            next_id: 0,
            current_branching_level: 0,
            branching_stack: Vec::new(),
            set_factory: DependencySetFactory::new(),
            active_dependencies: HashMap::new(),
        }
    }

    /// Create a new dependency node
    pub fn create_dependency(&mut self, node_type: DependencyType) -> DependencyId {
        let id = self.next_id;
        self.next_id += 1;

        let node = DependencyNode::new(id, node_type);
        self.nodes.insert(id, node);

        // Add to current branching point
        self.level_dependencies
            .entry(self.current_branching_level)
            .or_insert_with(HashSet::new)
            .insert(id);

        id
    }

    /// Get a dependency node by ID
    pub fn get_dependency(&self, id: DependencyId) -> Option<&DependencyNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable reference to a dependency node by ID
    pub fn get_dependency_mut(&mut self, id: DependencyId) -> Option<&mut DependencyNode> {
        self.nodes.get_mut(&id)
    }

    /// Add a dependency to a node
    pub fn add_dependency(&mut self, dependent: DependencyId, dependency: DependencyId) -> Result<()> {
        if self.would_create_cycle(dependent, dependency) {
            return Err(Error::internal("Dependency would create cycle"));
        }

        if let Some(dep_node) = self.nodes.get_mut(&dependent) {
            dep_node.add_dependent(dependent);
        }

        if let Some(dependant_node) = self.nodes.get_mut(&dependency) {
            let dep_set = DependencySet::with_dependency(dependency, true);
            dependant_node.add_dependency(dep_set);
        }

        Ok(())
    }

    /// Create a branching point
    pub fn create_branching_point(&mut self) -> BranchingPoint {
        self.current_branching_level += 1;
        self.branching_stack.push(self.current_branching_level);
        self.level_dependencies.insert(self.current_branching_level, HashSet::new());
        self.current_branching_level
    }

    /// Backtrack to a previous branching point
    pub fn backtrack_to(&mut self, branching_point: BranchingPoint) -> Result<()> {
        if !self.branching_stack.contains(&branching_point) {
            return Err(Error::internal("Invalid branching point for backtrack"));
        }
        if branching_point > self.current_branching_level {
            return Err(Error::internal("Cannot backtrack to a future branching point"));
        }

        // Make all dependencies at levels greater than the target inactive
        for level in (branching_point + 1)..=self.current_branching_level {
            if let Some(deps) = self.active_dependencies.get(&level) {
                for &dep_id in deps {
                    if let Some(node) = self.nodes.get_mut(&dep_id) {
                        node.set_status(DependencyStatus::Backtracked);
                    }
                }
            }
            self.active_dependencies.remove(&level);
        }

        // Update current branching level
        self.current_branching_level = branching_point;
        while self.branching_stack.len() > 1 && self.branching_stack.last().unwrap() > &branching_point {
            self.branching_stack.pop();
        }
        
        Ok(())
    }

    /// Get current branching level
    pub fn current_branching_level(&self) -> BranchingPoint {
        self.current_branching_level
    }

    /// Create a dependency set
    pub fn create_dependency_set(
        &mut self,
        branching_points: Vec<BranchingPoint>,
        dependencies: Vec<(DependenciesId, bool)>,
    ) -> Arc<DependencySet>
    {
        self.set_factory.create_set(branching_points, dependencies)
    }

    /// Get empty dependency set
    pub fn empty_set(&self) -> Arc<DependencySet> {
        self.set_factory.empty_set()
    }

    /// Check if a dependency would create a cycle
    fn would_create_cycle(&self, from: DependencyId, to: DependencyId) -> Result<bool> {
        if from == to {
            return Ok(true); // Self-dependency is a cycle
        }

        // Simple cycle detection logic
        let mut visited = HashSet::new();
        let mut stack = vec![from];

        while let Some(current) = stack.pop() {
            if visited.contains(&current) {
                if current == to {
                    return Ok(true); // Cycle detected
                }
                continue;
            }
            visited.insert(current);

            if current == from {
                return Ok(true);
            }

            if let Some(node) = self.nodes.get(&current) {
                for &dep in &node.dependencies.deterministic_deps {
                    stack.push(dep);
                }
                for &dep in &node.dependencies.nondeterministic_deps {
                    stack.push(dep);
                }
            }
        }

        Ok(false)
    }

    /// Get all active dependencies
    pub fn active_dependencies(&self) -> Vec<DependencyId> {
        self.nodes.iter()
            .filter(|(_, node)| node.is_active())
            .map(|(&id, _)| id)
            .collect()
    }

    /// Get dependencies at a specific branching point
    pub fn dependencies_at(&self, branching_point: BranchingPoint) -> Vec<DependencyId> {
        self.level_dependencies
            .get(&branching_point)
            .map_or_else(Vec::new, |deps| deps.iter().cloned().collect())
    }

    /// Create a track point for the current state
    pub fn create_track_point(&self) -> DependencyTrackPoint {
        DependencyTrackPoint {
            branching_point: self.current_branching_level,
            active_dependencies: self.active_dependencies.get(&self.current_branching_level).cloned().unwrap_or_default(),
            timestamp: std::time::Instant::now(),
        }
    }

    /// Check if a dependency set is consistent at a branching point
    pub fn is_consistent_at(&self, dep_set: &DependencySet, branching_point: BranchingPoint) -> bool {
        dep_set.is_valid_at(branching_point) && !dep_set.conflicts_with(&self.empty_set(), branching_point)
    }

    /// Clean up unused dependency sets
    pub fn garbage_collect(&mut self) {
        let active_deps: HashSet<_> = self.active_dependencies().into_iter().collect();
        
        self.nodes.retain(|&id, node| {
            active_deps.contains(&id) || !node.dependents.is_empty()
        });
        
        self.set_factory.garbage_collect();
    }
}

impl DependencySetFactory {
    /// Create a new factory
    pub fn new() -> Self {
        let empty_set = Arc::new(DependencySet::new());
        let mut set_cache = HashMap::new();
        let empty_key = DependencySetKey {
            branching_points: BTreeSet::new(),
            deterministic_deps: HashSet::new(),
            nondeterministic_deps: HashSet::new(),
        };
        set_cache.insert(empty_key, empty_set.clone());

        Self {
            set_cache: HashMap::new(),
            empty_set: Arc::new(DependencySet::new()),
            usage_counters: HashMap::new(),
        }
    }


    /// Get the empty dependency set
    pub fn empty_set(&self) -> Arc<DependencySet> {
        self.empty_set.clone()
    }

    /// Create a new dependency set
    pub fn create_set(
        &mut self,
        branching_points: Vec<BranchingPoint>,
        dependencies: Vec<(DependencyId, bool)>,
    ) -> Arc<DependencySet> {
        let key = DependencySetKey {
            branching_points: branching_points.into_iter().collect(),
            deterministic_deps: dependencies
                .iter()
                .filter(|(_, is_det)| *is_det)
                .map(|(id, _)| *id)
                .collect(),
            nondeterministic_deps: dependencies
                .iter()
                .filter(|(_, is_det)| !*is_det)
                .map(|(id, _)| *id)
                .collect(),
        };

        if let Some(set) = self.set_cache.get(&key) {
            *self.usage_counters.entry(key.clone()).or_insert(0) += 1;
            return set.clone();
        }

        let new_set = DependencySet {
            branching_points: key.branching_points.clone(),
            deterministic_deps: key.deterministic_deps
                .iter().cloned().collect(),
            nondeterministic_deps: key.nondeterministic_deps
                .iter().cloned().collect(),
            ref_count: 1,
        };

        let arc_set = Arc::new(new_set);
        self.set_cache.insert(key, arc_set.clone());
        self.usage_counters.insert(key, 1);

        arc_set
    }

    /// Union two dependency sets
    pub fn union_set(&mut self, set1: &Arc<DependencySet>, set2: &Arc<DependencySet>) -> Arc<DependencySet> {
        let branching_points: Vec<_> = set1.branching_points
            .union(set2.branching_points())
            .cloned()
            .collect();
        let deps: Vec<_> = set1.deterministic_deps
            .union(&set2.deterministic_deps)
            .map(|&id| (id, true))
            .chain(
                set1.nondeterministic_deps
                    .union(&set2.nondeterministic_deps)
                    .map(|&id| (id, false)),
            )
            .collect();
        self.create_set(branching_points, deps)
    }

    /// Garbage collect unused dependency sets
    pub fn garbage_collect(&mut self) {
        let threshold = 1000;
        if self.set_cache.len() < threshold {
            return; // No need to collect if under threshold
        }

        // Remove sets with low usage
        let to_remove: Vec<DependencySetKey> = self.usage_counters
            .iter()
            .filter(|(_, &count)| count < 2) // Keep sets used at least twice
            .map(|(key, _)| key.clone())
            .collect();

        for key in to_remove {
            self.set_cache.remove(&key);
            self.usage_counters.remove(&key);
        }
    }
}

impl DependencyTrackPoint {
    /// Get the branching level
    pub fn branching_level(&self) -> BranchingPoint {
        self.branching_level
    }
    
    /// Get active dependencies
    pub fn active_dependencies(&self) -> &HashSet<DependencyId> {
        &self.active_dependencies
    }
    
    /// Get timestamp
    pub fn timestamp(&self) -> std::time::Instant {
        self.timestamp
    }
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for DependencyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyType::Deterministic { rule, source_concept } => 
                write!(f, "Det[{}->{}]", rule, source_concept),
            DependencyType::NonDeterministic { rule, choices, chosen_index } => 
                write!(f, "NonDet[{}:{:?}@{:?}]", rule, choices, chosen_index),
            DependencyType::Merge { source_node, target_node } => 
                write!(f, "Merge[{}->{}]", source_node, target_node),
            DependencyType::Implication { antecedent, consequent } => 
                write!(f, "Impl[{}->{}]", antecedent, consequent),
            DependencyType::Functional { role, individual } => 
                write!(f, "Func[{}@{}]", role, individual),
            DependencyType::Distinct { individuals } => 
                write!(f, "Dist[{:?}]", individuals),
            DependencyType::Nominal { nominal, individual } => 
                write!(f, "Nom[{}@{}]", nominal, individual),
            DependencyType::Expanded { existential, witness } => 
                write!(f, "Exp[{}->{}]", existential, witness),
            DependencyType::Datatype { constraint, value } => 
                write!(f, "Data[{}@{}]", constraint, value),
        }
    }
}

impl Default for DependencySetFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl Hash for DependencySet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash the branching points
        for bp in &self.branching_points {
            bp.hash(state);
        }
        
        // Hash the deterministic dependencies (convert to sorted vec first)
        let mut det_deps: Vec<_> = self.deterministic_deps.iter().collect();
        det_deps.sort();
        for dep in det_deps {
            dep.hash(state);
        }
        
        // Hash the non-deterministic dependencies (convert to sorted vec first)
        let mut nondet_deps: Vec<_> = self.nondeterministic_deps.iter().collect();
        nondet_deps.sort();
        for dep in nondet_deps {
            dep.hash(state);
        }
        
        // Don't hash ref_count as it's not part of logical equality
    }
}
//! Branching and Backtracking for HyperTableau
//!
//! This module implements branching points and backtracking mechanisms for
//! handling non-deterministic disjunctions in the hypertableau algorithm.

use crate::{
    ontology::{Individual, ClassExpression},
    Error, Result,
};

use super::{
    dependency_tracking::{DependencyTracker, BranchingPointId},
    ground_disjunction::{GroundDisjunction, DisjunctPredicate},
};

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use serde::{Serialize, Deserialize};

/// Different types of branching decisions
#[derive(Debug, Clone, PartialEq)]
pub enum BranchingType {
    /// Branching on a ground disjunction
    GroundDisjunction {
        disjunction: GroundDisjunction,
        individual: Individual,
    },
    /// Branching on an existential restriction
    ExistentialRestriction {
        property: String,
        filler: ClassExpression,
        individual: Individual,
    },
    /// Branching on a universal restriction clash
    UniversalRestriction {
        property: String,
        filler: ClassExpression,
        individual: Individual,
    },
    /// Branching on number restrictions
    NumberRestriction {
        property: String,
        cardinality: usize,
        filler: Option<ClassExpression>,
        individual: Individual,
    },
}

/// Represents a choice within a branching point
#[derive(Debug, Clone)]
pub struct BranchingChoice {
    /// Index of this choice within the branching point
    pub choice_index: usize,
    
    /// Description of what this choice represents
    pub description: String,
    
    /// The assertion that would be made if this choice is selected
    pub assertion: ClassExpression,
    
    /// The individual this assertion applies to
    pub individual: Individual,
    
    /// Whether this choice has been explored
    pub explored: bool,
    
    /// Whether this choice led to a clash
    pub caused_clash: bool,
    
    /// Cost estimate for this choice (for heuristics)
    pub cost_estimate: f64,
}

impl BranchingChoice {
    /// Create a new branching choice
    pub fn new(
        choice_index: usize,
        description: String,
        assertion: ClassExpression,
        individual: Individual,
    ) -> Self {
        Self {
            choice_index,
            description,
            assertion,
            individual,
            explored: false,
            caused_clash: false,
            cost_estimate: 0.0,
        }
    }
    
    /// Mark this choice as explored
    pub fn mark_explored(&mut self) {
        self.explored = true;
    }
    
    /// Mark this choice as causing a clash
    pub fn mark_clash(&mut self) {
        self.caused_clash = true;
    }
    
    /// Set cost estimate for this choice
    pub fn set_cost_estimate(&mut self, cost: f64) {
        self.cost_estimate = cost;
    }
    
    /// Check if this choice is viable (not explored or caused clash)
    pub fn is_viable(&self) -> bool {
        !self.explored || !self.caused_clash
    }
}

/// A branching point in the search space
#[derive(Debug, Clone)]
pub struct BranchingPoint {
    /// Unique identifier for this branching point
    pub id: BranchingPointId,
    
    /// Type of branching
    pub branching_type: BranchingType,
    
    /// Available choices at this branching point
    pub choices: Vec<BranchingChoice>,
    
    /// Currently selected choice (if any)
    pub current_choice: Option<usize>,
    
    /// Level in the search tree
    pub level: usize,
    
    /// Parent branching point (if any)
    pub parent: Option<BranchingPointId>,
    
    /// Child branching points
    pub children: Vec<BranchingPointId>,
    
    /// Whether this branching point is currently active
    pub is_active: bool,
    
    /// Timestamp when this branching point was created
    pub timestamp: std::time::Instant,
    
    /// Priority for this branching point (higher = more important)
    pub priority: f64,
}

impl BranchingPoint {
    /// Create a new branching point
    pub fn new(
        id: BranchingPointId,
        branching_type: BranchingType,
        choices: Vec<BranchingChoice>,
    ) -> Self {
        Self {
            id,
            branching_type,
            choices,
            current_choice: None,
            level: 0,
            parent: None,
            children: Vec::new(),
            is_active: true,
            timestamp: std::time::Instant::now(),
            priority: 0.0,
        }
    }
    
    /// Get the next unexplored choice
    pub fn get_next_choice(&self) -> Option<usize> {
        self.choices
            .iter()
            .enumerate()
            .find(|(_, choice)| !choice.explored)
            .map(|(index, _)| index)
    }
    
    /// Get all viable (unexplored and non-clashing) choices
    pub fn get_viable_choices(&self) -> Vec<usize> {
        self.choices
            .iter()
            .enumerate()
            .filter_map(|(index, choice)| {
                if choice.is_viable() {
                    Some(index)
                } else {
                    None
                }
            })
            .collect()
    }
    
    /// Check if all choices have been explored
    pub fn is_exhausted(&self) -> bool {
        self.choices.iter().all(|choice| choice.explored)
    }
    
    /// Select a specific choice
    pub fn select_choice(&mut self, choice_index: usize) -> Result<()> {
        if choice_index >= self.choices.len() {
            return Err(Error::invalid_branching_choice(choice_index));
        }
        
        self.current_choice = Some(choice_index);
        self.choices[choice_index].mark_explored();
        Ok(())
    }
    
    /// Get the currently selected choice
    pub fn get_current_choice(&self) -> Option<&BranchingChoice> {
        self.current_choice
            .and_then(|index| self.choices.get(index))
    }
    
    /// Add a child branching point
    pub fn add_child(&mut self, child_id: BranchingPointId) {
        self.children.push(child_id);
    }
    
    /// Set parent branching point and level
    pub fn set_parent(&mut self, parent_id: BranchingPointId, level: usize) {
        self.parent = Some(parent_id);
        self.level = level;
    }
    
    /// Calculate priority based on heuristics
    pub fn calculate_priority(&mut self) {
        // Priority heuristics:
        // 1. Fewer choices = higher priority (fail faster)
        // 2. Lower cost estimates = higher priority
        // 3. Certain types get preference
        
        let choice_factor = 1.0 / (self.choices.len() as f64 + 1.0);
        let cost_factor = 1.0 / (self.get_average_cost() + 1.0);
        
        let type_factor = match &self.branching_type {
            BranchingType::GroundDisjunction { .. } => 1.0,
            BranchingType::ExistentialRestriction { .. } => 0.8,
            BranchingType::UniversalRestriction { .. } => 0.9,
            BranchingType::NumberRestriction { .. } => 0.7,
        };
        
        self.priority = choice_factor * cost_factor * type_factor;
    }
    
    /// Get average cost estimate of all choices
    fn get_average_cost(&self) -> f64 {
        if self.choices.is_empty() {
            return 0.0;
        }
        
        let total_cost: f64 = self.choices.iter().map(|c| c.cost_estimate).sum();
        total_cost / self.choices.len() as f64
    }
}

/// Strategies for selecting branching points
#[derive(Debug, Clone, PartialEq)]
pub enum BranchingStrategy {
    /// Depth-first search
    DepthFirst,
    /// Breadth-first search
    BreadthFirst,
    /// Best-first search based on priority
    BestFirst,
    /// Custom strategy with user-defined priority function
    Custom,
}

/// Statistics for branching operations
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BranchingStats {
    pub total_branching_points: usize,
    pub total_choices_explored: usize,
    pub total_backtracks: usize,
    pub max_depth: usize,
    pub average_branching_factor: f64,
    pub clash_count: usize,
    pub successful_branches: usize,
}

/// Manages branching points and backtracking in the hypertableau
#[derive(Debug)]
pub struct BranchingManager {
    /// All branching points indexed by ID
    branching_points: HashMap<BranchingPointId, BranchingPoint>,
    
    /// Stack of active branching points (for backtracking)
    branching_stack: Vec<BranchingPointId>,
    
    /// Current path in the search tree
    current_path: Vec<BranchingPointId>,
    
    /// Branching strategy to use
    strategy: BranchingStrategy,
    
    /// Queue for breadth-first or best-first search
    choice_queue: VecDeque<BranchingPointId>,
    
    /// Counter for generating unique IDs
    next_branching_id: usize,
    
    /// Reference to dependency tracker for backtracking
    dependency_tracker: Arc<Mutex<DependencyTracker>>,
    
    /// Statistics
    stats: BranchingStats,
    
    /// Maximum depth to prevent infinite search
    max_depth: Option<usize>,
}

impl BranchingManager {
    /// Create a new branching manager
    pub fn new(
        strategy: BranchingStrategy,
        dependency_tracker: Arc<Mutex<DependencyTracker>>,
    ) -> Self {
        Self {
            branching_points: HashMap::new(),
            branching_stack: Vec::new(),
            current_path: Vec::new(),
            strategy,
            choice_queue: VecDeque::new(),
            next_branching_id: 1,
            dependency_tracker,
            stats: BranchingStats::default(),
            max_depth: Some(1000), // Default depth limit
        }
    }
    
    /// Create a new branching point
    pub fn create_branching_point(
        &mut self,
        branching_type: BranchingType,
        choices: Vec<BranchingChoice>,
    ) -> Result<BranchingPointId> {
        let id = BranchingPointId(self.next_branching_id);
        self.next_branching_id += 1;
        
        let mut branching_point = BranchingPoint::new(id, branching_type, choices);
        
        // Set parent and level
        if let Some(&parent_id) = self.current_path.last() {
            let parent_level = self.branching_points
                .get(&parent_id)
                .map(|p| p.level)
                .unwrap_or(0);
            
            branching_point.set_parent(parent_id, parent_level + 1);
            
            // Add as child to parent
            if let Some(parent) = self.branching_points.get_mut(&parent_id) {
                parent.add_child(id);
            }
        }
        
        // Check depth limit
        if let Some(max_depth) = self.max_depth {
            if branching_point.level >= max_depth {
                return Err(Error::max_depth_exceeded(branching_point.level));
            }
        }
        
        // Calculate priority
        branching_point.calculate_priority();
        
        // Update statistics
        self.stats.total_branching_points += 1;
        self.stats.max_depth = self.stats.max_depth.max(branching_point.level);
        
        // Store branching point
        self.branching_points.insert(id, branching_point);
        
        // Add to appropriate data structure based on strategy
        match self.strategy {
            BranchingStrategy::DepthFirst => {
                self.branching_stack.push(id);
            }
            BranchingStrategy::BreadthFirst => {
                self.choice_queue.push_back(id);
            }
            BranchingStrategy::BestFirst => {
                // Insert in priority order
                let priority = self.branching_points[&id].priority;
                let insert_pos = self.choice_queue
                    .iter()
                    .position(|&other_id| {
                        self.branching_points[&other_id].priority < priority
                    })
                    .unwrap_or(self.choice_queue.len());
                self.choice_queue.insert(insert_pos, id);
            }
            BranchingStrategy::Custom => {
                self.choice_queue.push_back(id);
            }
        }
        
        Ok(id)
    }
    
    /// Get the next branching point to explore
    pub fn get_next_branching_point(&mut self) -> Option<BranchingPointId> {
        match self.strategy {
            BranchingStrategy::DepthFirst => {
                self.branching_stack.pop()
            }
            BranchingStrategy::BreadthFirst | 
            BranchingStrategy::BestFirst | 
            BranchingStrategy::Custom => {
                self.choice_queue.pop_front()
            }
        }
    }
    
    /// Make a choice at a branching point
    pub fn make_choice(
        &mut self,
        branching_id: BranchingPointId,
        choice_index: Option<usize>,
    ) -> Result<Option<(ClassExpression, Individual)>> {
        let branching_point = self.branching_points
            .get_mut(&branching_id)
            .ok_or(Error::branching_point_not_found(format!("BranchingPoint({})", branching_id.0)))?;
        
        // If no specific choice provided, get the next available one
        let choice_index = choice_index
            .or_else(|| branching_point.get_next_choice())
            .ok_or(Error::no_branching_choices_available())?;
        
        // Select the choice
        branching_point.select_choice(choice_index)?;
        
        // Add to current path
        self.current_path.push(branching_id);
        
        // Update statistics
        self.stats.total_choices_explored += 1;
        
        // Get the assertion to make
        let choice = branching_point.get_current_choice().unwrap();
        Ok(Some((choice.assertion.clone(), choice.individual.clone())))
    }
    
    /// Backtrack from the current branching point
    pub fn backtrack(&mut self) -> Result<Option<BranchingPointId>> {
        if self.current_path.is_empty() {
            return Ok(None);
        }
        
        // Get current branching point
        let current_id = self.current_path.pop().unwrap();
        
        // Mark current choice as causing clash if needed
        if let Some(branching_point) = self.branching_points.get_mut(&current_id) {
            if let Some(choice_index) = branching_point.current_choice {
                branching_point.choices[choice_index].mark_clash();
                self.stats.clash_count += 1;
            }
        }
        
        // Backtrack dependencies
        {
            let mut tracker = self.dependency_tracker.lock().unwrap();
            tracker.backtrack_branch(current_id)?;
        }
        
        // Check if current branching point has more choices
        let branching_point = &self.branching_points[&current_id];
        if !branching_point.is_exhausted() {
            // Add back to queue/stack for future exploration
            match self.strategy {
                BranchingStrategy::DepthFirst => {
                    self.branching_stack.push(current_id);
                }
                BranchingStrategy::BreadthFirst | 
                BranchingStrategy::BestFirst | 
                BranchingStrategy::Custom => {
                    self.choice_queue.push_back(current_id);
                }
            }
        }
        
        self.stats.total_backtracks += 1;
        
        // Return the parent to continue from
        Ok(self.current_path.last().copied())
    }
    
    /// Mark current branch as successful
    pub fn mark_success(&mut self) {
        self.stats.successful_branches += 1;
    }
    
    /// Get information about a branching point
    pub fn get_branching_point(&self, id: BranchingPointId) -> Option<&BranchingPoint> {
        self.branching_points.get(&id)
    }
    
    /// Get current path in the search tree
    pub fn get_current_path(&self) -> &[BranchingPointId] {
        &self.current_path
    }
    
    /// Check if there are any more branching points to explore
    pub fn has_more_choices(&self) -> bool {
        match self.strategy {
            BranchingStrategy::DepthFirst => !self.branching_stack.is_empty(),
            _ => !self.choice_queue.is_empty(),
        }
    }
    
    /// Get statistics
    pub fn get_stats(&self) -> &BranchingStats {
        &self.stats
    }
    
    /// Set maximum search depth
    pub fn set_max_depth(&mut self, max_depth: Option<usize>) {
        self.max_depth = max_depth;
    }
    
    /// Calculate average branching factor
    pub fn calculate_average_branching_factor(&mut self) {
        if self.branching_points.is_empty() {
            self.stats.average_branching_factor = 0.0;
            return;
        }
        
        let total_choices: usize = self.branching_points
            .values()
            .map(|bp| bp.choices.len())
            .sum();
        
        self.stats.average_branching_factor = 
            total_choices as f64 / self.branching_points.len() as f64;
    }
    
    /// Reset the branching manager
    pub fn reset(&mut self) {
        self.branching_points.clear();
        self.branching_stack.clear();
        self.current_path.clear();
        self.choice_queue.clear();
        self.next_branching_id = 1;
        self.stats = BranchingStats::default();
    }
}

/// Helper functions for creating branching points
pub mod utils {
    use super::*;
    
    /// Create choices for a ground disjunction
    pub fn create_disjunction_choices(
        disjunction: &GroundDisjunction,
        individual: &Individual,
    ) -> Vec<BranchingChoice> {
        disjunction.disjuncts()
            .iter()
            .enumerate()
            .filter_map(|(index, disjunct)| {
                // Extract ClassExpression from DisjunctPredicate
                match disjunct {
                    DisjunctPredicate::Concept { concept, .. } => {
                        Some(BranchingChoice::new(
                            index,
                            format!("Disjunct {}: {}", index, disjunct),
                            concept.clone(),
                            individual.clone(),
                        ))
                    }
                    _ => {
                        // For non-concept predicates, create a placeholder choice
                        // TODO: handle other predicate types properly
                        None
                    }
                }
            })
            .collect()
    }
    
    /// Create choices for an existential restriction
    pub fn create_existential_choices(
        property: &str,
        filler: &ClassExpression,
        individual: &Individual,
    ) -> Vec<BranchingChoice> {
        // For existential restrictions, we typically create one choice
        // to assert the existence of a witness individual
        vec![BranchingChoice::new(
            0,
            format!("Create witness for ∃{}.{}", property, filler),
            filler.clone(),
            individual.clone(),
        )]
    }
    
    /// Estimate cost for a branching choice based on complexity
    pub fn estimate_choice_cost(assertion: &ClassExpression) -> f64 {
        match assertion {
            ClassExpression::Class(_) => 1.0,
            ClassExpression::ObjectIntersectionOf(classes) => {
                classes.iter().map(estimate_choice_cost).sum::<f64>() + 1.0
            }
            ClassExpression::ObjectUnionOf(classes) => {
                classes.iter().map(estimate_choice_cost).sum::<f64>() + 2.0
            }
            ClassExpression::ObjectComplementOf(class) => {
                estimate_choice_cost(class) + 1.5
            }
            ClassExpression::ObjectSomeValuesFrom { property: _, filler } => {
                estimate_choice_cost(filler) + 3.0
            }
            ClassExpression::ObjectAllValuesFrom { property: _, filler } => {
                estimate_choice_cost(filler) + 2.0
            }
            _ => 5.0, // Default high cost for complex expressions
        }
    }
}
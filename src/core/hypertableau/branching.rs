//! Branching and Backtracking for HyperTableau
//!
//! This module implements branching points and backtracking mechanisms for
//! handling non-deterministic disjunctions in the hypertableau algorithm.

use crate::{
    ontology::{Individual, ClassExpression, Axiom},
    Error, Result,
};

use super::{
    dependency_tracking::{DependencyTracker, BranchingPointId, FactId},
    ground_disjunction::GroundDisjunction,
};

use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    fmt,
};

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
            return Err(Error::InvalidBranchingChoice(choice_index));
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
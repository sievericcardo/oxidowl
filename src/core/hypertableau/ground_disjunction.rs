//! Ground Disjunction Implementation
//!
//! This module implements ground disjunctions as used in HermiT's hypertableau algorithm.
//! Ground disjunctions represent disjunctive facts that must hold in the tableau.

use crate::{
    core::{
        dependency::DependencySet,
        hypertableau::extension_table::ExtensionManager,
    },
    ontology::{ClassExpression, ObjectProperty, ObjectPropertyExpression},
    Error, Result,
};

use std::{
    fmt,
    hash::{Hash, Hasher},
};

/// A ground disjunction represents a disjunctive clause where all variables are bound
#[derive(Debug, Clone)]
pub struct GroundDisjunction {
    /// Header containing the disjunctive structure
    header: GroundDisjunctionHeader,
    
    /// Arguments (nodes/individuals) for this ground disjunction
    arguments: Vec<usize>, // Node IDs
    
    /// Core flags for each argument (used for blocking)
    is_core: Vec<bool>,
    
    /// Dependency set for backtracking
    dependency_set: DependencySet,
    
    /// Unique identifier
    id: usize,
}

/// Header structure for ground disjunctions containing the disjunctive predicates
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GroundDisjunctionHeader {
    /// The predicates in this disjunction
    predicates: Vec<DisjunctPredicate>,
    
    /// Sorted indices for disjunct processing order
    sorted_disjunct_indices: Vec<usize>,
    
    /// Priority for processing
    priority: DisjunctionPriority,
}

/// A single predicate in a disjunction
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DisjunctPredicate {
    /// Concept assertion: C(x)
    Concept {
        concept: ClassExpression,
        argument: usize, // position in arguments array
    },
    
    /// Role assertion: R(x, y)
    Role {
        property: ObjectProperty,
        subject: usize,  // position in arguments array
        object: usize,   // position in arguments array
    },
    
    /// Equality: x = y
    Equality {
        left: usize,
        right: usize,
    },
    
    /// Inequality: x ≠ y
    Inequality {
        left: usize,
        right: usize,
    },
}

/// Priority levels for ground disjunction processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DisjunctionPriority {
    /// Highest priority - process immediately
    Critical = 0,
    /// High priority - process soon
    High = 1,
    /// Normal priority - standard processing
    Normal = 2,
    /// Low priority - process when convenient
    Low = 3,
}

impl GroundDisjunction {
    /// Create a new ground disjunction
    pub fn new(
        header: GroundDisjunctionHeader,
        arguments: Vec<usize>,
        is_core: Vec<bool>,
        dependency_set: DependencySet,
        id: usize,
    ) -> Self {
        assert_eq!(arguments.len(), is_core.len());
        
        Self {
            header,
            arguments,
            is_core,
            dependency_set,
            id,
        }
    }
    
    /// Get the number of disjuncts
    pub fn num_disjuncts(&self) -> usize {
        self.header.predicates.len()
    }
    
    /// Get the sorted disjunct indices for processing
    pub fn get_sorted_disjunct_indices(&self) -> &[usize] {
        &self.header.sorted_disjunct_indices
    }
    
    /// Get the header
    pub fn get_header(&self) -> &GroundDisjunctionHeader {
        &self.header
    }
    
    /// Get the dependency set
    pub fn get_dependency_set(&self) -> &DependencySet {
        &self.dependency_set
    }
    
    /// Get the unique ID
    pub fn get_id(&self) -> usize {
        self.id
    }
    
    /// Check if this disjunction is already satisfied
    pub fn is_satisfied(&self, extension_manager: &ExtensionManager) -> Result<bool> {
        // Check if any disjunct is already satisfied
        for (i, predicate) in self.header.predicates.iter().enumerate() {
            if self.is_disjunct_satisfied(i, predicate, extension_manager)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    
    /// Check if a specific disjunct is satisfied
    fn is_disjunct_satisfied(
        &self,
        _disjunct_index: usize,
        predicate: &DisjunctPredicate,
        extension_manager: &ExtensionManager,
    ) -> Result<bool> {
        match predicate {
            DisjunctPredicate::Concept { concept, argument } => {
                let node_id = format!("node_{}", self.arguments[*argument]);
                Ok(extension_manager.contains_concept_assertion(&node_id, concept))
            }
            DisjunctPredicate::Role { property, subject, object } => {
                let subj_id = format!("node_{}", self.arguments[*subject]);
                let obj_id = format!("node_{}", self.arguments[*object]);
                let property_expr = ObjectPropertyExpression::ObjectProperty(property.clone());
                Ok(extension_manager.contains_role_assertion(&subj_id, &property_expr, &obj_id))
            }
            DisjunctPredicate::Equality { left, right } => {
                let left_id = format!("node_{}", self.arguments[*left]);
                let right_id = format!("node_{}", self.arguments[*right]);
                Ok(extension_manager.are_nodes_equal(&left_id, &right_id))
            }
            DisjunctPredicate::Inequality { left, right } => {
                let left_id = format!("node_{}", self.arguments[*left]);
                let right_id = format!("node_{}", self.arguments[*right]);
                Ok(extension_manager.are_nodes_unequal(&left_id, &right_id))
            }
        }
    }
    
    /// Add a specific disjunct to the tableau
    pub fn add_disjunct_to_tableau(
        &self,
        disjunct_index: usize,
        extension_manager: &mut ExtensionManager,
        dependency_tracker: &DependencySet,
    ) -> Result<bool> {
        if disjunct_index >= self.header.predicates.len() {
            return Err(Error::InvalidDisjunctIndex { index: disjunct_index });
        }
        
        let predicate = &self.header.predicates[disjunct_index];
        self.add_predicate_to_tableau(predicate, extension_manager, dependency_tracker)
    }
    
    /// Add a specific predicate to the tableau
    fn add_predicate_to_tableau(
        &self,
        predicate: &DisjunctPredicate,
        extension_manager: &mut ExtensionManager,
        dependency_tracker: &DependencySet,
    ) -> Result<bool> {
        match predicate {
            DisjunctPredicate::Concept { concept, argument } => {
                let node_id = format!("node_{}", self.arguments[*argument]);
                extension_manager.add_concept_assertion_with_dependency(
                    &node_id,
                    concept,
                    dependency_tracker.clone(),
                )
            }
            DisjunctPredicate::Role { property, subject, object } => {
                let subj_id = format!("node_{}", self.arguments[*subject]);
                let obj_id = format!("node_{}", self.arguments[*object]);
                let prop_expr = ObjectPropertyExpression::ObjectProperty(property.clone());
                extension_manager.add_role_assertion_with_dependency(
                    &subj_id,
                    &prop_expr,
                    &obj_id,
                    dependency_tracker.clone(),
                )
            }
            DisjunctPredicate::Equality { left, right } => {
                let left_id = format!("node_{}", self.arguments[*left]);
                let right_id = format!("node_{}", self.arguments[*right]);
                extension_manager.add_equality_with_dependency(
                    &left_id,
                    &right_id,
                    dependency_tracker.clone(),
                )
            }
            DisjunctPredicate::Inequality { left, right } => {
                let left_id = format!("node_{}", self.arguments[*left]);
                let right_id = format!("node_{}", self.arguments[*right]);
                extension_manager.add_inequality_with_dependency(
                    &left_id,
                    &right_id,
                    dependency_tracker.clone(),
                )
            }
        }
    }
    
    /// Get the arguments for this disjunction
    pub fn get_arguments(&self) -> &[usize] {
        &self.arguments
    }
    
    /// Get the core flags
    pub fn get_core_flags(&self) -> &[bool] {
        &self.is_core
    }
    
    /// Get a specific predicate
    pub fn get_predicate(&self, index: usize) -> Option<&DisjunctPredicate> {
        self.header.predicates.get(index)
    }
    
    /// Get the priority
    pub fn get_priority(&self) -> DisjunctionPriority {
        self.header.priority
    }

    /// Get disjuncts (compatibility method)
    pub fn disjuncts(&self) -> &Vec<DisjunctPredicate> {
        &self.header.predicates
    }
    
    /// Get individual (compatibility method) - returns first argument as string
    pub fn individual(&self) -> String {
        // For compatibility, return the first argument as a string representation
        self.arguments.first().map(|&id| format!("node_{id}")).unwrap_or_else(|| "unknown".to_string())
    }
    
    /// Create a ground disjunction from a class expression
    pub fn from_class_expression(
        class_expr: ClassExpression,
        individual: crate::ontology::Individual,
        dependencies: DependencySet,
    ) -> Result<Self> {
        use crate::ontology::ClassExpression;
        
        let mut predicates = Vec::new();
        let arguments = vec![0]; // Single argument for the individual
        let is_core = vec![true]; // Mark as core
        
        match class_expr {
            ClassExpression::ObjectUnionOf(disjuncts) => {
                for (i, disjunct) in disjuncts.into_iter().enumerate() {
                    predicates.push(DisjunctPredicate::Concept {
                        concept: disjunct,
                        argument: 0, // All refer to the same individual
                    });
                }
            }
            // For non-union expressions, treat as single disjunct
            other => {
                predicates.push(DisjunctPredicate::Concept {
                    concept: other,
                    argument: 0,
                });
            }
        }
        
        let priority = if predicates.len() <= 2 {
            DisjunctionPriority::High
        } else {
            DisjunctionPriority::Normal
        };
        
        let header = GroundDisjunctionHeader::new(predicates, priority);
        
        Ok(GroundDisjunction::new(
            header,
            arguments,
            is_core,
            dependencies,
            crate::core::hypertableau::extension_table::generate_unique_id(),
        ))
    }
}

impl GroundDisjunctionHeader {
    /// Create a new ground disjunction header
    pub fn new(
        predicates: Vec<DisjunctPredicate>,
        priority: DisjunctionPriority,
    ) -> Self {
        // Create sorted indices based on predicate type and complexity
        let mut sorted_indices: Vec<usize> = (0..predicates.len()).collect();
        
        // Sort by predicate complexity (simpler predicates first)
        sorted_indices.sort_by_key(|&i| Self::predicate_complexity(&predicates[i]));
        
        Self {
            predicates,
            sorted_disjunct_indices: sorted_indices,
            priority,
        }
    }

    /// Create a new ground disjunction header with predicates
    pub fn new_with_predicates(
        mut predicates: Vec<DisjunctPredicate>,
        priority: DisjunctionPriority,
    ) -> Self {
        // Sort predicates by complexity (simpler first)
        predicates.sort_by_key(Self::predicate_complexity);
        
        let sorted_disjunct_indices: Vec<usize> = (0..predicates.len()).collect();
        
        Self {
            predicates,
            sorted_disjunct_indices,
            priority,
        }
    }
    
    /// Calculate the complexity of a predicate for sorting
    fn predicate_complexity(predicate: &DisjunctPredicate) -> u32 {
        match predicate {
            DisjunctPredicate::Equality { .. } => 1,  // Simplest
            DisjunctPredicate::Inequality { .. } => 2,
            DisjunctPredicate::Concept { concept, .. } => {
                // More complex concepts get higher scores
                match concept {
                    ClassExpression::Class(_) => 3,
                    ClassExpression::ObjectSomeValuesFrom { .. } => 5,
                    ClassExpression::ObjectAllValuesFrom { .. } => 6,
                    _ => 4,
                }
            }
            DisjunctPredicate::Role { .. } => 4,
        }
    }
    
    /// Get the predicates
    pub fn get_predicates(&self) -> &[DisjunctPredicate] {
        &self.predicates
    }
    
    /// Get the sorted disjunct indices
    pub fn get_sorted_disjunct_indices(&self) -> &[usize] {
        &self.sorted_disjunct_indices
    }
    
    /// Get the priority
    pub fn get_priority(&self) -> DisjunctionPriority {
        self.priority
    }
}

impl fmt::Display for GroundDisjunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "GroundDisjunction({})[", self.id)?;
        for (i, predicate) in self.header.predicates.iter().enumerate() {
            if i > 0 {
                write!(f, " ∨ ")?;
            }
            write!(f, "{predicate}")?;
        }
        write!(f, "]")
    }
}

impl fmt::Display for DisjunctPredicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DisjunctPredicate::Concept { concept, argument } => {
                write!(f, "{concept}(x{argument})")
            }
            DisjunctPredicate::Role { property, subject, object } => {
                write!(f, "{}(x{}, x{})", property.iri, subject, object)
            }
            DisjunctPredicate::Equality { left, right } => {
                write!(f, "x{left} = x{right}")
            }
            DisjunctPredicate::Inequality { left, right } => {
                write!(f, "x{left} ≠ x{right}")
            }
        }
    }
}

impl PartialEq for GroundDisjunction {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for GroundDisjunction {}

impl Hash for GroundDisjunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
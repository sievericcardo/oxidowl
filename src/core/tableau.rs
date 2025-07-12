//! Tableau algorithm implementation
//!
//! This module implements the core tableau reasoning algorithm for SROIQV(D),
//! including tableau construction, node management, and completion rules.

use crate::{
    config::ReasoningConfig,
    core::{
        blocking::{BlockingStrategy, BlockingChecker},
        expansion::{ExpansionStrategy, ExpansionManager, ExpansionContext, ExistentialCandidate, ExpansionPriority, ExpansionResult},
        completion::{CompletionRule, CompletionRuleSet, RuleContext, RuleApplication, RulePriority},
        dependency::{DependencyTracker, DependencySet},
    },
    ontology::{Ontology, ClassExpression, Individual, Axiom, ObjectProperty, ObjectPropertyExpression, IRI, Class, Role},
    Error, Result,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::{debug, trace, warn};

/// Current state of tableau expansion
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableauState {
    /// Tableau is satisfiable (open)
    Satisfiable,
    /// Tableau is unsatisfiable (closed)
    Unsatisfiable,
    /// State is unknown (timeout, resource limit, etc.)
    Unknown,
}

/// Main tableau structure for reasoning
#[derive(Debug)]
pub struct Tableau {
    /// All nodes in the tableau
    nodes: Vec<TableauNode>,
    
    /// Edges between nodes
    edges: Vec<TableauEdge>,
    
    /// Current completion rules to apply
    completion_rules: CompletionRuleSet,
    
    /// Blocking strategy
    blocking_strategy: Box<dyn BlockingChecker>,
    
    /// Expansion strategy
    expansion_strategy: Box<dyn ExpansionStrategy>,
    
    /// Dependency tracking
    dependency_tracker: DependencyTracker,
    
    /// Queue of pending rule applications
    pending_queue: VecDeque<RuleApplication>,
    
    /// Clash detection and handling
    clash_detector: ClashDetector,
    
    /// Current reasoning configuration
    config: ReasoningConfig,
    
    /// Statistics for this tableau run
    statistics: TableauStatistics,
    
    /// Current state
    state: TableauState,
    
    /// Property inclusions (SubObjectPropertyOf)
    property_inclusions: Vec<PropertyInclusion>,
    
    /// Inverse property relationships
    inverse_properties: HashMap<String, String>,
    
    /// Functional properties
    functional_properties: HashSet<String>,
    
    /// Inverse functional properties
    inverse_functional_properties: HashSet<String>,
    
    /// Transitive properties
    transitive_properties: HashSet<String>,
    
    /// Symmetric properties (handled via inverse_properties)
    /// Asymmetric properties
    asymmetric_properties: HashSet<String>,
    
    /// Reflexive properties
    reflexive_properties: HashSet<String>,
    
    /// Irreflexive properties
    irreflexive_properties: HashSet<String>,
}

/// Individual node in the tableau
#[derive(Debug, Clone)]
pub struct TableauNode {
    /// Unique node identifier
    pub id: NodeId,
    
    /// Concepts associated with this node
    pub concepts: HashSet<ConceptLabel>,
    
    /// Node type (individual, nominal, etc.)
    pub node_type: NodeType,
    
    /// Blocking information
    pub blocking_info: BlockingInfo,
    
    /// Dependency information for concepts
    pub concept_dependencies: HashMap<ConceptLabel, DependencySet>,
    
    /// Status flags
    pub status: NodeStatus,
}

/// Edge between tableau nodes
#[derive(Debug, Clone)]
pub struct TableauEdge {
    /// Source node
    pub from: NodeId,
    
    /// Target node
    pub to: NodeId,
    
    /// Role label
    pub role: RoleLabel,
    
    /// Dependency information
    pub dependencies: DependencySet,
}

/// Unique identifier for tableau nodes
pub type NodeId = usize;

/// Concept label in the tableau
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ConceptLabel {
    /// Atomic concept
    Atomic(String),
    
    /// Negated atomic concept
    NegatedAtomic(String),
    
    /// Complex concept expression
    Complex(Box<ClassExpression>),
    
    /// Existential restriction
    Existential {
        role: RoleLabel,
        filler: Box<ConceptLabel>,
    },
    
    /// Universal restriction
    Universal {
        role: RoleLabel,
        filler: Box<ConceptLabel>,
    },
    
    /// At least cardinality restriction
    AtLeast {
        cardinality: u32,
        role: RoleLabel,
        filler: Option<Box<ConceptLabel>>,
    },
    
    /// At most cardinality restriction
    AtMost {
        cardinality: u32,
        role: RoleLabel,
        filler: Option<Box<ConceptLabel>>,
    },
    
    /// Nominal (individual)
    Nominal(String),
}

/// Role label in the tableau
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoleLabel {
    /// Atomic role
    Atomic(String),
    
    /// Inverse role
    Inverse(String),
}
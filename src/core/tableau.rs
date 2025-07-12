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

impl RoleLabel {
    /// Convert a Role from the ontology to a RoleLabel
    pub fn from_role(role: &Role) -> Result<Self> {
        match role {
            Role::ObjectProperty(prop_expr) => {
                match prop_expr {
                    crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                        Ok(RoleLabel::Atomic(prop.iri.as_str().to_string()))
                    }
                    crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                        Ok(RoleLabel::Inverse(prop.iri.as_str().to_string()))
                    }
                    crate::ontology::ObjectPropertyExpression::PropertyChain(_) => {
                        Err(Error::reasoning("Property chains cannot be converted to role labels directly"))
                    }
                }
            }
            Role::DataProperty(prop) => {
                Ok(RoleLabel::Atomic(prop.iri.as_str().to_string()))
            }
        }
    }
}

/// Type of tableau node
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// Regular individual node
    Individual,
    
    /// Nominal node
    Nominal,
    
    /// Generated node from existential expansion
    Generated,
    
    /// Root node
    Root,
}

/// Blocking information for a node
#[derive(Debug, Clone, Default)]
pub struct BlockingInfo {
    /// Whether this node is blocked
    pub is_blocked: bool,
    
    /// Node that blocks this one (if any)
    pub blocker: Option<NodeId>,
    
    /// Nodes blocked by this one
    pub blocks: HashSet<NodeId>,
    
    /// Blocking signature
    pub signature: Option<Vec<ConceptLabel>>,
}

/// Status flags for tableau nodes
#[derive(Debug, Clone, Default)]
pub struct NodeStatus {
    /// Whether the node has been fully expanded
    pub fully_expanded: bool,
    
    /// Whether the node is involved in a clash
    pub clashed: bool,
    
    /// Whether the node is being processed
    pub processing: bool,
}

/// Rule application to be processed
/// Priority levels for rule applications
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    /// Highest priority (deterministic rules)
    Highest = 0,
    
    /// High priority (propagation rules)
    High = 1,
    
    /// Normal priority (expansion rules)
    Normal = 2,
    
    /// Low priority (non-deterministic rules)
    Low = 3,
    
    /// Lowest priority (optimization rules)
    Lowest = 4,
}

/// Clash detection and management
#[derive(Debug)]
pub struct ClashDetector {
    /// Currently detected clashes
    clashes: Vec<Clash>,
}

/// Representation of a clash in the tableau
#[derive(Debug, Clone)]
pub struct Clash {
    /// Type of clash
    pub clash_type: ClashType,
    
    /// Nodes involved in the clash
    pub nodes: Vec<NodeId>,
    
    /// Dependencies that led to this clash
    pub dependencies: DependencySet,
    
    /// Explanation for the clash
    pub explanation: String,
}

/// Types of clashes that can occur
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClashType {
    /// Concept clash (C and ¬C)
    Concept {
        concept: String,
        node: NodeId,
    },
    
    /// Cardinality clash
    Cardinality {
        role: RoleLabel,
        node: NodeId,
        min_cardinality: u32,
        max_cardinality: u32,
    },
    
    /// Functionality clash
    Functionality {
        role: RoleLabel,
        node: NodeId,
        individuals: Vec<NodeId>,
    },
    
    /// Nominal clash
    Nominal {
        individual: String,
        nodes: Vec<NodeId>,
    },
}

/// Statistics about tableau construction and expansion
#[derive(Debug, Default, Clone)]
pub struct TableauStatistics {
    /// Number of nodes created
    pub nodes_created: usize,
    
    /// Number of edges created
    pub edges_created: usize,
    
    /// Number of rule applications
    pub rule_applications: usize,
    
    /// Number of backtracking operations
    pub backtracking_operations: usize,
    
    /// Maximum depth reached
    pub max_depth: usize,
    
    /// Time spent in tableau construction
    pub construction_time: Duration,
    
    /// Time spent in rule application
    pub rule_application_time: Duration,
    
    /// Time spent in blocking checks
    pub blocking_time: Duration,
}
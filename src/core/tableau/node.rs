//! Tableau node management and structures
//!
//! This module contains all node-related structures and operations
//! for the tableau reasoning algorithm.

use crate::{
    Error, Result,
    core::dependency::DependencySet,
    ontology::{ClassExpression, Role},
};
use std::collections::{HashMap, HashSet};

/// Unique identifier for tableau nodes
pub type NodeId = usize;

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

    /// Role successors for this node
    pub role_successors: HashMap<String, HashSet<NodeId>>,

    /// Status flags
    pub status: NodeStatus,
}

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

    /// Intersection of concepts (conjunction)
    Intersection(Vec<ConceptLabel>),

    /// Union of concepts (disjunction)  
    Union(Vec<ConceptLabel>),
}

/// Role label in the tableau
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RoleLabel {
    /// Atomic role
    Atomic(String),

    /// Inverse role
    Inverse(String),

    /// Role chain (property chain)
    Chain(Vec<RoleLabel>),

    /// Complex role expression  
    Complex(String),
}

impl RoleLabel {
    /// Convert a Role from the ontology to a `RoleLabel`
    pub fn from_role(role: &Role) -> Result<Self> {
        match role {
            Role::ObjectProperty(prop_expr) => match prop_expr {
                crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                    Ok(RoleLabel::Atomic(prop.iri.as_str().to_string()))
                }
                crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                    Ok(RoleLabel::Inverse(prop.iri.as_str().to_string()))
                }
                crate::ontology::ObjectPropertyExpression::PropertyChain(_) => Err(
                    Error::reasoning("Property chains cannot be converted to role labels directly"),
                ),
            },
            Role::DataProperty(prop) => Ok(RoleLabel::Atomic(prop.iri.as_str().to_string())),
        }
    }

    /// Get the role name as a string
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            RoleLabel::Atomic(name) => name,
            RoleLabel::Inverse(name) => name,
            RoleLabel::Chain(_) => "chain", // Return a default name for chains
            RoleLabel::Complex(expr) => expr,
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

impl ConceptLabel {
    /// Check if this concept label is atomic
    pub fn is_atomic(&self) -> bool {
        matches!(self, ConceptLabel::Atomic(_))
    }

    /// Check if this concept label is negated
    pub fn is_negated(&self) -> bool {
        matches!(self, ConceptLabel::NegatedAtomic(_))
    }

    /// Check if this concept label is complex
    pub fn is_complex(&self) -> bool {
        matches!(self, ConceptLabel::Complex(_))
    }

    /// Get the atomic name if this is an atomic concept
    pub fn atomic_name(&self) -> Option<&str> {
        match self {
            ConceptLabel::Atomic(name) => Some(name),
            _ => None,
        }
    }

    /// Get the negated atomic name if this is a negated atomic concept
    pub fn negated_atomic_name(&self) -> Option<&str> {
        match self {
            ConceptLabel::NegatedAtomic(name) => Some(name),
            _ => None,
        }
    }

    /// Check if two concept labels are complementary (one is the negation of the other)
    pub fn is_complementary(&self, other: &ConceptLabel) -> bool {
        match (self, other) {
            (ConceptLabel::Atomic(name1), ConceptLabel::NegatedAtomic(name2)) => name1 == name2,
            (ConceptLabel::NegatedAtomic(name1), ConceptLabel::Atomic(name2)) => name1 == name2,
            _ => false,
        }
    }

    /// Create a negated version of this concept label
    pub fn negate(&self) -> ConceptLabel {
        match self {
            ConceptLabel::Atomic(name) => ConceptLabel::NegatedAtomic(name.clone()),
            ConceptLabel::NegatedAtomic(name) => ConceptLabel::Atomic(name.clone()),
            // For complex concepts, we don't negate here - should be handled at higher level
            _ => self.clone(),
        }
    }

    /// Parse a concept from string representation
    pub fn parse(concept_str: &str) -> Self {
        // Simple parsing - in practice this would be more sophisticated
        if concept_str.starts_with('!') {
            ConceptLabel::NegatedAtomic(concept_str[1..].to_string())
        } else {
            ConceptLabel::Atomic(concept_str.to_string())
        }
    }

    /// Convert concept to string representation  
    pub fn to_string(&self) -> String {
        match self {
            ConceptLabel::Atomic(name) => name.clone(),
            ConceptLabel::NegatedAtomic(name) => format!("!{}", name),
            ConceptLabel::Intersection(concepts) => {
                format!(
                    "({})",
                    concepts
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(" ⊓ ")
                )
            }
            ConceptLabel::Union(concepts) => {
                format!(
                    "({})",
                    concepts
                        .iter()
                        .map(|c| c.to_string())
                        .collect::<Vec<_>>()
                        .join(" ⊔ ")
                )
            }
            ConceptLabel::Existential { role, filler } => {
                format!("∃{}.{}", role.to_string(), filler.to_string())
            }
            ConceptLabel::Universal { role, filler } => {
                format!("∀{}.{}", role.to_string(), filler.to_string())
            }
            ConceptLabel::AtLeast {
                cardinality,
                role,
                filler,
            } => {
                if let Some(f) = filler {
                    format!("≥{}{}.{}", cardinality, role.to_string(), f.to_string())
                } else {
                    format!("≥{}{}", cardinality, role.to_string())
                }
            }
            ConceptLabel::AtMost {
                cardinality,
                role,
                filler,
            } => {
                if let Some(f) = filler {
                    format!("≤{}{}.{}", cardinality, role.to_string(), f.to_string())
                } else {
                    format!("≤{}{}", cardinality, role.to_string())
                }
            }
            ConceptLabel::Complex(expr) => {
                format!("Complex({:?})", expr)
            }
            ConceptLabel::Nominal(individual) => {
                format!("{{{}}}", individual) // Nominals are shown in curly braces
            }
        }
    }
}

impl RoleLabel {
    /// Parse a role from string representation
    pub fn parse(role_str: &str) -> Self {
        if role_str.starts_with("inv(") && role_str.ends_with(')') {
            let inner = &role_str[4..role_str.len() - 1];
            RoleLabel::Inverse(inner.to_string())
        } else {
            RoleLabel::Atomic(role_str.to_string())
        }
    }

    /// Convert role to string representation
    pub fn to_string(&self) -> String {
        match self {
            RoleLabel::Atomic(name) => name.clone(),
            RoleLabel::Inverse(name) => format!("inv({})", name),
            RoleLabel::Chain(roles) => roles
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(" ∘ "),
            RoleLabel::Complex(expr) => expr.clone(),
        }
    }
}

impl TableauNode {
    /// Create a new tableau node
    pub fn new(id: NodeId, node_type: NodeType) -> Self {
        Self {
            id,
            concepts: HashSet::new(),
            node_type,
            blocking_info: BlockingInfo::default(),
            concept_dependencies: HashMap::new(),
            role_successors: HashMap::new(),
            status: NodeStatus::default(),
        }
    }

    /// Add a concept to this node
    pub fn add_concept(&mut self, concept: ConceptLabel) {
        self.concepts.insert(concept);
    }

    /// Add a concept with dependency information
    pub fn add_concept_with_dependency(
        &mut self,
        concept: ConceptLabel,
        dependency: DependencySet,
    ) {
        self.concepts.insert(concept.clone());
        self.concept_dependencies.insert(concept, dependency);
    }

    /// Check if the node has a specific concept
    pub fn has_concept(&self, concept: &ConceptLabel) -> bool {
        self.concepts.contains(concept)
    }

    /// Check if the node has complementary concepts (clash)
    pub fn has_concept_clash(&self) -> bool {
        for concept1 in &self.concepts {
            for concept2 in &self.concepts {
                if concept1.is_complementary(concept2) {
                    return true;
                }
            }
        }
        false
    }

    /// Add a role successor
    pub fn add_role_successor(&mut self, role: String, successor: NodeId) {
        self.role_successors
            .entry(role)
            .or_insert_with(HashSet::new)
            .insert(successor);
    }

    /// Get all successors for a specific role
    pub fn get_role_successors(&self, role: &str) -> Option<&HashSet<NodeId>> {
        self.role_successors.get(role)
    }

    /// Get all role successors
    pub fn all_role_successors(&self) -> impl Iterator<Item = (&String, &HashSet<NodeId>)> {
        self.role_successors.iter()
    }

    /// Check if the node is blocked
    pub fn is_blocked(&self) -> bool {
        self.blocking_info.is_blocked
    }

    /// Set blocking status
    pub fn set_blocked(&mut self, blocked: bool, blocker: Option<NodeId>) {
        self.blocking_info.is_blocked = blocked;
        self.blocking_info.blocker = blocker;
    }

    /// Get the dependency set for a concept
    pub fn get_concept_dependency(&self, concept: &ConceptLabel) -> Option<&DependencySet> {
        self.concept_dependencies.get(concept)
    }
}

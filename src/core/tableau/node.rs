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
use std::sync::Arc;

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
    pub concept_dependencies: HashMap<ConceptLabel, Arc<DependencySet>>,

    /// Role successors for this node
    ///
    /// Keys are interned `Arc<str>` so multiple edges using the same role
    /// name share a single allocation instead of duplicating the string.
    pub role_successors: HashMap<Arc<str>, HashSet<NodeId>>,

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

    /// Quoted triple assertion (RDF-star)
    /// Represents << s p o >> as a concept that can be reasoned about
    /// The string contains a canonical identifier for the quoted triple
    QuotedTriple(String),

    /// Meta-assertion about a quoted triple
    /// Represents properties about << s p o >> like certainty, provenance, etc.
    MetaAssertion {
        quoted_triple_id: String,
        property: String,
        value: String,
    },
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
    #[must_use]
    pub fn is_atomic(&self) -> bool {
        matches!(self, ConceptLabel::Atomic(_))
    }

    /// Check if this concept label is negated
    #[must_use]
    pub fn is_negated(&self) -> bool {
        matches!(self, ConceptLabel::NegatedAtomic(_))
    }

    /// Check if this concept label is complex
    #[must_use]
    pub fn is_complex(&self) -> bool {
        matches!(self, ConceptLabel::Complex(_))
    }

    /// Get the atomic name if this is an atomic concept
    #[must_use]
    pub fn atomic_name(&self) -> Option<&str> {
        match self {
            ConceptLabel::Atomic(name) => Some(name),
            _ => None,
        }
    }

    /// Get the negated atomic name if this is a negated atomic concept
    #[must_use]
    pub fn negated_atomic_name(&self) -> Option<&str> {
        match self {
            ConceptLabel::NegatedAtomic(name) => Some(name),
            _ => None,
        }
    }

    /// Check if two concept labels are complementary (one is the negation of the other)
    #[must_use]
    pub fn is_complementary(&self, other: &ConceptLabel) -> bool {
        match (self, other) {
            (ConceptLabel::Atomic(name1), ConceptLabel::NegatedAtomic(name2)) => name1 == name2,
            (ConceptLabel::NegatedAtomic(name1), ConceptLabel::Atomic(name2)) => name1 == name2,
            // RDF-star concepts: QuotedTriple and MetaAssertion cannot be directly negated
            // They represent meta-level statements that don't have simple complements
            (ConceptLabel::QuotedTriple(_), _) => false,
            (_, ConceptLabel::QuotedTriple(_)) => false,
            (ConceptLabel::MetaAssertion { .. }, _) => false,
            (_, ConceptLabel::MetaAssertion { .. }) => false,
            _ => false,
        }
    }

    /// Create a negated version of this concept label
    #[must_use]
    pub fn negate(&self) -> ConceptLabel {
        match self {
            ConceptLabel::Atomic(name) => ConceptLabel::NegatedAtomic(name.clone()),
            ConceptLabel::NegatedAtomic(name) => ConceptLabel::Atomic(name.clone()),
            // For complex concepts, we don't negate here - should be handled at higher level
            _ => self.clone(),
        }
    }

    /// Parse a concept from string representation
    #[must_use]
    pub fn parse(concept_str: &str) -> Self {
        // Simple parsing - in practice this would be more sophisticated
        if let Some(stripped) = concept_str.strip_prefix('!') {
            ConceptLabel::NegatedAtomic(stripped.to_string())
        } else {
            ConceptLabel::Atomic(concept_str.to_string())
        }
    }
}

impl std::fmt::Display for ConceptLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConceptLabel::Atomic(name) => write!(f, "{name}"),
            ConceptLabel::NegatedAtomic(name) => write!(f, "!{name}"),
            ConceptLabel::Intersection(concepts) => {
                write!(f, "(")?;
                for (i, c) in concepts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊓ ")?;
                    }
                    write!(f, "{c}")?;
                }
                write!(f, ")")
            }
            ConceptLabel::Union(concepts) => {
                write!(f, "(")?;
                for (i, c) in concepts.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ⊔ ")?;
                    }
                    write!(f, "{c}")?;
                }
                write!(f, ")")
            }
            ConceptLabel::Existential { role, filler } => {
                write!(f, "∃{role}.{filler}")
            }
            ConceptLabel::Universal { role, filler } => {
                write!(f, "∀{role}.{filler}")
            }
            ConceptLabel::AtLeast {
                cardinality,
                role,
                filler,
            } => {
                if let Some(fil) = filler {
                    write!(f, "≥{cardinality}{role}.{fil}")
                } else {
                    write!(f, "≥{cardinality}{role}")
                }
            }
            ConceptLabel::AtMost {
                cardinality,
                role,
                filler,
            } => {
                if let Some(fil) = filler {
                    write!(f, "≤{cardinality}{role}.{fil}")
                } else {
                    write!(f, "≤{cardinality}{role}")
                }
            }
            ConceptLabel::Complex(expr) => {
                write!(f, "Complex({expr:?})")
            }
            ConceptLabel::Nominal(individual) => {
                write!(f, "{{{individual}}}") // Nominals are shown in curly braces
            }
            ConceptLabel::QuotedTriple(id) => {
                // RDF-star quoted triple shown with angle brackets
                write!(f, "{id}")
            }
            ConceptLabel::MetaAssertion {
                quoted_triple_id,
                property,
                value,
            } => {
                // Meta-assertion shown as property-value pair about quoted triple
                write!(f, "{quoted_triple_id} {property}={value}")
            }
        }
    }
}

impl RoleLabel {
    /// Parse a role from string representation
    #[must_use]
    pub fn parse(role_str: &str) -> Self {
        if role_str.starts_with("inv(") && role_str.ends_with(')') {
            let inner = &role_str[4..role_str.len() - 1];
            RoleLabel::Inverse(inner.to_string())
        } else {
            RoleLabel::Atomic(role_str.to_string())
        }
    }
}

impl std::fmt::Display for RoleLabel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoleLabel::Atomic(name) => write!(f, "{name}"),
            RoleLabel::Inverse(name) => write!(f, "inv({name})"),
            RoleLabel::Chain(roles) => {
                let chain_str = roles
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(" ∘ ");
                write!(f, "{chain_str}")
            }
            RoleLabel::Complex(expr) => write!(f, "{expr}"),
        }
    }
}

impl TableauNode {
    /// Create a new tableau node
    #[must_use]
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
        dependency: Arc<DependencySet>,
    ) {
        self.concepts.insert(concept.clone());
        self.concept_dependencies.insert(concept, dependency);
    }

    /// Check if the node has a specific concept
    #[must_use]
    pub fn has_concept(&self, concept: &ConceptLabel) -> bool {
        self.concepts.contains(concept)
    }

    /// Check if the node has complementary concepts (clash)
    #[must_use]
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
            .entry(Arc::from(role.as_str()))
            .or_default()
            .insert(successor);
    }

    /// Get all successors for a specific role
    #[must_use]
    pub fn get_role_successors(&self, role: &str) -> Option<&HashSet<NodeId>> {
        self.role_successors.get(role)
    }

    /// Get all role successors
    pub fn all_role_successors(&self) -> impl Iterator<Item = (&Arc<str>, &HashSet<NodeId>)> {
        self.role_successors.iter()
    }

    /// Check if the node is blocked
    #[must_use]
    pub fn is_blocked(&self) -> bool {
        self.blocking_info.is_blocked
    }

    /// Set blocking status
    pub fn set_blocked(&mut self, is_blocked: bool, blocking_node: Option<NodeId>) {
        self.blocking_info.is_blocked = is_blocked;
        self.blocking_info.blocker = blocking_node;
    }

    /// Get the dependency set for a concept
    #[must_use]
    pub fn get_concept_dependency(&self, concept: &ConceptLabel) -> Option<&Arc<DependencySet>> {
        self.concept_dependencies.get(concept)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_concept_label_quoted_triple() {
        let qt_id = "<<http://example.org/alice http://example.org/knows http://example.org/bob>>"
            .to_string();
        let qt_concept = ConceptLabel::QuotedTriple(qt_id.clone());

        assert_eq!(qt_concept, ConceptLabel::QuotedTriple(qt_id));
    }

    #[test]
    fn test_concept_label_meta_assertion() {
        let qt_id = "<<http://ex.org/s http://ex.org/p http://ex.org/o>>".to_string();
        let meta = ConceptLabel::MetaAssertion {
            quoted_triple_id: qt_id.clone(),
            property: "http://example.org/certainty".to_string(),
            value: "0.95".to_string(),
        };

        match meta {
            ConceptLabel::MetaAssertion {
                quoted_triple_id,
                property,
                value,
            } => {
                assert_eq!(quoted_triple_id, qt_id);
                assert_eq!(property, "http://example.org/certainty");
                assert_eq!(value, "0.95");
            }
            _ => panic!("Expected MetaAssertion variant"),
        }
    }

    #[test]
    fn test_tableau_node_with_quoted_triple() {
        let mut node = TableauNode::new(0, NodeType::Individual);
        let qt_id = "<<http://example.org/alice http://example.org/knows http://example.org/bob>>"
            .to_string();
        let qt_concept = ConceptLabel::QuotedTriple(qt_id);

        node.add_concept(qt_concept.clone());
        assert!(node.has_concept(&qt_concept));
        assert_eq!(node.concepts.len(), 1);
    }

    #[test]
    fn test_tableau_node_with_meta_assertion() {
        let mut node = TableauNode::new(1, NodeType::Individual);
        let qt_id = "<<http://ex.org/s http://ex.org/p http://ex.org/o>>".to_string();

        let meta = ConceptLabel::MetaAssertion {
            quoted_triple_id: qt_id.clone(),
            property: "http://example.org/certainty".to_string(),
            value: "0.95".to_string(),
        };

        node.add_concept(meta.clone());
        assert!(node.has_concept(&meta));
        assert_eq!(node.concepts.len(), 1);
    }

    #[test]
    fn test_quoted_triple_with_meta_assertion_combination() {
        let mut node = TableauNode::new(2, NodeType::Individual);
        let qt_id = "<<http://example.org/doc1 http://example.org/author \"Smith\">>".to_string();

        // Add the quoted triple concept
        let qt_concept = ConceptLabel::QuotedTriple(qt_id.clone());
        node.add_concept(qt_concept.clone());

        // Add meta-assertions about the quoted triple
        let provenance = ConceptLabel::MetaAssertion {
            quoted_triple_id: qt_id.clone(),
            property: "http://example.org/source".to_string(),
            value: "http://example.org/archive23".to_string(),
        };
        node.add_concept(provenance.clone());

        let timestamp = ConceptLabel::MetaAssertion {
            quoted_triple_id: qt_id.clone(),
            property: "http://example.org/timestamp".to_string(),
            value: "2024-01-15".to_string(),
        };
        node.add_concept(timestamp.clone());

        // Verify all concepts are present
        assert!(node.has_concept(&qt_concept));
        assert!(node.has_concept(&provenance));
        assert!(node.has_concept(&timestamp));
        assert_eq!(node.concepts.len(), 3);
    }

    #[test]
    fn test_nested_quoted_triple() {
        // Test nested quoted triple structure
        // << << :a :b :c >> :d :e >> :f :g
        let inner_qt_id = "<<http://ex.org/a http://ex.org/b http://ex.org/c>>".to_string();
        let outer_qt_id = format!("<<{} http://ex.org/d http://ex.org/e>>", inner_qt_id);

        let mut node = TableauNode::new(3, NodeType::Individual);

        // Add inner quoted triple
        let inner_qt = ConceptLabel::QuotedTriple(inner_qt_id.clone());
        node.add_concept(inner_qt.clone());

        // Add outer quoted triple
        let outer_qt = ConceptLabel::QuotedTriple(outer_qt_id.clone());
        node.add_concept(outer_qt.clone());

        // Add meta-assertion about outer quoted triple
        let meta = ConceptLabel::MetaAssertion {
            quoted_triple_id: outer_qt_id.clone(),
            property: "http://ex.org/confidence".to_string(),
            value: "0.85".to_string(),
        };
        node.add_concept(meta.clone());

        assert!(node.has_concept(&inner_qt));
        assert!(node.has_concept(&outer_qt));
        assert!(node.has_concept(&meta));
        assert_eq!(node.concepts.len(), 3);
    }

    #[test]
    fn test_rdf11_vs_rdfstar_concepts() {
        // Test that regular OWL concepts work alongside RDF-star concepts
        let mut node = TableauNode::new(4, NodeType::Individual);

        // Add regular atomic concept
        let person = ConceptLabel::Atomic("http://example.org/Person".to_string());
        node.add_concept(person.clone());

        // Add RDF-star quoted triple
        let qt_id = "<<http://ex.org/john http://ex.org/knows http://ex.org/mary>>".to_string();
        let qt = ConceptLabel::QuotedTriple(qt_id);
        node.add_concept(qt.clone());

        assert!(node.has_concept(&person));
        assert!(node.has_concept(&qt));
        assert_eq!(node.concepts.len(), 2);
    }
}

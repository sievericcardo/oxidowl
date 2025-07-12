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

impl Tableau {
    /// Create a new empty tableau
    pub fn new(config: ReasoningConfig) -> Self {
        let blocking_strategy = BlockingStrategy::create_checker(&config)?;
        // Create expansion strategy - placeholder implementation
        let expansion_strategy = Box::new(DefaultExpansionStrategy::new());
        let completion_rules = CompletionRuleSet::new();

        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            completion_rules,
            blocking_strategy,
            expansion_strategy,
            dependency_tracker: DependencyTracker::new(),
            pending_queue: VecDeque::new(),
            clash_detector: ClashDetector::new(),
            config,
            statistics: TableauStatistics::default(),
            state: TableauState::Unknown,
            property_inclusions: Vec::new(),
            inverse_properties: HashMap::new(),
            functional_properties: HashSet::new(),
            inverse_functional_properties: HashSet::new(),
            transitive_properties: HashSet::new(),
            asymmetric_properties: HashSet::new(),
            reflexive_properties: HashSet::new(),
            irreflexive_properties: HashSet::new(),
        }
    }

    /// Run the tableau algorithm to completion
    pub fn run(&mut self) -> Result<TableauState> {
        let start_time = Instant::now();
        debug!("Starting tableau expansion");
        
        // Initialize root node if needed
        if self.nodes.is_empty() {
            self.create_root_node()?;
        }
        
        // Main expansion loop
        while !self.pending_queue.is_empty() && self.state == TableauState::Unknown {
            // Check for timeout
            if let Some(timeout) = self.config.timeout {
                if start_time.elapsed() > timeout {
                    warn!("Tableau expansion timed out");
                    self.state = TableauState::Unknown;
                    break;
                }
            }
            
            // Get next rule application
            let rule_app = self.pending_queue.pop_front().unwrap();
            
            // Apply the rule
            self.apply_rule(rule_app)?;
            
            // Check for clashes
            if self.clash_detector.has_clashes() {
                debug!("Clash detected, tableau is unsatisfiable");
                self.state = TableauState::Unsatisfiable;
                break;
            }
            
            // Update blocking information
            self.update_blocking()?;
            
            // Check if tableau is complete
            if self.is_complete() {
                debug!("Tableau is complete and satisfiable");
                self.state = TableauState::Satisfiable;
                break;
            }
        }
        
        self.statistics.construction_time = start_time.elapsed();
        debug!("Tableau expansion completed with state: {:?}", self.state);
        
        Ok(self.state)
    }

    /// Create the root node for the tableau
    fn create_root_node(&mut self) -> Result<NodeId> {
        let node_id = self.nodes.len();
        let node = TableauNode {
            id: node_id,
            concepts: HashSet::new(),
            node_type: NodeType::Root,
            blocking_info: BlockingInfo::default(),
            concept_dependencies: HashMap::new(),
            status: NodeStatus::default(),
        };
        
        self.nodes.push(node);
        self.statistics.nodes_created += 1;
        
        Ok(node_id)
    }

    /// Create a new individual node
    pub fn create_node(&mut self, node_type: NodeType) -> NodeId {
        let node_id = self.nodes.len();
        let node = TableauNode {
            id: node_id,
            concepts: HashSet::new(),
            node_type,
            blocking_info: BlockingInfo::default(),
            concept_dependencies: HashMap::new(),
            status: NodeStatus::default(),
        };
        
        self.nodes.push(node);
        self.statistics.nodes_created += 1;
        
        node_id
    }

    /// Add a concept to a node
    pub fn add_concept(
        &mut self,
        node_id: NodeId,
        concept: ConceptLabel,
        dependencies: DependencySet,
    ) -> Result<()> {
        if node_id >= self.nodes.len() {
            return Err(Error::internal(format!("Invalid node ID: {}", node_id)));
        }
        
        let node = &mut self.nodes[node_id];
        
        // Check if concept is already present
        if node.concepts.contains(&concept) {
            return Ok(());
        }
        
        // Add the concept
        node.concepts.insert(concept.clone());
        node.concept_dependencies.insert(concept.clone(), dependencies);
        
        // Queue applicable rules
        self.queue_rules_for_concept(node_id, &concept)?;
        
        Ok(())
    }

    /// Add an edge between nodes
    pub fn add_edge(
        &mut self,
        from: NodeId,
        to: NodeId,
        role: RoleLabel,
        dependencies: DependencySet,
    ) -> Result<()> {
        let edge = TableauEdge {
            from,
            to,
            role: role.clone(),
            dependencies,
        };
        
        self.edges.push(edge);
        self.statistics.edges_created += 1;
        
        // Queue applicable rules for this edge
        self.queue_rules_for_edge(from, to, &role)?;
        
        Ok(())
    }

    /// Apply a completion rule
    fn apply_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        let start_time = Instant::now();
        trace!("Applying rule: {:?} to node: {}", rule_app.rule, rule_app.node);
        
        match rule_app.rule {
            CompletionRule::And => self.apply_and_rule(rule_app)?,
            CompletionRule::Or => self.apply_or_rule(rule_app)?,
            CompletionRule::Some => self.apply_some_rule(rule_app)?,
            CompletionRule::All => self.apply_all_rule(rule_app)?,
            CompletionRule::AtLeast => self.apply_at_least_rule(rule_app)?,
            CompletionRule::AtMost => self.apply_at_most_rule(rule_app)?,
            CompletionRule::Nominal => self.apply_nominal_rule(rule_app)?,
            CompletionRule::Self_ => self.apply_self_rule(rule_app)?,
            CompletionRule::Choose => self.apply_choose_rule(rule_app)?,
            CompletionRule::Datatype => self.apply_datatype_rule(rule_app)?,
            CompletionRule::Unfold => self.apply_unfold_rule(rule_app)?,
            CompletionRule::PropertyChain => self.apply_property_chain_rule(rule_app)?,
            CompletionRule::Guess => self.apply_guess_rule(rule_app)?,
        }
        
        self.statistics.rule_applications += 1;
        self.statistics.rule_application_time += start_time.elapsed();
        
        Ok(())
    }

    /// Apply conjunction rule (A ⊓ B → A, B)
    fn apply_and_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        if let RuleContext::Concept { concept, dependencies } = rule_app.context {
            if let ClassExpression::ObjectIntersectionOf(conjuncts) = concept {
                let node_idx = self.get_node_index(&rule_app.node)?;
                for conjunct in conjuncts {
                    let conjunct_label = ConceptLabel::from_class_expression(&conjunct)?;
                    self.add_concept(node_idx, conjunct_label, dependencies.clone())?;
                }
            }
        }
        Ok(())
    }

    /// Apply disjunction rule (A ⊔ B → A | B)
    fn apply_or_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        // This would involve creating choice points/branches
        // For now, implement a simplified version
        if let RuleContext::Concept { concept, dependencies } = rule_app.context {
            if let ClassExpression::ObjectUnionOf(disjuncts) = concept {
                let node_idx = self.get_node_index(&rule_app.node)?;
                // Create a choice point here
                // For simplicity, just take the first disjunct
                if let Some(first_disjunct) = disjuncts.into_iter().next() {
                    let disjunct_label = ConceptLabel::from_class_expression(&first_disjunct)?;
                    self.add_concept(node_idx, disjunct_label, dependencies)?;
                }
            }
        }
        Ok(())
    }

    /// Apply existential rule (∃R.C → create new node with R-edge and C)
    fn apply_some_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        if let RuleContext::Concept { concept, dependencies } = rule_app.context {
            let concept_label = ConceptLabel::from_class_expression(&concept)?;
            if let ConceptLabel::Existential { role, filler } = concept_label {
                let node_id: NodeId = rule_app.node.parse().map_err(|_| Error::Internal { message: format!("Invalid node ID: {}", rule_app.node) })?;
                
                // Check if there's already a suitable successor
                let suitable_successor = self.find_suitable_successor(node_id, &role, &filler)?;
                
                if suitable_successor.is_none() {
                    // Create new node
                    let new_node = self.create_node(NodeType::Generated);
                    
                    // Add edge
                    self.add_edge(node_id, new_node, role, dependencies.clone())?;
                    
                    // Add concept to new node
                    self.add_concept(new_node, *filler, dependencies)?;
                }
            }
        }
        Ok(())
    }

    /// Apply universal rule (∀R.C with R-edge to y → C on y)
    fn apply_all_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        if let RuleContext::Concept { concept, dependencies } = rule_app.context {
            let concept_label = ConceptLabel::from_class_expression(&concept)?;
            if let ConceptLabel::Universal { role, filler } = concept_label {
                let node_id: NodeId = rule_app.node.parse().map_err(|_| Error::Internal { message: format!("Invalid node ID: {}", rule_app.node) })?;
                
                // Find all R-successors
                let successors = self.find_role_successors(node_id, &role);
                
                for successor in successors {
                    self.add_concept(successor, *filler.clone(), dependencies.clone())?;
                }
            }
        }
        Ok(())
    }

    /// Apply at-least cardinality rule
    fn apply_at_least_rule(&mut self, _rule_app: RuleApplication) -> Result<()> {
        // Implementation would handle ≥n R.C rules
        // This is complex and involves creating multiple nodes
        Ok(())
    }

    /// Apply at-most cardinality rule
    fn apply_at_most_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        // Implementation for ≤n R.C rules (at-most cardinality restrictions)
        if let RuleContext::AtMost { node_id, cardinality, property, filler } = rule_app.context {
            // Convert Role to RoleLabel for tableau operations
            let role_label = RoleLabel::from_role(&property)?;
            
            // Find all R-successors of the node that are instances of C
            let node_index = self.get_node_index(&node_id)?;
            let successors = self.find_role_successors(node_index, &role_label);
            let matching_successors: Vec<_> = successors.into_iter()
                .filter(|successor_id| {
                    self.node_contains_concept(successor_id, &filler).unwrap_or(false)
                })
                .collect();

            // If we have more than n matching successors, we need to merge some
            if matching_successors.len() > cardinality as usize {
                debug!("At-most rule triggered: found {} successors for cardinality ≤{}", 
                       matching_successors.len(), cardinality);
                
                // For now, detect clash (this should ideally try merging first)
                // In a full implementation, we would attempt to merge compatible nodes
                // and only declare a clash if merging fails
                return Err(Error::reasoning("Cardinality clash: too many role successors"));
            }
        }
        Ok(())
    }

    /// Apply nominal rule (individuals)
    fn apply_nominal_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        if let RuleContext::Concept { concept, dependencies } = rule_app.context {
            if let ClassExpression::ObjectOneOf(individuals) = concept {
                let node_id: NodeId = rule_app.node.parse().map_err(|_| Error::Internal { message: format!("Invalid node ID: {}", rule_app.node) })?;
                
                for individual in individuals {
                    let individual_label = ConceptLabel::Nominal(individual.iri.to_string());
                    self.add_concept(node_id, individual_label, dependencies.clone())?;
                }
            }
        }
        Ok(())
    }

    /// Apply self rule (R.Self)
    fn apply_self_rule(&mut self, rule_app: RuleApplication) -> Result<()> {
        if let RuleContext::Concept { concept, dependencies } = rule_app.context {
            if let ClassExpression::ObjectHasSelf(role) = concept {
                let node_id: NodeId = rule_app.node.parse().map_err(|_| Error::Internal { message: format!("Invalid node ID: {}", rule_app.node) })?;
                
                // Create a self-loop edge
                let role_label = RoleLabel::from_role(&role)?;
                self.add_edge(node_id, node_id, role_label, dependencies)?;
            }
        }
        Ok(())
    }

    /// Apply choose rule (choice points)
    fn apply_choose_rule(&mut self, _rule_app: RuleApplication) -> Result<()> {
        // This would involve creating choice points for disjunctions
        // For now, we can just log that this rule was applied
        debug!("Choose rule applied, creating choice point");
        Ok(())
    }

    /// Apply datatype rule (data properties)
    fn apply_datatype_rule(&mut self, _rule_app: RuleApplication) -> Result<()> {
        // Implementation for data properties would go here
        // For now, we can just log that this rule was applied
        debug!("Datatype rule applied, handling data properties");
        Ok(())
    }

    /// Apply unfold rule (unfolding definitions)
    fn apply_unfold_rule(&mut self, _rule_app: RuleApplication) -> Result<()> {
        // This would involve unfolding definitions in the ontology
        // For now, we can just log that this rule was applied
        debug!("Unfold rule applied, unfolding definitions");
        Ok(())
    }

    /// Apply property chain rule (complex role chains)
    fn apply_property_chain_rule(&mut self, _rule_app: RuleApplication) -> Result<()> {
        // TODO: Implement property chain rule application as follows:
        // 1. Find property chain axioms from the ontology
        // 2. Check for sequences of edges that match the chain
        // 3. Add super property edges where chains are completed
        debug!("Property chain rule applied, checking for property chains");
        Ok(())
    }

    /// Check subsumption between two class expressions (placeholder)
    pub fn check_subsumption(&self, _subclass: &ClassExpression, _superclass: &ClassExpression) -> Result<bool> {
        // Placeholder implementation
        // TODO: implement proper subsumption checking
        Ok(false)
    }

    /// Queue completion rules for a newly added concept
    fn queue_rules_for_concept(&mut self, node_id: NodeId, concept: &ConceptLabel) -> Result<()> {
        // Determine which rules are applicable for this concept
        let class_expr = concept.to_class_expression()?;
        let applicable_rules = self.completion_rules.get_applicable_rules(&class_expr);
        
        for rule in applicable_rules {
            let rule_app = RuleApplication {
                rule,
                node: node_id.to_string(),
                context: RuleContext::Concept {
                    concept: class_expr.clone(),
                    dependencies: DependencySet::empty(), // Would be populated properly
                },
                priority: self.get_rule_priority(&rule),
                dependencies: DependencySet::empty(), // Would be populated properly
            };
            
            self.pending_queue.push_back(rule_app);
        }
        
        // Sort queue by priority
        self.pending_queue.make_contiguous().sort_by_key(|app| app.priority);
        
        Ok(())
    }

    /// Queue completion rules for a newly added edge
    fn queue_rules_for_edge(&mut self, from: NodeId, to: NodeId, role: &RoleLabel) -> Result<()> {
        // Queue universal rules that might apply
        // TODO: check all universal concepts on the source node
        Ok(())
    }

    /// Get priority for a completion rule
    fn get_rule_priority(&self, rule: &CompletionRule) -> RulePriority {
        match rule {
            CompletionRule::And => RulePriority::Highest,
            CompletionRule::All => RulePriority::High,
            CompletionRule::Some => RulePriority::Normal,
            CompletionRule::Or => RulePriority::Low,
            CompletionRule::AtLeast | CompletionRule::AtMost => RulePriority::Low,
            CompletionRule::Nominal => RulePriority::Normal,
            CompletionRule::Self_ => RulePriority::Normal,
            CompletionRule::Choose => RulePriority::Lowest,
            CompletionRule::Datatype => RulePriority::High,
            CompletionRule::Unfold => RulePriority::Highest,
            CompletionRule::PropertyChain => RulePriority::High,
            CompletionRule::Guess => RulePriority::Lowest,
        }
    }

    /// Find a suitable successor for an existential restriction
    fn find_suitable_successor(
        &self,
        node_id: NodeId,
        role: &RoleLabel,
        filler: &ConceptLabel,
    ) -> Result<Option<NodeId>> {
        // Look for existing R-successors that have the filler concept
        for edge in &self.edges {
            if edge.from == node_id && edge.role == *role {
                let successor = &self.nodes[edge.to];
                if successor.concepts.contains(filler) {
                    return Ok(Some(edge.to));
                }
            }
        }
        Ok(None)
    }

    /// Find all role successors of a node
    fn find_role_successors(&self, node_id: NodeId, role: &RoleLabel) -> Vec<NodeId> {
        self.edges
            .iter()
            .filter(|edge| edge.from == node_id && edge.role == *role)
            .map(|edge| edge.to)
            .collect()
    }

    /// Check if a node contains a specific concept
    fn node_contains_concept(&self, node_id: &NodeId, concept: &ClassExpression) -> Result<bool> {
        if let Some(node) = self.nodes.get(*node_id) {
            // Check if the node's concept set contains the given concept
            for concept_label in &node.concepts {
                if let Ok(class_expr) = concept_label.to_class_expression() {
                    if &class_expr == concept {
                        return Ok(true);
                    }
                }
            }
        }
        Ok(false)
    }

    /// Get the node index from a node ID string
    fn get_node_index(&self, node_id: &str) -> Result<NodeId> {
        // Simple implementation: try to parse the node_id as a number
        // TODO: maintain a mapping from string IDs to indices
        node_id.parse().map_err(|_| Error::reasoning("Invalid node ID"))
    }

    /// Update blocking information for all nodes
    fn update_blocking(&mut self) -> Result<()> {
        let start_time = Instant::now();
        
        // TODO: implement the blocking algorithm
        self.statistics.blocking_time += start_time.elapsed();
        Ok(())
    }

    /// Check if the tableau is complete (no more rules to apply)
    fn is_complete(&self) -> bool {
        self.pending_queue.is_empty() && !self.clash_detector.has_clashes()
    }

    /// Get the number of nodes in the tableau
    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of backtracking operations
    pub fn get_backtrack_count(&self) -> usize {
        self.statistics.backtracking_operations
    }

    /// Get the maximum depth reached
    pub fn get_max_depth(&self) -> usize {
        self.statistics.max_depth
    }
}

impl ConceptLabel {
    /// Convert a class expression to a concept label
    pub fn from_class_expression(class_expr: &ClassExpression) -> Result<Self> {
        match class_expr {
            ClassExpression::Class(class) => {
                Ok(ConceptLabel::Atomic(class.iri.to_string()))
            }
            ClassExpression::ObjectComplementOf(inner) => {
                match inner.as_ref() {
                    ClassExpression::Class(class) => {
                        Ok(ConceptLabel::NegatedAtomic(class.iri.to_string()))
                    }
                    _ => Ok(ConceptLabel::Complex(Box::new(class_expr.clone()))),
                }
            }
            _ => Ok(ConceptLabel::Complex(Box::new(class_expr.clone()))),
        }
    }

    /// Convert a concept label back to a class expression
    pub fn to_class_expression(&self) -> Result<ClassExpression> {
        match self {
            ConceptLabel::Atomic(iri) => {
                Ok(ClassExpression::Class(Class {
                    iri: IRI::new(iri.to_string()).to_url()?,
                }))
            }
            ConceptLabel::NegatedAtomic(iri) => {
                Ok(ClassExpression::ObjectComplementOf(Box::new(ClassExpression::Class(Class {
                    iri: IRI::new(iri.to_string()).to_url()?,
                }))))
            }
            ConceptLabel::Complex(expr) => Ok(*expr.clone()),
            ConceptLabel::Existential { role, filler } => {
                let role_iri = match role {
                    RoleLabel::Atomic(iri) => iri.clone(),
                    RoleLabel::Inverse(iri) => iri.clone(),
                };
                Ok(ClassExpression::ObjectSomeValuesFrom {
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: url::Url::parse(&role_iri).map_err(|e| crate::Error::ontology_parsing(format!("Invalid IRI: {}", e)))?,
                    }),
                    filler: Box::new(filler.to_class_expression()?),
                })
            }
            ConceptLabel::Universal { role, filler } => {
                let role_iri = match role {
                    RoleLabel::Atomic(iri) => iri.clone(),
                    RoleLabel::Inverse(iri) => iri.clone(),
                };
                Ok(ClassExpression::ObjectAllValuesFrom {
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: IRI::new(role_iri).to_url()?,
                    }),
                    filler: Box::new(filler.to_class_expression()?),
                })
            }
            ConceptLabel::AtLeast { cardinality, role, filler } => {
                let role_iri = match role {
                    RoleLabel::Atomic(iri) => iri.clone(),
                    RoleLabel::Inverse(iri) => iri.clone(),
                };
                Ok(ClassExpression::ObjectMinCardinality {
                    cardinality: *cardinality,
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: IRI::new(role_iri).to_url()?,
                    }),
                    filler: match filler.as_ref() {
                        Some(f) => Some(Box::new(f.to_class_expression()?)),
                        None => None,
                    },
                })
            }
            ConceptLabel::AtMost { cardinality, role, filler } => {
                let role_iri = match role {
                    RoleLabel::Atomic(iri) => iri.clone(),
                    RoleLabel::Inverse(iri) => iri.clone(),
                };
                Ok(ClassExpression::ObjectMaxCardinality {
                    cardinality: *cardinality,
                    property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                        iri: IRI::new(role_iri).to_url()?,
                    }),
                    filler: match filler.as_ref() {
                        Some(f) => Some(Box::new(f.to_class_expression()?)),
                        None => None,
                    },
                })
            }
            ConceptLabel::Nominal(individual_iri) => {
                // Convert nominal to ObjectOneOf with a single individual
                Ok(ClassExpression::ObjectOneOf(vec![
                    crate::ontology::Individual {
                        iri: IRI::new(individual_iri.clone()).to_url()?,
                    }
                ]))
            }
        }
    }
}

impl ClashDetector {
    /// Create a new clash detector
    pub fn new() -> Self {
        Self {
            clashes: Vec::new(),
        }
    }

    /// Check if there are any clashes
    pub fn has_clashes(&self) -> bool {
        !self.clashes.is_empty()
    }

    /// Detect clashes in the tableau
    pub fn detect_clashes(&mut self, tableau: &Tableau) -> Vec<Clash> {
        self.clashes.clear();
        
        // Check for concept clashes in each node
        for node in &tableau.nodes {
            self.check_concept_clashes(node);
        }
        
        // Check for cardinality clashes
        self.check_cardinality_clashes(tableau);
        
        self.clashes.clone()
    }

    /// Check for concept clashes (C and ¬C) in a node
    fn check_concept_clashes(&mut self, node: &TableauNode) {
        let mut atomic_concepts = HashSet::new();
        let mut negated_concepts = HashSet::new();
        
        for concept in &node.concepts {
            match concept {
                ConceptLabel::Atomic(name) => {
                    atomic_concepts.insert(name.clone());
                }
                ConceptLabel::NegatedAtomic(name) => {
                    negated_concepts.insert(name.clone());
                }
                _ => {} // Handle other concept types
            }
        }
        
        // Find intersections
        for concept in atomic_concepts.intersection(&negated_concepts) {
            let clash = Clash {
                clash_type: ClashType::Concept {
                    concept: concept.clone(),
                    node: node.id,
                },
                nodes: vec![node.id],
                dependencies: DependencySet::empty(), // Would be computed properly
                explanation: format!("Concept clash: {} and ¬{} in node {}", concept, concept, node.id),
            };
            self.clashes.push(clash);
        }
    }

    /// Check for cardinality clashes
    fn check_cardinality_clashes(&mut self, _tableau: &Tableau) {
        // Implementation would check for cardinality violations
        // This is complex and involves counting role successors
    }
}

/// Property inclusion relationship (SubObjectPropertyOf)
#[derive(Debug, Clone)]
pub struct PropertyInclusion {
    pub sub_property: String,
    pub super_property: String,
}

/// Default expansion strategy implementation
#[derive(Debug)]
struct DefaultExpansionStrategy;

impl DefaultExpansionStrategy {
    fn new() -> Self {
        Self
    }
}

impl ExpansionStrategy for DefaultExpansionStrategy {
    fn initialize(&mut self, _context: &ExpansionContext) -> Result<()> {
        Ok(())
    }
    
    fn select_next_existential(&mut self, candidates: &[ExistentialCandidate]) -> Option<ExistentialCandidate> {
        candidates.first().cloned()
    }
    
    fn order_expansions(&self, _existentials: &mut [ExistentialCandidate]) {
        // Default order - no reordering
    }
    
    fn should_delay_expansion(&self, _candidate: &ExistentialCandidate, _context: &ExpansionContext) -> bool {
        false
    }
    
    fn get_expansion_priority(&self, _candidate: &ExistentialCandidate) -> ExpansionPriority {
        ExpansionPriority::Normal
    }
    
    fn expansion_completed(&mut self, _candidate: &ExistentialCandidate, _result: &ExpansionResult) {
        // Default - no action needed
    }
    
    fn clear(&mut self) {
        // Default - no state to clear
    }
}

/// Tableau for OWL reasoning
#[derive(Debug)]
pub struct TableauBuilder {
    /// Reasoning configuration
    config: ReasoningConfig,
}

impl TableauBuilder {
    /// Create a new tableau builder
    pub fn new(config: &ReasoningConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
        })
    }

    /// Create a tableau (main method used by reasoner)
    pub fn create_tableau(&self) -> Result<Tableau> {
        Tableau::new(self.config.clone())
    }

    /// Build a tableau for consistency checking
    pub fn build_for_consistency(&self, ontology: &Ontology) -> Result<Tableau> {
        let mut tableau = Tableau::new(self.config.clone())?;
        
        // Add all axioms to the tableau
        for axiom in &ontology.axioms {
            self.add_axiom_to_tableau(&mut tableau, axiom)?;
        }
        
        Ok(tableau)
    }

    /// Build a tableau for satisfiability checking
    pub fn build_for_satisfiability(&self, ontology: &Ontology, class_iri: &str) -> Result<Tableau> {
        let mut tableau = Tableau::new(self.config.clone())?;
        
        // Add all axioms
        for axiom in &ontology.axioms {
            self.add_axiom_to_tableau(&mut tableau, axiom)?;
        }
        
        // Add the class to test to the root node
        let root = tableau.create_root_node()?;
        let concept = ConceptLabel::Atomic(class_iri.to_string());
        tableau.add_concept(root, concept, DependencySet::empty())?;
        
        Ok(tableau)
    }

    /// Build a tableau for subsumption checking (A ⊑ B iff A ⊓ ¬B is unsatisfiable)
    pub fn build_for_subsumption(
        &self,
        ontology: &Ontology,
        subclass: &str,
        superclass: &str,
    ) -> Result<Tableau> {
        let mut tableau = Tableau::new(self.config.clone())?;
        
        // Add all axioms
        for axiom in &ontology.axioms {
            self.add_axiom_to_tableau(&mut tableau, axiom)?;
        }
        
        // Add A ⊓ ¬B to the root node
        let root = tableau.create_root_node()?;
        let subclass_concept = ConceptLabel::Atomic(subclass.to_string());
        let negated_superclass = ConceptLabel::NegatedAtomic(superclass.to_string());
        
        tableau.add_concept(root, subclass_concept, DependencySet::empty())?;
        tableau.add_concept(root, negated_superclass, DependencySet::empty())?;
        
        Ok(tableau)
    }

    /// Build a tableau for instance checking
    pub fn build_for_instance_check(
        &self,
        ontology: &Ontology,
        individual: &str,
        class: &str,
    ) -> Result<Tableau> {
        let mut tableau = Tableau::new(self.config.clone())?;
        
        // Add all axioms
        for axiom in &ontology.axioms {
            self.add_axiom_to_tableau(&mut tableau, axiom)?;
        }
        
        // Add individual assertion and negated class
        let root = tableau.create_root_node()?;
        let individual_concept = ConceptLabel::Nominal(individual.to_string());
        let negated_class = ConceptLabel::NegatedAtomic(class.to_string());
        
        tableau.add_concept(root, individual_concept, DependencySet::empty())?;
        tableau.add_concept(root, negated_class, DependencySet::empty())?;
        
        Ok(tableau)
    }

    /// Add an axiom to the tableau
    fn add_axiom_to_tableau(&self, tableau: &mut Tableau, axiom: &Axiom) -> Result<()> {
        match axiom {
            Axiom::SubClassOf(subclass_axiom) => {
                // A ⊑ B becomes ¬A ⊔ B
                // Add this as a concept to all nodes or as a global constraint
                // For simplicity, we'll add it to the root node when created
            }
            Axiom::ClassAssertion(class_assertion) => {
                // Add C(a) - the individual a has concept C
                // This would typically be handled during node creation
            }
            // Handle other axiom types...
            _ => {
                // Handle SubObjectPropertyOf, InverseObjectProperties, etc.
                self.add_object_property_axiom_to_tableau(tableau, axiom)?;
            }
        }
        Ok(())
    }

    /// Add object property axiom to tableau
    fn add_object_property_axiom_to_tableau(&self, tableau: &mut Tableau, axiom: &Axiom) -> Result<()> {
        match axiom {
            Axiom::SubObjectPropertyOf(axiom) => {
                // Add property inclusion to tableau
                let property_inclusion = PropertyInclusion {
                    sub_property: format!("{:?}", axiom.sub_property),
                    super_property: format!("{:?}", axiom.super_property),
                };
                tableau.property_inclusions.push(property_inclusion);
            }
            Axiom::InverseObjectProperties(axiom) => {
                // Add inverse property relationship
                let first_str = format!("{:?}", axiom.first);
                let second_str = format!("{:?}", axiom.second);
                tableau.inverse_properties.insert(first_str.clone(), second_str.clone());
                tableau.inverse_properties.insert(second_str, first_str);
            }
            Axiom::FunctionalObjectProperty(axiom) => {
                // Mark property as functional
                tableau.functional_properties.insert(format!("{:?}", axiom.property));
            }
            Axiom::InverseFunctionalObjectProperty(axiom) => {
                // Mark property as inverse functional
                tableau.inverse_functional_properties.insert(format!("{:?}", axiom.property));
            }
            Axiom::TransitiveObjectProperty(axiom) => {
                // Mark property as transitive
                tableau.transitive_properties.insert(format!("{:?}", axiom.property));
            }
            Axiom::SymmetricObjectProperty(axiom) => {
                // Mark property as symmetric (equivalent to self-inverse)
                let property_str = format!("{:?}", axiom.property);
                tableau.inverse_properties.insert(property_str.clone(), property_str);
            }
            Axiom::AsymmetricObjectProperty(axiom) => {
                // Add asymmetric constraint
                tableau.asymmetric_properties.insert(format!("{:?}", axiom.property));
            }
            Axiom::ReflexiveObjectProperty(axiom) => {
                // Mark property as reflexive
                tableau.reflexive_properties.insert(format!("{:?}", axiom.property));
            }
            Axiom::IrreflexiveObjectProperty(axiom) => {
                // Mark property as irreflexive
                tableau.irreflexive_properties.insert(format!("{:?}", axiom.property));
            }
            _ => {
                // Handle other types or ignore
            }
        }
        Ok(())
    }
}
//! Tableau reasoning module
//!
//! This module provides a complete tableau-based reasoning system for
//! OWL 2 DL. The tableau is split into focused submodules for maintainability.

pub mod absorption;
pub mod builder;
pub mod clause_checker;
pub mod clause_index;
pub mod disjointness;
pub mod edge;
pub mod equivalence;
pub mod executor;
pub mod incremental_checker;
pub mod node;
pub mod state;

pub use absorption::{AbsorbablePattern, AbsorptionStats, ClauseAbsorber};
pub use builder::TableauBuilder;
pub use clause_checker::{ClauseChecker, ClauseCheckerConfig, ClauseViolation};
pub use clause_index::{ClauseIndex, IndexStatistics};
pub use disjointness::DisjointnessMap;
pub use edge::{PropertyInclusion, TableauEdge};
pub use equivalence::{ConceptId, EquivalenceClosure};
pub use executor::TableauExecutor;
pub use incremental_checker::{
    CacheStatistics, CachedCheckResult, ChangeTracker, CheckResultCache, NodeFingerprint,
};
pub use node::{ConceptLabel, NodeId, NodeStatus, NodeType, RoleLabel, TableauNode};
pub use state::{Clash, ClashDetector, ClashType, Priority, TableauState, TableauStatistics};

use crate::{
    Error, Result,
    config::{ReasoningConfig, TableauConfig},
    core::{
        blocking::BlockingStrategy,
        completion::{CompletionStrategy, RuleApplication},
        dependency::DependencySet,
        expansion::{DefaultExpansionStrategy, ExpansionStrategy},
    },
    ontology::{ClassExpression, Ontology},
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::Arc,
};

/// Main tableau structure
pub struct Tableau {
    /// All nodes in the tableau
    pub nodes: Vec<TableauNode>,

    /// All edges between nodes
    pub edges: Vec<TableauEdge>,

    /// Current state of the tableau
    pub state: TableauState,

    /// Clash detection system
    pub clash_detector: ClashDetector,

    /// Execution statistics
    pub statistics: TableauStatistics,

    /// Configuration for tableau expansion
    pub config: TableauConfig,

    /// Pending rule applications (priority queue)
    pub pending_queue: VecDeque<RuleApplication>,

    /// Completion strategy
    pub completion_strategy: CompletionStrategy,

    /// Blocking strategy
    pub blocking_strategy: BlockingStrategy,

    /// Expansion strategy
    pub expansion_strategy: DefaultExpansionStrategy,

    /// Clause checker for DL clause validation (optional)
    pub clause_checker: Option<ClauseChecker>,

    /// Reference to the ontology for querying axioms during reasoning
    pub ontology: Arc<Ontology>,

    /// Concept cache for performance
    concept_cache: HashMap<String, ConceptLabel>,

    /// Role cache for performance
    role_cache: HashMap<String, RoleLabel>,

    /// Individual mapping
    individual_map: HashMap<String, NodeId>,

    /// Backtrack stack for non-deterministic choices
    backtrack_stack: Vec<BacktrackPoint>,
}

/// A backtrack point for handling non-deterministic choices
#[derive(Debug, Clone)]
pub struct BacktrackPoint {
    /// ID of this backtrack point
    pub id: usize,

    /// Node where the choice was made
    pub node_id: NodeId,

    /// The choice that was made
    pub choice: Choice,

    /// State before the choice
    pub saved_state: SavedState,

    /// Dependencies at this point
    pub dependencies: Arc<DependencySet>,
}

/// Types of non-deterministic choices
#[derive(Debug, Clone)]
pub enum Choice {
    /// Disjunction choice (C ⊔ D)
    Disjunction {
        /// The concepts to choose from
        concepts: Vec<ConceptLabel>,
        /// Index of chosen concept
        chosen_index: usize,
    },

    /// At-most merging choice
    AtMostMerge {
        /// Nodes that could be merged
        candidates: Vec<NodeId>,
        /// Chosen merge target
        chosen: NodeId,
    },

    /// Blocking choice
    Blocking {
        /// Node being blocked
        blocked_node: NodeId,
        /// Blocking node
        blocker: NodeId,
    },
}

/// Saved state for backtracking
#[derive(Debug, Clone)]
pub struct SavedState {
    /// Number of nodes at this point
    pub node_count: usize,

    /// Number of edges at this point
    pub edge_count: usize,

    /// Pending queue state
    pub pending_queue: VecDeque<RuleApplication>,
}

impl Tableau {
    /// Create a new tableau with the given configuration and ontology
    pub fn new(config: ReasoningConfig, ontology: Arc<Ontology>) -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            state: TableauState::Unknown,
            clash_detector: ClashDetector::new(),
            statistics: TableauStatistics::new(),
            config: TableauConfig {
                max_depth: config.max_expansion_depth,
                timeout: config.timeout,
                blocking_enabled: true,
                optimization_enabled: true,
                rdf11_mode: false,              // RDF-star enabled by default
                quoted_triple_reasoning_depth: 2, // Allow 2 levels of nesting
            },
            pending_queue: VecDeque::new(),
            completion_strategy: CompletionStrategy::default(),
            blocking_strategy: BlockingStrategy,
            expansion_strategy: DefaultExpansionStrategy::default(),
            clause_checker: None, // Will be populated when ontology is loaded
            ontology,
            concept_cache: HashMap::new(),
            role_cache: HashMap::new(),
            individual_map: HashMap::new(),
            backtrack_stack: Vec::new(),
        }
    }

    /// Initialize tableau from ontology
    pub fn fontology_arc = Arc::new(ontology.clone());
        let mut tableau = Self::new(config, ontology_arcgy, config: ReasoningConfig) -> Result<Self> {
        let mut tableau = Self::new(config);
        tableau.load_ontology(ontology)?;
        Ok(tableau)
    }

    /// Load axioms from an ontology
    pub fn load_ontology(&mut self, ontology: &Ontology) -> Result<()> {
        // Generate DL clauses from the ontology
        use crate::dl_clauses::DLClauseGenerator;

        let mut generator = DLClauseGenerator::new();
        let clause_set = generator.generate_clauses(ontology)?;

        log::info!(
            "Loaded {} deterministic clauses and {} disjunctive clauses from ontology",
            clause_set.deterministic_clauses.len(),
            clause_set.disjunctive_clauses.len()
        );

        // Only create EquivalenceClosure and DisjointnessMap if the ontology has relevant axioms
        // This avoids expensive initialization for simple ontologies
        let has_equiv_or_disjoint = ontology.axioms().iter().any(|axiom| {
            matches!(
                axiom,
                crate::ontology::Axiom::EquivalentClasses(_)
                    | crate::ontology::Axiom::DisjointClasses(_)
                    | crate::ontology::Axiom::DisjointUnion(_)
            )
        });

        let checker = if has_equiv_or_disjoint {
            // Create EquivalenceClosure and DisjointnessMap for enhanced reasoning
            let eq_closure = EquivalenceClosure::from_ontology(ontology)?;
            let disj_map = DisjointnessMap::from_ontology(ontology, &eq_closure)?;
            ClauseChecker::with_reasoning_support(clause_set, eq_closure, disj_map)
        } else {
            // No equivalence or disjointness axioms, use simple checker
            ClauseChecker::new(clause_set)
        };

        // Store the ClauseChecker for use during tableau expansion
        self.clause_checker = Some(checker);
        log::debug!(
            "ClauseChecker initialized for dynamic clause checking during tableau expansion"
        );

        // Create root node if it doesn't exist
        if self.nodes.is_empty() {
            self.add_node(NodeType::Root)?;
        }

        // Process deterministic clauses to extract initial concepts and constraints
        // For consistency checking, we add concepts to the root node that represent
        // the ontology axioms. Key axioms to process:
        // 1. EquivalentClasses - bidirectional implications
        // 2. DisjointUnion - coverage and disjointness constraints
        // 3. DisjointClasses - disjointness constraints
        // 4. SubClassOf - subsumption constraints

        // For now, we note that DL clauses have been generated
        // The actual tableau expansion should use these clauses via the
        // completion rules that process them

        // Store reference to the ontology's clauses for use during expansion
        // This is a simplified implementation - a full implementation would
        // convert DL clauses into tableau concepts and rules

        // Process axioms directly to ensure they are applied to the tableau
        // For consistency checking, we only need to process ClassAssertion axioms
        // Other axioms (EquivalentClasses, DisjointClasses, SubClassOf) are handled
        // via the DL clause generation and don't need explicit processing here
        for axiom in ontology.axioms() {
            // Only process axioms that affect the tableau state
            if matches!(axiom, crate::ontology::Axiom::ClassAssertion(_)) {
                self.process_axiom(axiom)?;
            }
        }

        Ok(())
    }

    /// Process a single axiom and add its constraints to the tableau
    fn process_axiom(&mut self, axiom: &crate::ontology::Axiom) -> Result<()> {
        use crate::core::completion::{CompletionRule, RuleApplication, RuleContext, RulePriority};
        use crate::core::dependency::DependencySet;
        use crate::ontology::{Axiom, ClassExpression};

        let root_id = 0; // Root node

        match axiom {
            // Handle EquivalentClasses axioms
            Axiom::EquivalentClasses(equiv) => {
                // For C1 ≡ C2, add both C1 and C2 to the root
                // This allows the tableau to check their consistency
                for class_expr in &equiv.classes {
                    let concept = ConceptLabel::Complex(Box::new(class_expr.clone()));
                    if let Some(root) = self.nodes.get_mut(root_id) {
                        root.concepts.insert(concept.clone());
                    }
                    // Queue rules if needed
                    self.queue_rule_for_concept(root_id, class_expr)?;
                }
            }

            // Handle DisjointUnion axioms - most important for this test case!
            Axiom::DisjointUnion(disj_union) => {
                // For DisjointUnion(C, [C1, C2, ..., Cn]), we need to ensure:
                // 1. C ≡ (C1 ⊔ C2 ⊔ ... ⊔ Cn) - coverage
                // 2. Ci ⊓ Cj ≡ ⊥ for i ≠ j - pairwise disjointness

                // Add pairwise disjointness by adding Ci ⊓ Cj and expecting clashes
                for i in 0..disj_union.disjoint_classes.len() {
                    for j in (i + 1)..disj_union.disjoint_classes.len() {
                        // Create intersection Ci ⊓ Cj
                        let intersection = ClassExpression::ObjectIntersectionOf(vec![
                            disj_union.disjoint_classes[i].clone(),
                            disj_union.disjoint_classes[j].clone(),
                        ]);

                        let concept = ConceptLabel::Complex(Box::new(intersection.clone()));
                        if let Some(root) = self.nodes.get_mut(root_id) {
                            root.concepts.insert(concept);
                        }

                        // Queue AND rule to expand this intersection
                        let rule_app = RuleApplication {
                            rule: CompletionRule::And,
                            node: root_id.to_string(),
                            context: RuleContext::Concept {
                                concept: intersection,
                                dependencies: Arc::new(DependencySet::new()),
                            },
                            priority: RulePriority::High,
                            dependencies: Arc::new(DependencySet::new()),
                        };
                        self.pending_queue.push_back(rule_app);
                    }
                }
            }

            // Handle DisjointClasses axioms
            Axiom::DisjointClasses(disj) => {
                // Add pairwise disjointness
                for i in 0..disj.classes.len() {
                    for j in (i + 1)..disj.classes.len() {
                        let intersection = ClassExpression::ObjectIntersectionOf(vec![
                            disj.classes[i].clone(),
                            disj.classes[j].clone(),
                        ]);

                        let concept = ConceptLabel::Complex(Box::new(intersection.clone()));
                        if let Some(root) = self.nodes.get_mut(root_id) {
                            root.concepts.insert(concept);
                        }

                        // Queue AND rule
                        let rule_app = RuleApplication {
                            rule: CompletionRule::And,
                            node: root_id.to_string(),
                            context: RuleContext::Concept {
                                concept: intersection,
                                dependencies: Arc::new(DependencySet::new()),
                            },
                            priority: RulePriority::High,
                            dependencies: Arc::new(DependencySet::new()),
                        };
                        self.pending_queue.push_back(rule_app);
                    }
                }
            }

            // Handle SubClassOf axioms
            Axiom::SubClassOf(subclass) => {
                // A ⊑ B is checked by testing A ⊓ ¬B for unsatisfiability
                let negated_super =
                    ClassExpression::ObjectComplementOf(Box::new(subclass.superclass.clone()));
                let intersection = ClassExpression::ObjectIntersectionOf(vec![
                    subclass.subclass.clone(),
                    negated_super,
                ]);

                let concept = ConceptLabel::Complex(Box::new(intersection.clone()));
                if let Some(root) = self.nodes.get_mut(root_id) {
                    root.concepts.insert(concept);
                }

                // Queue AND rule
                let rule_app = RuleApplication {
                    rule: CompletionRule::And,
                    node: root_id.to_string(),
                    context: RuleContext::Concept {
                        concept: intersection,
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::High,
                    dependencies: Arc::new(DependencySet::new()),
                };
                self.pending_queue.push_back(rule_app);
            }

            // Handle ClassAssertion axioms
            Axiom::ClassAssertion(assertion) => {
                // Get or create node for the individual
                let individual_iri = assertion.individual.to_string();
                let node_id = if let Some(&existing_id) = self.individual_map.get(&individual_iri) {
                    existing_id
                } else {
                    let new_id = self.add_node(NodeType::Nominal)?;
                    self.individual_map.insert(individual_iri, new_id);
                    new_id
                };

                // Add the class assertion to the node
                let concept = ConceptLabel::Complex(Box::new(assertion.class.clone()));
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.concepts.insert(concept);
                }

                // Also add all equivalent classes (if any) - but only if we have equivalence closure
                // This is an optimization to avoid expensive lookups when not needed
                if let Some(checker) = &mut self.clause_checker {
                    if let ClassExpression::Class(ref class) = assertion.class {
                        if let Some(eq_closure) = checker.equivalence_closure() {
                            let concept_id = equivalence::ConceptId(class.iri.to_string());
                            let equiv_class = eq_closure.get_equivalence_class(&concept_id);

                            // Only add equivalent concepts if there are any (skip if just the original concept)
                            if equiv_class.len() > 1 {
                                for equiv_concept_id in equiv_class {
                                    // Skip if it's the same as the original concept
                                    if equiv_concept_id.0 == class.iri.to_string() {
                                        continue;
                                    }

                                    // Add the equivalent concept to the node
                                    let equiv_iri = crate::ontology::IRI::new(&equiv_concept_id.0);
                                    let equiv_class = crate::ontology::Class::new(equiv_iri);
                                    let equiv_expr = ClassExpression::Class(equiv_class);
                                    let equiv_concept = ConceptLabel::Complex(Box::new(equiv_expr));
                                    if let Some(node) = self.nodes.get_mut(node_id) {
                                        node.concepts.insert(equiv_concept);
                                    }
                                }
                            }
                        }
                    }
                }

                // Queue rule for expanding this concept
                self.queue_rule_for_concept(node_id, &assertion.class)?;
            }

            // Other axioms can be added as needed
            _ => {}
        }

        Ok(())
    }

    /// Queue a rule for a class expression if needed
    fn queue_rule_for_concept(
        &mut self,
        node_id: NodeId,
        class_expr: &ClassExpression,
    ) -> Result<()> {
        use crate::core::completion::{CompletionRule, RuleApplication, RuleContext, RulePriority};
        use crate::core::dependency::DependencySet;

        match class_expr {
            ClassExpression::ObjectIntersectionOf(_) => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::And,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::High,
                    dependencies: Arc::new(DependencySet::new()),
                };
                self.pending_queue.push_back(rule_app);
            }
            ClassExpression::ObjectUnionOf(_) => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::Or,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                self.pending_queue.push_back(rule_app);
            }
            ClassExpression::ObjectSomeValuesFrom { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::Some,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                self.pending_queue.push_back(rule_app);
            }
            ClassExpression::ObjectAllValuesFrom { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::All,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::High,
                    dependencies: Arc::new(DependencySet::new()),
                };
                self.pending_queue.push_back(rule_app);
            }
            _ => {
                // Atomic classes and other simple expressions don't need rules
            }
        }

        Ok(())
    }

    /// Run the tableau expansion algorithm
    pub fn expand(&mut self) -> Result<TableauState> {
        // Use the modular executor
        self.state = TableauExecutor::run(self)?;
        Ok(self.state)
    }

    /// Check if the tableau is satisfiable
    pub fn is_satisfiable(&self) -> bool {
        matches!(self.state, TableauState::Satisfiable)
    }

    /// Check if the tableau is unsatisfiable
    pub fn is_unsatisfiable(&self) -> bool {
        matches!(self.state, TableauState::Unsatisfiable)
    }

    /// Get the current state
    pub fn state(&self) -> TableauState {
        self.state
    }

    /// Get the current state (alias for backward compatibility)
    pub fn get_state(&self) -> TableauState {
        self.state
    }

    /// Get tableau statistics
    pub fn statistics(&self) -> &TableauStatistics {
        &self.statistics
    }

    /// Get the nodes in the tableau
    pub fn nodes(&self) -> &[TableauNode] {
        &self.nodes
    }

    /// Get the edges in the tableau
    pub fn edges(&self) -> &[TableauEdge] {
        &self.edges
    }

    /// Get a node by ID
    pub fn node(&self, id: NodeId) -> Option<&TableauNode> {
        self.nodes.get(id)
    }

    /// Get a mutable node by ID
    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut TableauNode> {
        self.nodes.get_mut(id)
    }

    /// Find edges from a specific node
    pub fn edges_from(&self, node_id: NodeId) -> impl Iterator<Item = &TableauEdge> {
        self.edges.iter().filter(move |edge| edge.from == node_id)
    }

    /// Find edges to a specific node
    pub fn edges_to(&self, node_id: NodeId) -> impl Iterator<Item = &TableauEdge> {
        self.edges.iter().filter(move |edge| edge.to == node_id)
    }

    /// Reset the tableau
    pub fn reset(&mut self) {
        self.nodes.clear();
        self.edges.clear();
        self.pending_queue.clear();
        self.clash_detector = ClashDetector::new();
        self.statistics = TableauStatistics::new();
        self.state = TableauState::Unknown;
        self.concept_cache.clear();
        self.role_cache.clear();
        self.individual_map.clear();
    }

    // Legacy methods for compatibility with existing code

    /// Check if tableau is complete
    pub fn is_complete(&self) -> bool {
        self.pending_queue.is_empty() && self.nodes.iter().all(|node| node.status.fully_expanded)
    }

    /// Get clash detector
    pub fn clash_detector(&self) -> &ClashDetector {
        &self.clash_detector
    }

    /// Get mutable clash detector
    pub fn clash_detector_mut(&mut self) -> &mut ClashDetector {
        &mut self.clash_detector
    }

    /// Get pending queue
    pub fn pending_queue(&self) -> &VecDeque<RuleApplication> {
        &self.pending_queue
    }

    /// Get mutable pending queue  
    pub fn pending_queue_mut(&mut self) -> &mut VecDeque<RuleApplication> {
        &mut self.pending_queue
    }

    /// Get config
    pub fn config(&self) -> &TableauConfig {
        &self.config
    }

    // Additional methods for compatibility

    /// Get node count
    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get backtrack count
    pub fn get_backtrack_count(&self) -> usize {
        self.backtrack_stack.len()
    }

    /// Get max depth  
    pub fn get_max_depth(&self) -> usize {
        self.config.max_depth as usize
    }

    /// Run tableau expansion - compatibility wrapper
    pub fn run(&mut self) -> Result<TableauState> {
        self.expand()
    }

    /// Add a node to the tableau
    pub fn add_node(&mut self, node_type: NodeType) -> Result<NodeId> {
        let id = self.nodes.len();
        let node = TableauNode::new(id, node_type);
        self.nodes.push(node);
        self.statistics.increment_nodes();
        Ok(id)
    }

    /// Add an edge between nodes
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, role: RoleLabel) -> Result<()> {
        if from >= self.nodes.len() || to >= self.nodes.len() {
            return Err(Error::reasoning(format!(
                "Invalid node id: {}",
                from.max(to)
            )));
        }

        let edge = TableauEdge::new(from, to, role.clone(), Arc::new(DependencySet::new()));
        self.edges.push(edge);

        // Update node connections
        if let Some(from_node) = self.nodes.get_mut(from) {
            // Update role successors - we'll use the role string as key
            let role_str = role.to_string();
            from_node
                .role_successors
                .entry(role_str)
                .or_insert_with(HashSet::new)
                .insert(to);
        }
        // For predecessors, we'd need a separate tracking mechanism or traverse edges

        self.statistics.increment_edges();
        Ok(())
    }

    /// Add a concept to a node
    pub fn add_concept_to_node(&mut self, node_id: NodeId, concept: ConceptLabel) -> Result<()> {
        if node_id >= self.nodes.len() {
            return Err(Error::reasoning(format!("Invalid node id: {}", node_id)));
        }

        if let Some(node) = self.nodes.get_mut(node_id) {
            node.concepts.insert(concept);
        }

        Ok(())
    }
}

impl std::fmt::Debug for Tableau {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tableau")
            .field("nodes", &self.nodes.len())
            .field("edges", &self.edges.len())
            .field("state", &self.state)
            .field("pending_rules", &self.pending_queue.len())
            .field("backtrack_points", &self.backtrack_stack.len())
            .finish()
    }
}

// Re-export common types for convenience

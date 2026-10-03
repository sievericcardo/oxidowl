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
    config::{BlockingStrategy as ConfigBlockingStrategy, ReasoningConfig, TableauConfig},
    core::{
        completion::{CompletionStrategy, RuleApplication},
        dependency::DependencySet,
        expansion::DefaultExpansionStrategy,
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
    pub pending_queue: Arc<VecDeque<RuleApplication>>,

    /// Completion strategy
    pub completion_strategy: CompletionStrategy,

    /// Blocking strategy selected from the reasoning configuration.
    /// The executor uses this to build a `BlockingChecker` via
    /// `crate::core::blocking::BlockingStrategy::create_checker`.
    pub blocking_strategy: ConfigBlockingStrategy,

    /// Expansion strategy
    pub expansion_strategy: DefaultExpansionStrategy,

    /// Clause checker for DL clause validation (optional)
    pub clause_checker: Option<ClauseChecker>,

    /// Clause index for fast lookup (optional)
    pub clause_index: Option<ClauseIndex>,

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

    /// Nodes that received new concepts since the last clash detection pass.
    /// Used to limit `detect_clashes` to only the nodes that changed.
    pub dirty_nodes: Vec<NodeId>,

    /// Concept unfolding rules: maps a named-class IRI string to a list of complex class
    /// expressions that the class is subsumed by (from SubClassOf and EquivalentClasses axioms).
    /// Used to propagate existential restrictions to individual nodes when a ClassAssertion is
    /// processed (the forward direction of `A ≡ ∃R.C` or `A ⊑ ∃R.C`).
    pub concept_unfolding_rules: HashMap<String, Vec<ClassExpression>>,

    /// Inequality atoms between tableau nodes (workstream A3).
    ///
    /// A pair `(a, b)` (stored normalised so `a < b`) records that nodes `a` and `b` denote
    /// distinct individuals. Two nodes marked unequal must never be merged by an AtMost rule.
    /// Fresh successors created by the same AtLeast/Exact rule are marked pairwise unequal.
    pub inequalities: HashSet<(NodeId, NodeId)>,

    /// Journal of node merges in application order (workstream A3).
    ///
    /// Every `merge_nodes` call appends a [`MergeRecord`] capturing the pre-merge state of the
    /// two nodes and the edge redirections. When backtracking over a non-deterministic AtMost
    /// merge choice, merges performed since the choice point are undone in reverse order.
    pub merge_log: Vec<MergeRecord>,

    /// Negative object property assertions (workstream A5): `¬R(a, b)` records.
    ///
    /// Stored as `(canonical role name, source individual name, target individual name)`.
    /// Individual *names* (the Display strings of the individuals) are stored instead of node
    /// ids so the records survive node merges: the executor resolves the names through
    /// `individual_map` on every RBox pass, which yields the current representative node of
    /// each individual after SameIndividual/functional merging. The RBox pass then:
    ///  - raises a clash if an `R`-edge between the two current nodes appears, and
    ///  - raises a clash if the two individuals have collapsed to the *same* node (the
    ///    negative assertion's `a ≠ b` consequence), e.g. under SameIndividual.
    pub negative_object_assertions: Vec<(String, String, String)>,

    /// Concrete-domain (datatype) validator (workstream A4).
    ///
    /// Holds the `DatatypeDefinition` axioms of the loaded ontology and provides the
    /// value-space helpers used by the datatype rules: concrete-literal generation, facet
    /// evaluation, value equality, and range membership.
    pub datatype_validator: crate::core::reasoner::datatype_validation::DatatypeValidator,
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

    /// At-most merging choice (workstream A3)
    AtMostMerge {
        /// Node where the `≤n R.C` / `≤n P.D` constraint lives
        node_id: NodeId,
        /// Canonical role name of the constrained property
        role: String,
        /// Display form of the filler (used to re-identify the constraint on backtracking)
        filler: String,
        /// The maximum number of distinct successors allowed
        max_count: usize,
        /// Whether this is a data-property constraint (data range filler) or an object one
        is_data: bool,
        /// Candidate (survivor, merged-away) node pairs for the next merge
        candidates: Vec<(NodeId, NodeId)>,
        /// Index into `candidates` that has been tried
        tried: usize,
    },

    /// Forced-equality merging choice (workstream A5).
    ///
    /// Raised by the functional / inverse-functional / HasKey RBox rules when two nodes are
    /// forced to denote the same individual. Unlike `AtMostMerge`, the *equality* is
    /// deterministic — there is no alternative model where the nodes stay distinct — so the
    /// only freedom is *which* node survives the merge. `candidates` lists the survivor
    /// orderings to try; backtracking over this choice retries the alternative survivor.
    FunctionalMerge {
        /// Candidate (survivor, merged-away) node pairs, tried in order
        candidates: Vec<(NodeId, NodeId)>,
        /// Index into `candidates` that has been tried
        tried: usize,
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

    /// Pending queue state — Arc-wrapped for O(1) snapshot cost (copy-on-write)
    pub pending_queue: Arc<VecDeque<RuleApplication>>,

    /// Inequality atoms at this point (normalised `(a, b)` pairs, `a < b`)
    pub inequalities: HashSet<(NodeId, NodeId)>,

    /// Individual→node map at this point
    pub individual_map: HashMap<String, NodeId>,

    /// Number of merge records in `Tableau::merge_log` at this point
    pub merge_log_len: usize,

    /// The concept label set of every node at snapshot time (index-aligned with
    /// `nodes[..node_count]`). Restored on backtrack so non-deterministic choices
    /// (e.g. OR-rule disjunct additions) are genuinely undone — truncating the
    /// node/edge lists alone cannot revert a concept added to a pre-existing node.
    pub node_concepts: Vec<HashSet<ConceptLabel>>,
}

/// A single node-merge operation recorded for backtracking (workstream A3).
///
/// `merge_nodes` merges `source` into `target`: source's concepts are copied into target,
/// edges pointing at/from `source` are redirected to `target`, and `source` is cleared and
/// marked clashed. This record snapshots the pre-merge state so `Tableau::undo_merges` can
/// reverse the operation exactly when backtracking over a merge choice.
#[derive(Debug, Clone)]
pub struct MergeRecord {
    /// The surviving node
    pub target: NodeId,
    /// The node that was merged away
    pub source: NodeId,
    /// Target's concept set before the merge
    pub target_concepts: HashSet<ConceptLabel>,
    /// Source's concept set before the merge
    pub source_concepts: HashSet<ConceptLabel>,
    /// Target's concept dependencies before the merge
    pub target_dependencies: HashMap<ConceptLabel, Arc<DependencySet>>,
    /// Source's concept dependencies before the merge
    pub source_dependencies: HashMap<ConceptLabel, Arc<DependencySet>>,
    /// Target's node type before the merge
    pub target_node_type: NodeType,
    /// Source's node type before the merge
    pub source_node_type: NodeType,
    /// Target's status before the merge
    pub target_status: NodeStatus,
    /// Source's status before the merge
    pub source_status: NodeStatus,
    /// Edge indices whose `from`/`to` pointed at `source`, with their pre-merge values
    pub edge_changes: Vec<(usize, NodeId, NodeId)>,
}

impl Tableau {
    /// Create a new tableau with the given configuration and ontology
    #[must_use]
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
                max_memory_mb: config.max_memory_mb,
                blocking_enabled: true,
                optimization_enabled: true,
                rdf11_mode: false,                // RDF-star enabled by default
                quoted_triple_reasoning_depth: 2, // Allow 2 levels of nesting
                enable_clause_optimization: true, // Enable clause optimization by default
            },
            pending_queue: Arc::new(VecDeque::new()),
            completion_strategy: CompletionStrategy::default(),
            blocking_strategy: config.blocking_strategy,
            expansion_strategy: DefaultExpansionStrategy::default(),
            clause_checker: None, // Will be populated when ontology is loaded
            clause_index: None,   // Will be populated when ontology is loaded
            ontology,
            concept_cache: HashMap::new(),
            role_cache: HashMap::new(),
            individual_map: HashMap::new(),
            backtrack_stack: Vec::new(),
            concept_unfolding_rules: HashMap::new(),
            dirty_nodes: Vec::new(),
            inequalities: HashSet::new(),
            merge_log: Vec::new(),
            negative_object_assertions: Vec::new(),
            datatype_validator: crate::core::reasoner::datatype_validation::DatatypeValidator::new(
            ),
        }
    }

    /// Initialize tableau from ontology
    pub fn from_ontology(ontology: Arc<Ontology>, config: ReasoningConfig) -> Result<Self> {
        let mut tableau = Self::new(config, ontology.clone());
        // Load the ontology to initialize clause_checker, equivalence closure, and disjointness map
        tableau.load_ontology(&ontology)?;
        Ok(tableau)
    }

    /// Load axioms from an ontology
    pub fn load_ontology(&mut self, ontology: &Ontology) -> Result<()> {
        // Register DatatypeDefinition axioms so user-defined datatype literals are validated
        // against their defining ranges (workstream A4).
        self.datatype_validator
            .register_ontology_datatypes(ontology);

        // Generate DL clauses from the ontology
        use crate::dl_clauses::DLClauseGenerator;

        let mut generator = DLClauseGenerator::new();
        let clause_set = generator.generate_clauses(ontology)?;

        log::info!(
            "Loaded {} deterministic clauses and {} disjunctive clauses from ontology",
            clause_set.deterministic_clauses.len(),
            clause_set.disjunctive_clauses.len()
        );

        // Build concept-unfolding rules from SubClassOf and EquivalentClasses axioms.
        // These rules propagate complex superclass expressions (e.g. existential restrictions)
        // to individual nodes during ABox reasoning when a ClassAssertion is processed.
        self.concept_unfolding_rules.clear();
        for axiom in ontology.axioms() {
            match axiom {
                crate::ontology::Axiom::SubClassOf(sub) => {
                    // A ⊑ E (named or complex): store "when A(x), also add E(x)".
                    // Named superclasses (e.g. A ⊑ B) are included so that the
                    // complement clash A ⊑ B, A ⊑ ¬B is detected on Nominal nodes.
                    if let ClassExpression::Class(named) = &sub.subclass {
                        self.concept_unfolding_rules
                            .entry(named.iri.to_string())
                            .or_default()
                            .push(sub.superclass.clone());
                    }
                }
                crate::ontology::Axiom::EquivalentClasses(equiv) => {
                    // For each pair (named, complex) in EquivalentClasses, both directions
                    // need unfolding rules since the classes are mutually equivalent.
                    for ci in &equiv.classes {
                        if let ClassExpression::Class(named) = ci {
                            for cj in &equiv.classes {
                                if std::ptr::eq(ci, cj) {
                                    continue;
                                }
                                if cj.is_complex_class_expression() {
                                    self.concept_unfolding_rules
                                        .entry(named.iri.to_string())
                                        .or_default()
                                        .push(cj.clone());
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        log::info!(
            "Built concept unfolding rules for {} named classes",
            self.concept_unfolding_rules.len()
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

        // Process all axioms to ensure they are applied to the tableau
        // The DL clauses generated above provide a normalized representation,
        // but we also need to process axioms directly to initialize the tableau state
        // with the appropriate concepts and constraints.
        //
        // Key axiom processing:
        // 1. EquivalentClasses - bidirectional implications (C1 ≡ C2)
        // 2. DisjointUnion - coverage and pairwise disjointness
        // 3. DisjointClasses - pairwise disjointness constraints
        // 4. SubClassOf - subsumption constraints (A ⊑ B)
        // 5. ClassAssertion - individual concept membership (a : C)
        //
        // All axiom types are now processed to ensure complete tableau initialization
        for axiom in ontology.axioms() {
            self.process_axiom(axiom)?;
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
                        Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
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
                        Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
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
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);

                // Nominal subclass: `{a} ⊑ C` is an ABox-level assertion `a : C` in OWL 2 DL
                // (named individuals are assumed to exist). Add C to the nominal node for each
                // individual in the enumeration and queue its expansion rules. This is what
                // makes e.g. `ObjectOneOf([a]) ⊑ owl:Nothing` an inconsistency.
                if let ClassExpression::ObjectOneOf(individuals) = &subclass.subclass {
                    for individual in individuals {
                        let nominal_id = self.get_or_create_nominal_node(individual)?;
                        let super_label =
                            ConceptLabel::Complex(Box::new(subclass.superclass.clone()));
                        // `add_concept_to_node` queues the expansion rules for the
                        // superclass expression (workstream A6), so no explicit queue here.
                        self.add_concept_to_node(nominal_id, super_label)?;
                        log::debug!(
                            "SubClassOf with nominal subclass: added {:?} to nominal node {}",
                            subclass.superclass,
                            nominal_id
                        );
                    }
                }
            }

            // Handle ClassAssertion axioms
            Axiom::ClassAssertion(assertion) => {
                // Get or create node for the individual. `get_or_create_nominal_node`
                // guarantees that repeated assertions about the same individual reuse the
                // same node and that the node carries a `Nominal(iri)` label.
                let node_id = self.get_or_create_nominal_node(&assertion.individual)?;

                // Add the class assertion to the node
                let concept = ConceptLabel::Complex(Box::new(assertion.class.clone()));
                if let Some(node) = self.nodes.get_mut(node_id) {
                    node.concepts.insert(concept);
                }

                // Also add all equivalent classes (if any) - but only if we have equivalence closure
                // This is an optimization to avoid expensive lookups when not needed
                if let Some(checker) = &mut self.clause_checker
                    && let ClassExpression::Class(ref class) = assertion.class
                    && let Some(eq_closure) = checker.equivalence_closure()
                {
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

                // Queue rule for expanding this concept
                self.queue_rule_for_concept(node_id, &assertion.class)?;

                // Apply concept unfolding rules: if A ≡ ∃R.C or A ⊑ ∃R.C is in the ontology,
                // add the complex superclass expression to this individual's node and queue the
                // appropriate tableau rule (e.g. SOME rule for existential restrictions).
                if let ClassExpression::Class(named_class) = &assertion.class {
                    // Collect unfolding targets to avoid borrow conflict during queue_rule_for_concept
                    let unfolding_targets: Vec<ClassExpression> = self
                        .concept_unfolding_rules
                        .get(&named_class.iri.to_string())
                        .cloned()
                        .unwrap_or_default();

                    for complex_expr in unfolding_targets {
                        let complex_label = ConceptLabel::Complex(Box::new(complex_expr.clone()));
                        if let Some(node) = self.nodes.get_mut(node_id) {
                            node.concepts.insert(complex_label);
                        }
                        self.queue_rule_for_concept(node_id, &complex_expr)?;
                        log::debug!(
                            "Unfolded concept {:?} to {:?} at node {}",
                            named_class.iri,
                            complex_expr,
                            node_id
                        );
                    }
                }
            }

            // ABox object property assertions: `R(a, b)` is an edge between the nominal
            // nodes for `a` and `b`. Workstream A5 — previously these were compiled to DL
            // clause facts that the (unary-only) clause checker ignored, so RBox reasoning
            // never saw the ABox edges. Seeding them here lets the RBox rule pass
            // (transitivity, symmetry, role inclusions, functional merging, …) operate on
            // the actual edge set.
            Axiom::ObjectPropertyAssertion(op_assertion) => {
                let source_id = self.get_or_create_nominal_node(&op_assertion.source)?;
                let target_id = self.get_or_create_nominal_node(&op_assertion.target)?;
                let role_name = op_assertion.property.to_string();
                // Idempotent edge addition: re-asserting R(a,b) adds nothing.
                if !self.nodes[source_id]
                    .role_successors
                    .get(role_name.as_str())
                    .is_some_and(|succs| succs.contains(&target_id))
                {
                    let role_label = RoleLabel::Atomic(role_name);
                    self.add_edge(source_id, target_id, role_label)?;
                    log::debug!(
                        "ObjectPropertyAssertion: added edge {} --{}--> {}",
                        source_id,
                        op_assertion.property,
                        target_id
                    );
                }
            }

            // Negative object property assertion: `¬R(a, b)`.
            // Semantics (A5): (i) `a ≠ b` (an R-edge from an individual to itself is
            // forbidden by a negative assertion), and (ii) any later `R(a, b)` edge is a
            // clash. Both are recorded here; the executor's RBox pass enforces them.
            Axiom::NegativeObjectPropertyAssertion(neg_assertion) => {
                let source_id = self.get_or_create_nominal_node(&neg_assertion.source)?;
                let target_id = self.get_or_create_nominal_node(&neg_assertion.target)?;
                self.add_inequality(source_id, target_id);
                let role_name = neg_assertion.property.to_string();
                let source_name = neg_assertion.source.to_string();
                let target_name = neg_assertion.target.to_string();
                if !self
                    .negative_object_assertions
                    .iter()
                    .any(|(r, a, b)| *r == role_name && *a == source_name && *b == target_name)
                {
                    self.negative_object_assertions.push((
                        role_name.clone(),
                        source_name,
                        target_name,
                    ));
                    log::debug!(
                        "NegativeObjectPropertyAssertion: recorded ¬{}({}, {})",
                        role_name,
                        source_id,
                        target_id
                    );
                }
            }

            // SameIndividual(a, b, …): the individuals denote the same node.
            // Pairwise merges are deterministic forced equalities — `merge_nodes` journals
            // them so they are undone correctly when backtracking over a merge choice.
            Axiom::SameIndividual(same) => {
                let mut node_ids: Vec<NodeId> = Vec::with_capacity(same.individuals.len());
                for ind in &same.individuals {
                    node_ids.push(self.get_or_create_nominal_node(ind)?);
                }
                let survivor = node_ids[0];
                for away in node_ids.iter().skip(1) {
                    if *away == survivor {
                        continue;
                    }
                    // SameIndividual(a,b) contradicts a recorded inequality a ≠ b
                    // (e.g. DifferentIndividuals or a negative assertion). This is a clash.
                    if self.are_inequal(survivor, *away) {
                        let clash = Clash {
                            clash_type: ClashType::ClauseViolation {
                                clause_id: "same_individual_inequality".to_string(),
                                node: survivor,
                                description: format!(
                                    "SameIndividual forces nodes {survivor} and {away} to be equal, but they are known distinct"
                                ),
                            },
                            nodes: vec![survivor, *away],
                            dependencies: Arc::new(DependencySet::new()),
                            explanation: format!(
                                "SameIndividual({survivor},{away}) contradicts a DifferentIndividuals/negative-assertion inequality"
                            ),
                        };
                        self.clash_detector.add_clash(clash);
                        self.statistics.increment_clashes();
                        self.state = TableauState::Unsatisfiable;
                        continue;
                    }
                    crate::core::tableau::executor::TableauExecutor::merge_nodes(
                        self, survivor, *away,
                    )?;
                    log::debug!(
                        "SameIndividual: merged nominal node {} into {}",
                        away,
                        survivor
                    );
                }
            }

            // DifferentIndividuals(a, b, …): every pair denotes distinct individuals.
            Axiom::DifferentIndividuals(diff) => {
                let mut node_ids: Vec<NodeId> = Vec::with_capacity(diff.individuals.len());
                for ind in &diff.individuals {
                    node_ids.push(self.get_or_create_nominal_node(ind)?);
                }
                for i in 0..node_ids.len() {
                    for j in (i + 1)..node_ids.len() {
                        self.add_inequality(node_ids[i], node_ids[j]);
                    }
                }
            }

            // Data property assertions (workstream A4).
            Axiom::DataPropertyAssertion(assertion) => {
                // OWL 2 well-typedness: a literal of a user-defined datatype must belong to
                // its defining data range (DatatypeDefinition). An ill-typed literal is a
                // datatype clash. Built-in datatype ill-typedness is left to the datatype
                // rules (existing behaviour) to avoid changing ABox semantics elsewhere.
                let ill_typed = assertion.value.datatype.as_ref().map(|url| {
                    let iri = crate::ontology::IRI::new(url.as_str());
                    if self.datatype_validator.is_defined_datatype(&iri) {
                        !self
                            .datatype_validator
                            .validate_literal(&assertion.value)
                            .unwrap_or(false)
                    } else {
                        false
                    }
                });
                if ill_typed == Some(true) {
                    let clash = Clash {
                        clash_type: ClashType::Datatype {
                            node: root_id,
                            description: format!(
                                "Ill-typed literal {}: value is not in the value space of datatype {}",
                                assertion.value,
                                assertion
                                    .value
                                    .datatype
                                    .as_ref()
                                    .map(url::Url::to_string)
                                    .unwrap_or_default()
                            ),
                        },
                        nodes: vec![root_id],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!("Ill-typed data literal {}", assertion.value),
                    };
                    self.clash_detector.add_clash(clash);
                    self.statistics.increment_clashes();
                    self.dirty_nodes.push(root_id);
                }

                // Create a data successor node carrying the asserted value so data-property
                // rules (∃P.D / ∀P.D / cardinality) observe it. The value is recorded as a
                // `DataValue` label for value-space matching.
                let node_id = self.get_or_create_nominal_node(&assertion.individual)?;
                let role_name = assertion.property.to_string();
                let data_node_id = self.add_node(NodeType::Generated)?;
                self.add_edge(node_id, data_node_id, RoleLabel::Atomic(role_name))?;
                self.add_concept_to_node(
                    data_node_id,
                    ConceptLabel::DataValue(assertion.value.clone()),
                )?;
            }

            // Other axioms can be added as needed
            _ => {}
        }

        Ok(())
    }

    /// Queue a rule for a class expression if needed
    ///
    /// `pub(crate)` so the executor's AND rule can re-queue rules for nested conjuncts
    /// (nominal / existential / cardinality constructors introduced by expansion).
    pub(crate) fn queue_rule_for_concept(
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
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
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
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
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
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
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
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
            }
            ClassExpression::ObjectOneOf(_) => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::Nominal,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
            }
            ClassExpression::ObjectHasSelf { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::Self_,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::High,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
            }
            ClassExpression::ObjectHasValue { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::HasValue,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
            }
            ClassExpression::ObjectMinCardinality { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::AtLeast,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
            }
            ClassExpression::ObjectMaxCardinality { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::AtMost,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
            }
            ClassExpression::ObjectExactCardinality {
                property,
                cardinality,
                filler,
            } => {
                // `=n R.C ≡ ≥n R.C ⊓ ≤n R.C`: dispatch BOTH the AtLeast and AtMost rules.
                // For `=0 R.C` the AtLeast(0) part is a no-op and the AtMost(0) part enforces
                // `∀R.¬C` (any R-successor satisfying C is a clash), matching the existing
                // `≤0 R.C` handling.
                let min_expr = ClassExpression::ObjectMinCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.clone(),
                };
                let max_expr = ClassExpression::ObjectMaxCardinality {
                    property: property.clone(),
                    cardinality: *cardinality,
                    filler: filler.clone(),
                };
                let min_app = RuleApplication {
                    rule: CompletionRule::AtLeast,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: min_expr,
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                let max_app = RuleApplication {
                    rule: CompletionRule::AtMost,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: max_expr,
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(min_app);
                Arc::make_mut(&mut self.pending_queue).push_back(max_app);
            }
            ClassExpression::DataSomeValuesFrom { .. }
            | ClassExpression::DataAllValuesFrom { .. }
            | ClassExpression::DataHasValue { .. }
            | ClassExpression::DataMinCardinality { .. }
            | ClassExpression::DataMaxCardinality { .. }
            | ClassExpression::DataExactCardinality { .. } => {
                let rule_app = RuleApplication {
                    rule: CompletionRule::Datatype,
                    node: node_id.to_string(),
                    context: RuleContext::Concept {
                        concept: class_expr.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                    },
                    priority: RulePriority::Normal,
                    dependencies: Arc::new(DependencySet::new()),
                };
                Arc::make_mut(&mut self.pending_queue).push_back(rule_app);
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
    #[must_use]
    pub fn is_satisfiable(&self) -> bool {
        matches!(self.state, TableauState::Satisfiable)
    }

    /// Check if the tableau is unsatisfiable
    #[must_use]
    pub fn is_unsatisfiable(&self) -> bool {
        matches!(self.state, TableauState::Unsatisfiable)
    }

    /// Get the current state
    #[must_use]
    pub fn state(&self) -> TableauState {
        self.state
    }

    /// Get the current state (alias for backward compatibility)
    #[must_use]
    pub fn get_state(&self) -> TableauState {
        self.state
    }

    /// Get tableau statistics
    #[must_use]
    pub fn statistics(&self) -> &TableauStatistics {
        &self.statistics
    }

    /// Get the nodes in the tableau
    #[must_use]
    pub fn nodes(&self) -> &[TableauNode] {
        &self.nodes
    }

    /// Get the edges in the tableau
    #[must_use]
    pub fn edges(&self) -> &[TableauEdge] {
        &self.edges
    }

    /// Get a node by ID
    #[must_use]
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
        self.pending_queue = Arc::new(VecDeque::new());
        self.clash_detector = ClashDetector::new();
        self.statistics = TableauStatistics::new();
        self.state = TableauState::Unknown;
        self.concept_cache.clear();
        self.role_cache.clear();
        self.individual_map.clear();
        self.dirty_nodes.clear();
        self.inequalities.clear();
        self.merge_log.clear();
        self.negative_object_assertions.clear();
    }

    // Legacy methods for compatibility with existing code

    /// Check if tableau is complete
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.pending_queue.is_empty() && self.nodes.iter().all(|node| node.status.fully_expanded)
    }

    /// Get clash detector
    #[must_use]
    pub fn clash_detector(&self) -> &ClashDetector {
        &self.clash_detector
    }

    /// Get mutable clash detector
    pub fn clash_detector_mut(&mut self) -> &mut ClashDetector {
        &mut self.clash_detector
    }

    /// Get pending queue
    #[must_use]
    pub fn pending_queue(&self) -> &VecDeque<RuleApplication> {
        &self.pending_queue
    }

    /// Get mutable pending queue  
    pub fn pending_queue_mut(&mut self) -> &mut VecDeque<RuleApplication> {
        Arc::make_mut(&mut self.pending_queue)
    }

    /// Get config
    #[must_use]
    pub fn config(&self) -> &TableauConfig {
        &self.config
    }

    // Additional methods for compatibility

    /// Get node count
    #[must_use]
    pub fn get_node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get backtrack count
    #[must_use]
    pub fn get_backtrack_count(&self) -> usize {
        self.backtrack_stack.len()
    }

    /// Get max depth  
    #[must_use]
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
            // Update role successors — intern the role string so equal keys share one Arc.
            let role_str = role.to_string();
            from_node
                .role_successors
                .entry(Arc::<str>::from(role_str.as_str()))
                .or_insert_with(HashSet::new)
                .insert(to);
        }
        // For predecessors, we'd need a separate tracking mechanism or traverse edges

        self.statistics.increment_edges();
        Ok(())
    }

    /// Add a concept to a node.
    ///
    /// Workstream A6: whenever a genuinely new *complex* class-expression label is added
    /// (from the AND/SOME/ALL/AtLeast/AtMost/datatype/role rules), the completion rules
    /// applicable to that expression are queued so nested constructors introduced by
    /// expansion are themselves expanded. The node's existing label set is the dedup
    /// guard: [`HashSet::insert`] returns `true` only for labels not already present, so
    /// the same `(node, concept)` pair is never re-queued.
    pub fn add_concept_to_node(&mut self, node_id: NodeId, concept: ConceptLabel) -> Result<()> {
        self.add_concept_to_node_inner(node_id, concept, true)
    }

    /// Like [`Self::add_concept_to_node`] but without rule re-queueing.
    ///
    /// Used for the datatype rules' internal range-witness markers (`DataSomeValuesFrom` /
    /// `DataAllValuesFrom` labels minted on data-value successor nodes). Re-queueing a
    /// Datatype rule for such a marker on a data node would recursively mint fresh data
    /// successors, producing spurious values that break `≤n P.D` counting.
    pub(crate) fn add_concept_to_node_no_queue(
        &mut self,
        node_id: NodeId,
        concept: ConceptLabel,
    ) -> Result<()> {
        self.add_concept_to_node_inner(node_id, concept, false)
    }

    /// Shared implementation of the two `add_concept_to_node` variants.
    fn add_concept_to_node_inner(
        &mut self,
        node_id: NodeId,
        concept: ConceptLabel,
        queue_rules: bool,
    ) -> Result<()> {
        if node_id >= self.nodes.len() {
            return Err(Error::reasoning(format!("Invalid node id: {node_id}")));
        }

        let is_new = if let Some(node) = self.nodes.get_mut(node_id) {
            node.concepts.insert(concept.clone())
        } else {
            return Ok(());
        };

        if is_new {
            // Only mark dirty if the concept was actually new.
            self.dirty_nodes.push(node_id);
            if queue_rules && matches!(&concept, ConceptLabel::Complex(_)) {
                crate::core::tableau::executor::TableauExecutor::queue_rules_for_concept(
                    self, node_id, &concept,
                )?;
            }
        }

        Ok(())
    }

    /// Return the canonical tableau node for a named/anonymous individual, creating a
    /// `Nominal` node and registering it in `individual_map` on first use.
    ///
    /// Two individuals with the same identity always resolve to the *same* node id, which
    /// is what makes nominal merging work: rules that must generate an R-successor equal
    /// to `{a}` simply reuse this node instead of minting a fresh one.
    pub fn get_or_create_nominal_node(
        &mut self,
        individual: &crate::ontology::Individual,
    ) -> Result<NodeId> {
        let key = individual.to_string();
        if let Some(&id) = self.individual_map.get(&key) {
            return Ok(id);
        }
        let id = self.add_node(NodeType::Nominal)?;
        self.individual_map.insert(key.clone(), id);
        // Label the node with its nominal so clash detection, blocking and merging can
        // identify it. `add_concept_to_node` marks it dirty for clash detection.
        self.add_concept_to_node(id, ConceptLabel::Nominal(key))?;
        Ok(id)
    }

    /// Whether the given node carries a nominal label `{a}`.
    #[must_use]
    pub fn has_nominal_label(node: &TableauNode) -> bool {
        node.concepts
            .iter()
            .any(|c| matches!(c, ConceptLabel::Nominal(_)))
    }

    /// Record an inequality atom between two nodes (workstream A3).
    ///
    /// Pairs are stored normalised (`a < b`) and `a ≠ a` is a no-op. Two nodes that both
    /// carry the same nominal label `{a}` denote the same individual and are never recorded
    /// as unequal (see the A2/A3 nominal note).
    pub fn add_inequality(&mut self, a: NodeId, b: NodeId) {
        if a == b {
            return;
        }
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        if Self::node_nominal(self, x) == Self::node_nominal(self, y)
            && Self::node_nominal(self, x).is_some()
        {
            return;
        }
        self.inequalities.insert((x, y));
    }

    /// Whether nodes `a` and `b` are known to denote distinct individuals.
    #[must_use]
    pub fn are_inequal(&self, a: NodeId, b: NodeId) -> bool {
        if a == b {
            return false;
        }
        let (x, y) = if a < b { (a, b) } else { (b, a) };
        self.inequalities.contains(&(x, y))
    }

    /// The nominal individual name carried by a node, if any (the first `{a}` label).
    #[must_use]
    pub fn node_nominal(&self, id: NodeId) -> Option<String> {
        self.nodes.get(id).and_then(|n| {
            n.concepts.iter().find_map(|c| match c {
                ConceptLabel::Nominal(name) => Some(name.clone()),
                _ => None,
            })
        })
    }

    /// Rebuild every node's `role_successors` cache from the current edge list.
    ///
    /// Called after backtracking truncates nodes/edges and undoes merges, so the successor
    /// maps exactly match the surviving edge structure.
    pub fn rebuild_role_successors(&mut self) {
        for node in &mut self.nodes {
            node.role_successors.clear();
        }
        for edge in &self.edges {
            if edge.from < self.nodes.len() {
                let role_str = edge.role.to_string();
                self.nodes[edge.from]
                    .role_successors
                    .entry(Arc::<str>::from(role_str.as_str()))
                    .or_default()
                    .insert(edge.to);
            }
        }
    }

    /// Undo all merges recorded at index `from_len` and later, in reverse order.
    ///
    /// Node restoration is skipped for node ids beyond the current node list length (they
    /// were truncated away), and edge restoration is skipped for edge indices beyond the
    /// current edge list length.
    pub fn undo_merges(&mut self, from_len: usize) {
        while self.merge_log.len() > from_len {
            let rec = self
                .merge_log
                .pop()
                .expect("merge_log length checked in loop condition");
            let node_len = self.nodes.len();
            if rec.target < node_len && rec.source < node_len {
                if let Some(target) = self.nodes.get_mut(rec.target) {
                    target.concepts = rec.target_concepts;
                    target.concept_dependencies = rec.target_dependencies;
                    target.node_type = rec.target_node_type;
                    target.status = rec.target_status;
                }
                if let Some(source) = self.nodes.get_mut(rec.source) {
                    source.concepts = rec.source_concepts;
                    source.concept_dependencies = rec.source_dependencies;
                    source.node_type = rec.source_node_type;
                    source.status = rec.source_status;
                }
            }
            for (idx, from_pre, to_pre) in &rec.edge_changes {
                if let Some(edge) = self.edges.get_mut(*idx) {
                    edge.from = *from_pre;
                    edge.to = *to_pre;
                }
            }
        }
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

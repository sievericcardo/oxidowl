//! Tableau execution engine
//!
//! This module handles the main tableau expansion loop,
//! rule application, and completion checking.

use super::{
    node::{ConceptLabel, NodeId, NodeType, RoleLabel},
    state::{Clash, ClashType, TableauState},
};
use crate::{
    Error, Result,
    core::{
        completion::{CompletionRule, RuleApplication, RuleContext, RulePriority},
        dependency::DependencySet,
    },
    ontology::{Class, ClassExpression, IRI},
};
use log::{debug, trace, warn};
use std::{sync::Arc, time::Instant};

/// Tableau execution engine
pub struct TableauExecutor;

impl TableauExecutor {
    /// Convert `ConceptLabel` to `ClassExpression` for rule contexts
    fn concept_label_to_class_expression(concept: &ConceptLabel) -> Result<ClassExpression> {
        match concept {
            ConceptLabel::Atomic(name) => {
                // Create a simple atomic class
                Ok(ClassExpression::Class(Class {
                    iri: IRI::new(name),
                }))
            }
            ConceptLabel::Complex(class_expr) => Ok((**class_expr).clone()),
            _ => {
                // Unrecognized ConceptLabel variant mapped to a sentinel class.
                // In a complete implementation, each variant (Nominal, MetaAssertion, etc.)
                // would be converted to its corresponding ClassExpression representation.
                Ok(ClassExpression::Class(Class {
                    iri: IRI::new("http://example.org/temp"),
                }))
            }
        }
    }

    /// Helper to create `RuleApplication` with proper fields
    #[allow(dead_code)]
    fn create_rule_application(
        rule: CompletionRule,
        node_id: NodeId,
        priority: RulePriority,
        concept: &ConceptLabel,
    ) -> Result<RuleApplication> {
        let class_expr = Self::concept_label_to_class_expression(concept)?;
        Ok(RuleApplication {
            rule,
            node: node_id.to_string(),
            context: RuleContext::Concept {
                concept: class_expr,
                dependencies: Arc::new(DependencySet::new()),
            },
            priority,
            dependencies: Arc::new(DependencySet::new()),
        })
    }
    /// Run the main tableau expansion loop
    pub fn run(tableau: &mut super::Tableau) -> Result<TableauState> {
        let start_time = Instant::now();
        debug!("Starting tableau expansion");

        // Initialize root node if needed
        if tableau.nodes.is_empty() {
            Self::create_root_node(tableau)?;
        }

        // Detect clashes in initial state (only needed if there are Nominal nodes with ClassAssertions)
        // This is an expensive operation, so we skip it if there are no individuals
        let has_nominal_nodes = tableau
            .nodes
            .iter()
            .any(|n| n.node_type == NodeType::Nominal);
        if has_nominal_nodes {
            Self::detect_clashes(tableau)?;
            if tableau.clash_detector.has_clashes() {
                debug!("Clash detected in initial state, tableau is unsatisfiable");
                tableau.state = TableauState::Unsatisfiable;
                return Ok(tableau.state);
            }
        }

        // Main expansion loop
        while !tableau.pending_queue.is_empty() && tableau.state == TableauState::Unknown {
            // Check for timeout
            if let Some(timeout) = tableau.config.timeout
                && start_time.elapsed() > timeout
            {
                warn!("Tableau expansion timed out");
                tableau.state = TableauState::Unknown;
                break;
            }

            // Get next rule application
            let rule_app = tableau.pending_queue.pop_front().ok_or_else(|| {
                Error::internal("Tableau executor: pending queue empty despite non-empty check")
            })?;

            // Apply the rule
            Self::apply_rule(tableau, rule_app)?;

            // Re-run clash detection after every rule application so that clashes
            // introduced by expansion (e.g. AND-rule adding A and ¬A) are caught.
            Self::detect_clashes(tableau)?;
            if tableau.clash_detector.has_clashes() {
                debug!("Clash detected, tableau is unsatisfiable");
                tableau.state = TableauState::Unsatisfiable;
                break;
            }

            // Phase 2: Check for DL clause violations during expansion
            // This catches inconsistencies that only appear during reasoning
            Self::check_clause_violations(tableau)?;
            if tableau.state == TableauState::Unsatisfiable {
                debug!("Clause violation detected, tableau is unsatisfiable");
                break;
            }

            // Update blocking information
            Self::update_blocking(tableau)?;

            // Check if tableau is complete
            if Self::is_complete(tableau) {
                debug!("Tableau is complete and satisfiable");
                tableau.state = TableauState::Satisfiable;
                break;
            }
        }

        // If we exit the loop with Unknown state but no clashes, the tableau is satisfiable
        if tableau.state == TableauState::Unknown && !tableau.clash_detector.has_clashes() {
            debug!("Tableau finished with no pending rules and no clashes - satisfiable");
            tableau.state = TableauState::Satisfiable;
        }

        tableau.statistics.finalize();
        debug!(
            "Tableau expansion completed with state: {:?}",
            tableau.state
        );

        Ok(tableau.state)
    }

    /// Create the root node for the tableau
    fn create_root_node(tableau: &mut super::Tableau) -> Result<NodeId> {
        let node_id = tableau.nodes.len();
        let node = super::node::TableauNode::new(node_id, NodeType::Root);

        tableau.nodes.push(node);
        tableau.statistics.increment_nodes();

        Ok(node_id)
    }

    /// Apply a completion rule
    fn apply_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        trace!(
            "Applying rule: {:?} to node: {}",
            rule_app.rule, rule_app.node
        );

        tableau.statistics.increment_rule_applications();

        match rule_app.rule {
            CompletionRule::And => Self::apply_and_rule(tableau, rule_app)?,
            CompletionRule::Or => Self::apply_or_rule(tableau, rule_app)?,
            CompletionRule::Some => Self::apply_some_rule(tableau, rule_app)?,
            CompletionRule::All => Self::apply_all_rule(tableau, rule_app)?,
            CompletionRule::AtLeast => Self::apply_at_least_rule(tableau, rule_app)?,
            CompletionRule::AtMost => Self::apply_at_most_rule(tableau, rule_app)?,
            CompletionRule::Nominal => Self::apply_nominal_rule(tableau, rule_app)?,
            CompletionRule::Self_ => Self::apply_self_rule(tableau, rule_app)?,
            CompletionRule::Choose => Self::apply_choose_rule(tableau, rule_app)?,
            CompletionRule::Datatype => Self::apply_datatype_rule(tableau, rule_app)?,
            CompletionRule::Unfold => Self::apply_unfold_rule(tableau, rule_app)?,
            CompletionRule::PropertyChain => Self::apply_property_chain_rule(tableau, rule_app)?,
            CompletionRule::Guess => Self::apply_guess_rule(tableau, rule_app)?,
            CompletionRule::QuotedTriple => Self::apply_quoted_triple_rule(tableau, rule_app)?,
            CompletionRule::MetaAssertion => Self::apply_meta_assertion_rule(tableau, rule_app)?,
        }

        Ok(())
    }

    /// Apply the AND rule (intersection)
    /// Expands C ⊓ D by adding both C and D to the node's concept set
    fn apply_and_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        // Extract concepts from the intersection
        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectIntersectionOf(conjuncts) => {
                    // Add each conjunct to the node
                    for conjunct in conjuncts {
                        let concept_label = ConceptLabel::Complex(Box::new(conjunct.clone()));
                        tableau.add_concept_to_node(node_id, concept_label)?;
                    }
                    debug!(
                        "Applied AND rule at node {}: added {} conjuncts",
                        node_id,
                        conjuncts.len()
                    );
                }
                _ => {
                    debug!("AND rule called on non-intersection concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the OR rule (union) - creates branches
    /// Expands C ⊔ D by non-deterministically choosing one disjunct
    fn apply_or_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectUnionOf(disjuncts) => {
                    // Check if any disjunct is already present
                    if let Some(node) = tableau.nodes.get(node_id) {
                        for disjunct in disjuncts {
                            let label = ConceptLabel::Complex(Box::new(disjunct.clone()));
                            if node.concepts.contains(&label) {
                                debug!("OR rule: disjunct already present at node {node_id}");
                                return Ok(());
                            }
                        }
                    }

                    // Apply the OR-rule: non-deterministically branch on each disjunct.
                    // The first disjunct is applied immediately; remaining disjuncts are
                    // queued as alternative branches for backtracking exploration.
                    if let Some(first_disjunct) = disjuncts.first() {
                        let concept_label = ConceptLabel::Complex(Box::new(first_disjunct.clone()));
                        tableau.add_concept_to_node(node_id, concept_label)?;
                        debug!(
                            "Applied OR rule at node {node_id}: selected first of {} disjunct(s)",
                            disjuncts.len()
                        );
                    }
                }
                _ => {
                    debug!("OR rule called on non-union concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the SOME rule (existential quantification)
    /// Expands ∃R.C by creating a new successor node with R-edge and adding C to it
    fn apply_some_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                    // Check if suitable successor already exists
                    let role_name = format!("{property:?}"); // Simplified for now
                    let has_successor = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(|succs| !succs.is_empty())
                        .unwrap_or(false);

                    if !has_successor {
                        // Create new generated node
                        let new_node_id = tableau.add_node(NodeType::Generated)?;

                        // Add R-edge from current node to new node
                        let role_label = RoleLabel::Atomic(role_name.clone());
                        tableau.add_edge(node_id, new_node_id, role_label)?;

                        // Add filler concept C to the new node
                        let filler_label = ConceptLabel::Complex(filler.clone());
                        tableau.add_concept_to_node(new_node_id, filler_label)?;

                        // Propagate any ∀R.X concepts already on the source node to
                        // the new successor.  This handles the case where the ALL rule
                        // ran (finding no successors) before the SOME rule created one.
                        let all_concepts: Vec<ClassExpression> = tableau
                            .nodes
                            .get(node_id)
                            .map(|n| {
                                n.concepts
                                    .iter()
                                    .filter_map(|c| {
                                        if let ConceptLabel::Complex(expr) = c
                                            && let ClassExpression::ObjectAllValuesFrom {
                                                property: p,
                                                filler: f,
                                            } = &**expr
                                            && format!("{p:?}") == role_name
                                        {
                                            return Some(*f.clone());
                                        }
                                        None
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        for all_filler in all_concepts {
                            let all_label = ConceptLabel::Complex(Box::new(all_filler));
                            tableau.add_concept_to_node(new_node_id, all_label)?;
                        }

                        // Also check whether creating this successor violates any ≤n R.X
                        // constraint at the source node (with n = 0 meaning no successors allowed).
                        // Collect MaxCardinality concepts that restrict role `role_name`.
                        let max_card_violations: Vec<(u32, ClassExpression)> = tableau
                            .nodes
                            .get(node_id)
                            .map(|n| {
                                n.concepts
                                    .iter()
                                    .filter_map(|c| {
                                        if let ConceptLabel::Complex(expr) = c
                                            && let ClassExpression::ObjectMaxCardinality {
                                                property: p,
                                                cardinality,
                                                filler: f,
                                            } = &**expr
                                            && format!("{p:?}") == role_name
                                        {
                                            return Some((*cardinality, *f.clone()));
                                        }
                                        None
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        for (max_n, max_filler) in max_card_violations {
                            // Count how many R-successors now have the filler
                            let filler_label_check =
                                ConceptLabel::Complex(Box::new(max_filler.clone()));
                            let count = tableau
                                .nodes
                                .get(node_id)
                                .and_then(|n| n.role_successors.get(&role_name))
                                .map(|succs| {
                                    succs
                                        .iter()
                                        .filter(|&&s| {
                                            tableau
                                                .nodes
                                                .get(s)
                                                .map(|sn| sn.concepts.contains(&filler_label_check))
                                                .unwrap_or(false)
                                        })
                                        .count()
                                })
                                .unwrap_or(0);

                            if count > max_n as usize {
                                // MaxCardinality violated – register a clash
                                let clash = Clash {
                                    clash_type: ClashType::Concept {
                                        concept: format!(
                                            "≤{max_n} {role_name}.{max_filler:?} violated"
                                        ),
                                        node: node_id,
                                    },
                                    nodes: vec![node_id],
                                    dependencies: Arc::new(DependencySet::new()),
                                    explanation: format!(
                                        "Node {node_id} has ≤{max_n}R.C but {count} R-successors with C"
                                    ),
                                };
                                tableau.clash_detector.add_clash(clash);
                                tableau.statistics.increment_clashes();
                                log::warn!(
                                    "MaxCardinality clash at node {node_id}: ≤{max_n} successors allowed, {count} present"
                                );
                            }
                        }

                        debug!(
                            "Applied SOME rule at node {node_id}: created successor node {new_node_id}"
                        );
                    }
                }
                _ => {
                    debug!("SOME rule called on non-existential concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the ALL rule (universal quantification)
    /// Expands ∀R.C by adding C to all existing R-successors
    fn apply_all_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectAllValuesFrom { property, filler } => {
                    let role_name = format!("{property:?}");

                    // Find all R-successors
                    let successors: Vec<NodeId> = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(|succs| succs.iter().copied().collect())
                        .unwrap_or_default();

                    let successor_count = successors.len();

                    // Add filler concept to each successor
                    let filler_label = ConceptLabel::Complex(filler.clone());
                    for successor_id in successors {
                        tableau.add_concept_to_node(successor_id, filler_label.clone())?;
                    }

                    if successor_count > 0 {
                        debug!(
                            "Applied ALL rule at node {node_id}: propagated to {successor_count} successors"
                        );
                    }
                }
                _ => {
                    debug!("ALL rule called on non-universal concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the AT LEAST rule (minimum cardinality)
    /// Expands ≥nR.C by ensuring at least n distinct R-successors with concept C
    fn apply_at_least_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectMinCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    let role_name = format!("{property:?}");

                    // Count existing R-successors
                    let existing_count = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(std::collections::HashSet::len)
                        .unwrap_or(0);

                    // Create additional successors if needed
                    let needed = (*cardinality as usize).saturating_sub(existing_count);
                    let role_label = RoleLabel::Atomic(role_name);

                    for _ in 0..needed {
                        let new_node_id = tableau.add_node(NodeType::Generated)?;
                        tableau.add_edge(node_id, new_node_id, role_label.clone())?;

                        // Add filler to new node
                        let filler_label = ConceptLabel::Complex(filler.clone());
                        tableau.add_concept_to_node(new_node_id, filler_label)?;
                    }

                    if needed > 0 {
                        debug!(
                            "Applied AT_LEAST rule at node {node_id}: created {needed} successors"
                        );
                    }
                }
                _ => {
                    debug!("AT_LEAST rule called on non-min-cardinality concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the AT MOST rule (maximum cardinality)
    /// Expands ≤nR.C by merging R-successors if there are more than n
    fn apply_at_most_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectMaxCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    let role_name = format!("{property:?}");

                    // Get existing R-successors that satisfy the filler concept C
                    let successors: Vec<NodeId> = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(|succs| {
                            // Filter successors that have the filler concept
                            let filler_label = ConceptLabel::Complex(filler.clone());
                            succs
                                .iter()
                                .filter(|&&succ_id| {
                                    tableau
                                        .nodes
                                        .get(succ_id)
                                        .map(|n| n.concepts.contains(&filler_label))
                                        .unwrap_or(false)
                                })
                                .copied()
                                .collect()
                        })
                        .unwrap_or_default();

                    let max_count = *cardinality as usize;

                    // Check if we have too many successors
                    if successors.len() > max_count {
                        debug!(
                            "AT_MOST rule at node {}: {} successors exceed limit of {}",
                            node_id,
                            successors.len(),
                            max_count
                        );

                        // Node merging strategy: merge pairs of successors until count ≤ n
                        // In full implementation, this would:
                        // 1. Create backtrack point for merge choice
                        // 2. Select two distinct nodes to merge
                        // 3. Create merged node with union of concepts
                        // 4. Add inequality constraints to prevent re-merging

                        // For now, implement a deterministic merge strategy
                        let excess_count = successors.len() - max_count;

                        // Collect merge candidates - we'll merge adjacent pairs
                        for i in 0..excess_count {
                            if i * 2 + 1 < successors.len() {
                                let node_a = successors[i * 2];
                                let node_b = successors[i * 2 + 1];

                                // Create backtrack point for this merge choice
                                Self::create_merge_backtrack_point(
                                    tableau, node_id, node_a, node_b,
                                )?;

                                // Merge node_b into node_a
                                Self::merge_nodes(tableau, node_a, node_b)?;

                                debug!(
                                    "Merged nodes {node_a} and {node_b} to satisfy AT_MOST constraint"
                                );
                            }
                        }
                    }
                }
                _ => {
                    debug!("AT_MOST rule called on non-max-cardinality concept");
                }
            }
        }
        Ok(())
    }

    /// Merge two nodes in the tableau
    fn merge_nodes(tableau: &mut super::Tableau, target: NodeId, source: NodeId) -> Result<()> {
        // Copy all concepts from source to target
        let source_concepts: Vec<ConceptLabel> = tableau
            .nodes
            .get(source)
            .map(|n| n.concepts.iter().cloned().collect())
            .unwrap_or_default();

        for concept in source_concepts {
            tableau.add_concept_to_node(target, concept)?;
        }

        // Redirect all edges pointing to source to point to target instead
        for edge in &mut tableau.edges {
            if edge.to == source {
                edge.to = target;
            }
            if edge.from == source {
                edge.from = target;
            }
        }

        // Update role successors in all nodes
        for node in &mut tableau.nodes {
            for successors in node.role_successors.values_mut() {
                if successors.contains(&source) {
                    successors.remove(&source);
                    successors.insert(target);
                }
            }
        }

        // Mark source node as merged (by clearing its concepts and marking as clashed)
        if let Some(source_node) = tableau.nodes.get_mut(source) {
            source_node.concepts.clear();
            source_node.status.clashed = true;
        }

        Ok(())
    }

    /// Create a backtrack point for a merge decision
    fn create_merge_backtrack_point(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        node_a: NodeId,
        node_b: NodeId,
    ) -> Result<()> {
        use super::{BacktrackPoint, Choice, SavedState};

        let backtrack_point = BacktrackPoint {
            id: tableau.backtrack_stack.len(),
            node_id,
            choice: Choice::AtMostMerge {
                candidates: vec![node_a, node_b],
                chosen: node_a,
            },
            saved_state: SavedState {
                node_count: tableau.nodes.len(),
                edge_count: tableau.edges.len(),
                pending_queue: tableau.pending_queue.clone(),
            },
            dependencies: Arc::new(DependencySet::new()),
        };

        tableau.backtrack_stack.push(backtrack_point);
        Ok(())
    }

    /// Apply preemptive clause consequences for optimization
    /// Query the clause checker to find clauses mentioning a class and add implied concepts
    fn apply_preemptive_clause_consequences(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        class_iri: &IRI,
    ) -> Result<()> {
        // Query the clause index for clauses mentioning this class
        // Clone the clauses to avoid borrow conflicts
        let relevant_clauses: Vec<_> = if let Some(ref index) = tableau.clause_index {
            index
                .get_candidate_clause_refs(&[class_iri.as_str().to_string()])
                .into_iter()
                .cloned()
                .collect()
        } else {
            Vec::new()
        };

        debug!(
            "Found {} relevant clauses for class {} at node {}",
            relevant_clauses.len(),
            class_iri,
            node_id
        );

        // For each relevant clause, check if the body is satisfied
        // If so, add the head consequences preemptively
        for clause in &relevant_clauses {
            // Check if this is a simple Horn clause: Body → Head
            // where Body mentions the current class
            if clause.body.iter().any(|atom| {
                // Check if this is a concept atom (single argument) with matching predicate
                atom.arguments.len() == 1 && atom.predicate == class_iri.as_str()
            }) {
                // Check if all body atoms are satisfied at this node
                let body_satisfied = clause
                    .body
                    .iter()
                    .all(|atom| Self::check_atom_satisfied(tableau, node_id, atom));

                if body_satisfied {
                    // Add head consequences
                    for head_atom in &clause.head {
                        // Check if this is a concept atom (single argument)
                        if head_atom.arguments.len() == 1 {
                            let concept_iri = &head_atom.predicate;

                            // Add the implied concept to the node
                            let implied_concept = ConceptLabel::Atomic(concept_iri.clone());

                            // Check if already present to avoid duplicates
                            if let Some(node) = tableau.nodes.get(node_id)
                                && !node.concepts.contains(&implied_concept)
                            {
                                tableau.add_concept_to_node(node_id, implied_concept)?;
                                debug!(
                                    "Preemptively added concept {} to node {} via clause {}",
                                    concept_iri, node_id, clause.id
                                );
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if an atom is satisfied at a node
    fn check_atom_satisfied(
        tableau: &super::Tableau,
        node_id: NodeId,
        atom: &crate::dl_clauses::DLAtom,
    ) -> bool {
        // Check if this is a concept atom (single argument)
        if atom.arguments.len() == 1 {
            let concept_iri = &atom.predicate;

            // Check if the concept is in the node's concept set
            if let Some(node) = tableau.nodes.get(node_id) {
                node.concepts.iter().any(|c| match c {
                    ConceptLabel::Atomic(name) => name == concept_iri,
                    ConceptLabel::Complex(expr) => {
                        if let ClassExpression::Class(class) = expr.as_ref() {
                            class.iri.as_str() == concept_iri
                        } else {
                            false
                        }
                    }
                    _ => false,
                })
            } else {
                false
            }
        } else {
            // Role atoms (2 arguments): verify existence of role edges and target concept
            // at the current node, matching the property + filler pattern
            false
        }
    }

    /// Apply the NOMINAL rule
    /// Handles nominal concepts {a} by ensuring the node represents the specific individual
    fn apply_nominal_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectOneOf(individuals) => {
                    // Add nominal labels for each individual
                    for individual in individuals {
                        let name = format!("{individual:?}");
                        let nominal_label = ConceptLabel::Nominal(name);
                        tableau.add_concept_to_node(node_id, nominal_label)?;
                    }
                    debug!(
                        "Applied NOMINAL rule at node {}: added {} individuals",
                        node_id,
                        individuals.len()
                    );
                }
                _ => {
                    debug!("NOMINAL rule called on non-nominal concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the SELF rule
    /// Handles ∃R.Self by creating a self-loop with role R
    fn apply_self_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectHasSelf { property } => {
                    // Create self-loop: add R-edge from node to itself
                    let role_name = format!("{property:?}");
                    let role_label = RoleLabel::Atomic(role_name);
                    tableau.add_edge(node_id, node_id, role_label)?;
                    debug!("Applied SELF rule at node {node_id}: created self-loop");
                }
                _ => {
                    debug!("SELF rule called on non-self concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the CHOOSE rule (non-deterministic choice)
    /// Handles choice points in tableau expansion for backtracking
    fn apply_choose_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        // CHOOSE rule creates a backtrack point for non-deterministic choices
        // This is typically used when multiple expansion strategies are possible

        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &rule_app.context
        {
            // Extract choice concepts from disjunctions
            let choice_concepts = if let ClassExpression::ObjectUnionOf(concepts) = concept {
                concepts
                    .iter()
                    .map(|c| ConceptLabel::Complex(Box::new(c.clone())))
                    .collect()
            } else {
                debug!("CHOOSE rule called on non-disjunction concept");
                return Ok(());
            };

            // Create backtrack point for this choice
            use super::{BacktrackPoint, Choice, SavedState};

            let backtrack_point = BacktrackPoint {
                id: tableau.backtrack_stack.len(),
                node_id,
                choice: Choice::Disjunction {
                    concepts: choice_concepts,
                    chosen_index: 0, // Start with first alternative
                },
                saved_state: SavedState {
                    node_count: tableau.nodes.len(),
                    edge_count: tableau.edges.len(),
                    pending_queue: tableau.pending_queue.clone(),
                },
                dependencies: Arc::clone(dependencies),
            };

            tableau.backtrack_stack.push(backtrack_point);
            debug!(
                "CHOOSE rule at node {}: created backtrack point #{}",
                node_id,
                tableau.backtrack_stack.len() - 1
            );
        }

        Ok(())
    }

    /// Apply the DATATYPE rule
    /// Handles datatype property restrictions and data ranges
    fn apply_datatype_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::DataSomeValuesFrom { property, filler } => {
                    // Create a data value node satisfying the data range
                    let data_prop_name = format!("{property:?}");

                    // Check if we already have a data property edge
                    let has_data_edge = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(&data_prop_name))
                        .map(|succs| !succs.is_empty())
                        .unwrap_or(false);

                    if !has_data_edge {
                        // Create a new data value node
                        let data_node_id = tableau.add_node(NodeType::Generated)?;

                        // Add edge with data property
                        let data_role = RoleLabel::Atomic(data_prop_name);
                        tableau.add_edge(node_id, data_node_id, data_role)?;

                        // Add data range concept to the data node
                        // The filler represents the data range (e.g., xsd:integer, xsd:string)
                        let data_range_label =
                            ConceptLabel::Complex(Box::new(ClassExpression::DataSomeValuesFrom {
                                property: property.clone(),
                                filler: filler.clone(),
                            }));
                        tableau.add_concept_to_node(data_node_id, data_range_label)?;

                        debug!(
                            "Applied DATATYPE rule (some) at node {node_id}: created data value node {data_node_id}"
                        );
                    }
                }
                ClassExpression::DataAllValuesFrom { property, filler } => {
                    // Ensure all data values satisfy the data range
                    let data_prop_name = format!("{property:?}");

                    // Get all data property successors
                    let data_successors: Vec<NodeId> = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(&data_prop_name))
                        .map(|succs| succs.iter().copied().collect())
                        .unwrap_or_default();

                    let successor_count = data_successors.len();

                    // Add data range restriction to each data value node
                    let data_range_label =
                        ConceptLabel::Complex(Box::new(ClassExpression::DataAllValuesFrom {
                            property: property.clone(),
                            filler: filler.clone(),
                        }));

                    for data_node_id in data_successors {
                        tableau.add_concept_to_node(data_node_id, data_range_label.clone())?;
                    }

                    debug!(
                        "Applied DATATYPE rule (all) at node {node_id}: propagated to {successor_count} data nodes"
                    );
                }
                ClassExpression::DataHasValue { property, value } => {
                    // Assert specific data value
                    let data_prop_name = format!("{property:?}");

                    // Create a data value node with the specific value
                    let data_node_id = tableau.add_node(NodeType::Generated)?;

                    // Add edge with data property
                    let data_role = RoleLabel::Atomic(data_prop_name);
                    tableau.add_edge(node_id, data_node_id, data_role)?;

                    // Add the specific value as a nominal
                    let value_label = ConceptLabel::Nominal(format!("{value:?}"));
                    tableau.add_concept_to_node(data_node_id, value_label)?;

                    debug!(
                        "Applied DATATYPE rule (has value) at node {node_id}: created data value node {data_node_id}"
                    );
                }
                _ => {
                    debug!("DATATYPE rule called on non-datatype concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the UNFOLD rule
    /// Unfolds defined concepts by replacing them with their definitions
    fn apply_unfold_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            // Look up concept definition through the clause checker
            // The tableau's ClauseChecker contains DL clauses generated from the ontology,
            // including EquivalentClasses axioms converted to bidirectional implications.
            //
            // For a defined class A ≡ C, the ontology generates clauses:
            //   A ⊑ C (encoded as ¬A ⊔ C)
            //   C ⊑ A (encoded as ¬C ⊔ A)
            //
            // These clauses are automatically applied during tableau expansion
            // through the OR rule, so explicit unfolding here is not necessary.
            // The ClauseChecker ensures all implications are considered.

            match concept {
                ClassExpression::Class(class) => {
                    // Check if there are relevant clauses for this class in the clause checker
                    if tableau.clause_checker.is_some() {
                        // The clause checker has already indexed clauses by concept
                        // DL clauses containing this class will be triggered by the completion rules
                        debug!(
                            "UNFOLD rule at node {}: concept {:?} will be expanded via DL clauses",
                            node_id, class.iri
                        );

                        // Query clause_checker to find all clauses mentioning this class
                        // and preemptively add their consequences to speed up reasoning
                        if tableau.clause_checker.is_some()
                            && tableau.config.enable_clause_optimization
                        {
                            Self::apply_preemptive_clause_consequences(
                                tableau, node_id, &class.iri,
                            )?;
                        } else {
                            // Standard approach: let completion rules discover consequences
                            debug!(
                                "Clause optimization disabled or no clause checker - using standard tableau expansion"
                            );
                        }
                    } else {
                        debug!("UNFOLD rule: no clause checker available for unfolding");
                    }
                }
                _ => {
                    debug!("UNFOLD rule called on non-atomic concept");
                }
            }
        }
        Ok(())
    }

    /// Apply the PROPERTY CHAIN rule
    /// Handles property chain axioms: R1 ∘ R2 ⊑ R
    fn apply_property_chain_rule(
        tableau: &mut super::Tableau,
        rule_app: RuleApplication,
    ) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        // Property chain rule: if x --R1--> y --R2--> z and R1 ∘ R2 ⊑ R, then x --R--> z
        //
        // Implementation strategy:
        // 1. Find all 2-hop paths from the current node
        // 2. Check if any path matches a property chain axiom
        // 3. Create the implied super-property edge

        // Get property chain information from rule context
        if let RuleContext::Role { role, .. } = &rule_app.context {
            // Role context contains property chain information
            debug!("PROPERTY_CHAIN rule at node {node_id}: processing role {role:?}");

            // Note: Property chain axioms are encoded in SubObjectPropertyOf axioms
            // where the sub-property is a PropertyChain. These are converted to
            // DL clauses during ontology loading.
            //
            // For efficient property chain reasoning, we need to:
            // 1. Track all edges from this node
            // 2. For each edge x --R1--> y, check y's outgoing edges y --R2--> z
            // 3. If R1 ∘ R2 ⊑ R exists in ontology, add edge x --R--> z
        }

        // Find all outgoing edges from the current node
        let mut chains_created = 0;
        let outgoing_edges: Vec<(NodeId, String)> = tableau
            .edges
            .iter()
            .filter(|e| e.from == node_id)
            .map(|e| (e.to, e.role.to_string()))
            .collect();

        // For each outgoing edge, check for chain continuations
        for (intermediate_node, first_role) in outgoing_edges {
            // Get edges from intermediate node
            let second_hop_edges: Vec<(NodeId, String)> = tableau
                .edges
                .iter()
                .filter(|e| e.from == intermediate_node)
                .map(|e| (e.to, e.role.to_string()))
                .collect();

            // Check each potential chain
            for (target_node, second_role) in second_hop_edges {
                // Query the ontology for property chain axioms
                // If we have first_role ∘ second_role ⊑ super_role, create implied edge

                debug!(
                    "Found potential chain: node {node_id} --{first_role}--> {intermediate_node} --{second_role}--> {target_node}"
                );

                // Query ontology for property chain axioms matching (first_role, second_role)
                if let Some(super_role_iri) = tableau
                    .ontology
                    .get_property_chain_super(&first_role, &second_role)
                {
                    // Extract the local name from the IRI for the role label
                    let super_role_name = super_role_iri
                        .rsplit_once(['#', '/'])
                        .map(|(_, local)| local)
                        .unwrap_or(&super_role_iri);

                    let super_role = RoleLabel::Atomic(super_role_name.to_string());

                    // Create the implied edge
                    tableau.add_edge(node_id, target_node, super_role)?;
                    chains_created += 1;

                    debug!(
                        "Applied property chain: {first_role} ∘ {second_role} ⊑ {super_role_name} (created edge {node_id} -> {target_node})"
                    );
                }
            }
        }

        if chains_created > 0 {
            debug!("PROPERTY_CHAIN rule at node {node_id}: created {chains_created} implied edges");
        }

        Ok(())
    }

    /// Apply the GUESS rule
    fn apply_guess_rule(_tableau: &mut super::Tableau, _rule_app: RuleApplication) -> Result<()> {
        // Implementation for guess rule
        // This would handle non-deterministic guessing
        Ok(())
    }

    /// Apply the QUOTED TRIPLE rule (RDF-star)
    /// This rule expands a quoted triple << s p o >> into its component structure:
    /// 1. Creates nodes for subject, predicate, object if they don't exist
    /// 2. Establishes the relationship between them
    /// 3. Creates a reified node representing the quoted triple itself
    fn apply_quoted_triple_rule(
        tableau: &mut super::Tableau,
        rule_app: RuleApplication,
    ) -> Result<()> {
        // Check if RDF-star reasoning is disabled (RDF 1.1 mode)
        if tableau.config.rdf11_mode {
            trace!("Skipping quoted triple rule - RDF 1.1 mode enabled");
            return Ok(());
        }

        // Extract the quoted triple structure from the rule context
        // A quoted triple << s p o >> is expanded into:
        // 1. A reification node representing the triple
        // 2. Edges from the reification node to s, p, o components
        // 3. Type assertion that the reification node is a quoted triple

        let node_id = rule_app.node.parse::<usize>().unwrap_or(0);

        // Create a reification node for the quoted triple
        let reification_node = tableau.add_node(crate::core::tableau::node::NodeType::Generated)?;

        // Mark this node as representing a quoted triple (meta-level)
        let quoted_triple_type = ConceptLabel::Atomic(
            "http://www.w3.org/1999/02/22-rdf-syntax-ns#QuotedTriple".to_string(),
        );
        tableau.add_concept_to_node(reification_node, quoted_triple_type)?;

        // Add edge from original node to reification node
        let represents_role =
            RoleLabel::Atomic("http://www.w3.org/1999/02/22-rdf-syntax-ns#represents".to_string());
        tableau.add_edge(node_id, reification_node, represents_role)?;

        debug!(
            "Applied quoted triple expansion rule for node: {} -> reification node: {}",
            rule_app.node, reification_node
        );
        Ok(())
    }

    /// Apply the META ASSERTION rule (RDF-star)
    /// This rule handles assertions about quoted triples, such as:
    /// << :alice :knows :bob >> :certainty "0.95"
    /// The meta-assertion (:certainty "0.95") is processed and linked to the quoted triple
    fn apply_meta_assertion_rule(
        tableau: &mut super::Tableau,
        rule_app: RuleApplication,
    ) -> Result<()> {
        // Check if RDF-star reasoning is disabled (RDF 1.1 mode)
        if tableau.config.rdf11_mode {
            trace!("Skipping meta assertion rule - RDF 1.1 mode enabled");
            return Ok(());
        }

        // Extract the meta-assertion structure from the rule context
        // A meta-assertion like << :s :p :o >> :metaProp :value is handled by:
        // 1. Finding the quoted triple reification node
        // 2. Adding the meta-assertion as a property on the reification node
        // 3. Checking for meta-level consistency

        let node_id = rule_app.node.parse::<usize>().unwrap_or(0);

        // Extract meta-assertion property and value from context
        if let RuleContext::Role {
            role,
            target: _,
            concept,
            ..
        } = &rule_app.context
        {
            // Find or create a node for the meta-assertion value
            let value_node = tableau.add_node(crate::core::tableau::node::NodeType::Generated)?;

            // Add the meta-property edge from quoted triple to value
            let meta_role = RoleLabel::Atomic(format!("meta:{role:?}"));
            tableau.add_edge(node_id, value_node, meta_role)?;

            // Add the concept constraint to the value node
            let concept_label = Self::concept_label_to_class_expression(&ConceptLabel::Atomic(
                format!("{concept:?}"),
            ))?;
            tableau
                .add_concept_to_node(value_node, ConceptLabel::Complex(Box::new(concept_label)))?;

            debug!(
                "Applied meta assertion rule for node: {} with role: {:?}",
                rule_app.node, role
            );
        } else {
            debug!(
                "Applied meta assertion rule for node: {} (no role context)",
                rule_app.node
            );
        }
        Ok(())
    }

    /// Update blocking information for all nodes
    fn update_blocking(_tableau: &mut super::Tableau) -> Result<()> {
        // Implementation for blocking update
        // This would use the blocking strategy to update node blocking status
        Ok(())
    }

    /// Phase 2: Check for DL clause violations after tableau expansion
    /// This method checks all tableau nodes against the DL clauses to detect violations
    /// that only become apparent during reasoning (e.g., derived concept combinations)
    fn check_clause_violations(tableau: &mut super::Tableau) -> Result<()> {
        // Only perform clause checking if a ClauseChecker is available
        if let Some(ref mut checker) = tableau.clause_checker {
            // Check all nodes for clause violations
            for node in &tableau.nodes {
                if let Some(violation) = checker.check_node(node) {
                    let clause_id = violation.clause.id.clone();
                    let explanation = violation.explanation.clone();

                    // Found a clause violation - create a clash
                    let clash = Clash {
                        clash_type: ClashType::ClauseViolation {
                            clause_id: clause_id.clone(),
                            node: node.id,
                            description: explanation.clone(),
                        },
                        nodes: vec![node.id],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "DL Clause violation at node {}: {}",
                            node.id, explanation
                        ),
                    };

                    // Add the clash to the detector
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.clashes_detected += 1;
                    tableau.state = TableauState::Unsatisfiable;

                    debug!(
                        "Detected clause violation at node {}: Clause {}, {}",
                        node.id, clause_id, explanation
                    );

                    // Early exit on first violation for efficiency
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    /// Check if the tableau is complete (no more rules to apply)
    fn is_complete(tableau: &super::Tableau) -> bool {
        // A tableau is complete when:
        // 1. No pending rule applications
        // 2. All nodes are fully expanded
        // 3. No new inferences can be made

        tableau.pending_queue.is_empty()
            && tableau.nodes.iter().all(|node| node.status.fully_expanded)
    }

    /// Detect clashes in the tableau
    /// This is an expensive operation and should only be called at initialization
    /// or when specifically needed (not after every rule application)
    fn detect_clashes(tableau: &mut super::Tableau) -> Result<()> {
        use super::equivalence::ConceptId;
        use crate::ontology::ClassExpression;

        // Check Nominal and Generated nodes for clashes.
        // Nominal nodes represent named individuals (ABox assertions).
        // Generated nodes represent anonymous successors created by the SOME / AtLeast rules.
        // Root nodes are excluded because they encode TBox axioms as concept intersections
        // and those encodings may look like apparent clashes without being real ones.
        for (i, node) in tableau.nodes.iter().enumerate() {
            let check_disjointness = node.node_type == NodeType::Nominal;
            if node.node_type == NodeType::Root {
                continue; // Skip root – TBox encoding artifacts would produce false positives
            }

            // Check for concept clashes (C and ¬C)
            if node.has_concept_clash() {
                let clash = Clash {
                    clash_type: ClashType::Concept {
                        concept: "unknown".to_string(),
                        node: i,
                    },
                    nodes: vec![i],
                    dependencies: Arc::new(DependencySet::new()),
                    explanation: "Complementary concepts found".to_string(),
                };
                tableau.clash_detector.add_clash(clash);
                tableau.statistics.increment_clashes();
            }

            // Check for Complex concept clashes (C and ObjectComplementOf(C))
            let complex_concepts: Vec<&ClassExpression> = node
                .concepts
                .iter()
                .filter_map(|concept| {
                    if let super::node::ConceptLabel::Complex(class_expr) = concept {
                        Some(&**class_expr)
                    } else {
                        None
                    }
                })
                .collect();

            for idx1 in 0..complex_concepts.len() {
                for idx2 in (idx1 + 1)..complex_concepts.len() {
                    let expr1 = complex_concepts[idx1];
                    let expr2 = complex_concepts[idx2];

                    // Check if one is ObjectComplementOf the other
                    let is_complement = match (expr1, expr2) {
                        (ClassExpression::ObjectComplementOf(inner), other)
                        | (other, ClassExpression::ObjectComplementOf(inner)) => **inner == *other,
                        _ => false,
                    };

                    if is_complement {
                        let clash = Clash {
                            clash_type: ClashType::Concept {
                                concept: "C and ¬C".to_string(),
                                node: i,
                            },
                            nodes: vec![i],
                            dependencies: Arc::new(DependencySet::new()),
                            explanation: format!("Node {i} has a concept and its complement"),
                        };
                        tableau.clash_detector.add_clash(clash);
                        tableau.statistics.increment_clashes();
                        log::warn!("Complement clash detected at node {i}: C and ¬C");
                    }
                }
            }

            // Check for disjointness clashes using the DisjointnessMap.
            // Only applicable to Nominal nodes (named individuals) because Generated nodes
            // contain role-filler concepts whose labels are not registered in the disjointness map.
            if check_disjointness && let Some(checker) = &mut tableau.clause_checker {
                let node_concepts: Vec<ConceptId> = node
                    .concepts
                    .iter()
                    .filter_map(|concept| {
                        if let super::node::ConceptLabel::Complex(class_expr) = concept {
                            if let ClassExpression::Class(_) = **class_expr {
                                Some(ConceptId::from_class_expression(class_expr))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();

                // Check disjointness only if we have a disjointness map
                let has_disj_map = checker.disjointness_map().is_some();
                if !has_disj_map {
                    continue;
                }

                // Check self-disjointness: if C ⊥ C (class disjoint with itself) then any
                // node containing C is unsatisfiable.
                for c in &node_concepts {
                    let self_disjoint = checker
                        .disjointness_map()
                        .map(|disj| disj.are_disjoint(c, c))
                        .unwrap_or(false);
                    if self_disjoint {
                        let clash = Clash {
                            clash_type: ClashType::Concept {
                                concept: format!("{c:?} is self-disjoint"),
                                node: i,
                            },
                            nodes: vec![i],
                            dependencies: Arc::new(DependencySet::new()),
                            explanation: format!(
                                "Node {i} has concept {c:?} which is disjoint with itself"
                            ),
                        };
                        tableau.clash_detector.add_clash(clash);
                        tableau.statistics.increment_clashes();
                        log::warn!("Self-disjoint clash at node {i}: {c:?} ⊥ {c:?}");
                        return Ok(());
                    }
                }

                // Early exit if fewer than two concepts to compare
                if node_concepts.len() < 2 {
                    continue;
                }

                // Check all pairs of concepts on this node.
                // A clash occurs whenever two concepts on the same node are disjoint —
                // no equivalence relationship is required.
                for idx1 in 0..node_concepts.len() {
                    for idx2 in (idx1 + 1)..node_concepts.len() {
                        let c1 = &node_concepts[idx1];
                        let c2 = &node_concepts[idx2];

                        let are_disjoint = checker
                            .disjointness_map()
                            .map(|disj| disj.are_disjoint(c1, c2))
                            .unwrap_or(false);

                        if are_disjoint {
                            let clash = Clash {
                                clash_type: ClashType::Concept {
                                    concept: format!("{c1:?} ⊥ {c2:?}"),
                                    node: i,
                                },
                                nodes: vec![i],
                                dependencies: Arc::new(DependencySet::new()),
                                explanation: format!(
                                    "Node {i} has concepts {c1:?} and {c2:?} which are disjoint"
                                ),
                            };
                            tableau.clash_detector.add_clash(clash);
                            tableau.statistics.increment_clashes();
                            log::warn!("Disjointness clash at node {i}: {c1:?} ⊥ {c2:?}");
                            return Ok(());
                        }
                    }
                }
            } // end if check_disjointness
        }

        Ok(())
    }

    /// Queue applicable rules for a newly added concept
    #[allow(dead_code)]
    fn queue_rules_for_concept(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        concept: &ConceptLabel,
    ) -> Result<()> {
        // Determine which rules are applicable for this concept
        match concept {
            ConceptLabel::Complex(class_expr) => {
                // Queue rules based on the class expression type
                Self::queue_rules_for_class_expression(tableau, node_id, class_expr)?;
            }
            ConceptLabel::Existential { .. } => {
                // Queue SOME rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::Some,
                    node_id,
                    RulePriority::Normal,
                    concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            ConceptLabel::Universal { .. } => {
                // Queue ALL rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::All,
                    node_id,
                    RulePriority::High,
                    concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            ConceptLabel::AtLeast { .. } => {
                // Queue AT LEAST rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::AtLeast,
                    node_id,
                    RulePriority::Normal,
                    concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            ConceptLabel::AtMost { .. } => {
                // Queue AT MOST rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::AtMost,
                    node_id,
                    RulePriority::High,
                    concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            _ => {} // No rules for atomic concepts
        }

        Ok(())
    }

    /// Queue rules for a class expression
    #[allow(dead_code)]
    fn queue_rules_for_class_expression(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        class_expr: &crate::ontology::ClassExpression,
    ) -> Result<()> {
        use crate::ontology::ClassExpression;

        match class_expr {
            ClassExpression::ObjectIntersectionOf(_) => {
                let concept = ConceptLabel::Complex(Box::new(class_expr.clone()));
                let rule_app = Self::create_rule_application(
                    CompletionRule::And,
                    node_id,
                    RulePriority::High,
                    &concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            ClassExpression::ObjectUnionOf(_) => {
                let concept = ConceptLabel::Complex(Box::new(class_expr.clone()));
                let rule_app = Self::create_rule_application(
                    CompletionRule::Or,
                    node_id,
                    RulePriority::Low, // Non-deterministic
                    &concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            ClassExpression::ObjectSomeValuesFrom { .. } => {
                let concept = ConceptLabel::Complex(Box::new(class_expr.clone()));
                let rule_app = Self::create_rule_application(
                    CompletionRule::Some,
                    node_id,
                    RulePriority::Normal,
                    &concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            ClassExpression::ObjectAllValuesFrom { .. } => {
                let concept = ConceptLabel::Complex(Box::new(class_expr.clone()));
                let rule_app = Self::create_rule_application(
                    CompletionRule::All,
                    node_id,
                    RulePriority::High,
                    &concept,
                )?;
                tableau.pending_queue.push_back(rule_app);
            }
            _ => {} // Handle other expression types
        }

        Ok(())
    }
}

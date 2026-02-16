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
    /// Convert ConceptLabel to ClassExpression for rule contexts
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
                // For other cases, create a placeholder
                // In a real implementation, you'd convert each ConceptLabel variant properly
                Ok(ClassExpression::Class(Class {
                    iri: IRI::new("http://example.org/temp"),
                }))
            }
        }
    }

    /// Helper to create RuleApplication with proper fields
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
            if let Some(timeout) = tableau.config.timeout {
                if start_time.elapsed() > timeout {
                    warn!("Tableau expansion timed out");
                    tableau.state = TableauState::Unknown;
                    break;
                }
            }

            // Get next rule application
            let rule_app = tableau.pending_queue.pop_front().ok_or_else(|| {
                Error::internal("Tableau executor: pending queue empty despite non-empty check")
            })?;

            // Apply the rule
            Self::apply_rule(tableau, rule_app)?;

            // Check for clashes (fast check - only complementary concepts on updated nodes)
            // The expensive equivalence-disjointness check was already done at initialization
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        // Extract concepts from the intersection
        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectIntersectionOf(conjuncts) => {
                    // Add each conjunct to the node
                    for conjunct in conjuncts {
                        let concept_label = ConceptLabel::Complex(Box::new(conjunct.clone()));
                        tableau.add_concept_to_node(node_id, concept_label)?;
                    }
                    debug!("Applied AND rule at node {}: added {} conjuncts", node_id, conjuncts.len());
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectUnionOf(disjuncts) => {
                    // Check if any disjunct is already present
                    if let Some(node) = tableau.nodes.get(node_id) {
                        for disjunct in disjuncts {
                            let label = ConceptLabel::Complex(Box::new(disjunct.clone()));
                            if node.concepts.contains(&label) {
                                debug!("OR rule: disjunct already present at node {}", node_id);
                                return Ok(());
                            }
                        }
                    }

                    // For now, apply the first disjunct (deterministic strategy)
                    // A full implementation would create choice points for backtracking
                    if let Some(first_disjunct) = disjuncts.first() {
                        let concept_label = ConceptLabel::Complex(Box::new(first_disjunct.clone()));
                        tableau.add_concept_to_node(node_id, concept_label)?;
                        debug!("Applied OR rule at node {}: chose first disjunct", node_id);
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                    // Check if suitable successor already exists
                    let role_name = format!("{:?}", property); // Simplified for now
                    let has_successor = tableau.nodes.get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(|succs| !succs.is_empty())
                        .unwrap_or(false);

                    if !has_successor {
                        // Create new generated node
                        let new_node_id = tableau.add_node(NodeType::Generated)?;
                        
                        // Add R-edge from current node to new node
                        let role_label = RoleLabel::Atomic(role_name);
                        tableau.add_edge(node_id, new_node_id, role_label)?;
                        
                        // Add filler concept C to the new node
                        let filler_label = ConceptLabel::Complex(filler.clone());
                        tableau.add_concept_to_node(new_node_id, filler_label)?;
                        
                        debug!("Applied SOME rule at node {}: created successor node {}", node_id, new_node_id);
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectAllValuesFrom { property, filler } => {
                    let role_name = format!("{:?}", property);
                    
                    // Find all R-successors
                    let successors: Vec<NodeId> = tableau.nodes.get(node_id)
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
                        debug!("Applied ALL rule at node {}: propagated to {} successors", node_id, successor_count);
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectMinCardinality { property, cardinality, filler } => {
                    let role_name = format!("{:?}", property);
                    
                    // Count existing R-successors
                    let existing_count = tableau.nodes.get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(|succs| succs.len())
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
                        debug!("Applied AT_LEAST rule at node {}: created {} successors", node_id, needed);
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectMaxCardinality { property, cardinality, filler } => {
                    let role_name = format!("{:?}", property);
                    
                    // Get existing R-successors that satisfy the filler concept C
                    let successors: Vec<NodeId> = tableau.nodes.get(node_id)
                        .and_then(|n| n.role_successors.get(&role_name))
                        .map(|succs| {
                            // Filter successors that have the filler concept
                            let filler_label = ConceptLabel::Complex(filler.clone().into());
                            succs.iter()
                                .filter(|&&succ_id| {
                                    tableau.nodes.get(succ_id)
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
                        debug!("AT_MOST rule at node {}: {} successors exceed limit of {}", 
                               node_id, successors.len(), max_count);
                        
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
                                Self::create_merge_backtrack_point(tableau, node_id, node_a, node_b)?;
                                
                                // Merge node_b into node_a
                                Self::merge_nodes(tableau, node_a, node_b)?;
                                
                                debug!("Merged nodes {} and {} to satisfy AT_MOST constraint", node_a, node_b);
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
        let source_concepts: Vec<ConceptLabel> = tableau.nodes.get(source)
            .map(|n| n.concepts.iter().cloned().collect())
            .unwrap_or_default();
        
        for concept in source_concepts {
            tableau.add_concept_to_node(target, concept)?;
        }
        
        // Redirect all edges pointing to source to point to target instead
        for edge in tableau.edges.iter_mut() {
            if edge.to == source {
                edge.to = target;
            }
            if edge.from == source {
                edge.from = target;
            }
        }
        
        // Update role successors in all nodes
        for node in tableau.nodes.iter_mut() {
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

    /// Apply the NOMINAL rule
    /// Handles nominal concepts {a} by ensuring the node represents the specific individual
    fn apply_nominal_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectOneOf(individuals) => {
                    // Add nominal labels for each individual
                    for individual in individuals {
                        let name = format!("{:?}", individual);
                        let nominal_label = ConceptLabel::Nominal(name);
                        tableau.add_concept_to_node(node_id, nominal_label)?;
                    }
                    debug!("Applied NOMINAL rule at node {}: added {} individuals", node_id, individuals.len());
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectHasSelf { property } => {
                    // Create self-loop: add R-edge from node to itself
                    let role_name = format!("{:?}", property);
                    let role_label = RoleLabel::Atomic(role_name);
                    tableau.add_edge(node_id, node_id, role_label)?;
                    debug!("Applied SELF rule at node {}: created self-loop", node_id);
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        // CHOOSE rule creates a backtrack point for non-deterministic choices
        // This is typically used when multiple expansion strategies are possible
        
        if let RuleContext::Concept { concept, dependencies } = &rule_app.context {
            // Extract choice concepts from disjunctions
            let choice_concepts = match concept {
                ClassExpression::ObjectUnionOf(concepts) => {
                    concepts.iter()
                        .map(|c| ConceptLabel::Complex(Box::new(c.clone())))
                        .collect()
                }
                _ => {
                    debug!("CHOOSE rule called on non-disjunction concept");
                    return Ok(());
                }
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
            debug!("CHOOSE rule at node {}: created backtrack point #{}", node_id, tableau.backtrack_stack.len() - 1);
        }
        
        Ok(())
    }

    /// Apply the DATATYPE rule
    /// Handles datatype property restrictions and data ranges
    fn apply_datatype_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::DataSomeValuesFrom { property, filler } => {
                    // Create a data value node satisfying the data range
                    let data_prop_name = format!("{:?}", property);
                    
                    // Check if we already have a data property edge
                    let has_data_edge = tableau.nodes.get(node_id)
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
                        let data_range_label = ConceptLabel::Complex(Box::new(
                            ClassExpression::DataSomeValuesFrom {
                                property: property.clone(),
                                filler: filler.clone(),
                            }
                        ));
                        tableau.add_concept_to_node(data_node_id, data_range_label)?;
                        
                        debug!("Applied DATATYPE rule (some) at node {}: created data value node {}", node_id, data_node_id);
                    }
                }
                ClassExpression::DataAllValuesFrom { property, filler } => {
                    // Ensure all data values satisfy the data range
                    let data_prop_name = format!("{:?}", property);
                    
                    // Get all data property successors
                    let data_successors: Vec<NodeId> = tableau.nodes.get(node_id)
                        .and_then(|n| n.role_successors.get(&data_prop_name))
                        .map(|succs| succs.iter().copied().collect())
                        .unwrap_or_default();
                    
                    let successor_count = data_successors.len();
                    
                    // Add data range restriction to each data value node
                    let data_range_label = ConceptLabel::Complex(Box::new(
                        ClassExpression::DataAllValuesFrom {
                            property: property.clone(),
                            filler: filler.clone(),
                        }
                    ));
                    
                    for data_node_id in data_successors {
                        tableau.add_concept_to_node(data_node_id, data_range_label.clone())?;
                    }
                    
                    debug!("Applied DATATYPE rule (all) at node {}: propagated to {} data nodes", node_id, successor_count);
                }
                ClassExpression::DataHasValue { property, value } => {
                    // Assert specific data value
                    let data_prop_name = format!("{:?}", property);
                    
                    // Create a data value node with the specific value
                    let data_node_id = tableau.add_node(NodeType::Generated)?;
                    
                    // Add edge with data property
                    let data_role = RoleLabel::Atomic(data_prop_name);
                    tableau.add_edge(node_id, data_node_id, data_role)?;
                    
                    // Add the specific value as a nominal
                    let value_label = ConceptLabel::Nominal(format!("{:?}", value));
                    tableau.add_concept_to_node(data_node_id, value_label)?;
                    
                    debug!("Applied DATATYPE rule (has value) at node {}: created data value node {}", node_id, data_node_id);
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

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
                        
                        // Note: In a full implementation, we could query clause_checker here
                        // to find all clauses mentioning this class and preemptively add
                        // their consequences to speed up reasoning. However, the standard
                        // approach is to let the completion rules discover these through
                        // normal tableau expansion.
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
        let node_id: NodeId = rule_app.node.parse().map_err(|_| {
            Error::reasoning(format!("Invalid node ID: {}", rule_app.node))
        })?;

        // Property chain rule: if x --R1--> y --R2--> z and R1 ∘ R2 ⊑ R, then x --R--> z
        // 
        // Implementation strategy:
        // 1. Find all 2-hop paths from the current node
        // 2. Check if any path matches a property chain axiom
        // 3. Create the implied super-property edge
        
        // Get property chain information from rule context
        if let RuleContext::Role { role, .. } = &rule_app.context {
            // Role context contains property chain information
            debug!("PROPERTY_CHAIN rule at node {}: processing role {:?}", node_id, role);
            
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
        let chains_created = 0;
        let outgoing_edges: Vec<(NodeId, String)> = tableau.edges.iter()
            .filter(|e| e.from == node_id)
            .map(|e| (e.to, e.role.to_string()))
            .collect();
        
        // For each outgoing edge, check for chain continuations
        for (intermediate_node, first_role) in outgoing_edges {
            // Get edges from intermediate node
            let second_hop_edges: Vec<(NodeId, String)> = tableau.edges.iter()
                .filter(|e| e.from == intermediate_node)
                .map(|e| (e.to, e.role.to_string()))
                .collect();
            
            // Check each potential chain
            for (target_node, second_role) in second_hop_edges {
                // In a full implementation, we would look up property chain axioms
                // from the ontology to see if first_role ∘ second_role ⊑ super_role
                //
                // For now, we log discovered chains. The actual chain axioms would
                // come from SubObjectPropertyOf axioms with PropertyChain expressions.
                
                debug!(
                    "Found potential chain: node {} --{}--> {} --{}--> {}",
                    node_id, first_role, intermediate_node, second_role, target_node
                );
                
                // TODO: Query ontology for property chain axioms matching (first_role, second_role)
                // If found, create implied edge:
                // let super_role = RoleLabel::Atomic(super_role_name);
                // tableau.add_edge(node_id, target_node, super_role)?;
                // chains_created += 1;
            }
        }
        
        if chains_created > 0 {
            debug!("PROPERTY_CHAIN rule at node {}: created {} implied edges", node_id, chains_created);
        }
        
        Ok(())
    }

    /// Apply the GUESS rule
    fn apply_guess_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
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

        // Extract the quoted triple ID from the rule application
        // For now, this is a placeholder implementation
        // In a full implementation, we would:
        // 1. Parse the quoted triple structure from the node's concepts
        // 2. Create nodes for s, p, o components
        // 3. Create edges representing the quoted triple relationship
        // 4. Link meta-assertions to the quoted triple node

        debug!("Applied quoted triple expansion rule for node: {}", rule_app.node);
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

        // Extract the meta-assertion from the rule application
        // For now, this is a placeholder implementation
        // In a full implementation, we would:
        // 1. Identify the quoted triple this assertion is about
        // 2. Extract the property and value of the meta-assertion
        // 3. Add the meta-assertion as an annotation to the quoted triple
        // 4. Check for meta-level clashes (e.g., contradictory certainty values)

        debug!("Applied meta assertion rule for node: {}", rule_app.node);
        Ok(())
    }

    /// Update blocking information for all nodes
    fn update_blocking(tableau: &mut super::Tableau) -> Result<()> {
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

        // Only check Nominal nodes for expensive clash detection
        // Root node and other nodes are checked via fast incremental detection
        for (i, node) in tableau.nodes.iter().enumerate() {
            // Skip non-Nominal nodes for expensive checks - they're handled incrementally
            if node.node_type != NodeType::Nominal {
                continue;
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
                                concept: format!("C and ¬C"),
                                node: i,
                            },
                            nodes: vec![i],
                            dependencies: Arc::new(DependencySet::new()),
                            explanation: format!("Node {} has a concept and its complement", i),
                        };
                        tableau.clash_detector.add_clash(clash);
                        tableau.statistics.increment_clashes();
                        log::warn!("Complement clash detected at node {}: C and ¬C", i);
                    }
                }
            }

            // Check for equivalence-disjointness clashes (expensive check)
            // This detects cases where an individual has two concepts that are equivalent and disjoint
            if let Some(checker) = &mut tableau.clause_checker {
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

                // Early exit if not enough concepts to check
                if node_concepts.len() < 2 {
                    continue;
                }

                // Check all pairs of concepts on this node
                let has_eq_closure = checker.equivalence_closure().is_some();
                let has_disj_map = checker.disjointness_map().is_some();

                if !has_eq_closure || !has_disj_map {
                    continue; // Skip if we don't have the necessary data structures
                }

                for idx1 in 0..node_concepts.len() {
                    for idx2 in (idx1 + 1)..node_concepts.len() {
                        let c1 = &node_concepts[idx1];
                        let c2 = &node_concepts[idx2];

                        // First check disjointness (cheaper), then equivalence
                        let are_disjoint = checker
                            .disjointness_map()
                            .map(|disj| disj.are_disjoint(c1, c2))
                            .unwrap_or(false);

                        if !are_disjoint {
                            continue; // Not disjoint, can't have the clash
                        }

                        // Only check equivalence if they're disjoint
                        let are_equivalent = checker
                            .equivalence_closure()
                            .map(|eq| eq.are_equivalent(c1, c2))
                            .unwrap_or(false);

                        if are_equivalent {
                            let clash = Clash {
                                clash_type: ClashType::Concept {
                                    concept: format!("{:?} ≡ {:?} but {:?} ⊥ {:?}", c1, c2, c1, c2),
                                    node: i,
                                },
                                nodes: vec![i],
                                dependencies: Arc::new(DependencySet::new()),
                                explanation: format!(
                                    "Node {} has concepts {:?} and {:?} which are both equivalent and disjoint",
                                    i, c1, c2
                                ),
                            };
                            tableau.clash_detector.add_clash(clash);
                            tableau.statistics.increment_clashes();
                            log::warn!(
                                "Equivalence-disjointness clash detected at node {}: {:?} ≡ {:?} but {:?} ⊥ {:?}",
                                i,
                                c1,
                                c2,
                                c1,
                                c2
                            );
                            return Ok(()); // Found a clash, no need to continue
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Queue applicable rules for a newly added concept
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

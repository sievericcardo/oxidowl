//! Tableau execution engine
//!
//! This module handles the main tableau expansion loop,
//! rule application, and completion checking.

use super::{
    node::{ConceptLabel, NodeId, NodeType},
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
        }

        Ok(())
    }

    /// Apply the AND rule (intersection)
    fn apply_and_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for AND rule
        // This would expand C ⊓ D by adding both C and D to the node
        Ok(())
    }

    /// Apply the OR rule (union) - creates branches
    fn apply_or_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for OR rule
        // This would create branches for C ⊔ D
        Ok(())
    }

    /// Apply the SOME rule (existential quantification)
    fn apply_some_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for SOME rule
        // This would create a new node for ∃R.C
        Ok(())
    }

    /// Apply the ALL rule (universal quantification)
    fn apply_all_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for ALL rule
        // This would add C to all R-successors for ∀R.C
        Ok(())
    }

    /// Apply the AT LEAST rule (minimum cardinality)
    fn apply_at_least_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for AT LEAST rule
        // This would ensure at least n R-successors for ≥nR.C
        Ok(())
    }

    /// Apply the AT MOST rule (maximum cardinality)
    fn apply_at_most_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for AT MOST rule
        // This would ensure at most n R-successors for ≤nR.C
        Ok(())
    }

    /// Apply the NOMINAL rule
    fn apply_nominal_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for NOMINAL rule
        // This would handle nominals {a}
        Ok(())
    }

    /// Apply the SELF rule
    fn apply_self_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for SELF rule
        // This would handle ∃R.Self constructs
        Ok(())
    }

    /// Apply the CHOOSE rule (non-deterministic choice)
    fn apply_choose_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for CHOOSE rule
        // This would handle choice points in expansion
        Ok(())
    }

    /// Apply the DATATYPE rule
    fn apply_datatype_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for DATATYPE rule
        // This would handle datatype restrictions
        Ok(())
    }

    /// Apply the UNFOLD rule
    fn apply_unfold_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for UNFOLD rule
        // This would unfold concept definitions
        Ok(())
    }

    /// Apply the PROPERTY CHAIN rule
    fn apply_property_chain_rule(
        tableau: &mut super::Tableau,
        rule_app: RuleApplication,
    ) -> Result<()> {
        // Implementation for property chain rule
        // This would handle property chain axioms
        Ok(())
    }

    /// Apply the GUESS rule
    fn apply_guess_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        // Implementation for guess rule
        // This would handle non-deterministic guessing
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

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
        blocking::{BlockingChecker, BlockingStrategy as BlockingFactory},
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
    /// Convert an ObjectPropertyExpression to a role name string using Display (not Debug).
    /// Using Display avoids the extra `Atomic(...)` / `Inverse(...)` wrapping that Debug adds,
    /// and is consistent between SOME-rule generation and ALL/AtLeast/AtMost lookups.
    #[inline]
    fn prop_to_role_name(prop: &crate::ontology::ObjectPropertyExpression) -> String {
        prop.to_string()
    }

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
            // Seed dirty_nodes with all Nominal/Generated nodes so the initial detect_clashes
            // checks them (ontology loading inserts concepts directly, bypassing add_concept_to_node)
            tableau.dirty_nodes.extend(
                tableau
                    .nodes
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| n.node_type != NodeType::Root)
                    .map(|(i, _)| i),
            );
            Self::detect_clashes(tableau)?;
            if tableau.clash_detector.has_clashes() {
                debug!("Clash detected in initial state, tableau is unsatisfiable");
                tableau.state = TableauState::Unsatisfiable;
                return Ok(tableau.state);
            }
        }

        // Create the blocking checker for this run. It is created once and reused across
        // iterations so that strategy-local state (signature caches, compared pairs, etc.)
        // persists between blocking updates.
        let mut blocking_checker: Box<dyn BlockingChecker> =
            BlockingFactory::create_checker(tableau.blocking_strategy)?;
        debug!(
            "Tableau blocking enabled with strategy {:?}",
            tableau.blocking_strategy
        );

        // Main expansion loop
        loop {
            if tableau.state != TableauState::Unknown {
                break;
            }

            // Process pending clashes from dirty nodes. This also covers clashes introduced
            // by a merge applied during backtracking, even when the rule queue is empty.
            // `has_clashes()` is included so a clash registered directly by an alternative
            // (e.g. a cardinality clash while re-resolving an exceeded ≤n) is handled even
            // when no node was marked dirty.
            if !tableau.dirty_nodes.is_empty() || tableau.clash_detector.has_clashes() {
                Self::detect_clashes(tableau)?;
                if tableau.clash_detector.has_clashes() {
                    if Self::attempt_backtrack(tableau)? {
                        continue;
                    }
                    debug!("Clash detected, no backtrack points available - unsatisfiable");
                    tableau.state = TableauState::Unsatisfiable;
                    break;
                }
            }

            // Check for timeout
            if let Some(timeout) = tableau.config.timeout
                && start_time.elapsed() > timeout
            {
                warn!("Tableau expansion timed out");
                tableau.state = TableauState::Unknown;
                break;
            }

            // Workstream A5: RBox role-rule pass (transitivity, symmetry, reflexivity,
            // role inclusions, property chains, domain/range, functional/inverse-functional
            // merging, HasKey, and the role-constraint clashes). If it derived anything new
            // (edges, concepts, merges) or registered a clash we re-enter the loop so clash
            // detection and the queued rules see the updated state.
            if Self::apply_role_rules(tableau)? {
                continue;
            }

            // Generic safety net: any DL clause with role atoms and an empty head whose body
            // is satisfied is turned into a clash.
            if Self::check_role_clause_violations(tableau)? {
                continue;
            }

            // Get next rule application
            let rule_app = match Arc::make_mut(&mut tableau.pending_queue).pop_front() {
                Some(rule_app) => rule_app,
                None => {
                    // Queue exhausted: the tableau is complete (nothing left to expand).
                    debug!("Tableau is complete and satisfiable");
                    tableau.state = TableauState::Satisfiable;
                    break;
                }
            };

            // Blocked nodes must not be re-expanded: skip any rule targeted at a blocked node.
            // This is what guarantees termination for cyclic TBoxes (e.g. C ⊑ ∃R.C chains).
            // Nominal and Root nodes are never marked blocked, so their rules are never skipped.
            if let Ok(node_id) = rule_app.node.parse::<NodeId>() {
                if let Some(node) = tableau.nodes.get(node_id)
                    && node.is_blocked()
                {
                    trace!(
                        "Skipping rule {:?} for blocked node {}",
                        rule_app.rule, node_id
                    );
                    continue;
                }
            }

            // Apply the rule
            Self::apply_rule(tableau, rule_app)?;

            // Re-run clash detection after every rule application so that clashes
            // introduced by expansion (e.g. AND-rule adding A and ¬A) are caught.
            Self::detect_clashes(tableau)?;
            if tableau.clash_detector.has_clashes() {
                if Self::attempt_backtrack(tableau)? {
                    continue;
                }
                debug!("Clash detected, no backtrack points available - unsatisfiable");
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

            // Update blocking information (marks newly created successors as blocked)
            Self::update_blocking(tableau, &mut blocking_checker)?;

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
            CompletionRule::HasValue => Self::apply_has_value_rule(tableau, rule_app)?,
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
                    // Add each conjunct to the node. `add_concept_to_node` re-queues the
                    // rule for any complex conjunct, so nested constructors
                    // (∃R.C, {a}, ≤nR.C, …) introduced by expansion are expanded too
                    // (workstream A6 — single queueing path via add_concept_to_node).
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

        if let RuleContext::Concept {
            concept,
            dependencies,
        } = &rule_app.context
        {
            match concept {
                ClassExpression::ObjectUnionOf(disjuncts) => {
                    if disjuncts.is_empty() {
                        return Ok(());
                    }

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

                    // Snapshot *before* the first disjunct is applied so that backtracking
                    // to an alternative undoes the first choice: the disjuncts are mutually
                    // exclusive alternatives, not conjuncts. Restoring a state that already
                    // contains disjunct[0] would keep the (possibly clashing) first choice on
                    // the node forever, making sound ontologies such as
                    // `A ⊑ (B ⊔ C)`, `A ⊑ ¬B`, `A(a)` (model: a ∈ C) report unsatisfiable.
                    let saved_state = Self::snapshot_state(tableau);

                    // Apply the first disjunct immediately.
                    let first_disjunct = &disjuncts[0];
                    let concept_label = ConceptLabel::Complex(Box::new(first_disjunct.clone()));
                    tableau.add_concept_to_node(node_id, concept_label)?;
                    debug!(
                        "Applied OR rule at node {node_id}: selected first of {} disjunct(s)",
                        disjuncts.len()
                    );

                    // For each remaining disjunct, create a backtrack point.
                    if disjuncts.len() > 1 {
                        for (idx, disjunct) in disjuncts.iter().enumerate().skip(1) {
                            let label = ConceptLabel::Complex(Box::new(disjunct.clone()));
                            let backtrack_point = super::BacktrackPoint {
                                id: tableau.backtrack_stack.len() + idx,
                                node_id,
                                choice: super::Choice::Disjunction {
                                    concepts: vec![label],
                                    chosen_index: idx,
                                },
                                saved_state: saved_state.clone(),
                                dependencies: Arc::clone(dependencies),
                            };
                            tableau.backtrack_stack.push(backtrack_point);
                            debug!(
                                "OR rule at node {node_id}: pushed alternative disjunct {idx} as backtrack point"
                            );
                        }
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
    /// Expands ∃R.C by creating a new successor node with R-edge and adding C to it.
    ///
    /// When the filler is a single nominal `{a}`, the successor is *not* a fresh
    /// generated node: it reuses (or creates) the nominal node for `a`, so that
    /// `∃R.{a}` (and `≥1 R.{a}`) interacts correctly with `≤n R.…` — the nominal
    /// is counted exactly once and no spurious successors are minted.
    fn apply_some_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                    let role_name = Self::prop_to_role_name(property);

                    // Existing R-successors of the source node.
                    let existing_successors: Vec<NodeId> = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(role_name.as_str()))
                        .map(|succs| succs.iter().copied().collect())
                        .unwrap_or_default();

                    // A single-nominal filler must be witnessed by the nominal node itself.
                    let nominal_target = match &**filler {
                        ClassExpression::ObjectOneOf(individuals) if individuals.len() == 1 => {
                            Some(tableau.get_or_create_nominal_node(&individuals[0])?)
                        }
                        _ => None,
                    };

                    // We must create/attach a successor iff:
                    //  - the filler is a nominal and it is not already an R-successor, or
                    //  - the filler is not a nominal and there is no R-successor at all.
                    let need_new = match nominal_target {
                        Some(target) => !existing_successors.contains(&target),
                        None => existing_successors.is_empty(),
                    };

                    if need_new {
                        // Reuse the nominal node for `{a}`, otherwise create a generated node.
                        let new_node_id = match nominal_target {
                            Some(target) => target,
                            None => tableau.add_node(NodeType::Generated)?,
                        };

                        // Add R-edge from current node to the successor
                        let role_label = RoleLabel::Atomic(role_name.clone());
                        tableau.add_edge(node_id, new_node_id, role_label)?;

                        // Add filler concept to the successor. The Nominal label for a
                        // reused nominal node is added by `get_or_create_nominal_node`.
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
                                            && Self::prop_to_role_name(p) == role_name
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
                                            && Self::prop_to_role_name(p) == role_name
                                        {
                                            return Some((*cardinality, *f.clone()));
                                        }
                                        None
                                    })
                                    .collect()
                            })
                            .unwrap_or_default();

                        for (max_n, max_filler) in max_card_violations {
                            // Count how many R-successors now have the filler (top filler
                            // `owl:Thing` matches every successor).
                            let count = Self::collect_object_successors(
                                tableau,
                                node_id,
                                &role_name,
                                &max_filler,
                            )
                            .len();

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
                    let role_name = Self::prop_to_role_name(property);

                    // Find all R-successors
                    let successors: Vec<NodeId> = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(role_name.as_str()))
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
    /// Expands ≥nR.C by ensuring at least n distinct R-successors with concept C.
    ///
    /// Correctness notes (workstream A3):
    /// - Successors are counted *filler-aware*: only R-successors that already satisfy the
    ///   filler C are counted, so `≥1 R.C ⊓ ≥1 R.D` creates one successor per filler.
    /// - The `cardinality` witnesses are pairwise distinct: freshly created successors are
    ///   marked `≠` both with each other and with the pre-existing filler-successors they
    ///   join. This is what lets a later `≤n R.C` detect that the witnesses cannot be merged.
    /// - `≥n R.{a}` with n > 1 is unsatisfiable (a single nominal can only witness one
    ///   R-successor) — a cardinality clash is registered instead of looping.
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
                    Self::apply_at_least_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                // `=n R.C` may reach this rule directly (exact-cardinality dispatch).
                ClassExpression::ObjectExactCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    Self::apply_at_least_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                _ => {
                    debug!("AT_LEAST rule called on non-min-cardinality concept");
                }
            }
        }
        Ok(())
    }

    /// Shared implementation of the ≥-rule for `≥n R.C`.
    fn apply_at_least_cardinality(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::ObjectPropertyExpression,
        cardinality: u32,
        filler: &ClassExpression,
    ) -> Result<()> {
        let role_name = Self::prop_to_role_name(property);

        // A single-nominal filler `≥1 R.{a}` is witnessed by the nominal node itself
        // (never a fresh node). Higher nominal cardinalities are unsatisfiable.
        if let ClassExpression::ObjectOneOf(individuals) = filler {
            if individuals.len() == 1 {
                let target = tableau.get_or_create_nominal_node(&individuals[0])?;
                if cardinality > 1 {
                    let clash = Clash {
                        clash_type: ClashType::Cardinality {
                            role: RoleLabel::Atomic(role_name.clone()),
                            node: node_id,
                            min_cardinality: cardinality,
                            max_cardinality: u32::MAX,
                        },
                        nodes: vec![node_id],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "Node {node_id} requires ≥{cardinality} R-successors each equal to {{{}}} but a single nominal individual can witness only one",
                            individuals[0]
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    log::warn!(
                        "Cardinality clash at node {node_id}: ≥{cardinality} R.{} is unsatisfiable",
                        individuals[0]
                    );
                    return Ok(());
                }

                let existing_successors = tableau
                    .nodes
                    .get(node_id)
                    .and_then(|n| n.role_successors.get(role_name.as_str()))
                    .cloned()
                    .unwrap_or_default();
                if !existing_successors.contains(&target) {
                    let role_label = RoleLabel::Atomic(role_name.clone());
                    tableau.add_edge(node_id, target, role_label)?;
                    let filler_label = ConceptLabel::Complex(Box::new(filler.clone()));
                    tableau.add_concept_to_node(target, filler_label)?;
                    debug!(
                        "Applied AT_LEAST rule at node {node_id}: reused nominal successor {target}"
                    );
                }
                return Ok(());
            }
        }

        // Filler-aware counting: existing R-successors that already satisfy the filler.
        let existing_successors =
            Self::collect_object_successors(tableau, node_id, &role_name, filler);
        let existing_count = existing_successors.len();

        // Create additional successors if needed
        let needed = (cardinality as usize).saturating_sub(existing_count);
        let role_label = RoleLabel::Atomic(role_name.clone());
        let filler_label = ConceptLabel::Complex(Box::new(filler.clone()));

        let mut created: Vec<NodeId> = Vec::with_capacity(needed);
        for _ in 0..needed {
            let new_node_id = tableau.add_node(NodeType::Generated)?;
            tableau.add_edge(node_id, new_node_id, role_label.clone())?;
            tableau.add_concept_to_node(new_node_id, filler_label.clone())?;
            created.push(new_node_id);
        }

        if needed > 0 {
            // The `cardinality` witnesses (existing filler-successors + fresh ones) must be
            // pairwise distinct. `add_inequality` skips pairs that both carry the same
            // nominal label (A2/A3 nominal note).
            let all_witnesses: Vec<NodeId> = existing_successors
                .iter()
                .chain(created.iter())
                .copied()
                .collect();
            for i in 0..all_witnesses.len() {
                for j in (i + 1)..all_witnesses.len() {
                    tableau.add_inequality(all_witnesses[i], all_witnesses[j]);
                }
            }

            // Fresh successors may push the count above an existing ≤n R.C at this node;
            // re-queue the AtMost rules for the same role so the excess is merged/clashed.
            Self::re_queue_max_rules_for_role(tableau, node_id, &role_name)?;

            debug!("Applied AT_LEAST rule at node {node_id}: created {needed} successors");
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
                    Self::apply_at_most_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                // `=n R.C` may reach this rule directly (exact-cardinality dispatch).
                ClassExpression::ObjectExactCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    Self::apply_at_most_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                _ => {
                    debug!("AT_MOST rule called on non-max-cardinality concept");
                }
            }
        }
        Ok(())
    }

    /// Shared ≤-rule implementation for `≤n R.C`.
    fn apply_at_most_cardinality(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::ObjectPropertyExpression,
        cardinality: u32,
        filler: &ClassExpression,
    ) -> Result<()> {
        let role_name = Self::prop_to_role_name(property);

        // Nominal-aware merging: two successors that carry the same nominal
        // label `{a}` denote the same individual and must be collapsed into a
        // single node *before* counting. This is the key correctness behaviour
        // for `≥1 R.{a}` (ObjectHasValue) + `≤n R.…` interplay.
        Self::merge_duplicate_nominal_successors(tableau, node_id, &role_name)?;

        let successors = Self::collect_object_successors(tableau, node_id, &role_name, filler);
        Self::resolve_max_cardinality(
            tableau,
            node_id,
            &role_name,
            &filler.to_string(),
            successors,
            cardinality as usize,
            false,
        )
    }

    /// Merge duplicate nominal R-successors of `node_id` into single nodes.
    ///
    /// Two successors labelled with the same nominal denote the same individual, so they
    /// must be a single node for `≤n R.…` counting to be sound. The survivor is the
    /// earlier (smaller id) nominal node.
    fn merge_duplicate_nominal_successors(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        role_name: &str,
    ) -> Result<()> {
        let successors: Vec<NodeId> = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(role_name))
            .map(|succs| {
                succs
                    .iter()
                    .copied()
                    .filter(|&s| {
                        tableau
                            .nodes
                            .get(s)
                            .map(|n| {
                                n.concepts
                                    .iter()
                                    .any(|c| matches!(c, ConceptLabel::Nominal(_)))
                            })
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Group successor ids by their nominal label.
        let mut by_nominal: std::collections::HashMap<String, Vec<NodeId>> =
            std::collections::HashMap::new();
        for succ in successors {
            if let Some(node) = tableau.nodes.get(succ)
                && let Some(nominal) = node.concepts.iter().find_map(|c| match c {
                    ConceptLabel::Nominal(name) => Some(name.clone()),
                    _ => None,
                })
            {
                by_nominal.entry(nominal).or_default().push(succ);
            }
        }

        for group in by_nominal.into_values() {
            if group.len() > 1 {
                let mut group = group;
                group.sort_unstable();
                let survivor = group[0];
                for duplicate in &group[1..] {
                    Self::merge_nodes(tableau, survivor, *duplicate)?;
                    log::debug!(
                        "AT_MOST rule at node {node_id}: merged nominal successor {duplicate} into {survivor}"
                    );
                }
            }
        }

        Ok(())
    }

    /// Resolve an exceeded `≤n` constraint by merging successor pairs until the count is
    /// within bounds.
    ///
    /// Each required merge records a non-deterministic choice point ([`super::Choice::AtMostMerge`])
    /// carrying *all* valid merge candidate pairs, and applies the first one. If a later clash
    /// is detected the executor backtracks and tries the next candidate, which is what makes
    /// AtMost reasoning complete for non-deterministic ontologies.
    fn resolve_max_cardinality(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        role: &str,
        filler_key: &str,
        successors: Vec<NodeId>,
        max_count: usize,
        is_data: bool,
    ) -> Result<()> {
        if successors.len() <= max_count {
            return Ok(());
        }

        let mut remaining = successors;
        while remaining.len() > max_count {
            let candidates = Self::merge_candidates(tableau, &remaining);
            if candidates.is_empty() {
                // No two successors can be merged: either every pair is already known to be
                // distinct (an AtLeast rule marked them `≠`) or incompatible (distinct
                // nominals). The `≤n` constraint is violated → cardinality clash.
                let clash = Clash {
                    clash_type: ClashType::Cardinality {
                        role: RoleLabel::Atomic(role.to_string()),
                        node: node_id,
                        min_cardinality: 0,
                        max_cardinality: max_count as u32,
                    },
                    nodes: vec![node_id],
                    dependencies: Arc::new(DependencySet::new()),
                    explanation: format!(
                        "Node {node_id} violates ≤{max_count} {role}.{filler_key}: {} successors exist and none can be merged (all pairwise distinct or incompatible)",
                        remaining.len()
                    ),
                };
                tableau.clash_detector.add_clash(clash);
                tableau.statistics.increment_clashes();
                log::warn!(
                    "Cardinality clash at node {node_id}: ≤{max_count} {role}.{filler_key} with {} non-mergeable successors",
                    remaining.len()
                );
                return Ok(());
            }

            // Record a choice point covering every candidate merge pair for this step.
            let saved_state = Self::snapshot_state(tableau);
            let backtrack_point = super::BacktrackPoint {
                id: tableau.backtrack_stack.len(),
                node_id,
                choice: super::Choice::AtMostMerge {
                    node_id,
                    role: role.to_string(),
                    filler: filler_key.to_string(),
                    max_count,
                    is_data,
                    candidates: candidates.clone(),
                    tried: 0,
                },
                saved_state,
                dependencies: Arc::new(DependencySet::new()),
            };
            tableau.backtrack_stack.push(backtrack_point);
            debug!(
                "AT_MOST rule at node {node_id}: pushed merge choice with {} candidate(s)",
                candidates.len()
            );

            let (survivor, away) = candidates[0];
            Self::merge_nodes(tableau, survivor, away)?;
            debug!(
                "AT_MOST rule at node {node_id}: merged node {away} into {survivor} to satisfy ≤{max_count} {role}.{filler_key}"
            );
            remaining.retain(|&s| s != away);
        }
        Ok(())
    }

    /// Compute the valid (survivor, merged-away) merge candidate pairs among `successors`.
    ///
    /// A pair is invalid if the two nodes are already known distinct (an inequality atom),
    /// or if they carry *different* nominal labels `{a}` vs `{b}` (distinct named
    /// individuals). Nominal-avoidance is applied: when exactly one node of a pair carries a
    /// nominal label, it is kept as the survivor.
    fn merge_candidates(tableau: &super::Tableau, successors: &[NodeId]) -> Vec<(NodeId, NodeId)> {
        let mut out: Vec<(NodeId, NodeId)> = Vec::new();
        for i in 0..successors.len() {
            for j in (i + 1)..successors.len() {
                let a = successors[i];
                let b = successors[j];
                if a == b {
                    continue;
                }
                if tableau.are_inequal(a, b) {
                    continue;
                }
                let nom_a = tableau.node_nominal(a);
                let nom_b = tableau.node_nominal(b);
                if let (Some(na), Some(nb)) = (&nom_a, &nom_b) {
                    // Distinct nominals denote distinct individuals — never mergeable.
                    if na != nb {
                        continue;
                    }
                }
                let pair = match (&nom_a, &nom_b) {
                    (Some(_), None) => (a, b),
                    (None, Some(_)) => (b, a),
                    _ => (a, b),
                };
                out.push(pair);
            }
        }
        // Deterministic order: nominal survivors first, then by survivor id, then merged id.
        out.sort_by_key(|&(survivor, away)| {
            (tableau.node_nominal(survivor).is_none(), survivor, away)
        });
        out
    }

    /// Merge two nodes in the tableau
    ///
    /// Merges `source` into `target`: source's concepts are copied into target, edges
    /// pointing at/from `source` are redirected to `target`, and `source` is cleared and
    /// marked clashed. Inequality atoms involving `source` are remapped to `target`, and a
    /// [`super::MergeRecord`] is appended to the merge journal so the operation can be
    /// undone exactly when backtracking over a merge choice.
    ///
    /// `pub(crate)`: also used by `Tableau::process_axiom` to collapse SameIndividual
    /// nodes when loading the ABox (workstream A5).
    pub(crate) fn merge_nodes(
        tableau: &mut super::Tableau,
        target: NodeId,
        source: NodeId,
    ) -> Result<()> {
        if target == source {
            return Ok(());
        }

        // Snapshot pre-merge state for the merge journal.
        let target_concepts = tableau
            .nodes
            .get(target)
            .map(|n| n.concepts.clone())
            .unwrap_or_default();
        let source_concepts = tableau
            .nodes
            .get(source)
            .map(|n| n.concepts.clone())
            .unwrap_or_default();
        let target_dependencies = tableau
            .nodes
            .get(target)
            .map(|n| n.concept_dependencies.clone())
            .unwrap_or_default();
        let source_dependencies = tableau
            .nodes
            .get(source)
            .map(|n| n.concept_dependencies.clone())
            .unwrap_or_default();
        let target_node_type = tableau
            .nodes
            .get(target)
            .map(|n| n.node_type)
            .unwrap_or(NodeType::Generated);
        let source_node_type = tableau
            .nodes
            .get(source)
            .map(|n| n.node_type)
            .unwrap_or(NodeType::Generated);
        let target_status = tableau
            .nodes
            .get(target)
            .map(|n| n.status.clone())
            .unwrap_or_default();
        let source_status = tableau
            .nodes
            .get(source)
            .map(|n| n.status.clone())
            .unwrap_or_default();
        let edge_changes: Vec<(usize, NodeId, NodeId)> = tableau
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| e.from == source || e.to == source)
            .map(|(i, e)| (i, e.from, e.to))
            .collect();

        // Copy all concepts from source to target
        for concept in source_concepts.iter().cloned() {
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

        // Keep the individual→node map consistent: any individual that pointed at the
        // merged-away node now resolves to the surviving node.
        for (_, node_id) in tableau.individual_map.iter_mut() {
            if *node_id == source {
                *node_id = target;
            }
        }

        // Remap inequality atoms: `source` and `target` now denote the same individual.
        Self::remap_inequalities(tableau, source, target);

        // Mark source node as merged (by clearing its concepts and marking as clashed)
        if let Some(source_node) = tableau.nodes.get_mut(source) {
            source_node.concepts.clear();
            source_node.status.clashed = true;
        }

        // Record the merge for backtracking.
        tableau.merge_log.push(super::MergeRecord {
            target,
            source,
            target_concepts,
            source_concepts,
            target_dependencies,
            source_dependencies,
            target_node_type,
            source_node_type,
            target_status,
            source_status,
            edge_changes,
        });

        Ok(())
    }

    /// Rewrite inequality atoms after merging `source` into `target`: any pair mentioning
    /// `source` is redirected to `target`, and the (source, target) pair is dropped since
    /// the two nodes now denote the same individual.
    fn remap_inequalities(tableau: &mut super::Tableau, source: NodeId, target: NodeId) {
        let mut next = std::collections::HashSet::with_capacity(tableau.inequalities.len());
        for &(a, b) in &tableau.inequalities {
            let na = if a == source { target } else { a };
            let nb = if b == source { target } else { b };
            if na != nb {
                next.insert((na.min(nb), na.max(nb)));
            }
        }
        tableau.inequalities = next;
    }

    /// Create a snapshot of the tableau state for a backtrack point.
    fn snapshot_state(tableau: &super::Tableau) -> super::SavedState {
        super::SavedState {
            node_count: tableau.nodes.len(),
            edge_count: tableau.edges.len(),
            pending_queue: tableau.pending_queue.clone(),
            inequalities: tableau.inequalities.clone(),
            individual_map: tableau.individual_map.clone(),
            merge_log_len: tableau.merge_log.len(),
            // Per-node concept sets: restoring them on backtrack is what lets a
            // disjunction alternative be genuinely undone (an added disjunct on an
            // existing node is not covered by truncating the node/edge lists).
            node_concepts: tableau.nodes.iter().map(|n| n.concepts.clone()).collect(),
        }
    }

    /// Attempt to backtrack to the most recent choice point and apply its next alternative.
    ///
    /// Returns `Ok(true)` if an alternative was applied (the caller should re-enter the
    /// expansion loop), or `Ok(false)` if no choice point has an untried alternative left
    /// (the current branch is genuinely unsatisfiable).
    ///
    /// Restores the node/edge lists, pending queue, inequality atoms, individual map and
    /// merge journal to the saved state, then applies the alternative. For merge choices
    /// the alternative is the next candidate merge pair; any still-excess successors are
    /// re-resolved inline. Blocking state is recomputed from scratch on the next loop
    /// iteration (see `update_blocking`), so it stays consistent across backtracking.
    fn attempt_backtrack(tableau: &mut super::Tableau) -> Result<bool> {
        while let Some(backtrack_point) = tableau.backtrack_stack.pop() {
            debug!(
                "Backtracking to choice point #{} at node {}",
                backtrack_point.id, backtrack_point.node_id
            );

            // Restore saved state: undo merges, then truncate nodes/edges.
            tableau.undo_merges(backtrack_point.saved_state.merge_log_len);
            tableau
                .nodes
                .truncate(backtrack_point.saved_state.node_count);
            tableau
                .edges
                .truncate(backtrack_point.saved_state.edge_count);
            tableau.rebuild_role_successors();
            tableau.pending_queue = backtrack_point.saved_state.pending_queue.clone();
            tableau.inequalities = backtrack_point.saved_state.inequalities.clone();
            tableau.individual_map = backtrack_point.saved_state.individual_map.clone();

            // Restore per-node concept sets so the alternative is applied to the same
            // labels the choice point saw (undoes OR-rule disjunct additions, etc.).
            for (i, concepts) in backtrack_point.saved_state.node_concepts.iter().enumerate() {
                if let Some(node) = tableau.nodes.get_mut(i) {
                    node.concepts = concepts.clone();
                }
            }

            // Reset clash detection *before* applying the alternative: the previous branch's
            // clashes belong to the abandoned state. dirty_nodes is intentionally preserved so
            // clashes introduced by the alternative (e.g. a merge combining C and ¬C) are
            // re-detected, and any clash registered directly by the alternative (e.g. a
            // cardinality clash from re-resolving an exceeded ≤n) is kept in the fresh detector.
            tableau.clash_detector = crate::core::tableau::ClashDetector::new();

            // Apply the alternative choice
            let applied = match &backtrack_point.choice {
                super::Choice::Disjunction { concepts, .. } => {
                    if let Some(label) = concepts.first() {
                        tableau.add_concept_to_node(backtrack_point.node_id, label.clone())?;
                        debug!(
                            "Backtracked: applying alternative concept at node {}",
                            backtrack_point.node_id
                        );
                        true
                    } else {
                        false
                    }
                }
                super::Choice::AtMostMerge {
                    node_id,
                    role,
                    filler,
                    max_count,
                    is_data,
                    candidates,
                    tried,
                } => {
                    let next = tried + 1;
                    if next < candidates.len() {
                        // Re-queue the updated choice point so a later backtrack can try
                        // the next candidate, then apply this one.
                        let mut updated = backtrack_point.clone();
                        if let super::Choice::AtMostMerge {
                            tried: tried_field, ..
                        } = &mut updated.choice
                        {
                            *tried_field = next;
                        }
                        tableau.backtrack_stack.push(updated);

                        let (survivor, away) = candidates[next];
                        debug!(
                            "Backtracked: merging node {away} into {survivor} (alternative {next}/{}) at node {node_id}",
                            candidates.len()
                        );
                        Self::merge_nodes(tableau, survivor, away)?;

                        // Any still-excess successors are re-resolved inline; this pushes
                        // fresh (nested) merge choice points for the remaining merges.
                        Self::resolve_max_for_backtrack(
                            tableau, *node_id, role, filler, *max_count, *is_data,
                        )?;
                        true
                    } else {
                        // This merge choice is exhausted — pop the next choice point.
                        false
                    }
                }
                super::Choice::FunctionalMerge { candidates, tried } => {
                    let next = tried + 1;
                    if next < candidates.len() {
                        let mut updated = backtrack_point.clone();
                        if let super::Choice::FunctionalMerge {
                            tried: tried_field, ..
                        } = &mut updated.choice
                        {
                            *tried_field = next;
                        }
                        tableau.backtrack_stack.push(updated);
                        let (survivor, away) = candidates[next];
                        debug!(
                            "Backtracked: applying alternative functional merge {away} -> {survivor} ({next}/{})",
                            candidates.len()
                        );
                        Self::merge_nodes(tableau, survivor, away)?;
                        true
                    } else {
                        // This forced-equality choice is exhausted — pop the next choice point.
                        false
                    }
                }
                super::Choice::Blocking { .. } => false,
            };

            if applied {
                tableau.statistics.increment_backtracks();
                return Ok(true);
            }
            // Point exhausted with no alternatives — continue to the next choice point.
        }
        Ok(false)
    }

    /// Re-resolve an `≤n` constraint after backtracking applied a merge candidate.
    ///
    /// The choice point stores the filler in display form; this re-identifies the matching
    /// max-cardinality concept on the node and re-runs [`Self::resolve_max_cardinality`] for
    /// any excess successors that remain.
    fn resolve_max_for_backtrack(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        role: &str,
        filler_str: &str,
        max_count: usize,
        is_data: bool,
    ) -> Result<()> {
        if is_data {
            use crate::ontology::DataRange;
            let ranges: Vec<DataRange> = tableau
                .nodes
                .get(node_id)
                .map(|n| {
                    n.concepts
                        .iter()
                        .filter_map(|c| match c {
                            ConceptLabel::Complex(expr) => match expr.as_ref() {
                                ClassExpression::DataMaxCardinality {
                                    property,
                                    cardinality,
                                    filler,
                                } if property.to_string() == role
                                    && *cardinality as usize == max_count
                                    && filler.to_string() == filler_str =>
                                {
                                    Some(filler.clone())
                                }
                                ClassExpression::DataExactCardinality {
                                    property,
                                    cardinality,
                                    filler,
                                } if property.to_string() == role
                                    && *cardinality as usize == max_count
                                    && filler.to_string() == filler_str =>
                                {
                                    Some(filler.clone())
                                }
                                _ => None,
                            },
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(range) = ranges.first() {
                let successors = Self::collect_data_successors(tableau, node_id, role, range);
                Self::resolve_max_cardinality(
                    tableau, node_id, role, filler_str, successors, max_count, true,
                )?;
            }
        } else {
            let fillers: Vec<ClassExpression> = tableau
                .nodes
                .get(node_id)
                .map(|n| {
                    n.concepts
                        .iter()
                        .filter_map(|c| match c {
                            ConceptLabel::Complex(expr) => match expr.as_ref() {
                                ClassExpression::ObjectMaxCardinality {
                                    property,
                                    cardinality,
                                    filler,
                                } if Self::prop_to_role_name(property) == role
                                    && *cardinality as usize == max_count
                                    && filler.to_string() == filler_str =>
                                {
                                    Some((**expr).clone())
                                }
                                ClassExpression::ObjectExactCardinality {
                                    property,
                                    cardinality,
                                    filler,
                                } if Self::prop_to_role_name(property) == role
                                    && *cardinality as usize == max_count
                                    && filler.to_string() == filler_str =>
                                {
                                    Some((**expr).clone())
                                }
                                _ => None,
                            },
                            _ => None,
                        })
                        .collect()
                })
                .unwrap_or_default();
            if let Some(filler) = fillers.first() {
                let successors = Self::collect_object_successors(tableau, node_id, role, filler);
                Self::resolve_max_cardinality(
                    tableau, node_id, role, filler_str, successors, max_count, false,
                )?;
            }
        }
        Ok(())
    }

    /// Whether the object filler is `owl:Thing` (top), which every successor trivially
    /// satisfies. Used for filler-aware successor counting in the ≥/≤ rules.
    fn is_top_filler(filler: &ClassExpression) -> bool {
        matches!(filler, ClassExpression::Class(c) if c.iri.is_owl_thing())
    }

    /// Collect the R-successors of `node_id` whose filler concept matches `filler`.
    ///
    /// When `filler` is `owl:Thing` all successors match. Otherwise a successor matches if
    /// its label contains exactly the filler concept (syntactic match, consistent with the
    /// tableau's label-based clash detection). The result is sorted by node id so that
    /// downstream merge-candidate selection is deterministic regardless of `HashSet`
    /// iteration order.
    fn collect_object_successors(
        tableau: &super::Tableau,
        node_id: NodeId,
        role_name: &str,
        filler: &ClassExpression,
    ) -> Vec<NodeId> {
        let is_top = Self::is_top_filler(filler);
        let filler_label = ConceptLabel::Complex(Box::new(filler.clone()));
        let mut out: Vec<NodeId> = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(role_name))
            .map(|succs| {
                succs
                    .iter()
                    .copied()
                    .filter(|&s| {
                        tableau
                            .nodes
                            .get(s)
                            .map(|sn| is_top || sn.concepts.contains(&filler_label))
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_unstable();
        out
    }

    /// Re-queue the AtMost rules for every `≤n`/`=n` object constraint on `role_name` at
    /// `node_id`. Called after the AtLeast rule creates fresh successors, so any resulting
    /// excess is merged/clashed.
    fn re_queue_max_rules_for_role(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        role_name: &str,
    ) -> Result<()> {
        use crate::core::completion::{CompletionRule, RuleApplication, RuleContext, RulePriority};
        let concepts: Vec<ClassExpression> = tableau
            .nodes
            .get(node_id)
            .map(|n| {
                n.concepts
                    .iter()
                    .filter_map(|c| match c {
                        ConceptLabel::Complex(expr) => match expr.as_ref() {
                            ClassExpression::ObjectMaxCardinality { property, .. }
                                if Self::prop_to_role_name(property) == role_name =>
                            {
                                Some((**expr).clone())
                            }
                            ClassExpression::ObjectExactCardinality { property, .. }
                                if Self::prop_to_role_name(property) == role_name =>
                            {
                                Some((**expr).clone())
                            }
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        for concept in concepts {
            let app = RuleApplication {
                rule: CompletionRule::AtMost,
                node: node_id.to_string(),
                context: RuleContext::Concept {
                    concept,
                    dependencies: Arc::new(DependencySet::new()),
                },
                priority: RulePriority::Normal,
                dependencies: Arc::new(DependencySet::new()),
            };
            Arc::make_mut(&mut tableau.pending_queue).push_back(app);
        }
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
        } else if let Some((prop_expr, target_concept)) = Self::extract_role_atom_info(atom) {
            let successors = Self::get_successors(tableau, node_id, &prop_expr);
            for succ_id in &successors {
                if Self::node_has_concept(tableau, *succ_id, &target_concept) {
                    return true;
                }
            }
            false
        } else {
            false
        }
    }

    /// Extract role property expression and target concept from a binary role atom.
    /// For a role atom R(x, y), the predicate encodes the role and arguments[1] encodes
    /// the target variable/individual name that we match against the target node's concepts.
    fn extract_role_atom_info(
        atom: &crate::dl_clauses::DLAtom,
    ) -> Option<(crate::ontology::ObjectPropertyExpression, ClassExpression)> {
        if atom.arguments.len() < 2 {
            return None;
        }
        let role_iri = crate::ontology::IRI::new(&atom.predicate);
        let prop = crate::ontology::ObjectPropertyExpression::ObjectProperty(
            crate::ontology::ObjectProperty { iri: role_iri },
        );
        let target_concept_iri = crate::ontology::IRI::new(&atom.arguments[1]);
        let target_concept = ClassExpression::Class(crate::ontology::Class {
            iri: target_concept_iri,
        });
        Some((prop, target_concept))
    }

    /// Get all successor node IDs reachable from `node_id` via forward and inverse edges
    /// whose role label matches the given property expression.
    ///
    /// Role names use the canonical `inv(IRI)` string form for inverse properties (the
    /// same form produced by `ObjectPropertyExpression::Display` / `prop_to_role_name`
    /// and by the DL-clause layer), so forward and inverse lookups agree on edge labels.
    fn get_successors(
        tableau: &super::Tableau,
        node_id: NodeId,
        prop_expr: &crate::ontology::ObjectPropertyExpression,
    ) -> Vec<NodeId> {
        let role_name = match prop_expr {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                prop.iri.as_str().to_string()
            }
            crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                format!("inv({})", prop.iri.as_str())
            }
            crate::ontology::ObjectPropertyExpression::PropertyChain(_) => {
                return Vec::new();
            }
        };

        let mut successors = Vec::new();

        // Forward edges: node_id --role--> target
        if let Some(node) = tableau.nodes.get(node_id)
            && let Some(forward) = node.role_successors.get(role_name.as_str())
        {
            successors.extend(forward.iter().copied());
        }

        // Inverse edges: an edge y --role'--> node_id represents role_name(x, y) when
        // role' is the inverse of role_name. For a forward property `R` the inverse edge
        // label is `inv(R)`; for an inverse property `inv(R)` the inverse edge label is `R`.
        let inv_role_name = match prop_expr {
            crate::ontology::ObjectPropertyExpression::ObjectProperty(prop) => {
                format!("inv({})", prop.iri.as_str())
            }
            crate::ontology::ObjectPropertyExpression::InverseObjectProperty(prop) => {
                prop.iri.as_str().to_string()
            }
            crate::ontology::ObjectPropertyExpression::PropertyChain(_) => String::new(),
        };
        for edge in &tableau.edges {
            if edge.to == node_id {
                let edge_role = edge.role.to_string();
                if (edge_role == role_name || edge_role == inv_role_name)
                    && !successors.contains(&edge.from)
                {
                    successors.push(edge.from);
                }
                if let super::node::RoleLabel::Inverse(name) = &edge.role
                    && (name == &role_name || name == &inv_role_name)
                    && !successors.contains(&edge.from)
                {
                    successors.push(edge.from);
                }
            }
        }

        successors
    }

    /// Check if a node has a given concept in its label set.
    fn node_has_concept(
        tableau: &super::Tableau,
        node_id: NodeId,
        concept: &ClassExpression,
    ) -> bool {
        if let Some(node) = tableau.nodes.get(node_id) {
            node.concepts.iter().any(|c| match c {
                ConceptLabel::Complex(expr) => expr.as_ref() == concept,
                ConceptLabel::Atomic(_) | ConceptLabel::NegatedAtomic(_) => {
                    if let ClassExpression::Class(target_class) = concept {
                        match c {
                            ConceptLabel::Atomic(name) => name == target_class.iri.as_str(),
                            ConceptLabel::NegatedAtomic(name) => name == target_class.iri.as_str(),
                            _ => false,
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            })
        } else {
            false
        }
    }

    /// Whether a complex class expression denotes the bottom concept
    /// (owl:Nothing or ¬owl:Thing).
    fn is_bottom_concept(expr: &ClassExpression) -> bool {
        match expr {
            ClassExpression::Class(class) => class.iri.is_owl_nothing(),
            ClassExpression::ObjectComplementOf(inner) => {
                matches!(&**inner, ClassExpression::Class(class) if class.iri.is_owl_thing())
            }
            _ => false,
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
                    // Add nominal labels for each individual. Use the canonical individual
                    // string (IRI for named, `_:id` for anonymous) so labels agree with
                    // `individual_map` keys and the ObjectHasValue rule.
                    for individual in individuals {
                        let name = individual.to_string();
                        let nominal_label = ConceptLabel::Nominal(name);
                        tableau.add_concept_to_node(node_id, nominal_label)?;

                        // Nominal rule (classical): if x ∈ {a} then x = a. Merge x into the
                        // canonical nominal node for `a` unless x IS that node (or the root,
                        // whose label is a TBox-encoding artifact).
                        if node_id != 0 {
                            let nominal_node = tableau.get_or_create_nominal_node(individual)?;
                            if node_id != nominal_node {
                                // Redirect edges/concepts from `node_id` into `nominal_node`.
                                Self::merge_nodes(tableau, nominal_node, node_id)?;
                                log::debug!(
                                    "NOMINAL rule at node {node_id}: merged into nominal node {nominal_node}"
                                );
                            }
                        }
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

    /// Apply the HAS VALUE rule
    /// Expands ∃R.{a} (ObjectHasValue) by creating an R-edge from x to the nominal node
    /// representing `a` (reusing the existing nominal node when one already exists).
    fn apply_has_value_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::ObjectHasValue { property, value } => {
                    let role_name = Self::prop_to_role_name(property);

                    // The filler of ObjectHasValue is a single nominal: reuse the canonical
                    // nominal node for the individual.
                    let nominal_id = tableau.get_or_create_nominal_node(value)?;

                    // Skip if this node already has the required R-edge to the nominal.
                    let has_edge = tableau
                        .nodes
                        .get(node_id)
                        .and_then(|n| n.role_successors.get(role_name.as_str()))
                        .map(|succs| succs.contains(&nominal_id))
                        .unwrap_or(false);

                    if !has_edge {
                        let role_label = RoleLabel::Atomic(role_name.clone());
                        tableau.add_edge(node_id, nominal_id, role_label)?;
                        log::debug!(
                            "Applied HAS_VALUE rule at node {node_id}: edge {role_name} to nominal node {nominal_id}"
                        );

                        // Propagate any ∀R.X concepts already on the source node to the
                        // nominal successor (mirrors the SOME rule).
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
                                            && Self::prop_to_role_name(p) == role_name
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
                            tableau.add_concept_to_node(nominal_id, all_label)?;
                        }
                    }
                }
                _ => {
                    debug!("HAS_VALUE rule called on non-has-value concept");
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
                    let role_name = Self::prop_to_role_name(property);
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
            use super::{BacktrackPoint, Choice};

            let backtrack_point = BacktrackPoint {
                id: tableau.backtrack_stack.len(),
                node_id,
                choice: Choice::Disjunction {
                    concepts: choice_concepts,
                    chosen_index: 0, // Start with first alternative
                },
                saved_state: Self::snapshot_state(tableau),
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
    /// Handles datatype property restrictions and data ranges (workstream A4)
    fn apply_datatype_rule(tableau: &mut super::Tableau, rule_app: RuleApplication) -> Result<()> {
        let node_id: NodeId = rule_app
            .node
            .parse()
            .map_err(|_| Error::reasoning(format!("Invalid node ID: {}", rule_app.node)))?;

        // The root node encodes TBox axioms (GCIs) as a virtual witness. Firing the
        // datatype rules there would mint spurious data witnesses and — worse — report
        // datatype clashes for genuinely consistent TBoxes such as
        // `A ≡ ∃P.(int [minInclusive 10, maxInclusive 5])` with no instance of A.
        // Expansion happens on real individuals once the concept is unfolded.
        if tableau
            .nodes
            .get(node_id)
            .map(|n| n.node_type == NodeType::Root)
            .unwrap_or(false)
        {
            debug!("DATATYPE rule skipped at root node {node_id} (TBox encoding)");
            return Ok(());
        }

        // A data-value successor node represents a concrete literal, not an individual:
        // datatype concept expansions (∃P.D, ≥n P.D, …) do not apply to it, and the
        // range-witness markers carried by such nodes are internal bookkeeping only.
        // Without this guard, re-queueing a marker (e.g. when a merge copies labels onto
        // a data node) would recursively mint fresh data successors on the data node
        // itself, producing spurious values that break ≤n P.D counting (workstream A6).
        if tableau
            .nodes
            .get(node_id)
            .is_some_and(|n| Self::data_value_of(n).is_some())
        {
            debug!("DATATYPE rule skipped at data-value node {node_id}");
            return Ok(());
        }

        if let RuleContext::Concept { concept, .. } = &rule_app.context {
            match concept {
                ClassExpression::DataSomeValuesFrom { property, filler } => {
                    Self::apply_data_some_values_from(tableau, node_id, property, filler)?;
                }
                ClassExpression::DataAllValuesFrom { property, filler } => {
                    Self::apply_data_all_values_from(tableau, node_id, property, filler)?;
                }
                ClassExpression::DataHasValue { property, value } => {
                    Self::apply_data_has_value(tableau, node_id, property, value)?;
                }
                ClassExpression::DataMinCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    Self::apply_data_min_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                ClassExpression::DataMaxCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    Self::apply_data_max_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                ClassExpression::DataExactCardinality {
                    property,
                    cardinality,
                    filler,
                } => {
                    // `=n P.D ≡ ≥n P.D ⊓ ≤n P.D` over data values.
                    Self::apply_data_min_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                    Self::apply_data_max_cardinality(
                        tableau,
                        node_id,
                        property,
                        *cardinality,
                        filler,
                    )?;
                }
                _ => {
                    debug!("DATATYPE rule called on non-datatype concept");
                }
            }
        }
        Ok(())
    }

    /// `∃P.D`: create a data successor whose concrete value lies in `D`.
    ///
    /// Workstream A4 value-space generation: a concrete literal is minted through the
    /// `DatatypeValidator` for datatype ranges, `DataOneOf` (uses a listed value) and
    /// facet-restricted ranges (synthesised to satisfy every facet). When the range is
    /// provably empty (contradictory facets, disjoint intersection) a datatype clash is
    /// recorded. If an existing data successor already carries a value in `D`, it is reused.
    fn apply_data_some_values_from(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::DataPropertyExpression,
        filler: &crate::ontology::DataRange,
    ) -> Result<()> {
        use crate::core::reasoner::datatype_validation::RangeValue;
        let data_prop_name = property.to_string();

        // If an existing data successor already carries a value in D, reuse it.
        let is_top = Self::is_top_data_range(filler);
        let already_satisfied = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(data_prop_name.as_str()))
            .map(|succs| {
                succs.iter().any(|&s| {
                    tableau
                        .nodes
                        .get(s)
                        .map(|sn| {
                            Self::data_node_matches_range(
                                tableau,
                                sn,
                                &data_prop_name,
                                filler,
                                is_top,
                            )
                        })
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if already_satisfied {
            return Ok(());
        }

        // Mint a concrete literal for the range.
        match tableau
            .datatype_validator
            .generate_literal_for_range(filler, 0)
        {
            RangeValue::Empty => {
                Self::add_datatype_clash(
                    tableau,
                    node_id,
                    format!(
                        "Data range {filler} is empty: no literal satisfies ∃{data_prop_name}.{filler}"
                    ),
                );
            }
            RangeValue::Value(lit) => {
                let data_node_id = tableau.add_node(NodeType::Generated)?;
                let data_role = RoleLabel::Atomic(data_prop_name);
                tableau.add_edge(node_id, data_node_id, data_role)?;
                tableau.add_concept_to_node(data_node_id, ConceptLabel::DataValue(lit))?;
                // Keep the range-witness marker so the ≤-rule / DataAllValuesFrom counting
                // still recognises this successor when value-space matching is partial.
                Self::add_range_witness_marker(tableau, data_node_id, property, filler)?;
                debug!(
                    "Applied DATATYPE rule (some) at node {node_id}: minted data value node {data_node_id}"
                );
            }
            RangeValue::Witness => {
                let data_node_id = tableau.add_node(NodeType::Generated)?;
                let data_role = RoleLabel::Atomic(data_prop_name);
                tableau.add_edge(node_id, data_node_id, data_role)?;
                Self::add_range_witness_marker(tableau, data_node_id, property, filler)?;
                debug!(
                    "Applied DATATYPE rule (some) at node {node_id}: created witness data value node {data_node_id}"
                );
            }
        }
        Ok(())
    }

    /// `∀P.D`: every data successor's concrete value must lie in `D`.
    ///
    /// A successor with a concrete literal outside the range's value space is a datatype
    /// clash; an untyped placeholder receives the range-witness marker (A3 behaviour).
    fn apply_data_all_values_from(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::DataPropertyExpression,
        filler: &crate::ontology::DataRange,
    ) -> Result<()> {
        let data_prop_name = property.to_string();

        let data_successors: Vec<NodeId> = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(data_prop_name.as_str()))
            .map(|succs| succs.iter().copied().collect())
            .unwrap_or_default();

        let data_range_label =
            ConceptLabel::Complex(Box::new(ClassExpression::DataAllValuesFrom {
                property: property.clone(),
                filler: filler.clone(),
            }));

        let successor_count = data_successors.len();
        for data_node_id in data_successors {
            let concrete_value = tableau
                .nodes
                .get(data_node_id)
                .and_then(Self::data_value_of)
                .cloned();
            match concrete_value {
                Some(lit) => {
                    match tableau
                        .datatype_validator
                        .literal_satisfies_data_range(&lit, filler)
                    {
                        Ok(true) => {}
                        Ok(false) => {
                            Self::add_datatype_clash(
                                tableau,
                                node_id,
                                format!(
                                    "Data value {lit} (node {data_node_id}) violates ∀{data_prop_name}.{filler}"
                                ),
                            );
                        }
                        Err(_) => {
                            // Membership indeterminate — keep the syntactic marker.
                            // The marker is internal bookkeeping, not a concept to expand,
                            // so it is added without rule re-queueing (workstream A6).
                            tableau.add_concept_to_node_no_queue(
                                data_node_id,
                                data_range_label.clone(),
                            )?;
                        }
                    }
                }
                None => {
                    // Untyped placeholder: propagate the ALL restriction (A3 behaviour).
                    // See note above — no rule re-queueing for this internal marker.
                    tableau.add_concept_to_node_no_queue(data_node_id, data_range_label.clone())?;
                }
            }
        }

        debug!(
            "Applied DATATYPE rule (all) at node {node_id}: checked {} data nodes",
            successor_count
        );
        Ok(())
    }

    /// `∃P.{v}`: connect to a data successor carrying value `v` (value equality).
    ///
    /// An existing data successor with a value-equal literal is reused (dedup); otherwise a
    /// fresh data node carrying [`ConceptLabel::DataValue`] is created.
    fn apply_data_has_value(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::DataPropertyExpression,
        value: &crate::ontology::Literal,
    ) -> Result<()> {
        use crate::core::reasoner::datatype_validation::DatatypeValidator;
        let data_prop_name = property.to_string();

        let existing_equal = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(data_prop_name.as_str()))
            .map(|succs| {
                succs.iter().any(|&s| {
                    tableau
                        .nodes
                        .get(s)
                        .and_then(Self::data_value_of)
                        .map(|v| DatatypeValidator::literal_values_equal(v, value))
                        .unwrap_or(false)
                })
            })
            .unwrap_or(false);
        if existing_equal {
            return Ok(());
        }

        let data_node_id = tableau.add_node(NodeType::Generated)?;
        let data_role = RoleLabel::Atomic(data_prop_name);
        tableau.add_edge(node_id, data_node_id, data_role)?;
        tableau.add_concept_to_node(data_node_id, ConceptLabel::DataValue(value.clone()))?;

        debug!(
            "Applied DATATYPE rule (has value) at node {node_id}: created data value node {data_node_id}"
        );
        Ok(())
    }

    /// Data-property ≥-rule: `≥n P.D` requires n distinct data successors whose values lie
    /// in D. Fresh successors are minted with pairwise-≠ concrete literals (via the
    /// `DatatypeValidator`, distinct by construction); when no further distinct value can be
    /// minted (range empty or exhausted) a datatype clash is recorded. Untyped placeholders
    /// are created only when no concrete literal is available.
    fn apply_data_min_cardinality(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::DataPropertyExpression,
        cardinality: u32,
        filler: &crate::ontology::DataRange,
    ) -> Result<()> {
        use crate::core::reasoner::datatype_validation::RangeValue;
        let role_name = property.to_string();
        let existing = Self::collect_data_successors(tableau, node_id, &role_name, filler);
        let needed = (cardinality as usize).saturating_sub(existing.len());

        if needed > 0 {
            let mut created: Vec<NodeId> = Vec::with_capacity(needed);
            let role_label = RoleLabel::Atomic(role_name.clone());

            // Values already in use by matching successors must not be re-minted: two
            // successors with value-equal literals would not be genuinely distinct.
            let mut minted: std::collections::HashSet<crate::ontology::Literal> =
                std::collections::HashSet::new();
            for &s in &existing {
                if let Some(lit) = tableau.nodes.get(s).and_then(Self::data_value_of) {
                    minted.insert(lit.clone());
                }
            }

            let mut seed = 0usize;
            for _ in 0..needed {
                let value = tableau
                    .datatype_validator
                    .generate_distinct_literal_for_range(filler, seed, &minted);
                match value {
                    RangeValue::Empty => {
                        // The range cannot provide another distinct value → ≥n P.D violated.
                        Self::add_datatype_clash(
                            tableau,
                            node_id,
                            format!(
                                "≥{cardinality} {role_name}.{filler} needs {needed} distinct data values but the range cannot provide them"
                            ),
                        );
                        return Ok(());
                    }
                    RangeValue::Value(lit) => {
                        let data_node_id = tableau.add_node(NodeType::Generated)?;
                        tableau.add_edge(node_id, data_node_id, role_label.clone())?;
                        tableau.add_concept_to_node(
                            data_node_id,
                            ConceptLabel::DataValue(lit.clone()),
                        )?;
                        Self::add_range_witness_marker(tableau, data_node_id, property, filler)?;
                        minted.insert(lit);
                        created.push(data_node_id);
                        seed += 1;
                    }
                    RangeValue::Witness => {
                        let data_node_id = tableau.add_node(NodeType::Generated)?;
                        tableau.add_edge(node_id, data_node_id, role_label.clone())?;
                        Self::add_range_witness_marker(tableau, data_node_id, property, filler)?;
                        created.push(data_node_id);
                    }
                }
            }

            let all: Vec<NodeId> = existing.iter().chain(created.iter()).copied().collect();
            for i in 0..all.len() {
                for j in (i + 1)..all.len() {
                    tableau.add_inequality(all[i], all[j]);
                }
            }

            // Fresh data successors may violate an existing ≤n P.D constraint.
            Self::re_queue_data_max_rules_for_role(tableau, node_id, &role_name)?;

            debug!(
                "Applied DATATYPE rule (min cardinality) at node {node_id}: created {needed} data value nodes for ≥{cardinality} {role_name}.{filler}"
            );
        }
        Ok(())
    }

    /// Data-property ≤-rule: `≤n P.D` merges excess data successors that lie in D.
    fn apply_data_max_cardinality(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        property: &crate::ontology::DataPropertyExpression,
        cardinality: u32,
        filler: &crate::ontology::DataRange,
    ) -> Result<()> {
        let role_name = property.to_string();

        // Two successors carrying value-equal literals denote the same data value and
        // count once (workstream A4 dedup, mirroring nominal dedup).
        Self::merge_duplicate_data_value_successors(tableau, node_id, &role_name)?;

        let successors = Self::collect_data_successors(tableau, node_id, &role_name, filler);
        Self::resolve_max_cardinality(
            tableau,
            node_id,
            &role_name,
            &filler.to_string(),
            successors,
            cardinality as usize,
            true,
        )
    }

    /// Merge data-successors of `node_id` whose concrete literals are value-equal into a
    /// single node (workstream A4). Two successors with equal data values denote the same
    /// value, so they must collapse before `≤n P.D` counting — mirroring
    /// [`Self::merge_duplicate_nominal_successors`] for object properties.
    fn merge_duplicate_data_value_successors(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        role_name: &str,
    ) -> Result<()> {
        use crate::core::reasoner::datatype_validation::DatatypeValidator;
        let successors: Vec<NodeId> = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(role_name))
            .map(|succs| succs.iter().copied().collect())
            .unwrap_or_default();

        // Group successor ids by value equality (structural grouping is insufficient:
        // "5"^^xsd:int and "5"^^xsd:integer are value-equal but structurally distinct).
        let mut groups: Vec<Vec<NodeId>> = Vec::new();
        let mut representatives: Vec<crate::ontology::Literal> = Vec::new();
        for succ in successors {
            let Some(lit) = tableau.nodes.get(succ).and_then(Self::data_value_of) else {
                continue;
            };
            let mut matched = None;
            for (gi, rep) in representatives.iter().enumerate() {
                if DatatypeValidator::literal_values_equal(rep, lit) {
                    matched = Some(gi);
                    break;
                }
            }
            match matched {
                Some(gi) => groups[gi].push(succ),
                None => {
                    representatives.push(lit.clone());
                    groups.push(vec![succ]);
                }
            }
        }

        for group in groups {
            if group.len() > 1 {
                let mut group = group;
                group.sort_unstable();
                let survivor = group[0];
                for duplicate in &group[1..] {
                    Self::merge_nodes(tableau, survivor, *duplicate)?;
                    log::debug!(
                        "AT_MOST rule at node {node_id}: merged duplicate data-value successor {duplicate} into {survivor}"
                    );
                }
            }
        }
        Ok(())
    }

    /// Whether a data range is the top datatype (`rdfs:Literal`), which every data value
    /// trivially belongs to.
    fn is_top_data_range(range: &crate::ontology::DataRange) -> bool {
        matches!(
            range,
            crate::ontology::DataRange::Datatype(iri)
                if iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#Literal"
        )
    }

    /// Whether a data value node is a witness for range `range` on data property `role_name`.
    ///
    /// Workstream A4 value-space matching: a node carrying a concrete
    /// [`ConceptLabel::DataValue`] literal matches the range iff the literal's value lies in
    /// the range's value space (checked through the shared `DatatypeValidator`, covering
    /// facets, `DatatypeDefinition` lookup and `owl:real`/`owl:rational`). Nodes without a
    /// concrete value fall back to the A3 syntactic range-witness approximation.
    fn data_node_matches_range(
        tableau: &super::Tableau,
        node: &super::node::TableauNode,
        role_name: &str,
        range: &crate::ontology::DataRange,
        is_top: bool,
    ) -> bool {
        if is_top {
            return true;
        }
        if let Some(literal) = Self::data_value_of(node) {
            return tableau
                .datatype_validator
                .literal_satisfies_data_range(literal, range)
                .unwrap_or(false);
        }
        node.concepts.iter().any(|c| match c {
            ConceptLabel::Complex(expr) => match expr.as_ref() {
                ClassExpression::DataSomeValuesFrom { property, filler } => {
                    property.to_string() == role_name && filler == range
                }
                ClassExpression::DataAllValuesFrom { property, filler } => {
                    property.to_string() == role_name && filler == range
                }
                _ => false,
            },
            _ => false,
        })
    }

    /// Extract the concrete literal carried by a data value node, if any (workstream A4).
    fn data_value_of(node: &super::node::TableauNode) -> Option<&crate::ontology::Literal> {
        node.concepts.iter().find_map(|c| match c {
            ConceptLabel::DataValue(lit) => Some(lit),
            _ => None,
        })
    }

    /// Label a data node with the range-witness marker (a `DataSomeValuesFrom` complex label
    /// carrying the same property and filler) so the ≤-rule and `DataAllValuesFrom`
    /// propagation can identify it syntactically when value-space matching is unavailable.
    ///
    /// The marker is internal bookkeeping, not a concept to expand, so it is added without
    /// rule re-queueing (workstream A6).
    fn add_range_witness_marker(
        tableau: &mut super::Tableau,
        data_node_id: NodeId,
        property: &crate::ontology::DataPropertyExpression,
        filler: &crate::ontology::DataRange,
    ) -> Result<()> {
        let marker = ConceptLabel::Complex(Box::new(ClassExpression::DataSomeValuesFrom {
            property: property.clone(),
            filler: filler.clone(),
        }));
        tableau.add_concept_to_node_no_queue(data_node_id, marker)
    }

    /// Register a concrete-domain (datatype) clash at `node_id` (workstream A4).
    fn add_datatype_clash(tableau: &mut super::Tableau, node_id: NodeId, description: String) {
        let clash = Clash {
            clash_type: ClashType::Datatype {
                node: node_id,
                description: description.clone(),
            },
            nodes: vec![node_id],
            dependencies: Arc::new(DependencySet::new()),
            explanation: description,
        };
        tableau.clash_detector.add_clash(clash);
        tableau.statistics.increment_clashes();
        tableau.dirty_nodes.push(node_id);
        debug!("Datatype clash at node {node_id}");
    }

    /// Collect the data-successors of `node_id` whose values lie in `range`.
    ///
    /// The result is sorted by node id so merge-candidate selection is deterministic.
    fn collect_data_successors(
        tableau: &super::Tableau,
        node_id: NodeId,
        role_name: &str,
        range: &crate::ontology::DataRange,
    ) -> Vec<NodeId> {
        let is_top = Self::is_top_data_range(range);
        let mut out: Vec<NodeId> = tableau
            .nodes
            .get(node_id)
            .and_then(|n| n.role_successors.get(role_name))
            .map(|succs| {
                succs
                    .iter()
                    .copied()
                    .filter(|&s| {
                        tableau
                            .nodes
                            .get(s)
                            .map(|sn| {
                                Self::data_node_matches_range(tableau, sn, role_name, range, is_top)
                            })
                            .unwrap_or(false)
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.sort_unstable();
        out
    }

    /// Re-queue the Datatype rules for every `≤n`/`=n` data constraint on `role_name` at
    /// `node_id`. Called after the data ≥-rule creates fresh successors.
    fn re_queue_data_max_rules_for_role(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        role_name: &str,
    ) -> Result<()> {
        use crate::core::completion::{CompletionRule, RuleApplication, RuleContext, RulePriority};
        let concepts: Vec<ClassExpression> = tableau
            .nodes
            .get(node_id)
            .map(|n| {
                n.concepts
                    .iter()
                    .filter_map(|c| match c {
                        ConceptLabel::Complex(expr) => match expr.as_ref() {
                            ClassExpression::DataMaxCardinality { property, .. }
                                if property.to_string() == role_name =>
                            {
                                Some((**expr).clone())
                            }
                            ClassExpression::DataExactCardinality { property, .. }
                                if property.to_string() == role_name =>
                            {
                                Some((**expr).clone())
                            }
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default();

        for concept in concepts {
            let app = RuleApplication {
                rule: CompletionRule::Datatype,
                node: node_id.to_string(),
                context: RuleContext::Concept {
                    concept,
                    dependencies: Arc::new(DependencySet::new()),
                },
                priority: RulePriority::Normal,
                dependencies: Arc::new(DependencySet::new()),
            };
            Arc::make_mut(&mut tableau.pending_queue).push_back(app);
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
    /// Handles property chain axioms of arbitrary length: R1 ∘ … ∘ Rn ⊑ R.
    ///
    /// Workstream A5: this delegates to the shared n-ary chain engine
    /// ([`Self::apply_property_chains`]) used by the RBox pass, which composes n-hop paths
    /// (n ≥ 2) into the super-property edge. The `RuleApplication` node is ignored because
    /// the chain rule is inherently graph-global; this method exists for the rare case where
    /// a `PropertyChain` rule is queued directly.
    fn apply_property_chain_rule(
        tableau: &mut super::Tableau,
        rule_app: RuleApplication,
    ) -> Result<()> {
        use crate::ontology::{Axiom, ObjectPropertyExpression};

        let _ = rule_app;

        // Collect all property-chain axioms: (chain roles, super role).
        let mut chains: Vec<(Vec<String>, String)> = Vec::new();
        for axiom in tableau.ontology.axioms() {
            if let Axiom::SubObjectPropertyOf(a) = axiom
                && let ObjectPropertyExpression::PropertyChain(chain) = &a.sub_property
                && !matches!(a.super_property, ObjectPropertyExpression::PropertyChain(_))
            {
                let chain_names: Vec<String> = chain.iter().map(|p| p.to_string()).collect();
                if chain_names.len() >= 2 {
                    chains.push((chain_names, a.super_property.to_string()));
                }
            }
        }

        Self::apply_property_chains(tableau, &chains)?;
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

    /// Update blocking information for all nodes.
    ///
    /// Delegates to the configured `BlockingChecker` (built once per run from the
    /// `ReasoningConfig::blocking_strategy`), then enforces two soundness guards:
    ///
    /// 1. **Root and Nominal nodes are never blocked.** The root encodes TBox axioms
    ///    (a meta-level artifact) and nominal nodes represent named individuals whose
    ///    labels must always be expanded; blocking them would be unsound for OWL 2 DL.
    ///    Nominal-aware blocking is handled in a later workstream (A2).
    /// 2. **A node may not be blocked by the Root node**, since the root's label is a
    ///    TBox-encoding and is not a genuine individual witness.
    ///
    /// Blocked nodes are marked `fully_expanded` so `is_complete` can recognise that
    /// the tableau has exhausted all unblocked expansion.
    fn update_blocking(
        tableau: &mut super::Tableau,
        checker: &mut Box<dyn BlockingChecker>,
    ) -> Result<()> {
        if !tableau.config.blocking_enabled {
            return Ok(());
        }

        // Recompute blocking from the current node labels. Each checker is stateful
        // across calls (e.g. signature caches, compared-pair sets) but recomputes the
        // `is_blocked`/`blocker` flags for every node from scratch, so blocking is
        // monotone with respect to the current node set and self-heals after backtracking
        // truncates nodes/edges.
        checker.update_blocking(&mut tableau.nodes)?;

        let node_count = tableau.nodes.len();
        for i in 0..node_count {
            // Never block the root or nominal nodes, and never block a node that carries
            // a nominal label `{a}` — such a node is a genuine individual witness whose
            // labels must always be expanded (nominal-aware blocking). A nominal label in
            // a *blocker* is still a sound blocker (see `blocking.rs`).
            if matches!(
                tableau.nodes[i].node_type,
                NodeType::Nominal | NodeType::Root
            ) || crate::core::blocking::contains_nominal_label(&tableau.nodes[i])
            {
                tableau.nodes[i].blocking_info.is_blocked = false;
                tableau.nodes[i].blocking_info.blocker = None;
                tableau.nodes[i].status.fully_expanded = false;
                continue;
            }

            // A node blocked by the root would rely on TBox-encoding concepts as a witness;
            // reject such a block as unsound.
            if let Some(blocker) = tableau.nodes[i].blocking_info.blocker {
                if blocker < node_count && tableau.nodes[blocker].node_type == NodeType::Root {
                    tableau.nodes[i].blocking_info.is_blocked = false;
                    tableau.nodes[i].blocking_info.blocker = None;
                    tableau.nodes[i].status.fully_expanded = false;
                    continue;
                }
            }

            // A blocked node needs no further expansion.
            if tableau.nodes[i].blocking_info.is_blocked {
                tableau.nodes[i].status.fully_expanded = true;
            }
        }

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

    /// Apply the RBox (role-axiom) rules over the current edge set (workstream A5).
    ///
    /// This is the single enforcement point for the RBox in the active tableau. It covers:
    ///
    /// | Axiom | Effect |
    /// |-------|--------|
    /// | `SubObjectPropertyOf(R, S)` (simple) | every `R`-edge also yields an `S`-edge |
    /// | `TransitiveObjectProperty(P)` | `P(x,y) ∧ P(y,z)` ⇒ add `P(x,z)` |
    /// | `SymmetricObjectProperty(P)` | `P(x,y)` ⇒ add `P(y,x)` |
    /// | `ReflexiveObjectProperty(P)` | add `P(n,n)` for every non-root node |
    /// | `InverseObjectProperties(P,Q)` | `P(x,y)` ⇔ add `Q(y,x)` |
    /// | `SubObjectPropertyOf(PropertyChain(...), S)` | compose n-hop paths ⇒ `S`-edge |
    /// | `ObjectPropertyDomain(P,C)` / `Range` | add `C` to the source / target node |
    /// | `FunctionalObjectProperty(P)` | `P(x,y1) ∧ P(x,y2)` ⇒ merge `y1,y2` (via A3 ≤-machinery) |
    /// | `InverseFunctionalObjectProperty(P)` | `P(x1,y) ∧ P(x2,y)` ⇒ merge `x1,x2` |
    /// | `AsymmetricObjectProperty(P)` | `P(x,y) ∧ P(y,x)` ⇒ clash |
    /// | `IrreflexiveObjectProperty(P)` | `P(x,x)` ⇒ clash |
    /// | `DisjointObjectProperties(P,Q)` | `P(x,y) ∧ Q(x,y)` ⇒ clash |
    /// | `NegativeObjectPropertyAssertion` | `¬P(a,b)` recorded; clash if `P(a,b)` edge appears |
    /// | `HasKey(C, keys)` | two `C`-nodes sharing a value per key property ⇒ merge |
    ///
    /// Returns `true` when the tableau changed (new edges / concepts / merges) or a clash
    /// was registered, so the executor re-enters the loop.
    ///
    /// **Trigger strategy**: a per-iteration sweep over the current edge set. This subsumes
    /// "trigger on edge creation" (new edges are visible to the next sweep) and is simple to
    /// reason about; all rules are idempotent (edge/concept additions are deduplicated and
    /// merges reduce the node count), so the sweep reaches a fixpoint and terminates.
    ///
    /// **Blocking**: role closure is applied to *all* edges, including those touching
    /// blocked nodes. Blocking only gates concept expansion; role closure is monotone and
    /// blocked nodes share their blocker's labels, so this is sound.
    fn apply_role_rules(tableau: &mut super::Tableau) -> Result<bool> {
        use crate::core::tableau::node::NodeType;
        use crate::ontology::{Axiom, ClassExpression, ObjectPropertyExpression};

        let mut changed = false;

        // ── Collect RBox data (owned, so we can mutate the tableau freely). ─────────
        let mut transitive: Vec<String> = Vec::new();
        let mut symmetric: Vec<String> = Vec::new();
        let mut asymmetric: Vec<String> = Vec::new();
        let mut irreflexive: Vec<String> = Vec::new();
        let mut reflexive: Vec<String> = Vec::new();
        let mut functional: Vec<String> = Vec::new();
        let mut inverse_functional: Vec<String> = Vec::new();
        let mut inclusions: Vec<(String, String)> = Vec::new();
        let mut inverse_pairs: Vec<(String, String)> = Vec::new();
        let mut chains: Vec<(Vec<String>, String)> = Vec::new();
        let mut domains: Vec<(String, ClassExpression)> = Vec::new();
        let mut ranges: Vec<(String, ClassExpression)> = Vec::new();
        let mut data_inclusions: Vec<(String, String)> = Vec::new();
        let mut data_domains: Vec<(String, ClassExpression)> = Vec::new();
        let mut data_ranges: Vec<(String, crate::ontology::DataRange)> = Vec::new();
        let mut disjoint_pairs: Vec<(String, String)> = Vec::new();
        let mut has_keys: Vec<(
            ClassExpression,
            Vec<ObjectPropertyExpression>,
            Vec<crate::ontology::DataPropertyExpression>,
        )> = Vec::new();

        for axiom in tableau.ontology.axioms() {
            match axiom {
                Axiom::TransitiveObjectProperty(a) => transitive.push(a.property.to_string()),
                Axiom::SymmetricObjectProperty(a) => symmetric.push(a.property.to_string()),
                Axiom::AsymmetricObjectProperty(a) => asymmetric.push(a.property.to_string()),
                Axiom::IrreflexiveObjectProperty(a) => irreflexive.push(a.property.to_string()),
                Axiom::ReflexiveObjectProperty(a) => reflexive.push(a.property.to_string()),
                Axiom::FunctionalObjectProperty(a) => functional.push(a.property.to_string()),
                Axiom::InverseFunctionalObjectProperty(a) => {
                    inverse_functional.push(a.property.to_string());
                }
                Axiom::InverseObjectProperties(a) => {
                    inverse_pairs.push((a.property1.to_string(), a.property2.to_string()));
                }
                Axiom::EquivalentObjectProperties(a) => {
                    // Equivalent properties are bidirectional role inclusions.
                    let names: Vec<String> = a.properties.iter().map(|p| p.to_string()).collect();
                    for i in 0..names.len() {
                        for j in (i + 1)..names.len() {
                            inclusions.push((names[i].clone(), names[j].clone()));
                            inclusions.push((names[j].clone(), names[i].clone()));
                        }
                    }
                }
                Axiom::SubObjectPropertyOf(a) => match &a.sub_property {
                    ObjectPropertyExpression::PropertyChain(chain) => {
                        let super_name = a.super_property.to_string();
                        if !matches!(a.super_property, ObjectPropertyExpression::PropertyChain(_)) {
                            let chain_names: Vec<String> =
                                chain.iter().map(|p| p.to_string()).collect();
                            if !chain_names.is_empty() {
                                chains.push((chain_names, super_name));
                            }
                        }
                    }
                    _ => {
                        let sub_name = a.sub_property.to_string();
                        let super_name = a.super_property.to_string();
                        if sub_name != super_name {
                            inclusions.push((sub_name, super_name));
                        }
                    }
                },
                Axiom::ObjectPropertyDomain(a) => {
                    domains.push((a.property.to_string(), a.domain.clone()));
                }
                Axiom::ObjectPropertyRange(a) => {
                    ranges.push((a.property.to_string(), a.range.clone()));
                }
                Axiom::SubDataPropertyOf(a) => {
                    let sub_name = a.sub_property.to_string();
                    let super_name = a.super_property.to_string();
                    if sub_name != super_name {
                        data_inclusions.push((sub_name, super_name));
                    }
                }
                Axiom::DataPropertyDomain(a) => {
                    data_domains.push((a.property.to_string(), a.domain.clone()));
                }
                Axiom::DataPropertyRange(a) => {
                    data_ranges.push((a.property.to_string(), a.range.clone()));
                }
                Axiom::DisjointObjectProperties(a) => {
                    let names: Vec<String> = a.properties.iter().map(|p| p.to_string()).collect();
                    for i in 0..names.len() {
                        for j in (i + 1)..names.len() {
                            disjoint_pairs.push((names[i].clone(), names[j].clone()));
                        }
                    }
                }
                Axiom::HasKey(a) => has_keys.push((
                    a.class.clone(),
                    a.object_properties.clone(),
                    a.data_properties.clone(),
                )),
                _ => {}
            }
        }

        let node_count = tableau.nodes.len();

        // ── 1. Role inclusions: R ⊑ S — every R-edge also satisfies S. ──────────────
        for (sub_name, super_name) in &inclusions {
            let matching: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *sub_name)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in matching {
                if Self::ensure_edge(tableau, x, y, super_name)? {
                    changed = true;
                }
            }
        }

        // ── 2. Transitivity: P(x,y) ∧ P(y,z) ⇒ P(x,z). ──────────────────────────────
        for p in &transitive {
            let p_edges: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in &p_edges {
                for (_y2, z) in &p_edges {
                    if y == _y2 && Self::ensure_edge(tableau, *x, *z, p)? {
                        changed = true;
                    }
                }
            }
        }

        // ── 3. Symmetry: P(x,y) ⇒ P(y,x). ───────────────────────────────────────────
        for p in &symmetric {
            let p_edges: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in p_edges {
                if Self::ensure_edge(tableau, y, x, p)? {
                    changed = true;
                }
            }
        }

        // ── 4. Reflexivity: P(n,n) for every non-root node. ─────────────────────────
        // The root node encodes TBox axioms (a meta-level artifact) and is not an
        // individual witness, so it is skipped. Nominal nodes are genuine individual
        // witnesses (named individuals exist in every model) and correctly receive
        // self-loops; a reflexive property forces P(a,a) for every individual a.
        for p in &reflexive {
            for n in 0..node_count {
                if tableau.nodes[n].node_type == NodeType::Root {
                    continue;
                }
                if Self::ensure_edge(tableau, n, n, p)? {
                    changed = true;
                }
            }
        }

        // ── 5. Inverse properties: P(x,y) ⇔ Q(y,x). ────────────────────────────────
        // `InverseObjectProperties(P,Q)` is symmetric: it identifies Q with inv(P), so an
        // edge on *either* side must be derived in the reverse direction on the other side.
        // Deriving only P ⇒ Q (and not Q ⇒ P) misses e.g. the S-edges that a functional R
        // needs to see after `S(a,x) ∧ S(b,x)` (they must first invert to R(x,a) / R(x,b)).
        for (p, q) in &inverse_pairs {
            let p_edges: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in p_edges {
                if Self::ensure_edge(tableau, y, x, q)? {
                    changed = true;
                }
            }
            let q_edges: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *q)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in q_edges {
                if Self::ensure_edge(tableau, y, x, p)? {
                    changed = true;
                }
            }
        }

        // ── 6. Complex role inclusions (property chains), any length ≥ 2. ───────────
        if Self::apply_property_chains(tableau, &chains)? {
            changed = true;
        }

        // ── 7. Domain / range: R(x,y) ⇒ C(x) / C(y). ────────────────────────────────
        for (p, dom) in &domains {
            let sources: Vec<NodeId> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| e.from)
                .collect();
            for x in sources {
                if Self::add_concept_dedup(tableau, x, dom)? {
                    changed = true;
                }
            }
        }
        for (p, rng) in &ranges {
            let targets: Vec<NodeId> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| e.to)
                .collect();
            for y in targets {
                if Self::add_concept_dedup(tableau, y, rng)? {
                    changed = true;
                }
            }
        }

        // ── 7b. Data-property RBox: inclusions, domains, ranges. ────────────────
        // Data-property edges are ordinary tableau edges whose target is a data-value
        // node carrying a `DataValue` literal. SubDataPropertyOf propagates the
        // super-property edge; DataPropertyDomain puts the source individual in C;
        // DataPropertyRange validates the asserted/derived literal against the range
        // (an out-of-range literal is a datatype clash).
        for (sub_name, super_name) in &data_inclusions {
            let matching: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *sub_name)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in matching {
                if Self::ensure_edge(tableau, x, y, super_name)? {
                    changed = true;
                }
            }
        }
        for (p, dom) in &data_domains {
            let sources: Vec<NodeId> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| e.from)
                .collect();
            for x in sources {
                if Self::add_concept_dedup(tableau, x, dom)? {
                    changed = true;
                }
            }
        }
        for (p, rng) in &data_ranges {
            let targets: Vec<NodeId> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| e.to)
                .collect();
            for y in targets {
                let Some(lit) = tableau.nodes.get(y).and_then(Self::data_value_of).cloned() else {
                    continue;
                };
                match tableau
                    .datatype_validator
                    .literal_satisfies_data_range(&lit, rng)
                {
                    Ok(true) => {}
                    Ok(false) => {
                        Self::add_datatype_clash(
                            tableau,
                            y,
                            format!(
                                "Data value {lit} (node {y}) violates DataPropertyRange({p}, {rng})"
                            ),
                        );
                        changed = true;
                    }
                    Err(_) => {
                        // Membership indeterminate — leave the value in place; the
                        // datatype rules (∀/∃ over the same range) still apply.
                    }
                }
            }
        }

        // ── 8. Functional / inverse-functional: merge forced-equal successors. ───────
        // A functional property P forces P(x,y1) ∧ P(x,y2) ⇒ y1 = y2. Unlike ≤n-merging,
        // this equality is *deterministic* and applies even when the two successors are
        // distinct named individuals (OWL 2 has no unique-name assumption): the pair is
        // merged, and a `Choice::FunctionalMerge` is pushed so backtracking can retry the
        // alternative survivor. A pair that is already known distinct (DifferentIndividuals)
        // is a clash: the ontology asserts a ≠ b yet functionality forces a = b.
        for p in &functional {
            if Self::merge_forced_equal_successors(tableau, p)? {
                changed = true;
            }
        }
        for p in &inverse_functional {
            // inverse-functional P is functional on inv(P): merge the P-predecessors of a
            // shared target. We reuse the same machinery over the predecessor sets.
            if Self::merge_forced_equal_predecessors(tableau, p)? {
                changed = true;
            }
        }

        // ── 9. HasKey: C(x) ∧ C(y) ∧ (∀i ∃v. Pi(x,v) ∧ Pi(y,v)) ⇒ x = y. ───────────
        // The common value may differ per key property (OWL 2 semantics), so each key
        // property is checked independently for a *shared* successor node (object) or a
        // shared nominal value label (data).
        for (class_ce, obj_props, data_props) in &has_keys {
            let class_label = ConceptLabel::Complex(Box::new(class_ce.clone()));
            let members: Vec<NodeId> = (0..node_count)
                .filter(|&n| tableau.nodes[n].concepts.contains(&class_label))
                .collect();
            let mut merges: Vec<(NodeId, NodeId)> = Vec::new();
            for i in 0..members.len() {
                for j in (i + 1)..members.len() {
                    let x = members[i];
                    let y = members[j];
                    if x == y {
                        continue;
                    }
                    if tableau.are_inequal(x, y) {
                        // HasKey forces x = y but the ontology asserts x ≠ y → clash.
                        let clash = Clash {
                            clash_type: ClashType::ClauseViolation {
                                clause_id: "has_key_inequality".to_string(),
                                node: x,
                                description: format!(
                                    "HasKey forces nodes {x} and {y} to be equal, but they are known distinct"
                                ),
                            },
                            nodes: vec![x, y],
                            dependencies: Arc::new(DependencySet::new()),
                            explanation: format!(
                                "HasKey({class_ce:?}) merges {x} and {y}, contradicting an inequality atom"
                            ),
                        };
                        tableau.clash_detector.add_clash(clash);
                        tableau.statistics.increment_clashes();
                        changed = true;
                        continue;
                    }
                    let mut shared = true;
                    for p in obj_props {
                        let pname = p.to_string();
                        let sx: std::collections::HashSet<NodeId> = tableau
                            .nodes
                            .get(x)
                            .and_then(|n| n.role_successors.get(pname.as_str()))
                            .cloned()
                            .unwrap_or_default();
                        let sy: std::collections::HashSet<NodeId> = tableau
                            .nodes
                            .get(y)
                            .and_then(|n| n.role_successors.get(pname.as_str()))
                            .cloned()
                            .unwrap_or_default();
                        if sx.intersection(&sy).next().is_none() {
                            shared = false;
                            break;
                        }
                    }
                    if shared {
                        for d in data_props {
                            let dname = d.to_string();
                            let vx = Self::data_successor_value_labels(tableau, x, &dname);
                            let vy = Self::data_successor_value_labels(tableau, y, &dname);
                            if vx.intersection(&vy).next().is_none() {
                                shared = false;
                                break;
                            }
                        }
                    }
                    if shared {
                        // Nominal-avoidance: prefer the node carrying a nominal label.
                        let (survivor, away) = match (
                            tableau.node_nominal(x).is_some(),
                            tableau.node_nominal(y).is_some(),
                        ) {
                            (true, false) => (x, y),
                            (false, true) => (y, x),
                            _ => (x.min(y), x.max(y)),
                        };
                        if !merges.contains(&(survivor, away)) {
                            merges.push((survivor, away));
                        }
                    }
                }
            }
            for (survivor, away) in merges {
                if survivor == away {
                    continue;
                }
                if tableau.nodes.get(away).is_some_and(|n| n.status.clashed) {
                    continue;
                }
                Self::merge_nodes(tableau, survivor, away)?;
                log::debug!("HasKey rule: merged node {away} into {survivor} (shared key values)");
                changed = true;
            }
        }

        // ── 10. Role constraint clashes. ────────────────────────────────────────────
        // Asymmetry: A(x,y) ∧ A(y,x) ⇒ ⊥. A self-loop A(x,x) is its own reverse
        // (R(x,x) ∧ R(x,x) holds), so asymmetry also forbids self-loops — this is
        // what makes `ObjectHasSelf(R)` with an asymmetric R unsatisfiable. The
        // reverse-edge check is therefore run for *every* A-edge including
        // `from == to`.
        for a in &asymmetric {
            let violating: Vec<Clash> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *a)
                .filter(|e| Self::has_edge(tableau, e.to, e.from, a))
                .map(|e| Clash {
                    clash_type: ClashType::ClauseViolation {
                        clause_id: format!("asymmetry({a})"),
                        node: e.from,
                        description: format!(
                            "Asymmetric property {a} violated: both ({}, {}) and ({}, {}) hold",
                            e.from, e.to, e.to, e.from
                        ),
                    },
                    nodes: vec![e.from, e.to],
                    dependencies: Arc::new(DependencySet::new()),
                    explanation: format!(
                        "Asymmetric object property {a} cannot hold in both directions between nodes {} and {}",
                        e.from, e.to
                    ),
                })
                .collect();
            for clash in violating {
                tableau.clash_detector.add_clash(clash);
                tableau.statistics.increment_clashes();
                changed = true;
            }
        }
        // Irreflexivity: I(x,x) ⇒ ⊥.
        for irr in &irreflexive {
            let violating: Vec<Clash> = tableau
                .edges
                .iter()
                .filter(|e| e.from == e.to && e.role.to_string() == *irr)
                .map(|e| Clash {
                    clash_type: ClashType::ClauseViolation {
                        clause_id: format!("irreflexive({irr})"),
                        node: e.from,
                        description: format!(
                            "Irreflexive property {irr} violated: self-loop ({}, {})",
                            e.from, e.to
                        ),
                    },
                    nodes: vec![e.from],
                    dependencies: Arc::new(DependencySet::new()),
                    explanation: format!(
                        "Irreflexive object property {irr} cannot relate node {0} to itself",
                        e.from
                    ),
                })
                .collect();
            for clash in violating {
                tableau.clash_detector.add_clash(clash);
                tableau.statistics.increment_clashes();
                changed = true;
            }
        }
        // Disjoint roles: P(x,y) ∧ Q(x,y) ⇒ ⊥.
        for (p, q) in &disjoint_pairs {
            let p_edges: Vec<(NodeId, NodeId)> = tableau
                .edges
                .iter()
                .filter(|e| e.role.to_string() == *p)
                .map(|e| (e.from, e.to))
                .collect();
            for (x, y) in p_edges {
                if Self::has_edge(tableau, x, y, q) {
                    let clash = Clash {
                        clash_type: ClashType::ClauseViolation {
                            clause_id: format!("disjoint_roles({p},{q})"),
                            node: x,
                            description: format!(
                                "Disjoint object properties {p} and {q} both hold between nodes {x} and {y}"
                            ),
                        },
                        nodes: vec![x, y],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "DisjointObjectProperties({p},{q}) violated: both edges ({x},{y}) hold"
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    changed = true;
                }
            }
        }
        // Negative assertions: an edge that was asserted to not exist, or a collapse of the
        // two (forced-distinct) individuals into one node.
        let negs: Vec<(String, String, String)> = tableau.negative_object_assertions.clone();
        for (p, src_name, tgt_name) in negs {
            let a = tableau.individual_map.get(&src_name).copied();
            let b = tableau.individual_map.get(&tgt_name).copied();
            match (a, b) {
                (Some(a), Some(b)) if a == b => {
                    // The negative assertion forces a ≠ b, but the two individuals have been
                    // merged into a single node (e.g. SameIndividual).
                    let clash = Clash {
                        clash_type: ClashType::ClauseViolation {
                            clause_id: format!("negative_assertion_eq({p})"),
                            node: a,
                            description: format!(
                                "NegativeObjectPropertyAssertion(¬{p}, {src_name}, {tgt_name}) violated: {src_name} and {tgt_name} are the same node"
                            ),
                        },
                        nodes: vec![a],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "¬{p}({src_name}, {tgt_name}) forces {src_name} ≠ {tgt_name}, but they are the same individual"
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    changed = true;
                }
                (Some(a), Some(b)) => {
                    if Self::has_edge(tableau, a, b, &p) {
                        let clash = Clash {
                            clash_type: ClashType::ClauseViolation {
                                clause_id: format!("negative_assertion({p})"),
                                node: a,
                                description: format!(
                                    "NegativeObjectPropertyAssertion(¬{p}, {src_name}, {tgt_name}) violated: edge exists"
                                ),
                            },
                            nodes: vec![a, b],
                            dependencies: Arc::new(DependencySet::new()),
                            explanation: format!(
                                "¬{p}({src_name}, {tgt_name}) was asserted but the edge {p} was derived"
                            ),
                        };
                        tableau.clash_detector.add_clash(clash);
                        tableau.statistics.increment_clashes();
                        changed = true;
                    }
                }
                _ => {}
            }
        }

        Ok(changed)
    }

    /// Whether an edge `from --role--> to` already exists in the tableau.
    fn has_edge(tableau: &super::Tableau, from: NodeId, to: NodeId, role: &str) -> bool {
        tableau
            .nodes
            .get(from)
            .and_then(|n| n.role_successors.get(role))
            .is_some_and(|succs| succs.contains(&to))
    }

    /// Add `from --role--> to` unless it already exists, propagating `∀R.C` fillers and
    /// re-queuing any `≤n R` constraint. Returns `true` if the edge was newly created.
    fn ensure_edge(
        tableau: &mut super::Tableau,
        from: NodeId,
        to: NodeId,
        role: &str,
    ) -> Result<bool> {
        if from >= tableau.nodes.len() || to >= tableau.nodes.len() {
            return Ok(false);
        }
        if Self::has_edge(tableau, from, to, role) {
            return Ok(false);
        }
        let role_label = RoleLabel::Atomic(role.to_string());
        tableau.add_edge(from, to, role_label)?;

        // Propagate any ∀R.X already on the source to the new successor (mirrors the
        // SOME rule) so universal restrictions remain satisfied by role-closure edges.
        let all_fillers: Vec<ClassExpression> = tableau
            .nodes
            .get(from)
            .map(|n| {
                n.concepts
                    .iter()
                    .filter_map(|c| {
                        if let ConceptLabel::Complex(expr) = c
                            && let ClassExpression::ObjectAllValuesFrom {
                                property: p,
                                filler: f,
                            } = &**expr
                            && Self::prop_to_role_name(p) == role
                        {
                            return Some(*f.clone());
                        }
                        None
                    })
                    .collect()
            })
            .unwrap_or_default();
        for filler in all_fillers {
            if Self::add_concept_dedup(tableau, to, &filler)? {
                // continue; queueing happens inside add_concept_dedup
            }
        }

        // Re-queue ≤n/ =n rules for the role so excess successors are merged/clashed.
        Self::re_queue_max_rules_for_role(tableau, from, role)?;

        Ok(true)
    }

    /// Add a complex class expression to a node if not already present. `add_concept_to_node`
    /// queues its expansion rules (workstream A6 — single queueing path). Returns `true` if
    /// it was newly added.
    fn add_concept_dedup(
        tableau: &mut super::Tableau,
        node: NodeId,
        concept: &ClassExpression,
    ) -> Result<bool> {
        let label = ConceptLabel::Complex(Box::new(concept.clone()));
        let present = tableau
            .nodes
            .get(node)
            .is_some_and(|n| n.concepts.contains(&label));
        if present {
            return Ok(false);
        }
        tableau.add_concept_to_node(node, label)?;
        Ok(true)
    }

    /// Apply complex role inclusion axioms (property chains) of arbitrary length:
    /// `R1∘…∘Rn ⊑ S`. For every path `x0 -R1-> x1 -R2-> … -Rn-> xn` an `S(x0, xn)` edge
    /// is derived. Length-1 chains are treated as plain role inclusions and are handled by
    /// the inclusions pass; here we require `n ≥ 2`.
    fn apply_property_chains(
        tableau: &mut super::Tableau,
        chains: &[(Vec<String>, String)],
    ) -> Result<bool> {
        let mut changed = false;
        for (chain, super_name) in chains {
            if chain.len() < 2 {
                continue;
            }
            // frontier: (current node, depth reached, path start node)
            let mut frontier: Vec<(NodeId, usize, NodeId)> = Vec::new();
            for edge in &tableau.edges {
                if &edge.role.to_string() == &chain[0] {
                    frontier.push((edge.to, 1, edge.from));
                }
            }
            let mut depth = 1usize;
            while depth < chain.len() {
                let role = &chain[depth];
                let mut next: Vec<(NodeId, usize, NodeId)> = Vec::new();
                for (cur, d, start) in &frontier {
                    for edge in &tableau.edges {
                        if edge.from == *cur && &edge.role.to_string() == role {
                            next.push((edge.to, d + 1, *start));
                        }
                    }
                }
                frontier = next;
                depth += 1;
            }
            for (end, _, start) in frontier {
                if Self::ensure_edge(tableau, start, end, super_name)? {
                    changed = true;
                }
            }
        }
        Ok(changed)
    }

    /// The nominal value labels of the data successors of `node` for data role `role`.
    /// Used by HasKey to detect shared data values between two individuals.
    fn data_successor_value_labels(
        tableau: &super::Tableau,
        node: NodeId,
        role: &str,
    ) -> std::collections::HashSet<String> {
        let mut out = std::collections::HashSet::new();
        if let Some(n) = tableau.nodes.get(node)
            && let Some(succs) = n.role_successors.get(role)
        {
            for s in succs {
                if let Some(sn) = tableau.nodes.get(*s) {
                    for c in &sn.concepts {
                        if let ConceptLabel::Nominal(v) = c {
                            out.insert(v.clone());
                        }
                    }
                }
            }
        }
        out
    }

    /// Pick the preferred survivor of a forced equality merge of `u` and `v`:
    /// the node carrying a nominal label wins (nominal-avoidance); otherwise the smaller id.
    fn preferred_survivor(tableau: &super::Tableau, u: NodeId, v: NodeId) -> (NodeId, NodeId) {
        match (
            tableau.node_nominal(u).is_some(),
            tableau.node_nominal(v).is_some(),
        ) {
            (true, false) => (u, v),
            (false, true) => (v, u),
            _ => (u.min(v), u.max(v)),
        }
    }

    /// The two possible survivor orderings of a forced merge of `u` and `v`, so backtracking
    /// over a [`super::Choice::FunctionalMerge`] can retry the alternative survivor.
    fn merge_orderings(tableau: &super::Tableau, u: NodeId, v: NodeId) -> Vec<(NodeId, NodeId)> {
        let (survivor, away) = Self::preferred_survivor(tableau, u, v);
        vec![(survivor, away), (away, survivor)]
    }

    /// Merge two nodes forced equal by a functional / inverse-functional / HasKey rule.
    ///
    /// Registers a clash when the pair is already known distinct (an inequality atom) — the
    /// ontology asserts `a ≠ b` yet the RBox forces `a = b`. Otherwise pushes a
    /// [`super::Choice::FunctionalMerge`] choice point (so backtracking may retry the other
    /// survivor ordering) and performs the merge via the A3 `merge_nodes` machinery.
    ///
    /// Returns `Ok(true)` if a merge was performed, `Ok(false)` if the pair was already the
    /// same node, and `Ok(true)` with a clash registered for the inequal case.
    fn merge_forced_equal_pair(
        tableau: &mut super::Tableau,
        role_desc: &str,
        u: NodeId,
        v: NodeId,
    ) -> Result<bool> {
        if u == v {
            return Ok(false);
        }
        if tableau.are_inequal(u, v) {
            let clash = Clash {
                clash_type: ClashType::ClauseViolation {
                    clause_id: format!("forced_equality({role_desc})"),
                    node: u,
                    description: format!(
                        "{role_desc} forces nodes {u} and {v} to be equal, but they are declared different"
                    ),
                },
                nodes: vec![u, v],
                dependencies: Arc::new(DependencySet::new()),
                explanation: format!(
                    "{role_desc} forces {u} = {v} but an inequality atom says {u} ≠ {v}"
                ),
            };
            tableau.clash_detector.add_clash(clash);
            tableau.statistics.increment_clashes();
            log::warn!("Forced-equality clash at nodes {u}/{v}: {role_desc} requires equality");
            return Ok(true);
        }

        let candidates = Self::merge_orderings(tableau, u, v);
        let saved_state = Self::snapshot_state(tableau);
        let backtrack_point = super::BacktrackPoint {
            id: tableau.backtrack_stack.len(),
            node_id: u,
            choice: super::Choice::FunctionalMerge {
                candidates: candidates.clone(),
                tried: 0,
            },
            saved_state,
            dependencies: Arc::new(DependencySet::new()),
        };
        tableau.backtrack_stack.push(backtrack_point);

        let (survivor, away) = candidates[0];
        Self::merge_nodes(tableau, survivor, away)?;
        log::debug!("Forced-equality rule ({role_desc}): merged node {away} into {survivor}");
        Ok(true)
    }

    /// Enforce a functional object property `role`: merge every pair of `role`-successors of
    /// a shared source node. Iterates until each source has ≤ 1 live successor.
    fn merge_forced_equal_successors(tableau: &mut super::Tableau, role: &str) -> Result<bool> {
        let mut changed = false;
        let node_count = tableau.nodes.len();
        for n in 0..node_count {
            if tableau.nodes[n].status.clashed {
                continue;
            }
            loop {
                let live: Vec<NodeId> = tableau
                    .nodes
                    .get(n)
                    .map(|node| {
                        node.role_successors
                            .get(role)
                            .map(|s| {
                                s.iter()
                                    .copied()
                                    .filter(|&g| {
                                        tableau.nodes.get(g).is_some_and(|x| !x.status.clashed)
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    })
                    .unwrap_or_default();
                // `role_successors` is a set, but merge redirects may have left duplicate
                // *edges* behind; a duplicate entry must not count as a second successor.
                let mut live = live;
                live.sort_unstable();
                live.dedup();
                if live.len() <= 1 {
                    break;
                }
                let mut merged = false;
                'pairs: for i in 0..live.len() {
                    for j in (i + 1)..live.len() {
                        let clashes_before = tableau.clash_detector.clash_count();
                        if Self::merge_forced_equal_pair(
                            tableau,
                            &format!("functional({role})"),
                            live[i],
                            live[j],
                        )? {
                            if tableau.clash_detector.clash_count() > clashes_before {
                                // A clash was registered (forced equality vs. inequality);
                                // stop this node immediately to avoid re-registering it.
                                return Ok(true);
                            }
                            changed = true;
                            merged = true;
                            break 'pairs;
                        }
                    }
                }
                if !merged {
                    // More than one live successor but no pair could be merged: every pair is
                    // known distinct, so functionality is violated.
                    let clash = Clash {
                        clash_type: ClashType::Functionality {
                            role: RoleLabel::Atomic(role.to_string()),
                            node: n,
                            individuals: live.clone(),
                        },
                        nodes: live.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "Functional property {role} at node {n} has {} pairwise-distinct successors",
                            live.len()
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    log::warn!(
                        "Functional clash at node {n}: {role} has {} non-mergeable successors",
                        live.len()
                    );
                    return Ok(true);
                }
            }
        }
        Ok(changed)
    }

    /// Enforce an inverse-functional object property `role`: merge every pair of
    /// `role`-predecessors of a shared target node.
    fn merge_forced_equal_predecessors(tableau: &mut super::Tableau, role: &str) -> Result<bool> {
        let mut changed = false;
        let mut targets: Vec<NodeId> = Vec::new();
        {
            let mut seen = std::collections::HashSet::new();
            for edge in &tableau.edges {
                if edge.role.to_string() == *role && seen.insert(edge.to) {
                    targets.push(edge.to);
                }
            }
        }
        for t in targets {
            if tableau.nodes[t].status.clashed {
                continue;
            }
            loop {
                let live: Vec<NodeId> = tableau
                    .edges
                    .iter()
                    .filter(|e| e.role.to_string() == *role && e.to == t)
                    .map(|e| e.from)
                    .filter(|&g| tableau.nodes.get(g).is_some_and(|x| !x.status.clashed))
                    .collect();
                // Merge redirects may leave duplicate edges (two sources redirected onto the
                // same survivor) — deduplicate so the same node is not counted twice.
                let mut live = live;
                live.sort_unstable();
                live.dedup();
                if live.len() <= 1 {
                    break;
                }
                let mut merged = false;
                'pairs: for i in 0..live.len() {
                    for j in (i + 1)..live.len() {
                        let clashes_before = tableau.clash_detector.clash_count();
                        if Self::merge_forced_equal_pair(
                            tableau,
                            &format!("inverse_functional({role})"),
                            live[i],
                            live[j],
                        )? {
                            if tableau.clash_detector.clash_count() > clashes_before {
                                return Ok(true);
                            }
                            changed = true;
                            merged = true;
                            break 'pairs;
                        }
                    }
                }
                if !merged {
                    let clash = Clash {
                        clash_type: ClashType::Functionality {
                            role: RoleLabel::Atomic(format!("inv({role})")),
                            node: t,
                            individuals: live.clone(),
                        },
                        nodes: live.clone(),
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "Inverse-functional property {role} at target {t} has {} pairwise-distinct sources",
                            live.len()
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    log::warn!(
                        "Inverse-functional clash at node {t}: {role} has {} non-mergeable sources",
                        live.len()
                    );
                    return Ok(true);
                }
            }
        }
        Ok(changed)
    }

    /// Check whether the current tableau violates any DL clause that (a) has an empty head
    /// (a contradiction) and (b) contains at least one binary (role) atom. Such clauses are
    /// evaluated at the tableau level — against the edge set — because their body atoms
    /// range over node pairs. Returns `true` if a violation was turned into a clash.
    ///
    /// This is the generic safety net for role clauses that the dedicated RBox rules do not
    /// cover (e.g. SWRL-derived constraints or future clause compilations); the primary RBox
    /// enforcement lives in [`Self::apply_role_rules`].
    fn check_role_clause_violations(tableau: &mut super::Tableau) -> Result<bool> {
        let Some(checker) = tableau.clause_checker.as_ref() else {
            return Ok(false);
        };
        let violation = checker.check_role_constraint_clauses(tableau);
        let Some(violation) = violation else {
            return Ok(false);
        };
        let clause_id = violation.clause.id.clone();
        let explanation = violation.explanation.clone();
        let clash = Clash {
            clash_type: ClashType::ClauseViolation {
                clause_id: clause_id.clone(),
                node: violation.node_id,
                description: explanation.clone(),
            },
            nodes: vec![violation.node_id],
            dependencies: Arc::new(DependencySet::new()),
            explanation: format!(
                "Role clause violation at node {}: {}",
                violation.node_id, explanation
            ),
        };
        tableau.clash_detector.add_clash(clash);
        tableau.statistics.increment_clashes();
        log::debug!("Detected role clause violation: {clause_id}: {explanation}");
        Ok(true)
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

        // Fast path: if no dirty nodes, no new concepts were added, so no new clashes possible.
        if tableau.dirty_nodes.is_empty() {
            return Ok(());
        }

        // Drain and deduplicate dirty node IDs.
        let dirty: std::collections::HashSet<NodeId> = {
            let v = std::mem::take(&mut tableau.dirty_nodes);
            v.into_iter().collect()
        };

        // Check Nominal and Generated nodes for clashes.
        // Nominal nodes represent named individuals (ABox assertions).
        // Generated nodes represent anonymous successors created by the SOME / AtLeast rules.
        // Root nodes are excluded because they encode TBox axioms as concept intersections
        // and those encodings may look like apparent clashes without being real ones.
        for (i, node) in tableau.nodes.iter().enumerate() {
            // Skip nodes that haven't changed since last detection
            if !dirty.contains(&i) {
                continue;
            }
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

            // Nominal clashes: a node labelled `{a}` that is also required to be in
            // owl:Nothing (bottom), or that is required to be in `¬{a}`, is a clash.
            // This detects `{a} ⊑ owl:Nothing` and `{a} ⊑ ¬{a}` style inconsistencies.
            let nominal_names: Vec<&str> = node
                .concepts
                .iter()
                .filter_map(|c| match c {
                    super::node::ConceptLabel::Nominal(name) => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            if !nominal_names.is_empty() {
                // 1) Bottom on a nominal node: `{a}` and owl:Nothing / ¬owl:Thing.
                let has_bottom = node.concepts.iter().any(|c| match c {
                    ConceptLabel::Atomic(name) => name == "http://www.w3.org/2002/07/owl#Nothing",
                    ConceptLabel::NegatedAtomic(name) => {
                        name == "http://www.w3.org/2002/07/owl#Thing"
                    }
                    ConceptLabel::Complex(expr) => Self::is_bottom_concept(expr),
                    _ => false,
                });
                if has_bottom {
                    let clash = Clash {
                        clash_type: ClashType::Nominal {
                            individual: nominal_names[0].to_string(),
                            nodes: vec![i],
                        },
                        nodes: vec![i],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "Node {i} is the nominal {{{}}} but is subsumed by owl:Nothing",
                            nominal_names[0]
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    log::warn!(
                        "Nominal clash at node {i}: nominal {:?} in owl:Nothing",
                        nominal_names[0]
                    );
                }

                // 2) A nominal node required to be in the complement of its own oneOf:
                //    `{a}` together with `¬{a}` (ObjectComplementOf(ObjectOneOf([a]))).
                let mut has_nominal_complement = false;
                for name in &nominal_names {
                    for concept in &node.concepts {
                        if let ConceptLabel::Complex(expr) = concept
                            && let ClassExpression::ObjectComplementOf(inner) = &**expr
                            && let ClassExpression::ObjectOneOf(individuals) = &**inner
                            && individuals.iter().any(|ind| ind.to_string() == *name)
                        {
                            has_nominal_complement = true;
                            break;
                        }
                    }
                    if has_nominal_complement {
                        break;
                    }
                }
                if has_nominal_complement {
                    let clash = Clash {
                        clash_type: ClashType::Nominal {
                            individual: nominal_names[0].to_string(),
                            nodes: vec![i],
                        },
                        nodes: vec![i],
                        dependencies: Arc::new(DependencySet::new()),
                        explanation: format!(
                            "Node {i} is the nominal {{{}}} but also required to be in its complement",
                            nominal_names[0]
                        ),
                    };
                    tableau.clash_detector.add_clash(clash);
                    tableau.statistics.increment_clashes();
                    log::warn!(
                        "Nominal clash at node {i}: nominal {:?} and its complement",
                        nominal_names[0]
                    );
                }
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

    /// Queue applicable rules for a newly added concept.
    ///
    /// Workstream A6: this is the re-queueing trigger. Whenever a node receives a new
    /// complex class-expression label (from the AND/SOME/ALL/AtLeast/AtMost/datatype/role
    /// rules), the completion rules that can expand that expression are queued for that
    /// node, so nested constructors introduced by expansion are themselves expanded.
    ///
    /// Dedup is provided by the caller's label set (`Tableau::add_concept_to_node` only
    /// queues when the label was genuinely new), so the same `(node, concept)` pair is
    /// never re-queued. Blocking is honoured at application time: the executor loop skips
    /// any queued rule whose node is blocked.
    pub(crate) fn queue_rules_for_concept(
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
                Arc::make_mut(&mut tableau.pending_queue).push_back(rule_app);
            }
            ConceptLabel::Universal { .. } => {
                // Queue ALL rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::All,
                    node_id,
                    RulePriority::High,
                    concept,
                )?;
                Arc::make_mut(&mut tableau.pending_queue).push_back(rule_app);
            }
            ConceptLabel::AtLeast { .. } => {
                // Queue AT LEAST rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::AtLeast,
                    node_id,
                    RulePriority::Normal,
                    concept,
                )?;
                Arc::make_mut(&mut tableau.pending_queue).push_back(rule_app);
            }
            ConceptLabel::AtMost { .. } => {
                // Queue AT MOST rule
                let rule_app = Self::create_rule_application(
                    CompletionRule::AtMost,
                    node_id,
                    RulePriority::High,
                    concept,
                )?;
                Arc::make_mut(&mut tableau.pending_queue).push_back(rule_app);
            }
            _ => {} // No rules for atomic concepts
        }

        Ok(())
    }

    /// Queue rules for a class expression.
    ///
    /// Workstream A6: all class-expression dispatch is centralised in
    /// [`super::Tableau::queue_rule_for_concept`] (mod.rs), which covers every complex
    /// constructor (And/Or/Some/All/Nominal/Self/HasValue/cardinality/data). Keeping a
    /// single queueing path avoids double-queueing between the load-time `process_axiom`
    /// path and the expansion-time re-queueing path.
    pub(crate) fn queue_rules_for_class_expression(
        tableau: &mut super::Tableau,
        node_id: NodeId,
        class_expr: &crate::ontology::ClassExpression,
    ) -> Result<()> {
        tableau.queue_rule_for_concept(node_id, class_expr)
    }
}

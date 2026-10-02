//! Clause checking during tableau expansion
//!
//! This module checks DL clauses against the current tableau state
//! to detect violations dynamically during reasoning.

use crate::Error;
use crate::core::tableau::absorption::ClauseAbsorber;
use crate::core::tableau::clause_index::ClauseIndex;
use crate::core::tableau::disjointness::DisjointnessMap;
use crate::core::tableau::equivalence::{ConceptId, EquivalenceClosure};
use crate::core::tableau::incremental_checker::{
    CachedCheckResult, ChangeTracker, CheckResultCache, NodeFingerprint,
};
use crate::core::tableau::node::{ConceptLabel, NodeId, TableauNode};
use crate::dl_clauses::{DLAtom, DLClause, DLClauseSet};
use std::collections::{HashMap, HashSet};

/// Configuration for clause checking optimizations
#[derive(Debug, Clone)]
pub struct ClauseCheckerConfig {
    /// Enable clause indexing for faster lookup
    pub enable_indexing: bool,

    /// Enable incremental checking with LRU cache
    pub enable_incremental: bool,

    /// Cache capacity for incremental checking (number of entries)
    pub cache_capacity: usize,

    /// Enable clause absorption (not yet implemented)
    pub enable_absorption: bool,
}

impl Default for ClauseCheckerConfig {
    fn default() -> Self {
        Self {
            enable_indexing: true,    // Enabled by default (2.28x speedup)
            enable_incremental: true, // NOW ENABLED: Provides 2-4x additional speedup
            cache_capacity: 10_000,   // 10K entries ≈ 880 KB
            enable_absorption: false,
        }
    }
}

/// Checks DL clauses during tableau expansion
pub struct ClauseChecker {
    /// DL clauses to check (either original or remaining after absorption)
    clauses: DLClauseSet,

    /// Clause absorber for optimization (optional)
    absorber: Option<ClauseAbsorber>,

    /// Equivalence closure for reasoning about equivalent concepts
    equivalence_closure: Option<EquivalenceClosure>,

    /// Disjointness map for reasoning about disjoint concepts
    disjointness_map: Option<DisjointnessMap>,

    /// Clause index for fast predicate-based lookup (optional optimization)
    clause_index: Option<ClauseIndex>,

    /// Check result cache for incremental checking (optional optimization)
    check_cache: Option<CheckResultCache>,

    /// Change tracker for cache invalidation (optional optimization)
    change_tracker: Option<ChangeTracker>,

    /// Configuration for optimizations
    config: ClauseCheckerConfig,
}

/// Represents a clause violation detected during checking
#[derive(Debug, Clone)]
pub struct ClauseViolation {
    /// The clause that was violated
    pub clause: DLClause,

    /// Concepts involved in the violation
    pub violating_concepts: Vec<String>,

    /// Explanation of why the clause was violated
    pub explanation: String,

    /// Node ID where violation occurred
    pub node_id: usize,
}

impl ClauseChecker {
    /// Create a new clause checker
    #[must_use]
    pub fn new(clauses: DLClauseSet) -> Self {
        Self::with_config(clauses, ClauseCheckerConfig::default())
    }

    /// Create clause checker with custom configuration
    #[must_use]
    pub fn with_config(clauses: DLClauseSet, config: ClauseCheckerConfig) -> Self {
        // Apply clause absorption if enabled
        let (working_clauses, absorber) = if config.enable_absorption {
            let absorber = ClauseAbsorber::absorb(&clauses);
            log::info!(
                "Clause absorption: {}/{} clauses absorbed ({:.1}%)",
                absorber.stats().absorbed_count,
                absorber.stats().total_clauses,
                absorber.stats().absorption_rate * 100.0
            );

            // Create a new clause set with only the remaining clauses
            let remaining_set = DLClauseSet {
                deterministic_clauses: absorber.remaining_clauses().to_vec(),
                disjunctive_clauses: clauses.disjunctive_clauses.clone(),
                abox_facts: clauses.abox_facts.clone(),
                prefixes: clauses.prefixes.clone(),
                statistics: clauses.statistics.clone(),
            };

            (remaining_set, Some(absorber))
        } else {
            (clauses.clone(), None)
        };

        let clause_index = if config.enable_indexing {
            Some(ClauseIndex::from_clause_set(&working_clauses))
        } else {
            None
        };

        let check_cache = if config.enable_incremental {
            Some(CheckResultCache::new(config.cache_capacity))
        } else {
            None
        };

        let change_tracker = if config.enable_incremental {
            Some(ChangeTracker::new())
        } else {
            None
        };

        Self {
            clauses: working_clauses,
            absorber,
            equivalence_closure: None,
            disjointness_map: None,
            clause_index,
            check_cache,
            change_tracker,
            config,
        }
    }

    /// Create clause checker with equivalence and disjointness information
    #[must_use]
    pub fn with_reasoning_support(
        clauses: DLClauseSet,
        equivalence_closure: EquivalenceClosure,
        disjointness_map: DisjointnessMap,
    ) -> Self {
        let config = ClauseCheckerConfig::default();

        // Apply clause absorption if enabled
        let (working_clauses, absorber) = if config.enable_absorption {
            let absorber = ClauseAbsorber::absorb(&clauses);
            log::info!(
                "Clause absorption: {}/{} clauses absorbed ({:.1}%)",
                absorber.stats().absorbed_count,
                absorber.stats().total_clauses,
                absorber.stats().absorption_rate * 100.0
            );

            // Create a new clause set with only the remaining clauses
            let remaining_set = DLClauseSet {
                deterministic_clauses: absorber.remaining_clauses().to_vec(),
                disjunctive_clauses: clauses.disjunctive_clauses.clone(),
                abox_facts: clauses.abox_facts.clone(),
                prefixes: clauses.prefixes.clone(),
                statistics: clauses.statistics.clone(),
            };

            (remaining_set, Some(absorber))
        } else {
            (clauses.clone(), None)
        };

        let clause_index = if config.enable_indexing {
            Some(ClauseIndex::from_clause_set(&working_clauses))
        } else {
            None
        };

        let check_cache = if config.enable_incremental {
            Some(CheckResultCache::new(config.cache_capacity))
        } else {
            None
        };

        let change_tracker = if config.enable_incremental {
            Some(ChangeTracker::new())
        } else {
            None
        };

        Self {
            clauses: working_clauses,
            absorber,
            equivalence_closure: Some(equivalence_closure),
            disjointness_map: Some(disjointness_map),
            clause_index,
            check_cache,
            change_tracker,
            config,
        }
    }

    /// Create clause checker with full configuration options
    #[must_use]
    pub fn with_full_config(
        clauses: DLClauseSet,
        equivalence_closure: Option<EquivalenceClosure>,
        disjointness_map: Option<DisjointnessMap>,
        config: ClauseCheckerConfig,
    ) -> Self {
        // Apply clause absorption if enabled
        let (working_clauses, absorber) = if config.enable_absorption {
            let absorber = ClauseAbsorber::absorb(&clauses);
            log::info!(
                "Clause absorption: {}/{} clauses absorbed ({:.1}%)",
                absorber.stats().absorbed_count,
                absorber.stats().total_clauses,
                absorber.stats().absorption_rate * 100.0
            );

            // Create a new clause set with only the remaining clauses
            let remaining_set = DLClauseSet {
                deterministic_clauses: absorber.remaining_clauses().to_vec(),
                disjunctive_clauses: clauses.disjunctive_clauses.clone(),
                abox_facts: clauses.abox_facts.clone(),
                prefixes: clauses.prefixes.clone(),
                statistics: clauses.statistics.clone(),
            };

            (remaining_set, Some(absorber))
        } else {
            (clauses.clone(), None)
        };

        let clause_index = if config.enable_indexing {
            Some(ClauseIndex::from_clause_set(&working_clauses))
        } else {
            None
        };

        let check_cache = if config.enable_incremental {
            Some(CheckResultCache::new(config.cache_capacity))
        } else {
            None
        };

        let change_tracker = if config.enable_incremental {
            Some(ChangeTracker::new())
        } else {
            None
        };

        Self {
            clauses: working_clauses,
            absorber,
            equivalence_closure,
            disjointness_map,
            clause_index,
            check_cache,
            change_tracker,
            config,
        }
    }

    /// Check if a node violates any clauses
    ///
    /// Returns the first violation found, or None if no violations
    ///
    /// This method uses caching when incremental checking is enabled.
    /// For immutable checking (without cache updates), use `check_node_immutable()`.
    pub fn check_node(&mut self, node: &TableauNode) -> Option<ClauseViolation> {
        log::trace!("Checking node {} for clause violations", node.id);

        // Compute fingerprint for caching
        let fingerprint = if self.config.enable_incremental {
            Some(NodeFingerprint::from_node(node))
        } else {
            None
        };

        // Check deterministic clauses (Horn clauses)
        if let Some(violation) = self.check_deterministic_clauses_cached(node, fingerprint) {
            return Some(violation);
        }

        // Check negative clauses (⊥ in head - these indicate inconsistency)
        if let Some(violation) = self.check_negative_clauses_cached(node, fingerprint) {
            return Some(violation);
        }

        // Check disjointness constraints (no caching yet)
        if let Some(violation) = self.check_disjointness_violations(node) {
            return Some(violation);
        }

        // Record this check for change tracking
        if let (Some(tracker), Some(fp)) = (&mut self.change_tracker, fingerprint) {
            tracker.record_check(node.id, fp);
        }

        None
    }

    /// Check node immutably (without cache updates)
    ///
    /// Use this when you need immutable access or don't want to update cache.
    #[must_use]
    pub fn check_node_immutable(&self, node: &TableauNode) -> Option<ClauseViolation> {
        log::trace!(
            "Checking node {} for clause violations (immutable)",
            node.id
        );

        // Check deterministic clauses (Horn clauses)
        if let Some(violation) = self.check_deterministic_clauses(node) {
            return Some(violation);
        }

        // Check negative clauses (⊥ in head - these indicate inconsistency)
        if let Some(violation) = self.check_negative_clauses(node) {
            return Some(violation);
        }

        // Check disjointness constraints
        if let Some(violation) = self.check_disjointness_violations(node) {
            return Some(violation);
        }

        None
    }

    /// Check deterministic clauses (single head)
    ///
    /// For clauses like: C(x) ← A1(x), A2(x), ..., An(x)
    /// If all body atoms are satisfied, head should be derivable
    fn check_deterministic_clauses(&self, node: &TableauNode) -> Option<ClauseViolation> {
        // Get the clauses to check (either from index or all clauses)
        let clauses_to_check: Vec<&DLClause> = if let Some(index) = &self.clause_index {
            // Use index to get candidate clauses based on node's atomic concepts
            let predicates: Vec<String> = node
                .concepts
                .iter()
                .filter_map(|c| match c {
                    ConceptLabel::Atomic(name) => Some(name.clone()),
                    _ => None,
                })
                .collect();

            if !predicates.is_empty() {
                let mut candidates = index.get_candidate_clause_refs(&predicates);

                // IMPORTANT: Also include clauses with empty bodies (they always apply!)
                for clause in index.deterministic_clauses() {
                    if clause.body.is_empty()
                        && !clause.head.is_empty()
                        && !candidates.iter().any(|c| c.id == clause.id)
                    {
                        candidates.push(clause);
                    }
                }

                candidates
            } else {
                // No atomic concepts, but still check clauses with empty bodies
                index
                    .deterministic_clauses()
                    .iter()
                    .filter(|c| c.body.is_empty())
                    .collect()
            }
        } else {
            // No index - check all clauses (O(n) approach)
            self.clauses.deterministic_clauses.iter().collect()
        };

        log::trace!(
            "Checking {} deterministic clauses (out of {} total) for node {}",
            clauses_to_check.len(),
            self.clauses.deterministic_clauses.len(),
            node.id
        );

        for clause in clauses_to_check {
            // Skip if empty head (handled by negative clause checking)
            if clause.head.is_empty() {
                continue;
            }

            // Workstream A5: clauses mentioning binary (role) atoms are evaluated at the
            // tableau level (against the edge set), not per-node. A per-node evaluation would
            // spuriously report role-assertion facts like `P(a,b)` (empty body) as violated.
            if Self::clause_has_binary_atom_anywhere(clause) {
                continue;
            }

            // Check if body is satisfied
            if !self.matches_body(node, &clause.body) {
                continue;
            }

            log::trace!("Clause body satisfied for clause: {}", clause.id);

            // Body satisfied - check if head is also satisfied
            let head_satisfied = clause
                .head
                .iter()
                .all(|head_atom| self.matches_atom(node, head_atom));

            if !head_satisfied {
                // Violation: body satisfied but head not satisfied
                log::warn!(
                    "Deterministic clause violated at node {}: body satisfied but head not satisfied",
                    node.id
                );

                let violating_concepts: Vec<String> = clause
                    .body
                    .iter()
                    .map(|atom| atom.predicate.clone())
                    .collect();

                return Some(ClauseViolation {
                    clause: clause.clone(),
                    violating_concepts,
                    explanation: format!(
                        "Deterministic clause violated: body satisfied but head not satisfied for clause {}",
                        clause.id
                    ),
                    node_id: node.id,
                });
            }
        }

        None
    }

    /// Check negative clauses (clauses with empty head deriving ⊥)
    ///
    /// For clauses like: ⊥ ← A1(x), A2(x), ..., An(x)
    /// If all body atoms are satisfied, we have inconsistency
    fn check_negative_clauses(&self, node: &TableauNode) -> Option<ClauseViolation> {
        // Get negative clauses (either from index or filter manually)
        let negative_clauses: Vec<&DLClause> = if let Some(index) = &self.clause_index {
            // Use index to get negative clauses directly
            index.get_negative_clauses()
        } else {
            // No index - filter manually
            self.clauses
                .deterministic_clauses
                .iter()
                .filter(|c| c.head.is_empty())
                .collect()
        };

        log::trace!(
            "Checking {} negative clauses for node {}",
            negative_clauses.len(),
            node.id
        );

        for clause in negative_clauses {
            // Workstream A5: role-atom constraint clauses are checked at the tableau level.
            if Self::clause_has_binary_atom_anywhere(clause) {
                continue;
            }

            // Check if body is satisfied
            if !self.matches_body(node, &clause.body) {
                continue;
            }

            // Body satisfied with empty head = contradiction!
            log::warn!(
                "Negative clause violated at node {}: {}",
                node.id,
                clause.id
            );

            let violating_concepts: Vec<String> = clause
                .body
                .iter()
                .map(|atom| atom.predicate.clone())
                .collect();

            return Some(ClauseViolation {
                clause: clause.clone(),
                violating_concepts,
                explanation: format!(
                    "negative clause violated: all body atoms {} are satisfied, deriving ⊥ (contradiction)",
                    clause
                        .body
                        .iter()
                        .map(|a| format!("{}({})", a.predicate, a.arguments.join(", ")))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                node_id: node.id,
            });
        }

        None
    }

    /// Check deterministic clauses with caching
    ///
    /// This is the cached version of `check_deterministic_clauses` that uses
    /// the incremental cache to avoid redundant checks.
    fn check_deterministic_clauses_cached(
        &mut self,
        node: &TableauNode,
        fingerprint: Option<NodeFingerprint>,
    ) -> Option<ClauseViolation> {
        // If no cache or no fingerprint, fall back to regular checking
        if self.check_cache.is_none() || fingerprint.is_none() {
            return self.check_deterministic_clauses(node);
        }

        let fp = fingerprint
            .ok_or_else(|| Error::internal("Clause checker: fingerprint is None despite check"))
            .ok()?;

        // Get clauses to check
        let clauses_to_check: Vec<DLClause> = if let Some(index) = &self.clause_index {
            let predicates: Vec<String> = node
                .concepts
                .iter()
                .filter_map(|c| match c {
                    ConceptLabel::Atomic(name) => Some(name.clone()),
                    _ => None,
                })
                .collect();

            if !predicates.is_empty() {
                let mut candidates = index
                    .get_candidate_clause_refs(&predicates)
                    .into_iter()
                    .cloned()
                    .collect::<Vec<_>>();

                // Include clauses with empty bodies
                for clause in index.deterministic_clauses() {
                    if clause.body.is_empty()
                        && !clause.head.is_empty()
                        && !candidates.iter().any(|c| c.id == clause.id)
                    {
                        candidates.push(clause.clone());
                    }
                }

                candidates
            } else {
                index
                    .deterministic_clauses()
                    .iter()
                    .filter(|c| c.body.is_empty())
                    .cloned()
                    .collect()
            }
        } else {
            self.clauses.deterministic_clauses.clone()
        };

        // Get clause ID to usize mapping (use hash of clause id string)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        for clause in &clauses_to_check {
            // Skip empty heads
            if clause.head.is_empty() {
                continue;
            }

            // Workstream A5: role-atom clauses are checked at the tableau level.
            if Self::clause_has_binary_atom_anywhere(clause) {
                continue;
            }

            // Compute clause ID hash
            let mut hasher = DefaultHasher::new();
            clause.id.hash(&mut hasher);
            let clause_id_hash = hasher.finish() as usize;

            // Check cache first (separate scope to release borrow)
            let cached_result = if let Some(cache) = &mut self.check_cache {
                cache.get(fp, clause_id_hash).cloned()
            } else {
                None
            };

            if let Some(cached_result) = cached_result {
                // Cache hit!
                match cached_result {
                    CachedCheckResult::NoViolation => {
                        // No violation cached - continue to next clause
                        continue;
                    }
                    CachedCheckResult::Violation {
                        clause_id,
                        description,
                    } => {
                        // Violation cached - reconstruct and return
                        log::trace!(
                            "Cache hit: violation for clause {} at node {}",
                            clause_id,
                            node.id
                        );

                        let violating_concepts: Vec<String> = clause
                            .body
                            .iter()
                            .map(|atom| atom.predicate.clone())
                            .collect();

                        return Some(ClauseViolation {
                            clause: clause.clone(),
                            violating_concepts,
                            explanation: description.clone(),
                            node_id: node.id,
                        });
                    }
                    CachedCheckResult::DisjunctSelected { .. } => {
                        // Not applicable for deterministic clauses
                        continue;
                    }
                }
            }

            // Cache miss - do actual check
            if !self.matches_body(node, &clause.body) {
                // Body not satisfied - cache and continue
                if let Some(cache) = &mut self.check_cache {
                    cache.put(fp, clause_id_hash, CachedCheckResult::NoViolation);
                }
                continue;
            }

            let head_satisfied = clause
                .head
                .iter()
                .all(|head_atom| self.matches_atom(node, head_atom));

            if !head_satisfied {
                // Violation found - cache and return
                let description = format!(
                    "Deterministic clause violated: body satisfied but head not satisfied for clause {}",
                    clause.id
                );

                if let Some(cache) = &mut self.check_cache {
                    cache.put(
                        fp,
                        clause_id_hash,
                        CachedCheckResult::Violation {
                            clause_id: clause_id_hash,
                            description: description.clone(),
                        },
                    );
                }

                let violating_concepts: Vec<String> = clause
                    .body
                    .iter()
                    .map(|atom| atom.predicate.clone())
                    .collect();

                return Some(ClauseViolation {
                    clause: clause.clone(),
                    violating_concepts,
                    explanation: description,
                    node_id: node.id,
                });
            }
            // No violation - cache this
            if let Some(cache) = &mut self.check_cache {
                cache.put(fp, clause_id_hash, CachedCheckResult::NoViolation);
            }
        }

        None
    }

    /// Check negative clauses with caching
    fn check_negative_clauses_cached(
        &mut self,
        node: &TableauNode,
        fingerprint: Option<NodeFingerprint>,
    ) -> Option<ClauseViolation> {
        // If no cache or no fingerprint, fall back to regular checking
        if self.check_cache.is_none() || fingerprint.is_none() {
            return self.check_negative_clauses(node);
        }

        let fp = fingerprint
            .ok_or_else(|| Error::internal("Clause checker: fingerprint is None despite check"))
            .ok()?;

        // Get negative clauses (clone to avoid borrow issues)
        let negative_clauses: Vec<DLClause> = if let Some(index) = &self.clause_index {
            index.get_negative_clauses().into_iter().cloned().collect()
        } else {
            self.clauses
                .deterministic_clauses
                .iter()
                .filter(|c| c.head.is_empty())
                .cloned()
                .collect()
        };

        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        for clause in &negative_clauses {
            // Workstream A5: role-atom constraint clauses are checked at the tableau level.
            if Self::clause_has_binary_atom_anywhere(clause) {
                continue;
            }

            // Compute clause ID hash
            let mut hasher = DefaultHasher::new();
            clause.id.hash(&mut hasher);
            let clause_id_hash = hasher.finish() as usize;

            // Check cache (separate scope)
            let cached_result = if let Some(cache) = &mut self.check_cache {
                cache.get(fp, clause_id_hash).cloned()
            } else {
                None
            };

            if let Some(cached_result) = cached_result {
                match cached_result {
                    CachedCheckResult::NoViolation => {
                        continue;
                    }
                    CachedCheckResult::Violation {
                        clause_id,
                        description,
                    } => {
                        log::trace!(
                            "Cache hit: negative clause violation for {} at node {}",
                            clause_id,
                            node.id
                        );

                        let violating_concepts: Vec<String> = clause
                            .body
                            .iter()
                            .map(|atom| atom.predicate.clone())
                            .collect();

                        return Some(ClauseViolation {
                            clause: clause.clone(),
                            violating_concepts,
                            explanation: description.clone(),
                            node_id: node.id,
                        });
                    }
                    CachedCheckResult::DisjunctSelected { .. } => {
                        continue;
                    }
                }
            }

            // Cache miss - do actual check
            if !self.matches_body(node, &clause.body) {
                if let Some(cache) = &mut self.check_cache {
                    cache.put(fp, clause_id_hash, CachedCheckResult::NoViolation);
                }
                continue;
            }

            // Body satisfied - violation!
            let description = format!(
                "negative clause violated: all body atoms {} are satisfied, deriving ⊥ (contradiction)",
                clause
                    .body
                    .iter()
                    .map(|a| format!("{}({})", a.predicate, a.arguments.join(", ")))
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            if let Some(cache) = &mut self.check_cache {
                cache.put(
                    fp,
                    clause_id_hash,
                    CachedCheckResult::Violation {
                        clause_id: clause_id_hash,
                        description: description.clone(),
                    },
                );
            }

            let violating_concepts: Vec<String> = clause
                .body
                .iter()
                .map(|atom| atom.predicate.clone())
                .collect();

            return Some(ClauseViolation {
                clause: clause.clone(),
                violating_concepts,
                explanation: description,
                node_id: node.id,
            });
        }

        None
    }

    /// Check if node concepts violate disjointness constraints
    fn check_disjointness_violations(&self, node: &TableauNode) -> Option<ClauseViolation> {
        let disj_map = self.disjointness_map.as_ref()?;

        // Check all pairs of atomic concepts in the node
        let atomic_concepts: Vec<String> = node
            .concepts
            .iter()
            .filter_map(|c| match c {
                ConceptLabel::Atomic(name) => Some(name.clone()),
                _ => None,
            })
            .collect();

        for i in 0..atomic_concepts.len() {
            for j in (i + 1)..atomic_concepts.len() {
                let c1 = ConceptId(atomic_concepts[i].clone());
                let c2 = ConceptId(atomic_concepts[j].clone());

                if disj_map.are_disjoint(&c1, &c2) {
                    log::warn!(
                        "Disjointness violation at node {}: {} and {} are disjoint",
                        node.id,
                        c1.0,
                        c2.0
                    );

                    return Some(ClauseViolation {
                        clause: DLClause {
                            head: vec![],
                            body: vec![
                                DLAtom::concept_assertion(&c1.0, "x"),
                                DLAtom::concept_assertion(&c2.0, "x"),
                            ],
                            variables: HashSet::from(["x".to_string()]),
                            id: format!("disjoint_{}_{}", c1.0, c2.0),
                        },
                        violating_concepts: vec![c1.0, c2.0],
                        explanation: format!(
                            "Disjointness violation: concepts {} and {} are both present but declared disjoint",
                            atomic_concepts[i], atomic_concepts[j]
                        ),
                        node_id: node.id,
                    });
                }
            }
        }

        None
    }

    /// Check if node satisfies all body atoms of a clause
    ///
    /// Body is satisfied if all positive atoms are present as concepts
    /// and all negative atoms are absent
    fn matches_body(&self, node: &TableauNode, body: &[DLAtom]) -> bool {
        for atom in body {
            if !self.matches_atom(node, atom) {
                return false;
            }
        }
        true
    }

    /// Check if a single atom matches the node state
    fn matches_atom(&self, node: &TableauNode, atom: &DLAtom) -> bool {
        // Workstream A5: a binary atom `R(x, x)` whose two arguments are the *same* variable
        // is evaluated against the node's self-loop edges (an R-successor that is the node
        // itself). This is the only binary-atom case the per-node checker can decide on its
        // own; general binary atoms range over node *pairs* and are evaluated at the tableau
        // level by `check_role_constraint_clauses`.
        if atom.arguments.len() == 2 && atom.arguments[0] == atom.arguments[1] {
            let present = node
                .role_successors
                .get(atom.predicate.as_str())
                .is_some_and(|s| s.contains(&node.id));
            return if atom.is_positive { present } else { !present };
        }

        // For now, only handle unary predicates (concept assertions)
        if atom.arguments.len() != 1 {
            log::trace!("Skipping non-unary atom: {atom:?}");
            return false;
        }

        // Check if concept is present in node
        let concept_present = node.concepts.iter().any(|c| match c {
            ConceptLabel::Atomic(name) => name == &atom.predicate,
            _ => false,
        });

        // For positive atoms, concept should be present
        // For negative atoms, concept should be absent
        if atom.is_positive {
            concept_present
        } else {
            !concept_present
        }
    }

    /// Whether a clause mentions any binary (role) atom or equality atom in its body or head.
    fn clause_has_binary_atom_anywhere(clause: &DLClause) -> bool {
        clause.body.iter().chain(clause.head.iter()).any(|a| {
            a.arguments.len() >= 2 || (a.arguments.is_empty() && a.predicate.starts_with('['))
        })
    }

    /// Whether a clause body mentions any binary (role) atom or equality atom.
    fn clause_has_binary_atom(clause: &DLClause) -> bool {
        clause.body.iter().any(|a| {
            a.arguments.len() >= 2 || (a.arguments.is_empty() && a.predicate.starts_with('['))
        })
    }

    /// The deterministic clauses that act as role-level constraints: empty head (a
    /// contradiction) and a body containing at least one binary (role) atom.
    pub fn role_constraint_clauses(&self) -> Vec<&DLClause> {
        self.clauses
            .deterministic_clauses
            .iter()
            .filter(|c| c.head.is_empty() && Self::clause_has_binary_atom(c))
            .collect()
    }

    /// The deterministic clauses whose body contains at least one binary (role) atom,
    /// including head-bearing rules (transitivity, symmetry, role inclusions, …). These are
    /// the clauses that the per-node checker cannot evaluate and that the executor applies
    /// via [`crate::core::tableau::executor::TableauExecutor::apply_role_rules`].
    pub fn role_body_clauses(&self) -> Vec<&DLClause> {
        self.clauses
            .deterministic_clauses
            .iter()
            .filter(|c| Self::clause_has_binary_atom(c))
            .collect()
    }

    /// Check role-level *constraint* clauses (empty head, at least one binary atom in the
    /// body) against the current tableau edge set.
    ///
    /// **Architecture choice (workstream A5):** the clause checker is invoked *per-node* by
    /// the executor, which is the right granularity for unary clauses. Binary atoms range
    /// over node *pairs* connected by edges, so clauses containing them are evaluated at the
    /// **tableau level** instead — this method — and the executor calls it once per loop
    /// iteration. Role-hierarchy is honoured when matching: an edge labelled `r` also
    /// satisfies an atom whose predicate is any transitive super-property of `r`.
    ///
    /// Returns the first violation found (a body satisfied with an empty head ⇒ ⊥), or
    /// `None`. Head-bearing role clauses are *not* checked here: they derive new facts and
    /// are applied by the executor's RBox rule pass.
    pub fn check_role_constraint_clauses(
        &self,
        tableau: &super::Tableau,
    ) -> Option<ClauseViolation> {
        let clauses = self.role_constraint_clauses();
        if clauses.is_empty() {
            return None;
        }

        // Lightweight role fact base over the current tableau.
        let edges: Vec<(NodeId, NodeId, String)> = tableau
            .edges
            .iter()
            .map(|e| (e.from, e.to, e.role.to_string()))
            .collect();
        let node_count = tableau.nodes.len();
        let mut node_atomics: Vec<HashSet<String>> = Vec::with_capacity(node_count);
        for n in &tableau.nodes {
            let mut s: HashSet<String> = HashSet::new();
            for c in &n.concepts {
                match c {
                    ConceptLabel::Atomic(name) => {
                        s.insert(name.clone());
                    }
                    ConceptLabel::NegatedAtomic(name) => {
                        s.insert(format!("!{name}"));
                    }
                    ConceptLabel::Complex(expr) => {
                        if let crate::ontology::ClassExpression::Class(cl) = expr.as_ref() {
                            s.insert(cl.iri.as_str().to_string());
                        }
                    }
                    ConceptLabel::Nominal(name) => {
                        s.insert(format!("{{{name}}}"));
                    }
                    _ => {}
                }
            }
            node_atomics.push(s);
        }
        let supers = super_properties_of(tableau.ontology.as_ref());

        for clause in clauses {
            let mut subst: HashMap<String, NodeId> = HashMap::new();
            if role_body_satisfiable(
                tableau,
                &edges,
                &node_atomics,
                &supers,
                &clause.body,
                &mut subst,
            ) {
                log::warn!(
                    "Role constraint clause violated at tableau level: {}",
                    clause.id
                );
                let violating_concepts: Vec<String> = clause
                    .body
                    .iter()
                    .map(|atom| atom.predicate.clone())
                    .collect();
                return Some(ClauseViolation {
                    clause: clause.clone(),
                    violating_concepts,
                    explanation: format!(
                        "role constraint clause violated: all body atoms {} are satisfied, deriving ⊥",
                        clause
                            .body
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ),
                    node_id: subst.values().next().copied().unwrap_or(0),
                });
            }
        }
        None
    }

    /// Get statistics about the clause set
    #[must_use]
    pub fn get_statistics(&self) -> &crate::dl_clauses::DLClauseStatistics {
        &self.clauses.statistics
    }

    /// Check if clause checker has any clauses
    #[must_use]
    pub fn has_clauses(&self) -> bool {
        !self.clauses.deterministic_clauses.is_empty()
            || !self.clauses.disjunctive_clauses.is_empty()
    }

    /// Get the configuration
    #[must_use]
    pub fn config(&self) -> &ClauseCheckerConfig {
        &self.config
    }

    /// Get the clause index (if enabled)
    #[must_use]
    pub fn clause_index(&self) -> Option<&ClauseIndex> {
        self.clause_index.as_ref()
    }

    /// Check if indexing is enabled
    #[must_use]
    pub fn is_indexing_enabled(&self) -> bool {
        self.clause_index.is_some()
    }

    /// Get the check result cache (if enabled)
    #[must_use]
    pub fn check_cache(&self) -> Option<&CheckResultCache> {
        self.check_cache.as_ref()
    }

    /// Get mutable check result cache (if enabled)
    pub fn check_cache_mut(&mut self) -> Option<&mut CheckResultCache> {
        self.check_cache.as_mut()
    }

    /// Get the equivalence closure (if available)
    pub fn equivalence_closure(&mut self) -> Option<&mut EquivalenceClosure> {
        self.equivalence_closure.as_mut()
    }

    /// Get the disjointness map (if available)
    #[must_use]
    pub fn disjointness_map(&self) -> Option<&DisjointnessMap> {
        self.disjointness_map.as_ref()
    }

    /// Check if incremental checking is enabled
    #[must_use]
    pub fn is_incremental_enabled(&self) -> bool {
        self.check_cache.is_some()
    }

    /// Get the clause absorber (if enabled)
    #[must_use]
    pub fn absorber(&self) -> Option<&ClauseAbsorber> {
        self.absorber.as_ref()
    }

    /// Check if clause absorption is enabled
    #[must_use]
    pub fn is_absorption_enabled(&self) -> bool {
        self.absorber.is_some()
    }

    /// Mark a node as changed (for cache invalidation)
    ///
    /// Call this when a node's concepts or roles change to invalidate
    /// cached results for that node.
    pub fn mark_node_changed(&mut self, node_id: usize) {
        if let Some(tracker) = &mut self.change_tracker {
            tracker.mark_changed(node_id);
        }
    }

    /// Clear all cached check results
    ///
    /// Useful when ontology changes or for debugging.
    pub fn clear_cache(&mut self) {
        if let Some(cache) = &mut self.check_cache {
            cache.clear();
        }
        if let Some(tracker) = &mut self.change_tracker {
            tracker.clear();
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Tableau-level role (binary) atom matching — workstream A5.
//
// A clause body is satisfied when there exists a variable substitution mapping the
// clause variables to tableau nodes such that every body atom holds. Unary atoms
// (`C(x)`) are checked against node concept labels; binary atoms (`P(x,y)`) are
// checked against the current edge set with role-hierarchy awareness; equality atoms
// (`[a == b]`) are checked against node identity.
// ─────────────────────────────────────────────────────────────────────────────

/// Whether `atom_predicate` is satisfied by an edge labelled `edge_role`, honouring the
/// role hierarchy: an `edge_role`-edge also satisfies any transitive super-property.
fn role_matches(
    atom_predicate: &str,
    edge_role: &str,
    supers: &HashMap<String, HashSet<String>>,
) -> bool {
    if atom_predicate == edge_role {
        return true;
    }
    supers
        .get(edge_role)
        .is_some_and(|s| s.contains(atom_predicate))
}

/// The transitive sub-property closure of the ontology: `role → {super-roles}`.
/// Only simple (non-chain) sub/super property expressions are considered.
fn super_properties_of(ontology: &crate::ontology::Ontology) -> HashMap<String, HashSet<String>> {
    use crate::ontology::{Axiom, ObjectPropertyExpression};
    let mut supers: HashMap<String, HashSet<String>> = HashMap::new();
    for ax in ontology.axioms() {
        if let Axiom::SubObjectPropertyOf(a) = ax {
            if matches!(a.sub_property, ObjectPropertyExpression::PropertyChain(_))
                || matches!(a.super_property, ObjectPropertyExpression::PropertyChain(_))
            {
                continue;
            }
            let sub = a.sub_property.to_string();
            let sup = a.super_property.to_string();
            if sub != sup {
                supers.entry(sub).or_default().insert(sup);
            }
        }
    }
    // Transitive closure by fixpoint.
    loop {
        let mut changed = false;
        let keys: Vec<String> = supers.keys().cloned().collect();
        for k in &keys {
            let direct: HashSet<String> = supers.get(k).cloned().unwrap_or_default();
            let mut extra: HashSet<String> = HashSet::new();
            for m in &direct {
                if let Some(ms) = supers.get(m) {
                    extra.extend(ms.iter().cloned());
                }
            }
            if !extra.is_empty() {
                let t = supers.entry(k.clone()).or_default();
                for v in extra {
                    if t.insert(v) {
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    supers
}

/// Resolve a ground individual name in a DL clause to a tableau node id.
///
/// DL clauses store individuals in prefix-shortened form (`iri_to_string`), so the exact
/// `individual_map` lookup is tried first and a local-name suffix match on nominal labels is
/// used as a fallback. `None` means the constant is unknown to the tableau (a clause body
/// mentioning it cannot be satisfied).
fn resolve_constant(tableau: &super::Tableau, name: &str) -> Option<NodeId> {
    if let Some(&id) = tableau.individual_map.get(name) {
        return Some(id);
    }
    for (i, n) in tableau.nodes.iter().enumerate() {
        for c in &n.concepts {
            if let ConceptLabel::Nominal(nom) = c {
                if nom == name
                    || nom.ends_with(&format!("/{name}"))
                    || nom.ends_with(&format!("#{name}"))
                {
                    return Some(i);
                }
            }
        }
    }
    None
}

/// Extend `subst` by binding `arg` to `node`, or verify consistency when already bound.
/// Clause variables are uppercase; anything else is treated as a ground individual constant.
fn try_bind(
    tableau: &super::Tableau,
    subst: &HashMap<String, NodeId>,
    arg: &str,
    node: NodeId,
) -> Option<HashMap<String, NodeId>> {
    if arg.is_empty() {
        return None;
    }
    if arg.chars().next().is_some_and(char::is_uppercase) {
        if let Some(&v) = subst.get(arg) {
            if v == node {
                return Some(subst.clone());
            }
            return None;
        }
        let mut s = subst.clone();
        s.insert(arg.to_string(), node);
        Some(s)
    } else {
        // Ground constant: must resolve to exactly `node`.
        match resolve_constant(tableau, arg) {
            Some(id) if id == node => Some(subst.clone()),
            _ => None,
        }
    }
}

/// Whether the body atoms of a role clause can be satisfied against the tableau edge set.
///
/// A backtracking search over the body atoms, extending `subst` with one candidate binding
/// per atom and recursing. The first satisfying substitution is written back to `subst`.
fn role_body_satisfiable(
    tableau: &super::Tableau,
    edges: &[(NodeId, NodeId, String)],
    node_atomics: &[HashSet<String>],
    supers: &HashMap<String, HashSet<String>>,
    body: &[DLAtom],
    subst: &mut HashMap<String, NodeId>,
) -> bool {
    role_body_at_index(tableau, edges, node_atomics, supers, body, 0, subst)
}

fn role_body_at_index(
    tableau: &super::Tableau,
    edges: &[(NodeId, NodeId, String)],
    node_atomics: &[HashSet<String>],
    supers: &HashMap<String, HashSet<String>>,
    body: &[DLAtom],
    idx: usize,
    subst: &mut HashMap<String, NodeId>,
) -> bool {
    if idx == body.len() {
        return true;
    }
    let atom = &body[idx];

    if atom.arguments.len() >= 2 {
        // Binary (role) atom: enumerate candidate edges whose role matches the predicate
        // (with role-hierarchy), consistent with the current substitution.
        let (from, to) = (atom.arguments[0].clone(), atom.arguments[1].clone());
        for (e_from, e_to, e_role) in edges {
            if !role_matches(&atom.predicate, e_role, supers) {
                continue;
            }
            let Some(s) = try_bind(tableau, subst, &from, *e_from) else {
                continue;
            };
            let Some(s) = try_bind(tableau, &s, &to, *e_to) else {
                continue;
            };
            let mut s = s;
            if role_body_at_index(tableau, edges, node_atomics, supers, body, idx + 1, &mut s) {
                *subst = s;
                return true;
            }
        }
        return false;
    }

    if atom.arguments.len() == 1 {
        // Unary (concept) atom: enumerate nodes carrying the concept.
        let arg = atom.arguments[0].clone();
        for (node, concepts) in node_atomics.iter().enumerate() {
            if !concepts.contains(&atom.predicate) {
                continue;
            }
            let Some(s) = try_bind(tableau, subst, &arg, node) else {
                continue;
            };
            let mut s = s;
            if role_body_at_index(tableau, edges, node_atomics, supers, body, idx + 1, &mut s) {
                *subst = s;
                return true;
            }
        }
        return false;
    }

    // Equality atom `[a == b]`: both operands must resolve to the same node.
    let pred = &atom.predicate;
    if pred.starts_with('[') && pred.ends_with(']') {
        let inner = &pred[1..pred.len() - 1];
        let Some(eq_pos) = inner.find("==") else {
            return false;
        };
        let left = inner[..eq_pos].trim().to_string();
        let right = inner[eq_pos + 2..].trim().to_string();
        if left.is_empty() || right.is_empty() {
            return false;
        }
        // Bind left to its (resolved) node, then require right to resolve to the same.
        let left_node = if left.chars().next().is_some_and(char::is_uppercase) {
            subst.get(&left).copied()
        } else {
            resolve_constant(tableau, &left)
        };
        let right_node = if right.chars().next().is_some_and(char::is_uppercase) {
            subst.get(&right).copied()
        } else {
            resolve_constant(tableau, &right)
        };
        if let (Some(l), Some(r)) = (left_node, right_node) {
            if l == r {
                return role_body_at_index(
                    tableau,
                    edges,
                    node_atomics,
                    supers,
                    body,
                    idx + 1,
                    subst,
                );
            }
        }
        // The operands may not be bound yet in this branch — try binding via unification.
        return false;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tableau::node::NodeType;
    use std::collections::HashMap;

    fn create_test_node(id: usize, concepts: Vec<&str>) -> TableauNode {
        let mut node = TableauNode::new(id, NodeType::Individual);
        for concept in concepts {
            node.add_concept(ConceptLabel::Atomic(concept.to_string()));
        }
        node
    }

    #[test]
    fn test_no_violation_empty_clauses() {
        let clause_set = DLClauseSet {
            deterministic_clauses: vec![],
            disjunctive_clauses: vec![],
            abox_facts: vec![],
            prefixes: HashMap::new(),
            statistics: Default::default(),
        };

        let mut checker = ClauseChecker::new(clause_set);
        let node = create_test_node(0, vec!["A", "B"]);

        assert!(checker.check_node(&node).is_none());
    }

    #[test]
    fn test_negative_clause_violation() {
        // Create a negative clause: ⊥ ← A(x), B(x)
        let clause = DLClause {
            head: vec![], // Empty head = derives contradiction
            body: vec![
                DLAtom::concept_assertion("A", "x"),
                DLAtom::concept_assertion("B", "x"),
            ],
            variables: HashSet::from(["x".to_string()]),
            id: "neg_test".to_string(),
        };

        let clause_set = DLClauseSet {
            deterministic_clauses: vec![clause],
            disjunctive_clauses: vec![],
            abox_facts: vec![],
            prefixes: HashMap::new(),
            statistics: Default::default(),
        };

        let mut checker = ClauseChecker::new(clause_set);

        // Node with both A and B should violate
        let node = create_test_node(0, vec!["A", "B"]);
        let violation = checker.check_node(&node);
        assert!(
            violation.is_some(),
            "Should detect violation when both A and B present"
        );

        // Node with only A should not violate
        let node2 = create_test_node(1, vec!["A"]);
        assert!(
            checker.check_node(&node2).is_none(),
            "Should not violate with only A"
        );
    }

    #[test]
    fn test_matches_body() {
        let clause_set = DLClauseSet {
            deterministic_clauses: vec![],
            disjunctive_clauses: vec![],
            abox_facts: vec![],
            prefixes: HashMap::new(),
            statistics: Default::default(),
        };

        let checker = ClauseChecker::new(clause_set);
        let node = create_test_node(0, vec!["A", "B", "C"]);

        // Body with all present concepts
        let body1 = vec![
            DLAtom::concept_assertion("A", "x"),
            DLAtom::concept_assertion("B", "x"),
        ];
        assert!(
            checker.matches_body(&node, &body1),
            "Should match when all body atoms present"
        );

        // Body with missing concept
        let body2 = vec![
            DLAtom::concept_assertion("A", "x"),
            DLAtom::concept_assertion("D", "x"), // Not in node
        ];
        assert!(
            !checker.matches_body(&node, &body2),
            "Should not match when body atom missing"
        );
    }
}

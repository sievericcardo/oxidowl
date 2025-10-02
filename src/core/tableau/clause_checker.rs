//! Clause checking during tableau expansion
//!
//! This module checks DL clauses against the current tableau state
//! to detect violations dynamically during reasoning.

use crate::{Error, Result};
use crate::dl_clauses::{DLClause, DLClauseSet, DLAtom};
use crate::core::tableau::node::{TableauNode, ConceptLabel};
use crate::core::tableau::equivalence::{ConceptId, EquivalenceClosure};
use crate::core::tableau::disjointness::DisjointnessMap;
use std::collections::{HashMap, HashSet};

/// Checks DL clauses during tableau expansion
pub struct ClauseChecker {
    /// DL clauses to check
    clauses: DLClauseSet,
    
    /// Equivalence closure for reasoning about equivalent concepts
    equivalence_closure: Option<EquivalenceClosure>,
    
    /// Disjointness map for reasoning about disjoint concepts
    disjointness_map: Option<DisjointnessMap>,
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
    pub fn new(clauses: DLClauseSet) -> Self {
        Self {
            clauses,
            equivalence_closure: None,
            disjointness_map: None,
        }
    }
    
    /// Create clause checker with equivalence and disjointness information
    pub fn with_reasoning_support(
        clauses: DLClauseSet,
        equivalence_closure: EquivalenceClosure,
        disjointness_map: DisjointnessMap,
    ) -> Self {
        Self {
            clauses,
            equivalence_closure: Some(equivalence_closure),
            disjointness_map: Some(disjointness_map),
        }
    }
    
    /// Check if a node violates any clauses
    /// 
    /// Returns the first violation found, or None if no violations
    pub fn check_node(&self, node: &TableauNode) -> Option<ClauseViolation> {
        log::trace!("Checking node {} for clause violations", node.id);
        
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
        for clause in &self.clauses.deterministic_clauses {
            // Skip if empty head (handled by negative clause checking)
            if clause.head.is_empty() {
                continue;
            }
            
            // Check if body is satisfied
            if !self.matches_body(node, &clause.body) {
                continue;
            }
            
            log::trace!("Clause body satisfied for clause: {}", clause.id);
            
            // Body satisfied - head should be present or derivable
            // For now, we just log this (head concepts will be added by expansion rules)
            // In a complete implementation, we might add the head concepts here
        }
        
        None
    }
    
    /// Check negative clauses (clauses with empty head deriving ⊥)
    ///
    /// For clauses like: ⊥ ← A1(x), A2(x), ..., An(x)
    /// If all body atoms are satisfied, we have inconsistency
    fn check_negative_clauses(&self, node: &TableauNode) -> Option<ClauseViolation> {
        for clause in &self.clauses.deterministic_clauses {
            // Only process clauses with empty head (deriving contradiction)
            if !clause.head.is_empty() {
                continue;
            }
            
            // Check if body is satisfied
            if !self.matches_body(node, &clause.body) {
                continue;
            }
            
            // Body satisfied with empty head = contradiction!
            log::warn!("Negative clause violated at node {}: {}", node.id, clause.id);
            
            let violating_concepts: Vec<String> = clause.body
                .iter()
                .map(|atom| atom.predicate.clone())
                .collect();
            
            return Some(ClauseViolation {
                clause: clause.clone(),
                violating_concepts,
                explanation: format!(
                    "Negative clause violated: all body atoms {} are satisfied, deriving contradiction",
                    clause.body.iter()
                        .map(|a| format!("{}({})", a.predicate, a.arguments.join(", ")))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                node_id: node.id,
            });
        }
        
        None
    }
    
    /// Check if node concepts violate disjointness constraints
    fn check_disjointness_violations(&self, node: &TableauNode) -> Option<ClauseViolation> {
        if self.disjointness_map.is_none() {
            return None;
        }
        
        let disj_map = self.disjointness_map.as_ref().unwrap();
        
        // Check all pairs of atomic concepts in the node
        let atomic_concepts: Vec<String> = node.concepts
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
                        node.id, c1.0, c2.0
                    );
                    
                    return Some(ClauseViolation {
                        clause: DLClause {
                            head: vec![],
                            body: vec![
                                DLAtom::concept_assertion(&c1.0, "x"),
                                DLAtom::concept_assertion(&c2.0, "x"),
                            ],
                            variables: HashSet::from(["x".to_string()]),
                            id: format!("disjoint_{}_{}",c1.0, c2.0),
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
        // For now, only handle unary predicates (concept assertions)
        if atom.arguments.len() != 1 {
            log::trace!("Skipping non-unary atom: {:?}", atom);
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
    
    /// Get statistics about the clause set
    pub fn get_statistics(&self) -> &crate::dl_clauses::DLClauseStatistics {
        &self.clauses.statistics
    }
    
    /// Check if clause checker has any clauses
    pub fn has_clauses(&self) -> bool {
        !self.clauses.deterministic_clauses.is_empty() 
            || !self.clauses.disjunctive_clauses.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::tableau::node::{NodeType, NodeStatus, BlockingInfo};
    
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
        
        let checker = ClauseChecker::new(clause_set);
        let node = create_test_node(0, vec!["A", "B"]);
        
        assert!(checker.check_node(&node).is_none());
    }
    
    #[test]
    fn test_negative_clause_violation() {
        // Create a negative clause: ⊥ ← A(x), B(x)
        let clause = DLClause {
            head: vec![],  // Empty head = derives contradiction
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
        
        let checker = ClauseChecker::new(clause_set);
        
        // Node with both A and B should violate
        let node = create_test_node(0, vec!["A", "B"]);
        let violation = checker.check_node(&node);
        assert!(violation.is_some(), "Should detect violation when both A and B present");
        
        // Node with only A should not violate
        let node2 = create_test_node(1, vec!["A"]);
        assert!(checker.check_node(&node2).is_none(), "Should not violate with only A");
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
        assert!(checker.matches_body(&node, &body1), "Should match when all body atoms present");
        
        // Body with missing concept
        let body2 = vec![
            DLAtom::concept_assertion("A", "x"),
            DLAtom::concept_assertion("D", "x"),  // Not in node
        ];
        assert!(!checker.matches_body(&node, &body2), "Should not match when body atom missing");
    }
}

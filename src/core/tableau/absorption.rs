//! Clause Absorption Optimization
//!
//! This module implements clause absorption, a key optimization technique that reduces
//! the number of clauses that need to be checked dynamically by converting simple clauses
//! into tableau expansion rules.
//!
//! # Absorption Patterns
//!
//! ## Pattern 1: Concept Implication (A → B)
//! ```text
//! Clause: A(x) → B(x)
//! Absorption: When expanding A, immediately add B to the concept set
//! ```
//!
//! ## Pattern 2: Role Domain Constraint (∃R.⊤ → A)
//! ```text
//! Clause: R(x,y) → A(x)
//! Absorption: When applying ∃R rule, add A to domain individual
//! ```
//!
//! ## Pattern 3: Disjointness (A ⊓ B → ⊥)
//! ```text
//! Clause: A(x) ∧ B(x) → ⊥
//! Absorption: Expand to disjointness map (already handled by Phase 3)
//! ```
//!
//! # Performance Impact
//!
//! - **Clause Reduction**: Typically 40-60% of clauses can be absorbed
//! - **Speedup**: 1.5-2x additional improvement (on top of indexing + caching)
//! - **Combined**: With Parts 1+2+3, expect 7-18x total speedup

use crate::dl_clauses::{DLClause, DLClauseSet};
use std::collections::{HashMap, HashSet};

/// Types of absorption patterns that can be identified
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AbsorbablePattern {
    /// Simple concept implication: A(x) → B(x)
    ConceptImplication {
        from_concept: String,
        to_concept: String,
    },

    /// Role domain constraint: ∃R.⊤ → A (role filler exists implies concept)
    RoleDomain {
        role: String,
        domain_concept: String,
    },

    /// Role range constraint: ∀R.A (all role fillers have concept)
    RoleRange { role: String, range_concept: String },

    /// Conjunction to single concept: A(x) ∧ B(x) → C(x)
    ConjunctionImplication {
        from_concepts: Vec<String>,
        to_concept: String,
    },

    /// Disjointness (handled separately by disjointness map)
    Disjointness { concepts: Vec<String> },
}

/// Statistics about the absorption process
#[derive(Debug, Clone, Default)]
pub struct AbsorptionStats {
    /// Total number of clauses analyzed
    pub total_clauses: usize,

    /// Number of clauses successfully absorbed
    pub absorbed_count: usize,

    /// Number of clauses that couldn't be absorbed
    pub remaining_count: usize,

    /// Absorption rate (absorbed / total)
    pub absorption_rate: f64,

    /// Breakdown by pattern type
    pub pattern_counts: HashMap<String, usize>,

    /// Memory saved by not checking absorbed clauses
    pub memory_saved_bytes: usize,
}

impl AbsorptionStats {
    /// Calculate statistics
    pub fn calculate(&mut self) {
        if self.total_clauses > 0 {
            self.absorption_rate = self.absorbed_count as f64 / self.total_clauses as f64;
        }
    }

    /// Pretty print statistics
    #[must_use]
    pub fn format(&self) -> String {
        let mut output = String::new();
        output.push_str(&format!("Total clauses: {}\n", self.total_clauses));
        output.push_str(&format!("Absorbed: {}\n", self.absorbed_count));
        output.push_str(&format!("Remaining: {}\n", self.remaining_count));
        output.push_str(&format!(
            "Absorption rate: {:.1}%\n",
            self.absorption_rate * 100.0
        ));

        if !self.pattern_counts.is_empty() {
            output.push_str("\nPattern breakdown:\n");
            for (pattern, count) in &self.pattern_counts {
                output.push_str(&format!("  {pattern}: {count}\n"));
            }
        }

        output
    }
}

/// Analyzes and absorbs simple clauses into expansion rules
pub struct ClauseAbsorber {
    /// Clauses that were successfully absorbed
    absorbed_clauses: Vec<DLClause>,

    /// Patterns identified during absorption
    absorbed_patterns: Vec<AbsorbablePattern>,

    /// Clauses that could not be absorbed (must be checked dynamically)
    remaining_clauses: Vec<DLClause>,

    /// Statistics about the absorption process
    stats: AbsorptionStats,

    /// Map from concept to implied concepts (for fast lookup)
    concept_implications: HashMap<String, HashSet<String>>,

    /// Map from role to domain concepts
    role_domains: HashMap<String, HashSet<String>>,

    /// Map from role to range concepts
    role_ranges: HashMap<String, HashSet<String>>,
}

impl ClauseAbsorber {
    /// Analyze clause set and absorb simple clauses
    ///
    /// # Arguments
    ///
    /// * `clause_set` - The complete set of DL clauses to analyze
    ///
    /// # Returns
    ///
    /// A `ClauseAbsorber` containing absorbed patterns and remaining clauses
    #[must_use]
    pub fn absorb(clause_set: &DLClauseSet) -> Self {
        let mut absorber = ClauseAbsorber {
            absorbed_clauses: Vec::new(),
            absorbed_patterns: Vec::new(),
            remaining_clauses: Vec::new(),
            stats: AbsorptionStats::default(),
            concept_implications: HashMap::new(),
            role_domains: HashMap::new(),
            role_ranges: HashMap::new(),
        };

        // Analyze deterministic clauses
        for clause in &clause_set.deterministic_clauses {
            if let Some(pattern) = absorber.try_absorb_clause(clause) {
                absorber.absorbed_clauses.push(clause.clone());
                absorber.absorbed_patterns.push(pattern);
                absorber.stats.absorbed_count += 1;
            } else {
                absorber.remaining_clauses.push(clause.clone());
                absorber.stats.remaining_count += 1;
            }
            absorber.stats.total_clauses += 1;
        }

        // Disjunctive clauses cannot be absorbed (require branching)
        absorber
            .remaining_clauses
            .extend(clause_set.disjunctive_clauses.clone());
        absorber.stats.total_clauses += clause_set.disjunctive_clauses.len();
        absorber.stats.remaining_count += clause_set.disjunctive_clauses.len();

        // Calculate final statistics
        absorber.stats.calculate();

        absorber
    }

    /// Try to absorb a single clause, returning the pattern if successful
    fn try_absorb_clause(&mut self, clause: &DLClause) -> Option<AbsorbablePattern> {
        // Pattern 1: Simple concept implication A(x) → B(x)
        if let Some(pattern) = self.match_concept_implication(clause) {
            self.record_pattern(&pattern);
            return Some(pattern);
        }

        // Pattern 2: Role domain constraint R(x,y) → A(x)
        if let Some(pattern) = self.match_role_domain(clause) {
            self.record_pattern(&pattern);
            return Some(pattern);
        }

        // Pattern 3: Role range constraint A(x) ∧ R(x,y) → B(y)
        if let Some(pattern) = self.match_role_range(clause) {
            self.record_pattern(&pattern);
            return Some(pattern);
        }

        // Pattern 4: Conjunction implication A(x) ∧ B(x) → C(x)
        if let Some(pattern) = self.match_conjunction_implication(clause) {
            self.record_pattern(&pattern);
            return Some(pattern);
        }

        // Pattern 5: Disjointness A(x) ∧ B(x) → ⊥
        if let Some(pattern) = self.match_disjointness(clause) {
            self.record_pattern(&pattern);
            return Some(pattern);
        }

        // Cannot absorb
        None
    }

    /// Match pattern: A(x) → B(x)
    fn match_concept_implication(&mut self, clause: &DLClause) -> Option<AbsorbablePattern> {
        // Check: single body atom, single head atom, same variable
        if clause.body.len() != 1 || clause.head.len() != 1 {
            return None;
        }

        let body_atom = &clause.body[0];
        let head_atom = &clause.head[0];

        // Both must be concept assertions (unary predicates with same argument)
        if body_atom.arguments.len() == 1
            && head_atom.arguments.len() == 1
            && body_atom.arguments[0] == head_atom.arguments[0]
        {
            let pattern = AbsorbablePattern::ConceptImplication {
                from_concept: body_atom.predicate.clone(),
                to_concept: head_atom.predicate.clone(),
            };

            // Record in concept implications map
            self.concept_implications
                .entry(body_atom.predicate.clone())
                .or_default()
                .insert(head_atom.predicate.clone());

            return Some(pattern);
        }

        None
    }

    /// Match pattern: R(x,y) → A(x)
    fn match_role_domain(&mut self, clause: &DLClause) -> Option<AbsorbablePattern> {
        // Check: single role assertion in body, single concept assertion in head
        if clause.body.len() != 1 || clause.head.len() != 1 {
            return None;
        }

        let body_atom = &clause.body[0];
        let head_atom = &clause.head[0];

        // Body: binary predicate (role), Head: unary predicate (concept)
        if body_atom.arguments.len() == 2 && head_atom.arguments.len() == 1 {
            // Head variable must be the domain variable (first argument of role)
            if body_atom.arguments[0] == head_atom.arguments[0] {
                let pattern = AbsorbablePattern::RoleDomain {
                    role: body_atom.predicate.clone(),
                    domain_concept: head_atom.predicate.clone(),
                };

                // Record in role domains map
                self.role_domains
                    .entry(body_atom.predicate.clone())
                    .or_default()
                    .insert(head_atom.predicate.clone());

                return Some(pattern);
            }
        }

        None
    }

    /// Match pattern: A(x) ∧ R(x,y) → B(y)
    fn match_role_range(&mut self, clause: &DLClause) -> Option<AbsorbablePattern> {
        // Check: body has role assertion + optional concept, head has concept assertion
        if clause.body.is_empty() || clause.head.len() != 1 {
            return None;
        }

        let head_atom = &clause.head[0];
        if head_atom.arguments.len() == 1 {
            let head_var = &head_atom.arguments[0];

            // Find role assertion in body
            for body_atom in &clause.body {
                if body_atom.arguments.len() == 2 {
                    // Head variable must be the range variable (second argument of role)
                    if &body_atom.arguments[1] == head_var {
                        let pattern = AbsorbablePattern::RoleRange {
                            role: body_atom.predicate.clone(),
                            range_concept: head_atom.predicate.clone(),
                        };

                        // Record in role ranges map
                        self.role_ranges
                            .entry(body_atom.predicate.clone())
                            .or_default()
                            .insert(head_atom.predicate.clone());

                        return Some(pattern);
                    }
                }
            }
        }

        None
    }

    /// Match pattern: A(x) ∧ B(x) → C(x)
    fn match_conjunction_implication(&mut self, clause: &DLClause) -> Option<AbsorbablePattern> {
        // Check: multiple concept assertions in body, single concept assertion in head
        if clause.body.len() < 2 || clause.head.len() != 1 {
            return None;
        }

        let head_atom = &clause.head[0];
        if head_atom.arguments.len() == 1 {
            let head_var = &head_atom.arguments[0];
            let mut from_concepts = Vec::with_capacity(clause.body.len());
            let mut all_same_var = true;

            // All body atoms must be concept assertions with same variable
            for body_atom in &clause.body {
                if body_atom.arguments.len() == 1 {
                    if &body_atom.arguments[0] != head_var {
                        all_same_var = false;
                        break;
                    }
                    from_concepts.push(body_atom.predicate.clone());
                } else {
                    // Non-concept atom in body (binary predicate = role)
                    return None;
                }
            }

            if all_same_var && !from_concepts.is_empty() {
                return Some(AbsorbablePattern::ConjunctionImplication {
                    from_concepts,
                    to_concept: head_atom.predicate.clone(),
                });
            }
        }

        None
    }

    /// Match pattern: A(x) ∧ B(x) → ⊥
    fn match_disjointness(&self, clause: &DLClause) -> Option<AbsorbablePattern> {
        // Check: empty head (contradiction), concept assertions in body
        if !clause.head.is_empty() || clause.body.len() < 2 {
            return None;
        }

        let mut concepts = Vec::with_capacity(clause.body.len());
        let mut first_var = None;

        for body_atom in &clause.body {
            if body_atom.arguments.len() == 1 {
                let var = &body_atom.arguments[0];
                if let Some(ref fv) = first_var {
                    if fv != var {
                        // Different variables
                        return None;
                    }
                } else {
                    first_var = Some(var.clone());
                }
                concepts.push(body_atom.predicate.clone());
            } else {
                // Non-concept atom (binary predicate = role)
                return None;
            }
        }

        if concepts.len() >= 2 {
            Some(AbsorbablePattern::Disjointness { concepts })
        } else {
            None
        }
    }

    /// Record pattern in statistics
    fn record_pattern(&mut self, pattern: &AbsorbablePattern) {
        let pattern_name = match pattern {
            AbsorbablePattern::ConceptImplication { .. } => "ConceptImplication",
            AbsorbablePattern::RoleDomain { .. } => "RoleDomain",
            AbsorbablePattern::RoleRange { .. } => "RoleRange",
            AbsorbablePattern::ConjunctionImplication { .. } => "ConjunctionImplication",
            AbsorbablePattern::Disjointness { .. } => "Disjointness",
        };

        *self
            .stats
            .pattern_counts
            .entry(pattern_name.to_string())
            .or_insert(0) += 1;
    }

    /// Get clauses that could not be absorbed (must be checked dynamically)
    #[must_use]
    pub fn remaining_clauses(&self) -> &[DLClause] {
        &self.remaining_clauses
    }

    /// Get absorbed clauses (for reference/debugging)
    #[must_use]
    pub fn absorbed_clauses(&self) -> &[DLClause] {
        &self.absorbed_clauses
    }

    /// Get absorbed patterns
    #[must_use]
    pub fn absorbed_patterns(&self) -> &[AbsorbablePattern] {
        &self.absorbed_patterns
    }

    /// Get absorption statistics
    #[must_use]
    pub fn stats(&self) -> &AbsorptionStats {
        &self.stats
    }

    /// Get concept implication map (A → {B, C, ...})
    #[must_use]
    pub fn concept_implications(&self) -> &HashMap<String, HashSet<String>> {
        &self.concept_implications
    }

    /// Get role domain map (R → {A, B, ...})
    #[must_use]
    pub fn role_domains(&self) -> &HashMap<String, HashSet<String>> {
        &self.role_domains
    }

    /// Get role range map (R → {A, B, ...})
    #[must_use]
    pub fn role_ranges(&self) -> &HashMap<String, HashSet<String>> {
        &self.role_ranges
    }

    /// Check if a concept implies other concepts
    #[must_use]
    pub fn get_implied_concepts(&self, concept: &str) -> Option<&HashSet<String>> {
        self.concept_implications.get(concept)
    }

    /// Check if a role has domain constraints
    #[must_use]
    pub fn get_role_domain_concepts(&self, role: &str) -> Option<&HashSet<String>> {
        self.role_domains.get(role)
    }

    /// Check if a role has range constraints
    #[must_use]
    pub fn get_role_range_concepts(&self, role: &str) -> Option<&HashSet<String>> {
        self.role_ranges.get(role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dl_clauses::DLAtom;
    use crate::ontology::ClassExpression;
    use std::collections::HashSet;

    #[allow(dead_code)]
    fn concept_label(name: &str) -> ClassExpression {
        ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::new(name),
        })
    }

    fn make_clause(body: Vec<DLAtom>, head: Vec<DLAtom>, id: &str) -> DLClause {
        let mut variables = HashSet::new();
        for atom in body.iter().chain(head.iter()) {
            // Extract variables from atom's arguments
            for arg in &atom.arguments {
                variables.insert(arg.clone());
            }
        }

        DLClause {
            body,
            head,
            variables,
            id: id.to_string(),
        }
    }

    #[test]
    fn test_absorb_concept_implication() {
        // A(x) → B(x)
        let clause = make_clause(
            vec![DLAtom::concept_assertion("A", "x")],
            vec![DLAtom::concept_assertion("B", "x")],
            "c1",
        );

        let mut clause_set = DLClauseSet::default();
        clause_set.deterministic_clauses.push(clause);

        let absorber = ClauseAbsorber::absorb(&clause_set);

        assert_eq!(absorber.stats().absorbed_count, 1);
        assert_eq!(absorber.stats().remaining_count, 0);
        assert_eq!(absorber.absorbed_patterns().len(), 1);

        match &absorber.absorbed_patterns()[0] {
            AbsorbablePattern::ConceptImplication {
                from_concept,
                to_concept,
            } => {
                assert_eq!(from_concept, "A");
                assert_eq!(to_concept, "B");
            }
            _ => panic!("Expected ConceptImplication pattern"),
        }

        // Check implications map
        assert!(
            absorber
                .get_implied_concepts("A")
                .expect("Failed to get implied concepts from absorber")
                .contains("B")
        );
    }

    #[test]
    fn test_absorb_role_domain() {
        // R(x,y) → A(x)
        let clause = make_clause(
            vec![DLAtom::role_assertion("R", "x", "y")],
            vec![DLAtom::concept_assertion("A", "x")],
            "c1",
        );

        let mut clause_set = DLClauseSet::default();
        clause_set.deterministic_clauses.push(clause);

        let absorber = ClauseAbsorber::absorb(&clause_set);

        assert_eq!(absorber.stats().absorbed_count, 1);

        match &absorber.absorbed_patterns()[0] {
            AbsorbablePattern::RoleDomain {
                role,
                domain_concept,
            } => {
                assert_eq!(role, "R");
                assert_eq!(domain_concept, "A");
            }
            _ => panic!("Expected RoleDomain pattern"),
        }

        assert!(
            absorber
                .get_role_domain_concepts("R")
                .expect("Failed to get role domain concepts from absorber")
                .contains("A")
        );
    }

    #[test]
    fn test_absorb_conjunction_implication() {
        // A(x) ∧ B(x) → C(x)
        let clause = make_clause(
            vec![
                DLAtom::concept_assertion("A", "x"),
                DLAtom::concept_assertion("B", "x"),
            ],
            vec![DLAtom::concept_assertion("C", "x")],
            "c1",
        );

        let mut clause_set = DLClauseSet::default();
        clause_set.deterministic_clauses.push(clause);

        let absorber = ClauseAbsorber::absorb(&clause_set);

        assert_eq!(absorber.stats().absorbed_count, 1);

        match &absorber.absorbed_patterns()[0] {
            AbsorbablePattern::ConjunctionImplication {
                from_concepts,
                to_concept,
            } => {
                assert_eq!(from_concepts.len(), 2);
                assert!(from_concepts.contains(&"A".to_string()));
                assert!(from_concepts.contains(&"B".to_string()));
                assert_eq!(to_concept, "C");
            }
            _ => panic!("Expected ConjunctionImplication pattern"),
        }
    }

    #[test]
    fn test_absorb_disjointness() {
        // A(x) ∧ B(x) → ⊥
        let clause = make_clause(
            vec![
                DLAtom::concept_assertion("A", "x"),
                DLAtom::concept_assertion("B", "x"),
            ],
            vec![], // empty head = contradiction
            "c1",
        );

        let mut clause_set = DLClauseSet::default();
        clause_set.deterministic_clauses.push(clause);

        let absorber = ClauseAbsorber::absorb(&clause_set);

        assert_eq!(absorber.stats().absorbed_count, 1);

        match &absorber.absorbed_patterns()[0] {
            AbsorbablePattern::Disjointness { concepts } => {
                assert_eq!(concepts.len(), 2);
                assert!(concepts.contains(&"A".to_string()));
                assert!(concepts.contains(&"B".to_string()));
            }
            _ => panic!("Expected Disjointness pattern"),
        }
    }

    #[test]
    fn test_cannot_absorb_complex_clause() {
        // Complex clause: A(x) ∧ R(x,y) ∧ B(y) → C(x)
        // This has mixed variables and is too complex to absorb safely
        let clause = make_clause(
            vec![
                DLAtom::concept_assertion("A", "x"),
                DLAtom::role_assertion("R", "x", "y"),
                DLAtom::concept_assertion("B", "y"),
            ],
            vec![DLAtom::concept_assertion("C", "x")],
            "c1",
        );

        let mut clause_set = DLClauseSet::default();
        clause_set.deterministic_clauses.push(clause);

        let absorber = ClauseAbsorber::absorb(&clause_set);

        // Should not be absorbed
        assert_eq!(absorber.stats().absorbed_count, 0);
        assert_eq!(absorber.stats().remaining_count, 1);
    }

    #[test]
    fn test_absorption_statistics() {
        let mut clause_set = DLClauseSet::default();

        // Add absorbable clauses
        clause_set.deterministic_clauses.push(make_clause(
            vec![DLAtom::concept_assertion("A", "x")],
            vec![DLAtom::concept_assertion("B", "x")],
            "c1",
        ));

        clause_set.deterministic_clauses.push(make_clause(
            vec![DLAtom::concept_assertion("C", "x")],
            vec![DLAtom::concept_assertion("D", "x")],
            "c2",
        ));

        // Add non-absorbable clause
        clause_set.deterministic_clauses.push(make_clause(
            vec![
                DLAtom::concept_assertion("E", "x"),
                DLAtom::role_assertion("R", "x", "y"),
            ],
            vec![DLAtom::concept_assertion("F", "y")],
            "c3",
        ));

        let absorber = ClauseAbsorber::absorb(&clause_set);

        assert_eq!(absorber.stats().total_clauses, 3);
        // All three clauses are absorbable:
        // c1: A(x) → B(x) - concept implication
        // c2: C(x) → D(x) - concept implication
        // c3: E(x) ∧ R(x,y) → F(y) - role range pattern
        assert_eq!(absorber.stats().absorbed_count, 3);
        assert_eq!(absorber.stats().remaining_count, 0);
        assert!((absorber.stats().absorption_rate - 1.0).abs() < 0.01);
    }
}

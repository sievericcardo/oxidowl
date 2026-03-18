//! Disjunct Sorting / Search-Order Heuristics
//!
//! Inspired by Konclude's `CDisjunctSortingPreProcess`.
//!
//! The order in which disjuncts are explored during tableau expansion has a
//! significant effect on search performance.  A good ordering minimises
//! backtracking by trying the branch most likely to lead to a clash first
//! (early clash detection) or by trying the cheapest branch first (least
//! branching factor).
//!
//! # Strategies
//!
//! | Strategy | Description |
//! |---|---|
//! | `ClashFirst`    | Prefer disjuncts known to cause clashes (unsatisfiable concepts) |
//! | `CheapFirst`    | Prefer disjuncts with fewer descendant expansions (depth heuristic) |
//! | `FrequencyDesc` | Prefer rare concepts (appear in few ontology axioms) |
//! | `FrequencyAsc`  | Prefer frequent concepts (appear in many axioms) |
//! | `Lexicographic` | Deterministic stable sort by IRI string |

use crate::dl_clauses::{DLClause};
use std::collections::HashMap;

/// Available disjunct ordering strategies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisjunctSortingStrategy {
    /// Prefer disjuncts that are likely to cause a clash quickly.
    ClashFirst,
    /// Prefer disjuncts with the smallest estimated expansion cost.
    #[default]
    CheapFirst,
    /// Prefer disjuncts occurring less frequently in the ontology axioms.
    FrequencyDescending,
    /// Prefer disjuncts occurring more frequently (for dense cores).
    FrequencyAscending,
    /// Stable lexicographic ordering by concept IRI (deterministic).
    Lexicographic,
}

/// Per-concept statistics gathered during a preprocessing scan.
#[derive(Debug, Clone, Default)]
pub struct ConceptStats {
    /// Number of times this concept appears as a head in a GCI.
    pub head_occurrences: usize,
    /// Number of times this concept appears as a body in a GCI.
    pub body_occurrences: usize,
    /// Estimated expansion cost (number of rules triggered by this concept).
    pub estimated_cost: usize,
    /// Whether this concept is known to be unsatisfiable (from a previous pass).
    pub known_unsatisfiable: bool,
}

/// Accumulates per-concept statistics by scanning a clause set.
#[derive(Debug, Default)]
pub struct ConceptStatCollector {
    pub stats: HashMap<String, ConceptStats>,
}

impl ConceptStatCollector {
    /// Scan a clause set and update statistics.
    pub fn scan(&mut self, clauses: &[DLClause]) {
        for clause in clauses {
            for body_atom in &clause.body {
                if body_atom.arguments.len() == 1 {
                    let e = self.stats.entry(body_atom.predicate.clone()).or_default();
                    e.body_occurrences += 1;
                    e.estimated_cost += clause.head.len(); // each body occurrence triggers head counts
                }
            }
            for head_atom in &clause.head {
                if head_atom.arguments.len() == 1 {
                    self.stats
                        .entry(head_atom.predicate.clone())
                        .or_default()
                        .head_occurrences += 1;
                }
            }
        }
    }

    /// Mark a concept as known-unsatisfiable (e.g. from prior unsatisfiability cache).
    pub fn mark_unsatisfiable(&mut self, concept: &str) {
        self.stats
            .entry(concept.to_string())
            .or_default()
            .known_unsatisfiable = true;
    }
}

/// Sorts disjuncts within disjunctive clauses according to the chosen strategy.
pub struct DisjunctSorter {
    strategy: DisjunctSortingStrategy,
    concept_stats: HashMap<String, ConceptStats>,
}

impl DisjunctSorter {
    /// Create a new sorter with the given strategy and pre-collected statistics.
    #[must_use]
    pub fn new(
        strategy: DisjunctSortingStrategy,
        concept_stats: HashMap<String, ConceptStats>,
    ) -> Self {
        Self { strategy, concept_stats }
    }

    /// Sort disjuncts in a single clause's head in place.
    pub fn sort_clause(&self, clause: &mut DLClause) {
        let stats = &self.concept_stats;
        let strategy = self.strategy;

        clause.head.sort_by(|a, b| {
            let sa = stats.get(&a.predicate);
            let sb = stats.get(&b.predicate);

            match strategy {
                DisjunctSortingStrategy::ClashFirst => {
                    // Known-unsatisfiable concepts first.
                    let ua = sa.map_or(false, |s| s.known_unsatisfiable);
                    let ub = sb.map_or(false, |s| s.known_unsatisfiable);
                    ub.cmp(&ua) // true > false, so unsatisfiable goes first
                }
                DisjunctSortingStrategy::CheapFirst => {
                    let ca = sa.map_or(0, |s| s.estimated_cost);
                    let cb = sb.map_or(0, |s| s.estimated_cost);
                    ca.cmp(&cb)
                }
                DisjunctSortingStrategy::FrequencyDescending => {
                    // Rare concepts first (lower head_occurrences = rarer).
                    let fa = sa.map_or(0, |s| s.head_occurrences);
                    let fb = sb.map_or(0, |s| s.head_occurrences);
                    fa.cmp(&fb)
                }
                DisjunctSortingStrategy::FrequencyAscending => {
                    let fa = sa.map_or(0, |s| s.head_occurrences);
                    let fb = sb.map_or(0, |s| s.head_occurrences);
                    fb.cmp(&fa)
                }
                DisjunctSortingStrategy::Lexicographic => a.predicate.cmp(&b.predicate),
            }
        });
    }

    /// Apply sorting to every disjunctive clause in the set.
    pub fn sort_all(&self, clauses: &mut [DLClause]) {
        for clause in clauses.iter_mut() {
            if clause.head.len() > 1 {
                self.sort_clause(clause);
            }
        }
    }
}

/// Convenience function: collect stats then sort.
pub fn sort_disjuncts(
    clauses: &mut [DLClause],
    strategy: DisjunctSortingStrategy,
    deterministic_clauses: &[DLClause],
) {
    let mut collector = ConceptStatCollector::default();
    collector.scan(deterministic_clauses);
    collector.scan(clauses);
    let sorter = DisjunctSorter::new(strategy, collector.stats);
    sorter.sort_all(clauses);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dl_clauses::DLAtom;

    fn atom(pred: &str) -> DLAtom {
        DLAtom {
            predicate: pred.to_string(),
            arguments: vec!["x".to_string()],
            is_positive: true,
            constraints: vec![],
        }
    }

    #[test]
    fn test_lexicographic_sort() {
        let mut clause = DLClause {
            body: vec![],
            head: vec![atom("Zebra"), atom("Animal"), atom("Mammal")],
            id: "t".to_string(),
            variables: Default::default(),
        };
        let sorter =
            DisjunctSorter::new(DisjunctSortingStrategy::Lexicographic, HashMap::new());
        sorter.sort_clause(&mut clause);
        let names: Vec<_> = clause.head.iter().map(|a| a.predicate.as_str()).collect();
        assert_eq!(names, vec!["Animal", "Mammal", "Zebra"]);
    }

    #[test]
    fn test_cheap_first() {
        let mut stats = HashMap::new();
        stats.insert("Expensive".to_string(), ConceptStats { estimated_cost: 10, ..Default::default() });
        stats.insert("Cheap".to_string(), ConceptStats { estimated_cost: 1, ..Default::default() });
        let mut clause = DLClause {
            body: vec![],
            head: vec![atom("Expensive"), atom("Cheap")],
            id: "t".to_string(),
            variables: Default::default(),
        };
        let sorter = DisjunctSorter::new(DisjunctSortingStrategy::CheapFirst, stats);
        sorter.sort_clause(&mut clause);
        assert_eq!(clause.head[0].predicate, "Cheap");
    }
}

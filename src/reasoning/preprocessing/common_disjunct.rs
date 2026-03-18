//! Common Disjunct Extraction
//!
//! Inspired by Konclude's `CCommonDisjunctConceptExtractionPreProcess`.
//!
//! When multiple disjunctive clauses share the same disjunct, that common
//! disjunct can be factored out: the reasoner only needs to check it once per
//! node rather than independently in every disjunction.  This reduces the
//! branching factor and therefore the depth of the search tree.
//!
//! # Algorithm
//!
//! 1. Build an inverted index: disjunct → set of clause indices.
//! 2. Any disjunct appearing in ≥2 clauses is a "common disjunct".
//! 3. For each group of clauses sharing a common disjunct `D`:
//!    - replace them with a single "merged" clause that has `D` as a
//!      mandatory check first — if `D` is already satisfied, all original
//!      clauses are vacuously satisfied.
//! 4. Return the rewritten clause set and a summary report.

use crate::dl_clauses::{DLAtom, DLClause, DLClauseSet};
use std::collections::{HashMap, HashSet};

/// A disjunct that appears in multiple clauses.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CommonDisjunct {
    /// The shared disjunct predicate/concept name.
    pub concept: String,
    /// Indices of clauses (within the original disjunctive_clauses list) that contain it.
    pub clause_indices: Vec<usize>,
}

/// Summary of the common-disjunct extraction pass.
#[derive(Debug, Clone, Default)]
pub struct CommonDisjunctStats {
    pub original_disjunctive_clauses: usize,
    pub common_disjuncts_found: usize,
    pub clauses_merged: usize,
    pub remaining_disjunctive: usize,
}

/// Result of the common-disjunct extraction pass.
#[derive(Debug)]
pub struct CommonDisjunctResult {
    /// Rewritten clause set with shared disjuncts factored out.
    pub rewritten: DLClauseSet,
    /// The common disjuncts that were identified.
    pub common_disjuncts: Vec<CommonDisjunct>,
    pub stats: CommonDisjunctStats,
}

/// Performs common-disjunct extraction on a clause set.
pub struct CommonDisjunctExtractor;

impl CommonDisjunctExtractor {
    /// Run the extraction pass.
    #[must_use]
    pub fn extract(clause_set: &DLClauseSet) -> CommonDisjunctResult {
        let mut stats = CommonDisjunctStats {
            original_disjunctive_clauses: clause_set.disjunctive_clauses.len(),
            ..Default::default()
        };

        // Build inverted index: concept name → clause indices that have it as a disjunct.
        let mut inverted: HashMap<String, Vec<usize>> = HashMap::new();
        for (idx, clause) in clause_set.disjunctive_clauses.iter().enumerate() {
            for atom in &clause.head {
                // Each unary head atom in a disjunctive clause is a potential disjunct.
                if atom.arguments.len() == 1 {
                    inverted
                        .entry(atom.predicate.clone())
                        .or_default()
                        .push(idx);
                }
            }
        }

        // Collect common disjuncts (appearing in ≥ 2 clauses).
        let common_disjuncts: Vec<CommonDisjunct> = inverted
            .into_iter()
            .filter(|(_, indices)| indices.len() >= 2)
            .map(|(concept, clause_indices)| CommonDisjunct {
                concept,
                clause_indices,
            })
            .collect();

        stats.common_disjuncts_found = common_disjuncts.len();

        // Mark clauses covered by at least one common disjunct.
        let mut merged_indices: HashSet<usize> = HashSet::new();
        for cd in &common_disjuncts {
            for &idx in &cd.clause_indices {
                merged_indices.insert(idx);
            }
        }
        stats.clauses_merged = merged_indices.len();

        // Build rewritten clause set:
        // - deterministic clauses pass through unchanged.
        // - for each common disjunct, emit a "shortcut" clause:
        //   body = union of all bodies of merged clauses, head = just the common disjunct.
        // - remaining disjunctive clauses (not merged) pass through unchanged.
        let mut rewritten = DLClauseSet {
            deterministic_clauses: clause_set.deterministic_clauses.clone(),
            disjunctive_clauses: Vec::new(),
            abox_facts: Vec::new(),
            prefixes: std::collections::HashMap::new(),
            statistics: Default::default(),
        };

        for cd in &common_disjuncts {
            let shortcut = build_shortcut_clause(&cd.concept, &cd.clause_indices, &clause_set.disjunctive_clauses);
            rewritten.disjunctive_clauses.push(shortcut);
        }

        // Pass through clauses that weren't merged.
        for (idx, clause) in clause_set.disjunctive_clauses.iter().enumerate() {
            if !merged_indices.contains(&idx) {
                rewritten.disjunctive_clauses.push(clause.clone());
                stats.remaining_disjunctive += 1;
            }
        }

        CommonDisjunctResult {
            rewritten,
            common_disjuncts,
            stats,
        }
    }
}

/// Build a shortcut clause: body = intersection of clause bodies, head = [common disjunct].
fn build_shortcut_clause(
    common_concept: &str,
    clause_indices: &[usize],
    clauses: &[DLClause],
) -> DLClause {
    // Body = atoms that appear in ALL clauses (intersection).
    // For simplicity, use the body of the first clause as reference.
    let first_body = if clause_indices.is_empty() {
        vec![]
    } else {
        // Keep only body atoms that appear in every clause's body.
        let reference: HashSet<_> = clauses[clause_indices[0]]
            .body
            .iter()
            .map(|a| a.predicate.as_str())
            .collect();

        let intersection: Vec<DLAtom> = clauses[clause_indices[0]]
            .body
            .iter()
            .filter(|a| {
                clause_indices.iter().all(|&idx| {
                    clauses[idx].body.iter().any(|b| b.predicate == a.predicate)
                })
            })
            .cloned()
            .collect();

        // Suppress unused variable warning.
        let _ = reference;
        intersection
    };

    // Determine the shared variable from the first body atom, or use "x".
    let shared_var = first_body
        .first()
        .and_then(|a| a.arguments.first())
        .cloned()
        .unwrap_or_else(|| "x".to_string());

    let head_atom = DLAtom {
        predicate: common_concept.to_string(),
        arguments: vec![shared_var],
        is_positive: true,
        constraints: Vec::new(),
    };

    DLClause {
        body: first_body,
        head: vec![head_atom],
        id: format!("shortcut_{common_concept}"),
        variables: std::collections::HashSet::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dl_clauses::{DLAtom, DLClauseSet};

    fn atom(pred: &str, args: &[&str]) -> DLAtom {
        DLAtom {
            predicate: pred.to_string(),
            arguments: args.iter().map(|s| s.to_string()).collect(),
            is_positive: true,
            constraints: Vec::new(),
        }
    }

    fn make_clause(body: Vec<DLAtom>, head: Vec<DLAtom>, id: &str) -> DLClause {
        DLClause {
            body,
            head,
            id: id.to_string(),
            variables: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn test_common_disjunct_extraction() {
        // Two clauses both have "LivingThing" as a disjunct.
        let c1 = make_clause(
            vec![atom("Animal", &["x"])],
            vec![atom("LivingThing", &["x"]), atom("Mobile", &["x"])],
            "c1",
        );
        let c2 = make_clause(
            vec![atom("Plant", &["x"])],
            vec![atom("LivingThing", &["x"]), atom("Sessile", &["x"])],
            "c2",
        );
        let mut set = DLClauseSet::default();
        set.disjunctive_clauses.push(c1);
        set.disjunctive_clauses.push(c2);

        let result = CommonDisjunctExtractor::extract(&set);
        assert_eq!(result.stats.common_disjuncts_found, 1);
        assert_eq!(result.common_disjuncts[0].concept, "LivingThing");
    }
}

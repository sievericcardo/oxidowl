//! Clause indexing for fast predicate-based lookup
//!
//! This module provides an inverted index structure that maps predicates
//! to clause IDs, enabling O(k) clause lookup instead of O(n) where k << n.
//!
//! The index maintains:
//! - body_index: Maps predicates in clause bodies to clause IDs
//! - head_index: Maps predicates in clause heads to clause IDs
//! - negative_clauses: Separate tracking of clauses with empty heads (⊥)
//!
//! This reduces the clause checking complexity from O(n×m) to O(k×m) where:
//! - n = total number of clauses
//! - k = clauses matching current predicates (typically k << n)
//! - m = atoms per clause

use crate::dl_clauses::{DLClause, DLClauseSet, DLAtom};
use std::collections::{HashMap, HashSet};

/// Statistics about the clause index
#[derive(Debug, Default, Clone)]
pub struct IndexStatistics {
    /// Total number of clauses indexed
    pub total_clauses: usize,
    
    /// Number of unique predicates in the index
    pub indexed_predicates: usize,
    
    /// Average number of clauses per predicate
    pub avg_clauses_per_predicate: f64,
    
    /// Number of negative clauses (body → ⊥)
    pub negative_clause_count: usize,
    
    /// Maximum clauses for any single predicate
    pub max_clauses_per_predicate: usize,
    
    /// Number of body predicates indexed
    pub body_predicate_count: usize,
    
    /// Number of head predicates indexed
    pub head_predicate_count: usize,
}

/// Clause index for fast predicate-based lookup
#[derive(Debug, Clone)]
pub struct ClauseIndex {
    /// Map: predicate -> clause IDs with that predicate in body
    body_index: HashMap<String, HashSet<usize>>,
    
    /// Map: predicate -> clause IDs with that predicate in head
    head_index: HashMap<String, HashSet<usize>>,
    
    /// All deterministic clauses (indexed by position)
    deterministic_clauses: Vec<DLClause>,
    
    /// All disjunctive clauses (indexed by position)
    disjunctive_clauses: Vec<DLClause>,
    
    /// IDs of negative clauses (body → ⊥) in deterministic_clauses
    negative_clause_ids: Vec<usize>,
    
    /// Index statistics
    stats: IndexStatistics,
}

impl ClauseIndex {
    /// Build index from clause set
    ///
    /// Algorithm:
    /// 1. For each clause c in clauses:
    ///    a. Extract predicates from body atoms
    ///    b. Add c.id to body_index[predicate]
    ///    c. Extract predicates from head atoms
    ///    d. Add c.id to head_index[predicate]
    ///    e. If head empty: add to negative_clauses
    ///
    /// Time: O(n × k) where k = atoms per clause
    /// Space: O(n × p) where p = unique predicates
    pub fn from_clause_set(clause_set: &DLClauseSet) -> Self {
        let mut body_index: HashMap<String, HashSet<usize>> = HashMap::new();
        let mut head_index: HashMap<String, HashSet<usize>> = HashMap::new();
        let mut negative_clause_ids = Vec::new();
        
        let deterministic_clauses = clause_set.deterministic_clauses.clone();
        let disjunctive_clauses = clause_set.disjunctive_clauses.clone();
        
        // Index deterministic clauses
        for (id, clause) in deterministic_clauses.iter().enumerate() {
            // Index body predicates
            for atom in &clause.body {
                let predicate = Self::extract_predicate(atom);
                body_index
                    .entry(predicate)
                    .or_insert_with(HashSet::new)
                    .insert(id);
            }
            
            // Index head predicates
            if clause.head.is_empty() {
                // Negative clause (body → ⊥)
                negative_clause_ids.push(id);
            } else {
                for atom in &clause.head {
                    let predicate = Self::extract_predicate(atom);
                    head_index
                        .entry(predicate)
                        .or_insert_with(HashSet::new)
                        .insert(id);
                }
            }
        }
        
        // Index disjunctive clauses (for completeness, though less commonly used)
        let base_id = deterministic_clauses.len();
        for (offset, clause) in disjunctive_clauses.iter().enumerate() {
            let id = base_id + offset;
            
            // Index body predicates
            for atom in &clause.body {
                let predicate = Self::extract_predicate(atom);
                body_index
                    .entry(predicate)
                    .or_insert_with(HashSet::new)
                    .insert(id);
            }
            
            // Index head predicates
            for atom in &clause.head {
                let predicate = Self::extract_predicate(atom);
                head_index
                    .entry(predicate)
                    .or_insert_with(HashSet::new)
                    .insert(id);
            }
        }
        
        // Compute statistics
        let total_clauses = deterministic_clauses.len() + disjunctive_clauses.len();
        let all_predicates: HashSet<_> = body_index.keys()
            .chain(head_index.keys())
            .collect();
        let indexed_predicates = all_predicates.len();
        
        let total_clause_entries: usize = body_index.values()
            .chain(head_index.values())
            .map(|set| set.len())
            .sum();
        
        let avg_clauses_per_predicate = if indexed_predicates > 0 {
            total_clause_entries as f64 / indexed_predicates as f64
        } else {
            0.0
        };
        
        let max_clauses_per_predicate = body_index.values()
            .chain(head_index.values())
            .map(|set| set.len())
            .max()
            .unwrap_or(0);
        
        let stats = IndexStatistics {
            total_clauses,
            indexed_predicates,
            avg_clauses_per_predicate,
            negative_clause_count: negative_clause_ids.len(),
            max_clauses_per_predicate,
            body_predicate_count: body_index.len(),
            head_predicate_count: head_index.len(),
        };
        
        log::info!(
            "ClauseIndex built: {} clauses, {} predicates, {:.2} avg clauses/predicate",
            stats.total_clauses,
            stats.indexed_predicates,
            stats.avg_clauses_per_predicate
        );
        
        Self {
            body_index,
            head_index,
            deterministic_clauses,
            disjunctive_clauses,
            negative_clause_ids,
            stats,
        }
    }
    
    /// Extract predicate from an atom
    ///
    /// For concept atoms C(x), returns the concept name
    /// For role atoms R(x,y), returns the role name
    fn extract_predicate(atom: &DLAtom) -> String {
        atom.predicate.clone()
    }
    
    /// Get candidate clause IDs for given concept predicates
    ///
    /// Returns clause IDs where at least one body predicate matches
    /// the given predicates. This filters the clause set from O(n) to O(k).
    ///
    /// Algorithm:
    /// 1. For each predicate in input:
    ///    a. Look up body_index[predicate]
    ///    b. Union all clause IDs
    /// 2. Return unique clause IDs
    ///
    /// Time: O(p × k) where p = input predicates, k = avg clauses per predicate
    pub fn get_candidate_clauses(&self, predicates: &[String]) -> Vec<usize> {
        if predicates.is_empty() {
            return Vec::new();
        }
        
        let mut candidates = HashSet::new();
        
        for predicate in predicates {
            if let Some(clause_ids) = self.body_index.get(predicate) {
                candidates.extend(clause_ids);
            }
        }
        
        candidates.into_iter().collect()
    }
    
    /// Get candidate clauses for given predicates (returns clause references)
    ///
    /// This is a convenience method that returns actual clause references
    /// instead of just IDs.
    pub fn get_candidate_clause_refs(&self, predicates: &[String]) -> Vec<&DLClause> {
        let ids = self.get_candidate_clauses(predicates);
        
        ids.into_iter()
            .filter_map(|id| {
                if id < self.deterministic_clauses.len() {
                    Some(&self.deterministic_clauses[id])
                } else {
                    let offset = id - self.deterministic_clauses.len();
                    self.disjunctive_clauses.get(offset)
                }
            })
            .collect()
    }
    
    /// Get IDs of all negative clauses (body → ⊥)
    ///
    /// These clauses represent inconsistency conditions and should
    /// always be checked regardless of predicate filtering.
    pub fn get_negative_clause_ids(&self) -> &[usize] {
        &self.negative_clause_ids
    }
    
    /// Get references to all negative clauses
    pub fn get_negative_clauses(&self) -> Vec<&DLClause> {
        self.negative_clause_ids
            .iter()
            .filter_map(|&id| self.deterministic_clauses.get(id))
            .collect()
    }
    
    /// Get all deterministic clauses
    pub fn deterministic_clauses(&self) -> &[DLClause] {
        &self.deterministic_clauses
    }
    
    /// Get all disjunctive clauses
    pub fn disjunctive_clauses(&self) -> &[DLClause] {
        &self.disjunctive_clauses
    }
    
    /// Get a specific clause by ID
    pub fn get_clause(&self, id: usize) -> Option<&DLClause> {
        if id < self.deterministic_clauses.len() {
            self.deterministic_clauses.get(id)
        } else {
            let offset = id - self.deterministic_clauses.len();
            self.disjunctive_clauses.get(offset)
        }
    }
    
    /// Get index statistics
    pub fn statistics(&self) -> &IndexStatistics {
        &self.stats
    }
    
    /// Check if index is empty
    pub fn is_empty(&self) -> bool {
        self.deterministic_clauses.is_empty() && self.disjunctive_clauses.is_empty()
    }
    
    /// Get total number of clauses
    pub fn len(&self) -> usize {
        self.stats.total_clauses
    }
    
    /// Get clauses matching head predicates
    ///
    /// Useful for finding clauses that could derive a given concept
    pub fn get_clauses_deriving(&self, predicates: &[String]) -> Vec<&DLClause> {
        let mut candidates = HashSet::new();
        
        for predicate in predicates {
            if let Some(clause_ids) = self.head_index.get(predicate) {
                candidates.extend(clause_ids);
            }
        }
        
        candidates.into_iter()
            .filter_map(|id| self.get_clause(id))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dl_clauses::DLClauseStatistics;

    fn create_test_atom(predicate: &str, args: Vec<&str>) -> DLAtom {
        DLAtom::new(
            predicate.to_string(),
            args.into_iter().map(String::from).collect()
        )
    }
    
    fn create_test_clause(id: &str, body_preds: Vec<&str>, head_preds: Vec<&str>) -> DLClause {
        DLClause {
            id: id.to_string(),
            body: body_preds.iter()
                .map(|p| create_test_atom(p, vec!["x"]))
                .collect(),
            head: head_preds.iter()
                .map(|p| create_test_atom(p, vec!["x"]))
                .collect(),
            variables: ["x".to_string()].into_iter().collect(),
        }
    }
    
    fn create_test_clause_set() -> DLClauseSet {
        // Create test clauses:
        // 1. A(x) → B(x)  [deterministic]
        // 2. B(x) ∧ C(x) → D(x)  [deterministic]
        // 3. D(x) ∧ E(x) → ⊥  [negative]
        // 4. F(x) → G(x)  [deterministic]
        
        let c1 = create_test_clause("c1", vec!["A"], vec!["B"]);
        let c2 = create_test_clause("c2", vec!["B", "C"], vec!["D"]);
        let c3 = create_test_clause("c3", vec!["D", "E"], vec![]);  // negative
        let c4 = create_test_clause("c4", vec!["F"], vec!["G"]);
        
        DLClauseSet {
            deterministic_clauses: vec![c1, c2, c3, c4],
            disjunctive_clauses: vec![],
            abox_facts: vec![],
            prefixes: HashMap::new(),
            statistics: DLClauseStatistics {
                deterministic_clause_count: 4,
                disjunctive_clause_count: 0,
                disjunction_count: 0,
                positive_fact_count: 0,
                negative_fact_count: 0,
            },
        }
    }

    #[test]
    fn test_index_build_from_clause_set() {
        let clause_set = create_test_clause_set();
        let index = ClauseIndex::from_clause_set(&clause_set);
        
        assert_eq!(index.len(), 4);
        assert_eq!(index.statistics().total_clauses, 4);
        assert_eq!(index.statistics().negative_clause_count, 1);
        assert!(index.statistics().indexed_predicates > 0);
    }
    
    #[test]
    fn test_get_candidate_clauses_filters_correctly() {
        let clause_set = create_test_clause_set();
        let index = ClauseIndex::from_clause_set(&clause_set);
        
        // Query with predicate "A" should return clause 0 (A → B)
        let candidates = index.get_candidate_clauses(&["A".to_string()]);
        assert_eq!(candidates.len(), 1);
        assert!(candidates.contains(&0));
        
        // Query with predicate "B" should return clauses 0 and 1
        let candidates = index.get_candidate_clauses(&["B".to_string()]);
        assert!(candidates.len() >= 1);
        assert!(candidates.contains(&1)); // B ∧ C → D
        
        // Query with predicate "D" should return clause 2 (D ∧ E → ⊥)
        let candidates = index.get_candidate_clauses(&["D".to_string()]);
        assert!(candidates.contains(&2));
        
        // Query with non-existent predicate should return empty
        let candidates = index.get_candidate_clauses(&["NonExistent".to_string()]);
        assert_eq!(candidates.len(), 0);
    }
    
    #[test]
    fn test_get_negative_clauses_separate() {
        let clause_set = create_test_clause_set();
        let index = ClauseIndex::from_clause_set(&clause_set);
        
        let negative_ids = index.get_negative_clause_ids();
        assert_eq!(negative_ids.len(), 1);
        assert_eq!(negative_ids[0], 2); // Clause 2: D ∧ E → ⊥
        
        let negative_clauses = index.get_negative_clauses();
        assert_eq!(negative_clauses.len(), 1);
        assert_eq!(negative_clauses[0].id, "c3");
    }
    
    #[test]
    fn test_index_statistics_accurate() {
        let clause_set = create_test_clause_set();
        let index = ClauseIndex::from_clause_set(&clause_set);
        
        let stats = index.statistics();
        assert_eq!(stats.total_clauses, 4);
        assert_eq!(stats.negative_clause_count, 1);
        
        // We have predicates: A, B, C, D, E, F, G (7 unique)
        assert!(stats.indexed_predicates >= 7);
        
        // Average clauses per predicate should be reasonable
        assert!(stats.avg_clauses_per_predicate > 0.0);
        assert!(stats.avg_clauses_per_predicate <= 4.0);
    }
    
    #[test]
    fn test_empty_clause_set() {
        let empty_set = DLClauseSet {
            deterministic_clauses: vec![],
            disjunctive_clauses: vec![],
            abox_facts: vec![],
            prefixes: HashMap::new(),
            statistics: DLClauseStatistics::default(),
        };
        
        let index = ClauseIndex::from_clause_set(&empty_set);
        
        assert!(index.is_empty());
        assert_eq!(index.len(), 0);
        assert_eq!(index.get_negative_clause_ids().len(), 0);
        
        let candidates = index.get_candidate_clauses(&["A".to_string()]);
        assert_eq!(candidates.len(), 0);
    }
    
    #[test]
    fn test_get_clauses_deriving() {
        let clause_set = create_test_clause_set();
        let index = ClauseIndex::from_clause_set(&clause_set);
        
        // Clauses deriving B: clause 0 (A → B)
        let deriving_b = index.get_clauses_deriving(&["B".to_string()]);
        assert_eq!(deriving_b.len(), 1);
        assert_eq!(deriving_b[0].id, "c1");
        
        // Clauses deriving D: clause 1 (B ∧ C → D)
        let deriving_d = index.get_clauses_deriving(&["D".to_string()]);
        assert_eq!(deriving_d.len(), 1);
        assert_eq!(deriving_d[0].id, "c2");
    }
    
    #[test]
    fn test_large_clause_set_performance() {
        use std::time::Instant;
        
        // Create a larger clause set
        let mut clauses = Vec::new();
        for i in 0..100 {
            let clause = create_test_clause(
                &format!("c{}", i),
                vec!["A", "B"],
                vec![&format!("D{}", i)]
            );
            clauses.push(clause);
        }
        
        let clause_set = DLClauseSet {
            deterministic_clauses: clauses,
            disjunctive_clauses: vec![],
            abox_facts: vec![],
            prefixes: HashMap::new(),
            statistics: DLClauseStatistics {
                deterministic_clause_count: 100,
                disjunctive_clause_count: 0,
                disjunction_count: 0,
                positive_fact_count: 0,
                negative_fact_count: 0,
            },
        };
        
        // Build index and measure time
        let start = Instant::now();
        let index = ClauseIndex::from_clause_set(&clause_set);
        let build_time = start.elapsed();
        
        assert_eq!(index.len(), 100);
        assert!(build_time.as_millis() < 10, "Index build should be fast (<10ms)");
        
        // Query and measure time
        let start = Instant::now();
        let candidates = index.get_candidate_clauses(&["A".to_string()]);
        let query_time = start.elapsed();
        
        assert_eq!(candidates.len(), 100); // All clauses have A in body
        assert!(query_time.as_micros() < 1000, "Query should be very fast (<1ms)");
    }
    
    #[test]
    fn test_candidate_clause_refs() {
        let clause_set = create_test_clause_set();
        let index = ClauseIndex::from_clause_set(&clause_set);
        
        let clause_refs = index.get_candidate_clause_refs(&["A".to_string()]);
        assert_eq!(clause_refs.len(), 1);
        assert_eq!(clause_refs[0].id, "c1");
        
        let clause_refs = index.get_candidate_clause_refs(&["B".to_string()]);
        assert!(clause_refs.len() >= 1);
        assert!(clause_refs.iter().any(|c| c.id == "c2"));
    }
}

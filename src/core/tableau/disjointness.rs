//! Disjointness tracking for clash detection
//!
//! This module tracks disjointness relationships between concepts,
//! including those from DisjointClasses and DisjointUnion axioms.

use crate::core::tableau::equivalence::{ConceptId, EquivalenceClosure};
use crate::ontology::{Axiom, ClassExpression, Ontology};
use crate::{Error, Result};
use std::collections::{HashMap, HashSet};

/// Tracks disjointness relationships between concepts
///
/// This structure maintains disjointness constraints from the ontology
/// and provides efficient lookup for clash detection.
pub struct DisjointnessMap {
    /// Direct disjointness: concept -> set of disjoint concepts
    disjoint_pairs: HashMap<ConceptId, HashSet<ConceptId>>,

    /// Disjoint unions: parent -> disjoint children
    disjoint_unions: HashMap<ConceptId, Vec<ConceptId>>,
}

impl DisjointnessMap {
    /// Create a new empty disjointness map
    pub fn new() -> Self {
        Self {
            disjoint_pairs: HashMap::new(),
            disjoint_unions: HashMap::new(),
        }
    }

    /// Build disjointness map from ontology
    ///
    /// Processes DisjointClasses and DisjointUnion axioms to build
    /// the complete disjointness map.
    pub fn from_ontology(ontology: &Ontology, _eq_closure: &EquivalenceClosure) -> Result<Self> {
        let mut map = Self::new();

        log::debug!("Building disjointness map from ontology");

        for axiom in ontology.axioms() {
            match axiom {
                // Handle DisjointClasses axioms
                Axiom::DisjointClasses(disj) => {
                    log::trace!(
                        "Processing DisjointClasses with {} classes",
                        disj.classes.len()
                    );

                    // Add pairwise disjointness for all pairs
                    for i in 0..disj.classes.len() {
                        for j in (i + 1)..disj.classes.len() {
                            let c1 = ConceptId::from_class_expression(&disj.classes[i]);
                            let c2 = ConceptId::from_class_expression(&disj.classes[j]);

                            log::trace!("Adding disjoint pair: {:?} ⊥ {:?}", c1, c2);
                            map.add_disjoint_pair(c1, c2);
                        }
                    }
                }

                // Handle DisjointUnion axioms
                Axiom::DisjointUnion(du) => {
                    let parent = ConceptId::from_class_expression(&du.class);
                    let children: Vec<ConceptId> = du
                        .disjoint_classes
                        .iter()
                        .map(ConceptId::from_class_expression)
                        .collect();

                    log::trace!("Processing DisjointUnion: {:?} = ⊔ {:?}", parent, children);

                    // Add pairwise disjointness for all children
                    for i in 0..children.len() {
                        for j in (i + 1)..children.len() {
                            log::trace!(
                                "Adding disjoint pair from union: {:?} ⊥ {:?}",
                                children[i],
                                children[j]
                            );
                            map.add_disjoint_pair(children[i].clone(), children[j].clone());
                        }
                    }

                    map.disjoint_unions.insert(parent, children);
                }

                _ => {}
            }
        }

        log::info!(
            "Built disjointness map with {} disjoint pairs and {} disjoint unions",
            map.disjoint_pairs.len(),
            map.disjoint_unions.len()
        );

        Ok(map)
    }

    /// Add a disjoint pair (symmetric relationship)
    fn add_disjoint_pair(&mut self, c1: ConceptId, c2: ConceptId) {
        self.disjoint_pairs
            .entry(c1.clone())
            .or_insert_with(HashSet::new)
            .insert(c2.clone());

        self.disjoint_pairs
            .entry(c2)
            .or_insert_with(HashSet::new)
            .insert(c1);
    }

    /// Check if two concepts are directly disjoint
    pub fn are_disjoint(&self, c1: &ConceptId, c2: &ConceptId) -> bool {
        self.disjoint_pairs
            .get(c1)
            .map(|set| set.contains(c2))
            .unwrap_or(false)
    }

    /// Get all concepts that are disjoint with the given concept
    pub fn get_disjoint_concepts(&self, concept: &ConceptId) -> HashSet<ConceptId> {
        self.disjoint_pairs
            .get(concept)
            .cloned()
            .unwrap_or_default()
    }

    /// Check for equivalence-disjointness violations
    ///
    /// This is the key check for detecting inconsistencies: if two concepts
    /// are both equivalent (from equivalence closure) and disjoint, the
    /// ontology is inconsistent.
    ///
    /// Returns the violating concepts if found, None otherwise.
    pub fn check_equivalence_consistency(
        &self,
        eq_closure: &mut EquivalenceClosure,
    ) -> Option<Vec<ConceptId>> {
        log::debug!("Checking for equivalence-disjointness violations");

        // Check each equivalence class
        for (_root, members) in &eq_closure.classes {
            // Skip singleton classes (no possible violations)
            if members.len() <= 1 {
                continue;
            }

            let members_vec: Vec<_> = members.iter().collect();

            // Check all pairs within the equivalence class
            for i in 0..members_vec.len() {
                for j in (i + 1)..members_vec.len() {
                    if self.are_disjoint(members_vec[i], members_vec[j]) {
                        // Found violation! Two concepts are both equivalent and disjoint
                        log::warn!(
                            "Equivalence-disjointness violation detected: {:?} ≡ {:?} but {:?} ⊥ {:?}",
                            members_vec[i],
                            members_vec[j],
                            members_vec[i],
                            members_vec[j]
                        );

                        return Some(vec![members_vec[i].clone(), members_vec[j].clone()]);
                    }
                }
            }
        }

        log::debug!("No equivalence-disjointness violations found");
        None
    }
}

impl Default for DisjointnessMap {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disjoint_pair() {
        let mut map = DisjointnessMap::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());

        map.add_disjoint_pair(a.clone(), b.clone());

        assert!(map.are_disjoint(&a, &b));
        assert!(map.are_disjoint(&b, &a)); // Symmetric
    }

    #[test]
    fn test_not_disjoint() {
        let map = DisjointnessMap::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());

        assert!(!map.are_disjoint(&a, &b));
    }

    #[test]
    fn test_get_disjoint_concepts() {
        let mut map = DisjointnessMap::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());
        let c = ConceptId("C".to_string());

        map.add_disjoint_pair(a.clone(), b.clone());
        map.add_disjoint_pair(a.clone(), c.clone());

        let disjoint = map.get_disjoint_concepts(&a);
        assert_eq!(disjoint.len(), 2);
        assert!(disjoint.contains(&b));
        assert!(disjoint.contains(&c));
    }

    #[test]
    fn test_equivalence_disjointness_violation() {
        let mut eq_closure = EquivalenceClosure::new();
        let mut disj_map = DisjointnessMap::new();

        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());

        // Make A≡B
        eq_closure.add_equivalence(a.clone(), b.clone());
        eq_closure.build_classes();

        // Make A⊥B
        disj_map.add_disjoint_pair(a.clone(), b.clone());

        // This should detect the violation
        let violation = disj_map.check_equivalence_consistency(&mut eq_closure);
        assert!(violation.is_some());
    }
}

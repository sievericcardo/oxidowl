//! Equivalence closure computation for concept equivalences
//!
//! This module implements a Union-Find data structure to efficiently
//! track and query equivalence relationships between concepts.

use crate::Result;
use crate::ontology::{Axiom, ClassExpression, Ontology};
use std::collections::{HashMap, HashSet};

/// Concept identifier for equivalence tracking
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct ConceptId(pub String);

impl ConceptId {
    /// Create `ConceptId` from a class expression
    #[must_use]
    pub fn from_class_expression(expr: &ClassExpression) -> Self {
        match expr {
            ClassExpression::Class(c) => ConceptId(c.iri.to_string()),
            _ => ConceptId(format!("{expr:?}")),
        }
    }
}

/// Tracks equivalence classes using Union-Find algorithm
///
/// This structure efficiently computes the transitive closure of
/// equivalence relationships (if A≡B and B≡C, then A≡C).
pub struct EquivalenceClosure {
    /// Parent pointer for Union-Find (path to representative)
    parent: HashMap<ConceptId, ConceptId>,

    /// Rank for union by rank optimization
    rank: HashMap<ConceptId, usize>,

    /// All members of each equivalence class (indexed by representative)
    pub classes: HashMap<ConceptId, HashSet<ConceptId>>,
}

impl EquivalenceClosure {
    /// Create a new empty equivalence closure
    #[must_use]
    pub fn new() -> Self {
        Self {
            parent: HashMap::new(),
            rank: HashMap::new(),
            classes: HashMap::new(),
        }
    }

    /// Build equivalence closure from ontology
    ///
    /// Processes all `EquivalentClasses` axioms and computes the
    /// transitive closure of equivalence relationships.
    pub fn from_ontology(ontology: &Ontology) -> Result<Self> {
        let mut closure = Self::new();

        log::debug!("Building equivalence closure from ontology");

        // Process all EquivalentClasses axioms
        for axiom in ontology.axioms() {
            if let Axiom::EquivalentClasses(equiv) = axiom {
                // Make all classes in the axiom equivalent
                for i in 0..equiv.classes.len() {
                    for j in (i + 1)..equiv.classes.len() {
                        let c1 = ConceptId::from_class_expression(&equiv.classes[i]);
                        let c2 = ConceptId::from_class_expression(&equiv.classes[j]);

                        log::trace!("Adding equivalence: {c1:?} ≡ {c2:?}");
                        closure.add_equivalence(c1, c2);
                    }
                }
            }
        }

        // Build equivalence classes after all unions
        closure.build_classes();

        log::info!(
            "Built equivalence closure with {} equivalence classes",
            closure.classes.len()
        );

        Ok(closure)
    }

    /// Add an equivalence relationship between two concepts
    pub fn add_equivalence(&mut self, c1: ConceptId, c2: ConceptId) {
        // Initialize if needed (each concept starts as its own parent)
        self.parent.entry(c1.clone()).or_insert_with(|| c1.clone());
        self.parent.entry(c2.clone()).or_insert_with(|| c2.clone());
        self.rank.entry(c1.clone()).or_insert(0);
        self.rank.entry(c2.clone()).or_insert(0);

        // Union the two concepts
        self.union(c1, c2);
    }

    /// Find the representative of a concept's equivalence class
    /// Uses path compression for efficiency (O(α(n)) amortized)
    fn find(&mut self, concept: &ConceptId) -> ConceptId {
        if self.parent.get(concept) == Some(concept) {
            return concept.clone();
        }

        // Path compression: make all nodes on path point directly to root
        let parent = self
            .parent
            .get(concept)
            .cloned()
            .unwrap_or_else(|| concept.clone());
        let root = self.find(&parent);
        self.parent.insert(concept.clone(), root.clone());
        root
    }

    /// Union two equivalence classes
    /// Uses union by rank for efficiency
    fn union(&mut self, c1: ConceptId, c2: ConceptId) {
        let root1 = self.find(&c1);
        let root2 = self.find(&c2);

        if root1 == root2 {
            return; // Already in same equivalence class
        }

        let rank1 = *self.rank.get(&root1).unwrap_or(&0);
        let rank2 = *self.rank.get(&root2).unwrap_or(&0);

        // Union by rank: attach smaller tree under larger tree
        if rank1 < rank2 {
            self.parent.insert(root1, root2);
        } else if rank1 > rank2 {
            self.parent.insert(root2, root1);
        } else {
            self.parent.insert(root2, root1.clone());
            self.rank.insert(root1, rank1 + 1);
        }
    }

    /// Build equivalence classes after all unions are complete
    pub fn build_classes(&mut self) {
        self.classes.clear();

        let all_concepts: Vec<ConceptId> = self.parent.keys().cloned().collect();

        for concept in all_concepts {
            let root = self.find(&concept);
            self.classes.entry(root).or_default().insert(concept);
        }
    }

    /// Check if two concepts are in the same equivalence class
    pub fn are_equivalent(&mut self, c1: &ConceptId, c2: &ConceptId) -> bool {
        if !self.parent.contains_key(c1) || !self.parent.contains_key(c2) {
            return false;
        }
        self.find(c1) == self.find(c2)
    }

    /// Get all concepts equivalent to the given concept
    pub fn get_equivalence_class(&mut self, concept: &ConceptId) -> HashSet<ConceptId> {
        if !self.parent.contains_key(concept) {
            return HashSet::new();
        }
        let root = self.find(concept);
        self.classes.get(&root).cloned().unwrap_or_default()
    }

    /// Get the representative of a concept's equivalence class
    pub fn get_representative(&mut self, concept: &ConceptId) -> Option<ConceptId> {
        if !self.parent.contains_key(concept) {
            return None;
        }
        Some(self.find(concept))
    }
}

impl Default for EquivalenceClosure {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_equivalence() {
        let mut closure = EquivalenceClosure::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());

        closure.add_equivalence(a.clone(), b.clone());
        closure.build_classes();

        assert!(closure.are_equivalent(&a, &b));
    }

    #[test]
    fn test_transitive_equivalence() {
        let mut closure = EquivalenceClosure::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());
        let c = ConceptId("C".to_string());

        // A≡B and B≡C should imply A≡C
        closure.add_equivalence(a.clone(), b.clone());
        closure.add_equivalence(b.clone(), c.clone());
        closure.build_classes();

        assert!(closure.are_equivalent(&a, &c));
    }

    #[test]
    fn test_equivalence_classes() {
        let mut closure = EquivalenceClosure::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());
        let c = ConceptId("C".to_string());

        closure.add_equivalence(a.clone(), b.clone());
        closure.add_equivalence(b.clone(), c.clone());
        closure.build_classes();

        let class_a = closure.get_equivalence_class(&a);
        assert_eq!(class_a.len(), 3);
        assert!(class_a.contains(&a));
        assert!(class_a.contains(&b));
        assert!(class_a.contains(&c));
    }

    #[test]
    fn test_separate_classes() {
        let mut closure = EquivalenceClosure::new();
        let a = ConceptId("A".to_string());
        let b = ConceptId("B".to_string());
        let c = ConceptId("C".to_string());
        let d = ConceptId("D".to_string());

        // A≡B and C≡D, but not connected
        closure.add_equivalence(a.clone(), b.clone());
        closure.add_equivalence(c.clone(), d.clone());
        closure.build_classes();

        assert!(closure.are_equivalent(&a, &b));
        assert!(closure.are_equivalent(&c, &d));
        assert!(!closure.are_equivalent(&a, &c));
        assert!(!closure.are_equivalent(&b, &d));
    }
}

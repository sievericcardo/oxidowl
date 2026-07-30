//! Clausal Normal Form converter.
//!
//! Transforms OWL class expressions into CNF (conjunctions of disjunctions).
//! Uses structural transformation: introduces fresh names for complex
//! sub-expressions, then distributes disjunction over conjunction.

use crate::ontology::concepts::ClassExpression;
use crate::transform::nnf::NNFConverter;
use std::collections::HashSet;

/// Converts class expressions to Clausal Normal Form.
pub struct CNFConverter {
    definition_counter: u64,
}

impl CNFConverter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            definition_counter: 0,
        }
    }

    /// Convert a class expression to CNF.
    /// Returns a vector of clauses, where each clause is a vector of literals.
    pub fn to_cnf(&mut self, ce: &ClassExpression) -> Vec<Vec<ClassExpression>> {
        // Step 1: Convert to NNF
        let nnf_ce = NNFConverter::default().to_nnf(ce);

        // Step 2: Apply structural transformation
        let mut clauses = Vec::new();
        let mut definitions = Vec::new();
        let top_level = self.structural_transform(&nnf_ce, &mut definitions);
        clauses.push(top_level);
        clauses.extend(definitions);

        // Step 3: Distribute disjunction over conjunction in each clause
        clauses = self.distribute_all(clauses);

        // Step 4: Remove redundant clauses (supersets)
        self.remove_supersets(&mut clauses);

        clauses
    }

    /// Check if a class expression is in CNF.
    /// In CNF: expression is a conjunction of disjunctions, where each
    /// disjunction is a union of literals (complement-of-named or named).
    #[must_use]
    pub fn is_in_cnf(&self, ce: &ClassExpression) -> bool {
        match ce {
            ClassExpression::ObjectIntersectionOf(conjuncts) => conjuncts
                .iter()
                .all(|c| Self::is_clause(c)),
            _ => Self::is_clause(ce),
        }
    }

    fn is_clause(ce: &ClassExpression) -> bool {
        match ce {
            ClassExpression::ObjectUnionOf(disjuncts) => disjuncts
                .iter()
                .all(Self::is_literal),
            _ => Self::is_literal(ce),
        }
    }

    fn is_literal(ce: &ClassExpression) -> bool {
        matches!(
            ce,
            ClassExpression::Class(_)
                | ClassExpression::ObjectComplementOf(_)
                | ClassExpression::ObjectSomeValuesFrom { .. }
                | ClassExpression::ObjectAllValuesFrom { .. }
        )
    }

    /// Generate a fresh concept name for structural transformation.
    fn fresh_class(&mut self) -> ClassExpression {
        self.definition_counter += 1;
        ClassExpression::Class(crate::ontology::Class {
            iri: crate::ontology::IRI::new(&format!(
                "urn:oxidowl:def#C{}",
                self.definition_counter
            )),
        })
    }

    /// Apply structural transformation to a class expression.
    /// Returns the transformed expression; definitions are collected into
    /// `defs` as clauses of the form X ≡ expr (i.e., X ⊓ ¬expr and ¬X ⊔ expr).
    fn structural_transform(
        &mut self,
        ce: &ClassExpression,
        defs: &mut Vec<Vec<ClassExpression>>,
    ) -> Vec<ClassExpression> {
        match ce {
            // Disjunctions at top level: transform each operand
            ClassExpression::ObjectUnionOf(disjuncts) => {
                let mut clause = Vec::new();
                for d in disjuncts {
                    match d {
                        // Complex sub-expressions become fresh names
                        ClassExpression::ObjectIntersectionOf(_)
                        | ClassExpression::ObjectSomeValuesFrom { .. }
                        | ClassExpression::ObjectAllValuesFrom { .. }
                        | ClassExpression::ObjectMinCardinality { .. }
                        | ClassExpression::ObjectMaxCardinality { .. }
                        | ClassExpression::ObjectExactCardinality { .. } => {
                            let fresh = self.fresh_class();
                            // Add definition: fresh ≡ d
                            let neg_fresh =
                                ClassExpression::ObjectComplementOf(Box::new(fresh.clone()));
                            defs.push(vec![neg_fresh.clone(), d.clone()]);
                            defs.push(vec![fresh.clone(), Self::negate(d)]);
                            clause.push(fresh);
                        }
                        _ => {
                            clause.push(d.clone());
                        }
                    }
                }
                clause
            }
            // Conjunctions: return empty clause (top is TRUE, no clause needed)
            ClassExpression::ObjectIntersectionOf(conjuncts) => {
                let fresh = self.fresh_class();
                for c in conjuncts {
                    let sub = self.structural_transform(c, defs);
                    if !sub.is_empty() {
                        let neg_fresh =
                            ClassExpression::ObjectComplementOf(Box::new(fresh.clone()));
                        defs.push(vec![neg_fresh, c.clone()]);
                    }
                }
                vec![fresh]
            }
            // Named class or complement: return as-is (already a literal)
            ClassExpression::Class(_)
            | ClassExpression::ObjectComplementOf(_) => {
                vec![ce.clone()]
            }
            // Quantifiers and cardinalities at top level
            ClassExpression::ObjectSomeValuesFrom { .. }
            | ClassExpression::ObjectAllValuesFrom { .. }
            | ClassExpression::ObjectHasValue { .. }
            | ClassExpression::ObjectHasSelf { .. }
            | ClassExpression::ObjectMinCardinality { .. }
            | ClassExpression::ObjectMaxCardinality { .. }
            | ClassExpression::ObjectExactCardinality { .. } => {
                let fresh = self.fresh_class();
                let neg_fresh = ClassExpression::ObjectComplementOf(Box::new(fresh.clone()));
                defs.push(vec![neg_fresh, ce.clone()]);
                defs.push(vec![fresh.clone(), Self::negate(ce)]);
                vec![fresh]
            }
            // Other cases: return as-is
            _ => vec![ce.clone()],
        }
    }

    /// Compute the negation of a class expression (in NNF).
    fn negate(ce: &ClassExpression) -> ClassExpression {
        match ce {
            ClassExpression::Class(_) => {
                ClassExpression::ObjectComplementOf(Box::new(ce.clone()))
            }
            ClassExpression::ObjectComplementOf(inner) => *inner.clone(),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                ClassExpression::ObjectAllValuesFrom {
                    property: property.clone(),
                    filler: Box::new(Self::negate(filler)),
                }
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                ClassExpression::ObjectSomeValuesFrom {
                    property: property.clone(),
                    filler: Box::new(Self::negate(filler)),
                }
            }
            _ => ClassExpression::ObjectComplementOf(Box::new(ce.clone())),
        }
    }

    /// Distribute disjunction over conjunction in a set of clauses.
    fn distribute_all(&self, clauses: Vec<Vec<ClassExpression>>) -> Vec<Vec<ClassExpression>> {
        clauses
            .into_iter()
            .map(|clause| self.distribute_one(clause))
            .flatten()
            .collect()
    }

    /// Apply (A ⊓ B) ⊔ C → (A ⊔ C) ⊓ (B ⊔ C) for one clause.
    fn distribute_one(&self, clause: Vec<ClassExpression>) -> Vec<Vec<ClassExpression>> {
        let mut result = vec![clause];
        loop {
            let mut changed = false;
            let mut new_result = Vec::new();
            for c in &result {
                // Find first conjunction as a disjunct
                if let Some(idx) = c.iter().position(|d| {
                    matches!(d, ClassExpression::ObjectIntersectionOf(_))
                }) {
                    changed = true;
                    if let ClassExpression::ObjectIntersectionOf(conjuncts) = &c[idx] {
                        let mut rest = c.clone();
                        rest.remove(idx);
                        for conj in conjuncts {
                            let mut new_clause = rest.clone();
                            new_clause.push(conj.clone());
                            new_result.push(new_clause);
                        }
                    }
                } else {
                    new_result.push(c.clone());
                }
            }
            result = new_result;
            if !changed {
                break;
            }
        }
        result
    }

    /// Remove clauses that are supersets of other clauses.
    fn remove_supersets(&self, clauses: &mut Vec<Vec<ClassExpression>>) {
        let mut to_remove = HashSet::new();
        for i in 0..clauses.len() {
            for j in 0..clauses.len() {
                if i != j && !to_remove.contains(&j) {
                    if Self::is_superset(&clauses[i], &clauses[j]) {
                        to_remove.insert(i);
                    }
                }
            }
        }
        // Remove in reverse order
        let mut indices: Vec<_> = to_remove.into_iter().collect();
        indices.sort_by(|a, b| b.cmp(a));
        for idx in indices {
            clauses.remove(idx);
        }
    }

    fn is_superset(a: &[ClassExpression], b: &[ClassExpression]) -> bool {
        b.iter().all(|be| a.contains(be))
    }
}

impl Default for CNFConverter {
    fn default() -> Self {
        Self::new()
    }
}

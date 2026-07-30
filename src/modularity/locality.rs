//! Locality Evaluators — determine if axioms are local w.r.t. a signature.

use crate::ontology::axioms::*;
use crate::ontology::{ClassExpression, ObjectPropertyExpression, IRI};
use std::collections::HashSet;

/// Locality type for module extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocalityClass {
    /// ⊤-locality: check if axiom is trivially true.
    Top,
    /// ⊥-locality: check if axiom is trivially false.
    Bottom,
    /// ∅-locality (Star): specialized for module extraction.
    Star,
    /// ρ-locality: extended syntactic check.
    Rho,
}

/// Determines whether an axiom is local with respect to a signature.
/// An axiom is local if it does not provide new knowledge about the signature.
pub trait LocalityEvaluator: Send + Sync {
    fn is_local(&self, axiom: &Axiom, signature: &HashSet<IRI>) -> bool;
}

// ── SyntacticLocalityEvaluator ───────────────────────────────────────────────

/// Fast syntactic locality check based on axiom shape analysis.
pub struct SyntacticLocalityEvaluator {
    #[allow(dead_code)]
    locality_class: LocalityClass,
}

impl SyntacticLocalityEvaluator {
    #[must_use]
    pub fn new(locality_class: LocalityClass) -> Self { Self { locality_class } }
}

impl LocalityEvaluator for SyntacticLocalityEvaluator {
    fn is_local(&self, axiom: &Axiom, signature: &HashSet<IRI>) -> bool {
        match axiom {
            Axiom::SubClassOf(a) => {
                let lhs_local = self.ce_is_top_local(&a.subclass, signature);
                let rhs_local = self.ce_is_bottom_local(&a.superclass, signature);
                lhs_local || rhs_local
            }
            Axiom::EquivalentClasses(a) => {
                // Local if all classes are local (trivially equivalent)
                a.classes.iter().all(|c| self.ce_is_top_local(c, signature))
                    || a.classes.iter().all(|c| self.ce_is_bottom_local(c, signature))
            }
            Axiom::DisjointClasses(a) => {
                a.classes.iter().all(|c| self.ce_is_bottom_local(c, signature))
            }
            Axiom::SubObjectPropertyOf(a) => {
                self.ope_is_top_local(&a.sub_property, signature)
                    || self.ope_is_bottom_local(&a.super_property, signature)
            }
            Axiom::ClassAssertion(a) => {
                self.ce_is_bottom_local(&a.class, signature)
            }
            _ => false,
        }
    }
}

impl SyntacticLocalityEvaluator {
    /// Check if a class expression is ⊤-local (trivially true, no new info about signature).
    fn ce_is_top_local(&self, ce: &ClassExpression, sig: &HashSet<IRI>) -> bool {
        match ce {
            ClassExpression::Class(cls) => !sig.contains(&cls.iri),
            ClassExpression::ObjectIntersectionOf(ops) => ops.iter().all(|op| self.ce_is_top_local(op, sig)),
            ClassExpression::ObjectUnionOf(ops) => ops.iter().any(|op| self.ce_is_top_local(op, sig)),
            ClassExpression::ObjectComplementOf(inner) => self.ce_is_bottom_local(inner, sig),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.ope_is_top_local(property, sig) || self.ce_is_top_local(filler, sig)
            }
            ClassExpression::ObjectAllValuesFrom { property: _, filler } => {
                self.ce_is_top_local(filler, sig)
            }
            ClassExpression::ObjectHasValue { property, value: _ } => self.ope_is_top_local(property, sig),
            ClassExpression::ObjectHasSelf { property } => self.ope_is_top_local(property, sig),
            ClassExpression::ObjectMinCardinality { cardinality, .. } if *cardinality == 0 => true,
            ClassExpression::ObjectOneOf(inds) => inds.is_empty(),
            _ => false,
        }
    }

    fn ce_is_bottom_local(&self, ce: &ClassExpression, sig: &HashSet<IRI>) -> bool {
        match ce {
            ClassExpression::Class(cls) => !sig.contains(&cls.iri),
            ClassExpression::ObjectIntersectionOf(ops) => ops.iter().any(|op| self.ce_is_bottom_local(op, sig)),
            ClassExpression::ObjectUnionOf(ops) => ops.iter().all(|op| self.ce_is_bottom_local(op, sig)),
            ClassExpression::ObjectComplementOf(inner) => self.ce_is_top_local(inner, sig),
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                self.ope_is_top_local(property, sig) || self.ce_is_bottom_local(filler, sig)
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.ope_is_bottom_local(property, sig) && self.ce_is_bottom_local(filler, sig)
            }
            _ => false,
        }
    }

    fn ope_is_top_local(&self, ope: &ObjectPropertyExpression, sig: &HashSet<IRI>) -> bool {
        match ope {
            ObjectPropertyExpression::ObjectProperty(p) => !sig.contains(&p.iri),
            ObjectPropertyExpression::InverseObjectProperty(p) => !sig.contains(&p.iri),
            ObjectPropertyExpression::PropertyChain(_) => false,
        }
    }

    fn ope_is_bottom_local(&self, ope: &ObjectPropertyExpression, sig: &HashSet<IRI>) -> bool {
        match ope {
            ObjectPropertyExpression::ObjectProperty(p) => !sig.contains(&p.iri),
            ObjectPropertyExpression::InverseObjectProperty(p) => !sig.contains(&p.iri),
            ObjectPropertyExpression::PropertyChain(_) => false,
        }
    }
}

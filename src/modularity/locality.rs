//! Locality Evaluators — determine if axioms are local w.r.t. a signature.
//!
//! The `SyntacticLocalityEvaluator` implements ⊥-locality and ⊤-locality
//! checks as defined in "Syntactic Locality for Module Extraction in OWL 2 DL".
//!
//! **Note:** Rho and Star locality are semantic (require a full reasoner).
//! Only syntactic Top and Bottom locality are implemented here.

use crate::ontology::axioms::*;
use crate::ontology::{ClassExpression, DataPropertyExpression, IRI, ObjectPropertyExpression};
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
    pub fn new(locality_class: LocalityClass) -> Self {
        Self { locality_class }
    }
}

impl LocalityEvaluator for SyntacticLocalityEvaluator {
    fn is_local(&self, axiom: &Axiom, signature: &HashSet<IRI>) -> bool {
        let use_top = !matches!(self.locality_class, LocalityClass::Bottom);
        match axiom {
            // ── Class Axioms ──────────────────────────────────────────────
            Axiom::SubClassOf(a) => {
                if use_top {
                    self.ce_is_top_local(&a.superclass, signature)
                } else {
                    self.ce_is_bottom_local(&a.subclass, signature)
                }
            }
            Axiom::EquivalentClasses(a) => {
                a.classes
                    .iter()
                    .all(|c| self.ce_is_top_local(c, signature))
                    || a.classes
                        .iter()
                        .all(|c| self.ce_is_bottom_local(c, signature))
            }
            Axiom::DisjointClasses(a) => a
                .classes
                .iter()
                .all(|c| self.ce_is_bottom_local(c, signature)),
            Axiom::DisjointUnion(a) => {
                self.ce_is_top_local(&a.class, signature)
                    && a.disjoint_classes
                        .iter()
                        .all(|c| self.ce_is_bottom_local(c, signature))
            }
            Axiom::ClassAssertion(a) => self.ce_is_bottom_local(&a.class, signature),

            // ── Object Property Axioms ────────────────────────────────────
            Axiom::SubObjectPropertyOf(a) => {
                self.ope_is_top_local(&a.sub_property, signature)
                    || self.ope_is_bottom_local(&a.super_property, signature)
            }
            Axiom::EquivalentObjectProperties(a) => {
                a.properties
                    .iter()
                    .all(|p| self.ope_is_top_local(p, signature))
                    || a.properties
                        .iter()
                        .all(|p| self.ope_is_bottom_local(p, signature))
            }
            Axiom::DisjointObjectProperties(a) => a
                .properties
                .iter()
                .all(|p| self.ope_is_bottom_local(p, signature)),
            Axiom::InverseObjectProperties(a) => {
                (self.ope_is_top_local(&a.property1, signature)
                    && self.ope_is_top_local(&a.property2, signature))
                    || (self.ope_is_bottom_local(&a.property1, signature)
                        && self.ope_is_bottom_local(&a.property2, signature))
            }
            Axiom::ObjectPropertyDomain(a) => {
                self.ope_is_top_local(&a.property, signature)
                    || self.ce_is_top_local(&a.domain, signature)
            }
            Axiom::ObjectPropertyRange(a) => {
                self.ope_is_top_local(&a.property, signature)
                    || self.ce_is_top_local(&a.range, signature)
            }
            Axiom::FunctionalObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::InverseFunctionalObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::ReflexiveObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::IrreflexiveObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::SymmetricObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::AsymmetricObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::TransitiveObjectProperty(a) => {
                self.ope_is_top_local(&a.property, signature)
            }
            Axiom::ObjectPropertyAssertion(a) => {
                self.ope_is_bottom_local(&a.property, signature)
            }
            Axiom::NegativeObjectPropertyAssertion(a) => {
                self.ope_is_bottom_local(&a.property, signature)
            }

            // ── Data Property Axioms ──────────────────────────────────────
            Axiom::SubDataPropertyOf(a) => {
                self.dpe_is_top_local(&a.sub_property, signature)
                    || self.dpe_is_bottom_local(&a.super_property, signature)
            }
            Axiom::EquivalentDataProperties(a) => {
                a.properties
                    .iter()
                    .all(|p| self.dpe_is_top_local(p, signature))
                    || a.properties
                        .iter()
                        .all(|p| self.dpe_is_bottom_local(p, signature))
            }
            Axiom::DisjointDataProperties(a) => a
                .properties
                .iter()
                .all(|p| self.dpe_is_bottom_local(p, signature)),
            Axiom::DataPropertyDomain(a) => {
                self.dpe_is_top_local(&a.property, signature)
                    || self.ce_is_top_local(&a.domain, signature)
            }
            Axiom::DataPropertyRange(a) => {
                // Range can contain data ranges; if property is top-local, axiom is local
                self.dpe_is_top_local(&a.property, signature)
            }
            Axiom::FunctionalDataProperty(a) => {
                self.dpe_is_top_local(&a.property, signature)
            }
            Axiom::DataPropertyAssertion(a) => {
                self.dpe_is_bottom_local(&a.property, signature)
            }
            Axiom::NegativeDataPropertyAssertion(a) => {
                self.dpe_is_bottom_local(&a.property, signature)
            }

            // ── Individual Axioms ─────────────────────────────────────────
            Axiom::SameIndividual(a) => a.individuals.len() <= 1,
            Axiom::DifferentIndividuals(a) => a.individuals.len() <= 1,

            // ── Annotation Axioms ─────────────────────────────────────────
            // Annotation axioms are always non-local for the signature
            // because annotation properties live in a separate namespace.
            Axiom::AnnotationAssertion(_)
            | Axiom::SubAnnotationPropertyOf(_)
            | Axiom::AnnotationPropertyDomain(_)
            | Axiom::AnnotationPropertyRange(_) => false,

            // ── Miscellaneous ──────────────────────────────────────────────
            Axiom::Declaration(_) => true,
            Axiom::HasKey(a) => self.ce_is_top_local(&a.class, signature),
            Axiom::DatatypeDefinition(_) => false,
            Axiom::Rule(_) => false,
        }
    }
}

impl SyntacticLocalityEvaluator {
    /// Check if a class expression is ⊤-local (trivially true, no new info about signature).
    fn ce_is_top_local(&self, ce: &ClassExpression, sig: &HashSet<IRI>) -> bool {
        match ce {
            ClassExpression::Class(cls) => !sig.contains(&cls.iri),
            ClassExpression::ObjectIntersectionOf(ops) => {
                ops.iter().all(|op| self.ce_is_top_local(op, sig))
            }
            ClassExpression::ObjectUnionOf(ops) => {
                ops.iter().any(|op| self.ce_is_top_local(op, sig))
            }
            ClassExpression::ObjectComplementOf(inner) => self.ce_is_bottom_local(inner, sig),
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                self.ope_is_top_local(property, sig) || self.ce_is_top_local(filler, sig)
            }
            ClassExpression::ObjectAllValuesFrom {
                property: _,
                filler,
            } => self.ce_is_top_local(filler, sig),
            ClassExpression::ObjectHasValue { property, value: _ } => {
                self.ope_is_top_local(property, sig)
            }
            ClassExpression::ObjectHasSelf { property } => self.ope_is_top_local(property, sig),
            ClassExpression::ObjectMinCardinality { cardinality, .. } if *cardinality == 0 => true,
            ClassExpression::ObjectOneOf(inds) => inds.is_empty(),
            _ => false,
        }
    }

    fn ce_is_bottom_local(&self, ce: &ClassExpression, sig: &HashSet<IRI>) -> bool {
        match ce {
            ClassExpression::Class(cls) => !sig.contains(&cls.iri),
            ClassExpression::ObjectIntersectionOf(ops) => {
                ops.iter().any(|op| self.ce_is_bottom_local(op, sig))
            }
            ClassExpression::ObjectUnionOf(ops) => {
                ops.iter().all(|op| self.ce_is_bottom_local(op, sig))
            }
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

    fn dpe_is_top_local(&self, dpe: &DataPropertyExpression, sig: &HashSet<IRI>) -> bool {
        match dpe {
            DataPropertyExpression::DataProperty(p) => !sig.contains(&p.iri),
        }
    }

    fn dpe_is_bottom_local(&self, dpe: &DataPropertyExpression, sig: &HashSet<IRI>) -> bool {
        match dpe {
            DataPropertyExpression::DataProperty(p) => !sig.contains(&p.iri),
        }
    }
}

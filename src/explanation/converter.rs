//! SatisfiabilityConverter — converts entailment checks to consistency checks.

use crate::manager::changes::OntologyChange;
use crate::ontology::axioms::*;
use crate::ontology::{ClassExpression, IRI, Individual, OntologyRef};

/// Converts "O entails α" queries into "O ∪ {¬α} is inconsistent" queries.
/// Enables uniform explanation via consistency checking.
pub struct SatisfiabilityConverter;

impl SatisfiabilityConverter {
    /// Convert an entailment check to a satisfiability test.
    /// Returns (temp_ontology, cleanup_changes) where temp_ontology = O ∪ {¬α}.
    pub fn convert(
        ontology: &OntologyRef,
        entailment: &Axiom,
    ) -> (OntologyRef, Vec<OntologyChange>) {
        let negated = Self::negate_axiom(entailment);
        let guard = ontology.read().unwrap_or_else(|e| e.into_inner());
        let mut o = guard.clone();
        let iri = guard
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:temp"));
        drop(guard);

        let mut changes = Vec::new();
        for nax in &negated {
            o.add_axiom(nax.clone());
            changes.push(OntologyChange::RemoveAxiom {
                ontology_iri: iri.clone(),
                axiom: nax.clone(),
            });
        }

        (OntologyRef::new(std::sync::RwLock::new(o)), changes)
    }

    /// Negate an axiom: return axioms that, when added to O, make O∪{¬α} unsatisfiable iff O⊨α.
    fn negate_axiom(axiom: &Axiom) -> Vec<Axiom> {
        match axiom {
            Axiom::SubClassOf(a) => {
                let fresh = Individual::Named(crate::ontology::NamedIndividual {
                    iri: IRI::new("urn:fresh:counterexample"),
                });
                vec![
                    Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: 0,
                        class: a.subclass.clone(),
                        individual: fresh.clone(),
                        annotations: vec![],
                    }),
                    Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: 0,
                        class: ClassExpression::ObjectComplementOf(Box::new(a.superclass.clone())),
                        individual: fresh,
                        annotations: vec![],
                    }),
                ]
            }
            Axiom::EquivalentClasses(a) if a.classes.len() >= 2 => {
                let fresh = Individual::Named(crate::ontology::NamedIndividual {
                    iri: IRI::new("urn:fresh:counterexample"),
                });
                vec![
                    Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: 0,
                        class: a.classes[0].clone(),
                        individual: fresh.clone(),
                        annotations: vec![],
                    }),
                    Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: 0,
                        class: ClassExpression::ObjectComplementOf(Box::new(a.classes[1].clone())),
                        individual: fresh,
                        annotations: vec![],
                    }),
                ]
            }
            Axiom::ClassAssertion(a) => {
                // ¬(C(a)) → add ¬C(a)
                vec![Axiom::ClassAssertion(ClassAssertionAxiom {
                    id: 0,
                    class: ClassExpression::ObjectComplementOf(Box::new(a.class.clone())),
                    individual: a.individual.clone(),
                    annotations: vec![],
                })]
            }
            Axiom::ObjectPropertyAssertion(a) => {
                // ¬(P(s,t)) → add NegativeObjectPropertyAssertion P(s,t)
                vec![Axiom::NegativeObjectPropertyAssertion(
                    NegativeObjectPropertyAssertionAxiom {
                        id: 0,
                        source: a.source.clone(),
                        target: a.target.clone(),
                        property: a.property.clone(),
                        annotations: vec![],
                    },
                )]
            }
            Axiom::NegativeObjectPropertyAssertion(a) => {
                // ¬(¬P(s,t)) → add ObjectPropertyAssertion P(s,t)
                vec![Axiom::ObjectPropertyAssertion(
                    ObjectPropertyAssertionAxiom {
                        id: 0,
                        source: a.source.clone(),
                        target: a.target.clone(),
                        property: a.property.clone(),
                        annotations: vec![],
                    },
                )]
            }
            Axiom::SameIndividual(a) if a.individuals.len() >= 2 => {
                // ¬(i1 = i2 = ...) → i1 ≠ i2 ≠ ...
                vec![Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
                    id: 0,
                    individuals: a.individuals.clone(),
                    annotations: vec![],
                })]
            }
            Axiom::DifferentIndividuals(a) if a.individuals.len() >= 2 => {
                // ¬(i1 ≠ i2 ≠ ...) → i1 = i2 = ...
                vec![Axiom::SameIndividual(SameIndividualAxiom {
                    id: 0,
                    individuals: a.individuals.clone(),
                    annotations: vec![],
                })]
            }
            _ => {
                // Unsupported negation — fall back to original axiom
                // for direct structural comparison by the caller.
                vec![axiom.clone()]
            }
        }
    }
}

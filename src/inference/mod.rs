//! Inferred Axiom Generators and Ontology Metrics.

pub mod metrics;

use crate::ontology::{ClassExpression, ObjectPropertyExpression, DataPropertyExpression, Ontology};
use crate::ontology::axioms::*;
use crate::reasoner_api::OWLReasoner;
use std::collections::HashSet;

// ── InferredAxiomGenerator Trait ────────────────────────────────────────────

/// Generates inferred axioms from a classified ontology.
pub trait InferredAxiomGenerator<T: Clone> {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<T>;
    fn get_label(&self) -> &'static str;
}

// ── Generators ───────────────────────────────────────────────────────────────

pub struct InferredSubClassOfAxiomGenerator;
impl InferredAxiomGenerator<SubClassOfAxiom> for InferredSubClassOfAxiomGenerator {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<SubClassOfAxiom> {
        let mut seen_pairs = HashSet::new();
        let mut result = Vec::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    let ce = ClassExpression::Class(crate::ontology::Class { iri: iri.clone() });
                    if let Ok(subs) = reasoner.get_sub_classes(&ce, true) {
                        for node in subs.get_nodes() {
                            for sub in node.get_entities() {
                                if !seen_pairs.insert((ce.clone(), sub.clone())) { continue; }
                                result.push(SubClassOfAxiom { id: 0, subclass: sub.clone(), superclass: ce.clone(), annotations: vec![] });
                            }
                        }
                    }
                }
            }
        }
        result
    }
    fn get_label(&self) -> &'static str { "Inferred SubClassOf Axioms" }
}

pub struct InferredEquivalentClassAxiomGenerator;
impl InferredAxiomGenerator<EquivalentClassesAxiom> for InferredEquivalentClassAxiomGenerator {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<EquivalentClassesAxiom> {
        let mut result = Vec::new();
        let mut seen_sets = HashSet::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    let ce = ClassExpression::Class(crate::ontology::Class { iri: iri.clone() });
                    if let Ok(node) = reasoner.get_equivalent_classes(&ce) {
                        let mut eq_set: Vec<_> = node.get_entities().iter().cloned().collect();
                        eq_set.sort_by(|a, b| format!("{a:?}").cmp(&format!("{b:?}")));
                        if eq_set.len() > 1 && seen_sets.insert(format!("{eq_set:?}")) {
                            result.push(EquivalentClassesAxiom { id: 0, classes: eq_set, annotations: vec![] });
                        }
                    }
                }
            }
        }
        result
    }
    fn get_label(&self) -> &'static str { "Inferred EquivalentClasses Axioms" }
}

pub struct InferredDisjointClassesAxiomGenerator;
impl InferredAxiomGenerator<DisjointClassesAxiom> for InferredDisjointClassesAxiomGenerator {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<DisjointClassesAxiom> {
        let mut result = Vec::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    let ce = ClassExpression::Class(crate::ontology::Class { iri: iri.clone() });
                    if let Ok(disj) = reasoner.get_disjoint_classes(&ce) {
                        for node in disj.get_nodes() {
                            for other in node.get_entities() {
                                result.push(DisjointClassesAxiom { id: 0, classes: vec![ce.clone(), other.clone()], annotations: vec![] });
                            }
                        }
                    }
                }
            }
        }
        result
    }
    fn get_label(&self) -> &'static str { "Inferred DisjointClasses Axioms" }
}

pub struct InferredClassAssertionAxiomGenerator;
impl InferredAxiomGenerator<ClassAssertionAxiom> for InferredClassAssertionAxiomGenerator {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<ClassAssertionAxiom> {
        let mut result = Vec::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::Class(iri) = &d.entity {
                    let ce = ClassExpression::Class(crate::ontology::Class { iri: iri.clone() });
                    if let Ok(instances) = reasoner.get_instances(&ce, false) {
                        for node in instances.get_nodes() {
                            for ind in node.get_entities() {
                                result.push(ClassAssertionAxiom { id: 0, class: ce.clone(), individual: ind.clone(), annotations: vec![] });
                            }
                        }
                    }
                }
            }
        }
        result
    }
    fn get_label(&self) -> &'static str { "Inferred ClassAssertion Axioms" }
}

pub struct InferredSubObjectPropertyAxiomGenerator;
impl InferredAxiomGenerator<SubObjectPropertyOfAxiom> for InferredSubObjectPropertyAxiomGenerator {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<SubObjectPropertyOfAxiom> {
        let mut result = Vec::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::ObjectProperty(iri) = &d.entity {
                    let ope = ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty { iri: iri.clone() });
                    if let Ok(subs) = reasoner.get_sub_object_properties(&ope, false) {
                        for node in subs.get_nodes() {
                            for sub in node.get_entities() {
                                result.push(SubObjectPropertyOfAxiom { id: 0, sub_property: sub.clone(), super_property: ope.clone(), annotations: vec![] });
                            }
                        }
                    }
                }
            }
        }
        result
    }
    fn get_label(&self) -> &'static str { "Inferred SubObjectPropertyOf Axioms" }
}

pub struct InferredSubDataPropertyAxiomGenerator;
impl InferredAxiomGenerator<SubDataPropertyOfAxiom> for InferredSubDataPropertyAxiomGenerator {
    fn create_axioms(&self, ontology: &Ontology, reasoner: &dyn OWLReasoner) -> Vec<SubDataPropertyOfAxiom> {
        let mut result = Vec::new();
        for axiom in ontology.axioms() {
            if let Axiom::Declaration(d) = axiom {
                if let Entity::DataProperty(iri) = &d.entity {
                    let dpe = DataPropertyExpression::DataProperty(crate::ontology::DataProperty { iri: iri.clone() });
                    if let Ok(subs) = reasoner.get_sub_data_properties(&dpe, false) {
                        for node in subs.get_nodes() {
                            for sub in node.get_entities() {
                                result.push(SubDataPropertyOfAxiom { id: 0, sub_property: sub.clone(), super_property: dpe.clone(), annotations: vec![] });
                            }
                        }
                    }
                }
            }
        }
        result
    }
    fn get_label(&self) -> &'static str { "Inferred SubDataPropertyOf Axioms" }
}

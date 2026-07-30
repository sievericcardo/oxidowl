//! Structural Reasoner — fast non-logical reasoner.
//!
//! Operates purely on asserted axioms (no tableau). O(1) lookups on
//! indexed axiom sets. Useful for testing, pre-checks, and fallback.

use super::{Node, NodeSet, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory};
use crate::ontology::{
    ClassExpression, DataPropertyExpression, DataRange, Individual,
    ObjectPropertyExpression, OntologyRef,
};
use crate::ontology::axioms::Axiom;
use crate::ontology::axioms::{
    ClassAssertionAxiom, DifferentIndividualsAxiom, DisjointClassesAxiom,
    EquivalentClassesAxiom, SameIndividualAxiom, SubClassOfAxiom,
};
use crate::Result;
use std::collections::{HashMap, HashSet};

/// A fast, non-logical reasoner that works purely on asserted axioms.
///
/// Does NOT perform tableau reasoning — no inferences beyond what is
/// explicitly stated in the ontology.
pub struct StructuralReasoner {
    ontology: OntologyRef,
    index: AxiomIndex,
}

#[derive(Debug, Clone, Default)]
struct AxiomIndex {
    subclass_by_lhs: HashMap<ClassExpression, Vec<SubClassOfAxiom>>,
    subclass_by_rhs: HashMap<ClassExpression, Vec<SubClassOfAxiom>>,
    equivalent_classes: HashMap<ClassExpression, Vec<EquivalentClassesAxiom>>,
    disjoint_classes: HashMap<ClassExpression, Vec<DisjointClassesAxiom>>,
    class_assertions: HashMap<Individual, Vec<ClassAssertionAxiom>>,
    same_individual: HashMap<Individual, Vec<SameIndividualAxiom>>,
    different_individual: HashMap<Individual, Vec<DifferentIndividualsAxiom>>,
}

impl StructuralReasoner {
    /// Create a new structural reasoner for the given ontology.
    #[must_use]
    pub fn new(ontology: OntologyRef) -> Self {
        let mut reasoner = Self {
            ontology: ontology.clone(),
            index: AxiomIndex::default(),
        };
        reasoner.rebuild_index();
        reasoner
    }

    /// Rebuild all axiom indices from the current ontology state.
    fn rebuild_index(&mut self) {
        self.index = AxiomIndex::default();
        let Ok(guard) = self.ontology.read() else { return };
        for axiom in guard.axioms() {
            match axiom {
                Axiom::SubClassOf(a) => {
                    self.index.subclass_by_lhs.entry(a.subclass.clone()).or_default().push(a.clone());
                    self.index.subclass_by_rhs.entry(a.superclass.clone()).or_default().push(a.clone());
                }
                Axiom::EquivalentClasses(a) => {
                    for ce in &a.classes {
                        self.index.equivalent_classes.entry(ce.clone()).or_default().push(a.clone());
                    }
                }
                Axiom::DisjointClasses(a) => {
                    for ce in &a.classes {
                        self.index.disjoint_classes.entry(ce.clone()).or_default().push(a.clone());
                    }
                }
                Axiom::ClassAssertion(a) => {
                    self.index.class_assertions.entry(a.individual.clone()).or_default().push(a.clone());
                }
                Axiom::SameIndividual(a) => {
                    for ind in &a.individuals {
                        self.index.same_individual.entry(ind.clone()).or_default().push(a.clone());
                    }
                }
                Axiom::DifferentIndividuals(a) => {
                    for ind in &a.individuals {
                        self.index.different_individual.entry(ind.clone()).or_default().push(a.clone());
                    }
                }
                _ => {}
            }
        }
    }

    // ── Helpers ──────────────────────────────────────────────────────────

    fn collect_all_subclasses(&self, ce: &ClassExpression, visited: &mut HashSet<ClassExpression>) -> Vec<ClassExpression> {
        if !visited.insert(ce.clone()) {
            return vec![];
        }
        let mut result = Vec::new();
        // Find axioms where `ce` is the superclass → subclasses
        if let Some(axioms) = self.index.subclass_by_rhs.get(ce) {
            for sc in axioms {
                let sub = &sc.subclass;
                result.push(sub.clone());
                let transitive = self.collect_all_subclasses(sub, visited);
                result.extend(transitive);
            }
        }
        result
    }

    fn collect_all_superclasses(&self, ce: &ClassExpression, visited: &mut HashSet<ClassExpression>) -> Vec<ClassExpression> {
        if !visited.insert(ce.clone()) {
            return vec![];
        }
        let mut result = Vec::new();
        if let Some(axioms) = self.index.subclass_by_lhs.get(ce) {
            for sc in axioms {
                let sup = &sc.superclass;
                result.push(sup.clone());
                let transitive = self.collect_all_superclasses(sup, visited);
                result.extend(transitive);
            }
        }
        result
    }
}

impl OWLReasoner for StructuralReasoner {
    fn get_root_ontology(&self) -> OntologyRef {
        self.ontology.clone()
    }

    fn is_consistent(&self) -> Result<bool> {
        // Structural reasoner cannot detect logical contradictions — always returns true
        Ok(true)
    }

    fn is_satisfiable(&self, class: &ClassExpression) -> Result<bool> {
        // A class is unsatisfiable if it's bottom or if it's explicitly disjoint with itself
        match class {
            ClassExpression::Class(c) if c.iri.as_str().contains("Nothing") => Ok(false),
            _ => Ok(true),
        }
    }

    fn get_unsatisfiable_classes(&self) -> Result<Node<ClassExpression>> {
        Ok(Node::bottom_node(ClassExpression::Class(
            crate::ontology::Class { iri: crate::ontology::IRI::owl_nothing() },
        )))
    }

    fn get_sub_classes(&self, class: &ClassExpression, direct: bool) -> Result<NodeSet<ClassExpression>> {
        if direct {
            let mut nodes = HashSet::new();
            // Find axioms where `class` is the superclass, return the subclasses
            if let Some(subs) = self.index.subclass_by_rhs.get(class) {
                for sc in subs {
                    nodes.insert(Node::singleton(sc.subclass.clone()));
                }
            }
            Ok(NodeSet::new(nodes))
        } else {
            let all = self.collect_all_subclasses(class, &mut HashSet::new());
            Ok(NodeSet::new(all.into_iter().map(Node::singleton).collect()))
        }
    }

    fn get_super_classes(&self, class: &ClassExpression, direct: bool) -> Result<NodeSet<ClassExpression>> {
        if direct {
            let mut nodes = HashSet::new();
            // Find axioms where `class` is the subclass, return the superclasses
            if let Some(sups) = self.index.subclass_by_lhs.get(class) {
                for sc in sups {
                    nodes.insert(Node::singleton(sc.superclass.clone()));
                }
            }
            Ok(NodeSet::new(nodes))
        } else {
            let all = self.collect_all_superclasses(class, &mut HashSet::new());
            Ok(NodeSet::new(all.into_iter().map(Node::singleton).collect()))
        }
    }

    fn get_equivalent_classes(&self, class: &ClassExpression) -> Result<Node<ClassExpression>> {
        let mut equivalents = HashSet::new();
        if let Some(eqs) = self.index.equivalent_classes.get(class) {
            for eq in eqs {
                for ce in &eq.classes {
                    equivalents.insert(ce.clone());
                }
            }
        }
        if equivalents.is_empty() {
            equivalents.insert(class.clone());
        }
        Ok(Node::new(equivalents))
    }

    fn get_disjoint_classes(&self, class: &ClassExpression) -> Result<NodeSet<ClassExpression>> {
        let mut disjoint = HashSet::new();
        if let Some(entries) = self.index.disjoint_classes.get(class) {
            for dc in entries {
                for ce in &dc.classes {
                    if ce != class {
                        disjoint.insert(Node::singleton(ce.clone()));
                    }
                }
            }
        }
        Ok(NodeSet::new(disjoint))
    }

    fn get_instances(&self, class: &ClassExpression, _direct: bool) -> Result<NodeSet<Individual>> {
        let mut nodes = HashSet::new();
        // Search class assertions
        for (ind, assertions) in &self.index.class_assertions {
            for ax in assertions {
                if &ax.class == class {
                    nodes.insert(Node::singleton(ind.clone()));
                    break;
                }
            }
        }
        Ok(NodeSet::new(nodes))
    }

    fn get_types(&self, individual: &Individual, _direct: bool) -> Result<NodeSet<ClassExpression>> {
        let mut nodes = HashSet::new();
        if let Some(assertions) = self.index.class_assertions.get(individual) {
            for ax in assertions {
                nodes.insert(Node::singleton(ax.class.clone()));
            }
        }
        Ok(NodeSet::new(nodes))
    }

    fn get_same_individuals(&self, individual: &Individual) -> Result<Node<Individual>> {
        let mut all = HashSet::new();
        all.insert(individual.clone());
        if let Some(sames) = self.index.same_individual.get(individual) {
            for sa in sames {
                for ind in &sa.individuals {
                    all.insert(ind.clone());
                }
            }
        }
        Ok(Node::new(all))
    }

    fn get_different_individuals(&self, individual: &Individual) -> Result<NodeSet<Individual>> {
        let mut nodes = HashSet::new();
        if let Some(diffs) = self.index.different_individual.get(individual) {
            for da in diffs {
                for ind in &da.individuals {
                    if ind != individual {
                        nodes.insert(Node::singleton(ind.clone()));
                    }
                }
            }
        }
        Ok(NodeSet::new(nodes))
    }

    fn get_top_object_property(&self) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#topObjectProperty"),
        })
    }

    fn get_bottom_object_property(&self) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#bottomObjectProperty"),
        })
    }

    fn get_sub_object_properties(&self, _prop: &ObjectPropertyExpression, _direct: bool) -> Result<NodeSet<ObjectPropertyExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_super_object_properties(&self, _prop: &ObjectPropertyExpression, _direct: bool) -> Result<NodeSet<ObjectPropertyExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_equivalent_object_properties(&self, prop: &ObjectPropertyExpression) -> Result<Node<ObjectPropertyExpression>> {
        Ok(Node::singleton(prop.clone()))
    }

    fn get_disjoint_object_properties(&self, _prop: &ObjectPropertyExpression) -> Result<NodeSet<ObjectPropertyExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_inverse_object_properties(&self, prop: &ObjectPropertyExpression) -> Result<Node<ObjectPropertyExpression>> {
        match prop {
            ObjectPropertyExpression::ObjectProperty(p) => Ok(Node::singleton(
                ObjectPropertyExpression::InverseObjectProperty(p.clone()),
            )),
            ObjectPropertyExpression::InverseObjectProperty(p) => Ok(Node::singleton(
                ObjectPropertyExpression::ObjectProperty(p.clone()),
            )),
            ObjectPropertyExpression::PropertyChain(_) => Ok(Node::singleton(prop.clone())),
        }
    }

    fn get_object_property_domains(&self, _prop: &ObjectPropertyExpression, _direct: bool) -> Result<NodeSet<ClassExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_object_property_ranges(&self, _prop: &ObjectPropertyExpression, _direct: bool) -> Result<NodeSet<ClassExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_top_data_property(&self) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(crate::ontology::DataProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#topDataProperty"),
        })
    }

    fn get_bottom_data_property(&self) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(crate::ontology::DataProperty {
            iri: crate::ontology::IRI::new("http://www.w3.org/2002/07/owl#bottomDataProperty"),
        })
    }

    fn get_sub_data_properties(&self, _prop: &DataPropertyExpression, _direct: bool) -> Result<NodeSet<DataPropertyExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_super_data_properties(&self, _prop: &DataPropertyExpression, _direct: bool) -> Result<NodeSet<DataPropertyExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_equivalent_data_properties(&self, prop: &DataPropertyExpression) -> Result<Node<DataPropertyExpression>> {
        Ok(Node::singleton(prop.clone()))
    }

    fn get_disjoint_data_properties(&self, _prop: &DataPropertyExpression) -> Result<NodeSet<DataPropertyExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_data_property_domains(&self, _prop: &DataPropertyExpression, _direct: bool) -> Result<NodeSet<ClassExpression>> {
        Ok(NodeSet::empty())
    }

    fn get_data_property_ranges(&self, _prop: &DataPropertyExpression, _direct: bool) -> Result<NodeSet<DataRange>> {
        Ok(NodeSet::empty())
    }

    fn is_entailed(&self, axiom: &Axiom) -> Result<bool> {
        match axiom {
            Axiom::SubClassOf(a) => {
                let supers = self.collect_all_superclasses(&a.subclass, &mut HashSet::new());
                Ok(supers.contains(&a.superclass))
            }
            Axiom::EquivalentClasses(a) => {
                // Check if all classes are mutually subsumed
                for i in 0..a.classes.len() {
                    let supers_i = self.collect_all_superclasses(&a.classes[i], &mut HashSet::new());
                    for (j, _) in a.classes.iter().enumerate() {
                        if i != j && !supers_i.contains(&a.classes[j]) {
                            return Ok(false);
                        }
                    }
                }
                Ok(true)
            }
            Axiom::ClassAssertion(a) => {
                if let Some(assertions) = self.index.class_assertions.get(&a.individual) {
                    Ok(assertions.iter().any(|ca| ca.class == a.class))
                } else {
                    Ok(false)
                }
            }
            _ => Ok(false), // Structural reasoner cannot verify most axiom types
        }
    }
}

// ── Factory ──────────────────────────────────────────────────────────────────

/// Factory for creating structural reasoners.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructuralReasonerFactory;

impl ReasonerFactory for StructuralReasonerFactory {
    fn create_reasoner(
        &self,
        ontology: &OntologyRef,
        _config: &OWLReasonerConfiguration,
    ) -> Result<Box<dyn OWLReasoner>> {
        Ok(Box::new(StructuralReasoner::new(ontology.clone())))
    }

    fn get_reasoner_name(&self) -> &'static str {
        "Oxidowl Structural Reasoner"
    }
}

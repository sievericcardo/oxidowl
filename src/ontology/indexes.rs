//! Bidirectional axiom indexes for O(1) ontology lookups.
//!
//! `AxiomIndex` provides pre-built maps from each class/individual/property
//! to the axioms that mention it, eliminating the need to scan the full axiom
//! list on every query.

use crate::ontology::{
    ClassExpression, Individual, ObjectPropertyExpression,
    axioms::Axiom,
};
use std::collections::HashMap;

/// Pre-computed index over all axioms in an ontology snapshot.
///
/// Build once with `AxiomIndex::build(axioms)` and reuse across queries.
/// All maps use `ClassExpression` as keys (which implement `Hash + Eq`).
#[derive(Debug, Default, Clone)]
pub struct AxiomIndex {
    /// SubClassOf axioms indexed by subclass — maps sub → list of supers
    pub subclass_by_sub: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// SubClassOf axioms indexed by superclass — maps super → list of subs
    pub subclass_by_super: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// EquivalentClasses axioms — maps each class → the other equivalent classes
    pub equivalent_by_class: HashMap<ClassExpression, Vec<ClassExpression>>,

    /// ClassAssertion axioms indexed by class — maps class → list of individuals
    pub class_assertion_by_class: HashMap<ClassExpression, Vec<Individual>>,

    /// ClassAssertion axioms indexed by individual — maps individual → list of classes
    pub class_assertion_by_ind: HashMap<Individual, Vec<ClassExpression>>,

    /// ObjectPropertyDomain axioms indexed by property
    pub domain_by_prop: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,

    /// ObjectPropertyRange axioms indexed by property
    pub range_by_prop: HashMap<ObjectPropertyExpression, Vec<ClassExpression>>,

    /// DisjointClasses axioms — maps each class → the classes disjoint with it
    pub disjoint_by_class: HashMap<ClassExpression, Vec<ClassExpression>>,
}

impl AxiomIndex {
    /// Build an index from a slice of axioms.
    #[must_use]
    pub fn build(axioms: &[Axiom]) -> Self {
        let mut idx = Self::default();
        for axiom in axioms {
            match axiom {
                Axiom::SubClassOf(ax) => {
                    idx.subclass_by_sub
                        .entry(ax.subclass.clone())
                        .or_default()
                        .push(ax.superclass.clone());
                    idx.subclass_by_super
                        .entry(ax.superclass.clone())
                        .or_default()
                        .push(ax.subclass.clone());
                }
                Axiom::EquivalentClasses(ax) => {
                    for (i, ci) in ax.classes.iter().enumerate() {
                        for (j, cj) in ax.classes.iter().enumerate() {
                            if i != j {
                                idx.equivalent_by_class
                                    .entry(ci.clone())
                                    .or_default()
                                    .push(cj.clone());
                            }
                        }
                    }
                }
                Axiom::ClassAssertion(ax) => {
                    idx.class_assertion_by_class
                        .entry(ax.class.clone())
                        .or_default()
                        .push(ax.individual.clone());
                    idx.class_assertion_by_ind
                        .entry(ax.individual.clone())
                        .or_default()
                        .push(ax.class.clone());
                }
                Axiom::ObjectPropertyDomain(ax) => {
                    idx.domain_by_prop
                        .entry(ax.property.clone())
                        .or_default()
                        .push(ax.domain.clone());
                }
                Axiom::ObjectPropertyRange(ax) => {
                    idx.range_by_prop
                        .entry(ax.property.clone())
                        .or_default()
                        .push(ax.range.clone());
                }
                Axiom::DisjointClasses(ax) => {
                    for (i, ci) in ax.classes.iter().enumerate() {
                        for (j, cj) in ax.classes.iter().enumerate() {
                            if i != j {
                                idx.disjoint_by_class
                                    .entry(ci.clone())
                                    .or_default()
                                    .push(cj.clone());
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        idx
    }

    /// Look up all direct superclasses of a class expression.
    #[must_use]
    pub fn direct_superclasses(&self, class: &ClassExpression) -> &[ClassExpression] {
        self.subclass_by_sub
            .get(class)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up all direct subclasses of a class expression.
    #[must_use]
    pub fn direct_subclasses(&self, class: &ClassExpression) -> &[ClassExpression] {
        self.subclass_by_super
            .get(class)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Look up all equivalent classes.
    #[must_use]
    pub fn equivalent_classes(&self, class: &ClassExpression) -> &[ClassExpression] {
        self.equivalent_by_class
            .get(class)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

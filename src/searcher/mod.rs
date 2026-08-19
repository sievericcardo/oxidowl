//! Entity Searcher — fast O(1) axiom lookup by entity.
//!
//! Builds an inverted index from entity IRIs to axiom IDs, then
//! provides filtering methods for every axiom type.

use crate::ontology::axioms::AxiomTrait;
use crate::ontology::axioms::*;
use crate::ontology::{
    AnnotationProperty, ClassExpression, DataPropertyExpression, EntityType, IRI, Individual,
    ObjectPropertyExpression, Ontology,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Inverted index: entity IRI → set of axiom IDs mentioning that entity.
#[derive(Debug, Clone, Default)]
pub struct EntityIndex {
    by_entity: HashMap<IRI, HashSet<AxiomId>>,
    axioms: HashMap<AxiomId, Arc<Axiom>>,
}

impl EntityIndex {
    /// Build an index from an ontology's axioms.
    #[must_use]
    pub fn from_ontology(ontology: &Ontology) -> Self {
        let mut index = Self::default();
        for axiom in ontology.axioms() {
            index.add_axiom(axiom.clone());
        }
        index
    }

    /// Add a single axiom to the index.
    pub fn add_axiom(&mut self, axiom: Axiom) {
        let id = axiom.axiom_id();
        let iris = Self::extract_iris(&axiom);
        for iri in iris {
            self.by_entity.entry(iri).or_default().insert(id);
        }
        self.axioms.insert(id, Arc::new(axiom));
    }

    /// Remove a single axiom from the index.
    pub fn remove_axiom(&mut self, axiom: &Axiom) {
        let id = axiom.axiom_id();
        let iris = Self::extract_iris(axiom);
        for iri in iris {
            if let Some(ids) = self.by_entity.get_mut(&iri) {
                ids.remove(&id);
            }
        }
        self.axioms.remove(&id);
    }

    /// Get the axiom by ID.
    pub fn get_axiom(&self, id: AxiomId) -> Option<&Arc<Axiom>> {
        self.axioms.get(&id)
    }

    /// Get all axiom IDs mentioning an entity IRI.
    pub fn ids_for_entity(&self, iri: &IRI) -> HashSet<AxiomId> {
        self.by_entity.get(iri).cloned().unwrap_or_default()
    }

    /// Extract all IRIs from an axiom's signature.
    fn extract_iris(axiom: &Axiom) -> Vec<IRI> {
        let mut iris = Vec::new();
        axiom_extract_iris(axiom, &mut iris);
        iris
    }
}

/// Public wrapper for extracting IRIs from an axiom's signature.
pub fn axiom_extract_iris_public<S: std::hash::BuildHasher>(
    axiom: &Axiom,
    out: &mut HashSet<IRI, S>,
) {
    let mut vec = Vec::new();
    axiom_extract_iris(axiom, &mut vec);
    out.extend(vec);
}

fn axiom_extract_iris(axiom: &Axiom, out: &mut Vec<IRI>) {
    match axiom {
        Axiom::Declaration(d) => {
            if let Some(iri) = entity_iri(&d.entity) {
                out.push(iri);
            }
        }
        Axiom::SubClassOf(a) => {
            ce_extract_iris(&a.subclass, out);
            ce_extract_iris(&a.superclass, out);
        }
        Axiom::EquivalentClasses(a) => {
            for c in &a.classes {
                ce_extract_iris(c, out);
            }
        }
        Axiom::DisjointClasses(a) => {
            for c in &a.classes {
                ce_extract_iris(c, out);
            }
        }
        Axiom::DisjointUnion(a) => {
            ce_extract_iris(&a.class, out);
            for c in &a.disjoint_classes {
                ce_extract_iris(c, out);
            }
        }
        Axiom::SubObjectPropertyOf(a) => {
            ope_extract_iris(&a.sub_property, out);
            ope_extract_iris(&a.super_property, out);
        }
        Axiom::EquivalentObjectProperties(a) => {
            for p in &a.properties {
                ope_extract_iris(p, out);
            }
        }
        Axiom::DisjointObjectProperties(a) => {
            for p in &a.properties {
                ope_extract_iris(p, out);
            }
        }
        Axiom::InverseObjectProperties(a) => {
            ope_extract_iris(&a.property1, out);
            ope_extract_iris(&a.property2, out);
        }
        Axiom::ObjectPropertyDomain(a) => {
            ope_extract_iris(&a.property, out);
            ce_extract_iris(&a.domain, out);
        }
        Axiom::ObjectPropertyRange(a) => {
            ope_extract_iris(&a.property, out);
            ce_extract_iris(&a.range, out);
        }
        Axiom::FunctionalObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::InverseFunctionalObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::ReflexiveObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::IrreflexiveObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::SymmetricObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::AsymmetricObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::TransitiveObjectProperty(a) => {
            ope_extract_iris(&a.property, out);
        }
        Axiom::SubDataPropertyOf(a) => {
            dpe_extract_iris(&a.sub_property, out);
            dpe_extract_iris(&a.super_property, out);
        }
        Axiom::EquivalentDataProperties(a) => {
            for p in &a.properties {
                dpe_extract_iris(p, out);
            }
        }
        Axiom::DisjointDataProperties(a) => {
            for p in &a.properties {
                dpe_extract_iris(p, out);
            }
        }
        Axiom::DataPropertyDomain(a) => {
            dpe_extract_iris(&a.property, out);
            ce_extract_iris(&a.domain, out);
        }
        Axiom::DataPropertyRange(a) => {
            dpe_extract_iris(&a.property, out); /* data range can be simple IRI */
        }
        Axiom::FunctionalDataProperty(a) => {
            dpe_extract_iris(&a.property, out);
        }
        Axiom::ClassAssertion(a) => {
            ind_extract_iris(&a.individual, out);
            ce_extract_iris(&a.class, out);
        }
        Axiom::ObjectPropertyAssertion(a) => {
            ope_extract_iris(&a.property, out);
            ind_extract_iris(&a.source, out);
            ind_extract_iris(&a.target, out);
        }
        Axiom::DataPropertyAssertion(a) => {
            dpe_extract_iris(&a.property, out);
            ind_extract_iris(&a.individual, out);
        }
        Axiom::NegativeObjectPropertyAssertion(a) => {
            ope_extract_iris(&a.property, out);
            ind_extract_iris(&a.source, out);
            ind_extract_iris(&a.target, out);
        }
        Axiom::NegativeDataPropertyAssertion(a) => {
            dpe_extract_iris(&a.property, out);
            ind_extract_iris(&a.individual, out);
        }
        Axiom::SameIndividual(a) => {
            for i in &a.individuals {
                ind_extract_iris(i, out);
            }
        }
        Axiom::DifferentIndividuals(a) => {
            for i in &a.individuals {
                ind_extract_iris(i, out);
            }
        }
        Axiom::AnnotationAssertion(a) => {
            if let crate::ontology::AnnotationSubject::IRI(iri) = &a.subject {
                out.push(iri.clone());
            }
        }
        Axiom::SubAnnotationPropertyOf(a) => {
            out.push(a.sub_property.iri.clone());
            out.push(a.super_property.iri.clone());
        }
        Axiom::AnnotationPropertyDomain(a) => {
            out.push(a.property.iri.clone());
        }
        Axiom::AnnotationPropertyRange(a) => {
            out.push(a.property.iri.clone());
        }
        Axiom::HasKey(a) => {
            ce_extract_iris(&a.class, out);
            for p in &a.object_properties {
                ope_extract_iris(p, out);
            }
            for p in &a.data_properties {
                dpe_extract_iris(p, out);
            }
        }
        Axiom::DatatypeDefinition(_) => {}
        _ => {}
    }
}

fn entity_iri(e: &Entity) -> Option<IRI> {
    Some(e.iri().clone())
}

fn ce_extract_iris(ce: &ClassExpression, out: &mut Vec<IRI>) {
    match ce {
        ClassExpression::Class(cls) => out.push(cls.iri.clone()),
        ClassExpression::ObjectIntersectionOf(ops) | ClassExpression::ObjectUnionOf(ops) => {
            for op in ops {
                ce_extract_iris(op, out);
            }
        }
        ClassExpression::ObjectComplementOf(op) => {
            ce_extract_iris(op, out);
        }
        ClassExpression::ObjectSomeValuesFrom { property, filler }
        | ClassExpression::ObjectAllValuesFrom { property, filler } => {
            ope_extract_iris(property, out);
            ce_extract_iris(filler, out);
        }
        ClassExpression::ObjectHasValue { property, value } => {
            ope_extract_iris(property, out);
            ind_extract_iris(value, out);
        }
        ClassExpression::ObjectHasSelf { property } => {
            ope_extract_iris(property, out);
        }
        ClassExpression::ObjectMinCardinality {
            property, filler, ..
        }
        | ClassExpression::ObjectMaxCardinality {
            property, filler, ..
        }
        | ClassExpression::ObjectExactCardinality {
            property, filler, ..
        } => {
            ope_extract_iris(property, out);
            ce_extract_iris(filler, out);
        }
        ClassExpression::ObjectOneOf(inds) => {
            for ind in inds {
                ind_extract_iris(ind, out);
            }
        }
        ClassExpression::DataSomeValuesFrom { property, .. }
        | ClassExpression::DataAllValuesFrom { property, .. } => {
            dpe_extract_iris(property, out);
        }
        ClassExpression::DataHasValue { property, .. } => {
            dpe_extract_iris(property, out);
        }
        ClassExpression::DataMinCardinality { property, .. }
        | ClassExpression::DataMaxCardinality { property, .. }
        | ClassExpression::DataExactCardinality { property, .. } => {
            dpe_extract_iris(property, out);
        }
    }
}

fn ope_extract_iris(ope: &ObjectPropertyExpression, out: &mut Vec<IRI>) {
    match ope {
        ObjectPropertyExpression::ObjectProperty(p) => out.push(p.iri.clone()),
        ObjectPropertyExpression::InverseObjectProperty(p) => out.push(p.iri.clone()),
        ObjectPropertyExpression::PropertyChain(chain) => {
            for p in chain {
                ope_extract_iris(p, out);
            }
        }
    }
}

fn dpe_extract_iris(dpe: &DataPropertyExpression, out: &mut Vec<IRI>) {
    match dpe {
        DataPropertyExpression::DataProperty(p) => out.push(p.iri.clone()),
    }
}

fn ind_extract_iris(ind: &Individual, out: &mut Vec<IRI>) {
    match ind {
        Individual::Named(n) => out.push(n.iri.clone()),
        Individual::Anonymous(_) => {}
    }
}

// ── EntitySearcher ───────────────────────────────────────────────────────────

/// Optimized O(1) lookup for axioms mentioning a specific entity.
///
/// Usage:
/// ```ignore
/// let index = EntityIndex::from_ontology(&ontology);
/// let searcher = EntitySearcher::new(ontology, &index);
/// let axioms = searcher.get_sub_class_axioms_for_lhs(&class_expr);
/// ```
pub struct EntitySearcher<'a> {
    _ontology: &'a Ontology,
    index: &'a EntityIndex,
}

impl<'a> EntitySearcher<'a> {
    #[must_use]
    pub fn new(ontology: &'a Ontology, index: &'a EntityIndex) -> Self {
        Self {
            _ontology: ontology,
            index,
        }
    }

    // ── Class Axioms ────────────────────────────────────────────────────

    pub fn get_sub_class_axioms_for_lhs(&self, class: &ClassExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms(class, |a| {
            matches!(
                a,
                Axiom::SubClassOf(_) | Axiom::EquivalentClasses(_) | Axiom::DisjointClasses(_)
            )
        })
    }

    pub fn get_sub_class_axioms_for_rhs(&self, class: &ClassExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms(
            class,
            |a| matches!(a, Axiom::SubClassOf(sc) if &sc.superclass == class),
        )
    }

    pub fn get_equivalent_classes_axioms(&self, class: &ClassExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms(class, |a| {
            matches!(
                a,
                Axiom::EquivalentClasses(_) | Axiom::DisjointClasses(_) | Axiom::DisjointUnion(_)
            )
        })
    }

    pub fn get_disjoint_classes_axioms(&self, class: &ClassExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms(class, |a| matches!(a, Axiom::DisjointClasses(_)))
    }

    pub fn get_disjoint_union_axioms(&self, class: &ClassExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms(class, |a| matches!(a, Axiom::DisjointUnion(_)))
    }

    pub fn get_has_key_axioms(&self, class: &ClassExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms(class, |a| matches!(a, Axiom::HasKey(_)))
    }

    // ── Object Property Axioms ──────────────────────────────────────────

    pub fn get_object_property_domain_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| matches!(a, Axiom::ObjectPropertyDomain(_)))
    }

    pub fn get_object_property_range_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| matches!(a, Axiom::ObjectPropertyRange(_)))
    }

    pub fn get_sub_object_property_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| matches!(a, Axiom::SubObjectPropertyOf(_)))
    }

    pub fn get_equivalent_object_properties_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| matches!(a, Axiom::EquivalentObjectProperties(_)))
    }

    pub fn get_disjoint_object_properties_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| matches!(a, Axiom::DisjointObjectProperties(_)))
    }

    pub fn get_inverse_object_properties_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| matches!(a, Axiom::InverseObjectProperties(_)))
    }

    pub fn get_object_property_characteristic_axioms(
        &self,
        prop: &ObjectPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ope(prop, |a| {
            matches!(
                a,
                Axiom::FunctionalObjectProperty(_)
                    | Axiom::InverseFunctionalObjectProperty(_)
                    | Axiom::ReflexiveObjectProperty(_)
                    | Axiom::IrreflexiveObjectProperty(_)
                    | Axiom::SymmetricObjectProperty(_)
                    | Axiom::AsymmetricObjectProperty(_)
                    | Axiom::TransitiveObjectProperty(_)
            )
        })
    }

    // ── Data Property Axioms ────────────────────────────────────────────

    pub fn get_data_property_domain_axioms(
        &self,
        prop: &DataPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_dpe(prop, |a| matches!(a, Axiom::DataPropertyDomain(_)))
    }

    pub fn get_data_property_range_axioms(&self, prop: &DataPropertyExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms_dpe(prop, |a| matches!(a, Axiom::DataPropertyRange(_)))
    }

    pub fn get_sub_data_property_axioms(&self, prop: &DataPropertyExpression) -> Vec<Arc<Axiom>> {
        self.filter_axioms_dpe(prop, |a| matches!(a, Axiom::SubDataPropertyOf(_)))
    }

    pub fn get_equivalent_data_properties_axioms(
        &self,
        prop: &DataPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_dpe(prop, |a| matches!(a, Axiom::EquivalentDataProperties(_)))
    }

    pub fn get_disjoint_data_properties_axioms(
        &self,
        prop: &DataPropertyExpression,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_dpe(prop, |a| matches!(a, Axiom::DisjointDataProperties(_)))
    }

    // ── Individual Axioms ───────────────────────────────────────────────

    pub fn get_class_assertion_axioms(&self, individual: &Individual) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| matches!(a, Axiom::ClassAssertion(_)))
    }

    pub fn get_object_property_assertion_axioms(&self, individual: &Individual) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| {
            matches!(a, Axiom::ObjectPropertyAssertion(_))
        })
    }

    pub fn get_data_property_assertion_axioms(&self, individual: &Individual) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| matches!(a, Axiom::DataPropertyAssertion(_)))
    }

    pub fn get_negative_object_property_assertion_axioms(
        &self,
        individual: &Individual,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| {
            matches!(a, Axiom::NegativeObjectPropertyAssertion(_))
        })
    }

    pub fn get_negative_data_property_assertion_axioms(
        &self,
        individual: &Individual,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| {
            matches!(a, Axiom::NegativeDataPropertyAssertion(_))
        })
    }

    pub fn get_different_individual_axioms(&self, individual: &Individual) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| matches!(a, Axiom::DifferentIndividuals(_)))
    }

    pub fn get_same_individual_axioms(&self, individual: &Individual) -> Vec<Arc<Axiom>> {
        self.filter_axioms_ind(individual, |a| matches!(a, Axiom::SameIndividual(_)))
    }

    // ── Annotation Axioms ───────────────────────────────────────────────

    pub fn get_annotation_assertion_axioms(&self, subject_iri: &IRI) -> Vec<Arc<Axiom>> {
        self.filter_axioms_by_iri(subject_iri, |a| matches!(a, Axiom::AnnotationAssertion(_)))
    }

    pub fn get_sub_annotation_property_axioms(&self, prop_iri: &IRI) -> Vec<Arc<Axiom>> {
        self.filter_axioms_by_iri(prop_iri, |a| matches!(a, Axiom::SubAnnotationPropertyOf(_)))
    }

    pub fn get_annotation_property_domain_axioms(
        &self,
        prop: &AnnotationProperty,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_by_iri(&prop.iri, |a| {
            matches!(a, Axiom::AnnotationPropertyDomain(_))
        })
    }

    pub fn get_annotation_property_range_axioms(
        &self,
        prop: &AnnotationProperty,
    ) -> Vec<Arc<Axiom>> {
        self.filter_axioms_by_iri(&prop.iri, |a| {
            matches!(a, Axiom::AnnotationPropertyRange(_))
        })
    }

    pub fn get_datatype_definition_axioms(&self, datatype: &IRI) -> Vec<Arc<Axiom>> {
        self.filter_axioms_by_iri(datatype, |a| matches!(a, Axiom::DatatypeDefinition(_)))
    }

    pub fn get_declaration_axioms_by_type(&self, entity_type: &EntityType) -> Vec<Arc<Axiom>> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (iri, ids) in &self.index.by_entity {
            for id in ids {
                if seen.contains(id) {
                    continue;
                }
                seen.insert(*id);
                if let Some(ax) = self.index.get_axiom(*id)
                    && let Axiom::Declaration(d) = ax.as_ref()
                {
                    let matches = match entity_type {
                        EntityType::Class => matches!(d.entity, Entity::Class(_)),
                        EntityType::ObjectProperty => {
                            matches!(d.entity, Entity::ObjectProperty(_))
                        }
                        EntityType::DataProperty => {
                            matches!(d.entity, Entity::DataProperty(_))
                        }
                        EntityType::AnnotationProperty => {
                            matches!(d.entity, Entity::AnnotationProperty(_))
                        }
                        EntityType::NamedIndividual => {
                            matches!(d.entity, Entity::NamedIndividual(_))
                        }
                        EntityType::Datatype => matches!(d.entity, Entity::Datatype(_)),
                    };
                    if matches {
                        result.push(ax.clone());
                    }
                }
            }
            let _ = iri;
        }
        result
    }

    // ── Declaration ─────────────────────────────────────────────────────

    pub fn get_declaration_axioms(&self, entity: &Entity) -> Vec<Arc<Axiom>> {
        if let Some(iri) = entity_iri(entity) {
            self.filter_axioms_by_iri(&iri, |a| matches!(a, Axiom::Declaration(_)))
        } else {
            vec![]
        }
    }

    // ── Internal helpers ────────────────────────────────────────────────

    fn filter_axioms<F: Fn(&Axiom) -> bool>(
        &self,
        ce: &ClassExpression,
        pred: F,
    ) -> Vec<Arc<Axiom>> {
        let mut iris = Vec::new();
        ce_extract_iris(ce, &mut iris);
        self.filter_by_iris(&iris, pred)
    }

    fn filter_axioms_ope<F: Fn(&Axiom) -> bool>(
        &self,
        prop: &ObjectPropertyExpression,
        pred: F,
    ) -> Vec<Arc<Axiom>> {
        let mut iris = Vec::new();
        ope_extract_iris(prop, &mut iris);
        self.filter_by_iris(&iris, pred)
    }

    fn filter_axioms_dpe<F: Fn(&Axiom) -> bool>(
        &self,
        prop: &DataPropertyExpression,
        pred: F,
    ) -> Vec<Arc<Axiom>> {
        let mut iris = Vec::new();
        dpe_extract_iris(prop, &mut iris);
        self.filter_by_iris(&iris, pred)
    }

    fn filter_axioms_ind<F: Fn(&Axiom) -> bool>(
        &self,
        ind: &Individual,
        pred: F,
    ) -> Vec<Arc<Axiom>> {
        let mut iris = Vec::new();
        ind_extract_iris(ind, &mut iris);
        self.filter_by_iris(&iris, pred)
    }

    fn filter_axioms_by_iri<F: Fn(&Axiom) -> bool>(&self, iri: &IRI, pred: F) -> Vec<Arc<Axiom>> {
        self.filter_by_iris(std::slice::from_ref(iri), pred)
    }

    fn filter_by_iris<F: Fn(&Axiom) -> bool>(&self, iris: &[IRI], pred: F) -> Vec<Arc<Axiom>> {
        let mut result = Vec::new();
        for iri in iris {
            for id in self.index.ids_for_entity(iri) {
                if let Some(ax) = self.index.get_axiom(id)
                    && pred(ax)
                    && !result.iter().any(|r: &Arc<Axiom>| Arc::ptr_eq(r, ax))
                {
                    result.push(ax.clone());
                }
            }
        }
        result
    }
}

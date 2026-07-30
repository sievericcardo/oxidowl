//! OWL Object Transformation Utilities.
//!
//! Provides `OWLObjectTransformer`, `OWLEntityRenamer`, `OWLEntityRemover`,
//! NNF converter, and DL expressivity checker.

pub mod nnf;
pub mod expressivity;

use crate::ontology::{
    ClassExpression, DataPropertyExpression, DataRange, Individual,
    ObjectPropertyExpression, OntologyRef,
    Annotation, AnnotationValue, IRI,
};
use crate::ontology::axioms::*;

use crate::searcher::{EntityIndex, EntitySearcher};
use crate::manager::changes::OntologyChange;
use crate::Result;
use std::collections::{HashMap, HashSet};


// ── OWLObjectTransformer ─────────────────────────────────────────────────────

/// Generic transformer that applies a function to all OWL objects within
/// an axiom, producing a transformed axiom.
pub struct OWLObjectTransformer {
    ce_fn: Box<dyn Fn(&ClassExpression) -> Option<ClassExpression> + Send + Sync>,
    ope_fn: Box<dyn Fn(&ObjectPropertyExpression) -> Option<ObjectPropertyExpression> + Send + Sync>,
    dpe_fn: Box<dyn Fn(&DataPropertyExpression) -> Option<DataPropertyExpression> + Send + Sync>,
    ind_fn: Box<dyn Fn(&Individual) -> Option<Individual> + Send + Sync>,
    #[allow(dead_code)]
    dr_fn: Box<dyn Fn(&DataRange) -> Option<DataRange> + Send + Sync>,
}

impl OWLObjectTransformer {
    /// Create a transformer that applies `f` to all class expressions.
    pub fn new_ce<F>(f: F) -> Self where F: Fn(&ClassExpression) -> Option<ClassExpression> + Send + Sync + 'static {
        Self {
            ce_fn: Box::new(f),
            ope_fn: Box::new(|x| Some(x.clone())),
            dpe_fn: Box::new(|x| Some(x.clone())),
            ind_fn: Box::new(|x| Some(x.clone())),
            dr_fn: Box::new(|x| Some(x.clone())),
        }
    }

    /// Transform an axiom, applying all registered functions to sub-objects.
    pub fn transform_axiom(&self, axiom: &Axiom) -> Option<Axiom> {
        Some(match axiom {
            Axiom::Declaration(d) => Axiom::Declaration(DeclarationAxiom { id: d.id, entity: d.entity.clone() }),
            Axiom::SubClassOf(a) => Axiom::SubClassOf(SubClassOfAxiom {
                subclass: (self.ce_fn)(&a.subclass)?,
                superclass: (self.ce_fn)(&a.superclass)?,
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::EquivalentClasses(a) => Axiom::EquivalentClasses(EquivalentClassesAxiom {
                classes: a.classes.iter().filter_map(|c| (self.ce_fn)(c)).collect(),
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::DisjointClasses(a) => Axiom::DisjointClasses(DisjointClassesAxiom {
                classes: a.classes.iter().filter_map(|c| (self.ce_fn)(c)).collect(),
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::ClassAssertion(a) => Axiom::ClassAssertion(ClassAssertionAxiom {
                class: (self.ce_fn)(&a.class)?,
                individual: (self.ind_fn)(&a.individual)?,
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::SubObjectPropertyOf(a) => Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                sub_property: (self.ope_fn)(&a.sub_property)?,
                super_property: (self.ope_fn)(&a.super_property)?,
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::ObjectPropertyAssertion(a) => Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                property: (self.ope_fn)(&a.property)?,
                source: (self.ind_fn)(&a.source)?,
                target: (self.ind_fn)(&a.target)?,
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::ObjectPropertyDomain(a) => Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
                property: (self.ope_fn)(&a.property)?,
                domain: (self.ce_fn)(&a.domain)?,
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::ObjectPropertyRange(a) => Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
                property: (self.ope_fn)(&a.property)?,
                range: (self.ce_fn)(&a.range)?,
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            Axiom::DataPropertyAssertion(a) => Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                property: (self.dpe_fn)(&a.property)?,
                individual: (self.ind_fn)(&a.individual)?,
                value: a.value.clone(),
                annotations: a.annotations.iter().filter_map(|ann| self.transform_annotation(ann)).collect(),
                id: a.id,
            }),
            other => other.clone(),
        })
    }

    fn transform_annotation(&self, ann: &Annotation) -> Option<Annotation> {
        Some(Annotation {
            property: ann.property.clone(),
            value: match &ann.value {
                AnnotationValue::IRI(iri) => AnnotationValue::IRI(iri.clone()),
                AnnotationValue::Literal(lit) => AnnotationValue::Literal(lit.clone()),
                AnnotationValue::AnonymousIndividual(a) => AnnotationValue::AnonymousIndividual(a.clone()),
            },
        })
    }
}

// ── OWLEntityRenamer ─────────────────────────────────────────────────────────

/// Renames entities in an ontology by IRI.
pub struct OWLEntityRenamer {
    mappings: HashMap<(IRI, EntityType), IRI>,
}

impl OWLEntityRenamer {
    #[must_use]
    pub fn new() -> Self { Self { mappings: HashMap::new() } }

    /// Map an old IRI to a new IRI for a specific entity type (punning-aware).
    pub fn add_rename(&mut self, old_iri: IRI, new_iri: IRI, entity_type: EntityType) {
        self.mappings.insert((old_iri, entity_type), new_iri);
    }

    /// Generate change operations that rename all matching entities.
    pub fn rename_ontology(&self, ontology: &OntologyRef) -> Result<Vec<OntologyChange>> {
        let guard = ontology.read().map_err(|e| crate::Error::Internal { message: format!("{e}") })?;
        let index = EntityIndex::from_ontology(&guard);
        let _searcher = EntitySearcher::new(&guard, &index);
        let ontology_iri = guard.get_iri().cloned().unwrap_or_else(|| IRI::new("urn:anon"));
        drop(guard);

        let mut changes = Vec::new();
        // Collect all axioms referencing old IRIs, create remove+add pairs
        let mut seen_ids = HashSet::new();
        for ((old_iri, _et), _new_iri) in &self.mappings {
            for id in index.ids_for_entity(old_iri) {
                if !seen_ids.insert(id) { continue; }
                if let Some(ax) = index.get_axiom(id) {
                    changes.push(OntologyChange::RemoveAxiom {
                        ontology_iri: ontology_iri.clone(),
                        axiom: (**ax).clone(),
                    });
                }
            }
        }
        // For now, return just the remove changes (add would require full rename)
        Ok(changes)
    }
}

impl Default for OWLEntityRenamer {
    fn default() -> Self { Self::new() }
}

// ── OWLEntityRemover ─────────────────────────────────────────────────────────

/// Removes all axioms mentioning specified entities.
pub struct OWLEntityRemover {
    entities_to_remove: HashSet<(IRI, EntityType)>,
    remove_declarations: bool,
}

impl OWLEntityRemover {
    #[must_use]
    pub fn new() -> Self {
        Self { entities_to_remove: HashSet::new(), remove_declarations: true }
    }

    /// Add an entity to be removed.
    pub fn add_entity(&mut self, iri: IRI, entity_type: EntityType) {
        self.entities_to_remove.insert((iri, entity_type));
    }

    /// Generate RemoveAxiom changes for all axioms mentioning target entities.
    pub fn remove_entities(&self, ontology: &OntologyRef) -> Result<Vec<OntologyChange>> {
        let guard = ontology.read().map_err(|e| crate::Error::Internal { message: format!("{e}") })?;
        let index = EntityIndex::from_ontology(&guard);
        let ontology_iri = guard.get_iri().cloned().unwrap_or_else(|| IRI::new("urn:anon"));
        drop(guard);

        let mut seen_ids = HashSet::new();
        let mut changes = Vec::new();
        for (iri, _et) in &self.entities_to_remove {
            for id in index.ids_for_entity(iri) {
                if !seen_ids.insert(id) { continue; }
                if let Some(ax) = index.get_axiom(id) {
                    let is_decl = matches!(ax.as_ref(), Axiom::Declaration(_));
                    if !is_decl || self.remove_declarations {
                        changes.push(OntologyChange::RemoveAxiom {
                            ontology_iri: ontology_iri.clone(),
                            axiom: (**ax).clone(),
                        });
                    }
                }
            }
        }
        Ok(changes)
    }
}

impl Default for OWLEntityRemover {
    fn default() -> Self { Self::new() }
}

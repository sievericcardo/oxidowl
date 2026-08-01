//! OWL Object Transformation Utilities.
//!
//! Provides `OWLObjectTransformer`, `OWLEntityRenamer`, `OWLEntityRemover`,
//! NNF converter, and DL expressivity checker.

pub mod cnf;
pub mod expressivity;
pub mod nnf;

use crate::ontology::axioms::*;
use crate::ontology::{
    Annotation, AnnotationProperty, AnnotationSubject, AnnotationValue, ClassExpression,
    DataPropertyExpression, DataRange, IRI, Individual, ObjectPropertyExpression, OntologyRef,
};

use crate::Result;
use crate::manager::changes::OntologyChange;
use crate::searcher::EntityIndex;
use std::collections::{HashMap, HashSet};

// ── OWLObjectTransformer ─────────────────────────────────────────────────────

/// Generic transformer that applies a function to all OWL objects within
/// an axiom, producing a transformed axiom.
pub struct OWLObjectTransformer {
    ce_fn: Box<dyn Fn(&ClassExpression) -> Option<ClassExpression> + Send + Sync>,
    ope_fn:
        Box<dyn Fn(&ObjectPropertyExpression) -> Option<ObjectPropertyExpression> + Send + Sync>,
    dpe_fn: Box<dyn Fn(&DataPropertyExpression) -> Option<DataPropertyExpression> + Send + Sync>,
    ind_fn: Box<dyn Fn(&Individual) -> Option<Individual> + Send + Sync>,
    #[allow(dead_code)]
    dr_fn: Box<dyn Fn(&DataRange) -> Option<DataRange> + Send + Sync>,
}

impl OWLObjectTransformer {
    /// Create a transformer that applies `f` to all class expressions.
    pub fn new_ce<F>(f: F) -> Self
    where
        F: Fn(&ClassExpression) -> Option<ClassExpression> + Send + Sync + 'static,
    {
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
            Axiom::Declaration(d) => Axiom::Declaration(DeclarationAxiom {
                id: d.id,
                entity: d.entity.clone(),
            }),
            Axiom::SubClassOf(a) => Axiom::SubClassOf(SubClassOfAxiom {
                subclass: (self.ce_fn)(&a.subclass)?,
                superclass: (self.ce_fn)(&a.superclass)?,
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::EquivalentClasses(a) => Axiom::EquivalentClasses(EquivalentClassesAxiom {
                classes: a.classes.iter().filter_map(|c| (self.ce_fn)(c)).collect(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::DisjointClasses(a) => Axiom::DisjointClasses(DisjointClassesAxiom {
                classes: a.classes.iter().filter_map(|c| (self.ce_fn)(c)).collect(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::DisjointUnion(a) => Axiom::DisjointUnion(DisjointUnionAxiom {
                class: (self.ce_fn)(&a.class)?,
                disjoint_classes: a
                    .disjoint_classes
                    .iter()
                    .filter_map(|c| (self.ce_fn)(c))
                    .collect(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::ClassAssertion(a) => Axiom::ClassAssertion(ClassAssertionAxiom {
                class: (self.ce_fn)(&a.class)?,
                individual: (self.ind_fn)(&a.individual)?,
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::SubObjectPropertyOf(a) => Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                sub_property: (self.ope_fn)(&a.sub_property)?,
                super_property: (self.ope_fn)(&a.super_property)?,
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::EquivalentObjectProperties(a) => {
                Axiom::EquivalentObjectProperties(EquivalentObjectPropertiesAxiom {
                    properties: a
                        .properties
                        .iter()
                        .filter_map(|p| (self.ope_fn)(p))
                        .collect(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::DisjointObjectProperties(a) => {
                Axiom::DisjointObjectProperties(DisjointObjectPropertiesAxiom {
                    properties: a
                        .properties
                        .iter()
                        .filter_map(|p| (self.ope_fn)(p))
                        .collect(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::InverseObjectProperties(a) => {
                Axiom::InverseObjectProperties(InverseObjectPropertiesAxiom {
                    property1: (self.ope_fn)(&a.property1)?,
                    property2: (self.ope_fn)(&a.property2)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::ObjectPropertyDomain(a) => {
                Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    domain: (self.ce_fn)(&a.domain)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::ObjectPropertyRange(a) => Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
                property: (self.ope_fn)(&a.property)?,
                range: (self.ce_fn)(&a.range)?,
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::ObjectPropertyAssertion(a) => {
                Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    source: (self.ind_fn)(&a.source)?,
                    target: (self.ind_fn)(&a.target)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::DataPropertyAssertion(a) => {
                Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                    property: (self.dpe_fn)(&a.property)?,
                    individual: (self.ind_fn)(&a.individual)?,
                    value: a.value.clone(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::FunctionalObjectProperty(a) => {
                Axiom::FunctionalObjectProperty(FunctionalObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::InverseFunctionalObjectProperty(a) => {
                Axiom::InverseFunctionalObjectProperty(InverseFunctionalObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::ReflexiveObjectProperty(a) => {
                Axiom::ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::IrreflexiveObjectProperty(a) => {
                Axiom::IrreflexiveObjectProperty(IrreflexiveObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::SymmetricObjectProperty(a) => {
                Axiom::SymmetricObjectProperty(SymmetricObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::AsymmetricObjectProperty(a) => {
                Axiom::AsymmetricObjectProperty(AsymmetricObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::TransitiveObjectProperty(a) => {
                Axiom::TransitiveObjectProperty(TransitiveObjectPropertyAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::SubDataPropertyOf(a) => Axiom::SubDataPropertyOf(SubDataPropertyOfAxiom {
                sub_property: (self.dpe_fn)(&a.sub_property)?,
                super_property: (self.dpe_fn)(&a.super_property)?,
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::EquivalentDataProperties(a) => {
                Axiom::EquivalentDataProperties(EquivalentDataPropertiesAxiom {
                    properties: a
                        .properties
                        .iter()
                        .filter_map(|p| (self.dpe_fn)(p))
                        .collect(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::DisjointDataProperties(a) => {
                Axiom::DisjointDataProperties(DisjointDataPropertiesAxiom {
                    properties: a
                        .properties
                        .iter()
                        .filter_map(|p| (self.dpe_fn)(p))
                        .collect(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::DataPropertyDomain(a) => Axiom::DataPropertyDomain(DataPropertyDomainAxiom {
                property: (self.dpe_fn)(&a.property)?,
                domain: (self.ce_fn)(&a.domain)?,
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::DataPropertyRange(a) => Axiom::DataPropertyRange(DataPropertyRangeAxiom {
                property: (self.dpe_fn)(&a.property)?,
                range: a.range.clone(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::FunctionalDataProperty(a) => {
                Axiom::FunctionalDataProperty(FunctionalDataPropertyAxiom {
                    property: (self.dpe_fn)(&a.property)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::SameIndividual(a) => Axiom::SameIndividual(SameIndividualAxiom {
                individuals: a
                    .individuals
                    .iter()
                    .filter_map(|i| (self.ind_fn)(i))
                    .collect(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::DifferentIndividuals(a) => {
                Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
                    individuals: a
                        .individuals
                        .iter()
                        .filter_map(|i| (self.ind_fn)(i))
                        .collect(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::NegativeObjectPropertyAssertion(a) => {
                Axiom::NegativeObjectPropertyAssertion(NegativeObjectPropertyAssertionAxiom {
                    property: (self.ope_fn)(&a.property)?,
                    source: (self.ind_fn)(&a.source)?,
                    target: (self.ind_fn)(&a.target)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::NegativeDataPropertyAssertion(a) => {
                Axiom::NegativeDataPropertyAssertion(NegativeDataPropertyAssertionAxiom {
                    property: (self.dpe_fn)(&a.property)?,
                    individual: (self.ind_fn)(&a.individual)?,
                    value: a.value.clone(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::AnnotationAssertion(a) => Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                subject: a.subject.clone(),
                property: a.property.clone(),
                value: a.value.clone(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::SubAnnotationPropertyOf(a) => {
                Axiom::SubAnnotationPropertyOf(SubAnnotationPropertyOfAxiom {
                    sub_property: a.sub_property.clone(),
                    super_property: a.super_property.clone(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::AnnotationPropertyDomain(a) => {
                Axiom::AnnotationPropertyDomain(AnnotationPropertyDomainAxiom {
                    property: a.property.clone(),
                    domain: (self.ce_fn)(&a.domain)?,
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::AnnotationPropertyRange(a) => {
                Axiom::AnnotationPropertyRange(AnnotationPropertyRangeAxiom {
                    property: a.property.clone(),
                    range: a.range.clone(),
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                    id: a.id,
                })
            }
            Axiom::HasKey(a) => Axiom::HasKey(HasKeyAxiom {
                class: (self.ce_fn)(&a.class)?,
                object_properties: a
                    .object_properties
                    .iter()
                    .filter_map(|p| (self.ope_fn)(p))
                    .collect(),
                data_properties: a
                    .data_properties
                    .iter()
                    .filter_map(|p| (self.dpe_fn)(p))
                    .collect(),
                annotations: a
                    .annotations
                    .iter()
                    .filter_map(|ann| self.transform_annotation(ann))
                    .collect(),
                id: a.id,
            }),
            Axiom::DatatypeDefinition(a) => Axiom::DatatypeDefinition(a.clone()),
            Axiom::Rule(a) => {
                let head: Vec<crate::ontology::axioms::SWRLAtom> = a
                    .rule
                    .head
                    .iter()
                    .filter_map(|atom| self.transform_swrl_atom(atom))
                    .collect();
                let body: Vec<crate::ontology::axioms::SWRLAtom> = a
                    .rule
                    .body
                    .iter()
                    .filter_map(|atom| self.transform_swrl_atom(atom))
                    .collect();
                Axiom::Rule(crate::ontology::axioms::SWRLRuleAxiom {
                    id: a.id,
                    rule: crate::ontology::axioms::SWRLRule { head, body },
                    annotations: a
                        .annotations
                        .iter()
                        .filter_map(|ann| self.transform_annotation(ann))
                        .collect(),
                })
            }
        })
    }

    fn transform_annotation(&self, ann: &Annotation) -> Option<Annotation> {
        let nested: Vec<Annotation> = ann
            .annotations
            .iter()
            .filter_map(|a| self.transform_annotation(a))
            .collect();
        Some(Annotation {
            property: ann.property.clone(),
            value: match &ann.value {
                AnnotationValue::IRI(iri) => AnnotationValue::IRI(iri.clone()),
                AnnotationValue::Literal(lit) => AnnotationValue::Literal(lit.clone()),
                AnnotationValue::AnonymousIndividual(a) => {
                    AnnotationValue::AnonymousIndividual(a.clone())
                }
            },
            annotations: nested,
        })
    }

    fn transform_swrl_atom(
        &self,
        atom: &crate::ontology::axioms::SWRLAtom,
    ) -> Option<crate::ontology::axioms::SWRLAtom> {
        use crate::ontology::axioms::SWRLAtom;
        Some(match atom {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => SWRLAtom::ClassAtom {
                predicate: (self.ce_fn)(predicate)?,
                argument: self
                    .transform_swrl_iarg(argument)
                    .unwrap_or_else(|| argument.clone()),
            },
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => SWRLAtom::ObjectPropertyAtom {
                predicate: (self.ope_fn)(predicate)?,
                first_argument: self
                    .transform_swrl_iarg(first_argument)
                    .unwrap_or_else(|| first_argument.clone()),
                second_argument: self
                    .transform_swrl_iarg(second_argument)
                    .unwrap_or_else(|| second_argument.clone()),
            },
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => SWRLAtom::DataPropertyAtom {
                predicate: (self.dpe_fn)(predicate)?,
                first_argument: self
                    .transform_swrl_iarg(first_argument)
                    .unwrap_or_else(|| first_argument.clone()),
                second_argument: second_argument.clone(),
            },
            SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => SWRLAtom::DataRangeAtom {
                predicate: predicate.clone(),
                argument: argument.clone(),
            },
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => SWRLAtom::SameIndividualAtom {
                first_argument: self
                    .transform_swrl_iarg(first_argument)
                    .unwrap_or_else(|| first_argument.clone()),
                second_argument: self
                    .transform_swrl_iarg(second_argument)
                    .unwrap_or_else(|| second_argument.clone()),
            },
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => SWRLAtom::DifferentIndividualsAtom {
                first_argument: self
                    .transform_swrl_iarg(first_argument)
                    .unwrap_or_else(|| first_argument.clone()),
                second_argument: self
                    .transform_swrl_iarg(second_argument)
                    .unwrap_or_else(|| second_argument.clone()),
            },
            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => SWRLAtom::BuiltInAtom {
                predicate: predicate.clone(),
                arguments: arguments.clone(),
            },
        })
    }

    fn transform_swrl_iarg(
        &self,
        arg: &crate::ontology::axioms::SWRLIArgument,
    ) -> Option<crate::ontology::axioms::SWRLIArgument> {
        use crate::ontology::axioms::SWRLIArgument;
        match arg {
            SWRLIArgument::Individual(ind) => (self.ind_fn)(ind).map(SWRLIArgument::Individual),
            SWRLIArgument::Variable(_) => None,
        }
    }
}

// ── OWLEntityRenamer ─────────────────────────────────────────────────────────

/// Renames entities in an ontology by IRI.
pub struct OWLEntityRenamer {
    mappings: HashMap<(IRI, EntityType), IRI>,
}

impl OWLEntityRenamer {
    #[must_use]
    pub fn new() -> Self {
        Self {
            mappings: HashMap::new(),
        }
    }

    /// Map an old IRI to a new IRI for a specific entity type (punning-aware).
    pub fn add_rename(&mut self, old_iri: IRI, new_iri: IRI, entity_type: EntityType) {
        self.mappings.insert((old_iri, entity_type), new_iri);
    }

    /// Look up the replacement IRI for an old IRI and entity type pair.
    fn lookup(&self, iri: &IRI, entity_type: EntityType) -> Option<&IRI> {
        self.mappings.get(&(iri.clone(), entity_type))
    }

    fn rename_in_class_expression(&self, ce: &ClassExpression) -> ClassExpression {
        match ce {
            ClassExpression::Class(cls) => {
                if let Some(new_iri) = self.lookup(&cls.iri, EntityType::Class) {
                    ClassExpression::Class(crate::ontology::Class {
                        iri: new_iri.clone(),
                    })
                } else {
                    ce.clone()
                }
            }
            ClassExpression::ObjectIntersectionOf(ops) => {
                ClassExpression::ObjectIntersectionOf(
                    ops.iter()
                        .map(|op| self.rename_in_class_expression(op))
                        .collect(),
                )
            }
            ClassExpression::ObjectUnionOf(ops) => ClassExpression::ObjectUnionOf(
                ops.iter()
                    .map(|op| self.rename_in_class_expression(op))
                    .collect(),
            ),
            ClassExpression::ObjectComplementOf(op) => {
                ClassExpression::ObjectComplementOf(Box::new(
                    self.rename_in_class_expression(op),
                ))
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                ClassExpression::ObjectSomeValuesFrom {
                    property: self.rename_in_ope(property),
                    filler: Box::new(self.rename_in_class_expression(filler)),
                }
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                ClassExpression::ObjectAllValuesFrom {
                    property: self.rename_in_ope(property),
                    filler: Box::new(self.rename_in_class_expression(filler)),
                }
            }
            ClassExpression::ObjectHasValue { property, value } => {
                ClassExpression::ObjectHasValue {
                    property: self.rename_in_ope(property),
                    value: self.rename_in_individual(value),
                }
            }
            ClassExpression::ObjectHasSelf { property } => ClassExpression::ObjectHasSelf {
                property: self.rename_in_ope(property),
            },
            ClassExpression::ObjectMinCardinality {
                cardinality,
                property,
                filler,
            } => ClassExpression::ObjectMinCardinality {
                cardinality: *cardinality,
                property: self.rename_in_ope(property),
                filler: Box::new(self.rename_in_class_expression(filler)),
            },
            ClassExpression::ObjectMaxCardinality {
                cardinality,
                property,
                filler,
            } => ClassExpression::ObjectMaxCardinality {
                cardinality: *cardinality,
                property: self.rename_in_ope(property),
                filler: Box::new(self.rename_in_class_expression(filler)),
            },
            ClassExpression::ObjectExactCardinality {
                cardinality,
                property,
                filler,
            } => ClassExpression::ObjectExactCardinality {
                cardinality: *cardinality,
                property: self.rename_in_ope(property),
                filler: Box::new(self.rename_in_class_expression(filler)),
            },
            ClassExpression::ObjectOneOf(inds) => ClassExpression::ObjectOneOf(
                inds.iter()
                    .map(|i| self.rename_in_individual(i))
                    .collect(),
            ),
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                ClassExpression::DataSomeValuesFrom {
                    property: self.rename_in_dpe(property),
                    filler: filler.clone(),
                }
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                ClassExpression::DataAllValuesFrom {
                    property: self.rename_in_dpe(property),
                    filler: filler.clone(),
                }
            }
            ClassExpression::DataHasValue { property, value } => {
                ClassExpression::DataHasValue {
                    property: self.rename_in_dpe(property),
                    value: value.clone(),
                }
            }
            ClassExpression::DataMinCardinality {
                cardinality,
                property,
                filler,
            } => ClassExpression::DataMinCardinality {
                cardinality: *cardinality,
                property: self.rename_in_dpe(property),
                filler: filler.clone(),
            },
            ClassExpression::DataMaxCardinality {
                cardinality,
                property,
                filler,
            } => ClassExpression::DataMaxCardinality {
                cardinality: *cardinality,
                property: self.rename_in_dpe(property),
                filler: filler.clone(),
            },
            ClassExpression::DataExactCardinality {
                cardinality,
                property,
                filler,
            } => ClassExpression::DataExactCardinality {
                cardinality: *cardinality,
                property: self.rename_in_dpe(property),
                filler: filler.clone(),
            },
        }
    }

    fn rename_in_ope(
        &self,
        ope: &ObjectPropertyExpression,
    ) -> ObjectPropertyExpression {
        match ope {
            ObjectPropertyExpression::ObjectProperty(p) => {
                if let Some(new_iri) = self.lookup(&p.iri, EntityType::ObjectProperty) {
                    ObjectPropertyExpression::ObjectProperty(crate::ontology::ObjectProperty {
                        iri: new_iri.clone(),
                    })
                } else {
                    ope.clone()
                }
            }
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                if let Some(new_iri) = self.lookup(&p.iri, EntityType::ObjectProperty) {
                    ObjectPropertyExpression::InverseObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: new_iri.clone(),
                        },
                    )
                } else {
                    ope.clone()
                }
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                ObjectPropertyExpression::PropertyChain(
                    chain.iter().map(|p| self.rename_in_ope(p)).collect(),
                )
            }
        }
    }

    fn rename_in_dpe(
        &self,
        dpe: &DataPropertyExpression,
    ) -> DataPropertyExpression {
        match dpe {
            DataPropertyExpression::DataProperty(p) => {
                if let Some(new_iri) = self.lookup(&p.iri, EntityType::DataProperty) {
                    DataPropertyExpression::DataProperty(crate::ontology::DataProperty {
                        iri: new_iri.clone(),
                    })
                } else {
                    dpe.clone()
                }
            }
        }
    }

    fn rename_in_individual(&self, ind: &Individual) -> Individual {
        match ind {
            Individual::Named(ni) => {
                if let Some(new_iri) = self.lookup(&ni.iri, EntityType::NamedIndividual) {
                    Individual::Named(crate::ontology::NamedIndividual {
                        iri: new_iri.clone(),
                    })
                } else {
                    ind.clone()
                }
            }
            Individual::Anonymous(_) => ind.clone(),
        }
    }

    fn rename_in_axiom(&self, axiom: &Axiom) -> Axiom {
        match axiom {
            Axiom::Declaration(d) => {
                let new_entity = match &d.entity {
                    Entity::Class(iri) => {
                        if let Some(new_iri) = self.lookup(iri, EntityType::Class) {
                            Entity::Class(new_iri.clone())
                        } else {
                            d.entity.clone()
                        }
                    }
                    Entity::ObjectProperty(iri) => {
                        if let Some(new_iri) = self.lookup(iri, EntityType::ObjectProperty) {
                            Entity::ObjectProperty(new_iri.clone())
                        } else {
                            d.entity.clone()
                        }
                    }
                    Entity::DataProperty(iri) => {
                        if let Some(new_iri) = self.lookup(iri, EntityType::DataProperty) {
                            Entity::DataProperty(new_iri.clone())
                        } else {
                            d.entity.clone()
                        }
                    }
                    Entity::AnnotationProperty(iri) => {
                        if let Some(new_iri) = self.lookup(iri, EntityType::AnnotationProperty) {
                            Entity::AnnotationProperty(new_iri.clone())
                        } else {
                            d.entity.clone()
                        }
                    }
                    Entity::NamedIndividual(iri) => {
                        if let Some(new_iri) = self.lookup(iri, EntityType::NamedIndividual) {
                            Entity::NamedIndividual(new_iri.clone())
                        } else {
                            d.entity.clone()
                        }
                    }
                    Entity::Datatype(iri) => {
                        if let Some(new_iri) = self.lookup(iri, EntityType::Datatype) {
                            Entity::Datatype(new_iri.clone())
                        } else {
                            d.entity.clone()
                        }
                    }
                };
                Axiom::Declaration(DeclarationAxiom {
                    id: d.id,
                    entity: new_entity,
                })
            }
            Axiom::SubClassOf(a) => Axiom::SubClassOf(SubClassOfAxiom {
                id: a.id,
                subclass: self.rename_in_class_expression(&a.subclass),
                superclass: self.rename_in_class_expression(&a.superclass),
                annotations: a.annotations.clone(),
            }),
            Axiom::EquivalentClasses(a) => Axiom::EquivalentClasses(EquivalentClassesAxiom {
                id: a.id,
                classes: a
                    .classes
                    .iter()
                    .map(|c| self.rename_in_class_expression(c))
                    .collect(),
                annotations: a.annotations.clone(),
            }),
            Axiom::DisjointClasses(a) => Axiom::DisjointClasses(DisjointClassesAxiom {
                id: a.id,
                classes: a
                    .classes
                    .iter()
                    .map(|c| self.rename_in_class_expression(c))
                    .collect(),
                annotations: a.annotations.clone(),
            }),
            Axiom::DisjointUnion(a) => Axiom::DisjointUnion(DisjointUnionAxiom {
                id: a.id,
                class: self.rename_in_class_expression(&a.class),
                disjoint_classes: a
                    .disjoint_classes
                    .iter()
                    .map(|c| self.rename_in_class_expression(c))
                    .collect(),
                annotations: a.annotations.clone(),
            }),
            Axiom::SubObjectPropertyOf(a) => {
                Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                    id: a.id,
                    sub_property: self.rename_in_ope(&a.sub_property),
                    super_property: self.rename_in_ope(&a.super_property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::EquivalentObjectProperties(a) => {
                Axiom::EquivalentObjectProperties(EquivalentObjectPropertiesAxiom {
                    id: a.id,
                    properties: a
                        .properties
                        .iter()
                        .map(|p| self.rename_in_ope(p))
                        .collect(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::DisjointObjectProperties(a) => {
                Axiom::DisjointObjectProperties(DisjointObjectPropertiesAxiom {
                    id: a.id,
                    properties: a
                        .properties
                        .iter()
                        .map(|p| self.rename_in_ope(p))
                        .collect(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::InverseObjectProperties(a) => {
                Axiom::InverseObjectProperties(InverseObjectPropertiesAxiom {
                    id: a.id,
                    property1: self.rename_in_ope(&a.property1),
                    property2: self.rename_in_ope(&a.property2),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::ObjectPropertyDomain(a) => {
                Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    domain: self.rename_in_class_expression(&a.domain),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::ObjectPropertyRange(a) => {
                Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    range: self.rename_in_class_expression(&a.range),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::FunctionalObjectProperty(a) => {
                Axiom::FunctionalObjectProperty(FunctionalObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::InverseFunctionalObjectProperty(a) => {
                Axiom::InverseFunctionalObjectProperty(InverseFunctionalObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::ReflexiveObjectProperty(a) => {
                Axiom::ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::IrreflexiveObjectProperty(a) => {
                Axiom::IrreflexiveObjectProperty(IrreflexiveObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::SymmetricObjectProperty(a) => {
                Axiom::SymmetricObjectProperty(SymmetricObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::AsymmetricObjectProperty(a) => {
                Axiom::AsymmetricObjectProperty(AsymmetricObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::TransitiveObjectProperty(a) => {
                Axiom::TransitiveObjectProperty(TransitiveObjectPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::SubDataPropertyOf(a) => Axiom::SubDataPropertyOf(SubDataPropertyOfAxiom {
                id: a.id,
                sub_property: self.rename_in_dpe(&a.sub_property),
                super_property: self.rename_in_dpe(&a.super_property),
                annotations: a.annotations.clone(),
            }),
            Axiom::EquivalentDataProperties(a) => {
                Axiom::EquivalentDataProperties(EquivalentDataPropertiesAxiom {
                    id: a.id,
                    properties: a
                        .properties
                        .iter()
                        .map(|p| self.rename_in_dpe(p))
                        .collect(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::DisjointDataProperties(a) => {
                Axiom::DisjointDataProperties(DisjointDataPropertiesAxiom {
                    id: a.id,
                    properties: a
                        .properties
                        .iter()
                        .map(|p| self.rename_in_dpe(p))
                        .collect(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::DataPropertyDomain(a) => {
                Axiom::DataPropertyDomain(DataPropertyDomainAxiom {
                    id: a.id,
                    property: self.rename_in_dpe(&a.property),
                    domain: self.rename_in_class_expression(&a.domain),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::DataPropertyRange(a) => Axiom::DataPropertyRange(DataPropertyRangeAxiom {
                id: a.id,
                property: self.rename_in_dpe(&a.property),
                range: a.range.clone(),
                annotations: a.annotations.clone(),
            }),
            Axiom::FunctionalDataProperty(a) => {
                Axiom::FunctionalDataProperty(FunctionalDataPropertyAxiom {
                    id: a.id,
                    property: self.rename_in_dpe(&a.property),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::SameIndividual(a) => Axiom::SameIndividual(SameIndividualAxiom {
                id: a.id,
                individuals: a
                    .individuals
                    .iter()
                    .map(|i| self.rename_in_individual(i))
                    .collect(),
                annotations: a.annotations.clone(),
            }),
            Axiom::DifferentIndividuals(a) => {
                Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
                    id: a.id,
                    individuals: a
                        .individuals
                        .iter()
                        .map(|i| self.rename_in_individual(i))
                        .collect(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::ClassAssertion(a) => Axiom::ClassAssertion(ClassAssertionAxiom {
                id: a.id,
                class: self.rename_in_class_expression(&a.class),
                individual: self.rename_in_individual(&a.individual),
                annotations: a.annotations.clone(),
            }),
            Axiom::ObjectPropertyAssertion(a) => {
                Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    source: self.rename_in_individual(&a.source),
                    target: self.rename_in_individual(&a.target),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::DataPropertyAssertion(a) => {
                Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                    id: a.id,
                    property: self.rename_in_dpe(&a.property),
                    individual: self.rename_in_individual(&a.individual),
                    value: a.value.clone(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::NegativeObjectPropertyAssertion(a) => {
                Axiom::NegativeObjectPropertyAssertion(NegativeObjectPropertyAssertionAxiom {
                    id: a.id,
                    property: self.rename_in_ope(&a.property),
                    source: self.rename_in_individual(&a.source),
                    target: self.rename_in_individual(&a.target),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::NegativeDataPropertyAssertion(a) => {
                Axiom::NegativeDataPropertyAssertion(NegativeDataPropertyAssertionAxiom {
                    id: a.id,
                    property: self.rename_in_dpe(&a.property),
                    individual: self.rename_in_individual(&a.individual),
                    value: a.value.clone(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::HasKey(a) => Axiom::HasKey(HasKeyAxiom {
                id: a.id,
                class: self.rename_in_class_expression(&a.class),
                object_properties: a
                    .object_properties
                    .iter()
                    .map(|p| self.rename_in_ope(p))
                    .collect(),
                data_properties: a
                    .data_properties
                    .iter()
                    .map(|p| self.rename_in_dpe(p))
                    .collect(),
                annotations: a.annotations.clone(),
            }),
            other => self.rename_in_annotation_axiom(other),
        }
    }

    fn rename_in_annotation_axiom(&self, axiom: &Axiom) -> Axiom {
        match axiom {
            Axiom::AnnotationAssertion(a) => {
                let subject = match &a.subject {
                    crate::ontology::AnnotationSubject::IRI(iri) => {
                        crate::ontology::AnnotationSubject::IRI(
                            self.lookup(iri, EntityType::NamedIndividual)
                                .or_else(|| self.lookup(iri, EntityType::Class))
                                .cloned()
                                .unwrap_or_else(|| iri.clone()),
                        )
                    }
                    other => other.clone(),
                };
                let property = if let Some(new_iri) =
                    self.lookup(&a.property.iri, EntityType::AnnotationProperty)
                {
                    crate::ontology::AnnotationProperty {
                        iri: new_iri.clone(),
                    }
                } else {
                    a.property.clone()
                };
                Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                    id: a.id,
                    subject,
                    property,
                    value: a.value.clone(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::SubAnnotationPropertyOf(a) => {
                Axiom::SubAnnotationPropertyOf(SubAnnotationPropertyOfAxiom {
                    id: a.id,
                    sub_property: if let Some(new_iri) =
                        self.lookup(&a.sub_property.iri, EntityType::AnnotationProperty)
                    {
                        crate::ontology::AnnotationProperty {
                            iri: new_iri.clone(),
                        }
                    } else {
                        a.sub_property.clone()
                    },
                    super_property: if let Some(new_iri) =
                        self.lookup(&a.super_property.iri, EntityType::AnnotationProperty)
                    {
                        crate::ontology::AnnotationProperty {
                            iri: new_iri.clone(),
                        }
                    } else {
                        a.super_property.clone()
                    },
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::AnnotationPropertyDomain(a) => {
                Axiom::AnnotationPropertyDomain(AnnotationPropertyDomainAxiom {
                    id: a.id,
                    property: if let Some(new_iri) =
                        self.lookup(&a.property.iri, EntityType::AnnotationProperty)
                    {
                        crate::ontology::AnnotationProperty {
                            iri: new_iri.clone(),
                        }
                    } else {
                        a.property.clone()
                    },
                    domain: self.rename_in_class_expression(&a.domain),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::AnnotationPropertyRange(a) => {
                Axiom::AnnotationPropertyRange(AnnotationPropertyRangeAxiom {
                    id: a.id,
                    property: if let Some(new_iri) =
                        self.lookup(&a.property.iri, EntityType::AnnotationProperty)
                    {
                        crate::ontology::AnnotationProperty {
                            iri: new_iri.clone(),
                        }
                    } else {
                        a.property.clone()
                    },
                    range: a.range.clone(),
                    annotations: a.annotations.clone(),
                })
            }
            Axiom::DatatypeDefinition(a) => Axiom::DatatypeDefinition(a.clone()),
            Axiom::Rule(a) => Axiom::Rule(a.clone()),
            other => other.clone(),
        }
    }

    // ── Single-IRI rename helpers ─────────────────────────────────────

    fn rename_ce(ce: &ClassExpression, from: &IRI, to: &IRI) -> Option<ClassExpression> {
        match ce {
            ClassExpression::Class(c) => {
                if c.iri == *from {
                    Some(ClassExpression::Class(crate::ontology::Class { iri: to.clone() }))
                } else {
                    None
                }
            }
            ClassExpression::ObjectIntersectionOf(v) => {
                let mut changed = false;
                let r: Vec<_> = v
                    .iter()
                    .map(|x| {
                        if let Some(n) = Self::rename_ce(x, from, to) {
                            changed = true;
                            n
                        } else {
                            x.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(ClassExpression::ObjectIntersectionOf(r))
                } else {
                    None
                }
            }
            ClassExpression::ObjectUnionOf(v) => {
                let mut changed = false;
                let r: Vec<_> = v
                    .iter()
                    .map(|x| {
                        if let Some(n) = Self::rename_ce(x, from, to) {
                            changed = true;
                            n
                        } else {
                            x.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(ClassExpression::ObjectUnionOf(r))
                } else {
                    None
                }
            }
            ClassExpression::ObjectComplementOf(b) => {
                Self::rename_ce(b, from, to)
                    .map(|n| ClassExpression::ObjectComplementOf(Box::new(n)))
            }
            ClassExpression::ObjectOneOf(v) => {
                let mut changed = false;
                let r: Vec<_> = v
                    .iter()
                    .map(|x| {
                        if let Some(n) = Self::rename_individual(x, from, to) {
                            changed = true;
                            n
                        } else {
                            x.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(ClassExpression::ObjectOneOf(r))
                } else {
                    None
                }
            }
            ClassExpression::ObjectSomeValuesFrom { property, filler } => {
                let p = Self::rename_ope(property, from, to);
                let f = Self::rename_ce(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::ObjectSomeValuesFrom {
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: Box::new(f.unwrap_or_else(|| *filler.clone())),
                    })
                } else {
                    None
                }
            }
            ClassExpression::ObjectAllValuesFrom { property, filler } => {
                let p = Self::rename_ope(property, from, to);
                let f = Self::rename_ce(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::ObjectAllValuesFrom {
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: Box::new(f.unwrap_or_else(|| *filler.clone())),
                    })
                } else {
                    None
                }
            }
            ClassExpression::ObjectHasValue { property, value } => {
                let p = Self::rename_ope(property, from, to);
                let v = Self::rename_individual(value, from, to);
                if p.is_some() || v.is_some() {
                    Some(ClassExpression::ObjectHasValue {
                        property: p.unwrap_or_else(|| property.clone()),
                        value: v.unwrap_or_else(|| value.clone()),
                    })
                } else {
                    None
                }
            }
            ClassExpression::ObjectHasSelf { property } => {
                Self::rename_ope(property, from, to)
                    .map(|p| ClassExpression::ObjectHasSelf { property: p })
            }
            ClassExpression::ObjectMinCardinality {
                cardinality,
                property,
                filler,
            } => {
                let p = Self::rename_ope(property, from, to);
                let f = Self::rename_ce(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::ObjectMinCardinality {
                        cardinality: *cardinality,
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: Box::new(f.unwrap_or_else(|| *filler.clone())),
                    })
                } else {
                    None
                }
            }
            ClassExpression::ObjectMaxCardinality {
                cardinality,
                property,
                filler,
            } => {
                let p = Self::rename_ope(property, from, to);
                let f = Self::rename_ce(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::ObjectMaxCardinality {
                        cardinality: *cardinality,
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: Box::new(f.unwrap_or_else(|| *filler.clone())),
                    })
                } else {
                    None
                }
            }
            ClassExpression::ObjectExactCardinality {
                cardinality,
                property,
                filler,
            } => {
                let p = Self::rename_ope(property, from, to);
                let f = Self::rename_ce(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::ObjectExactCardinality {
                        cardinality: *cardinality,
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: Box::new(f.unwrap_or_else(|| *filler.clone())),
                    })
                } else {
                    None
                }
            }
            ClassExpression::DataSomeValuesFrom { property, filler } => {
                let p = Self::rename_dpe(property, from, to);
                let f = Self::rename_dr(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::DataSomeValuesFrom {
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: f.unwrap_or_else(|| filler.clone()),
                    })
                } else {
                    None
                }
            }
            ClassExpression::DataAllValuesFrom { property, filler } => {
                let p = Self::rename_dpe(property, from, to);
                let f = Self::rename_dr(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::DataAllValuesFrom {
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: f.unwrap_or_else(|| filler.clone()),
                    })
                } else {
                    None
                }
            }
            ClassExpression::DataHasValue { property, value } => {
                Self::rename_dpe(property, from, to)
                    .map(|p| ClassExpression::DataHasValue {
                        property: p,
                        value: value.clone(),
                    })
            }
            ClassExpression::DataMinCardinality {
                cardinality,
                property,
                filler,
            } => {
                let p = Self::rename_dpe(property, from, to);
                let f = Self::rename_dr(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::DataMinCardinality {
                        cardinality: *cardinality,
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: f.unwrap_or_else(|| filler.clone()),
                    })
                } else {
                    None
                }
            }
            ClassExpression::DataMaxCardinality {
                cardinality,
                property,
                filler,
            } => {
                let p = Self::rename_dpe(property, from, to);
                let f = Self::rename_dr(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::DataMaxCardinality {
                        cardinality: *cardinality,
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: f.unwrap_or_else(|| filler.clone()),
                    })
                } else {
                    None
                }
            }
            ClassExpression::DataExactCardinality {
                cardinality,
                property,
                filler,
            } => {
                let p = Self::rename_dpe(property, from, to);
                let f = Self::rename_dr(filler, from, to);
                if p.is_some() || f.is_some() {
                    Some(ClassExpression::DataExactCardinality {
                        cardinality: *cardinality,
                        property: p.unwrap_or_else(|| property.clone()),
                        filler: f.unwrap_or_else(|| filler.clone()),
                    })
                } else {
                    None
                }
            }
        }
    }

    fn rename_ope(
        ope: &ObjectPropertyExpression,
        from: &IRI,
        to: &IRI,
    ) -> Option<ObjectPropertyExpression> {
        match ope {
            ObjectPropertyExpression::ObjectProperty(p) => {
                if p.iri == *from {
                    Some(ObjectPropertyExpression::ObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: to.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            ObjectPropertyExpression::InverseObjectProperty(p) => {
                if p.iri == *from {
                    Some(ObjectPropertyExpression::InverseObjectProperty(
                        crate::ontology::ObjectProperty {
                            iri: to.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            ObjectPropertyExpression::PropertyChain(chain) => {
                let mut changed = false;
                let r: Vec<_> = chain
                    .iter()
                    .map(|x| {
                        if let Some(n) = Self::rename_ope(x, from, to) {
                            changed = true;
                            n
                        } else {
                            x.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(ObjectPropertyExpression::PropertyChain(r))
                } else {
                    None
                }
            }
        }
    }

    fn rename_dpe(
        dpe: &DataPropertyExpression,
        from: &IRI,
        to: &IRI,
    ) -> Option<DataPropertyExpression> {
        match dpe {
            DataPropertyExpression::DataProperty(p) => {
                if p.iri == *from {
                    Some(DataPropertyExpression::DataProperty(
                        crate::ontology::DataProperty {
                            iri: to.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
        }
    }

    fn rename_individual(ind: &Individual, from: &IRI, to: &IRI) -> Option<Individual> {
        match ind {
            Individual::Named(ni) => {
                if ni.iri == *from {
                    Some(Individual::Named(crate::ontology::NamedIndividual {
                        iri: to.clone(),
                    }))
                } else {
                    None
                }
            }
            Individual::Anonymous(_) => None,
        }
    }

    fn rename_dr(dr: &DataRange, from: &IRI, to: &IRI) -> Option<DataRange> {
        match dr {
            DataRange::Datatype(dt) => {
                if dt == from {
                    Some(DataRange::Datatype(to.clone()))
                } else {
                    None
                }
            }
            DataRange::DatatypeRestriction {
                datatype,
                restrictions,
            } => {
                if datatype == from {
                    Some(DataRange::DatatypeRestriction {
                        datatype: to.clone(),
                        restrictions: restrictions.clone(),
                    })
                } else {
                    None
                }
            }
            DataRange::DataIntersectionOf(ranges) => {
                let mut changed = false;
                let r: Vec<_> = ranges
                    .iter()
                    .map(|x| {
                        if let Some(n) = Self::rename_dr(x, from, to) {
                            changed = true;
                            n
                        } else {
                            x.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(DataRange::DataIntersectionOf(r))
                } else {
                    None
                }
            }
            DataRange::DataUnionOf(ranges) => {
                let mut changed = false;
                let r: Vec<_> = ranges
                    .iter()
                    .map(|x| {
                        if let Some(n) = Self::rename_dr(x, from, to) {
                            changed = true;
                            n
                        } else {
                            x.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(DataRange::DataUnionOf(r))
                } else {
                    None
                }
            }
            DataRange::DataComplementOf(b) => {
                Self::rename_dr(b, from, to)
                    .map(|n| DataRange::DataComplementOf(Box::new(n)))
            }
            DataRange::DataOneOf(_) => None,
        }
    }

    fn rename_annotation_value(
        val: &AnnotationValue,
        from: &IRI,
        to: &IRI,
    ) -> Option<AnnotationValue> {
        match val {
            AnnotationValue::IRI(iri) if iri == from => {
                Some(AnnotationValue::IRI(to.clone()))
            }
            _ => None,
        }
    }

    fn rename_annotation_subject(
        subj: &AnnotationSubject,
        from: &IRI,
        to: &IRI,
    ) -> Option<AnnotationSubject> {
        match subj {
            AnnotationSubject::IRI(iri) if iri == from => {
                Some(AnnotationSubject::IRI(to.clone()))
            }
            _ => None,
        }
    }

    fn rename_annotation_property(
        ap: &AnnotationProperty,
        from: &IRI,
        to: &IRI,
    ) -> Option<AnnotationProperty> {
        if ap.iri == *from {
            Some(crate::ontology::AnnotationProperty {
                iri: to.clone(),
            })
        } else {
            None
        }
    }

    fn rename_swrl_atom(
        atom: &crate::ontology::axioms::SWRLAtom,
        from: &IRI,
        to: &IRI,
    ) -> Option<crate::ontology::axioms::SWRLAtom> {
        use crate::ontology::axioms::SWRLAtom;
        match atom {
            SWRLAtom::ClassAtom {
                predicate,
                argument,
            } => {
                let p = Self::rename_ce(predicate, from, to);
                let a = Self::rename_swrl_iarg(argument, from, to);
                if p.is_some() || a.is_some() {
                    Some(SWRLAtom::ClassAtom {
                        predicate: p.unwrap_or_else(|| predicate.clone()),
                        argument: a.unwrap_or_else(|| argument.clone()),
                    })
                } else {
                    None
                }
            }
            SWRLAtom::ObjectPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                let p = Self::rename_ope(predicate, from, to);
                let a1 = Self::rename_swrl_iarg(first_argument, from, to);
                let a2 = Self::rename_swrl_iarg(second_argument, from, to);
                if p.is_some() || a1.is_some() || a2.is_some() {
                    Some(SWRLAtom::ObjectPropertyAtom {
                        predicate: p.unwrap_or_else(|| predicate.clone()),
                        first_argument: a1.unwrap_or_else(|| first_argument.clone()),
                        second_argument: a2.unwrap_or_else(|| second_argument.clone()),
                    })
                } else {
                    None
                }
            }
            SWRLAtom::DataPropertyAtom {
                predicate,
                first_argument,
                second_argument,
            } => {
                let p = Self::rename_dpe(predicate, from, to);
                let a1 = Self::rename_swrl_iarg(first_argument, from, to);
                if p.is_some() || a1.is_some() {
                    Some(SWRLAtom::DataPropertyAtom {
                        predicate: p.unwrap_or_else(|| predicate.clone()),
                        first_argument: a1.unwrap_or_else(|| first_argument.clone()),
                        second_argument: second_argument.clone(),
                    })
                } else {
                    None
                }
            }
            SWRLAtom::DataRangeAtom {
                predicate,
                argument,
            } => {
                let p = Self::rename_dr(predicate, from, to);
                if p.is_some() {
                    Some(SWRLAtom::DataRangeAtom {
                        predicate: p.unwrap(),
                        argument: argument.clone(),
                    })
                } else {
                    None
                }
            }
            SWRLAtom::SameIndividualAtom {
                first_argument,
                second_argument,
            } => {
                let a1 = Self::rename_swrl_iarg(first_argument, from, to);
                let a2 = Self::rename_swrl_iarg(second_argument, from, to);
                if a1.is_some() || a2.is_some() {
                    Some(SWRLAtom::SameIndividualAtom {
                        first_argument: a1.unwrap_or_else(|| first_argument.clone()),
                        second_argument: a2.unwrap_or_else(|| second_argument.clone()),
                    })
                } else {
                    None
                }
            }
            SWRLAtom::DifferentIndividualsAtom {
                first_argument,
                second_argument,
            } => {
                let a1 = Self::rename_swrl_iarg(first_argument, from, to);
                let a2 = Self::rename_swrl_iarg(second_argument, from, to);
                if a1.is_some() || a2.is_some() {
                    Some(SWRLAtom::DifferentIndividualsAtom {
                        first_argument: a1.unwrap_or_else(|| first_argument.clone()),
                        second_argument: a2.unwrap_or_else(|| second_argument.clone()),
                    })
                } else {
                    None
                }
            }
            SWRLAtom::BuiltInAtom {
                predicate,
                arguments,
            } => {
                if *predicate == *from {
                    Some(SWRLAtom::BuiltInAtom {
                        predicate: to.clone(),
                        arguments: arguments.clone(),
                    })
                } else {
                    None
                }
            }
        }
    }

    fn rename_swrl_atoms(
        atoms: &[crate::ontology::axioms::SWRLAtom],
        from: &IRI,
        to: &IRI,
    ) -> Option<Vec<crate::ontology::axioms::SWRLAtom>> {
        let mut changed = false;
        let r: Vec<_> = atoms
            .iter()
            .map(|a| {
                if let Some(n) = Self::rename_swrl_atom(a, from, to) {
                    changed = true;
                    n
                } else {
                    a.clone()
                }
            })
            .collect();
        if changed {
            Some(r)
        } else {
            None
        }
    }

    fn rename_swrl_iarg(
        arg: &crate::ontology::axioms::SWRLIArgument,
        from: &IRI,
        to: &IRI,
    ) -> Option<crate::ontology::axioms::SWRLIArgument> {
        use crate::ontology::axioms::SWRLIArgument;
        match arg {
            SWRLIArgument::Individual(ind) => Self::rename_individual(ind, from, to)
                .map(SWRLIArgument::Individual),
            SWRLIArgument::Variable(_) => None,
        }
    }

    fn rename_axiom_one(axiom: &Axiom, from: &IRI, to: &IRI) -> Option<Axiom> {
        match axiom {
            Axiom::Declaration(d) => {
                let entity = match &d.entity {
                    Entity::Class(iri) if iri == from => Entity::Class(to.clone()),
                    Entity::ObjectProperty(iri) if iri == from => Entity::ObjectProperty(to.clone()),
                    Entity::DataProperty(iri) if iri == from => Entity::DataProperty(to.clone()),
                    Entity::AnnotationProperty(iri) if iri == from => {
                        Entity::AnnotationProperty(to.clone())
                    }
                    Entity::NamedIndividual(iri) if iri == from => {
                        Entity::NamedIndividual(to.clone())
                    }
                    Entity::Datatype(iri) if iri == from => Entity::Datatype(to.clone()),
                    _ => return None,
                };
                Some(Axiom::Declaration(DeclarationAxiom {
                    id: d.id,
                    entity,
                }))
            }
            Axiom::SubClassOf(a) => {
                let s = Self::rename_ce(&a.subclass, from, to);
                let u = Self::rename_ce(&a.superclass, from, to);
                if s.is_some() || u.is_some() {
                    Some(Axiom::SubClassOf(SubClassOfAxiom {
                        id: a.id,
                        subclass: s.unwrap_or_else(|| a.subclass.clone()),
                        superclass: u.unwrap_or_else(|| a.superclass.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::EquivalentClasses(a) => {
                let mut changed = false;
                let classes: Vec<_> = a
                    .classes
                    .iter()
                    .map(|c| {
                        if let Some(n) = Self::rename_ce(c, from, to) {
                            changed = true;
                            n
                        } else {
                            c.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                        id: a.id,
                        classes,
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::DisjointClasses(a) => {
                let mut changed = false;
                let classes: Vec<_> = a
                    .classes
                    .iter()
                    .map(|c| {
                        if let Some(n) = Self::rename_ce(c, from, to) {
                            changed = true;
                            n
                        } else {
                            c.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::DisjointClasses(DisjointClassesAxiom {
                        id: a.id,
                        classes,
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::DisjointUnion(a) => {
                let c = Self::rename_ce(&a.class, from, to);
                let mut changed = c.is_some();
                let disjoints: Vec<_> = a
                    .disjoint_classes
                    .iter()
                    .map(|d| {
                        if let Some(n) = Self::rename_ce(d, from, to) {
                            changed = true;
                            n
                        } else {
                            d.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::DisjointUnion(DisjointUnionAxiom {
                        id: a.id,
                        class: c.unwrap_or_else(|| a.class.clone()),
                        disjoint_classes: disjoints,
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::ClassAssertion(a) => {
                let c = Self::rename_ce(&a.class, from, to);
                let i = Self::rename_individual(&a.individual, from, to);
                if c.is_some() || i.is_some() {
                    Some(Axiom::ClassAssertion(ClassAssertionAxiom {
                        id: a.id,
                        class: c.unwrap_or_else(|| a.class.clone()),
                        individual: i.unwrap_or_else(|| a.individual.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::SubObjectPropertyOf(a) => {
                let s = Self::rename_ope(&a.sub_property, from, to);
                let u = Self::rename_ope(&a.super_property, from, to);
                if s.is_some() || u.is_some() {
                    Some(Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                        id: a.id,
                        sub_property: s.unwrap_or_else(|| a.sub_property.clone()),
                        super_property: u.unwrap_or_else(|| a.super_property.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::EquivalentObjectProperties(a) => {
                let mut changed = false;
                let props: Vec<_> = a
                    .properties
                    .iter()
                    .map(|p| {
                        if let Some(n) = Self::rename_ope(p, from, to) {
                            changed = true;
                            n
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::EquivalentObjectProperties(
                        EquivalentObjectPropertiesAxiom {
                            id: a.id,
                            properties: props,
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::DisjointObjectProperties(a) => {
                let mut changed = false;
                let props: Vec<_> = a
                    .properties
                    .iter()
                    .map(|p| {
                        if let Some(n) = Self::rename_ope(p, from, to) {
                            changed = true;
                            n
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::DisjointObjectProperties(
                        DisjointObjectPropertiesAxiom {
                            id: a.id,
                            properties: props,
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::InverseObjectProperties(a) => {
                let p1 = Self::rename_ope(&a.property1, from, to);
                let p2 = Self::rename_ope(&a.property2, from, to);
                if p1.is_some() || p2.is_some() {
                    Some(Axiom::InverseObjectProperties(
                        InverseObjectPropertiesAxiom {
                            id: a.id,
                            property1: p1.unwrap_or_else(|| a.property1.clone()),
                            property2: p2.unwrap_or_else(|| a.property2.clone()),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::ObjectPropertyDomain(a) => {
                let p = Self::rename_ope(&a.property, from, to);
                let d = Self::rename_ce(&a.domain, from, to);
                if p.is_some() || d.is_some() {
                    Some(Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
                        id: a.id,
                        property: p.unwrap_or_else(|| a.property.clone()),
                        domain: d.unwrap_or_else(|| a.domain.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::ObjectPropertyRange(a) => {
                let p = Self::rename_ope(&a.property, from, to);
                let r = Self::rename_ce(&a.range, from, to);
                if p.is_some() || r.is_some() {
                    Some(Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
                        id: a.id,
                        property: p.unwrap_or_else(|| a.property.clone()),
                        range: r.unwrap_or_else(|| a.range.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::FunctionalObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::FunctionalObjectProperty(FunctionalObjectPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::InverseFunctionalObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::InverseFunctionalObjectProperty(
                        InverseFunctionalObjectPropertyAxiom {
                            id: a.id,
                            property: p,
                            annotations: a.annotations.clone(),
                        },
                    )
                })
            }
            Axiom::ReflexiveObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::IrreflexiveObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::IrreflexiveObjectProperty(IrreflexiveObjectPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::SymmetricObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::SymmetricObjectProperty(SymmetricObjectPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::AsymmetricObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::AsymmetricObjectProperty(AsymmetricObjectPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::TransitiveObjectProperty(a) => {
                Self::rename_ope(&a.property, from, to).map(|p| {
                    Axiom::TransitiveObjectProperty(TransitiveObjectPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::SubDataPropertyOf(a) => {
                let s = Self::rename_dpe(&a.sub_property, from, to);
                let u = Self::rename_dpe(&a.super_property, from, to);
                if s.is_some() || u.is_some() {
                    Some(Axiom::SubDataPropertyOf(SubDataPropertyOfAxiom {
                        id: a.id,
                        sub_property: s.unwrap_or_else(|| a.sub_property.clone()),
                        super_property: u.unwrap_or_else(|| a.super_property.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::EquivalentDataProperties(a) => {
                let mut changed = false;
                let props: Vec<_> = a
                    .properties
                    .iter()
                    .map(|p| {
                        if let Some(n) = Self::rename_dpe(p, from, to) {
                            changed = true;
                            n
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::EquivalentDataProperties(
                        EquivalentDataPropertiesAxiom {
                            id: a.id,
                            properties: props,
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::DisjointDataProperties(a) => {
                let mut changed = false;
                let props: Vec<_> = a
                    .properties
                    .iter()
                    .map(|p| {
                        if let Some(n) = Self::rename_dpe(p, from, to) {
                            changed = true;
                            n
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::DisjointDataProperties(
                        DisjointDataPropertiesAxiom {
                            id: a.id,
                            properties: props,
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::DataPropertyDomain(a) => {
                let p = Self::rename_dpe(&a.property, from, to);
                let d = Self::rename_ce(&a.domain, from, to);
                if p.is_some() || d.is_some() {
                    Some(Axiom::DataPropertyDomain(DataPropertyDomainAxiom {
                        id: a.id,
                        property: p.unwrap_or_else(|| a.property.clone()),
                        domain: d.unwrap_or_else(|| a.domain.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::DataPropertyRange(a) => {
                let p = Self::rename_dpe(&a.property, from, to);
                let r = Self::rename_dr(&a.range, from, to);
                if p.is_some() || r.is_some() {
                    Some(Axiom::DataPropertyRange(DataPropertyRangeAxiom {
                        id: a.id,
                        property: p.unwrap_or_else(|| a.property.clone()),
                        range: r.unwrap_or_else(|| a.range.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::FunctionalDataProperty(a) => {
                Self::rename_dpe(&a.property, from, to).map(|p| {
                    Axiom::FunctionalDataProperty(FunctionalDataPropertyAxiom {
                        id: a.id,
                        property: p,
                        annotations: a.annotations.clone(),
                    })
                })
            }
            Axiom::ObjectPropertyAssertion(a) => {
                let p = Self::rename_ope(&a.property, from, to);
                let s = Self::rename_individual(&a.source, from, to);
                let t = Self::rename_individual(&a.target, from, to);
                if p.is_some() || s.is_some() || t.is_some() {
                    Some(Axiom::ObjectPropertyAssertion(
                        ObjectPropertyAssertionAxiom {
                            id: a.id,
                            property: p.unwrap_or_else(|| a.property.clone()),
                            source: s.unwrap_or_else(|| a.source.clone()),
                            target: t.unwrap_or_else(|| a.target.clone()),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::DataPropertyAssertion(a) => {
                let p = Self::rename_dpe(&a.property, from, to);
                let i = Self::rename_individual(&a.individual, from, to);
                if p.is_some() || i.is_some() {
                    Some(Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
                        id: a.id,
                        property: p.unwrap_or_else(|| a.property.clone()),
                        individual: i.unwrap_or_else(|| a.individual.clone()),
                        value: a.value.clone(),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::NegativeObjectPropertyAssertion(a) => {
                let p = Self::rename_ope(&a.property, from, to);
                let s = Self::rename_individual(&a.source, from, to);
                let t = Self::rename_individual(&a.target, from, to);
                if p.is_some() || s.is_some() || t.is_some() {
                    Some(Axiom::NegativeObjectPropertyAssertion(
                        NegativeObjectPropertyAssertionAxiom {
                            id: a.id,
                            property: p.unwrap_or_else(|| a.property.clone()),
                            source: s.unwrap_or_else(|| a.source.clone()),
                            target: t.unwrap_or_else(|| a.target.clone()),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::NegativeDataPropertyAssertion(a) => {
                let p = Self::rename_dpe(&a.property, from, to);
                let i = Self::rename_individual(&a.individual, from, to);
                if p.is_some() || i.is_some() {
                    Some(Axiom::NegativeDataPropertyAssertion(
                        NegativeDataPropertyAssertionAxiom {
                            id: a.id,
                            property: p.unwrap_or_else(|| a.property.clone()),
                            individual: i.unwrap_or_else(|| a.individual.clone()),
                            value: a.value.clone(),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::SameIndividual(a) => {
                let mut changed = false;
                let inds: Vec<_> = a
                    .individuals
                    .iter()
                    .map(|i| {
                        if let Some(n) = Self::rename_individual(i, from, to) {
                            changed = true;
                            n
                        } else {
                            i.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::SameIndividual(SameIndividualAxiom {
                        id: a.id,
                        individuals: inds,
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::DifferentIndividuals(a) => {
                let mut changed = false;
                let inds: Vec<_> = a
                    .individuals
                    .iter()
                    .map(|i| {
                        if let Some(n) = Self::rename_individual(i, from, to) {
                            changed = true;
                            n
                        } else {
                            i.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
                        id: a.id,
                        individuals: inds,
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::AnnotationAssertion(a) => {
                let s = Self::rename_annotation_subject(&a.subject, from, to);
                let p = Self::rename_annotation_property(&a.property, from, to);
                let v = Self::rename_annotation_value(&a.value, from, to);
                if s.is_some() || p.is_some() || v.is_some() {
                    Some(Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                        id: a.id,
                        subject: s.unwrap_or_else(|| a.subject.clone()),
                        property: p.unwrap_or_else(|| a.property.clone()),
                        value: v.unwrap_or_else(|| a.value.clone()),
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::SubAnnotationPropertyOf(a) => {
                let s = Self::rename_annotation_property(&a.sub_property, from, to);
                let u = Self::rename_annotation_property(&a.super_property, from, to);
                if s.is_some() || u.is_some() {
                    Some(Axiom::SubAnnotationPropertyOf(
                        SubAnnotationPropertyOfAxiom {
                            id: a.id,
                            sub_property: s.unwrap_or_else(|| a.sub_property.clone()),
                            super_property: u.unwrap_or_else(|| a.super_property.clone()),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::AnnotationPropertyDomain(a) => {
                let p = Self::rename_annotation_property(&a.property, from, to);
                let d = Self::rename_ce(&a.domain, from, to);
                if p.is_some() || d.is_some() {
                    Some(Axiom::AnnotationPropertyDomain(
                        AnnotationPropertyDomainAxiom {
                            id: a.id,
                            property: p.unwrap_or_else(|| a.property.clone()),
                            domain: d.unwrap_or_else(|| a.domain.clone()),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::AnnotationPropertyRange(a) => {
                let p = Self::rename_annotation_property(&a.property, from, to);
                let r = Self::rename_dr(&a.range, from, to);
                if p.is_some() || r.is_some() {
                    Some(Axiom::AnnotationPropertyRange(
                        AnnotationPropertyRangeAxiom {
                            id: a.id,
                            property: p.unwrap_or_else(|| a.property.clone()),
                            range: r.unwrap_or_else(|| a.range.clone()),
                            annotations: a.annotations.clone(),
                        },
                    ))
                } else {
                    None
                }
            }
            Axiom::HasKey(a) => {
                let c = Self::rename_ce(&a.class, from, to);
                let mut changed = c.is_some();
                let obj_props: Vec<_> = a
                    .object_properties
                    .iter()
                    .map(|p| {
                        if let Some(n) = Self::rename_ope(p, from, to) {
                            changed = true;
                            n
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                let data_props: Vec<_> = a
                    .data_properties
                    .iter()
                    .map(|p| {
                        if let Some(n) = Self::rename_dpe(p, from, to) {
                            changed = true;
                            n
                        } else {
                            p.clone()
                        }
                    })
                    .collect();
                if changed {
                    Some(Axiom::HasKey(HasKeyAxiom {
                        id: a.id,
                        class: c.unwrap_or_else(|| a.class.clone()),
                        object_properties: obj_props,
                        data_properties: data_props,
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
            Axiom::DatatypeDefinition(_a) => {
                None
            }
            Axiom::Rule(a) => {
                let head = Self::rename_swrl_atoms(&a.rule.head, from, to);
                let body = Self::rename_swrl_atoms(&a.rule.body, from, to);
                if head.is_some() || body.is_some() {
                    Some(Axiom::Rule(crate::ontology::axioms::SWRLRuleAxiom {
                        id: a.id,
                        rule: crate::ontology::axioms::SWRLRule {
                            head: head.unwrap_or_else(|| a.rule.head.clone()),
                            body: body.unwrap_or_else(|| a.rule.body.clone()),
                        },
                        annotations: a.annotations.clone(),
                    }))
                } else {
                    None
                }
            }
        }
    }

    /// Rename a single IRI in an axiom. Returns None if no replacement was needed.
    #[must_use]
    pub fn rename_iri_in_axiom(axiom: &Axiom, from: &IRI, to: &IRI) -> Option<Axiom> {
        Self::rename_axiom_one(axiom, from, to)
    }

    /// Generate change operations that rename all matching entities.
    pub fn rename_ontology(&self, ontology: &OntologyRef) -> Result<Vec<OntologyChange>> {
        let guard = ontology.read().map_err(|e| crate::Error::Internal {
            message: format!("{e}"),
        })?;
        let index = EntityIndex::from_ontology(&guard);
        let ontology_iri = guard
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:anon"));
        drop(guard);

        let mut changes = Vec::new();
        let mut seen_ids = HashSet::new();
        for (old_iri, _et) in self.mappings.keys() {
            for id in index.ids_for_entity(old_iri) {
                if !seen_ids.insert(id) {
                    continue;
                }
                if let Some(ax) = index.get_axiom(id) {
                    let original = (**ax).clone();
                    let renamed = self.rename_in_axiom(&original);
                    changes.push(OntologyChange::RemoveAxiom {
                        ontology_iri: ontology_iri.clone(),
                        axiom: original,
                    });
                    changes.push(OntologyChange::AddAxiom {
                        ontology_iri: ontology_iri.clone(),
                        axiom: renamed,
                    });
                }
            }
        }
        Ok(changes)
    }
}

impl Default for OWLEntityRenamer {
    fn default() -> Self {
        Self::new()
    }
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
        Self {
            entities_to_remove: HashSet::new(),
            remove_declarations: true,
        }
    }

    /// Add an entity to be removed.
    pub fn add_entity(&mut self, iri: IRI, entity_type: EntityType) {
        self.entities_to_remove.insert((iri, entity_type));
    }

    /// Generate RemoveAxiom changes for all axioms mentioning target entities.
    pub fn remove_entities(&self, ontology: &OntologyRef) -> Result<Vec<OntologyChange>> {
        let guard = ontology.read().map_err(|e| crate::Error::Internal {
            message: format!("{e}"),
        })?;
        let index = EntityIndex::from_ontology(&guard);
        let ontology_iri = guard
            .get_iri()
            .cloned()
            .unwrap_or_else(|| IRI::new("urn:anon"));
        drop(guard);

        let mut seen_ids = HashSet::new();
        let mut changes = Vec::new();
        for (iri, _et) in &self.entities_to_remove {
            for id in index.ids_for_entity(iri) {
                if !seen_ids.insert(id) {
                    continue;
                }
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
    fn default() -> Self {
        Self::new()
    }
}

//! OWL 2 DL Axioms
//! 
//! This module implements the various types of axioms in OWL 2 DL ontologies,
//! following the OWL 2 specification structure.

usa crate::{Error, Result};
use std::collections::{HashMap, HashSet};

/// Unique identifier for an OWL 2 DL axiom.
pub type AxiomId = u64;

/// Trait for OWL 2 DL axioms.
pub trait AxiomTrait {
    /// Returns the unique identifier for the axiom.
    fn axiom_id(&self) -> AxiomId;

    /// Returns the type of the axiom.
    fn axiom_type(&self) -> String;
    
    /// Returns whether the axiom is logical.
    fn is_logical(&self) -> bool;
}

/// Types of OWL 2 DL axioms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AxiomType {
    // Class Axioms
    SubClassOf,
    EquivalentClasses,
    DisjointClasses,
    DisjointUnion,

    // Object Property Axioms
    SubObjectPropertyOf,
    EquivalentObjectProperties,
    DisjointObjectProperties,
    InverseObjectProperties,
    ObjectPropertyDomain,
    ObjectPropertyRange,
    FunctionalObjectProperty,
    InverseFunctionalObjectProperty,
    ReflexiveObjectProperty,
    IrreflexiveObjectProperty,
    SymmetricObjectProperty,
    AsymmetricObjectProperty,
    TransitiveObjectProperty,

    // Data Property Axioms
    SubDataPropertyOf,
    EquivalentDataProperties,
    DisjointDataProperties,
    DataPropertyDomain,
    DataPropertyRange,
    FunctionalDataProperty,

    // Individual Axioms
    SameIndividual,
    DifferentIndividuals,
    ClassAssertion,
    ObjectPropertyAssertion,
    DataPropertyAssertion,
    NegativeObjectPropertyAssertion,
    NegativeDataPropertyAssertion,

    // Annotation Axioms
    AnnotationAssertion,
    SubAnnotationPropertyOf,
    AnnotationPropertyDomain,
    AnnotationPropertyRange,
}

/// Axiom enum representing all OWL 2 DL axioms.
#[derive(Debug, Clone, PartialEq)]
pub enum Axiom {
    // Class Axioms
    SubClassOf(SubClassOfAxiom),
    EquivalentClasses(EquivalentClassesAxiom),
    DisjointClasses(DisjointClassesAxiom),
    DisjointUnion(DisjointUnionAxiom),

    // Object Property Axioms
    SubObjectPropertyOf(SubObjectPropertyOfAxiom),
    EquivalentObjectProperties(EquivalentObjectPropertiesAxiom),
    DisjointObjectProperties(DisjointObjectPropertiesAxiom),
    InverseObjectProperties(InverseObjectPropertiesAxiom),
    ObjectPropertyDomain(ObjectPropertyDomainAxiom),
    ObjectPropertyRange(ObjectPropertyRangeAxiom),
    FunctionalObjectProperty(FunctionalObjectPropertyAxiom),
    InverseFunctionalObjectProperty(InverseFunctionalObjectPropertyAxiom),
    ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom),
    IrreflexiveObjectProperty(IrreflexiveObjectPropertyAxiom),
    SymmetricObjectProperty(SymmetricObjectPropertyAxiom),
    AsymmetricObjectProperty(AsymmetricObjectPropertyAxiom),
    TransitiveObjectProperty(TransitiveObjectPropertyAxiom),

    // Data Property Axioms
    SubDataPropertyOf(SubDataPropertyOfAxiom),
    EquivalentDataProperties(EquivalentDataPropertiesAxiom),
    DisjointDataProperties(DisjointDataPropertiesAxiom),
    DataPropertyDomain(DataPropertyDomainAxiom),
    DataPropertyRange(DataPropertyRangeAxiom),
    FunctionalDataProperty(FunctionalDataPropertyAxiom),

    // Individual Axioms
    SameIndividual(SameIndividualAxiom),
    DifferentIndividuals(DifferentIndividualsAxiom),
    ClassAssertion(ClassAssertionAxiom),
    ObjectPropertyAssertion(ObjectPropertyAssertionAxiom),
    DataPropertyAssertion(DataPropertyAssertionAxiom),
    NegativeObjectPropertyAssertion(NegativeObjectPropertyAssertionAxiom),
    NegativeDataPropertyAssertion(NegativeDataPropertyAssertionAxiom),

    // Annotation Axioms
    AnnotationAssertion(AnnotationAssertionAxiom),
    SubAnnotationPropertyOf(SubAnnotationPropertyOfAxiom),
    AnnotationPropertyDomain(AnnotationPropertyDomainAxiom),
    AnnotationPropertyRange(AnnotationPropertyRangeAxiom),
}

/// Class Axioms
#[derive(Debug, Clone, PartialEq)]
pub struct SubClassOfAxiom {
    pub id: AxiomId,
    pub subclass: crate::ontology::ClassExpression,
    pub superclass: crate::ontology::ClassExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquivalentClassesAxiom {
    pub id: AxiomId,
    pub classes: Vec<crate::ontology::ClassExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisjointClassesAxiom {
    pub id: AxiomId,
    pub classes: Vec<crate::ontology::ClassExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisjointUnionAxiom {
    pub id: AxiomId,
    pub class: crate::ontology::ClassExpression,
    pub disjoint_classes: Vec<crate::ontology::ClassExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

/// Object Property Axioms
#[derive(Debug, Clone, PartialEq)]
pub struct SubObjectPropertyOfAxiom {
    pub id: AxiomId,
    pub sub_property: crate::ontology::ObjectPropertyExpression,
    pub super_property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquivalentObjectPropertiesAxiom {
    pub id: AxiomId,
    pub properties: Vec<crate::ontology::ObjectPropertyExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisjointObjectPropertiesAxiom {
    pub id: AxiomId,
    pub properties: Vec<crate::ontology::ObjectPropertyExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InverseObjectPropertiesAxiom {
    pub id: AxiomId,
    pub property1: crate::ontology::ObjectPropertyExpression,
    pub property2: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPropertyDomainAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub domain: crate::ontology::ClassExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPropertyRangeAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub range: crate::ontology::ClassExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionalObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InverseFunctionalObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReflexiveObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrreflexiveObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SymmetricObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AsymmetricObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitiveObjectPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

/// Data Property Axioms
#[derive(Debug, Clone, PartialEq)]
pub struct SubDataPropertyOfAxiom {
    pub id: AxiomId,
    pub sub_property: crate::ontology::DataPropertyExpression,
    pub super_property: crate::ontology::DataPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquivalentDataPropertiesAxiom {
    pub id: AxiomId,
    pub properties: Vec<crate::ontology::DataPropertyExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DisjointDataPropertiesAxiom {
    pub id: AxiomId,
    pub properties: Vec<crate::ontology::DataPropertyExpression>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataPropertyDomainAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::DataPropertyExpression,
    pub domain: crate::ontology::ClassExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataPropertyRangeAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::DataPropertyExpression,
    pub range: crate::ontology::DataRange,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionalDataPropertyAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::DataPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

/// Individual Axioms
#[derive(Debug, Clone, PartialEq)]
pub struct SameIndividualAxiom {
    pub id: AxiomId,
    pub individuals: Vec<crate::ontology::Individual>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DifferentIndividualsAxiom {
    pub id: AxiomId,
    pub individuals: Vec<crate::ontology::Individual>,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ClassAssertionAxiom {
    pub id: AxiomId,
    pub individual: crate::ontology::Individual,
    pub class: crate::ontology::ClassExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectPropertyAssertionAxiom {
    pub id: AxiomId,
    pub source: crate::ontology::Individual,
    pub target: crate::ontology::Individual,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DataPropertyAssertionAxiom {
    pub id: AxiomId,
    pub individual: crate::ontology::Individual,
    pub property: crate::ontology::DataPropertyExpression,
    pub value: crate::ontology::DataValue,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegativeObjectPropertyAssertionAxiom {
    pub id: AxiomId,
    pub source: crate::ontology::Individual,
    pub target: crate::ontology::Individual,
    pub property: crate::ontology::ObjectPropertyExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NegativeDataPropertyAssertionAxiom {
    pub id: AxiomId,
    pub individual: crate::ontology::Individual,
    pub property: crate::ontology::DataPropertyExpression,
    pub value: crate::ontology::DataValue,
    pub annotations: Vec<crate::ontology::Annotation>,
}

/// Annotation Axioms
#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationAssertionAxiom {
    pub id: AxiomId,
    pub subject: crate::ontology::AnnotationSubject,
    pub property: crate::ontology::AnnotationProperty,
    pub value: crate::ontology::AnnotationValue,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubAnnotationPropertyOfAxiom {
    pub id: AxiomId,
    pub sub_property: crate::ontology::AnnotationProperty,
    pub super_property: crate::ontology::AnnotationProperty,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationPropertyDomainAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::AnnotationProperty,
    pub domain: crate::ontology::ClassExpression,
    pub annotations: Vec<crate::ontology::Annotation>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnnotationPropertyRangeAxiom {
    pub id: AxiomId,
    pub property: crate::ontology::AnnotationProperty,
    pub range: crate::ontology::DataRange,
    pub annotations: Vec<crate::ontology::Annotation>,
}

impl AxiomTrait for Axiom {
    fn axiom_id(&self) -> AxiomId {
        match self {
            Axiom::SubClassOf(axiom) => axiom.id,
            Axiom::EquivalentClasses(axiom) => axiom.id,
            Axiom::DisjointClasses(axiom) => axiom.id,
            Axiom::DisjointUnion(axiom) => axiom.id,
            Axiom::SubObjectPropertyOf(axiom) => axiom.id,
            Axiom::EquivalentObjectProperties(axiom) => axiom.id,
            Axiom::DisjointObjectProperties(axiom) => axiom.id,
            Axiom::InverseObjectProperties(axiom) => axiom.id,
            Axiom::ObjectPropertyDomain(axiom) => axiom.id,
            Axiom::ObjectPropertyRange(axiom) => axiom.id,
            Axiom::FunctionalObjectProperty(axiom) => axiom.id,
            Axiom::InverseFunctionalObjectProperty(axiom) => axiom.id,
            Axiom::ReflexiveObjectProperty(axiom) => axiom.id,
            Axiom::IrreflexiveObjectProperty(axiom) => axiom.id,
            Axiom::SymmetricObjectProperty(axiom) => axiom.id,
            Axiom::AsymmetricObjectProperty(axiom) => axiom.id,
            Axiom::TransitiveObjectProperty(axiom) => axiom.id,
            Axiom::SubDataPropertyOf(axiom) => axiom.id,
            Axiom::EquivalentDataProperties(axiom) => axiom.id,
            Axiom::DisjointDataProperties(axiom) => axiom.id,
            Axiom::DataPropertyDomain(axiom) => axiom.id,
            Axiom::DataPropertyRange(axiom) => axiom.id,
            Axiom::FunctionalDataProperty(axiom) => axiom.id,
            Axiom::SameIndividual(axiom) => axiom.id,
            Axiom::DifferentIndividuals(axiom) => axiom.id,
            Axiom::ClassAssertion(axiom) => axiom.id,
            Axiom::ObjectPropertyAssertion(axiom) => axiom.id,
            Axiom::DataPropertyAssertion(axiom) => axiom.id,
            Axiom::NegativeObjectPropertyAssertion(axiom) => axiom.id,
            Axiom::NegativeDataPropertyAssertion(axiom) => axiom.id,
            Axiom::AnnotationAssertion(axiom) => axiom.id,
            Axiom::SubAnnotationPropertyOf(axiom) => axiom.id,
            Axiom::AnnotationPropertyDomain(axiom) => axiom.id,
            Axiom::AnnotationPropertyRange(axiom) => axiom.id,
        }
    }

    fn axiom_type(&self) -> AxiomType {
        match self {
            Axiom::SubClassOf(_) => AxiomType::SubClassOf,
            Axiom::EquivalentClasses(_) => AxiomType::EquivalentClasses,
            Axiom::DisjointClasses(_) => AxiomType::DisjointClasses,
            Axiom::DisjointUnion(_) => AxiomType::DisjointUnion,
            Axiom::SubObjectPropertyOf(_) => AxiomType::SubObjectPropertyOf,
            Axiom::EquivalentObjectProperties(_) => AxiomType::EquivalentObjectProperties,
            Axiom::DisjointObjectProperties(_) => AxiomType::DisjointObjectProperties,
            Axiom::InverseObjectProperties(_) => AxiomType::InverseObjectProperties,
            Axiom::ObjectPropertyDomain(_) => AxiomType::ObjectPropertyDomain,
            Axiom::ObjectPropertyRange(_) => AxiomType::ObjectPropertyRange,
            Axiom::FunctionalObjectProperty(_) => AxiomType::FunctionalObjectProperty,
            Axiom::InverseFunctionalObjectProperty(_) => AxiomType::InverseFunctionalObjectProperty,
            Axiom::ReflexiveObjectProperty(_) => AxiomType::ReflexiveObjectProperty,
            Axiom::IrreflexiveObjectProperty(_) => AxiomType::IrreflexiveObjectProperty,
            Axiom::SymmetricObjectProperty(_) => AxiomType::SymmetricObjectProperty,
            Axiom::AsymmetricObjectProperty(_) => AxiomType::AsymmetricObjectProperty,
            Axiom::TransitiveObjectProperty(_) => AxiomType::TransitiveObjectProperty,
            Axiom::SubDataPropertyOf(_) => AxiomType::SubDataPropertyOf,
            Axiom::EquivalentDataProperties(_) => AxiomType::EquivalentDataProperties,
            Axiom::DisjointDataProperties(_) => AxiomType::DisjointDataProperties,
            Axiom::DataPropertyDomain(_) => AxiomType::DataPropertyDomain,
            Axiom::DataPropertyRange(_) => AxiomType::DataPropertyRange,
            Axiom::FunctionalDataProperty(_) => AxiomType::FunctionalDataProperty,
            Axiom::SameIndividual(_) => AxiomType::SameIndividual,
            Axiom::DifferentIndividuals(_) => AxiomType::DifferentIndividuals,
            Axiom::ClassAssertion(_) => AxiomType::ClassAssertion,
            Axiom::ObjectPropertyAssertion(_) => AxiomType::ObjectPropertyAssertion,
            Axiom::DataPropertyAssertion(_) => AxiomType::DataPropertyAssertion,
            Axiom::NegativeObjectPropertyAssertion(_) => AxiomType::NegativeObjectPropertyAssertion,
            Axiom::NegativeDataPropertyAssertion(_) => AxiomType::NegativeDataPropertyAssertion,
            Axiom::AnnotationAssertion(_) => AxiomType::AnnotationAssertion,
            Axiom::SubAnnotationPropertyOf(_) => AxiomType::SubAnnotationPropertyOf,
            Axiom::AnnotationPropertyDomain(_) => AxiomType::AnnotationPropertyDomain,
            Axiom::AnnotationPropertyRange(_) => AxiomType::AnnotationPropertyRange,
        }
    }

    fn is_logical(&self) -> bool {
        !matches!(self,
            Axiom::AnnotationAssertion(_) |
            Axiom::SubAnnotationPropertyOf(_) |
            Axiom::AnnotationPropertyDomain(_) |
            Axiom::AnnotationPropertyRange(_)
        )
    }
}

//! Entity provider traits decomposed from DataFactory.
//!
//! These traits allow code to depend on only the entity creation methods
//! they need, following the interface segregation principle.

use crate::ontology::{
    AnnotationProperty, AnonymousIndividual, Class, DataProperty, DataRange,
    ObjectProperty, IRI,
};
use crate::ontology::axioms::{
    AnnotationAssertionAxiom, AnnotationPropertyDomainAxiom, AnnotationPropertyRangeAxiom,
    AsymmetricObjectPropertyAxiom, AxiomId, ClassAssertionAxiom,
    DataPropertyAssertionAxiom, DataPropertyDomainAxiom, DataPropertyRangeAxiom,
    DeclarationAxiom, DifferentIndividualsAxiom, DisjointClassesAxiom,
    DisjointDataPropertiesAxiom, DisjointObjectPropertiesAxiom, DisjointUnionAxiom,
    Entity, EquivalentClassesAxiom, EquivalentDataPropertiesAxiom,
    EquivalentObjectPropertiesAxiom, FunctionalDataPropertyAxiom,
    FunctionalObjectPropertyAxiom, HasKeyAxiom, InverseFunctionalObjectPropertyAxiom,
    InverseObjectPropertiesAxiom, IrreflexiveObjectPropertyAxiom,
    NegativeDataPropertyAssertionAxiom, NegativeObjectPropertyAssertionAxiom,
    ObjectPropertyAssertionAxiom, ObjectPropertyDomainAxiom, ObjectPropertyRangeAxiom,
    ReflexiveObjectPropertyAxiom, SameIndividualAxiom, SubAnnotationPropertyOfAxiom,
    SubClassOfAxiom, SubDataPropertyOfAxiom, SubObjectPropertyOfAxiom,
    SymmetricObjectPropertyAxiom, TransitiveObjectPropertyAxiom,
};
use crate::ontology::concepts::ClassExpression;
use crate::ontology::individuals::{Individual, NamedIndividual};
use crate::ontology::{Annotation, AnnotationSubject, AnnotationValue, Literal};

// ── Entity Providers ─────────────────────────────────────────────────────────

pub trait ClassProvider {
    fn get_class(&self, iri: &IRI) -> Class;
}

pub trait ObjectPropertyProvider {
    fn get_object_property(&self, iri: &IRI) -> ObjectProperty;
}

pub trait DataPropertyProvider {
    fn get_data_property(&self, iri: &IRI) -> DataProperty;
}

pub trait IndividualProvider {
    fn get_named_individual(&self, iri: &IRI) -> NamedIndividual;
    fn get_anonymous_individual(&self) -> AnonymousIndividual;
}

pub trait DatatypeProvider {
    fn get_datatype(&self, iri: &IRI) -> DataRange;
}

pub trait AnnotationPropertyProvider {
    fn get_annotation_property(&self, iri: &IRI) -> AnnotationProperty;
}

pub trait EntityProvider:
    ClassProvider
    + ObjectPropertyProvider
    + DataPropertyProvider
    + IndividualProvider
    + DatatypeProvider
    + AnnotationPropertyProvider
{
}

impl<T> EntityProvider for T where
    T: ClassProvider
        + ObjectPropertyProvider
        + DataPropertyProvider
        + IndividualProvider
        + DatatypeProvider
        + AnnotationPropertyProvider
{
}

// ── Axiom Provider (creation-building methods) ───────────────────────────────

#[allow(clippy::too_many_arguments)]
pub trait AxiomCreationProvider {
    fn next_axiom_id(&self) -> AxiomId;

    // Class axioms
    fn make_sub_class_of_axiom(
        &self,
        subclass: ClassExpression,
        superclass: ClassExpression,
        annotations: Vec<Annotation>,
    ) -> SubClassOfAxiom {
        SubClassOfAxiom {
            id: self.next_axiom_id(),
            subclass,
            superclass,
            annotations,
        }
    }

    fn make_equivalent_classes_axiom(
        &self,
        classes: Vec<ClassExpression>,
        annotations: Vec<Annotation>,
    ) -> EquivalentClassesAxiom {
        EquivalentClassesAxiom {
            id: self.next_axiom_id(),
            classes,
            annotations,
        }
    }

    fn make_disjoint_classes_axiom(
        &self,
        classes: Vec<ClassExpression>,
        annotations: Vec<Annotation>,
    ) -> DisjointClassesAxiom {
        DisjointClassesAxiom {
            id: self.next_axiom_id(),
            classes,
            annotations,
        }
    }

    fn make_disjoint_union_axiom(
        &self,
        class: ClassExpression,
        disjoint_classes: Vec<ClassExpression>,
        annotations: Vec<Annotation>,
    ) -> DisjointUnionAxiom {
        DisjointUnionAxiom {
            id: self.next_axiom_id(),
            class,
            disjoint_classes,
            annotations,
        }
    }

    fn make_sub_object_property_of_axiom(
        &self,
        sub_property: crate::ontology::ObjectPropertyExpression,
        super_property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> SubObjectPropertyOfAxiom {
        SubObjectPropertyOfAxiom {
            id: self.next_axiom_id(),
            sub_property,
            super_property,
            annotations,
        }
    }

    fn make_equivalent_object_properties_axiom(
        &self,
        properties: Vec<crate::ontology::ObjectPropertyExpression>,
        annotations: Vec<Annotation>,
    ) -> EquivalentObjectPropertiesAxiom {
        EquivalentObjectPropertiesAxiom {
            id: self.next_axiom_id(),
            properties,
            annotations,
        }
    }

    fn make_disjoint_object_properties_axiom(
        &self,
        properties: Vec<crate::ontology::ObjectPropertyExpression>,
        annotations: Vec<Annotation>,
    ) -> DisjointObjectPropertiesAxiom {
        DisjointObjectPropertiesAxiom {
            id: self.next_axiom_id(),
            properties,
            annotations,
        }
    }

    fn make_inverse_object_properties_axiom(
        &self,
        property1: crate::ontology::ObjectPropertyExpression,
        property2: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> InverseObjectPropertiesAxiom {
        InverseObjectPropertiesAxiom {
            id: self.next_axiom_id(),
            property1,
            property2,
            annotations,
        }
    }

    fn make_object_property_domain_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        domain: ClassExpression,
        annotations: Vec<Annotation>,
    ) -> ObjectPropertyDomainAxiom {
        ObjectPropertyDomainAxiom {
            id: self.next_axiom_id(),
            property,
            domain,
            annotations,
        }
    }

    fn make_object_property_range_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        range: ClassExpression,
        annotations: Vec<Annotation>,
    ) -> ObjectPropertyRangeAxiom {
        ObjectPropertyRangeAxiom {
            id: self.next_axiom_id(),
            property,
            range,
            annotations,
        }
    }

    fn make_functional_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> FunctionalObjectPropertyAxiom {
        FunctionalObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_inverse_functional_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> InverseFunctionalObjectPropertyAxiom {
        InverseFunctionalObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_reflexive_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> ReflexiveObjectPropertyAxiom {
        ReflexiveObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_irreflexive_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> IrreflexiveObjectPropertyAxiom {
        IrreflexiveObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_symmetric_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> SymmetricObjectPropertyAxiom {
        SymmetricObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_asymmetric_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> AsymmetricObjectPropertyAxiom {
        AsymmetricObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_transitive_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> TransitiveObjectPropertyAxiom {
        TransitiveObjectPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_sub_data_property_of_axiom(
        &self,
        sub_property: crate::ontology::DataPropertyExpression,
        super_property: crate::ontology::DataPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> SubDataPropertyOfAxiom {
        SubDataPropertyOfAxiom {
            id: self.next_axiom_id(),
            sub_property,
            super_property,
            annotations,
        }
    }

    fn make_equivalent_data_properties_axiom(
        &self,
        properties: Vec<crate::ontology::DataPropertyExpression>,
        annotations: Vec<Annotation>,
    ) -> EquivalentDataPropertiesAxiom {
        EquivalentDataPropertiesAxiom {
            id: self.next_axiom_id(),
            properties,
            annotations,
        }
    }

    fn make_disjoint_data_properties_axiom(
        &self,
        properties: Vec<crate::ontology::DataPropertyExpression>,
        annotations: Vec<Annotation>,
    ) -> DisjointDataPropertiesAxiom {
        DisjointDataPropertiesAxiom {
            id: self.next_axiom_id(),
            properties,
            annotations,
        }
    }

    fn make_data_property_domain_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        domain: ClassExpression,
        annotations: Vec<Annotation>,
    ) -> DataPropertyDomainAxiom {
        DataPropertyDomainAxiom {
            id: self.next_axiom_id(),
            property,
            domain,
            annotations,
        }
    }

    fn make_data_property_range_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        range: DataRange,
        annotations: Vec<Annotation>,
    ) -> DataPropertyRangeAxiom {
        DataPropertyRangeAxiom {
            id: self.next_axiom_id(),
            property,
            range,
            annotations,
        }
    }

    fn make_functional_data_property_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        annotations: Vec<Annotation>,
    ) -> FunctionalDataPropertyAxiom {
        FunctionalDataPropertyAxiom {
            id: self.next_axiom_id(),
            property,
            annotations,
        }
    }

    fn make_same_individual_axiom(
        &self,
        individuals: Vec<Individual>,
        annotations: Vec<Annotation>,
    ) -> SameIndividualAxiom {
        SameIndividualAxiom {
            id: self.next_axiom_id(),
            individuals,
            annotations,
        }
    }

    fn make_different_individuals_axiom(
        &self,
        individuals: Vec<Individual>,
        annotations: Vec<Annotation>,
    ) -> DifferentIndividualsAxiom {
        DifferentIndividualsAxiom {
            id: self.next_axiom_id(),
            individuals,
            annotations,
        }
    }

    fn make_class_assertion_axiom(
        &self,
        class: ClassExpression,
        individual: Individual,
        annotations: Vec<Annotation>,
    ) -> ClassAssertionAxiom {
        ClassAssertionAxiom {
            id: self.next_axiom_id(),
            class,
            individual,
            annotations,
        }
    }

    fn make_object_property_assertion_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        source: Individual,
        target: Individual,
        annotations: Vec<Annotation>,
    ) -> ObjectPropertyAssertionAxiom {
        ObjectPropertyAssertionAxiom {
            id: self.next_axiom_id(),
            property,
            source,
            target,
            annotations,
        }
    }

    fn make_data_property_assertion_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        individual: Individual,
        value: Literal,
        annotations: Vec<Annotation>,
    ) -> DataPropertyAssertionAxiom {
        DataPropertyAssertionAxiom {
            id: self.next_axiom_id(),
            property,
            individual,
            value,
            annotations,
        }
    }

    fn make_negative_object_property_assertion_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        source: Individual,
        target: Individual,
        annotations: Vec<Annotation>,
    ) -> NegativeObjectPropertyAssertionAxiom {
        NegativeObjectPropertyAssertionAxiom {
            id: self.next_axiom_id(),
            property,
            source,
            target,
            annotations,
        }
    }

    fn make_negative_data_property_assertion_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        individual: Individual,
        value: Literal,
        annotations: Vec<Annotation>,
    ) -> NegativeDataPropertyAssertionAxiom {
        NegativeDataPropertyAssertionAxiom {
            id: self.next_axiom_id(),
            property,
            individual,
            value,
            annotations,
        }
    }

    fn make_annotation_assertion_axiom(
        &self,
        property: AnnotationProperty,
        subject: AnnotationSubject,
        value: AnnotationValue,
        annotations: Vec<Annotation>,
    ) -> AnnotationAssertionAxiom {
        AnnotationAssertionAxiom {
            id: self.next_axiom_id(),
            property,
            subject,
            value,
            annotations,
        }
    }

    fn make_sub_annotation_property_of_axiom(
        &self,
        sub_property: AnnotationProperty,
        super_property: AnnotationProperty,
        annotations: Vec<Annotation>,
    ) -> SubAnnotationPropertyOfAxiom {
        SubAnnotationPropertyOfAxiom {
            id: self.next_axiom_id(),
            sub_property,
            super_property,
            annotations,
        }
    }

    fn make_annotation_property_domain_axiom(
        &self,
        property: AnnotationProperty,
        domain: IRI,
        annotations: Vec<Annotation>,
    ) -> AnnotationPropertyDomainAxiom {
        AnnotationPropertyDomainAxiom {
            id: self.next_axiom_id(),
            property,
            domain: crate::ontology::ClassExpression::Class(crate::ontology::concepts::Class { iri: domain }),
            annotations,
        }
    }

    fn make_annotation_property_range_axiom(
        &self,
        property: AnnotationProperty,
        range: IRI,
        annotations: Vec<Annotation>,
    ) -> AnnotationPropertyRangeAxiom {
        AnnotationPropertyRangeAxiom {
            id: self.next_axiom_id(),
            property,
            range: DataRange::Datatype(range),
            annotations,
        }
    }

    fn make_has_key_axiom(
        &self,
        class: ClassExpression,
        object_properties: Vec<crate::ontology::ObjectPropertyExpression>,
        data_properties: Vec<crate::ontology::DataPropertyExpression>,
        annotations: Vec<Annotation>,
    ) -> HasKeyAxiom {
        HasKeyAxiom {
            id: self.next_axiom_id(),
            class,
            object_properties,
            data_properties,
            annotations,
        }
    }

    fn make_declaration_axiom(&self, entity: Entity) -> DeclarationAxiom {
        DeclarationAxiom {
            id: self.next_axiom_id(),
            entity,
        }
    }

}

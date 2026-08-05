//! OWL Functional Syntax Factory — static convenience methods for
//! constructing OWL 2 axioms and class expressions.
//!
//! Equivalent to OWL API v5's `OWLFunctionalSyntaxFactory`.
//! All methods delegate to `DataFactory` internally.

use crate::factory::DataFactory;
use crate::ontology::axioms::*;
use crate::ontology::concepts::ClassExpression;
use crate::ontology::{
    Annotation, Class, DataProperty, DataPropertyExpression, DataRange, IRI, ObjectProperty,
    ObjectPropertyExpression,
};

/// Static convenience factory for constructing OWL 2 objects.
///
/// Provides short-hand static methods for common axiom and class expression
/// construction, mirroring the OWL API's `OWLFunctionalSyntaxFactory`.
pub struct FunctionalSyntaxFactory;

impl FunctionalSyntaxFactory {
    #[must_use]
    pub fn data_factory() -> DataFactory {
        DataFactory::new()
    }

    // ── Class Axioms ──────────────────────────────────────────────────────

    #[must_use]
    pub fn sub_class_of(sub: ClassExpression, sup: ClassExpression) -> SubClassOfAxiom {
        let df = Self::data_factory();
        SubClassOfAxiom {
            id: df.next_id(),
            subclass: sub,
            superclass: sup,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn equivalent_classes(classes: Vec<ClassExpression>) -> EquivalentClassesAxiom {
        let df = Self::data_factory();
        EquivalentClassesAxiom {
            id: df.next_id(),
            classes,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn disjoint_classes(classes: Vec<ClassExpression>) -> DisjointClassesAxiom {
        let df = Self::data_factory();
        DisjointClassesAxiom {
            id: df.next_id(),
            classes,
            annotations: vec![],
        }
    }

    // ── Class Expressions ─────────────────────────────────────────────────

    #[must_use]
    pub fn owl_class(iri: &IRI) -> ClassExpression {
        ClassExpression::Class(Class { iri: iri.clone() })
    }

    #[must_use]
    pub fn object_intersection_of(operands: Vec<ClassExpression>) -> ClassExpression {
        ClassExpression::ObjectIntersectionOf(operands)
    }

    #[must_use]
    pub fn object_union_of(operands: Vec<ClassExpression>) -> ClassExpression {
        ClassExpression::ObjectUnionOf(operands)
    }

    #[must_use]
    pub fn object_complement_of(operand: ClassExpression) -> ClassExpression {
        ClassExpression::ObjectComplementOf(Box::new(operand))
    }

    #[must_use]
    pub fn object_some_values_from(
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    #[must_use]
    pub fn object_all_values_from(
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectAllValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    #[must_use]
    pub fn object_has_value(
        property: ObjectPropertyExpression,
        value: crate::ontology::Individual,
    ) -> ClassExpression {
        ClassExpression::ObjectHasValue { property, value }
    }

    #[must_use]
    pub fn object_has_self(property: ObjectPropertyExpression) -> ClassExpression {
        ClassExpression::ObjectHasSelf { property }
    }

    #[must_use]
    pub fn object_one_of(individuals: Vec<crate::ontology::Individual>) -> ClassExpression {
        ClassExpression::ObjectOneOf(individuals)
    }

    #[must_use]
    pub fn data_some_values_from(
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataSomeValuesFrom { property, filler }
    }

    #[must_use]
    pub fn data_all_values_from(
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataAllValuesFrom { property, filler }
    }

    #[must_use]
    pub fn data_has_value(
        property: DataPropertyExpression,
        value: crate::ontology::Literal,
    ) -> ClassExpression {
        ClassExpression::DataHasValue { property, value }
    }

    // ── Object Property Axioms ────────────────────────────────────────────

    #[must_use]
    pub fn functional_object_property(
        property: ObjectPropertyExpression,
    ) -> FunctionalObjectPropertyAxiom {
        let df = Self::data_factory();
        FunctionalObjectPropertyAxiom {
            id: df.next_id(),
            property,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn transitive_object_property(
        property: ObjectPropertyExpression,
    ) -> TransitiveObjectPropertyAxiom {
        let df = Self::data_factory();
        TransitiveObjectPropertyAxiom {
            id: df.next_id(),
            property,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn symmetric_object_property(
        property: ObjectPropertyExpression,
    ) -> SymmetricObjectPropertyAxiom {
        let df = Self::data_factory();
        SymmetricObjectPropertyAxiom {
            id: df.next_id(),
            property,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn asymmetric_object_property(
        property: ObjectPropertyExpression,
    ) -> AsymmetricObjectPropertyAxiom {
        let df = Self::data_factory();
        AsymmetricObjectPropertyAxiom {
            id: df.next_id(),
            property,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn reflexive_object_property(
        property: ObjectPropertyExpression,
    ) -> ReflexiveObjectPropertyAxiom {
        let df = Self::data_factory();
        ReflexiveObjectPropertyAxiom {
            id: df.next_id(),
            property,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn irreflexive_object_property(
        property: ObjectPropertyExpression,
    ) -> IrreflexiveObjectPropertyAxiom {
        let df = Self::data_factory();
        IrreflexiveObjectPropertyAxiom {
            id: df.next_id(),
            property,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn inverse_object_properties(
        prop1: ObjectPropertyExpression,
        prop2: ObjectPropertyExpression,
    ) -> InverseObjectPropertiesAxiom {
        let df = Self::data_factory();
        InverseObjectPropertiesAxiom {
            id: df.next_id(),
            property1: prop1,
            property2: prop2,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn sub_object_property_of(
        sub: ObjectPropertyExpression,
        sup: ObjectPropertyExpression,
    ) -> SubObjectPropertyOfAxiom {
        let df = Self::data_factory();
        SubObjectPropertyOfAxiom {
            id: df.next_id(),
            sub_property: sub,
            super_property: sup,
            annotations: vec![],
        }
    }

    // ── Class Assertion ───────────────────────────────────────────────────

    #[must_use]
    pub fn class_assertion(
        class: ClassExpression,
        individual: crate::ontology::Individual,
    ) -> ClassAssertionAxiom {
        let df = Self::data_factory();
        ClassAssertionAxiom {
            id: df.next_id(),
            class,
            individual,
            annotations: vec![],
        }
    }

    #[must_use]
    pub fn declaration(entity: Entity) -> DeclarationAxiom {
        let df = Self::data_factory();
        DeclarationAxiom {
            id: df.next_id(),
            entity,
        }
    }

    // ── Annotation ────────────────────────────────────────────────────────

    #[must_use]
    pub fn annotation(
        property: crate::ontology::AnnotationProperty,
        value: crate::ontology::AnnotationValue,
    ) -> Annotation {
        Annotation {
            property,
            value,
            annotations: vec![],
        }
    }

    // ── Object / Data Property Expressions ────────────────────────────────

    #[must_use]
    pub fn object_property(iri: &IRI) -> ObjectPropertyExpression {
        ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: iri.clone() })
    }

    #[must_use]
    pub fn inverse_object_property(iri: &IRI) -> ObjectPropertyExpression {
        ObjectPropertyExpression::InverseObjectProperty(ObjectProperty { iri: iri.clone() })
    }

    #[must_use]
    pub fn data_property(iri: &IRI) -> DataPropertyExpression {
        DataPropertyExpression::DataProperty(DataProperty { iri: iri.clone() })
    }
}

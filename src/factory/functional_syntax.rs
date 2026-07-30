//! OWL Functional Syntax Factory — static convenience methods for
//! constructing OWL 2 axioms and class expressions.
//!
//! Equivalent to OWL API v5's `OWLFunctionalSyntaxFactory`.
//! All methods are static convenience constructors with short names.

use crate::ontology::axioms::*;
use crate::ontology::*;

/// Static convenience factory for creating OWL entities, class expressions,
/// and axioms with short, ergonomic method names.
///
/// Models the pattern from OWL API v5's `OWLFunctionalSyntaxFactory`.
/// All methods are static associated functions that provide a concise
/// API for construction-heavy code.
pub struct FunctionalSyntaxFactory;

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl FunctionalSyntaxFactory {
    fn next_id() -> u64 {
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    // ── Entity shortcuts ─────────────────────────────────────────────────────

    #[must_use]
    pub fn class(iri: &str) -> Class {
        Class {
            iri: IRI::new(iri),
        }
    }

    #[must_use]
    pub fn object_property(iri: &str) -> ObjectProperty {
        ObjectProperty {
            iri: IRI::new(iri),
        }
    }

    #[must_use]
    pub fn data_property(iri: &str) -> DataProperty {
        DataProperty {
            iri: IRI::new(iri),
        }
    }

    #[must_use]
    pub fn named_individual(iri: &str) -> NamedIndividual {
        NamedIndividual {
            iri: IRI::new(iri),
        }
    }

    #[must_use]
    pub fn annotation_property(iri: &str) -> AnnotationProperty {
        AnnotationProperty {
            iri: IRI::new(iri),
        }
    }

    #[must_use]
    pub fn datatype(iri: &str) -> Datatype {
        Datatype {
            iri: IRI::new(iri),
        }
    }

    // ── Class expression constructors ───────────────────────────────────────

    #[must_use]
    pub fn owl_thing() -> ClassExpression {
        ClassExpression::Class(Class::thing())
    }

    #[must_use]
    pub fn owl_nothing() -> ClassExpression {
        ClassExpression::Class(Class::nothing())
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
    pub fn object_one_of(individuals: Vec<Individual>) -> ClassExpression {
        ClassExpression::ObjectOneOf(individuals)
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
        value: Individual,
    ) -> ClassExpression {
        ClassExpression::ObjectHasValue { property, value }
    }

    #[must_use]
    pub fn object_has_self(
        property: ObjectPropertyExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectHasSelf { property }
    }

    #[must_use]
    pub fn object_min_cardinality(
        cardinality: u32,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectMinCardinality {
            cardinality,
            property,
            filler: Box::new(filler),
        }
    }

    #[must_use]
    pub fn object_max_cardinality(
        cardinality: u32,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectMaxCardinality {
            cardinality,
            property,
            filler: Box::new(filler),
        }
    }

    #[must_use]
    pub fn object_exact_cardinality(
        cardinality: u32,
        property: ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectExactCardinality {
            cardinality,
            property,
            filler: Box::new(filler),
        }
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
        value: Literal,
    ) -> ClassExpression {
        ClassExpression::DataHasValue { property, value }
    }

    #[must_use]
    pub fn data_min_cardinality(
        cardinality: u32,
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataMinCardinality {
            cardinality,
            property,
            filler,
        }
    }

    #[must_use]
    pub fn data_max_cardinality(
        cardinality: u32,
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataMaxCardinality {
            cardinality,
            property,
            filler,
        }
    }

    #[must_use]
    pub fn data_exact_cardinality(
        cardinality: u32,
        property: DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataExactCardinality {
            cardinality,
            property,
            filler,
        }
    }

    // ── Property expression helpers ──────────────────────────────────────────

    #[must_use]
    pub fn object_inverse_of(property: ObjectProperty) -> ObjectPropertyExpression {
        ObjectPropertyExpression::InverseObjectProperty(property)
    }

    #[must_use]
    pub fn property_chain(
        chain: Vec<ObjectPropertyExpression>,
    ) -> ObjectPropertyExpression {
        ObjectPropertyExpression::PropertyChain(chain)
    }

    // ── Axiom constructors ──────────────────────────────────────────────────

    #[must_use]
    pub fn declaration(entity: Entity) -> Axiom {
        Axiom::Declaration(DeclarationAxiom {
            id: Self::next_id(),
            entity,
        })
    }

    #[must_use]
    pub fn sub_class_of(subclass: ClassExpression, superclass: ClassExpression) -> Axiom {
        Axiom::SubClassOf(SubClassOfAxiom {
            id: Self::next_id(),
            subclass,
            superclass,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn equivalent_classes(classes: Vec<ClassExpression>) -> Axiom {
        Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: Self::next_id(),
            classes,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn disjoint_classes(classes: Vec<ClassExpression>) -> Axiom {
        Axiom::DisjointClasses(DisjointClassesAxiom {
            id: Self::next_id(),
            classes,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn disjoint_union(
        class: ClassExpression,
        disjoint_classes: Vec<ClassExpression>,
    ) -> Axiom {
        Axiom::DisjointUnion(DisjointUnionAxiom {
            id: Self::next_id(),
            class,
            disjoint_classes,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn sub_object_property_of(
        sub: ObjectPropertyExpression,
        sup: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
            id: Self::next_id(),
            sub_property: sub,
            super_property: sup,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn equivalent_object_properties(
        properties: Vec<ObjectPropertyExpression>,
    ) -> Axiom {
        Axiom::EquivalentObjectProperties(EquivalentObjectPropertiesAxiom {
            id: Self::next_id(),
            properties,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn disjoint_object_properties(
        properties: Vec<ObjectPropertyExpression>,
    ) -> Axiom {
        Axiom::DisjointObjectProperties(DisjointObjectPropertiesAxiom {
            id: Self::next_id(),
            properties,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn inverse_object_properties(
        p1: ObjectPropertyExpression,
        p2: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::InverseObjectProperties(InverseObjectPropertiesAxiom {
            id: Self::next_id(),
            property1: p1,
            property2: p2,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn object_property_domain(
        property: ObjectPropertyExpression,
        domain: ClassExpression,
    ) -> Axiom {
        Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
            id: Self::next_id(),
            property,
            domain,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn object_property_range(
        property: ObjectPropertyExpression,
        range: ClassExpression,
    ) -> Axiom {
        Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
            id: Self::next_id(),
            property,
            range,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn functional_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::FunctionalObjectProperty(FunctionalObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn inverse_functional_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::InverseFunctionalObjectProperty(InverseFunctionalObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn reflexive_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::ReflexiveObjectProperty(ReflexiveObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn irreflexive_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::IrreflexiveObjectProperty(IrreflexiveObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn symmetric_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::SymmetricObjectProperty(SymmetricObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn asymmetric_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::AsymmetricObjectProperty(AsymmetricObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn transitive_object_property(
        property: ObjectPropertyExpression,
    ) -> Axiom {
        Axiom::TransitiveObjectProperty(TransitiveObjectPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn sub_data_property_of(
        sub: DataPropertyExpression,
        sup: DataPropertyExpression,
    ) -> Axiom {
        Axiom::SubDataPropertyOf(SubDataPropertyOfAxiom {
            id: Self::next_id(),
            sub_property: sub,
            super_property: sup,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn equivalent_data_properties(
        properties: Vec<DataPropertyExpression>,
    ) -> Axiom {
        Axiom::EquivalentDataProperties(EquivalentDataPropertiesAxiom {
            id: Self::next_id(),
            properties,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn disjoint_data_properties(
        properties: Vec<DataPropertyExpression>,
    ) -> Axiom {
        Axiom::DisjointDataProperties(DisjointDataPropertiesAxiom {
            id: Self::next_id(),
            properties,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn data_property_domain(
        property: DataPropertyExpression,
        domain: ClassExpression,
    ) -> Axiom {
        Axiom::DataPropertyDomain(DataPropertyDomainAxiom {
            id: Self::next_id(),
            property,
            domain,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn data_property_range(
        property: DataPropertyExpression,
        range: DataRange,
    ) -> Axiom {
        Axiom::DataPropertyRange(DataPropertyRangeAxiom {
            id: Self::next_id(),
            property,
            range,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn functional_data_property(
        property: DataPropertyExpression,
    ) -> Axiom {
        Axiom::FunctionalDataProperty(FunctionalDataPropertyAxiom {
            id: Self::next_id(),
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn class_assertion(
        class: ClassExpression,
        individual: Individual,
    ) -> Axiom {
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: Self::next_id(),
            individual,
            class,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn object_property_assertion(
        property: ObjectPropertyExpression,
        source: Individual,
        target: Individual,
    ) -> Axiom {
        Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
            id: Self::next_id(),
            source,
            target,
            property,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn data_property_assertion(
        property: DataPropertyExpression,
        individual: Individual,
        value: Literal,
    ) -> Axiom {
        Axiom::DataPropertyAssertion(DataPropertyAssertionAxiom {
            id: Self::next_id(),
            individual,
            property,
            value,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn same_individual(individuals: Vec<Individual>) -> Axiom {
        Axiom::SameIndividual(SameIndividualAxiom {
            id: Self::next_id(),
            individuals,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn different_individuals(individuals: Vec<Individual>) -> Axiom {
        Axiom::DifferentIndividuals(DifferentIndividualsAxiom {
            id: Self::next_id(),
            individuals,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn annotation_assertion(
        property: AnnotationProperty,
        subject: AnnotationSubject,
        value: AnnotationValue,
    ) -> Axiom {
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: Self::next_id(),
            subject,
            property,
            value,
            annotations: vec![],
        })
    }

    #[must_use]
    pub fn has_key(
        class: ClassExpression,
        object_properties: Vec<ObjectPropertyExpression>,
        data_properties: Vec<DataPropertyExpression>,
    ) -> Axiom {
        Axiom::HasKey(HasKeyAxiom {
            id: Self::next_id(),
            class,
            object_properties,
            data_properties,
            annotations: vec![],
        })
    }
}

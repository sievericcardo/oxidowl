//! OWL DataFactory — unified entity/axiom/expression builder.
//!
//! Provides entity interning (deduplication by IRI), consistent ID
//! generation for axioms, and builder methods for all OWL 2 constructs.

pub mod providers;

use self::providers::AxiomCreationProvider;
use crate::ontology::axioms::{AxiomId, Entity, EntityType};
use crate::ontology::concepts::ClassExpression;
use crate::ontology::individuals::{Individual, NamedIndividual};
use crate::ontology::{Annotation, AnnotationValue, Literal};
use crate::ontology::{
    AnnotationProperty, AnonymousIndividual, Class, DataProperty, DataRange, Datatype, IRI,
    ObjectProperty,
};
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};

// ── DataFactory ──────────────────────────────────────────────────────────────

/// Central factory for creating OWL objects with deduplication and
/// consistent ID generation.
///
/// Entity interning ensures that calling `get_class(iri)` twice with
/// the same IRI returns the same `Class` object. Axiom IDs are
/// monotonically increasing and thread-safe.
pub struct DataFactory {
    class_cache: RwLock<HashMap<IRI, Class>>,
    object_property_cache: RwLock<HashMap<IRI, ObjectProperty>>,
    data_property_cache: RwLock<HashMap<IRI, DataProperty>>,
    individual_cache: RwLock<HashMap<IRI, NamedIndividual>>,
    annotation_property_cache: RwLock<HashMap<IRI, AnnotationProperty>>,
    datatype_cache: RwLock<HashMap<IRI, Datatype>>,
    next_axiom_id: AtomicU64,
}

impl std::fmt::Debug for DataFactory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataFactory")
            .field("next_axiom_id", &self.next_axiom_id.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl DataFactory {
    /// Create a new DataFactory with empty entity caches.
    #[must_use]
    pub fn new() -> Self {
        Self {
            class_cache: RwLock::new(HashMap::new()),
            object_property_cache: RwLock::new(HashMap::new()),
            data_property_cache: RwLock::new(HashMap::new()),
            individual_cache: RwLock::new(HashMap::new()),
            annotation_property_cache: RwLock::new(HashMap::new()),
            datatype_cache: RwLock::new(HashMap::new()),
            next_axiom_id: AtomicU64::new(1),
        }
    }

    /// Get the current entity counts (for debugging/telemetry).
    #[allow(dead_code)]
    #[must_use]
    pub(crate) fn stats(&self) -> HashMap<&'static str, usize> {
        let mut s = HashMap::with_capacity(5);
        s.insert(
            "classes",
            self.class_cache
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        );
        s.insert(
            "object_properties",
            self.object_property_cache
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        );
        s.insert(
            "data_properties",
            self.data_property_cache
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        );
        s.insert(
            "individuals",
            self.individual_cache
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        );
        s.insert(
            "annotation_properties",
            self.annotation_property_cache
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        );
        s.insert(
            "datatypes",
            self.datatype_cache
                .read()
                .unwrap_or_else(|e| e.into_inner())
                .len(),
        );
        s
    }

    // ── Entity creation (with interning) ─────────────────────────────────

    /// Generate a fresh, monotonically increasing axiom ID.
    #[must_use]
    pub fn next_id(&self) -> AxiomId {
        self.next_axiom_id.fetch_add(1, Ordering::Relaxed)
    }

    /// Get or create a named class for the given IRI.
    pub fn get_class(&self, iri: &IRI) -> Class {
        let cache = self.class_cache.read().unwrap_or_else(|e| e.into_inner());
        if let Some(cls) = cache.get(iri) {
            return cls.clone();
        }
        drop(cache);
        let mut cache = self.class_cache.write().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| Class { iri: iri.clone() })
            .clone()
    }

    /// Get or create an object property for the given IRI.
    pub fn get_object_property(&self, iri: &IRI) -> ObjectProperty {
        let cache = self
            .object_property_cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(prop) = cache.get(iri) {
            return prop.clone();
        }
        drop(cache);
        let mut cache = self
            .object_property_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| ObjectProperty { iri: iri.clone() })
            .clone()
    }

    /// Get or create a data property for the given IRI.
    pub fn get_data_property(&self, iri: &IRI) -> DataProperty {
        let cache = self
            .data_property_cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(prop) = cache.get(iri) {
            return prop.clone();
        }
        drop(cache);
        let mut cache = self
            .data_property_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| DataProperty { iri: iri.clone() })
            .clone()
    }

    /// Get or create a named individual for the given IRI.
    pub fn get_named_individual(&self, iri: &IRI) -> NamedIndividual {
        let cache = self
            .individual_cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(ind) = cache.get(iri) {
            return ind.clone();
        }
        drop(cache);
        let mut cache = self
            .individual_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| NamedIndividual { iri: iri.clone() })
            .clone()
    }

    /// Create a fresh anonymous individual.
    #[must_use]
    pub fn get_anonymous_individual(&self) -> AnonymousIndividual {
        match Individual::fresh() {
            Individual::Anonymous(anon) => anon,
            Individual::Named(_) => unreachable!(),
        }
    }

    /// Get or create an annotation property for the given IRI.
    pub fn get_annotation_property(&self, iri: &IRI) -> AnnotationProperty {
        let cache = self
            .annotation_property_cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(prop) = cache.get(iri) {
            return prop.clone();
        }
        drop(cache);
        let mut cache = self
            .annotation_property_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| AnnotationProperty { iri: iri.clone() })
            .clone()
    }

    /// Get or create a named datatype for the given IRI.
    pub fn get_owl_datatype(&self, iri: &IRI) -> Datatype {
        let cache = self
            .datatype_cache
            .read()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(dt) = cache.get(iri) {
            return dt.clone();
        }
        drop(cache);
        let mut cache = self
            .datatype_cache
            .write()
            .unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| Datatype { iri: iri.clone() })
            .clone()
    }

    /// Get a datatype (interned) for the given IRI.
    #[must_use]
    pub fn get_datatype(&self, iri: &IRI) -> Datatype {
        self.get_owl_datatype(iri)
    }

    /// Get a datatype data range for the given IRI.
    #[must_use]
    pub fn get_datatype_range(&self, iri: &IRI) -> DataRange {
        DataRange::Datatype(iri.clone())
    }

    /// Get an entity by IRI and entity type (OWL 2 punning support).
    #[must_use]
    pub fn get_entity(&self, iri: &IRI, entity_type: &EntityType) -> Entity {
        match entity_type {
            EntityType::Class => Entity::Class(iri.clone()),
            EntityType::ObjectProperty => Entity::ObjectProperty(iri.clone()),
            EntityType::DataProperty => Entity::DataProperty(iri.clone()),
            EntityType::NamedIndividual => Entity::NamedIndividual(iri.clone()),
            EntityType::Datatype => Entity::Datatype(iri.clone()),
            EntityType::AnnotationProperty => Entity::AnnotationProperty(iri.clone()),
        }
    }

    // ── Literal creation ─────────────────────────────────────────────────

    /// Create a plain string literal.
    #[must_use]
    pub fn get_string_literal(&self, value: &str) -> Literal {
        Literal::new(value.to_string())
    }

    /// Create a boolean literal.
    #[must_use]
    pub fn get_boolean_literal(&self, value: bool) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::BOOLEAN),
        )
    }

    /// Create an integer literal.
    #[must_use]
    pub fn get_integer_literal(&self, value: i64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::INTEGER),
        )
    }

    /// Create a double literal.
    #[must_use]
    pub fn get_double_literal(&self, value: f64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::DOUBLE),
        )
    }

    /// Create a float literal typed with xsd:float.
    #[must_use]
    pub fn get_float_literal(&self, value: f32) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::FLOAT),
        )
    }

    /// Create a long literal typed with xsd:long.
    #[must_use]
    pub fn get_long_literal(&self, value: i64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::LONG),
        )
    }

    /// Create a short literal typed with xsd:short.
    #[must_use]
    pub fn get_short_literal(&self, value: i16) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::SHORT),
        )
    }

    /// Create a byte literal typed with xsd:byte.
    #[must_use]
    pub fn get_byte_literal(&self, value: i8) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::BYTE),
        )
    }

    /// Create an unsigned byte literal typed with xsd:unsignedByte.
    #[must_use]
    pub fn get_unsigned_byte_literal(&self, value: u8) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::UNSIGNED_BYTE),
        )
    }

    /// Create an unsigned short literal typed with xsd:unsignedShort.
    #[must_use]
    pub fn get_unsigned_short_literal(&self, value: u16) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::UNSIGNED_SHORT),
        )
    }

    /// Create an unsigned int literal typed with xsd:unsignedInt.
    #[must_use]
    pub fn get_unsigned_int_literal(&self, value: u32) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::UNSIGNED_INT),
        )
    }

    /// Create an unsigned long literal typed with xsd:unsignedLong.
    #[must_use]
    pub fn get_unsigned_long_literal(&self, value: u64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::UNSIGNED_LONG),
        )
    }

    /// Create a positive integer literal typed with xsd:positiveInteger.
    #[must_use]
    pub fn get_positive_integer_literal(&self, value: u64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::POSITIVE_INTEGER),
        )
    }

    /// Create a negative integer literal typed with xsd:negativeInteger.
    #[must_use]
    pub fn get_negative_integer_literal(&self, value: i64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::NEGATIVE_INTEGER),
        )
    }

    /// Create a non-negative integer literal typed with xsd:nonNegativeInteger.
    #[must_use]
    pub fn get_non_negative_integer_literal(&self, value: u64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::NON_NEGATIVE_INTEGER),
        )
    }

    /// Create a non-positive integer literal typed with xsd:nonPositiveInteger.
    #[must_use]
    pub fn get_non_positive_integer_literal(&self, value: i64) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::NON_POSITIVE_INTEGER),
        )
    }

    /// Create an owl:real literal.
    #[must_use]
    pub fn get_owl_real_literal(&self, value: &str) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::owl::REAL),
        )
    }

    /// Create an owl:rational literal.
    #[must_use]
    pub fn get_owl_rational_literal(&self, value: &str) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::owl::RATIONAL),
        )
    }

    /// Create a dateTime literal typed with xsd:dateTime.
    #[must_use]
    pub fn get_date_time_literal(&self, value: &str) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::DATE_TIME),
        )
    }

    /// Create a date literal typed with xsd:date.
    #[must_use]
    pub fn get_date_literal(&self, value: &str) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::DATE),
        )
    }

    /// Create a time literal typed with xsd:time.
    #[must_use]
    pub fn get_time_literal(&self, value: &str) -> Literal {
        Literal::with_datatype(
            value.to_string(),
            IRI::new(crate::ontology::vocabulary::xsd::TIME),
        )
    }

    /// Create a typed literal (lexical form + datatype IRI).
    #[must_use]
    pub fn get_typed_literal(&self, value: &str, datatype: &IRI) -> Literal {
        Literal::with_datatype(value.to_string(), datatype.clone())
    }

    /// Create a language-tagged literal.
    #[must_use]
    pub fn get_lang_literal(&self, value: &str, lang: &str) -> Literal {
        Literal::with_language(value.to_string(), lang.to_string())
    }

    // ── Annotation creation ──────────────────────────────────────────────

    /// Create an annotation with an IRI value.
    #[must_use]
    pub fn get_annotation(&self, property: AnnotationProperty, iri_value: IRI) -> Annotation {
        Annotation {
            property,
            value: AnnotationValue::IRI(iri_value),
            annotations: Vec::new(),
        }
    }

    /// Create an annotation with an IRI value and nested annotations.
    #[must_use]
    pub fn get_annotation_with_annotations(
        &self,
        property: AnnotationProperty,
        iri_value: IRI,
        annotations: Vec<Annotation>,
    ) -> Annotation {
        Annotation {
            property,
            value: AnnotationValue::IRI(iri_value),
            annotations,
        }
    }

    /// Create an annotation with a literal value.
    #[must_use]
    pub fn get_annotation_literal(
        &self,
        property: AnnotationProperty,
        literal: Literal,
    ) -> Annotation {
        Annotation {
            property,
            value: AnnotationValue::Literal(literal),
            annotations: Vec::new(),
        }
    }

    /// Create an annotation with a literal value and nested annotations.
    #[must_use]
    pub fn get_annotation_literal_with_annotations(
        &self,
        property: AnnotationProperty,
        literal: Literal,
        annotations: Vec<Annotation>,
    ) -> Annotation {
        Annotation {
            property,
            value: AnnotationValue::Literal(literal),
            annotations,
        }
    }

    // ── Class expression builders ────────────────────────────────────────

    /// Create an intersection class expression.
    #[must_use]
    pub fn get_object_intersection_of(&self, operands: Vec<ClassExpression>) -> ClassExpression {
        ClassExpression::ObjectIntersectionOf(operands)
    }

    /// Create a union class expression.
    #[must_use]
    pub fn get_object_union_of(&self, operands: Vec<ClassExpression>) -> ClassExpression {
        ClassExpression::ObjectUnionOf(operands)
    }

    /// Create a complement class expression.
    #[must_use]
    pub fn get_object_complement_of(&self, operand: ClassExpression) -> ClassExpression {
        ClassExpression::ObjectComplementOf(Box::new(operand))
    }

    /// Create a one-of (enumeration) class expression.
    #[must_use]
    pub fn get_object_one_of(&self, individuals: Vec<Individual>) -> ClassExpression {
        ClassExpression::ObjectOneOf(individuals)
    }

    /// Create an existential restriction.
    #[must_use]
    pub fn get_object_some_values_from(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    /// Create a universal restriction.
    #[must_use]
    pub fn get_object_all_values_from(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectAllValuesFrom {
            property,
            filler: Box::new(filler),
        }
    }

    /// Create a has-value restriction.
    #[must_use]
    pub fn get_object_has_value(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        value: Individual,
    ) -> ClassExpression {
        ClassExpression::ObjectHasValue { property, value }
    }

    /// Create a has-self restriction.
    #[must_use]
    pub fn get_object_has_self(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectHasSelf { property }
    }

    /// Create a min-cardinality restriction.
    #[must_use]
    pub fn get_object_min_cardinality(
        &self,
        cardinality: u32,
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectMinCardinality {
            property,
            cardinality,
            filler: Box::new(filler),
        }
    }

    /// Create a max-cardinality restriction.
    #[must_use]
    pub fn get_object_max_cardinality(
        &self,
        cardinality: u32,
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectMaxCardinality {
            property,
            cardinality,
            filler: Box::new(filler),
        }
    }

    /// Create an exact-cardinality restriction.
    #[must_use]
    pub fn get_object_exact_cardinality(
        &self,
        cardinality: u32,
        property: crate::ontology::ObjectPropertyExpression,
        filler: ClassExpression,
    ) -> ClassExpression {
        ClassExpression::ObjectExactCardinality {
            property,
            cardinality,
            filler: Box::new(filler),
        }
    }

    /// Create a data some-values-from restriction.
    #[must_use]
    pub fn get_data_some_values_from(
        &self,
        property: crate::ontology::DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataSomeValuesFrom { property, filler }
    }

    /// Create a data all-values-from restriction.
    #[must_use]
    pub fn get_data_all_values_from(
        &self,
        property: crate::ontology::DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataAllValuesFrom { property, filler }
    }

    /// Create a data has-value restriction.
    #[must_use]
    pub fn get_data_has_value(
        &self,
        property: crate::ontology::DataPropertyExpression,
        value: Literal,
    ) -> ClassExpression {
        ClassExpression::DataHasValue { property, value }
    }

    /// Create a data min-cardinality restriction.
    #[must_use]
    pub fn get_data_min_cardinality(
        &self,
        cardinality: u32,
        property: crate::ontology::DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataMinCardinality {
            property,
            cardinality,
            filler,
        }
    }

    /// Create a data max-cardinality restriction.
    #[must_use]
    pub fn get_data_max_cardinality(
        &self,
        cardinality: u32,
        property: crate::ontology::DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataMaxCardinality {
            property,
            cardinality,
            filler,
        }
    }

    /// Create a data exact-cardinality restriction.
    #[must_use]
    pub fn get_data_exact_cardinality(
        &self,
        cardinality: u32,
        property: crate::ontology::DataPropertyExpression,
        filler: DataRange,
    ) -> ClassExpression {
        ClassExpression::DataExactCardinality {
            property,
            cardinality,
            filler,
        }
    }
    // ── Axiom Creation ───────────────────────────────────────────────────

    /// Create a declaration axiom.
    #[must_use]
    pub fn get_declaration_axiom(&self, entity: Entity) -> crate::ontology::axioms::DeclarationAxiom {
        crate::ontology::axioms::DeclarationAxiom {
            id: self.next_id(),
            entity,
        }
    }

    /// Create a SubClassOf axiom.
    #[must_use]
    pub fn get_sub_class_of_axiom(
        &self,
        subclass: ClassExpression,
        superclass: ClassExpression,
    ) -> crate::ontology::axioms::SubClassOfAxiom {
        crate::ontology::axioms::SubClassOfAxiom {
            id: self.next_id(),
            subclass,
            superclass,
            annotations: vec![],
        }
    }

    /// Create an EquivalentClasses axiom.
    #[must_use]
    pub fn get_equivalent_classes_axiom(
        &self,
        classes: Vec<ClassExpression>,
    ) -> crate::ontology::axioms::EquivalentClassesAxiom {
        crate::ontology::axioms::EquivalentClassesAxiom {
            id: self.next_id(),
            classes,
            annotations: vec![],
        }
    }

    /// Create a DisjointClasses axiom.
    #[must_use]
    pub fn get_disjoint_classes_axiom(
        &self,
        classes: Vec<ClassExpression>,
    ) -> crate::ontology::axioms::DisjointClassesAxiom {
        crate::ontology::axioms::DisjointClassesAxiom {
            id: self.next_id(),
            classes,
            annotations: vec![],
        }
    }

    /// Create a DisjointUnion axiom.
    #[must_use]
    pub fn get_disjoint_union_axiom(
        &self,
        class: ClassExpression,
        disjoint_classes: Vec<ClassExpression>,
    ) -> crate::ontology::axioms::DisjointUnionAxiom {
        crate::ontology::axioms::DisjointUnionAxiom {
            id: self.next_id(),
            class,
            disjoint_classes,
            annotations: vec![],
        }
    }

    /// Create a SubObjectPropertyOf axiom.
    #[must_use]
    pub fn get_sub_object_property_of_axiom(
        &self,
        sub_property: crate::ontology::ObjectPropertyExpression,
        super_property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::SubObjectPropertyOfAxiom {
        crate::ontology::axioms::SubObjectPropertyOfAxiom {
            id: self.next_id(),
            sub_property,
            super_property,
            annotations: vec![],
        }
    }

    /// Create a SubDataPropertyOf axiom.
    #[must_use]
    pub fn get_sub_data_property_of_axiom(
        &self,
        sub_property: crate::ontology::DataPropertyExpression,
        super_property: crate::ontology::DataPropertyExpression,
    ) -> crate::ontology::axioms::SubDataPropertyOfAxiom {
        crate::ontology::axioms::SubDataPropertyOfAxiom {
            id: self.next_id(),
            sub_property,
            super_property,
            annotations: vec![],
        }
    }

    /// Create a ClassAssertion axiom.
    #[must_use]
    pub fn get_class_assertion_axiom(
        &self,
        class: ClassExpression,
        individual: Individual,
    ) -> crate::ontology::axioms::ClassAssertionAxiom {
        crate::ontology::axioms::ClassAssertionAxiom {
            id: self.next_id(),
            class,
            individual,
            annotations: vec![],
        }
    }

    /// Create an ObjectPropertyAssertion axiom.
    #[must_use]
    pub fn get_object_property_assertion_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        source: Individual,
        target: Individual,
    ) -> crate::ontology::axioms::ObjectPropertyAssertionAxiom {
        crate::ontology::axioms::ObjectPropertyAssertionAxiom {
            id: self.next_id(),
            source,
            target,
            property,
            annotations: vec![],
        }
    }

    /// Create a DataPropertyAssertion axiom.
    #[must_use]
    pub fn get_data_property_assertion_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        individual: Individual,
        value: Literal,
    ) -> crate::ontology::axioms::DataPropertyAssertionAxiom {
        crate::ontology::axioms::DataPropertyAssertionAxiom {
            id: self.next_id(),
            individual,
            property,
            value,
            annotations: vec![],
        }
    }

    /// Create a SameIndividual axiom.
    #[must_use]
    pub fn get_same_individual_axiom(
        &self,
        individuals: Vec<Individual>,
    ) -> crate::ontology::axioms::SameIndividualAxiom {
        crate::ontology::axioms::SameIndividualAxiom {
            id: self.next_id(),
            individuals,
            annotations: vec![],
        }
    }

    /// Create a DifferentIndividuals axiom.
    #[must_use]
    pub fn get_different_individuals_axiom(
        &self,
        individuals: Vec<Individual>,
    ) -> crate::ontology::axioms::DifferentIndividualsAxiom {
        crate::ontology::axioms::DifferentIndividualsAxiom {
            id: self.next_id(),
            individuals,
            annotations: vec![],
        }
    }

    /// Create a FunctionalObjectProperty axiom.
    #[must_use]
    pub fn get_functional_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::FunctionalObjectPropertyAxiom {
        crate::ontology::axioms::FunctionalObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create a FunctionalDataProperty axiom.
    #[must_use]
    pub fn get_functional_data_property_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
    ) -> crate::ontology::axioms::FunctionalDataPropertyAxiom {
        crate::ontology::axioms::FunctionalDataPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create a TransitiveObjectProperty axiom.
    #[must_use]
    pub fn get_transitive_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::TransitiveObjectPropertyAxiom {
        crate::ontology::axioms::TransitiveObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create a SymmetricObjectProperty axiom.
    #[must_use]
    pub fn get_symmetric_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::SymmetricObjectPropertyAxiom {
        crate::ontology::axioms::SymmetricObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create an AsymmetricObjectProperty axiom.
    #[must_use]
    pub fn get_asymmetric_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::AsymmetricObjectPropertyAxiom {
        crate::ontology::axioms::AsymmetricObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create a ReflexiveObjectProperty axiom.
    #[must_use]
    pub fn get_reflexive_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::ReflexiveObjectPropertyAxiom {
        crate::ontology::axioms::ReflexiveObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create an IrreflexiveObjectProperty axiom.
    #[must_use]
    pub fn get_irreflexive_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::IrreflexiveObjectPropertyAxiom {
        crate::ontology::axioms::IrreflexiveObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create an InverseFunctionalObjectProperty axiom.
    #[must_use]
    pub fn get_inverse_functional_object_property_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::InverseFunctionalObjectPropertyAxiom {
        crate::ontology::axioms::InverseFunctionalObjectPropertyAxiom {
            id: self.next_id(),
            property,
            annotations: vec![],
        }
    }

    /// Create an InverseObjectProperties axiom.
    #[must_use]
    pub fn get_inverse_object_properties_axiom(
        &self,
        property1: crate::ontology::ObjectPropertyExpression,
        property2: crate::ontology::ObjectPropertyExpression,
    ) -> crate::ontology::axioms::InverseObjectPropertiesAxiom {
        crate::ontology::axioms::InverseObjectPropertiesAxiom {
            id: self.next_id(),
            property1,
            property2,
            annotations: vec![],
        }
    }

    /// Create an ObjectPropertyDomain axiom.
    #[must_use]
    pub fn get_object_property_domain_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        domain: ClassExpression,
    ) -> crate::ontology::axioms::ObjectPropertyDomainAxiom {
        crate::ontology::axioms::ObjectPropertyDomainAxiom {
            id: self.next_id(),
            property,
            domain,
            annotations: vec![],
        }
    }

    /// Create an ObjectPropertyRange axiom.
    #[must_use]
    pub fn get_object_property_range_axiom(
        &self,
        property: crate::ontology::ObjectPropertyExpression,
        range: ClassExpression,
    ) -> crate::ontology::axioms::ObjectPropertyRangeAxiom {
        crate::ontology::axioms::ObjectPropertyRangeAxiom {
            id: self.next_id(),
            property,
            range,
            annotations: vec![],
        }
    }

    /// Create a DataPropertyDomain axiom.
    #[must_use]
    pub fn get_data_property_domain_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        domain: ClassExpression,
    ) -> crate::ontology::axioms::DataPropertyDomainAxiom {
        crate::ontology::axioms::DataPropertyDomainAxiom {
            id: self.next_id(),
            property,
            domain,
            annotations: vec![],
        }
    }

    /// Create a DataPropertyRange axiom.
    #[must_use]
    pub fn get_data_property_range_axiom(
        &self,
        property: crate::ontology::DataPropertyExpression,
        range: DataRange,
    ) -> crate::ontology::axioms::DataPropertyRangeAxiom {
        crate::ontology::axioms::DataPropertyRangeAxiom {
            id: self.next_id(),
            property,
            range,
            annotations: vec![],
        }
    }
}

impl Default for DataFactory {
    fn default() -> Self {
        Self::new()
    }
}

// ── Provider Trait Implementations ───────────────────────────────────────────

impl providers::ClassProvider for DataFactory {
    fn get_class(&self, iri: &IRI) -> Class {
        self.get_class(iri)
    }
}

impl providers::ObjectPropertyProvider for DataFactory {
    fn get_object_property(&self, iri: &IRI) -> ObjectProperty {
        self.get_object_property(iri)
    }
}

impl providers::DataPropertyProvider for DataFactory {
    fn get_data_property(&self, iri: &IRI) -> DataProperty {
        self.get_data_property(iri)
    }
}

impl providers::IndividualProvider for DataFactory {
    fn get_named_individual(&self, iri: &IRI) -> NamedIndividual {
        self.get_named_individual(iri)
    }

    fn get_anonymous_individual(&self) -> AnonymousIndividual {
        self.get_anonymous_individual()
    }
}

impl providers::DatatypeProvider for DataFactory {
    fn get_datatype(&self, iri: &IRI) -> Datatype {
        self.get_owl_datatype(iri)
    }
}

impl providers::AnnotationPropertyProvider for DataFactory {
    fn get_annotation_property(&self, iri: &IRI) -> AnnotationProperty {
        self.get_annotation_property(iri)
    }
}

impl AxiomCreationProvider for DataFactory {
    fn next_axiom_id(&self) -> AxiomId {
        self.next_axiom_id.fetch_add(1, Ordering::Relaxed)
    }
}

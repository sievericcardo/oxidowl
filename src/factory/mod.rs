//! OWL DataFactory — unified entity/axiom/expression builder.
//!
//! Provides entity interning (deduplication by IRI), consistent ID
//! generation for axioms, and builder methods for all OWL 2 constructs.

pub mod providers;

use self::providers::AxiomCreationProvider;
use crate::ontology::{
    AnnotationProperty, AnonymousIndividual, Class, DataProperty, DataRange, ObjectProperty, IRI,
};
use crate::ontology::axioms::{
    AxiomId, Entity, EntityType,
};
use crate::ontology::concepts::ClassExpression;
use crate::ontology::individuals::{Individual, NamedIndividual};
use crate::ontology::{Annotation, AnnotationValue, Literal};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

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
            self.class_cache.read().unwrap_or_else(|e| e.into_inner()).len(),
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
        s
    }

    // ── Entity creation (with interning) ─────────────────────────────────

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
        let cache = self.object_property_cache.read().unwrap_or_else(|e| e.into_inner());
        if let Some(prop) = cache.get(iri) {
            return prop.clone();
        }
        drop(cache);
        let mut cache = self.object_property_cache.write().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| ObjectProperty { iri: iri.clone() })
            .clone()
    }

    /// Get or create a data property for the given IRI.
    pub fn get_data_property(&self, iri: &IRI) -> DataProperty {
        let cache = self.data_property_cache.read().unwrap_or_else(|e| e.into_inner());
        if let Some(prop) = cache.get(iri) {
            return prop.clone();
        }
        drop(cache);
        let mut cache = self.data_property_cache.write().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| DataProperty { iri: iri.clone() })
            .clone()
    }

    /// Get or create a named individual for the given IRI.
    pub fn get_named_individual(&self, iri: &IRI) -> NamedIndividual {
        let cache = self.individual_cache.read().unwrap_or_else(|e| e.into_inner());
        if let Some(ind) = cache.get(iri) {
            return ind.clone();
        }
        drop(cache);
        let mut cache = self.individual_cache.write().unwrap_or_else(|e| e.into_inner());
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
        let cache = self.annotation_property_cache.read().unwrap_or_else(|e| e.into_inner());
        if let Some(prop) = cache.get(iri) {
            return prop.clone();
        }
        drop(cache);
        let mut cache = self.annotation_property_cache.write().unwrap_or_else(|e| e.into_inner());
        cache
            .entry(iri.clone())
            .or_insert_with(|| AnnotationProperty { iri: iri.clone() })
            .clone()
    }

    /// Get a datatype data range for the given IRI.
    #[must_use]
    pub fn get_datatype(&self, iri: &IRI) -> DataRange {
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
        Literal::new(value.to_string())
    }

    /// Create an integer literal.
    #[must_use]
    pub fn get_integer_literal(&self, value: i64) -> Literal {
        Literal::new(value.to_string())
    }

    /// Create a double literal.
    #[must_use]
    pub fn get_double_literal(&self, value: f64) -> Literal {
        Literal::new(value.to_string())
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
    fn get_datatype(&self, iri: &IRI) -> DataRange {
        self.get_datatype(iri)
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

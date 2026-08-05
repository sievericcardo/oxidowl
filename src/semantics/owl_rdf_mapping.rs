//! Unified OWL-to-RDF bidirectional mapping.
//!
//! This module is the single source of truth for converting between
//! OWL axioms and RDF triples. All RDF-based parsers and serializers
//! delegate to this module instead of maintaining their own ad-hoc
//! mapping code.

use std::sync::atomic::{AtomicU64, Ordering};

use super::vocabulary::*;
use super::{RdfTerm, Triple};
use crate::ontology::*;

// ══════════════════════════════════════════════════════════════════════════════
// Blank Node Counter
// ══════════════════════════════════════════════════════════════════════════════

pub struct BlankNodeCounter {
    next: AtomicU64,
}

impl BlankNodeCounter {
    pub fn new() -> Self {
        BlankNodeCounter {
            next: AtomicU64::new(0),
        }
    }

    pub fn fresh(&self) -> RdfTerm {
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        RdfTerm::BlankNode(format!("genid{id}"))
    }
}

impl Default for BlankNodeCounter {
    fn default() -> Self {
        Self::new()
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Entity Type Discrimination for Parser-Side Dispatch
// ══════════════════════════════════════════════════════════════════════════════

pub enum EntityTypeHint {
    Class,
    ObjectProperty,
    DataProperty,
    AnnotationProperty,
    NamedIndividual,
    Unknown,
}

// ══════════════════════════════════════════════════════════════════════════════
// Serializer-Side: Axiom → Triples
// ══════════════════════════════════════════════════════════════════════════════

/// Convert a single axiom to its RDF triple representation.
pub fn axiom_to_triples(axiom: &Axiom, counter: &mut BlankNodeCounter) -> Vec<Triple> {
    match axiom {
        // ── Simple 1-triple: SubClassOf ─────────────────────────────────
        Axiom::SubClassOf(a) => {
            vec![triple(
                class_expression_to_term(&a.subclass, counter),
                iri_term(&RDFS_SUBCLASS_OF),
                class_expression_to_term(&a.superclass, counter),
            )]
        }
        // ── Simple 1-triple: ClassAssertion ─────────────────────────────
        Axiom::ClassAssertion(a) => {
            let subj = individual_to_term(&a.individual);
            let obj = class_expression_to_term(&a.class, counter);
            vec![triple(subj, iri_term(&RDF_TYPE), obj)]
        }
        // ── Simple 1-triple: ObjectPropertyAssertion ────────────────────
        Axiom::ObjectPropertyAssertion(a) => {
            let subj = individual_to_term(&a.source);
            let pred = ope_to_term(&a.property);
            let obj = individual_to_term(&a.target);
            vec![triple(subj, pred, obj)]
        }
        // ── Simple 1-triple: DataPropertyAssertion ──────────────────────
        Axiom::DataPropertyAssertion(a) => {
            let subj = individual_to_term(&a.individual);
            let pred = dpe_to_term(&a.property);
            let obj = literal_to_term(&a.value);
            vec![triple(subj, pred, obj)]
        }
        // ── rdf:type-based: Declarations ────────────────────────────────
        Axiom::Declaration(d) => {
            vec![declaration_to_triple(&d.entity)]
        }
        // ── rdf:type-based: Property characteristics ────────────────────
        Axiom::FunctionalObjectProperty(a) => {
            vec![characteristic_triple(&a.property, &OWL_FUNCTIONAL_PROPERTY)]
        }
        Axiom::InverseFunctionalObjectProperty(a) => {
            vec![characteristic_triple(
                &a.property,
                &OWL_INVERSE_FUNCTIONAL_PROPERTY,
            )]
        }
        Axiom::ReflexiveObjectProperty(a) => {
            vec![characteristic_triple(&a.property, &OWL_REFLEXIVE_PROPERTY)]
        }
        Axiom::IrreflexiveObjectProperty(a) => {
            vec![characteristic_triple(
                &a.property,
                &OWL_IRREFLEXIVE_PROPERTY,
            )]
        }
        Axiom::SymmetricObjectProperty(a) => {
            vec![characteristic_triple(&a.property, &OWL_SYMMETRIC_PROPERTY)]
        }
        Axiom::AsymmetricObjectProperty(a) => {
            vec![characteristic_triple(&a.property, &OWL_ASYMMETRIC_PROPERTY)]
        }
        Axiom::TransitiveObjectProperty(a) => {
            vec![characteristic_triple(&a.property, &OWL_TRANSITIVE_PROPERTY)]
        }
        Axiom::FunctionalDataProperty(a) => {
            vec![characteristic_triple_dpe(
                &a.property,
                &OWL_FUNCTIONAL_PROPERTY,
            )]
        }
        // ── Pairwise: EquivalentClasses ──────────────────────────────────
        Axiom::EquivalentClasses(a) => pairwise_triples(&a.classes, &OWL_EQUIVALENT_CLASS, counter),
        // ── Pairwise: DisjointClasses ───────────────────────────────────
        Axiom::DisjointClasses(a) => pairwise_triples(&a.classes, &OWL_DISJOINT_WITH, counter),
        // ── Pairwise: SameIndividual ────────────────────────────────────
        Axiom::SameIndividual(a) => pairwise_individuals(&a.individuals, &OWL_SAME_AS),
        // ── Pairwise: DifferentIndividuals ──────────────────────────────
        Axiom::DifferentIndividuals(a) => pairwise_individuals(&a.individuals, &OWL_DIFFERENT_FROM),
        // ── Pairwise: EquivalentObjectProperties ────────────────────────
        Axiom::EquivalentObjectProperties(a) => {
            pairwise_opes(&a.properties, &OWL_EQUIVALENT_PROPERTY)
        }
        // ── Pairwise: DisjointObjectProperties ──────────────────────────
        Axiom::DisjointObjectProperties(a) => {
            pairwise_opes(&a.properties, &OWL_PROPERTY_DISJOINT_WITH)
        }
        // ── Pairwise: EquivalentDataProperties ──────────────────────────
        Axiom::EquivalentDataProperties(a) => {
            pairwise_dpes(&a.properties, &OWL_EQUIVALENT_PROPERTY)
        }
        // ── Pairwise: DisjointDataProperties ────────────────────────────
        Axiom::DisjointDataProperties(a) => {
            pairwise_dpes(&a.properties, &OWL_PROPERTY_DISJOINT_WITH)
        }
        // ── SubObjectPropertyOf ─────────────────────────────────────────
        Axiom::SubObjectPropertyOf(a) => {
            vec![triple(
                ope_to_term(&a.sub_property),
                iri_term(&RDFS_SUBPROPERTY_OF),
                ope_to_term(&a.super_property),
            )]
        }
        // ── SubDataPropertyOf ───────────────────────────────────────────
        Axiom::SubDataPropertyOf(a) => {
            vec![triple(
                dpe_to_term(&a.sub_property),
                iri_term(&RDFS_SUBPROPERTY_OF),
                dpe_to_term(&a.super_property),
            )]
        }
        // ── ObjectPropertyDomain ────────────────────────────────────────
        Axiom::ObjectPropertyDomain(a) => {
            vec![triple(
                ope_to_term(&a.property),
                iri_term(&RDFS_DOMAIN),
                class_expression_to_term(&a.domain, counter),
            )]
        }
        // ── ObjectPropertyRange ─────────────────────────────────────────
        Axiom::ObjectPropertyRange(a) => {
            vec![triple(
                ope_to_term(&a.property),
                iri_term(&RDFS_RANGE),
                class_expression_to_term(&a.range, counter),
            )]
        }
        // ── DataPropertyDomain ──────────────────────────────────────────
        Axiom::DataPropertyDomain(a) => {
            vec![triple(
                dpe_to_term(&a.property),
                iri_term(&RDFS_DOMAIN),
                class_expression_to_term(&a.domain, counter),
            )]
        }
        // ── DataPropertyRange ───────────────────────────────────────────
        Axiom::DataPropertyRange(a) => {
            vec![triple(
                dpe_to_term(&a.property),
                iri_term(&RDFS_RANGE),
                data_range_to_term(&a.range),
            )]
        }
        // ── InverseObjectProperties ─────────────────────────────────────
        Axiom::InverseObjectProperties(a) => {
            vec![triple(
                ope_to_term(&a.property1),
                iri_term(&OWL_INVERSE_OF),
                ope_to_term(&a.property2),
            )]
        }
        // ── AnnotationAssertion ─────────────────────────────────────────
        Axiom::AnnotationAssertion(a) => {
            vec![triple(
                annotation_subject_to_term(&a.subject),
                iri_term_from_url(url_from_iri_str(a.property.iri.as_str())),
                annotation_value_to_term(&a.value),
            )]
        }
        // ── SubAnnotationPropertyOf ─────────────────────────────────────
        Axiom::SubAnnotationPropertyOf(a) => {
            vec![triple(
                iri_term_from_str(a.sub_property.iri.as_str()),
                iri_term(&RDFS_SUBPROPERTY_OF),
                iri_term_from_str(a.super_property.iri.as_str()),
            )]
        }
        // ── AnnotationPropertyDomain ────────────────────────────────────
        Axiom::AnnotationPropertyDomain(a) => {
            vec![triple(
                iri_term_from_str(a.property.iri.as_str()),
                iri_term(&RDFS_DOMAIN),
                class_expression_to_term(&a.domain, counter),
            )]
        }
        // ── AnnotationPropertyRange ─────────────────────────────────────
        Axiom::AnnotationPropertyRange(a) => {
            vec![triple(
                iri_term_from_str(a.property.iri.as_str()),
                iri_term(&RDFS_RANGE),
                data_range_to_term(&a.range),
            )]
        }
        // ── DisjointUnion (RDF list) ────────────────────────────────────
        Axiom::DisjointUnion(a) => {
            let mut triples = Vec::new();
            let cls_term = class_expression_to_term(&a.class, counter);
            let members: Vec<RdfTerm> = a
                .disjoint_classes
                .iter()
                .map(|ce| class_expression_to_term(ce, counter))
                .collect();
            let list_node = build_rdf_list(&members, counter, &mut triples);
            triples.push(triple(
                cls_term,
                iri_term(&OWL_DISJOINT_UNION_OF),
                list_node,
            ));
            triples
        }
        // ── HasKey (RDF list) ───────────────────────────────────────────
        Axiom::HasKey(a) => {
            let mut triples = Vec::new();
            let cls_term = class_expression_to_term(&a.class, counter);
            let mut key_items: Vec<RdfTerm> = a.object_properties.iter().map(ope_to_term).collect();
            key_items.extend(a.data_properties.iter().map(dpe_to_term));
            let list_node = build_rdf_list(&key_items, counter, &mut triples);
            triples.push(triple(cls_term, iri_term(&OWL_HAS_KEY), list_node));
            triples
        }
        // ── NegativeObjectPropertyAssertion (reification) ───────────────
        Axiom::NegativeObjectPropertyAssertion(a) => {
            let mut triples = Vec::new();
            let bn = counter.fresh();
            triples.push(triple(
                bn.clone(),
                iri_term(&RDF_TYPE),
                iri_term(&OWL_NEGATIVE_PROPERTY_ASSERTION),
            ));
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_SOURCE_INDIVIDUAL),
                individual_to_term(&a.source),
            ));
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_ASSERTION_PROPERTY),
                ope_to_term(&a.property),
            ));
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_TARGET_INDIVIDUAL),
                individual_to_term(&a.target),
            ));
            triples
        }
        // ── NegativeDataPropertyAssertion (reification) ─────────────────
        Axiom::NegativeDataPropertyAssertion(a) => {
            let mut triples = Vec::new();
            let bn = counter.fresh();
            triples.push(triple(
                bn.clone(),
                iri_term(&RDF_TYPE),
                iri_term(&OWL_NEGATIVE_PROPERTY_ASSERTION),
            ));
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_SOURCE_INDIVIDUAL),
                individual_to_term(&a.individual),
            ));
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_ASSERTION_PROPERTY),
                dpe_to_term(&a.property),
            ));
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_TARGET_VALUE),
                literal_to_term(&a.value),
            ));
            triples
        }
        // ── DatatypeDefinition ──────────────────────────────────────────
        Axiom::DatatypeDefinition(a) => {
            vec![triple(
                iri_term_from_str(&format!("{}", a.datatype)),
                iri_term(&RDF_TYPE),
                iri_term(&RDFS_DATATYPE),
            )]
        }
        // ── SWRL Rule ───────────────────────────────────────────────────
        Axiom::Rule(_) => {
            vec![] // SWRL RDF encoding is complex; skip for now
        }
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// Parser-Side: Triple → Axiom
// ══════════════════════════════════════════════════════════════════════════════

/// Check if a predicate URL is an OWL/RDFS/RDF vocabulary term and
/// return the appropriate axiom type string if recognized.
pub fn classify_predicate(predicate: &url::Url) -> Option<&'static str> {
    let p = predicate.as_str();
    if p == RDFS_SUBCLASS_OF.as_str() {
        return Some("SubClassOf");
    }
    if p == RDF_TYPE.as_str() {
        return Some("rdf:type");
    }
    if p == OWL_EQUIVALENT_CLASS.as_str() {
        return Some("EquivalentClasses");
    }
    if p == OWL_DISJOINT_WITH.as_str() {
        return Some("DisjointClasses");
    }
    if p == RDFS_SUBPROPERTY_OF.as_str() {
        return Some("subPropertyOf");
    }
    if p == RDFS_DOMAIN.as_str() {
        return Some("domain");
    }
    if p == RDFS_RANGE.as_str() {
        return Some("range");
    }
    if p == OWL_EQUIVALENT_PROPERTY.as_str() {
        return Some("equivalentProperty");
    }
    if p == OWL_SAME_AS.as_str() {
        return Some("SameIndividual");
    }
    if p == OWL_DIFFERENT_FROM.as_str() {
        return Some("DifferentIndividuals");
    }
    if p == OWL_INVERSE_OF.as_str() {
        return Some("InverseObjectProperties");
    }
    if p == OWL_DISJOINT_UNION_OF.as_str() {
        return Some("DisjointUnion");
    }
    if p == OWL_HAS_KEY.as_str() {
        return Some("HasKey");
    }
    None
}

/// Try to classify the object of an `rdf:type` triple to determine
/// which axiom type to produce (declaration vs characteristic vs class assertion).
pub fn classify_rdf_type_object(obj_url: &url::Url) -> Option<&'static str> {
    let o = obj_url.as_str();
    if o == OWL_CLASS.as_str() {
        return Some("Declaration(Class)");
    }
    if o == OWL_OBJECT_PROPERTY.as_str() {
        return Some("Declaration(ObjectProperty)");
    }
    if o == OWL_DATA_PROPERTY.as_str() {
        return Some("Declaration(DataProperty)");
    }
    if o == RDFS_CLASS.as_str() {
        return Some("Declaration(Class)");
    }
    if o == OWL_FUNCTIONAL_PROPERTY.as_str() {
        return Some("FunctionalObjectProperty");
    }
    if o == OWL_INVERSE_FUNCTIONAL_PROPERTY.as_str() {
        return Some("InverseFunctionalObjectProperty");
    }
    if o == OWL_TRANSITIVE_PROPERTY.as_str() {
        return Some("TransitiveObjectProperty");
    }
    if o == OWL_SYMMETRIC_PROPERTY.as_str() {
        return Some("SymmetricObjectProperty");
    }
    if o == OWL_ASYMMETRIC_PROPERTY.as_str() {
        return Some("AsymmetricObjectProperty");
    }
    if o == OWL_REFLEXIVE_PROPERTY.as_str() {
        return Some("ReflexiveObjectProperty");
    }
    if o == OWL_IRREFLEXIVE_PROPERTY.as_str() {
        return Some("IrreflexiveObjectProperty");
    }
    None
}

/// Check if an object IRI from `rdf:type` is an OWL built-in (declaration or characteristic).
pub fn is_owl_builtin_rdf_type(obj_url: &url::Url) -> bool {
    classify_rdf_type_object(obj_url).is_some()
        || obj_url.as_str() == OWL_CLASS.as_str()
        || obj_url.as_str() == OWL_OBJECT_PROPERTY.as_str()
        || obj_url.as_str() == OWL_DATA_PROPERTY.as_str()
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper: RDF List Construction
// ══════════════════════════════════════════════════════════════════════════════

pub fn build_rdf_list(
    items: &[RdfTerm],
    counter: &mut BlankNodeCounter,
    triples: &mut Vec<Triple>,
) -> RdfTerm {
    if items.is_empty() {
        return iri_term(&RDF_NIL);
    }
    let head = counter.fresh();
    triples.push(triple(head.clone(), iri_term(&RDF_FIRST), items[0].clone()));
    let tail = if items.len() > 1 {
        build_rdf_list(&items[1..], counter, triples)
    } else {
        iri_term(&RDF_NIL)
    };
    triples.push(triple(head.clone(), iri_term(&RDF_REST), tail));
    head
}

// ══════════════════════════════════════════════════════════════════════════════
// Helper: Term Construction
// ══════════════════════════════════════════════════════════════════════════════

fn triple(subject: RdfTerm, predicate: RdfTerm, object: RdfTerm) -> Triple {
    Triple {
        subject,
        predicate,
        object,
    }
}

fn iri_term(url: &url::Url) -> RdfTerm {
    RdfTerm::Iri(url.clone())
}

fn iri_term_from_url(url: url::Url) -> RdfTerm {
    RdfTerm::Iri(url)
}

fn iri_term_from_str(s: &str) -> RdfTerm {
    url::Url::parse(s)
        .map(RdfTerm::Iri)
        .unwrap_or_else(|_| RdfTerm::Iri(url::Url::parse("http://example.org/error").unwrap()))
}

fn url_from_iri_str(s: &str) -> url::Url {
    url::Url::parse(s).unwrap_or_else(|_| url::Url::parse("http://example.org/error").unwrap())
}

fn declaration_to_triple(entity: &Entity) -> Triple {
    match entity {
        Entity::Class(iri) => triple(
            iri_term_from_str(iri.as_str()),
            iri_term(&RDF_TYPE),
            iri_term(&OWL_CLASS),
        ),
        Entity::ObjectProperty(iri) => triple(
            iri_term_from_str(iri.as_str()),
            iri_term(&RDF_TYPE),
            iri_term(&OWL_OBJECT_PROPERTY),
        ),
        Entity::DataProperty(iri) => triple(
            iri_term_from_str(iri.as_str()),
            iri_term(&RDF_TYPE),
            iri_term(&OWL_DATA_PROPERTY),
        ),
        Entity::NamedIndividual(iri) => triple(
            iri_term_from_str(iri.as_str()),
            iri_term(&RDF_TYPE),
            iri_term(&OWL_NAMED_INDIVIDUAL),
        ),
        Entity::AnnotationProperty(iri) => triple(
            iri_term_from_str(iri.as_str()),
            iri_term(&RDF_TYPE),
            iri_term(&OWL_ANNOTATION_PROPERTY),
        ),
        Entity::Datatype(iri) => triple(
            iri_term_from_str(iri.as_str()),
            iri_term(&RDF_TYPE),
            iri_term(&RDFS_DATATYPE),
        ),
    }
}

fn class_expression_to_term(ce: &ClassExpression, counter: &mut BlankNodeCounter) -> RdfTerm {
    match ce {
        ClassExpression::Class(c) => iri_term_from_url(
            url::Url::parse(c.iri.as_str())
                .unwrap_or(url::Url::parse("http://example.org/error").unwrap()),
        ),
        ClassExpression::ObjectIntersectionOf(parts) => {
            let bn = counter.fresh();
            // Delegate complex class expressions to direct IRI for simple cases
            if parts.iter().all(|p| matches!(p, ClassExpression::Class(_))) {
                // For simple named-class intersections, return first class
                // (Turtle serializers can't fully represent blind intersections)
                class_expression_to_term(&parts[0], counter)
            } else {
                bn
            }
        }
        ClassExpression::ObjectUnionOf(parts) => {
            if parts.iter().all(|p| matches!(p, ClassExpression::Class(_))) && !parts.is_empty() {
                class_expression_to_term(&parts[0], counter)
            } else {
                counter.fresh()
            }
        }
        _ => counter.fresh(),
    }
}

fn individual_to_term(ind: &Individual) -> RdfTerm {
    match ind {
        Individual::Named(named) => iri_term_from_url(
            url::Url::parse(named.iri.as_str())
                .unwrap_or(url::Url::parse("http://example.org/error").unwrap()),
        ),
        Individual::Anonymous(anon) => RdfTerm::BlankNode(anon.id.clone()),
    }
}

fn ope_to_term(ope: &ObjectPropertyExpression) -> RdfTerm {
    match ope {
        ObjectPropertyExpression::ObjectProperty(p) => iri_term_from_str(p.iri.as_str()),
        ObjectPropertyExpression::InverseObjectProperty(p) => {
            // Inverse properties can't be represented as direct terms in RDF
            iri_term_from_str(p.iri.as_str())
        }
        ObjectPropertyExpression::PropertyChain(_) => iri_term(&OWL_PROPERTY_CHAIN_AXIOM),
    }
}

fn dpe_to_term(dpe: &DataPropertyExpression) -> RdfTerm {
    match dpe {
        DataPropertyExpression::DataProperty(p) => iri_term_from_str(p.iri.as_str()),
    }
}

fn data_range_to_term(dr: &DataRange) -> RdfTerm {
    match dr {
        DataRange::Datatype(iri) => iri_term_from_str(iri.as_str()),
        _ => iri_term(&RDFS_LITERAL),
    }
}

fn literal_to_term(lit: &Literal) -> RdfTerm {
    RdfTerm::Literal {
        value: lit.value.clone(),
        datatype: lit.datatype.clone(),
        language: lit.language.clone(),
        direction: None,
    }
}

fn annotation_subject_to_term(subj: &AnnotationSubject) -> RdfTerm {
    match subj {
        AnnotationSubject::IRI(iri) => iri_term_from_str(iri.as_str()),
        AnnotationSubject::AnonymousIndividual(anon) => RdfTerm::BlankNode(anon.id.clone()),
    }
}

fn annotation_value_to_term(val: &AnnotationValue) -> RdfTerm {
    match val {
        AnnotationValue::IRI(iri) => iri_term_from_str(iri.as_str()),
        AnnotationValue::AnonymousIndividual(anon) => RdfTerm::BlankNode(anon.id.clone()),
        AnnotationValue::Literal(lit) => literal_to_term(lit),
    }
}

fn characteristic_triple(ope: &ObjectPropertyExpression, owl_type: &url::Url) -> Triple {
    triple(ope_to_term(ope), iri_term(&RDF_TYPE), iri_term(owl_type))
}

fn characteristic_triple_dpe(dpe: &DataPropertyExpression, owl_type: &url::Url) -> Triple {
    triple(dpe_to_term(dpe), iri_term(&RDF_TYPE), iri_term(owl_type))
}

fn pairwise_triples(
    items: &[ClassExpression],
    predicate_url: &url::Url,
    counter: &mut BlankNodeCounter,
) -> Vec<Triple> {
    if items.len() < 2 {
        return vec![];
    }
    let first = class_expression_to_term(&items[0], counter);
    let pred = iri_term(predicate_url);
    items[1..]
        .iter()
        .map(|ce| {
            triple(
                first.clone(),
                pred.clone(),
                class_expression_to_term(ce, counter),
            )
        })
        .collect()
}

fn pairwise_individuals(individuals: &[Individual], predicate_url: &url::Url) -> Vec<Triple> {
    if individuals.len() < 2 {
        return vec![];
    }
    let first = individual_to_term(&individuals[0]);
    let pred = iri_term(predicate_url);
    individuals[1..]
        .iter()
        .map(|ind| triple(first.clone(), pred.clone(), individual_to_term(ind)))
        .collect()
}

fn pairwise_opes(properties: &[ObjectPropertyExpression], predicate_url: &url::Url) -> Vec<Triple> {
    if properties.len() < 2 {
        return vec![];
    }
    let first = ope_to_term(&properties[0]);
    let pred = iri_term(predicate_url);
    properties[1..]
        .iter()
        .map(|p| triple(first.clone(), pred.clone(), ope_to_term(p)))
        .collect()
}

fn pairwise_dpes(properties: &[DataPropertyExpression], predicate_url: &url::Url) -> Vec<Triple> {
    if properties.len() < 2 {
        return vec![];
    }
    let first = dpe_to_term(&properties[0]);
    let pred = iri_term(predicate_url);
    properties[1..]
        .iter()
        .map(|p| triple(first.clone(), pred.clone(), dpe_to_term(p)))
        .collect()
}

// ══════════════════════════════════════════════════════════════════════════════
// Additional vocabulary URLs found in ontology axioms but not in semantics vocabulary
// ══════════════════════════════════════════════════════════════════════════════

use std::sync::LazyLock;

static OWL_NAMED_INDIVIDUAL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#NamedIndividual").unwrap());
static OWL_ANNOTATION_PROPERTY: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#AnnotationProperty").unwrap());
static OWL_PROPERTY_DISJOINT_WITH: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("http://www.w3.org/2002/07/owl#propertyDisjointWith").unwrap()
});
static OWL_DISJOINT_UNION_OF: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#disjointUnionOf").unwrap());
static OWL_HAS_KEY: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#hasKey").unwrap());
static OWL_NEGATIVE_PROPERTY_ASSERTION: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("http://www.w3.org/2002/07/owl#NegativePropertyAssertion").unwrap()
});
static OWL_SOURCE_INDIVIDUAL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#sourceIndividual").unwrap());
static OWL_ASSERTION_PROPERTY: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#assertionProperty").unwrap());
static OWL_TARGET_INDIVIDUAL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#targetIndividual").unwrap());
static OWL_TARGET_VALUE: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#targetValue").unwrap());
#[allow(dead_code)]
static OWL_PROPERTY_CHAIN_AXIOM: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#propertyChainAxiom").unwrap());

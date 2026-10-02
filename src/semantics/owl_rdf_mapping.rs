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
            let mut out = Vec::new();
            let subj = class_expression_to_term(&a.subclass, counter, &mut out);
            let pred = iri_term(&RDFS_SUBCLASS_OF);
            let obj = class_expression_to_term(&a.superclass, counter, &mut out);
            out.push(triple(subj, pred, obj));
            out
        }
        // ── Simple 1-triple: ClassAssertion ─────────────────────────────
        Axiom::ClassAssertion(a) => {
            let mut out = Vec::new();
            let subj = individual_to_term(&a.individual);
            let pred = iri_term(&RDF_TYPE);
            let obj = class_expression_to_term(&a.class, counter, &mut out);
            out.push(triple(subj, pred, obj));
            out
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
        Axiom::EquivalentClasses(a) => {
            let mut out = Vec::new();
            pairwise_triples(&a.classes, &OWL_EQUIVALENT_CLASS, counter, &mut out);
            out
        }
        // ── Pairwise: DisjointClasses ───────────────────────────────────
        Axiom::DisjointClasses(a) => {
            let mut out = Vec::new();
            pairwise_triples(&a.classes, &OWL_DISJOINT_WITH, counter, &mut out);
            out
        }
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
            let mut out = Vec::new();
            let pred = iri_term(&RDFS_DOMAIN);
            let obj = class_expression_to_term(&a.domain, counter, &mut out);
            out.push(triple(ope_to_term(&a.property), pred, obj));
            out
        }
        // ── ObjectPropertyRange ─────────────────────────────────────────
        Axiom::ObjectPropertyRange(a) => {
            let mut out = Vec::new();
            let pred = iri_term(&RDFS_RANGE);
            let obj = class_expression_to_term(&a.range, counter, &mut out);
            out.push(triple(ope_to_term(&a.property), pred, obj));
            out
        }
        // ── DataPropertyDomain ──────────────────────────────────────────
        Axiom::DataPropertyDomain(a) => {
            let mut out = Vec::new();
            let pred = iri_term(&RDFS_DOMAIN);
            let obj = class_expression_to_term(&a.domain, counter, &mut out);
            out.push(triple(dpe_to_term(&a.property), pred, obj));
            out
        }
        // ── DataPropertyRange ───────────────────────────────────────────
        Axiom::DataPropertyRange(a) => {
            let mut out = Vec::new();
            let pred = iri_term(&RDFS_RANGE);
            let obj = data_range_to_term(&a.range, counter, &mut out);
            out.push(triple(dpe_to_term(&a.property), pred, obj));
            out
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
            let mut out = Vec::new();
            let pred = iri_term(&RDFS_DOMAIN);
            let obj = class_expression_to_term(&a.domain, counter, &mut out);
            out.push(triple(
                iri_term_from_str(a.property.iri.as_str()),
                pred,
                obj,
            ));
            out
        }
        // ── AnnotationPropertyRange ─────────────────────────────────────
        Axiom::AnnotationPropertyRange(a) => {
            let mut out = Vec::new();
            let pred = iri_term(&RDFS_RANGE);
            let obj = data_range_to_term(&a.range, counter, &mut out);
            out.push(triple(
                iri_term_from_str(a.property.iri.as_str()),
                pred,
                obj,
            ));
            out
        }
        // ── DisjointUnion (RDF list) ────────────────────────────────────
        Axiom::DisjointUnion(a) => {
            let mut triples = Vec::new();
            let cls_term = class_expression_to_term(&a.class, counter, &mut triples);
            let members: Vec<RdfTerm> = a
                .disjoint_classes
                .iter()
                .map(|ce| class_expression_to_term(ce, counter, &mut triples))
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
            let cls_term = class_expression_to_term(&a.class, counter, &mut triples);
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
            // Rules have no per-axiom triple form here: they are emitted as
            // a preserved functional-syntax text literal (and decoded back
            // by `rdf_to_owl`) via [`ontology_preservation_triples`], which
            // [`ontology_to_triples`] includes.  Writers that call this
            // function directly either render rules natively (N3's
            // `{ … } => { … }`) or predate rule support (RDF/XML) — see the
            // module docs (CR-011 M2).
            vec![]
        }
    }
}

/// Fallback subject IRI for ontology-level data when the ontology has no
/// IRI of its own (CR-011 M2): annotations and SWRL rules are attached to
/// this well-known subject instead of being silently dropped.  The parser
/// side (`rdf_to_owl::map_rdf_triples` / `process_triple`) recognises it and
/// re-attaches the data to `Ontology::annotations` / `Axiom::Rule`.
pub const ONTOLOGY_FALLBACK_IRI: &str = "urn:oxidowl:ontology";

/// Predicate under which [`ontology_to_triples`] preserves a SWRL rule as a
/// functional-syntax `DLSafeRule(...)` text literal (CR-011 M2).  Decoded
/// back into [`Axiom::Rule`] by `rdf_to_owl`.
pub const SWRL_RULE_PROPERTY: &str = "urn:oxidowl:swrlRule";

/// Ontology-level data that must survive every RDF-based round-trip even
/// though it is not an axiom triple: the ontology-level annotations
/// (attached to the ontology IRI, or to [`ONTOLOGY_FALLBACK_IRI`] when the
/// ontology has none) and the SWRL rules (as [`SWRL_RULE_PROPERTY`] text
/// literals).  Nothing is ever silently discarded (CR-011 M2).
///
/// [`ontology_to_triples`] includes these; writers that render axioms
/// themselves (e.g. the basic Turtle serializer) can call this directly.
pub fn ontology_preservation_triples(ontology: &Ontology) -> crate::Result<Vec<Triple>> {
    use crate::{Error, Result};

    fn parse_iri(what: &str, s: &str) -> Result<url::Url> {
        url::Url::parse(s)
            .map_err(|e| Error::ontology_parsing(format!("invalid {what} IRI '{s}': {e}")))
    }

    let mut triples: Vec<Triple> = Vec::new();
    // Annotations: ontology IRI when there is one, the fallback subject
    // otherwise (previously gated on `get_iri()`, which dropped them for
    // IRI-less ontologies).
    if !ontology.annotations.is_empty() {
        let subject = match ontology.get_iri() {
            Some(iri) => RdfTerm::Iri(parse_iri("ontology", iri.as_str())?),
            None => RdfTerm::Iri(parse_iri("ontology fallback", ONTOLOGY_FALLBACK_IRI)?),
        };
        for annotation in &ontology.annotations {
            triples.push(Triple {
                subject: subject.clone(),
                predicate: RdfTerm::Iri(parse_iri(
                    "annotation property",
                    annotation.property.iri.as_str(),
                )?),
                object: annotation_value_to_term(&annotation.value),
            });
        }
    }
    // SWRL rules: preserved under the fallback subject as the exact text
    // `FunctionalParser::parse_swrl_rule_text` decodes.
    for axiom in ontology.axioms() {
        if let Axiom::Rule(rule_axiom) = axiom {
            triples.push(Triple {
                subject: RdfTerm::Iri(parse_iri("ontology fallback", ONTOLOGY_FALLBACK_IRI)?),
                predicate: RdfTerm::Iri(parse_iri("SWRL rule property", SWRL_RULE_PROPERTY)?),
                object: RdfTerm::Literal {
                    value: crate::parsers::functional::serialize_swrl_rule(&rule_axiom.rule),
                    datatype: None,
                    language: None,
                    direction: None,
                },
            });
        }
    }
    Ok(triples)
}

/// Full RDF triple representation of an ontology — the shared
/// serializer-side entry point for RDF-based *writers*.
///
/// Emits, in order:
/// 1. `<ontology-iri> rdf:type owl:Ontology` (when the ontology has an IRI),
/// 2. `<ontology-iri> owl:versionIRI <version>` (when set),
/// 3. the ontology-level annotations and SWRL rules via
///    [`ontology_preservation_triples`] (CR-011 M2 — emitted
///    **unconditionally**, falling back to `urn:oxidowl:ontology` when the
///    ontology has no IRI),
/// 4. the triples of every axiom via [`axiom_to_triples`].
///
/// The BinaryRDF and HDT writers use this so every RDF-based writer agrees
/// on one canonical triple set — the same emission the JSON-LD / N3 /
/// Turtle renderers perform inline — and the result round-trips through the
/// shared RDF→OWL mapping ([`crate::parsers::rdf_to_owl::map_rdf_triples`]).
pub fn ontology_to_triples(ontology: &Ontology) -> crate::Result<Vec<Triple>> {
    use crate::{Error, Result};

    fn parse_iri(what: &str, s: &str) -> Result<url::Url> {
        url::Url::parse(s)
            .map_err(|e| Error::ontology_parsing(format!("invalid {what} IRI '{s}': {e}")))
    }

    let mut counter = BlankNodeCounter::new();
    let mut triples: Vec<Triple> = Vec::new();
    if let Some(iri) = ontology.get_iri() {
        let subject = RdfTerm::Iri(parse_iri("ontology", iri.as_str())?);
        triples.push(Triple {
            subject: subject.clone(),
            predicate: RdfTerm::Iri(parse_iri(
                "rdf:type",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#type",
            )?),
            object: RdfTerm::Iri(parse_iri(
                "owl:Ontology",
                "http://www.w3.org/2002/07/owl#Ontology",
            )?),
        });
        if let Some(version) = ontology.id.get_version_iri() {
            triples.push(Triple {
                subject,
                predicate: RdfTerm::Iri(parse_iri(
                    "owl:versionIRI",
                    "http://www.w3.org/2002/07/owl#versionIRI",
                )?),
                object: RdfTerm::Iri(parse_iri("version", version.as_str())?),
            });
        }
    }
    triples.extend(ontology_preservation_triples(ontology)?);
    for axiom in ontology.axioms() {
        triples.extend(axiom_to_triples(axiom, &mut counter));
    }
    Ok(triples)
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

/// Emit `_:b rdf:type owl:Restriction` and return the fresh blank node.
fn fresh_restriction(counter: &mut BlankNodeCounter, triples: &mut Vec<Triple>) -> RdfTerm {
    let bn = counter.fresh();
    triples.push(triple(
        bn.clone(),
        iri_term(&RDF_TYPE),
        iri_term(&OWL_RESTRICTION),
    ));
    bn
}

/// Emit `<bn> <predicate> <filler>` for a freshly created restriction node.
fn push_restriction(
    bn: &RdfTerm,
    predicate: &url::Url,
    filler: RdfTerm,
    triples: &mut Vec<Triple>,
) {
    triples.push(triple(bn.clone(), iri_term(predicate), filler));
}

/// Cardinality literal: `"n"^^xsd:nonNegativeInteger`.
fn cardinality_literal(n: u32) -> RdfTerm {
    RdfTerm::Literal {
        value: n.to_string(),
        datatype: Some(XSD_NON_NEGATIVE_INTEGER_URL.clone()),
        language: None,
        direction: None,
    }
}

/// Convert a class expression to its RDF term, emitting any structural
/// triples (restriction nodes, RDF lists) into `triples`.
///
/// This is the full OWL 2 RDF mapping for class expressions: restrictions
/// (`owl:Restriction` blank nodes with `owl:onProperty` + filler),
/// `owl:intersectionOf` / `owl:unionOf` / `owl:complementOf` / `owl:oneOf`
/// list structures, and qualified cardinalities.  (Previously complex
/// expressions returned a *naked* blank node with no describing triples,
/// which silently dropped the axiom on re-parse.)
fn class_expression_to_term(
    ce: &ClassExpression,
    counter: &mut BlankNodeCounter,
    triples: &mut Vec<Triple>,
) -> RdfTerm {
    match ce {
        ClassExpression::Class(c) => iri_term_from_url(
            url::Url::parse(c.iri.as_str())
                .unwrap_or(url::Url::parse("http://example.org/error").unwrap()),
        ),
        ClassExpression::ObjectIntersectionOf(parts) => {
            let items: Vec<RdfTerm> = parts
                .iter()
                .map(|p| class_expression_to_term(p, counter, triples))
                .collect();
            let head = build_rdf_list(&items, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_INTERSECTION_OF), head));
            bn
        }
        ClassExpression::ObjectUnionOf(parts) => {
            let items: Vec<RdfTerm> = parts
                .iter()
                .map(|p| class_expression_to_term(p, counter, triples))
                .collect();
            let head = build_rdf_list(&items, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_UNION_OF), head));
            bn
        }
        ClassExpression::ObjectOneOf(individuals) => {
            let items: Vec<RdfTerm> = individuals.iter().map(individual_to_term).collect();
            let head = build_rdf_list(&items, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_ONE_OF), head));
            bn
        }
        ClassExpression::ObjectComplementOf(inner) => {
            let inner_term = class_expression_to_term(inner, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_COMPLEMENT_OF), inner_term));
            bn
        }
        ClassExpression::ObjectSomeValuesFrom { property, filler } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            let filler_term = class_expression_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_SOME_VALUES_FROM_URL, filler_term, triples);
            bn
        }
        ClassExpression::ObjectAllValuesFrom { property, filler } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            let filler_term = class_expression_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ALL_VALUES_FROM_URL, filler_term, triples);
            bn
        }
        ClassExpression::ObjectHasValue { property, value } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            push_restriction(&bn, &OWL_HAS_VALUE_URL, individual_to_term(value), triples);
            bn
        }
        ClassExpression::ObjectHasSelf { property } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            push_restriction(&bn, &OWL_HAS_SELF_URL, boolean_literal(true), triples);
            bn
        }
        ClassExpression::ObjectMinCardinality {
            property,
            cardinality,
            filler,
        } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            let filler_term = class_expression_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ON_CLASS_URL, filler_term, triples);
            push_restriction(
                &bn,
                &OWL_MIN_QUALIFIED_CARDINALITY_URL,
                cardinality_literal(*cardinality),
                triples,
            );
            bn
        }
        ClassExpression::ObjectMaxCardinality {
            property,
            cardinality,
            filler,
        } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            let filler_term = class_expression_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ON_CLASS_URL, filler_term, triples);
            push_restriction(
                &bn,
                &OWL_MAX_QUALIFIED_CARDINALITY_URL,
                cardinality_literal(*cardinality),
                triples,
            );
            bn
        }
        ClassExpression::ObjectExactCardinality {
            property,
            cardinality,
            filler,
        } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, ope_to_term(property), triples);
            let filler_term = class_expression_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ON_CLASS_URL, filler_term, triples);
            push_restriction(
                &bn,
                &OWL_QUALIFIED_CARDINALITY_URL,
                cardinality_literal(*cardinality),
                triples,
            );
            bn
        }
        ClassExpression::DataSomeValuesFrom { property, filler } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, dpe_to_term(property), triples);
            let filler_term = data_range_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_SOME_VALUES_FROM_URL, filler_term, triples);
            bn
        }
        ClassExpression::DataAllValuesFrom { property, filler } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, dpe_to_term(property), triples);
            let filler_term = data_range_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ALL_VALUES_FROM_URL, filler_term, triples);
            bn
        }
        ClassExpression::DataHasValue { property, value } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, dpe_to_term(property), triples);
            push_restriction(&bn, &OWL_HAS_VALUE_URL, literal_to_term(value), triples);
            bn
        }
        ClassExpression::DataMinCardinality {
            property,
            cardinality,
            filler,
        } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, dpe_to_term(property), triples);
            let filler_term = data_range_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ON_DATA_RANGE_URL, filler_term, triples);
            push_restriction(
                &bn,
                &OWL_MIN_QUALIFIED_CARDINALITY_URL,
                cardinality_literal(*cardinality),
                triples,
            );
            bn
        }
        ClassExpression::DataMaxCardinality {
            property,
            cardinality,
            filler,
        } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, dpe_to_term(property), triples);
            let filler_term = data_range_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ON_DATA_RANGE_URL, filler_term, triples);
            push_restriction(
                &bn,
                &OWL_MAX_QUALIFIED_CARDINALITY_URL,
                cardinality_literal(*cardinality),
                triples,
            );
            bn
        }
        ClassExpression::DataExactCardinality {
            property,
            cardinality,
            filler,
        } => {
            let bn = fresh_restriction(counter, triples);
            push_restriction(&bn, &OWL_ON_PROPERTY_URL, dpe_to_term(property), triples);
            let filler_term = data_range_to_term(filler, counter, triples);
            push_restriction(&bn, &OWL_ON_DATA_RANGE_URL, filler_term, triples);
            push_restriction(
                &bn,
                &OWL_QUALIFIED_CARDINALITY_URL,
                cardinality_literal(*cardinality),
                triples,
            );
            bn
        }
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

fn data_range_to_term(
    dr: &DataRange,
    counter: &mut BlankNodeCounter,
    triples: &mut Vec<Triple>,
) -> RdfTerm {
    match dr {
        DataRange::Datatype(iri) => iri_term_from_str(iri.as_str()),
        DataRange::DataIntersectionOf(ranges) => {
            let items: Vec<RdfTerm> = ranges
                .iter()
                .map(|r| data_range_to_term(r, counter, triples))
                .collect();
            let head = build_rdf_list(&items, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_INTERSECTION_OF), head));
            bn
        }
        DataRange::DataUnionOf(ranges) => {
            let items: Vec<RdfTerm> = ranges
                .iter()
                .map(|r| data_range_to_term(r, counter, triples))
                .collect();
            let head = build_rdf_list(&items, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_UNION_OF), head));
            bn
        }
        DataRange::DataComplementOf(inner) => {
            let inner_term = data_range_to_term(inner, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_COMPLEMENT_OF), inner_term));
            bn
        }
        DataRange::DataOneOf(literals) => {
            let items: Vec<RdfTerm> = literals.iter().map(literal_to_term).collect();
            let head = build_rdf_list(&items, counter, triples);
            let bn = counter.fresh();
            triples.push(triple(bn.clone(), iri_term(&OWL_ONE_OF), head));
            bn
        }
        DataRange::DatatypeRestriction {
            datatype,
            restrictions,
        } => {
            let bn = counter.fresh();
            triples.push(triple(
                bn.clone(),
                iri_term(&OWL_ON_DATATYPE),
                iri_term_from_str(datatype.as_str()),
            ));
            let mut facet_terms = Vec::new();
            for fr in restrictions {
                let facet_bn = counter.fresh();
                triples.push(triple(
                    facet_bn.clone(),
                    iri_term_from_str(fr.facet.as_str()),
                    literal_to_term(&fr.value),
                ));
                facet_terms.push(facet_bn);
            }
            let head = build_rdf_list(&facet_terms, counter, triples);
            triples.push(triple(bn.clone(), iri_term(&OWL_WITH_RESTRICTIONS), head));
            bn
        }
    }
}

fn boolean_literal(value: bool) -> RdfTerm {
    RdfTerm::Literal {
        value: value.to_string(),
        datatype: Some(XSD_BOOLEAN_URL.clone()),
        language: None,
        direction: None,
    }
}

static XSD_BOOLEAN_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2001/XMLSchema#boolean").unwrap());

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
    triples: &mut Vec<Triple>,
) {
    if items.len() < 2 {
        return;
    }
    let first = class_expression_to_term(&items[0], counter, triples);
    let pred = iri_term(predicate_url);
    for ce in &items[1..] {
        let obj = class_expression_to_term(ce, counter, triples);
        triples.push(triple(first.clone(), pred.clone(), obj));
    }
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

// ── Restriction / class-expression vocabulary (OWL 2 RDF mapping) ──────
static OWL_RESTRICTION: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#Restriction").unwrap());
static OWL_ON_PROPERTY_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#onProperty").unwrap());
static OWL_SOME_VALUES_FROM_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#someValuesFrom").unwrap());
static OWL_ALL_VALUES_FROM_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#allValuesFrom").unwrap());
static OWL_HAS_VALUE_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#hasValue").unwrap());
static OWL_HAS_SELF_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#hasSelf").unwrap());
static OWL_ON_CLASS_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#onClass").unwrap());
static OWL_ON_DATA_RANGE_URL: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#onDataRange").unwrap());
static OWL_MIN_QUALIFIED_CARDINALITY_URL: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("http://www.w3.org/2002/07/owl#minQualifiedCardinality").unwrap()
});
static OWL_MAX_QUALIFIED_CARDINALITY_URL: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("http://www.w3.org/2002/07/owl#maxQualifiedCardinality").unwrap()
});
static OWL_QUALIFIED_CARDINALITY_URL: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("http://www.w3.org/2002/07/owl#qualifiedCardinality").unwrap()
});
static OWL_INTERSECTION_OF: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#intersectionOf").unwrap());
static OWL_UNION_OF: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#unionOf").unwrap());
static OWL_COMPLEMENT_OF: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#complementOf").unwrap());
static OWL_ONE_OF: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#oneOf").unwrap());
static OWL_ON_DATATYPE: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#onDatatype").unwrap());
static OWL_WITH_RESTRICTIONS: LazyLock<url::Url> =
    LazyLock::new(|| url::Url::parse("http://www.w3.org/2002/07/owl#withRestrictions").unwrap());
static XSD_NON_NEGATIVE_INTEGER_URL: LazyLock<url::Url> = LazyLock::new(|| {
    url::Url::parse("http://www.w3.org/2001/XMLSchema#nonNegativeInteger").unwrap()
});

//! Shared RDF → OWL mapping.
//!
//! Single source of truth for turning RDF triples into `Ontology` axioms.
//! All RDF-based parsers delegate here instead of duplicating mapping logic:
//!
//! * [`process_triple`] — string-level triple → axiom mapping.  This is the
//!   mapping the Turtle parser historically owned (`process_enhanced_triple`);
//!   it was factored out here for B5 Slice 1 so JSON-LD, RDF/XML and Turtle
//!   share *one* implementation.
//! * [`process_term_triple`] — term-level wrapper (blank nodes, real RDF
//!   literals) used by parsers that produce typed terms rather than
//!   Turtle-syntax strings.
//! * [`map_rdf_triples`] — whole-graph mapping with blank-node
//!   reconstruction: it resolves `owl:Restriction` blank nodes into
//!   `ClassExpression`s (`ObjectSomeValuesFrom`, …) and RDF lists into
//!   `owl:intersectionOf` / `owl:unionOf` / `owl:oneOf` expressions, the
//!   gap that previously blocked nested restrictions on every RDF syntax.
//!
//! The serializer-side counterpart lives in
//! [`crate::semantics::owl_rdf_mapping`] (`axiom_to_triples`).

use std::collections::{HashMap, HashSet};

use crate::Result;
use crate::ontology::*;
use crate::semantics::owl_rdf_mapping;
use crate::semantics::{RdfTerm, Triple as RdfTriple};

/// Convert an RDF term into an [`AnnotationValue`] (fallback-subject
/// ontology annotations, CR-011 M2).  Blank objects become anonymous
/// individuals; unsupported term kinds return `None` (the caller warns).
fn term_to_annotation_value(term: &RdfTerm) -> Option<AnnotationValue> {
    match term {
        RdfTerm::Iri(url) => Some(AnnotationValue::IRI(IRI::new(url.as_str()))),
        RdfTerm::BlankNode(id) => Some(AnnotationValue::AnonymousIndividual(
            crate::ontology::AnonymousIndividual { id: id.clone() },
        )),
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => Some(AnnotationValue::Literal(normalize_literal(&Literal {
            value: value.clone(),
            language: language.clone(),
            datatype: datatype.clone(),
        }))),
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Vocabulary IRIs
// ═══════════════════════════════════════════════════════════════════════════

pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
pub const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
pub const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
pub const RDF_LIST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#List";
pub const RDFS_SUBCLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const RDFS_CLASS: &str = "http://www.w3.org/2000/01/rdf-schema#Class";

pub const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub const OWL_OBJECT_PROPERTY: &str = "http://www.w3.org/2002/07/owl#ObjectProperty";
pub const OWL_DATATYPE_PROPERTY: &str = "http://www.w3.org/2002/07/owl#DatatypeProperty";
pub const OWL_ANNOTATION_PROPERTY: &str = "http://www.w3.org/2002/07/owl#AnnotationProperty";
pub const OWL_NAMED_INDIVIDUAL: &str = "http://www.w3.org/2002/07/owl#NamedIndividual";
pub const OWL_EQUIVALENT_CLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
pub const OWL_DISJOINT_WITH: &str = "http://www.w3.org/2002/07/owl#disjointWith";
pub const OWL_DISJOINT_UNION_OF: &str = "http://www.w3.org/2002/07/owl#disjointUnionOf";
pub const OWL_INTERSECTION_OF: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
pub const OWL_UNION_OF: &str = "http://www.w3.org/2002/07/owl#unionOf";
pub const OWL_COMPLEMENT_OF: &str = "http://www.w3.org/2002/07/owl#complementOf";
pub const OWL_ONE_OF: &str = "http://www.w3.org/2002/07/owl#oneOf";
pub const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
pub const OWL_ON_PROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
pub const OWL_SOME_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
pub const OWL_ALL_VALUES_FROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
pub const OWL_HAS_VALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
pub const OWL_HAS_SELF: &str = "http://www.w3.org/2002/07/owl#hasSelf";
pub const OWL_ON_CLASS: &str = "http://www.w3.org/2002/07/owl#onClass";
pub const OWL_ON_DATA_RANGE: &str = "http://www.w3.org/2002/07/owl#onDataRange";
pub const OWL_MIN_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#minCardinality";
pub const OWL_MAX_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#maxCardinality";
pub const OWL_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#cardinality";
pub const OWL_MIN_QUALIFIED_CARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#minQualifiedCardinality";
pub const OWL_MAX_QUALIFIED_CARDINALITY: &str =
    "http://www.w3.org/2002/07/owl#maxQualifiedCardinality";
pub const OWL_QUALIFIED_CARDINALITY: &str = "http://www.w3.org/2002/07/owl#qualifiedCardinality";
pub const OWL_ON_DATATYPE: &str = "http://www.w3.org/2002/07/owl#onDatatype";
pub const OWL_WITH_RESTRICTIONS: &str = "http://www.w3.org/2002/07/owl#withRestrictions";
pub const OWL_PROPERTY_CHAIN_AXIOM: &str = "http://www.w3.org/2002/07/owl#propertyChainAxiom";
pub const OWL_HAS_KEY: &str = "http://www.w3.org/2002/07/owl#hasKey";
pub const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
pub const RDF_LITERAL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#Literal";
pub const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
pub const XSD_PREFIX: &str = "http://www.w3.org/2001/XMLSchema#";

// ═══════════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════════

fn generate_axiom_id() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Return true if `s` is a bare (unquoted) Turtle literal –
/// i.e. an integer (`0`, `-1`), decimal (`3.14`), double (`1.0e5`),
/// or boolean (`true` / `false`).
/// These are NOT IRIs and must be treated as literal values.
fn is_bare_literal(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    if s == "true" || s == "false" {
        return true;
    }
    // Strip optional leading minus
    let s = s.strip_prefix('-').unwrap_or(s);
    if s.is_empty() {
        return false;
    }
    // First character must be a digit or '.'
    let first = s.chars().next().unwrap();
    if !first.is_ascii_digit() && first != '.' {
        return false;
    }
    // All remaining characters must be digits, '.', 'e', 'E', '+', '-'
    s.chars()
        .all(|c| c.is_ascii_digit() || c == '.' || c == 'e' || c == 'E' || c == '+' || c == '-')
}

/// Given a bare numeric/boolean literal string, return `(value, datatype_url)`.
fn bare_literal_parts(s: &str) -> (String, Option<url::Url>) {
    if s == "true" || s == "false" {
        return (
            s.to_owned(),
            url::Url::parse("http://www.w3.org/2001/XMLSchema#boolean").ok(),
        );
    }
    let lower = s.to_lowercase();
    if lower.contains('e') {
        // double
        (
            s.to_owned(),
            url::Url::parse("http://www.w3.org/2001/XMLSchema#double").ok(),
        )
    } else if lower.contains('.') {
        // decimal
        (
            s.to_owned(),
            url::Url::parse("http://www.w3.org/2001/XMLSchema#decimal").ok(),
        )
    } else {
        // integer
        (
            s.to_owned(),
            url::Url::parse("http://www.w3.org/2001/XMLSchema#integer").ok(),
        )
    }
}

/// Check if a subject IRI is declared as a `DatatypeProperty`.
pub fn is_data_property(iri: &str, ontology: &Ontology) -> bool {
    ontology.axioms().iter().any(|a| {
        if let Axiom::Declaration(d) = a
            && let Entity::DataProperty(dp_iri) = &d.entity
        {
            return dp_iri.as_str() == iri;
        }
        false
    })
}

/// Check if a predicate IRI is a well-known or declared `AnnotationProperty`.
pub fn is_annotation_property(iri: &str, ontology: &Ontology) -> bool {
    if matches!(
        iri,
        "http://www.w3.org/2000/01/rdf-schema#label"
            | "http://www.w3.org/2000/01/rdf-schema#comment"
            | "http://www.w3.org/2000/01/rdf-schema#isDefinedBy"
            | "http://www.w3.org/2000/01/rdf-schema#seeAlso"
            | "http://www.w3.org/2002/07/owl#deprecated"
            | "http://www.w3.org/2002/07/owl#priorVersion"
            | "http://www.w3.org/2002/07/owl#backwardCompatibleWith"
            | "http://www.w3.org/2002/07/owl#incompatibleWith"
            | "http://www.w3.org/2002/07/owl#versionInfo"
    ) {
        return true;
    }
    ontology.axioms().iter().any(|a| {
        if let Axiom::Declaration(d) = a
            && let Entity::AnnotationProperty(ap_iri) = &d.entity
        {
            return ap_iri.as_str() == iri;
        }
        false
    })
}

/// Auto-declare an `AnnotationProperty` if not already present (built-in
/// OWL/RDFS annotation properties are never declared explicitly).
pub fn ensure_annotation_property_declared(iri: &str, ontology: &mut Ontology) {
    if matches!(
        iri,
        "http://www.w3.org/2000/01/rdf-schema#label"
            | "http://www.w3.org/2000/01/rdf-schema#comment"
            | "http://www.w3.org/2000/01/rdf-schema#isDefinedBy"
            | "http://www.w3.org/2000/01/rdf-schema#seeAlso"
            | "http://www.w3.org/2002/07/owl#deprecated"
            | "http://www.w3.org/2002/07/owl#priorVersion"
            | "http://www.w3.org/2002/07/owl#backwardCompatibleWith"
            | "http://www.w3.org/2002/07/owl#incompatibleWith"
            | "http://www.w3.org/2002/07/owl#versionInfo"
    ) {
        return;
    }
    let already_declared = ontology.axioms().iter().any(|a| {
        if let Axiom::Declaration(d) = a
            && let Entity::AnnotationProperty(ap_iri) = &d.entity
        {
            return ap_iri.as_str() == iri;
        }
        false
    });
    if !already_declared {
        let decl = DeclarationAxiom {
            id: generate_axiom_id(),
            entity: Entity::AnnotationProperty(IRI::new(iri)),
        };
        ontology.add_axiom(Axiom::Declaration(decl));
    }
}

/// Structural RDF/OWL predicates (restriction internals, RDF lists, list-valued
/// axioms).  They must never fall through to a property-assertion default arm —
/// they are consumed by the blank-node / list resolvers instead.
pub fn is_structural_owl_predicate(predicate: &str) -> bool {
    matches!(
        predicate,
        "http://www.w3.org/2002/07/owl#onProperty"
            | "http://www.w3.org/2002/07/owl#allValuesFrom"
            | "http://www.w3.org/2002/07/owl#someValuesFrom"
            | "http://www.w3.org/2002/07/owl#hasValue"
            | "http://www.w3.org/2002/07/owl#hasSelf"
            | "http://www.w3.org/2002/07/owl#onClass"
            | "http://www.w3.org/2002/07/owl#onDataRange"
            | "http://www.w3.org/2002/07/owl#minCardinality"
            | "http://www.w3.org/2002/07/owl#maxCardinality"
            | "http://www.w3.org/2002/07/owl#cardinality"
            | "http://www.w3.org/2002/07/owl#minQualifiedCardinality"
            | "http://www.w3.org/2002/07/owl#maxQualifiedCardinality"
            | "http://www.w3.org/2002/07/owl#qualifiedCardinality"
            | "http://www.w3.org/2002/07/owl#distinctMembers"
            | "http://www.w3.org/2002/07/owl#members"
            | "http://www.w3.org/2002/07/owl#oneOf"
            | "http://www.w3.org/2002/07/owl#unionOf"
            | "http://www.w3.org/2002/07/owl#intersectionOf"
            | "http://www.w3.org/2002/07/owl#complementOf"
            | "http://www.w3.org/2002/07/owl#disjointUnionOf"
            | "http://www.w3.org/2002/07/owl#withRestrictions"
            | "http://www.w3.org/2002/07/owl#onDatatype"
            | "http://www.w3.org/2002/07/owl#propertyChainAxiom"
            | "http://www.w3.org/2002/07/owl#hasKey"
            | RDF_FIRST
            | RDF_REST
            | RDF_NIL
    )
}

/// Is this IRI a well-known RDF/RDFS/OWL vocabulary term (used to decide
/// whether `_:b rdf:type <iri>` is a structural marker or a real assertion)?
fn is_wellknown_vocab_term(iri: &str) -> bool {
    iri.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
        || iri.starts_with("http://www.w3.org/2000/01/rdf-schema#")
        || iri.starts_with("http://www.w3.org/2002/07/owl#")
}

/// Datatype IRIs that make an `owl:someValuesFrom`/`owl:allValuesFrom`
/// filler a *data* range rather than a class.
fn is_datatype_iri(iri: &str) -> bool {
    iri.starts_with(XSD_PREFIX) || iri == RDF_LITERAL || iri == RDFS_LITERAL
}

/// RDF 1.1 simple literals and explicit `xsd:string` are the same value;
/// the ontology model stores them as `datatype: None` (as the Turtle and
/// Functional parsers do), so normalize on the way in.
fn normalize_literal(lit: &Literal) -> Literal {
    let mut out = lit.clone();
    if out.language.is_none()
        && out
            .datatype
            .as_ref()
            .is_some_and(|d| d.as_str() == format!("{XSD_PREFIX}string"))
    {
        out.datatype = None;
    }
    out
}

// ═══════════════════════════════════════════════════════════════════════════
// String-level mapping (shared by Turtle, RDF/XML and — via
// process_term_triple — JSON-LD)
// ═══════════════════════════════════════════════════════════════════════════

/// Parse the string form of an RDF term (as the string-level
/// [`process_triple`] receives it) into an [`AnnotationValue`] — used for
/// the fallback-subject ontology annotations (CR-011 M2).
///
/// Accepts `"lexical"` (optionally `^^datatype` / `@lang` suffixed),
/// `<iri>`, and `_:label`.
fn string_to_annotation_value(object: &str) -> Option<AnnotationValue> {
    if object.starts_with('"') {
        // `"lexical"` optionally followed by `^^<datatype>` / `@lang`.
        if let Some(idx) = object.find("\"^^") {
            let lexical = object[..idx].strip_prefix('"')?;
            let dt_raw = object[idx + 3..].trim_matches(['<', '>']);
            return Some(AnnotationValue::Literal(Literal {
                value: lexical.to_string(),
                language: None,
                datatype: Some(IRI::new(dt_raw).to_url().ok()?),
            }));
        }
        let closing = object.rfind('"')?;
        let suffix = &object[closing + 1..];
        if let Some(lang) = suffix.strip_prefix('@')
            && !lang.is_empty()
            && lang.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Some(AnnotationValue::Literal(Literal {
                value: object[1..closing].to_string(),
                language: Some(lang.to_string()),
                datatype: None,
            }));
        }
        // Plain literal: strip exactly the outer quotes so a lexical value
        // that itself ends in `"` keeps it.
        let inner = object.strip_suffix('"').unwrap_or(object);
        return Some(AnnotationValue::Literal(Literal {
            value: inner.strip_prefix('"').unwrap_or(inner).to_string(),
            language: None,
            datatype: None,
        }));
    }
    if let Some(inner) = object.strip_prefix('<').and_then(|s| s.strip_suffix('>'))
        && url::Url::parse(inner).is_ok()
    {
        return Some(AnnotationValue::IRI(IRI::new(inner)));
    }
    // Turtle-style bare IRI object (no angle brackets).
    if url::Url::parse(object).is_ok() {
        return Some(AnnotationValue::IRI(IRI::new(object)));
    }
    if let Some(label) = object.strip_prefix("_:")
        && !label.is_empty()
    {
        return Some(AnnotationValue::AnonymousIndividual(
            crate::ontology::AnonymousIndividual {
                id: label.to_string(),
            },
        ));
    }
    None
}

/// Map a single RDF triple (as plain strings) into ontology axioms.
///
/// This is the Turtle parser's `process_enhanced_triple` factored out of
/// `TurtleParser` so every RDF syntax shares one mapping.  Structural OWL
/// predicates (restrictions, RDF lists) are ignored here — parsers that
/// support blank-node reconstruction use [`map_rdf_triples`] instead.
pub fn process_triple(
    ontology: &mut Ontology,
    subject: &str,
    predicate: &str,
    object: &str,
) -> Result<()> {
    // ── Preservation triples from the shared writer (CR-011 M2) ─────────
    if predicate == owl_rdf_mapping::SWRL_RULE_PROPERTY
        && let Some(text) = object.strip_prefix('"').and_then(|s| s.strip_suffix('"'))
        && !text.is_empty()
    {
        match crate::parsers::functional::FunctionalParser::new().parse_swrl_rule_text(text) {
            Ok(rule) => {
                ontology.add_axiom(Axiom::Rule(rule));
                return Ok(());
            }
            Err(e) => {
                log::warn!(
                    "rdf_to_owl: preserved SWRL rule text did not parse ({e}); \
                     keeping the text as an ontology annotation"
                );
                // fall through — the text is preserved as an annotation below
            }
        }
    }
    // Ontology-level data of an IRI-less ontology, written under the
    // well-known fallback subject instead of being dropped (CR-011 M2).
    // `rdf:type owl:Ontology` is left to the normal dispatch so a document
    // that genuinely types the fallback subject still sets an ontology IRI.
    if subject == owl_rdf_mapping::ONTOLOGY_FALLBACK_IRI
        && !(predicate == RDF_TYPE && object == OWL_ONTOLOGY)
    {
        if let Some(value) = string_to_annotation_value(object) {
            ontology.annotations.push(Annotation {
                property: AnnotationProperty {
                    iri: IRI::new(predicate),
                },
                value,
                annotations: vec![],
            });
        } else {
            log::warn!(
                "rdf_to_owl: dropping ontology annotation with unsupported value {object:?}"
            );
        }
        return Ok(());
    }

    match predicate {
        RDF_TYPE => {
            match object {
                OWL_CLASS => {
                    let decl_axiom = DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::Class(IRI::new(subject)),
                    };
                    ontology.add_axiom(Axiom::Declaration(decl_axiom));
                }
                OWL_ONTOLOGY => {
                    ontology.set_iri(IRI::new(subject));
                }
                OWL_OBJECT_PROPERTY => {
                    let _property = ObjectProperty::new(IRI::new(subject));
                    let decl_axiom = DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::ObjectProperty(IRI::new(subject)),
                    };
                    ontology.add_axiom(Axiom::Declaration(decl_axiom));
                }
                OWL_DATATYPE_PROPERTY => {
                    let decl_axiom = DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::DataProperty(IRI::new(subject)),
                    };
                    ontology.add_axiom(Axiom::Declaration(decl_axiom));
                }
                OWL_ANNOTATION_PROPERTY => {
                    let decl_axiom = DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::AnnotationProperty(IRI::new(subject)),
                    };
                    ontology.add_axiom(Axiom::Declaration(decl_axiom));
                }
                OWL_NAMED_INDIVIDUAL => {
                    let decl_axiom = DeclarationAxiom {
                        id: generate_axiom_id(),
                        entity: Entity::NamedIndividual(IRI::new(subject)),
                    };
                    ontology.add_axiom(Axiom::Declaration(decl_axiom));
                }
                "http://www.w3.org/2002/07/owl#FunctionalProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = FunctionalObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::FunctionalObjectProperty(axiom));
                }
                "http://www.w3.org/2002/07/owl#TransitiveProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = TransitiveObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::TransitiveObjectProperty(axiom));
                }
                "http://www.w3.org/2002/07/owl#SymmetricProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = SymmetricObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::SymmetricObjectProperty(axiom));
                }
                "http://www.w3.org/2002/07/owl#AsymmetricProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = AsymmetricObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::AsymmetricObjectProperty(axiom));
                }
                "http://www.w3.org/2002/07/owl#ReflexiveProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = ReflexiveObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::ReflexiveObjectProperty(axiom));
                }
                "http://www.w3.org/2002/07/owl#IrreflexiveProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = IrreflexiveObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::IrreflexiveObjectProperty(axiom));
                }
                "http://www.w3.org/2002/07/owl#InverseFunctionalProperty" => {
                    let prop = ObjectProperty::new(IRI::new(subject))?;
                    let axiom = InverseFunctionalObjectPropertyAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(prop),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::InverseFunctionalObjectProperty(axiom));
                }
                _ => {
                    // Class assertion
                    let individual = Individual::Named(NamedIndividual {
                        iri: IRI::new(subject),
                    });
                    let class = Class::new(IRI::new(object));

                    let axiom = ClassAssertionAxiom {
                        id: generate_axiom_id(),
                        individual,
                        class: ClassExpression::Class(class),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::ClassAssertion(axiom));
                }
            }
        }
        "http://www.w3.org/2002/07/owl#versionIRI" => {
            // Fix 1.1: record the version IRI on the ontology
            ontology.set_version_iri(Some(IRI::new(object)));
        }
        "http://www.w3.org/2002/07/owl#imports" => {
            // Prevent owl:imports falling through to a spurious ObjectPropertyAssertion.
            // (Full import-following is handled separately; here we just no-op.)
        }
        RDFS_SUBCLASS_OF => {
            let subclass = Class::new(IRI::new(subject));
            let superclass = Class::new(IRI::new(object));

            let axiom = SubClassOfAxiom {
                id: generate_axiom_id(),
                subclass: ClassExpression::Class(subclass),
                superclass: ClassExpression::Class(superclass),
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::SubClassOf(axiom));
        }
        OWL_EQUIVALENT_CLASS => {
            let class1 = Class::new(IRI::new(subject));
            let class2 = Class::new(IRI::new(object));
            let axiom = EquivalentClassesAxiom {
                id: generate_axiom_id(),
                classes: vec![
                    ClassExpression::Class(class1),
                    ClassExpression::Class(class2),
                ],
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::EquivalentClasses(axiom));
        }
        OWL_DISJOINT_WITH => {
            let class1 = Class::new(IRI::new(subject));
            let class2 = Class::new(IRI::new(object));
            let axiom = DisjointClassesAxiom {
                id: generate_axiom_id(),
                classes: vec![
                    ClassExpression::Class(class1),
                    ClassExpression::Class(class2),
                ],
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::DisjointClasses(axiom));
        }
        "http://www.w3.org/2000/01/rdf-schema#domain" => {
            // Fix 1.2: route to DataPropertyDomain when the subject is a DatatypeProperty
            if is_data_property(subject, ontology) {
                let prop = DataProperty {
                    iri: IRI::new(subject),
                };
                let axiom = DataPropertyDomainAxiom {
                    id: generate_axiom_id(),
                    property: DataPropertyExpression::DataProperty(prop),
                    domain: ClassExpression::Class(Class::new(IRI::new(object))),
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::DataPropertyDomain(axiom));
            } else {
                let prop = ObjectProperty::new(IRI::new(subject))?;
                let domain = ClassExpression::Class(Class::new(IRI::new(object)));
                let axiom = ObjectPropertyDomainAxiom {
                    id: generate_axiom_id(),
                    property: ObjectPropertyExpression::ObjectProperty(prop),
                    domain,
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::ObjectPropertyDomain(axiom));
            }
        }
        "http://www.w3.org/2000/01/rdf-schema#range" => {
            // Fix 1.2: route to DataPropertyRange when subject is DatatypeProperty
            // or when the range is an XSD / RDF datatype IRI.
            let is_xsd_range = object.starts_with(XSD_PREFIX)
                || object.starts_with("http://www.w3.org/1999/02/22-rdf-syntax-ns#");
            if is_data_property(subject, ontology) || is_xsd_range {
                let prop = DataProperty {
                    iri: IRI::new(subject),
                };
                let axiom = DataPropertyRangeAxiom {
                    id: generate_axiom_id(),
                    property: DataPropertyExpression::DataProperty(prop),
                    range: DataRange::Datatype(IRI::new(object)),
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::DataPropertyRange(axiom));
            } else {
                let prop = ObjectProperty::new(IRI::new(subject))?;
                let range = ClassExpression::Class(Class::new(IRI::new(object)));
                let axiom = ObjectPropertyRangeAxiom {
                    id: generate_axiom_id(),
                    property: ObjectPropertyExpression::ObjectProperty(prop),
                    range,
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::ObjectPropertyRange(axiom));
            }
        }
        "http://www.w3.org/2002/07/owl#sameAs" => {
            let ind1 = Individual::Named(NamedIndividual {
                iri: IRI::new(subject),
            });
            let ind2 = Individual::Named(NamedIndividual {
                iri: IRI::new(object),
            });
            let axiom = SameIndividualAxiom {
                id: generate_axiom_id(),
                individuals: vec![ind1, ind2],
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::SameIndividual(axiom));
        }
        "http://www.w3.org/2002/07/owl#differentFrom" => {
            let ind1 = Individual::Named(NamedIndividual {
                iri: IRI::new(subject),
            });
            let ind2 = Individual::Named(NamedIndividual {
                iri: IRI::new(object),
            });
            let axiom = DifferentIndividualsAxiom {
                id: generate_axiom_id(),
                individuals: vec![ind1, ind2],
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::DifferentIndividuals(axiom));
        }
        "http://www.w3.org/2000/01/rdf-schema#subPropertyOf" => {
            // Fix 1.3: route to SubDataPropertyOf when subject is a DatatypeProperty
            if is_data_property(subject, ontology) {
                let sub_prop = DataProperty {
                    iri: IRI::new(subject),
                };
                let sup_prop = DataProperty {
                    iri: IRI::new(object),
                };
                let axiom = SubDataPropertyOfAxiom {
                    id: generate_axiom_id(),
                    sub_property: DataPropertyExpression::DataProperty(sub_prop),
                    super_property: DataPropertyExpression::DataProperty(sup_prop),
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::SubDataPropertyOf(axiom));
            } else {
                let sub_prop = ObjectProperty::new(IRI::new(subject))?;
                let sup_prop = ObjectProperty::new(IRI::new(object))?;
                let axiom = SubObjectPropertyOfAxiom {
                    id: generate_axiom_id(),
                    sub_property: ObjectPropertyExpression::ObjectProperty(sub_prop),
                    super_property: ObjectPropertyExpression::ObjectProperty(sup_prop),
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::SubObjectPropertyOf(axiom));
            }
        }
        "http://www.w3.org/2002/07/owl#inverseOf" => {
            let prop1 = ObjectProperty::new(IRI::new(subject))?;
            let prop2 = ObjectProperty::new(IRI::new(object))?;
            let axiom = InverseObjectPropertiesAxiom {
                id: generate_axiom_id(),
                property1: ObjectPropertyExpression::ObjectProperty(prop1),
                property2: ObjectPropertyExpression::ObjectProperty(prop2),
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::InverseObjectProperties(axiom));
        }
        "http://www.w3.org/2002/07/owl#hasKey" => {
            // hasKey with non-list object – skip silently (Fix 1.7)
        }
        OWL_PROPERTY_CHAIN_AXIOM => {
            // propertyChainAxiom with non-list object – skip silently (Fix 1.7)
        }
        // ── Fix 1.7: other well-known OWL/RDFS structural predicates that must NOT
        //    fall through to the ObjectPropertyAssertion default arm ──────────────
        "http://www.w3.org/2002/07/owl#onProperty"
        | "http://www.w3.org/2002/07/owl#allValuesFrom"
        | "http://www.w3.org/2002/07/owl#someValuesFrom"
        | "http://www.w3.org/2002/07/owl#hasValue"
        | "http://www.w3.org/2002/07/owl#hasSelf"
        | "http://www.w3.org/2002/07/owl#onClass"
        | "http://www.w3.org/2002/07/owl#onDataRange"
        | "http://www.w3.org/2002/07/owl#minCardinality"
        | "http://www.w3.org/2002/07/owl#maxCardinality"
        | "http://www.w3.org/2002/07/owl#cardinality"
        | "http://www.w3.org/2002/07/owl#minQualifiedCardinality"
        | "http://www.w3.org/2002/07/owl#maxQualifiedCardinality"
        | "http://www.w3.org/2002/07/owl#qualifiedCardinality"
        | "http://www.w3.org/2002/07/owl#distinctMembers"
        | "http://www.w3.org/2002/07/owl#members"
        | "http://www.w3.org/2002/07/owl#oneOf"
        | "http://www.w3.org/2002/07/owl#unionOf"
        | "http://www.w3.org/2002/07/owl#intersectionOf"
        | "http://www.w3.org/2002/07/owl#complementOf"
        | "http://www.w3.org/2002/07/owl#disjointUnionOf"
        | "http://www.w3.org/2002/07/owl#withRestrictions"
        | "http://www.w3.org/2002/07/owl#onDatatype"
        | RDF_FIRST
        | RDF_REST
        | RDF_NIL => {
            // Structural RDF/OWL predicates – handled by the blank-node / list
            // parsers.  Ignore here to prevent spurious ObjectPropertyAssertions.
        }
        _ => {
            // Fix 1.4: if the predicate is a (well-known or declared) annotation
            // property, emit an AnnotationAssertionAxiom rather than a property
            // assertion.  This covers IRI-valued annotations that are not already
            // handled by the semicolon-statement literal path.
            if is_annotation_property(predicate, ontology) {
                let annotation_value = if object.starts_with('"') {
                    // Strip surrounding quotes and create a plain literal
                    let raw = object.trim_matches('"').to_string();
                    AnnotationValue::Literal(Literal::new(raw))
                } else if object.is_empty() || object.starts_with("_:") {
                    // Anonymous individual or empty – skip
                    return Ok(());
                } else {
                    AnnotationValue::IRI(IRI::new(object))
                };

                // Ontology-level annotation vs entity annotation
                if ontology.get_iri().map(IRI::as_str) == Some(subject) {
                    let annotation = Annotation {
                        property: AnnotationProperty {
                            iri: IRI::new(predicate),
                        },
                        value: annotation_value,
                        annotations: vec![],
                    };
                    ontology.annotations.push(annotation);
                } else {
                    // Fix 1.8: auto-declare the annotation property if needed
                    ensure_annotation_property_declared(predicate, ontology);
                    let axiom = AnnotationAssertionAxiom {
                        id: generate_axiom_id(),
                        subject: AnnotationSubject::IRI(IRI::new(subject)),
                        property: AnnotationProperty {
                            iri: IRI::new(predicate),
                        },
                        value: annotation_value,
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::AnnotationAssertion(axiom));
                }
            } else {
                // Detect if object is a literal (data property) or an individual
                // (object property).  Cover three literal forms:
                //  1. Quoted string:  "value" or "value"^^datatype
                //  2. Bare numeric:   42, -3, 1.5, 1e6
                //  3. Bare boolean:   true / false
                let is_quoted_literal = object.starts_with('"') || object.contains("^^");
                let is_bare_num = !is_quoted_literal && is_bare_literal(object);
                let is_literal = is_quoted_literal || is_bare_num;

                if is_literal {
                    // DataPropertyAssertion
                    let subject_ind = Individual::Named(NamedIndividual {
                        iri: IRI::new(subject),
                    });
                    let data_property = DataProperty {
                        iri: IRI::new(predicate),
                    };

                    // Parse literal value and datatype
                    let (value, datatype) = if let Some(idx) = object.find("^^") {
                        let val = object[..idx].trim_matches('"').to_string();
                        let dt_str = object[idx + 2..].to_string();
                        let dt_iri = IRI::new(&dt_str);
                        let dt_url = dt_iri.to_url().ok();
                        (val, dt_url)
                    } else if is_bare_num {
                        bare_literal_parts(object)
                    } else {
                        (object.trim_matches('"').to_string(), None)
                    };

                    let literal = Literal {
                        value,
                        language: None,
                        datatype,
                    };

                    let axiom = DataPropertyAssertionAxiom {
                        id: generate_axiom_id(),
                        property: DataPropertyExpression::DataProperty(data_property),
                        individual: subject_ind,
                        value: literal,
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::DataPropertyAssertion(axiom));
                } else {
                    // ObjectPropertyAssertion
                    let subject_ind = Individual::Named(NamedIndividual {
                        iri: IRI::new(subject),
                    });
                    let object_ind = Individual::Named(NamedIndividual {
                        iri: IRI::new(object),
                    });
                    let property = ObjectProperty::new(IRI::new(predicate))?;

                    let axiom = ObjectPropertyAssertionAxiom {
                        id: generate_axiom_id(),
                        property: ObjectPropertyExpression::ObjectProperty(property),
                        source: subject_ind,
                        target: object_ind,
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::ObjectPropertyAssertion(axiom));
                }
            }
        }
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// Term-level mapping (JSON-LD and any parser producing typed RDF terms)
// ═══════════════════════════════════════════════════════════════════════════

/// Convert an RDF term literal (`RdfTerm::Literal` struct variant) into the
/// ontology's `Literal` type (dropping the RDF 1.2 direction field, which the
/// OWL model does not carry), then normalize.
fn rdf_term_literal_to_literal(
    value: &str,
    datatype: &Option<url::Url>,
    language: &Option<String>,
) -> Literal {
    normalize_literal(&Literal {
        value: value.to_owned(),
        language: language.clone(),
        datatype: datatype.clone(),
    })
}

fn term_to_individual(term: &RdfTerm) -> Option<Individual> {
    match term {
        RdfTerm::Iri(url) => Some(Individual::Named(NamedIndividual {
            iri: IRI::new(url.as_str()),
        })),
        RdfTerm::BlankNode(id) => Some(Individual::Anonymous(AnonymousIndividual {
            id: id.clone(),
        })),
        _ => None,
    }
}

/// Map a typed RDF triple into ontology axioms.  Blank nodes become
/// anonymous individuals; literals become `DataPropertyAssertion`s /
/// `AnnotationAssertion`s.  Named subject + named object triples are
/// delegated to the shared string-level [`process_triple`] so behaviour
/// is identical across RDF syntaxes.
pub fn process_term_triple(
    ontology: &mut Ontology,
    subject: &RdfTerm,
    predicate: &str,
    object: &RdfTerm,
) -> Result<()> {
    if is_structural_owl_predicate(predicate) {
        return Ok(());
    }
    match object {
        RdfTerm::Literal {
            value,
            datatype,
            language,
            ..
        } => {
            let literal = rdf_term_literal_to_literal(value, datatype, language);
            if is_annotation_property(predicate, ontology) {
                let value = AnnotationValue::Literal(literal);
                return push_annotation_assertion(ontology, subject, predicate, value);
            }
            let Some(individual) = term_to_individual(subject) else {
                return Ok(());
            };
            let axiom = DataPropertyAssertionAxiom {
                id: generate_axiom_id(),
                property: DataPropertyExpression::DataProperty(DataProperty {
                    iri: IRI::new(predicate),
                }),
                individual,
                value: literal,
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::DataPropertyAssertion(axiom));
            Ok(())
        }
        RdfTerm::Iri(obj) => match subject {
            RdfTerm::Iri(sub) => process_triple(ontology, sub.as_str(), predicate, obj.as_str()),
            RdfTerm::BlankNode(id) => {
                if predicate == RDF_TYPE {
                    if is_wellknown_vocab_term(obj.as_str()) {
                        // e.g. `_:b a owl:Restriction` — structural marker.
                        return Ok(());
                    }
                    let axiom = ClassAssertionAxiom {
                        id: generate_axiom_id(),
                        individual: Individual::Anonymous(AnonymousIndividual { id: id.clone() }),
                        class: ClassExpression::Class(Class::new(IRI::new(obj.as_str()))),
                        annotations: vec![],
                    };
                    ontology.add_axiom(Axiom::ClassAssertion(axiom));
                    return Ok(());
                }
                if is_annotation_property(predicate, ontology) {
                    let value = AnnotationValue::IRI(IRI::new(obj.as_str()));
                    return push_annotation_assertion(ontology, subject, predicate, value);
                }
                let Some(source) = term_to_individual(subject) else {
                    return Ok(());
                };
                let target = Individual::Named(NamedIndividual {
                    iri: IRI::new(obj.as_str()),
                });
                let property = ObjectProperty::new(IRI::new(predicate))?;
                let axiom = ObjectPropertyAssertionAxiom {
                    id: generate_axiom_id(),
                    property: ObjectPropertyExpression::ObjectProperty(property),
                    source,
                    target,
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::ObjectPropertyAssertion(axiom));
                Ok(())
            }
            _ => Ok(()),
        },
        RdfTerm::BlankNode(obj_id) => {
            // Anonymous object individual.  `rdf:type <bnode>` is handled by
            // the expression resolver in `map_rdf_triples`; anything reaching
            // here without resolution is skipped.
            if predicate == RDF_TYPE {
                return Ok(());
            }
            let target = Individual::Anonymous(AnonymousIndividual { id: obj_id.clone() });
            if is_annotation_property(predicate, ontology) {
                let value = AnnotationValue::AnonymousIndividual(AnonymousIndividual {
                    id: obj_id.clone(),
                });
                return push_annotation_assertion(ontology, subject, predicate, value);
            }
            let Some(source) = term_to_individual(subject) else {
                return Ok(());
            };
            let property = ObjectProperty::new(IRI::new(predicate))?;
            let axiom = ObjectPropertyAssertionAxiom {
                id: generate_axiom_id(),
                property: ObjectPropertyExpression::ObjectProperty(property),
                source,
                target,
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::ObjectPropertyAssertion(axiom));
            Ok(())
        }
        // RDF-star quoted triples are out of scope for OWL axioms here.
        _ => Ok(()),
    }
}

fn push_annotation_assertion(
    ontology: &mut Ontology,
    subject: &RdfTerm,
    predicate: &str,
    value: AnnotationValue,
) -> Result<()> {
    match subject {
        RdfTerm::Iri(sub) => {
            if ontology.get_iri().map(IRI::as_str) == Some(sub.as_str()) {
                ontology.annotations.push(Annotation {
                    property: AnnotationProperty {
                        iri: IRI::new(predicate),
                    },
                    value,
                    annotations: vec![],
                });
            } else {
                ensure_annotation_property_declared(predicate, ontology);
                let axiom = AnnotationAssertionAxiom {
                    id: generate_axiom_id(),
                    subject: AnnotationSubject::IRI(IRI::new(sub.as_str())),
                    property: AnnotationProperty {
                        iri: IRI::new(predicate),
                    },
                    value,
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::AnnotationAssertion(axiom));
            }
        }
        RdfTerm::BlankNode(id) => {
            ensure_annotation_property_declared(predicate, ontology);
            let axiom = AnnotationAssertionAxiom {
                id: generate_axiom_id(),
                subject: AnnotationSubject::AnonymousIndividual(AnonymousIndividual {
                    id: id.clone(),
                }),
                property: AnnotationProperty {
                    iri: IRI::new(predicate),
                },
                value,
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::AnnotationAssertion(axiom));
        }
        _ => {}
    }
    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════════
// Whole-graph mapping with blank-node reconstruction
// ═══════════════════════════════════════════════════════════════════════════

/// Per-graph indexes needed to reconstruct blank-node class expressions.
struct BNodeIndex<'a> {
    /// blank node id → its outgoing (predicate, object) pairs
    nodes: HashMap<&'a str, Vec<(&'a str, &'a RdfTerm)>>,
    /// IRIs declared as `owl:DatatypeProperty`
    data_properties: HashSet<&'a str>,
}

impl<'a> BNodeIndex<'a> {
    fn build(triples: &'a [RdfTriple]) -> Self {
        let mut nodes: HashMap<&str, Vec<(&str, &RdfTerm)>> = HashMap::new();
        let mut data_properties = HashSet::new();
        for t in triples {
            if let RdfTerm::BlankNode(subj_id) = &t.subject
                && let RdfTerm::Iri(pred_url) = &t.predicate
            {
                nodes
                    .entry(subj_id.as_str())
                    .or_default()
                    .push((pred_url.as_str(), &t.object));
            }
            if let (RdfTerm::Iri(subj), RdfTerm::Iri(pred), RdfTerm::Iri(obj)) =
                (&t.subject, &t.predicate, &t.object)
                && pred.as_str() == RDF_TYPE
            {
                if obj.as_str() == OWL_DATATYPE_PROPERTY {
                    data_properties.insert(subj.as_str());
                }
            }
        }
        Self {
            nodes,
            data_properties,
        }
    }

    /// Outgoing triples of a blank node.
    fn entries(&self, id: &str) -> Option<&[(&'a str, &'a RdfTerm)]> {
        self.nodes.get(id).map(Vec::as_slice)
    }

    /// First object of `predicate` from a blank node's description.
    fn find(&self, id: &str, predicate: &str) -> Option<&'a RdfTerm> {
        self.nodes
            .get(id)?
            .iter()
            .find(|(p, _)| *p == predicate)
            .map(|(_, o)| *o)
    }
}

fn as_iri(term: &RdfTerm) -> Option<&str> {
    match term {
        RdfTerm::Iri(url) => Some(url.as_str()),
        _ => None,
    }
}

fn as_u32_literal(term: &RdfTerm) -> Option<u32> {
    match term {
        RdfTerm::Literal { value, .. } => value.parse::<u32>().ok(),
        _ => None,
    }
}

/// Follow an RDF list from `head` until `rdf:nil`.
fn resolve_list<'a>(index: &BNodeIndex<'a>, head: &'a RdfTerm) -> Option<Vec<&'a RdfTerm>> {
    let mut items = Vec::new();
    let mut visited: HashSet<&str> = HashSet::new();
    let mut current = head;
    loop {
        match current {
            RdfTerm::Iri(url) if url.as_str() == RDF_NIL => return Some(items),
            RdfTerm::BlankNode(id) => {
                if !visited.insert(id.as_str()) {
                    return None; // cycle guard
                }
                if items.len() > 10_000 {
                    return None;
                }
                let first = index.find(id, RDF_FIRST)?;
                items.push(first);
                current = index.find(id, RDF_REST)?;
            }
            _ => return None,
        }
    }
}

/// Reconstruct a class expression from a term (named class IRI or blank node).
fn resolve_class_expression(
    term: &RdfTerm,
    index: &BNodeIndex<'_>,
    depth: usize,
) -> Option<ClassExpression> {
    if depth > 32 {
        return None;
    }
    match term {
        RdfTerm::Iri(url) => Some(ClassExpression::Class(Class::new(IRI::new(url.as_str())))),
        RdfTerm::BlankNode(id) => {
            let entries = index.entries(id)?;
            let has = |p: &str| entries.iter().any(|(q, _)| *q == p);
            if has(OWL_ON_PROPERTY) {
                return resolve_restriction(id, index, depth);
            }
            if has(OWL_INTERSECTION_OF) {
                let head = index.find(id, OWL_INTERSECTION_OF)?;
                let items = resolve_list(index, head)?;
                let exprs: Vec<ClassExpression> = items
                    .into_iter()
                    .filter_map(|i| resolve_class_expression(i, index, depth + 1))
                    .collect();
                return Some(ClassExpression::intersection_of(exprs));
            }
            if has(OWL_UNION_OF) {
                let head = index.find(id, OWL_UNION_OF)?;
                let items = resolve_list(index, head)?;
                let exprs: Vec<ClassExpression> = items
                    .into_iter()
                    .filter_map(|i| resolve_class_expression(i, index, depth + 1))
                    .collect();
                return Some(ClassExpression::union_of(exprs));
            }
            if let Some(inner) = index.find(id, OWL_COMPLEMENT_OF) {
                let expr = resolve_class_expression(inner, index, depth + 1)?;
                return Some(ClassExpression::ObjectComplementOf(Box::new(expr)));
            }
            if has(OWL_ONE_OF) {
                let head = index.find(id, OWL_ONE_OF)?;
                let items = resolve_list(index, head)?;
                let individuals: Vec<Individual> =
                    items.into_iter().filter_map(term_to_individual).collect();
                if individuals.is_empty() {
                    return None;
                }
                return Some(ClassExpression::ObjectOneOf(individuals));
            }
            None
        }
        _ => None,
    }
}

/// Decide whether a restriction filler is a data range (data restriction)
/// rather than a class expression (object restriction).
fn filler_is_data(
    entries: &[(&str, &RdfTerm)],
    index: &BNodeIndex<'_>,
    property: &str,
    filler: &RdfTerm,
) -> bool {
    if entries.iter().any(|(p, _)| *p == OWL_ON_DATA_RANGE) {
        return true;
    }
    if index.data_properties.contains(property) {
        return true;
    }
    match filler {
        RdfTerm::Literal { .. } => true,
        RdfTerm::Iri(url) => is_datatype_iri(url.as_str()),
        RdfTerm::BlankNode(_) => false,
        _ => false,
    }
}

/// Reconstruct an `owl:Restriction` blank node into a class expression.
fn resolve_restriction(id: &str, index: &BNodeIndex<'_>, depth: usize) -> Option<ClassExpression> {
    let entries = index.entries(id)?;
    let prop_term = index.find(id, OWL_ON_PROPERTY)?;
    let property_iri = as_iri(prop_term)?;
    let is_data_prop = index.data_properties.contains(property_iri);

    let object_property = ObjectPropertyExpression::ObjectProperty(ObjectProperty {
        iri: IRI::new(property_iri),
    });
    let data_property = DataPropertyExpression::DataProperty(DataProperty {
        iri: IRI::new(property_iri),
    });

    // ── filler-based restrictions ───────────────────────────────────────
    for predicate in [OWL_SOME_VALUES_FROM, OWL_ALL_VALUES_FROM] {
        if let Some(filler_term) = index.find(id, predicate) {
            let data = filler_is_data(entries, index, property_iri, filler_term);
            let is_some = predicate == OWL_SOME_VALUES_FROM;
            if data {
                let filler = resolve_data_range(filler_term, index, depth + 1)?;
                return Some(if is_some {
                    ClassExpression::DataSomeValuesFrom {
                        property: data_property,
                        filler,
                    }
                } else {
                    ClassExpression::DataAllValuesFrom {
                        property: data_property,
                        filler,
                    }
                });
            }
            let filler = resolve_class_expression(filler_term, index, depth + 1)?;
            return Some(if is_some {
                ClassExpression::ObjectSomeValuesFrom {
                    property: object_property,
                    filler: Box::new(filler),
                }
            } else {
                ClassExpression::ObjectAllValuesFrom {
                    property: object_property,
                    filler: Box::new(filler),
                }
            });
        }
    }

    // ── hasValue ────────────────────────────────────────────────────────
    if let Some(value_term) = index.find(id, OWL_HAS_VALUE) {
        match value_term {
            RdfTerm::Literal {
                value,
                datatype,
                language,
                ..
            } => {
                return Some(ClassExpression::DataHasValue {
                    property: data_property,
                    value: rdf_term_literal_to_literal(value, datatype, language),
                });
            }
            _ if !is_data_prop => {
                let value = term_to_individual(value_term)?;
                return Some(ClassExpression::ObjectHasValue {
                    property: object_property,
                    value,
                });
            }
            _ => return None,
        }
    }

    // ── hasSelf ─────────────────────────────────────────────────────────
    if let Some(true_term) = index.find(id, OWL_HAS_SELF) {
        if matches!(true_term, RdfTerm::Literal { value, .. } if value == "true") {
            return Some(ClassExpression::ObjectHasSelf {
                property: object_property,
            });
        }
    }

    // ── cardinalities ───────────────────────────────────────────────────
    let cardinality_predicates = [
        (OWL_MIN_QUALIFIED_CARDINALITY, OWL_MIN_CARDINALITY),
        (OWL_MAX_QUALIFIED_CARDINALITY, OWL_MAX_CARDINALITY),
        (OWL_QUALIFIED_CARDINALITY, OWL_CARDINALITY),
    ];
    for (qualified, unqualified) in cardinality_predicates {
        let card = if let Some(t) = index.find(id, qualified) {
            as_u32_literal(t)?
        } else if let Some(t) = index.find(id, unqualified) {
            as_u32_literal(t)?
        } else {
            continue;
        };
        let is_data = is_data_prop || index.find(id, OWL_ON_DATA_RANGE).is_some();
        if is_data {
            let filler = match index.find(id, OWL_ON_DATA_RANGE) {
                Some(t) => resolve_data_range(t, index, depth + 1)?,
                None => DataRange::Datatype(IRI::new(RDFS_LITERAL)),
            };
            return Some(if qualified == OWL_MIN_QUALIFIED_CARDINALITY {
                ClassExpression::DataMinCardinality {
                    property: data_property,
                    cardinality: card,
                    filler,
                }
            } else if qualified == OWL_MAX_QUALIFIED_CARDINALITY {
                ClassExpression::DataMaxCardinality {
                    property: data_property,
                    cardinality: card,
                    filler,
                }
            } else {
                ClassExpression::DataExactCardinality {
                    property: data_property,
                    cardinality: card,
                    filler,
                }
            });
        }
        let filler = match index.find(id, OWL_ON_CLASS) {
            Some(t) => resolve_class_expression(t, index, depth + 1)?,
            None => ClassExpression::Class(Class::new(IRI::new(OWL_THING))),
        };
        return Some(if qualified == OWL_MIN_QUALIFIED_CARDINALITY {
            ClassExpression::ObjectMinCardinality {
                property: object_property,
                cardinality: card,
                filler: Box::new(filler),
            }
        } else if qualified == OWL_MAX_QUALIFIED_CARDINALITY {
            ClassExpression::ObjectMaxCardinality {
                property: object_property,
                cardinality: card,
                filler: Box::new(filler),
            }
        } else {
            ClassExpression::ObjectExactCardinality {
                property: object_property,
                cardinality: card,
                filler: Box::new(filler),
            }
        });
    }

    None
}

/// Reconstruct a data range from a term (datatype IRI or blank node).
fn resolve_data_range(term: &RdfTerm, index: &BNodeIndex<'_>, depth: usize) -> Option<DataRange> {
    if depth > 32 {
        return None;
    }
    match term {
        RdfTerm::Iri(url) => Some(DataRange::Datatype(IRI::new(url.as_str()))),
        RdfTerm::BlankNode(id) => {
            let entries = index.entries(id)?;
            let has = |p: &str| entries.iter().any(|(q, _)| *q == p);
            if has(OWL_ON_DATATYPE) {
                let base = index.find(id, OWL_ON_DATATYPE)?;
                let base_iri = as_iri(base)?;
                let head = index.find(id, OWL_WITH_RESTRICTIONS)?;
                let facets = resolve_list(index, head)?;
                let mut restrictions = Vec::new();
                for facet_node in facets {
                    let RdfTerm::BlankNode(facet_id) = facet_node else {
                        return None;
                    };
                    let facet_entries = index.entries(facet_id)?;
                    let (facet_pred, value) = facet_entries.first()?;
                    if let RdfTerm::Literal {
                        value: lit_value,
                        datatype: lit_dt,
                        language: lit_lang,
                        ..
                    } = value
                    {
                        restrictions.push(FacetRestriction {
                            facet: IRI::new(*facet_pred),
                            value: rdf_term_literal_to_literal(lit_value, lit_dt, lit_lang),
                        });
                    } else {
                        return None;
                    }
                }
                return Some(DataRange::DatatypeRestriction {
                    datatype: IRI::new(base_iri),
                    restrictions,
                });
            }
            if has(OWL_INTERSECTION_OF) {
                let head = index.find(id, OWL_INTERSECTION_OF)?;
                let items = resolve_list(index, head)?;
                let ranges: Vec<DataRange> = items
                    .into_iter()
                    .filter_map(|i| resolve_data_range(i, index, depth + 1))
                    .collect();
                return Some(DataRange::DataIntersectionOf(ranges));
            }
            if has(OWL_UNION_OF) {
                let head = index.find(id, OWL_UNION_OF)?;
                let items = resolve_list(index, head)?;
                let ranges: Vec<DataRange> = items
                    .into_iter()
                    .filter_map(|i| resolve_data_range(i, index, depth + 1))
                    .collect();
                return Some(DataRange::DataUnionOf(ranges));
            }
            if let Some(inner) = index.find(id, OWL_COMPLEMENT_OF) {
                let range = resolve_data_range(inner, index, depth + 1)?;
                return Some(DataRange::DataComplementOf(Box::new(range)));
            }
            if has(OWL_ONE_OF) {
                let head = index.find(id, OWL_ONE_OF)?;
                let items = resolve_list(index, head)?;
                let literals: Vec<Literal> = items
                    .into_iter()
                    .filter_map(|i| match i {
                        RdfTerm::Literal {
                            value,
                            datatype,
                            language,
                            ..
                        } => Some(rdf_term_literal_to_literal(value, datatype, language)),
                        _ => None,
                    })
                    .collect();
                if literals.is_empty() {
                    return None;
                }
                return Some(DataRange::DataOneOf(literals));
            }
            None
        }
        _ => None,
    }
}

/// Map a full RDF graph into an ontology, reconstructing blank-node class
/// expressions (restrictions, intersections, enumerations, …) before falling
/// back to the shared per-triple mapping.
///
/// Blank-node subjects that are *not* consumed as expressions become
/// anonymous-individual assertions (nested node support); structural
/// restriction internals are skipped.
pub fn map_rdf_triples<'a>(triples: &'a [RdfTriple], ontology: &mut Ontology) -> Result<()> {
    let index = BNodeIndex::build(triples);
    let mut processed = vec![false; triples.len()];

    // ── Pre-pass 0: oxidowl preservation triples (CR-011 M2) — SWRL rules
    //    stored as functional-syntax text literals under
    //    `urn:oxidowl:swrlRule`, and ontology-level annotations of an
    //    IRI-less ontology stored under the `urn:oxidowl:ontology` fallback
    //    subject.  The shared writer emits both unconditionally; decoding
    //    them here is what makes them survive JSON-LD / HDT / BinaryRDF /
    //    RDFa (and N3 ground statements) round-trips. ─────────────────────
    for (i, t) in triples.iter().enumerate() {
        if processed[i] {
            continue;
        }
        let RdfTerm::Iri(pred) = &t.predicate else {
            continue;
        };
        if pred.as_str() == owl_rdf_mapping::SWRL_RULE_PROPERTY {
            match &t.object {
                RdfTerm::Literal { value, .. } => {
                    match crate::parsers::functional::FunctionalParser::new()
                        .parse_swrl_rule_text(value)
                    {
                        Ok(rule) => {
                            ontology.add_axiom(Axiom::Rule(rule));
                        }
                        Err(e) => {
                            log::warn!(
                                "rdf_to_owl: preserved SWRL rule text did not parse ({e}); \
                                 keeping the text as an ontology annotation"
                            );
                            ontology.annotations.push(Annotation {
                                property: AnnotationProperty {
                                    iri: IRI::new(pred.as_str()),
                                },
                                value: term_to_annotation_value(&t.object).unwrap_or_else(|| {
                                    AnnotationValue::Literal(Literal::new(value.clone()))
                                }),
                                annotations: vec![],
                            });
                        }
                    }
                }
                other => {
                    if let Some(value) = term_to_annotation_value(other) {
                        ontology.annotations.push(Annotation {
                            property: AnnotationProperty {
                                iri: IRI::new(pred.as_str()),
                            },
                            value,
                            annotations: vec![],
                        });
                    }
                }
            }
            processed[i] = true;
            continue;
        }
        if matches!(&t.subject, RdfTerm::Iri(s) if s.as_str() == owl_rdf_mapping::ONTOLOGY_FALLBACK_IRI)
        {
            if let Some(value) = term_to_annotation_value(&t.object) {
                ontology.annotations.push(Annotation {
                    property: AnnotationProperty {
                        iri: IRI::new(pred.as_str()),
                    },
                    value,
                    annotations: vec![],
                });
            } else {
                log::warn!("rdf_to_owl: dropping ontology annotation with unsupported value");
            }
            processed[i] = true;
        }
    }

    // ── Pre-pass 1: `:x rdf:type owl:Ontology` (so ontology-level
    //    annotations in the main pass can find the ontology IRI). ────────
    for (i, t) in triples.iter().enumerate() {
        if let (RdfTerm::Iri(sub), RdfTerm::Iri(pred), RdfTerm::Iri(obj)) =
            (&t.subject, &t.predicate, &t.object)
            && pred.as_str() == RDF_TYPE
            && obj.as_str() == OWL_ONTOLOGY
        {
            process_triple(ontology, sub.as_str(), pred.as_str(), obj.as_str())?;
            processed[i] = true;
        }
    }

    // ── Pre-pass 2: entity declarations, in document order, so that
    //    `is_data_property` / `is_annotation_property` see them regardless of
    //    where the corresponding assertion appears in the document. ──────
    for (i, t) in triples.iter().enumerate() {
        if processed[i] {
            continue;
        }
        if let (RdfTerm::Iri(sub), RdfTerm::Iri(pred), RdfTerm::Iri(obj)) =
            (&t.subject, &t.predicate, &t.object)
            && pred.as_str() == RDF_TYPE
            && matches!(
                obj.as_str(),
                OWL_CLASS
                    | OWL_OBJECT_PROPERTY
                    | OWL_DATATYPE_PROPERTY
                    | OWL_ANNOTATION_PROPERTY
                    | OWL_NAMED_INDIVIDUAL
            )
        {
            process_triple(ontology, sub.as_str(), pred.as_str(), obj.as_str())?;
            processed[i] = true;
        }
    }

    // ── Main pass ───────────────────────────────────────────────────────
    for (i, t) in triples.iter().enumerate() {
        if processed[i] {
            continue;
        }
        let RdfTerm::Iri(pred_url) = &t.predicate else {
            continue; // blank-node predicate is not valid RDF
        };
        let predicate = pred_url.as_str();
        let subject = &t.subject;
        let object = &t.object;

        match predicate {
            // rdf:type with a blank side — expression or anonymous typing.
            RDF_TYPE => match object {
                RdfTerm::BlankNode(_) => {
                    if let RdfTerm::Iri(sub) = subject
                        && let Some(ce) = resolve_class_expression(object, &index, 0)
                    {
                        let axiom = ClassAssertionAxiom {
                            id: generate_axiom_id(),
                            individual: Individual::Named(NamedIndividual {
                                iri: IRI::new(sub.as_str()),
                            }),
                            class: ce,
                            annotations: vec![],
                        };
                        ontology.add_axiom(Axiom::ClassAssertion(axiom));
                    }
                }
                RdfTerm::Iri(_) => {
                    // Handled by the shared `process_term_triple` call below
                    // (the fall-through at the end of this pass): a blank
                    // subject with a custom class becomes a ClassAssertion, a
                    // well-known vocabulary term stays a structural marker.
                    // Calling it here as well would create the axiom twice.
                }
                _ => {}
            },

            // Class-expression positions: reconstruct blank sides.
            RDFS_SUBCLASS_OF | OWL_EQUIVALENT_CLASS | OWL_DISJOINT_WITH => {
                let sub_blank = matches!(subject, RdfTerm::BlankNode(_));
                let obj_blank = matches!(object, RdfTerm::BlankNode(_));
                if sub_blank || obj_blank {
                    if let (Some(sub_ce), Some(obj_ce)) = (
                        resolve_class_expression(subject, &index, 0),
                        resolve_class_expression(object, &index, 0),
                    ) {
                        let id = generate_axiom_id();
                        match predicate {
                            RDFS_SUBCLASS_OF => {
                                ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                                    id,
                                    subclass: sub_ce,
                                    superclass: obj_ce,
                                    annotations: vec![],
                                }))
                            }
                            OWL_EQUIVALENT_CLASS => {
                                ontology.add_axiom(Axiom::EquivalentClasses(
                                    EquivalentClassesAxiom {
                                        id,
                                        classes: vec![sub_ce, obj_ce],
                                        annotations: vec![],
                                    },
                                ));
                            }
                            _ => {
                                ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
                                    id,
                                    classes: vec![sub_ce, obj_ce],
                                    annotations: vec![],
                                }));
                            }
                        }
                    } else {
                        log::warn!(
                            "JSON-LD/RDF: could not reconstruct blank-node class expression in {predicate} triple; skipping"
                        );
                    }
                    continue;
                }
                if let (RdfTerm::Iri(sub), RdfTerm::Iri(obj)) = (subject, object) {
                    process_triple(ontology, sub.as_str(), predicate, obj.as_str())?;
                }
                continue;
            }

            // List-valued single-triple axioms.
            OWL_DISJOINT_UNION_OF => {
                if let (RdfTerm::Iri(sub), RdfTerm::BlankNode(_)) = (subject, object)
                    && let Some(items) = resolve_list(&index, object)
                {
                    let members: Vec<ClassExpression> = items
                        .into_iter()
                        .filter_map(|i| resolve_class_expression(i, &index, 1))
                        .collect();
                    if !members.is_empty() {
                        ontology.add_axiom(Axiom::DisjointUnion(DisjointUnionAxiom {
                            id: generate_axiom_id(),
                            class: ClassExpression::Class(Class::new(IRI::new(sub.as_str()))),
                            disjoint_classes: members,
                            annotations: vec![],
                        }));
                    }
                }
                continue;
            }
            OWL_PROPERTY_CHAIN_AXIOM => {
                if let (RdfTerm::Iri(sub), RdfTerm::BlankNode(_)) = (subject, object)
                    && let Some(items) = resolve_list(&index, object)
                {
                    let chain: Vec<ObjectPropertyExpression> = items
                        .into_iter()
                        .filter_map(|i| {
                            as_iri(i).map(|iri| {
                                ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                                    iri: IRI::new(iri),
                                })
                            })
                        })
                        .collect();
                    if !chain.is_empty() {
                        ontology.add_axiom(Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
                            id: generate_axiom_id(),
                            sub_property: ObjectPropertyExpression::PropertyChain(chain),
                            super_property: ObjectPropertyExpression::ObjectProperty(
                                ObjectProperty {
                                    iri: IRI::new(sub.as_str()),
                                },
                            ),
                            annotations: vec![],
                        }));
                    }
                }
                continue;
            }
            OWL_HAS_KEY => {
                if let RdfTerm::BlankNode(_) = object
                    && let Some(items) = resolve_list(&index, object)
                    && let Some(ce) = resolve_class_expression(subject, &index, 0)
                {
                    let mut object_properties = Vec::new();
                    let mut data_properties = Vec::new();
                    for item in items {
                        if let Some(iri) = as_iri(item) {
                            if index.data_properties.contains(iri) {
                                data_properties.push(DataPropertyExpression::DataProperty(
                                    DataProperty { iri: IRI::new(iri) },
                                ));
                            } else {
                                object_properties.push(ObjectPropertyExpression::ObjectProperty(
                                    ObjectProperty { iri: IRI::new(iri) },
                                ));
                            }
                        }
                    }
                    ontology.add_axiom(Axiom::HasKey(HasKeyAxiom {
                        id: generate_axiom_id(),
                        class: ce,
                        object_properties,
                        data_properties,
                        annotations: vec![],
                    }));
                }
                continue;
            }

            _ => {}
        }

        // Everything else: shared term-level mapping (skips structural
        // predicates, routes literals/annotations, keeps nested nodes as
        // anonymous individuals).
        process_term_triple(ontology, subject, predicate, object)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triple(s: &str, p: &str, o: RdfTerm) -> RdfTriple {
        RdfTriple {
            subject: RdfTerm::Iri(url::Url::parse(s).unwrap()),
            predicate: RdfTerm::Iri(url::Url::parse(p).unwrap()),
            object: o,
        }
    }

    fn bnode_triple(s: &str, p: &str, o: RdfTerm) -> RdfTriple {
        RdfTriple {
            subject: RdfTerm::BlankNode(s.to_string()),
            predicate: RdfTerm::Iri(url::Url::parse(p).unwrap()),
            object: o,
        }
    }

    fn iri(s: &str) -> RdfTerm {
        RdfTerm::Iri(url::Url::parse(s).unwrap())
    }

    #[test]
    fn maps_simple_triples_like_turtle() {
        let triples = vec![
            triple(
                "http://ex.org/A",
                RDF_TYPE,
                iri("http://www.w3.org/2002/07/owl#Class"),
            ),
            triple("http://ex.org/A", RDFS_SUBCLASS_OF, iri("http://ex.org/B")),
            triple("http://ex.org/a", RDF_TYPE, iri("http://ex.org/A")),
        ];
        let mut o = Ontology::new();
        map_rdf_triples(&triples, &mut o).expect("map");
        assert!(o.axioms().iter().any(|a| matches!(
            a,
            Axiom::Declaration(d) if matches!(d.entity, Entity::Class(_))
        )));
        assert!(o.axioms().iter().any(|a| matches!(a, Axiom::SubClassOf(_))));
        assert!(
            o.axioms()
                .iter()
                .any(|a| matches!(a, Axiom::ClassAssertion(_)))
        );
    }

    #[test]
    fn reconstructs_some_values_from_restriction() {
        let triples = vec![
            triple(
                "http://ex.org/A",
                OWL_EQUIVALENT_CLASS,
                RdfTerm::BlankNode("b0".into()),
            ),
            bnode_triple("b0", RDF_TYPE, iri(OWL_RESTRICTION)),
            bnode_triple("b0", OWL_ON_PROPERTY, iri("http://ex.org/p")),
            bnode_triple("b0", OWL_SOME_VALUES_FROM, iri("http://ex.org/B")),
        ];
        let mut o = Ontology::new();
        map_rdf_triples(&triples, &mut o).expect("map");
        let eq: Vec<_> = o
            .axioms()
            .iter()
            .filter(|a| matches!(a, Axiom::EquivalentClasses(_)))
            .collect();
        assert_eq!(eq.len(), 1, "exactly one EquivalentClasses axiom: {eq:?}");
        match &eq[0] {
            Axiom::EquivalentClasses(a) => {
                assert_eq!(a.classes.len(), 2);
                assert!(
                    matches!(&a.classes[1], ClassExpression::ObjectSomeValuesFrom { .. }),
                    "second operand must be a restriction, got {:?}",
                    a.classes[1]
                );
            }
            _ => unreachable!(),
        }
        // No garbage ClassAssertion for the restriction node itself.
        assert!(
            !o.axioms()
                .iter()
                .any(|a| matches!(a, Axiom::ClassAssertion(_))),
            "restriction internals must not become class assertions"
        );
    }

    #[test]
    fn reconstructs_data_restriction_for_datatype_filler() {
        let triples = vec![
            triple(
                "http://ex.org/A",
                OWL_EQUIVALENT_CLASS,
                RdfTerm::BlankNode("b0".into()),
            ),
            bnode_triple("b0", RDF_TYPE, iri(OWL_RESTRICTION)),
            bnode_triple("b0", OWL_ON_PROPERTY, iri("http://ex.org/P")),
            bnode_triple("b0", OWL_SOME_VALUES_FROM, iri(&format!("{XSD_PREFIX}int"))),
        ];
        let mut o = Ontology::new();
        map_rdf_triples(&triples, &mut o).expect("map");
        let found = o.axioms().iter().any(|a| {
            matches!(a, Axiom::EquivalentClasses(eq)
                if eq.classes.iter().any(|c| matches!(c, ClassExpression::DataSomeValuesFrom { .. })))
        });
        assert!(found, "expected DataSomeValuesFrom, got {:?}", o.axioms());
    }

    #[test]
    fn structural_predicates_never_become_assertions() {
        let triples = vec![bnode_triple(
            "b0",
            "http://www.w3.org/2002/07/owl#hasSelf",
            RdfTerm::Literal {
                value: "true".to_string(),
                datatype: None,
                language: None,
                direction: None,
            },
        )];
        let mut o = Ontology::new();
        map_rdf_triples(&triples, &mut o).expect("map");
        assert!(o.axioms().is_empty(), "got {:?}", o.axioms());
    }
}

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::*;
use oxidowl::parsers::*;

// ══════════════════════════════════════════════════════════════════════════════
// Parser Error Handling Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn parser_invalid_functional_syntax() {
    let result = parse_functional("Not valid functional syntax at all");
    assert!(result.is_err());
}

#[test]
fn parser_invalid_owl_xml() {
    let result = parse_owl_xml("<NotOWL>invalid</NotOWL>");
    assert!(result.is_ok()); // Parser is lenient; returns empty ontology
    assert!(result.unwrap().axioms().is_empty());
}

#[test]
fn parser_invalid_rdf_xml() {
    let result = parse_rdf_xml("<rdf:NotValid></rdf:NotValid>");
    assert!(result.is_ok()); // Parser is lenient
}

#[test]
fn parser_invalid_turtle() {
    let result = parse_turtle("@prefix : .");
    assert!(result.is_err());
}

#[test]
fn parser_invalid_ntriples() {
    let result = parse_ntriples("not a valid ntriples line");
    assert!(result.is_err());
}

// ══════════════════════════════════════════════════════════════════════════════
// Format Detection Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn format_detect_extension() {
    assert_eq!(
        OntologyFormat::from_extension("ofn"),
        Some(OntologyFormat::Functional)
    );
    assert_eq!(
        OntologyFormat::from_extension("ttl"),
        Some(OntologyFormat::Turtle)
    );
    assert_eq!(
        OntologyFormat::from_extension("rdf"),
        Some(OntologyFormat::RdfXml)
    );
    assert_eq!(
        OntologyFormat::from_extension("nt"),
        Some(OntologyFormat::NTriples)
    );
    assert_eq!(
        OntologyFormat::from_extension("omn"),
        Some(OntologyFormat::Manchester)
    );
    assert_eq!(
        OntologyFormat::from_extension("jsonld"),
        Some(OntologyFormat::JsonLd)
    );
}

#[test]
fn format_detect_unknown_extension() {
    assert_eq!(OntologyFormat::from_extension("xyz"), None);
}

// ══════════════════════════════════════════════════════════════════════════════
// Serializer Config Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn serializer_config_default() {
    let config = SerializerConfig::default();
    assert_eq!(config.indent_size, 2);
    assert!(config.pretty_print);
}

// ══════════════════════════════════════════════════════════════════════════════
// Parser Config Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn parser_config_standard() {
    let _config = ParserConfig::standard();
}

#[test]
fn parser_config_minimal() {
    let _config = ParserConfig::minimal();
}

// ══════════════════════════════════════════════════════════════════════════════
// Manchester Parser Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn manchester_parser_prefix_declarations() {
    let config = ManchesterParserConfig::default();
    let mut parser = ManchesterParser::new(config);
    let result = parser.parse_string("Prefix: ex: <http://example.org/>");
    assert!(result.is_ok());
}

#[test]
fn manchester_parser_class_frame() {
    let config = ManchesterParserConfig::default();
    let mut parser = ManchesterParser::new(config);
    let input = concat!(
        "Prefix: ex: <http://example.org/>\n",
        "Class: ex:Person\n",
        "    SubClassOf: ex:Animal\n"
    );
    let result = parser.parse_string(input);
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// Turtle `a` shorthand (B5 Slice 0 — Part D)
// ══════════════════════════════════════════════════════════════════════════════

/// Turtle's predefined keyword `a` in *predicate* position must resolve to
/// `rdf:type` (Turtle spec §2.3 / §4.1.2), never to an object property
/// literally named "a".
#[test]
fn turtle_predicate_a_shorthand_is_rdf_type() {
    let ont = parse_turtle(concat!(
        "@prefix : <http://example.org/> .\n",
        "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n",
        ":A a owl:Class .\n",
        ":i a :A .\n",
    ))
    .expect("Turtle documents using the `a` shorthand must parse");

    // `:A a owl:Class` ⇒ Declaration(Class :A)
    let declares_a = ont.axioms().iter().any(|ax| match ax {
        Axiom::Declaration(d) => {
            matches!(&d.entity, Entity::Class(iri) if iri.as_str() == "http://example.org/A")
        }
        _ => false,
    });
    assert!(
        declares_a,
        "`:A a owl:Class` must declare class A; axioms: {:?}",
        ont.axioms()
    );

    // `:i a :A` ⇒ ClassAssertion(A, i)
    let asserts_i = ont.axioms().iter().any(|ax| match ax {
        Axiom::ClassAssertion(ca) => {
            ca.individual
                .iri()
                .is_some_and(|i| i.as_str() == "http://example.org/i")
                && matches!(&ca.class, ClassExpression::Class(c)
                    if c.iri.as_str() == "http://example.org/A")
        }
        _ => false,
    });
    assert!(
        asserts_i,
        "`:i a :A` must produce ClassAssertion(:A, :i); axioms: {:?}",
        ont.axioms()
    );

    // The bare predicate `a` must not leak through as an object property IRI.
    let spurious = ont.axioms().iter().any(|ax| match ax {
        Axiom::ObjectPropertyAssertion(opa) => {
            matches!(&opa.property, ObjectPropertyExpression::ObjectProperty(p)
                if p.iri.as_str() == "a")
        }
        _ => false,
    });
    assert!(
        !spurious,
        "predicate `a` must not become an ObjectPropertyAssertion; axioms: {:?}",
        ont.axioms()
    );
}

/// The `a` shorthand must also work in semicolon-separated predicate lists.
#[test]
fn turtle_predicate_a_shorthand_in_semicolon_list() {
    let ont = parse_turtle(concat!(
        "@prefix : <http://example.org/> .\n",
        "@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n",
        ":A a owl:Class ; rdfs:subClassOf :B .\n",
    ))
    .expect("Turtle `a` inside a semicolon list must parse");

    let declares_a = ont.axioms().iter().any(|ax| match ax {
        Axiom::Declaration(d) => {
            matches!(&d.entity, Entity::Class(iri) if iri.as_str() == "http://example.org/A")
        }
        _ => false,
    });
    assert!(declares_a, "axioms: {:?}", ont.axioms());

    let sub = ont.axioms().iter().any(|ax| match ax {
        Axiom::SubClassOf(s) => {
            matches!(&s.subclass, ClassExpression::Class(c)
                if c.iri.as_str() == "http://example.org/A")
                && matches!(&s.superclass, ClassExpression::Class(c)
                    if c.iri.as_str() == "http://example.org/B")
        }
        _ => false,
    });
    assert!(sub, "rdfs:subClassOf must still produce SubClassOf; axioms: {:?}", ont.axioms());
}

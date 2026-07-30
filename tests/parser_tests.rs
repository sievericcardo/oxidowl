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

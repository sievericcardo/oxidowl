//! B5 Slice 3 — Binary RDF 1.0 conformance tests (docs/PLAN.md).
//!
//! Locks the RDF4J Binary RDF 1.0 implementation in
//! `src/parsers/rio/binary_rdf.rs`:
//!
//! 1. **Round-trip axiom equality** for a rich ontology (declarations,
//!    subclass, class/object/data/annotation assertions, `EquivalentClasses`,
//!    a blank-node restriction, literals of several datatypes) carrying a
//!    dictionary of ≥ 130 distinct values (value ids ≥ 128 ⇒ multi-byte
//!    varints).
//! 2. **Byte-level layout** against the spec: 8-byte header (`BRDF` +
//!    big-endian `i32` version = 1), record-type bytes
//!    (`NAMESPACE_DECL=0`, `STATEMENT=1`, `COMMENT=2`, `VALUE_DECL=3`,
//!    `END_OF_DATA=127`), 7-bit varints (RDF4J `IOUtil.writeVarInt`
//!    vectors), value-reference slots and the `NULL` context slot.
//! 3. **Namespaces** round-trip through `NAMESPACE_DECL` records.
//! 4. **`END_OF_DATA`** terminator presence and exclusivity.
//! 5. **Malformed / truncated input** returns an error — never a panic.

#[allow(dead_code)]
#[path = "helpers/mod.rs"]
mod helpers;

use helpers::assertions::assert_ontologies_axiom_equal;
use helpers::df::DF;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::rio::binary_rdf::{
    self, BinaryRdfParser, BinaryRdfRenderer, END_OF_DATA, NAMESPACE_DECL, NULL_VALUE, STATEMENT,
    URI_VALUE, VALUE_DECL, VALUE_REF, read_varint, write_varint,
};
use oxidowl::parsers::{parse_bytes, save_to_bytes, save_to_string};

// ═════════════════════════════════════════════════════════════════════════════
// Helpers
// ═════════════════════════════════════════════════════════════════════════════

/// Independent test-side implementation of the spec's 7-bit varint
/// (least-significant group first, `0x80` continuation) — used to spell out
/// *expected* byte streams without going through the production encoder.
fn vint(mut value: u64) -> Vec<u8> {
    let mut out = Vec::new();
    while value >= 0x80 {
        out.push(((value as u8) & 0x7F) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
    out
}

/// `BRDF` + big-endian `i32` version = 1 (the spec's 8-byte header).
fn spec_header() -> Vec<u8> {
    let mut out = b"BRDF".to_vec();
    out.extend_from_slice(&1i32.to_be_bytes());
    out
}

fn brdf_parse(bytes: &[u8]) -> Ontology {
    parse_bytes(bytes, OntologyFormat::BinaryRdf)
        .unwrap_or_else(|e| panic!("BinaryRDF parse failed: {e}"))
}

/// A rich ontology covering every axiom kind required by the slice:
/// declarations, subclass, class/object/data/annotation assertions,
/// `EquivalentClasses` (named pair + blank-node restriction), literals of
/// several datatypes — plus `130` extra class declarations so the value
/// dictionary exceeds 128 entries (multi-byte varint ids).
fn rich_ontology() -> Ontology {
    let df = DF::new();
    let a = df.class_ce("http://example.org/A");
    let b = df.class_ce("http://example.org/B");
    let c = df.class_ce("http://example.org/C");
    let d = df.class_ce("http://example.org/D");
    let p = df.obj_prop("http://example.org/p");
    let dp = df.data_prop("http://example.org/age");
    let dp2 = df.data_prop("http://example.org/score");
    let i = df.named("http://example.org/i");
    let j = df.named("http://example.org/j");

    let mut ont = df.build_ontology(vec![
        // SubClassOf
        df.sub_class_of(a.clone(), b.clone()),
        // ClassAssertion
        df.class_assertion(c.clone(), i.clone()),
        // ObjectPropertyAssertion
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        // DataPropertyAssertion — integer literal
        df.data_property_assertion(dp.clone(), i.clone(), df.int_literal(42)),
        // DataPropertyAssertion — double literal
        df.data_property_assertion(
            dp2.clone(),
            i.clone(),
            df.double_literal(std::f64::consts::PI),
        ),
        // AnnotationAssertion — plain literal
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: df.next_id(),
            subject: AnnotationSubject::IRI(IRI::new("http://example.org/A")),
            property: df.annotation_property("http://example.org/ann"),
            value: AnnotationValue::Literal(df.literal("hello")),
            annotations: vec![],
        }),
        // AnnotationAssertion — language-tagged literal
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: df.next_id(),
            subject: AnnotationSubject::IRI(IRI::new("http://example.org/B")),
            property: df.annotation_property("http://example.org/label"),
            value: AnnotationValue::Literal(df.lang_literal("chat", "fr")),
            annotations: vec![],
        }),
        // EquivalentClasses (named pair)
        df.equivalent_classes(vec![c.clone(), d.clone()]),
        // EquivalentClasses + ObjectSomeValuesFrom (blank-node restriction)
        df.equivalent_classes(vec![df.some_values_from(p.clone(), b.clone()), a.clone()]),
    ]);
    df.auto_declare(&mut ont);

    // ≥ 130 dictionary entries: 130 extra class declarations.
    for k in 0..130 {
        ont.add_axiom(Axiom::Declaration(DeclarationAxiom {
            id: 1000 + k as u64,
            entity: Entity::Class(IRI::new(&format!("http://example.org/big/C{k}"))),
        }));
    }
    ont
}

/// Count the distinct RDF terms (dictionary entries) the ontology's RDF
/// emission would declare in a Binary RDF file.
fn distinct_dictionary_entries(ont: &Ontology) -> usize {
    use std::collections::BTreeSet;
    let triples =
        oxidowl::semantics::owl_rdf_mapping::ontology_to_triples(ont).expect("ontology triples");
    let mut terms: BTreeSet<String> = BTreeSet::new();
    for t in &triples {
        for term in [&t.subject, &t.predicate, &t.object] {
            terms.insert(format!("{term:?}"));
        }
    }
    terms.len()
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Round-trip axiom equality (rich ontology, ≥130 dictionary entries)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn rich_ontology_round_trips_with_130plus_dictionary_entries() {
    let ont = rich_ontology();
    let entries = distinct_dictionary_entries(&ont);
    assert!(
        entries >= 130,
        "fixture must exercise ≥130 dictionary entries, got {entries}"
    );

    let bytes = save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("serialize");
    assert!(
        bytes.iter().any(|&b| b >= 0x80),
        "value ids ≥ 128 must be varint-encoded with bytes ≥ 0x80"
    );
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        "a 130+ entry stream must be genuinely non-UTF-8 (the reason save_to_string refuses)"
    );

    let reloaded = brdf_parse(&bytes);
    assert_ontologies_axiom_equal(&ont, &reloaded);

    // Every required kind must survive individually.
    for kind in [
        AxiomType::Declaration,
        AxiomType::SubClassOf,
        AxiomType::ClassAssertion,
        AxiomType::ObjectPropertyAssertion,
        AxiomType::DataPropertyAssertion,
        AxiomType::AnnotationAssertion,
        AxiomType::EquivalentClasses,
    ] {
        assert!(
            reloaded.axioms().iter().any(|ax| ax.axiom_type() == kind),
            "{kind:?} lost in BinaryRDF round-trip; axioms: {:?}",
            reloaded.axioms()
        );
    }

    // Literals of several datatypes keep their datatypes/language.
    // (`url::Url`'s Debug is the struct form, so the datatype is matched via
    // its fragment — `fragment: Some("integer")` — rather than the URL text.)
    let rendered: Vec<String> = reloaded.axioms().iter().map(|a| format!("{a:?}")).collect();
    let all = rendered.join("\n");
    assert!(
        all.contains("value: \"42\"") && all.contains("fragment: Some(\"integer\")"),
        "integer literal lost"
    );
    assert!(
        all.contains("value: \"3.141592653589793\"") && all.contains("fragment: Some(\"double\")"),
        "double literal lost"
    );
    assert!(
        all.contains("value: \"chat\"") && all.contains("language: Some(\"fr\")"),
        "language-tagged literal lost"
    );

    // The blank-node restriction survived (EquivalentClasses + ∃p.B).
    let restriction_found = reloaded.axioms().iter().any(|ax| {
        let s = format!("{ax:?}");
        s.contains("ObjectSomeValuesFrom") && s.contains("example.org/p")
    });
    assert!(restriction_found, "restriction lost; axioms: {all}");

    // String save must refuse (binary format).
    assert!(save_to_string(&ont, OntologyFormat::BinaryRdf).is_err());
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Byte-level layout against the spec
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn spec_record_and_value_type_constants() {
    // RDF4J BinaryRDFConstants.
    assert_eq!(NAMESPACE_DECL, 0);
    assert_eq!(STATEMENT, 1);
    assert_eq!(binary_rdf::COMMENT, 2);
    assert_eq!(VALUE_DECL, 3);
    assert_eq!(END_OF_DATA, 127);
}

#[test]
fn header_and_record_bytes_match_spec_layout() {
    // Minimal ontology: one class declaration, no prefixes, no ontology IRI.
    let mut ont = Ontology::new();
    ont.add_axiom(Axiom::Declaration(DeclarationAxiom {
        id: 1,
        entity: Entity::Class(IRI::new("http://example.org/A")),
    }));
    let bytes = save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("serialize");

    // ── Header: `BRDF` + 4-byte signed version = 1 ─────────────────────────
    assert_eq!(&bytes[0..4], b"BRDF", "magic must be `BRDF`");
    assert_eq!(
        i32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]),
        1,
        "version must be the signed integer 1 (big-endian, Java writeInt order)"
    );

    // ── Full expected stream, spelled out per the spec ─────────────────────
    let class_iri = "http://example.org/A";
    let rdf_type = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let owl_class = "http://www.w3.org/2002/07/owl#Class";

    let mut expected = spec_header();
    // VALUE_DECL id=0: URI value
    expected.push(VALUE_DECL);
    expected.extend(vint(0));
    expected.push(URI_VALUE); // URI_VALUE
    expected.extend(vint(class_iri.len() as u64));
    expected.extend_from_slice(class_iri.as_bytes());
    // VALUE_DECL id=1: URI value
    expected.push(VALUE_DECL);
    expected.extend(vint(1));
    expected.push(URI_VALUE);
    expected.extend(vint(rdf_type.len() as u64));
    expected.extend_from_slice(rdf_type.as_bytes());
    // VALUE_DECL id=2: URI value
    expected.push(VALUE_DECL);
    expected.extend(vint(2));
    expected.push(URI_VALUE);
    expected.extend(vint(owl_class.len() as u64));
    expected.extend_from_slice(owl_class.as_bytes());
    // STATEMENT: three VALUE_REF slots + NULL context
    expected.push(STATEMENT);
    expected.push(VALUE_REF); // VALUE_REF
    expected.extend(vint(0));
    expected.push(VALUE_REF);
    expected.extend(vint(1));
    expected.push(VALUE_REF);
    expected.extend(vint(2));
    expected.push(NULL_VALUE); // default graph context
    // END_OF_DATA as the final byte
    expected.push(END_OF_DATA);

    assert_eq!(
        bytes,
        expected,
        "stream must match the spec layout exactly (got {})",
        hex(&bytes)
    );

    // And it parses back to the same declaration.
    let reloaded = brdf_parse(&bytes);
    assert_eq!(reloaded.axioms().len(), 1, "single declaration round-trip");
}

#[test]
fn varint_encoding_matches_rdf4j_vectors() {
    // Vectors from RDF4J `IOUtil.writeVarInt` documentation.
    let vectors: [(u64, &[u8]); 6] = [
        (0, &[0x00]),
        (1, &[0x01]),
        (127, &[0x7F]),
        (128, &[0x80, 0x01]),
        (421, &[0xA5, 0x03]),
        (100_000, &[0xA0, 0x8D, 0x06]),
    ];
    for (value, expected) in vectors {
        let mut buf = Vec::new();
        write_varint(&mut buf, value).expect("write_varint");
        assert_eq!(buf, expected, "varint encoding of {value}");

        // The independent test-side encoder must agree.
        assert_eq!(vint(value), expected, "test vint of {value}");

        // And the production reader must decode it back.
        let mut pos = 0;
        assert_eq!(
            read_varint(&buf, &mut pos).expect("read_varint"),
            value,
            "varint decode of {value}"
        );
        assert_eq!(pos, buf.len());
    }

    // Truncated varint (continuation bit set, no next byte) → error.
    let mut pos = 0;
    assert!(read_varint(&[0x80], &mut pos).is_err(), "truncated varint");
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Namespaces
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn namespace_declarations_round_trip() {
    let mut ont = Ontology::new();
    ont.prefixes
        .insert("ex".to_owned(), IRI::new("http://example.org/"));
    ont.prefixes
        .insert("owl".to_owned(), IRI::new("http://www.w3.org/2002/07/owl#"));
    ont.add_axiom(Axiom::Declaration(DeclarationAxiom {
        id: 1,
        entity: Entity::Class(IRI::new("http://example.org/A")),
    }));

    let bytes = save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("serialize");

    // The first record after the 8-byte header is a NAMESPACE_DECL record.
    assert_eq!(
        bytes[8], NAMESPACE_DECL,
        "first record must be NAMESPACE_DECL"
    );
    // …and its payload is `varint(len) prefix` exactly as specified:
    // prefixes sort by name → "ex" first.
    assert_eq!(bytes[9], 2, "prefix byte length");
    assert_eq!(&bytes[10..12], b"ex", "prefix bytes");
    assert_eq!(bytes[12], b"http://example.org/".len() as u8);

    let reloaded = brdf_parse(&bytes);
    assert_eq!(
        reloaded.prefixes.get("ex").map(IRI::as_str),
        Some("http://example.org/"),
        "`ex` prefix must round-trip through the NAMESPACE_DECL record"
    );
    assert_eq!(
        reloaded.prefixes.get("owl").map(IRI::as_str),
        Some("http://www.w3.org/2002/07/owl#"),
        "`owl` prefix must round-trip"
    );
    assert!(reloaded.axioms().iter().any(
        |a| matches!(a, Axiom::Declaration(d) if d.entity.iri().as_str() == "http://example.org/A")
    ));
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. END_OF_DATA terminator
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn end_of_data_is_the_final_record() {
    // Empty ontology: header + END_OF_DATA and nothing else.
    let empty = Ontology::new();
    let bytes = save_to_bytes(&empty, OntologyFormat::BinaryRdf).expect("serialize");
    assert_eq!(
        bytes,
        spec_header_with_end(),
        "empty document = header + END"
    );

    // Rich ontology: END_OF_DATA is the last byte and appears exactly once
    // as a record (0x7F can also occur inside string payloads, so locate it
    // structurally via a round-trip + trailing-byte rejection instead).
    let bytes = save_to_bytes(&rich_ontology(), OntologyFormat::BinaryRdf).expect("serialize");
    assert_eq!(
        *bytes.last().expect("non-empty stream"),
        END_OF_DATA,
        "last byte must be END_OF_DATA (127)"
    );
    // Stripping the terminator makes the document malformed.
    let mut without_end = bytes.clone();
    without_end.pop();
    assert!(
        parse_bytes(&without_end, OntologyFormat::BinaryRdf).is_err(),
        "document without END_OF_DATA must be rejected"
    );
}

fn spec_header_with_end() -> Vec<u8> {
    let mut v = spec_header();
    v.push(END_OF_DATA);
    v
}

// ═════════════════════════════════════════════════════════════════════════════
// 5. Comments, inline values and statement contexts
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn comments_inline_values_and_named_contexts_decode() {
    let i = "http://example.org/i";
    let p = "http://example.org/p";
    let j = "http://example.org/j";
    let g = "http://example.org/g";

    let mut bytes = spec_header();
    // COMMENT record: "hello world"
    bytes.push(binary_rdf::COMMENT);
    bytes.extend(vint(11));
    bytes.extend_from_slice(b"hello world");
    // STATEMENT with *inline* full values (no VALUE_DECL) and a named
    // graph context in the 4th slot.
    bytes.push(STATEMENT);
    for term in [i, p, j] {
        bytes.push(URI_VALUE); // URI_VALUE, inline
        bytes.extend(vint(term.len() as u64));
        bytes.extend_from_slice(term.as_bytes());
    }
    bytes.push(URI_VALUE); // context: named graph
    bytes.extend(vint(g.len() as u64));
    bytes.extend_from_slice(g.as_bytes());
    bytes.push(END_OF_DATA);

    let reloaded = brdf_parse(&bytes);

    // The statement maps as if in the default graph (an `Ontology` has no
    // named-graph structure — the documented context semantics).
    let found = reloaded.axioms().iter().any(|ax| {
        let s = format!("{ax:?}");
        s.contains("ObjectPropertyAssertion")
            && s.contains("http://example.org/i")
            && s.contains("http://example.org/p")
            && s.contains("http://example.org/j")
    });
    assert!(
        found,
        "contextual statement must map; axioms: {:?}",
        reloaded.axioms()
    );

    // The COMMENT record is preserved as an ontology-level rdfs:comment.
    let comment = reloaded
        .annotations
        .iter()
        .find(|a| a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#comment");
    let comment = comment.unwrap_or_else(|| panic!("comment lost; got {:?}", reloaded.annotations));
    match &comment.value {
        AnnotationValue::Literal(l) => assert_eq!(l.value, "hello world"),
        other => panic!("expected literal comment, got {other:?}"),
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 6. Malformed / truncated input → error, never panic
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn malformed_inputs_return_errors_not_panics() {
    let good = save_to_bytes(&rich_ontology(), OntologyFormat::BinaryRdf).expect("serialize");

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();

    // Truncations of a valid file (header, mid-record, missing terminator).
    for cut in [0, 1, 3, 4, 7, 8, 9, 20, good.len() / 2, good.len() - 1] {
        cases.push((format!("truncated at {cut} bytes"), good[..cut].to_vec()));
    }
    // Bad magic.
    let mut v = b"Xrdf".to_vec();
    v.extend_from_slice(&1i32.to_be_bytes());
    cases.push(("bad magic".to_owned(), v));
    // Unsupported version.
    let mut v = b"BRDF".to_vec();
    v.extend_from_slice(&2i32.to_be_bytes());
    cases.push(("version 2".to_owned(), v));
    // Unknown record type.
    let mut v = spec_header();
    v.push(0x55);
    v.push(END_OF_DATA);
    cases.push(("unknown record type".to_owned(), v));
    // Truncated varint (continuation bit, then EOF).
    let mut v = spec_header();
    v.push(VALUE_DECL);
    v.push(0x80);
    cases.push(("truncated varint".to_owned(), v));
    // Truncated string (declared length beyond EOF).
    let mut v = spec_header();
    v.push(VALUE_DECL);
    v.push(0x00); // id 0
    v.push(URI_VALUE); // URI_VALUE
    v.push(0x64); // length 100 — only a few bytes follow
    v.extend_from_slice(b"abc");
    cases.push(("truncated string".to_owned(), v));
    // Reference to an undeclared value id.
    let mut v = spec_header();
    v.push(STATEMENT);
    v.push(VALUE_REF); // VALUE_REF
    v.push(0x09); // id 9 — never declared
    v.push(VALUE_REF);
    v.push(0x00);
    v.push(VALUE_REF);
    v.push(0x00);
    v.push(NULL_VALUE);
    v.push(END_OF_DATA);
    cases.push(("undeclared value ref".to_owned(), v));
    // Duplicate VALUE_DECL for the same id.
    let mut v = spec_header();
    for _ in 0..2 {
        v.push(VALUE_DECL);
        v.push(0x00); // id 0
        v.push(URI_VALUE); // URI_VALUE
        v.push(0x01); // length 1
        v.push(b'a');
    }
    v.push(END_OF_DATA);
    cases.push(("duplicate value decl".to_owned(), v));
    // Statement with a null subject.
    let mut v = spec_header();
    v.push(STATEMENT);
    v.push(NULL_VALUE); // NULL subject
    v.push(VALUE_REF);
    v.push(0x00);
    v.push(VALUE_REF);
    v.push(0x00);
    v.push(NULL_VALUE);
    v.push(END_OF_DATA);
    cases.push(("null subject".to_owned(), v));
    // Trailing bytes after END_OF_DATA.
    let mut v = spec_header();
    v.push(END_OF_DATA);
    v.push(0x00);
    cases.push(("trailing bytes".to_owned(), v));
    // Empty input.
    cases.push(("empty input".to_owned(), Vec::new()));

    for (name, bytes) in &cases {
        // catch_unwind turns a would-be panic into a failure with the case
        // name, making it obvious *which* input misbehaved.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parse_bytes(bytes, OntologyFormat::BinaryRdf)
        }));
        match result {
            Ok(Ok(_)) => panic!("malformed case `{name}` must be rejected"),
            Ok(Err(_)) => {} // expected: clean error
            Err(_) => panic!("malformed case `{name}` panicked instead of erroring"),
        }
    }
}

/// The parser entry points (`BinaryRdfParser::parse`, module `parse`/
/// `parse_bytes`) agree on the same rejection behaviour.
#[test]
fn parser_entry_points_agree() {
    assert!(BinaryRdfParser::new().parse(b"nope").is_err());
    assert!(binary_rdf::parse("nope").is_err());
    assert!(binary_rdf::parse_bytes(b"").is_err());

    let ont = rich_ontology();
    let bytes = BinaryRdfRenderer::new().serialize(&ont).expect("render");
    let via_renderer = BinaryRdfParser::new().parse(&bytes).expect("parse");
    let via_module = binary_rdf::parse_bytes(&bytes).expect("parse");
    assert_ontologies_axiom_equal(&via_renderer, &via_module);
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-011 M2 — N3 rule + IRI-less ontology annotation survive BinaryRDF
// ═════════════════════════════════════════════════════════════════════════════

const LOG_FORMULA: &str = "http://www.w3.org/2000/10/swap/log#Formula";

fn rule_of(ont: &Ontology) -> SWRLRule {
    ont.axioms()
        .iter()
        .find_map(|a| match a {
            Axiom::Rule(rule) => Some(rule.rule.clone()),
            _ => None,
        })
        .expect("ontology must carry the SWRL rule")
}

fn formula_annotation(ont: &Ontology) -> Vec<Annotation> {
    ont.annotations
        .iter()
        .filter(|a| a.property.iri.as_str() == LOG_FORMULA)
        .cloned()
        .collect()
}

/// Twin of `n3_rule_and_iri_less_annotation_survive_{jsonld,hdt}_roundtrip`
/// for `BinaryRDF` (CR-011 M2, re-review NR-2): the `BinaryRDF` writer emits the
/// shared preservation triples (`urn:oxidowl:swrlRule` rule text +
/// `urn:oxidowl:ontology` annotations) and the parser decodes them back —
/// both were only *statically* wired before this asserted round-trip.
#[test]
fn n3_rule_and_iri_less_annotation_survive_binary_rdf_roundtrip() {
    use oxidowl::parsers::parse_str;

    let source = "@prefix ex: <http://example.org/> .\n\
                  { ?x a ex:A } => { ?x a ex:B } .\n\
                  { ex:k ex:q ex:v } .\n";
    let ont = parse_str(source, OntologyFormat::N3).expect("N3 fixture must parse");
    assert!(ont.get_iri().is_none(), "fixture ontology must be IRI-less");
    let rule = rule_of(&ont);
    let annotations = formula_annotation(&ont);
    assert_eq!(annotations.len(), 1, "fixture must carry one log:Formula");

    let bytes = save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("BinaryRDF serialize");
    assert!(
        bytes.starts_with(b"BRDF"),
        "payload must be a real Binary RDF file"
    );
    let back = parse_bytes(&bytes, OntologyFormat::BinaryRdf).expect("BinaryRDF reparse");

    assert!(
        back.get_iri().is_none(),
        "IRI-less must round-trip as IRI-less"
    );
    assert_eq!(
        rule_of(&back),
        rule,
        "the SWRL rule must round-trip structurally unchanged (incl. variable IRIs)"
    );
    assert_eq!(
        formula_annotation(&back),
        annotations,
        "the IRI-less log:Formula annotation must round-trip unchanged"
    );
    assert!(
        !back
            .axioms()
            .iter()
            .any(|a| matches!(a, Axiom::ObjectPropertyAssertion(_))),
        "preservation triples must not leak into junk axioms: {:?}",
        back.axioms()
    );
}

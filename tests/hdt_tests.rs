//! B5 Slice 5 — HDT conformance tests (docs/PLAN.md).
//!
//! Locks the real HDT implementation (`hdt` crate) in
//! `src/parsers/rio/hdt.rs`:
//!
//! 1. **Control information bytes** of a written file: `$HDT` magic + the
//!    global control-info chunk (control byte `0x01` = Global, format URI
//!    `<http://purl.org/HDT/hdt#HDTv1>`, NUL-terminated, CRC16-ARC).
//! 2. **Write → read → `Ontology` axiom equality** for a rich ontology
//!    (declarations, subclass, class/object/data/annotation assertions,
//!    `EquivalentClasses`, a blank-node restriction, literals of several
//!    datatypes incl. escaped characters, ontology IRI + version IRI +
//!    ontology-level annotation).
//! 3. **Real upstream fixture** (`tests/fixtures/snikmeta.hdt`, produced by
//!    external HDT tooling — *not* by this crate’s writer) parses correctly:
//!    verified against the fixture’s documented 328 triples and known
//!    content.
//! 4. **Truncated input** returns an error — never a panic.

#[allow(dead_code)]
#[path = "helpers/mod.rs"]
mod helpers;

use std::io::BufReader;

use helpers::assertions::assert_ontologies_axiom_equal;
use helpers::df::DF;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::{parse_bytes, save_to_bytes, save_to_string};

const FIXTURE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/snikmeta.hdt");

/// CRC16-ARC (poly 0x8005 reflected → 0xA001, init 0) — the checksum the HDT
/// binary format uses for every control-information chunk
/// (`crc::CRC_16_ARC` upstream).  Reimplemented here independently so the
/// control-information assertion is a genuine spec check.
fn crc16_arc(bytes: &[u8]) -> u16 {
    let mut crc: u16 = 0;
    for &b in bytes {
        crc ^= u16::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xA001
            } else {
                crc >> 1
            };
        }
    }
    crc
}

/// A rich ontology covering every axiom kind required by the slice, plus the
/// ontology IRI / version IRI / ontology-level annotation and a literal
/// containing characters that must be escaped (`"` and newline).
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

    let mut ont = df.build_ontology_with_iri(
        "http://example.org/onto",
        vec![
            df.sub_class_of(a.clone(), b.clone()),
            df.class_assertion(c.clone(), i.clone()),
            df.object_property_assertion(p.clone(), i.clone(), j.clone()),
            df.data_property_assertion(dp.clone(), i.clone(), df.int_literal(42)),
            df.data_property_assertion(
                dp2.clone(),
                i.clone(),
                df.double_literal(std::f64::consts::PI),
            ),
            // AnnotationAssertion — plain literal with `"` and newline escapes
            Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
                id: df.next_id(),
                subject: AnnotationSubject::IRI(IRI::new("http://example.org/A")),
                property: df.annotation_property("http://example.org/ann"),
                value: AnnotationValue::Literal(df.literal("say \"hi\"\\ and\nnewline")),
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
            // EquivalentClasses (named pair) + blank-node restriction
            df.equivalent_classes(vec![c.clone(), d.clone()]),
            df.equivalent_classes(vec![df.some_values_from(p.clone(), b.clone()), a.clone()]),
        ],
    );
    df.auto_declare(&mut ont);
    ont.set_version_iri(Some(IRI::new("http://example.org/onto/v1")));
    ont.annotations
        .push(df.rdfs_comment("ontology-level comment"));

    for k in 0..130 {
        ont.add_axiom(Axiom::Declaration(DeclarationAxiom {
            id: 1000 + k as u64,
            entity: Entity::Class(IRI::new(&format!("http://example.org/big/C{k}"))),
        }));
    }
    ont
}

fn hdt_parse(bytes: &[u8]) -> Ontology {
    parse_bytes(bytes, OntologyFormat::Hdt).unwrap_or_else(|e| panic!("HDT parse failed: {e}"))
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Control information of a written file
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn written_file_has_spec_control_information() {
    let ont = Ontology::new();
    let bytes = save_to_bytes(&ont, OntologyFormat::Hdt).expect("serialize");

    // ── Global control info chunk: `$HDT` + control byte + format URI ──────
    assert!(
        bytes.starts_with(b"$HDT"),
        "HDT files start with the `$HDT` magic"
    );
    assert_eq!(bytes[4], 1, "control byte must be 1 (ControlType::Global)");
    assert!(
        bytes[5..].starts_with(b"<http://purl.org/HDT/hdt#HDTv1>\x00"),
        "format URI must follow the control byte, NUL-terminated; got {:?}",
        &bytes[5..40]
    );

    // ── CRC16-ARC over the whole chunk (magic + type + format + props) ─────
    // `write`: magic, type, format NUL, properties NUL, then the CRC as
    // little-endian u16.  One property: `Software=hdt_rs;`.
    let mut chunk = b"$HDT\x01<http://purl.org/HDT/hdt#HDTv1>\x00Software=hdt_rs;\x00".to_vec();
    assert_eq!(
        &bytes[..chunk.len()],
        &chunk[..],
        "global control info must be magic + control byte + format URI + properties"
    );
    let crc_bytes = crc16_arc(&chunk).to_le_bytes();
    assert_eq!(
        &bytes[chunk.len()..chunk.len() + 2],
        &crc_bytes,
        "control info must end with its CRC16-ARC checksum (spec-mandated)"
    );
    chunk.extend_from_slice(&crc_bytes);
    let global_ci_end = chunk.len();

    // The header section's own control info follows (`ControlType::Header`
    // = 2, format `ntriples`).
    assert_eq!(
        bytes[global_ci_end], b'$',
        "next chunk starts with the $HDT magic"
    );
    assert_eq!(
        bytes[global_ci_end + 4],
        2,
        "header section control byte must be 2 (ControlType::Header)"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Write → read → axiom equality
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn write_read_roundtrip_rich_ontology() {
    let ont = rich_ontology();
    let bytes = save_to_bytes(&ont, OntologyFormat::Hdt).expect("serialize");
    assert!(bytes.starts_with(b"$HDT"));

    // HDT is binary: `save_to_string` must refuse instead of corrupting.
    assert!(save_to_string(&ont, OntologyFormat::Hdt).is_err());

    let reloaded = hdt_parse(&bytes);
    assert_ontologies_axiom_equal(&ont, &reloaded);

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
            "{kind:?} lost in HDT round-trip; axioms: {:?}",
            reloaded.axioms()
        );
    }

    // Ontology IRI, version IRI and the ontology-level annotation survive.
    assert_eq!(
        reloaded.get_iri().map(IRI::as_str),
        Some("http://example.org/onto"),
        "ontology IRI must round-trip"
    );
    assert_eq!(
        reloaded.id.get_version_iri().map(IRI::as_str),
        Some("http://example.org/onto/v1"),
        "version IRI must round-trip"
    );
    assert!(
        reloaded.annotations.iter().any(|a| {
            a.property.iri.as_str() == "http://www.w3.org/2000/01/rdf-schema#comment"
                && matches!(&a.value, AnnotationValue::Literal(l) if l.value == "ontology-level comment")
        }),
        "ontology-level annotation must round-trip; got {:?}",
        reloaded.annotations
    );

    // Literals: datatypes, language tags and escaped characters survive.
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
    assert!(
        all.contains("say \\\"hi\\\"\\\\ and\\nnewline")
            || all.contains("say \"hi\"\\ and\nnewline"),
        "escaped literal lost; axioms: {all}"
    );

    // The blank-node restriction survived.
    assert!(
        reloaded.axioms().iter().any(|ax| {
            let s = format!("{ax:?}");
            s.contains("ObjectSomeValuesFrom") && s.contains("example.org/p")
        }),
        "restriction lost; axioms: {all}"
    );
}

/// An empty ontology serializes to a valid (zero-triple) HDT file that reads
/// back as an empty ontology.
#[test]
fn empty_ontology_round_trips() {
    let bytes = save_to_bytes(&Ontology::new(), OntologyFormat::Hdt).expect("serialize");
    assert!(bytes.starts_with(b"$HDT"));
    let reloaded = hdt_parse(&bytes);
    assert!(
        reloaded.axioms().is_empty(),
        "zero-triple HDT must read back empty; got {:?}",
        reloaded.axioms()
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Real upstream fixture (external tooling, not our writer)
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn upstream_fixture_snikmeta_hdt_parses_correctly() {
    let bytes = std::fs::read(FIXTURE).expect("read tests/fixtures/snikmeta.hdt");

    // Independent sanity check straight from the `hdt` crate: the fixture is
    // a complete HDT file with the 328 triples its upstream test suite
    // documents (created by external HDT tooling — hdt-rs’ own test
    // resource, not a product of this crate’s writer).
    let raw = hdt::Hdt::read(BufReader::new(&bytes[..])).expect("hdt crate must read fixture");
    assert_eq!(
        raw.triples_all().count(),
        328,
        "fixture must contain its documented 328 triples"
    );

    // Our parser maps those triples into axioms via the shared RDF→OWL
    // mapping.
    let ont = hdt_parse(&bytes);
    assert!(!ont.axioms().is_empty(), "fixture must produce axioms");

    // Known fixture content: <meta/Top> rdfs:label "top class"@en .
    let label_found = ont.axioms().iter().any(|ax| {
        let s = format!("{ax:?}");
        s.contains("AnnotationAssertion")
            && s.contains("http://www.snik.eu/ontology/meta/Top")
            && s.contains("\"top class\"")
            && s.contains("Some(\"en\")")
    });
    assert!(
        label_found,
        "rdfs:label \"top class\"@en annotation missing; axioms: {:?}",
        &ont.axioms()[..ont.axioms().len().min(20)]
    );

    // The fixture documents 12 rdfs:subClassOf triples → SubClassOf axioms.
    let subclass_count = ont
        .axioms()
        .iter()
        .filter(|ax| ax.axiom_type() == AxiomType::SubClassOf)
        .count();
    assert!(
        (10..=12).contains(&subclass_count),
        "expected the fixture's ~12 SubClassOf axioms, got {subclass_count}"
    );

    // Class declarations from `X a owl:Class` triples.
    let declarations = ont
        .axioms()
        .iter()
        .filter(|ax| ax.axiom_type() == AxiomType::Declaration)
        .count();
    assert!(
        declarations >= 10,
        "fixture must declare its classes, got {declarations}"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Truncated input → error, never panic
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn truncated_input_returns_errors_not_panics() {
    let good = save_to_bytes(&rich_ontology(), OntologyFormat::Hdt).expect("serialize");
    let fixture = std::fs::read(FIXTURE).expect("fixture");

    let mut cases: Vec<(String, Vec<u8>)> = Vec::new();
    for (name, bytes) in [("written", &good), ("fixture", &fixture)] {
        for cut in [
            0,
            1,
            3,
            4,
            5,
            10,
            40,
            bytes.len() / 4,
            bytes.len() / 2,
            bytes.len() - 1,
        ] {
            if cut < bytes.len() {
                cases.push((format!("{name} truncated at {cut}"), bytes[..cut].to_vec()));
            }
        }
    }
    // Garbage that is not an HDT file at all.
    cases.push(("not hdt".to_owned(), b"not an HDT file".to_vec()));
    cases.push(("empty".to_owned(), Vec::new()));
    // Magic present, control byte invalid.
    cases.push(("bad control byte".to_owned(), b"$HDT\xFF\x00\x00".to_vec()));

    for (name, bytes) in &cases {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            parse_bytes(bytes, OntologyFormat::Hdt)
        }));
        match result {
            Ok(Ok(_)) => panic!("malformed case `{name}` must be rejected"),
            Ok(Err(_)) => {} // expected: clean error
            Err(_) => panic!("malformed case `{name}` panicked instead of erroring"),
        }
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// CR-011 M2 — N3 rule + IRI-less ontology annotation survive the HDT trip
// ═════════════════════════════════════════════════════════════════════════════

const LOG_FORMULA: &str = "http://www.w3.org/2000/10/swap/log#Formula";

/// The regression behind CR-011 M2 on the binary side: a SWRL rule (minted
/// by the N3 parser) and a `log:Formula` annotation on an ontology with **no
/// IRI** must survive `save_to_bytes(Hdt)` → `parse_bytes` — previously the
/// rule vanished (`Rule → vec![]`) and the annotation with the `get_iri()`
/// gate.
#[test]
fn n3_rule_and_iri_less_annotation_survive_hdt_roundtrip() {
    use oxidowl::parsers::parse_str;

    let source = "@prefix ex: <http://example.org/> .\n\
                  { ?x a ex:A } => { ?x a ex:B } .\n\
                  { ex:k ex:q ex:v } .\n";
    let ont = parse_str(source, OntologyFormat::N3).expect("N3 fixture must parse");
    assert!(ont.get_iri().is_none(), "fixture ontology must be IRI-less");

    let rule = ont
        .axioms()
        .iter()
        .find_map(|a| match a {
            Axiom::Rule(rule) => Some(rule.rule.clone()),
            _ => None,
        })
        .expect("fixture must carry one SWRL rule");
    let annotations: Vec<Annotation> = ont
        .annotations
        .iter()
        .filter(|a| a.property.iri.as_str() == LOG_FORMULA)
        .cloned()
        .collect();
    assert_eq!(annotations.len(), 1, "fixture must carry one log:Formula");

    let bytes = save_to_bytes(&ont, OntologyFormat::Hdt).expect("HDT serialize");
    assert!(
        bytes.starts_with(b"$HDT"),
        "payload must be a real HDT file"
    );
    let back = parse_bytes(&bytes, OntologyFormat::Hdt).expect("HDT reparse");

    assert!(
        back.get_iri().is_none(),
        "IRI-less must round-trip as IRI-less"
    );
    let rule_back = back
        .axioms()
        .iter()
        .find_map(|a| match a {
            Axiom::Rule(rule) => Some(rule.rule.clone()),
            _ => None,
        })
        .expect("the SWRL rule must survive the HDT round-trip");
    assert_eq!(
        rule_back, rule,
        "rule structure (incl. variable IRIs) must round-trip unchanged"
    );
    let annotations_back: Vec<Annotation> = back
        .annotations
        .iter()
        .filter(|a| a.property.iri.as_str() == LOG_FORMULA)
        .cloned()
        .collect();
    assert_eq!(
        annotations_back, annotations,
        "the IRI-less log:Formula annotation must survive the HDT round-trip"
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

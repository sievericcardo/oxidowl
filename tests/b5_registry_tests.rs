//! B5 Slice 0 — format-registry foundation (docs/PLAN.md, CR-008 follow-up).
//!
//! Locks the *registry plumbing* that makes the B5 formats (JSON-LD, N3,
//! N-Quads, TriG, TriX, RDFa, RDF/JSON, HDT, BinaryRDF) reachable:
//!
//! 1. **Extension mapping** — `OntologyFormat::from_extension` and
//!    `parse_file_auto` route every B5 extension to its format.
//! 2. **Content detection** — `$HDT` / `BRDF` magic, JSON-LD `@context` /
//!    `@graph`, HTML → RDFa; plus non-regression of the pre-existing
//!    detections (Turtle, Functional, RDF/XML).
//! 3. **Binary-safe I/O** — `BinaryRDF` byte round-trip with a ≥128-entry
//!    dictionary (varint bytes ≥ 0x80), the `save_to_string` refusal guard,
//!    and a byte-oriented file round-trip through `parse_file_auto`.
//! 4. **Reachability** — `ParserFactory` / `FormatParser` and
//!    `OntologyLoader` can reach every RIO format.
//!
//! Scope note: Slices 0–2 originally shipped toy-level BinaryRDF/HDT
//! implementations; B5 Slice 3 (Binary RDF 1.0) and Slice 5 (HDT via the
//! `hdt` crate) replaced them with spec-compliant codecs, and their full
//! spec assertions live in `binary_rdf_tests.rs` / `hdt_tests.rs`.  These
//! tests assert the *registry* — extension mapping, detection, binary-safe
//! I/O and reachability — for every B5 format.

use std::collections::BTreeSet;
use std::sync::{Arc, RwLock};

use oxidowl::ontology::axioms::{Axiom, DeclarationAxiom};
use oxidowl::ontology::{Entity, IRI, Ontology, OntologyFormat};
use oxidowl::parsers::{
    FormatParser, Parser, ParserFactory, RdfCompatibilityMode, detect_format_from_bytes,
    detect_format_from_content_public, parse_bytes, parse_file_auto, save_to_bytes, save_to_string,
};
use oxidowl::{OntologyLoader, OntologyManager};

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Build an ontology carrying `n` unique class declarations
/// (`http://example.org/b5/C0` … `C{n-1}`).
fn ontology_with_class_declarations(n: usize) -> Ontology {
    let mut ont = Ontology::new();
    for i in 0..n {
        let iri = IRI::new(&format!("http://example.org/b5/C{i}"));
        ont.add_axiom(Axiom::Declaration(DeclarationAxiom {
            id: i as u64 + 1,
            entity: Entity::Class(iri),
        }));
    }
    ont
}

/// The set of declaration IRIs of an ontology (order-independent).
fn declaration_iris(ont: &Ontology) -> BTreeSet<String> {
    ont.axioms()
        .iter()
        .filter_map(|a| match a {
            Axiom::Declaration(d) => Some(d.entity.iri().to_string()),
            _ => None,
        })
        .collect()
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Extension mapping
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn extension_map_covers_all_b5_formats() {
    // B5 formats (C1 deliverable).
    assert_eq!(
        OntologyFormat::from_extension("jsonld"),
        Some(OntologyFormat::JsonLd)
    );
    assert_eq!(
        OntologyFormat::from_extension("json-ld"),
        Some(OntologyFormat::JsonLd)
    );
    assert_eq!(
        OntologyFormat::from_extension("n3"),
        Some(OntologyFormat::N3)
    );
    assert_eq!(
        OntologyFormat::from_extension("nq"),
        Some(OntologyFormat::NQuads)
    );
    assert_eq!(
        OntologyFormat::from_extension("nquads"),
        Some(OntologyFormat::NQuads)
    );
    assert_eq!(
        OntologyFormat::from_extension("trig"),
        Some(OntologyFormat::TriG)
    );
    assert_eq!(
        OntologyFormat::from_extension("trix"),
        Some(OntologyFormat::TriX)
    );
    assert_eq!(
        OntologyFormat::from_extension("rdfa"),
        Some(OntologyFormat::Rdfa)
    );
    assert_eq!(
        OntologyFormat::from_extension("html"),
        Some(OntologyFormat::Rdfa)
    );
    assert_eq!(
        OntologyFormat::from_extension("xhtml"),
        Some(OntologyFormat::Rdfa)
    );
    assert_eq!(
        OntologyFormat::from_extension("brdf"),
        Some(OntologyFormat::BinaryRdf)
    );
    assert_eq!(
        OntologyFormat::from_extension("binaryrdf"),
        Some(OntologyFormat::BinaryRdf)
    );
    assert_eq!(
        OntologyFormat::from_extension("hdt"),
        Some(OntologyFormat::Hdt)
    );
    assert_eq!(
        OntologyFormat::from_extension("rj"),
        Some(OntologyFormat::RdfJson)
    );
    assert_eq!(
        OntologyFormat::from_extension("rdfjson"),
        Some(OntologyFormat::RdfJson)
    );
    assert_eq!(
        OntologyFormat::from_extension("rjson"),
        Some(OntologyFormat::RdfJson)
    );

    // Pre-existing mappings must be preserved.
    assert_eq!(
        OntologyFormat::from_extension("ttl"),
        Some(OntologyFormat::Turtle)
    );
    assert_eq!(
        OntologyFormat::from_extension("nt"),
        Some(OntologyFormat::NTriples)
    );
    assert_eq!(
        OntologyFormat::from_extension("ofn"),
        Some(OntologyFormat::Functional)
    );
    assert_eq!(OntologyFormat::from_extension("xyz"), None);
}

/// `parse_file_auto` must route B5 extensions to their parser — including the
/// byte-oriented path for `BinaryRdf` / `Hdt` (no `read_to_string`).
#[test]
fn parse_file_auto_routes_b5_extensions() {
    let dir = tempfile::tempdir().expect("tempdir");

    // ── .n3 (N3 parses Turtle-compatible syntax) ────────────────────────────
    let n3_path = dir.path().join("ont.n3");
    std::fs::write(
        &n3_path,
        "@prefix ex: <http://example.org/> .\nex:a ex:knows ex:b .\n",
    )
    .expect("write n3");
    let n3_ont = parse_file_auto(&n3_path).expect(".n3 must parse");
    assert!(
        !n3_ont.axioms().is_empty(),
        ".n3 content must produce axioms"
    );

    // ── .jsonld (JSON-LD document) ──────────────────────────────────────────
    let jsonld_path = dir.path().join("ont.jsonld");
    std::fs::write(
        &jsonld_path,
        r#"{"@context":{"ex":"http://example.org/","owl":"http://www.w3.org/2002/07/owl#"},"@graph":[{"@id":"ex:A","@type":"owl:Class"}]}"#,
    )
    .expect("write jsonld");
    let jsonld_ont = parse_file_auto(&jsonld_path).expect(".jsonld must parse");
    assert!(
        declaration_iris(&jsonld_ont).contains("http://example.org/A"),
        ".jsonld class declaration must round-trip; axioms: {:?}",
        jsonld_ont.axioms()
    );

    // ── .hdt (real binary HDT file, read as bytes — B5 Slice 5) ────────────
    let ont = ontology_with_class_declarations(4);
    let hdt_bytes = save_to_bytes(&ont, OntologyFormat::Hdt).expect("hdt bytes");
    assert!(
        hdt_bytes.starts_with(b"$HDT"),
        "real HDT files start with the $HDT magic"
    );
    let hdt_path = dir.path().join("ont.hdt");
    std::fs::write(&hdt_path, &hdt_bytes).expect("write hdt");
    let hdt_ont = parse_file_auto(&hdt_path).expect(".hdt must parse");
    assert_eq!(
        declaration_iris(&hdt_ont),
        declaration_iris(&ont),
        ".hdt file round-trip must preserve declarations"
    );

    // ── .brdf (real BinaryRDF 1.0, non-UTF-8 — must never hit `read_to_string`)
    // 130 entries ⇒ value ids ≥ 128 ⇒ varint bytes ≥ 0x80 ⇒ the raw
    // file is genuinely not valid UTF-8.
    let brdf_ont_src = ontology_with_class_declarations(130);
    let brdf_bytes = save_to_bytes(&brdf_ont_src, OntologyFormat::BinaryRdf).expect("brdf bytes");
    assert!(
        brdf_bytes.iter().any(|&b| b >= 0x80),
        "fixture must exercise multi-byte varints"
    );
    assert!(
        std::str::from_utf8(&brdf_bytes).is_err(),
        "fixture must be genuinely non-UTF-8"
    );
    let brdf_path = dir.path().join("ont.brdf");
    std::fs::write(&brdf_path, &brdf_bytes).expect("write brdf");
    let brdf_ont = parse_file_auto(&brdf_path).expect(".brdf must parse as bytes");
    assert_eq!(
        declaration_iris(&brdf_ont),
        declaration_iris(&brdf_ont_src),
        ".brdf file round-trip must preserve declarations"
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. Content detection
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn content_detection_b5_magic_and_structure() {
    let path = std::path::Path::new("detected.dat");

    // HDT: `$HDT` magic (the leading bytes of a real binary HDT file).
    assert_eq!(
        detect_format_from_content_public(path, "$HDT\ntriples=0\n").expect("hdt detect"),
        OntologyFormat::Hdt
    );

    // BinaryRDF: `BRDF` magic prefix (text view; byte-exact below).
    assert_eq!(
        detect_format_from_content_public(path, "BRDF\x01\x00fake").expect("brdf detect"),
        OntologyFormat::BinaryRdf
    );

    // JSON-LD: JSON object carrying `@context` / `@graph`.
    assert_eq!(
        detect_format_from_content_public(
            path,
            r#"{"@context":{"ex":"http://example.org/"},"@graph":[]}"#
        )
        .expect("jsonld detect"),
        OntologyFormat::JsonLd
    );
    assert_eq!(
        detect_format_from_content_public(path, r#"{"@graph":[{"@id":"http://example.org/a"}]}"#)
            .expect("graph-only jsonld detect"),
        OntologyFormat::JsonLd
    );

    // HTML / XHTML → RDFa (must win over the generic `<…>` → OWL/XML rule).
    assert_eq!(
        detect_format_from_content_public(
            path,
            "<!DOCTYPE html>\n<html><body vocab=\"http://example.org/\"></body></html>"
        )
        .expect("doctype detect"),
        OntologyFormat::Rdfa
    );
    assert_eq!(
        detect_format_from_content_public(path, "<html lang=\"en\"><body></body></html>")
            .expect("html detect"),
        OntologyFormat::Rdfa
    );
}

/// Byte-level detection must recognise binary magic on content that is *not*
/// valid UTF-8 (the whole point of the byte-oriented read path).
#[test]
fn content_detection_from_bytes_handles_non_utf8() {
    let path = std::path::Path::new("detected.bin");

    // BRDF header + a non-UTF-8 tail (varint bytes ≥ 0x80).
    let mut brdf = b"BRDF".to_vec();
    brdf.extend_from_slice(&[0x82, 0x01, 0xFF, 0xFE]);
    assert_eq!(
        detect_format_from_bytes(path, &brdf).expect("brdf bytes detect"),
        OntologyFormat::BinaryRdf
    );

    // `$HDT` magic.
    assert_eq!(
        detect_format_from_bytes(path, b"$HDT\ntriples=0\n").expect("hdt bytes detect"),
        OntologyFormat::Hdt
    );

    // Plain UTF-8 still delegates to the text heuristics.
    assert_eq!(
        detect_format_from_bytes(path, b"@prefix ex: <http://example.org/> .\nex:a ex:b .\n")
            .expect("turtle bytes detect"),
        OntologyFormat::Turtle
    );
}

/// The pre-existing detections must not have been displaced by the B5 rules.
#[test]
fn content_detection_existing_formats_preserved() {
    let path = std::path::Path::new("detected.dat");

    assert_eq!(
        detect_format_from_content_public(
            path,
            "@prefix ex: <http://example.org/> .\nex:a ex:b .\n"
        )
        .expect("turtle detect"),
        OntologyFormat::Turtle
    );
    assert_eq!(
        detect_format_from_content_public(
            path,
            "Prefix(:=<http://example.org/>)\nOntology(<http://example.org/onto>)\n)"
        )
        .expect("functional detect"),
        OntologyFormat::Functional
    );
    assert_eq!(
        detect_format_from_content_public(
            path,
            "<?xml version=\"1.0\"?>\n<rdf:RDF xmlns:rdf=\"http://www.w3.org/1999/02/22-rdf-syntax-ns#\"></rdf:RDF>"
        )
        .expect("rdf/xml detect"),
        OntologyFormat::RdfXml
    );
    assert_eq!(
        detect_format_from_content_public(
            path,
            "Prefix: : <http://example.org/>\nOntology: <http://example.org/>\nClass: :A\n"
        )
        .expect("manchester detect"),
        OntologyFormat::Manchester
    );
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Binary-safe I/O — BinaryRDF byte round-trip
// ═════════════════════════════════════════════════════════════════════════════

/// The regression the string-save path caused: value ids ≥ 128 are
/// varint-encoded with a continuation bit (bytes ≥ 0x80), which a `String`
/// cannot hold.  The byte path must round-trip losslessly and the string path
/// must *refuse* instead of corrupting.  (The full Binary RDF 1.0 record
/// layout is asserted in `binary_rdf_tests.rs` — this test locks the
/// registry's byte-safe plumbing.)
#[test]
fn binary_rdf_byte_round_trip_with_128plus_dictionary() {
    let n = 130; // ≥ 128 ⇒ value ids 128/129 encode as 2-byte varints
    let ont = ontology_with_class_declarations(n);
    let original = declaration_iris(&ont);
    assert_eq!(original.len(), n);

    // Byte save produces the `BRDF` stream with multi-byte varints.
    let bytes = save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("byte save must succeed");
    assert!(bytes.starts_with(b"BRDF"), "BRDF magic header expected");
    assert!(
        bytes.iter().any(|&b| b >= 0x80),
        "varint bytes ≥ 0x80 expected for value ids ≥ 128"
    );

    // String save must refuse (never corrupt).
    assert!(
        save_to_string(&ont, OntologyFormat::BinaryRdf).is_err(),
        "BinaryRDF must not be representable as a String (use save_to_bytes)"
    );

    // Byte parse round-trips every declaration.
    let reloaded = parse_bytes(&bytes, OntologyFormat::BinaryRdf).expect("byte parse");
    assert_eq!(
        declaration_iris(&reloaded),
        original,
        "byte round-trip must preserve all {n} declaration IRIs"
    );

    // File round-trip through the auto-detected path (extension + byte read).
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("roundtrip.brdf");
    std::fs::write(&path, &bytes).expect("write brdf");
    let auto = parse_file_auto(&path).expect("parse_file_auto(.brdf)");
    assert_eq!(declaration_iris(&auto), original);
}

#[test]
fn hdt_byte_save_round_trips() {
    let ont = ontology_with_class_declarations(10);
    let original = declaration_iris(&ont);

    let bytes = save_to_bytes(&ont, OntologyFormat::Hdt).expect("hdt byte save");
    assert!(bytes.starts_with(b"$HDT"), "$HDT magic expected");

    // HDT is binary: `save_to_string` must refuse instead of corrupting
    // (same contract as BinaryRDF — B5 Slice 5).
    assert!(
        save_to_string(&ont, OntologyFormat::Hdt).is_err(),
        "HDT must not be representable as a String (use save_to_bytes)"
    );

    let reloaded = parse_bytes(&bytes, OntologyFormat::Hdt).expect("hdt byte parse");
    assert_eq!(declaration_iris(&reloaded), original);
}

// ═════════════════════════════════════════════════════════════════════════════
// 4. Reachability — ParserFactory / FormatParser / OntologyLoader
// ═════════════════════════════════════════════════════════════════════════════

const RIO_FORMATS: &[OntologyFormat] = &[
    OntologyFormat::NQuads,
    OntologyFormat::N3,
    OntologyFormat::TriG,
    OntologyFormat::TriX,
    OntologyFormat::JsonLd,
    OntologyFormat::RdfJson,
    OntologyFormat::Rdfa,
    OntologyFormat::BinaryRdf,
    OntologyFormat::Hdt,
];

/// CR-008: `ParserFactory` used to reject every RIO format.  All of them must
/// now produce a parser (the RIO formats are bridged through `FormatParser`).
#[test]
fn parser_factory_reaches_all_rio_formats() {
    for &format in RIO_FORMATS {
        assert!(
            ParserFactory::create_parser(format).is_ok(),
            "create_parser({format:?}) must succeed"
        );
        assert!(
            ParserFactory::create_parser_with_rdf_version(format, RdfCompatibilityMode::RDF11)
                .is_ok(),
            "create_parser_with_rdf_version({format:?}) must succeed"
        );
    }
}

/// Round-trips through the `Parser`-trait bridge for the text formats whose
/// renderer/parser pair is symmetric (registry plumbing, not spec compliance).
///
/// The *binary* formats (BinaryRDF, HDT) are excluded from the string path
/// on purpose: `save_to_string` refuses them by design (a `String` cannot
/// hold their bytes losslessly), so they reach the bridge through the
/// byte-oriented `parse_file` — exercised in the second loop.
#[test]
fn format_parser_bridge_round_trips_text_rio_formats() {
    let ont = ontology_with_class_declarations(3);
    let original = declaration_iris(&ont);

    for &format in &[
        OntologyFormat::N3,
        OntologyFormat::TriG,
        OntologyFormat::JsonLd,
    ] {
        let text = save_to_string(&ont, format).expect("text save");
        let parser = ParserFactory::create_parser(format).expect("factory parser");
        let back = parser.parse(&text).unwrap_or_else(|e| {
            panic!("Parser-trait parse failed for {format:?}: {e}");
        });
        assert_eq!(
            declaration_iris(&back),
            original,
            "{format:?} must round-trip through the Parser-trait bridge"
        );
    }

    // Binary formats reach the bridge through the byte-oriented `parse_file`
    // (real BinaryRDF 1.0 records and real binary HDT files — B5 Slices 3/5).
    for (format, ext) in [
        (OntologyFormat::BinaryRdf, "brdf"),
        (OntologyFormat::Hdt, "hdt"),
    ] {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(format!("bridge.{ext}"));
        std::fs::write(&path, save_to_bytes(&ont, format).expect("binary bytes"))
            .expect("write binary file");
        let binary_parser: Box<dyn Parser> =
            ParserFactory::create_parser(format).expect("binary parser");
        let back = binary_parser.parse_file(&path).expect("bridge parse_file");
        assert_eq!(
            declaration_iris(&back),
            original,
            "{format:?} must round-trip through the byte-oriented bridge"
        );
    }
}

/// `FormatParser` (the thin trait adapter) dispatches `BinaryRdf` through the
/// byte-oriented file path — never a UTF-8 decode of the raw stream.
#[test]
fn format_parser_reads_binary_bytes() {
    // 130 entries ⇒ varint bytes ≥ 0x80 ⇒ raw non-UTF-8 payload.
    let ont = ontology_with_class_declarations(130);
    let bytes = save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("bytes");
    assert!(bytes.iter().any(|&b| b >= 0x80), "non-UTF-8 varint bytes");
    assert!(
        std::str::from_utf8(&bytes).is_err(),
        "payload must be genuinely non-UTF-8"
    );

    let parser = FormatParser {
        format: OntologyFormat::BinaryRdf,
    };
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("x.brdf");
    std::fs::write(&path, &bytes).expect("write");
    let back = parser.parse_file(&path).expect("byte parse");
    assert_eq!(declaration_iris(&back), declaration_iris(&ont));
}

/// `OntologyLoader::load_file` must reach the B5 formats: byte detection for
/// the binary ones, content/extension detection for the text ones, and
/// `load_from_string` must go through the `ParserFactory` bridge.
#[test]
fn ontology_loader_reaches_rio_formats() {
    let dir = tempfile::tempdir().expect("tempdir");
    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let loader = OntologyLoader::new(manager);

    // JSON-LD file (extension + content detection).
    let jsonld_path = dir.path().join("loaded.jsonld");
    std::fs::write(
        &jsonld_path,
        r#"{"@context":{"ex":"http://example.org/","owl":"http://www.w3.org/2002/07/owl#"},"@graph":[{"@id":"ex:A","@type":"owl:Class"}]}"#,
    )
    .expect("write jsonld");
    let loaded = loader
        .load_file(&jsonld_path)
        .expect("loader must reach the JSON-LD parser");
    assert!(
        declaration_iris(&loaded.read().expect("lock")).contains("http://example.org/A"),
        "loaded JSON-LD ontology must carry the class declaration"
    );

    // BinaryRDF file — raw bytes, never `read_to_string`.
    let ont = ontology_with_class_declarations(130);
    let brdf_path = dir.path().join("loaded.brdf");
    std::fs::write(
        &brdf_path,
        save_to_bytes(&ont, OntologyFormat::BinaryRdf).expect("brdf bytes"),
    )
    .expect("write brdf");
    let loaded = loader
        .load_file(&brdf_path)
        .expect("loader must reach the BinaryRDF byte parser");
    assert_eq!(
        declaration_iris(&loaded.read().expect("lock")),
        declaration_iris(&ont)
    );

    // load_from_string → ParserFactory bridge (N3 via `FormatParser`).
    let n3_doc = "@prefix ex: <http://example.org/> .\nex:a ex:knows ex:b .\n";
    let loaded = loader
        .load_from_string(n3_doc, OntologyFormat::N3, "inline.n3")
        .expect("load_from_string must reach the N3 parser");
    assert!(
        !loaded.read().expect("lock").axioms().is_empty(),
        "N3 document must produce axioms through load_from_string"
    );
}

/// CR-011 F-3: `OntologyLoader::load_file` used to sniff content only — the
/// file extension was ignored, so an XML-shaped HTML fragment in `page.html`
/// (no `<!DOCTYPE html>` / `<html>` head) was detected as OWL/XML and
/// silently yielded 0 axioms.  Detection now consults the shared
/// extension table first, consistent with `parse_file_auto`.
#[test]
fn loader_uses_extension_for_html_fragment() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("fragment.html");
    // XML-shaped fragment: content sniffing alone would say OWL/XML.
    std::fs::write(&path, "<div typeof=\"http://example.org/Thing\">text</div>")
        .expect("write fragment");

    // Content-only detection really is ambiguous for this input…
    assert_eq!(
        detect_format_from_content_public(
            &path,
            "<div typeof=\"http://example.org/Thing\">text</div>"
        )
        .expect("detect"),
        OntologyFormat::Rdfa,
        "the .html extension must route to RDFa (shared extension table)"
    );

    let manager = Arc::new(RwLock::new(OntologyManager::new()));
    let loader = OntologyLoader::new(manager);
    let loaded = loader
        .load_file(&path)
        .expect("loader must parse the .html fragment as RDFa");
    let guard = loaded.read().expect("lock");
    assert!(
        !guard.axioms().is_empty(),
        "the .html fragment must produce axioms (F-3: no silent 0-axiom parse)"
    );
}

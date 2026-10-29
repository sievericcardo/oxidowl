//! JSON-LD 1.1 conformance tests (B5 Slice 1 — docs/PLAN.md).
//!
//! Locks the oxjsonld/oxrdfio-based parser and serializer in
//! `src/parsers/rio/jsonld.rs`:
//!
//! 1. **Expansion equivalence** — a fully *expanded* document (absolute IRIs,
//!    array-valued properties) and a *compacted* `@context` + `@graph`
//!    document for the same ontology must parse to equal axiom sets.
//! 2. **Context features** — `@vocab` term resolution, `@base`/parser-base
//!    relative-IRI resolution, and `@type` coercion (both value datatype
//!    coercion and `"@type": "@id"` IRI coercion).
//! 3. **Serialize → parse axiom equality** for `SubClassOf`, `ClassAssertion`,
//!    `ObjectPropertyAssertion`, `DataPropertyAssertion`, `AnnotationAssertion`,
//!    `ObjectSomeValuesFrom` (incl. inside `EquivalentClasses`) and nested
//!    blank-node restrictions, plus language-tagged literals.

#[allow(dead_code)]
#[path = "helpers/mod.rs"]
mod helpers;

use helpers::assertions::assert_ontologies_axiom_equal;
use helpers::df::DF;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::parsers::rio::jsonld::JsonLdParser;
use oxidowl::parsers::{parse_str, save_to_string};

/// Serialize an ontology as JSON-LD and parse it back.
fn jsonld_roundtrip(ont: &Ontology) -> Ontology {
    let text = save_to_string(ont, OntologyFormat::JsonLd)
        .unwrap_or_else(|e| panic!("JSON-LD serialization failed: {e}"));
    parse_str(&text, OntologyFormat::JsonLd)
        .unwrap_or_else(|e| panic!("JSON-LD parse-back failed: {e}"))
}

/// Parse a JSON-LD document, panicking with context on failure.
fn parse_jsonld(doc: &str) -> Ontology {
    parse_str(doc, OntologyFormat::JsonLd).unwrap_or_else(|e| panic!("JSON-LD parse failed: {e}"))
}

// ═════════════════════════════════════════════════════════════════════════════
// 1. Expanded vs compacted document equivalence
// ═════════════════════════════════════════════════════════════════════════════

/// Fully expanded JSON-LD 1.1 form: absolute IRIs everywhere, properties and
/// `@type` as arrays, no `@context`.
const EXPANDED_DOC: &str = r#"
{
  "@id": "http://example.org/onto",
  "@type": ["http://www.w3.org/2002/07/owl#Ontology"],
  "@graph": [
    {
      "@id": "http://example.org/A",
      "@type": ["http://www.w3.org/2002/07/owl#Class"]
    },
    {
      "@id": "http://example.org/B",
      "@type": ["http://www.w3.org/2002/07/owl#Class"]
    },
    {
      "@id": "http://example.org/p",
      "@type": ["http://www.w3.org/2002/07/owl#ObjectProperty"]
    },
    {
      "@id": "http://example.org/age",
      "@type": ["http://www.w3.org/2002/07/owl#DatatypeProperty"]
    },
    {
      "@id": "http://example.org/i",
      "@type": [
        "http://www.w3.org/2002/07/owl#NamedIndividual",
        "http://example.org/A"
      ],
      "http://example.org/p": [{ "@id": "http://example.org/j" }],
      "http://example.org/age": [
        { "@value": "42", "@type": "http://www.w3.org/2001/XMLSchema#integer" }
      ]
    },
    {
      "@id": "http://example.org/j",
      "@type": ["http://www.w3.org/2002/07/owl#NamedIndividual"]
    },
    {
      "@id": "http://example.org/A",
      "http://www.w3.org/2000/01/rdf-schema#subClassOf": [
        { "@id": "http://example.org/B" }
      ]
    }
  ]
}
"#;

/// Compacted form of the same ontology: `@context` with prefixes, `@base`,
/// `@type` coercion (`xsd:integer` value coercion and `"@type": "@id"` IRI
/// coercion) and native `@type` keywords.
const COMPACTED_DOC: &str = r#"
{
  "@context": {
    "@base": "http://example.org/",
    "owl": "http://www.w3.org/2002/07/owl#",
    "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
    "xsd": "http://www.w3.org/2001/XMLSchema#",
    "p": { "@id": "http://example.org/p", "@type": "@id" },
    "age": { "@id": "http://example.org/age", "@type": "xsd:integer" }
  },
  "@graph": [
    { "@id": "onto", "@type": "owl:Ontology" },
    { "@id": "A", "@type": "owl:Class" },
    { "@id": "B", "@type": "owl:Class" },
    { "@id": "p", "@type": "owl:ObjectProperty" },
    { "@id": "age", "@type": "owl:DatatypeProperty" },
    {
      "@id": "i",
      "@type": ["owl:NamedIndividual", "A"],
      "p": "j",
      "age": "42"
    },
    { "@id": "j", "@type": "owl:NamedIndividual" },
    { "@id": "A", "rdfs:subClassOf": { "@id": "B" } }
  ]
}
"#;

#[test]
fn expanded_and_compacted_documents_yield_equal_axioms() {
    let expanded = parse_jsonld(EXPANDED_DOC);
    let compacted = parse_jsonld(COMPACTED_DOC);

    // Sanity: the docs are not empty and both anchor the ontology IRI.
    assert!(
        !expanded.axioms().is_empty(),
        "expanded document must produce axioms"
    );
    assert_eq!(
        expanded.get_iri().map(ToString::to_string).as_deref(),
        Some("http://example.org/onto"),
        "expanded document must recover the ontology IRI"
    );
    assert_eq!(
        compacted.get_iri().map(ToString::to_string).as_deref(),
        Some("http://example.org/onto"),
        "compacted document must resolve the relative ontology IRI via @base"
    );

    // The two syntactic forms must map to the identical axiom set.
    assert_ontologies_axiom_equal(&expanded, &compacted);

    // Spot-check the expected content (declarations + four assertion kinds).
    assert!(
        expanded
            .axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::SubClassOf(_))),
        "SubClassOf missing"
    );
    assert!(
        expanded
            .axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::ClassAssertion(_))),
        "ClassAssertion missing"
    );
    assert!(
        expanded
            .axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::ObjectPropertyAssertion(_))),
        "ObjectPropertyAssertion missing"
    );
    assert!(
        expanded
            .axioms()
            .iter()
            .any(|ax| matches!(ax, Axiom::DataPropertyAssertion(_))),
        "DataPropertyAssertion missing"
    );
    // The xsd:integer datatype from the `@type`-coerced value must survive
    // in *both* forms.
    for ont in [&expanded, &compacted] {
        assert!(
            ont.axioms().iter().any(|ax| {
                matches!(ax, Axiom::DataPropertyAssertion(d)
                    if d.value.datatype.as_ref().is_some_and(|dt|
                        dt.as_str() == "http://www.w3.org/2001/XMLSchema#integer"))
            }),
            "typed DataPropertyAssertion (xsd:integer) missing; axioms: {:?}",
            ont.axioms()
        );
    }
}

// ═════════════════════════════════════════════════════════════════════════════
// 2. @vocab / @base resolution and @type coercion
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn at_vocab_resolves_bare_terms() {
    // `subClassOf` (no prefix, no colon) must expand against `@vocab`.
    let doc = r#"
    {
      "@context": {
        "@vocab": "http://www.w3.org/2000/01/rdf-schema#",
        "owl": "http://www.w3.org/2002/07/owl#",
        "ex": "http://example.org/"
      },
      "@graph": [
        { "@id": "ex:A", "@type": "owl:Class" },
        { "@id": "ex:B", "@type": "owl:Class" },
        { "@id": "ex:A", "subClassOf": { "@id": "ex:B" } }
      ]
    }"#;
    let ont = parse_jsonld(doc);
    let df = DF::new();
    assert_ontologies_axiom_equal(
        &ont,
        &df.build_ontology(vec![
            df.declaration_axiom(Entity::Class(IRI::new("http://example.org/A"))),
            df.declaration_axiom(Entity::Class(IRI::new("http://example.org/B"))),
            df.sub_class_of(
                df.class_ce("http://example.org/A"),
                df.class_ce("http://example.org/B"),
            ),
        ]),
    );
}

#[test]
fn at_base_resolves_relative_iris() {
    // Relative `@id`s and `@type` values resolve against the document `@base`.
    let doc = r#"
    {
      "@context": {
        "@base": "http://example.org/base/",
        "owl": "http://www.w3.org/2002/07/owl#",
        "rdfs": "http://www.w3.org/2000/01/rdf-schema#",
        "knows": "http://example.org/knows"
      },
      "@graph": [
        { "@id": "A", "@type": "owl:Class" },
        { "@id": "B", "@type": "owl:Class" },
        { "@id": "A", "rdfs:subClassOf": { "@id": "B" } },
        { "@id": "i", "knows": { "@id": "j" } }
      ]
    }"#;
    let ont = parse_jsonld(doc);
    let df = DF::new();
    let expected = df.build_ontology(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://example.org/base/A"))),
        df.declaration_axiom(Entity::Class(IRI::new("http://example.org/base/B"))),
        df.sub_class_of(
            df.class_ce("http://example.org/base/A"),
            df.class_ce("http://example.org/base/B"),
        ),
        df.object_property_assertion(
            df.obj_prop("http://example.org/knows"),
            df.named("http://example.org/base/i"),
            df.named("http://example.org/base/j"),
        ),
    ]);
    assert_ontologies_axiom_equal(&ont, &expected);

    // A document without `@base` uses the parser's base IRI option instead.
    let no_base = r#"
    {
      "@context": {
        "owl": "http://www.w3.org/2002/07/owl#",
        "rdfs": "http://www.w3.org/2000/01/rdf-schema#"
      },
      "@graph": [
        { "@id": "A", "@type": "owl:Class" },
        { "@id": "B", "@type": "owl:Class" },
        { "@id": "A", "rdfs:subClassOf": { "@id": "B" } }
      ]
    }"#;
    let ont = JsonLdParser::new()
        .with_base_iri("http://example.org/opt/")
        .parse(no_base)
        .unwrap_or_else(|e| panic!("parse with parser base failed: {e}"));
    assert!(
        ont.axioms().iter().any(|ax| {
            let debug = format!("{ax:?}");
            debug.contains("http://example.org/opt/A") && debug.contains("http://example.org/opt/B")
        }),
        "relative IRIs must resolve against the parser base IRI; axioms: {:?}",
        ont.axioms()
    );
}

#[test]
fn at_type_coercion_value_datatype_and_iri() {
    // Value coercion: `"age": {"@id": …, "@type": "xsd:integer"}` makes the
    // plain string `"42"` an xsd:integer literal → DataPropertyAssertion.
    // IRI coercion: `"knows": {"@id": …, "@type": "@id"}` makes the plain
    // string `"ex:j"` an IRI → ObjectPropertyAssertion.
    let doc = r#"
    {
      "@context": {
        "owl": "http://www.w3.org/2002/07/owl#",
        "xsd": "http://www.w3.org/2001/XMLSchema#",
        "ex": "http://example.org/",
        "age": { "@id": "http://example.org/age", "@type": "xsd:integer" },
        "knows": { "@id": "http://example.org/knows", "@type": "@id" }
      },
      "@graph": [
        {
          "@id": "ex:i",
          "@type": "owl:NamedIndividual",
          "age": "42",
          "knows": "ex:j"
        }
      ]
    }"#;
    let ont = parse_jsonld(doc);
    let df = DF::new();
    let expected = df.build_ontology(vec![
        df.declaration_axiom(Entity::NamedIndividual(IRI::new("http://example.org/i"))),
        df.data_property_assertion(
            df.data_prop("http://example.org/age"),
            df.named("http://example.org/i"),
            df.int_literal(42),
        ),
        df.object_property_assertion(
            df.obj_prop("http://example.org/knows"),
            df.named("http://example.org/i"),
            df.named("http://example.org/j"),
        ),
    ]);
    assert_ontologies_axiom_equal(&ont, &expected);
}

// ═════════════════════════════════════════════════════════════════════════════
// 3. Serialize → parse axiom equality
// ═════════════════════════════════════════════════════════════════════════════

#[test]
fn serialize_parse_roundtrip_core_axiom_kinds() {
    let df = DF::new();
    let a = df.class_ce("http://example.org/A");
    let b = df.class_ce("http://example.org/B");
    let c = df.class_ce("http://example.org/C");
    let d = df.class_ce("http://example.org/D");
    let p = df.obj_prop("http://example.org/p");
    let dp = df.data_prop("http://example.org/age");
    let i = df.named("http://example.org/i");
    let j = df.named("http://example.org/j");

    let mut ont = df.build_ontology(vec![
        // SubClassOf
        df.sub_class_of(a.clone(), b.clone()),
        // ClassAssertion
        df.class_assertion(c.clone(), i.clone()),
        // ObjectPropertyAssertion
        df.object_property_assertion(p.clone(), i.clone(), j.clone()),
        // DataPropertyAssertion (typed literal)
        df.data_property_assertion(dp.clone(), i.clone(), df.int_literal(42)),
        // AnnotationAssertion
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: df.next_id(),
            subject: AnnotationSubject::IRI(IRI::new("http://example.org/A")),
            property: df.annotation_property("http://example.org/ann"),
            value: AnnotationValue::Literal(df.literal("hello")),
            annotations: vec![],
        }),
        // EquivalentClasses (named pair)
        df.equivalent_classes(vec![c.clone(), d.clone()]),
        // EquivalentClasses + ObjectSomeValuesFrom (blank-node restriction)
        df.equivalent_classes(vec![df.some_values_from(p.clone(), b.clone()), a.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let reloaded = jsonld_roundtrip(&ont);
    assert_ontologies_axiom_equal(&ont, &reloaded);

    // Every required kind must be individually present after the round-trip.
    for kind in [
        AxiomType::SubClassOf,
        AxiomType::ClassAssertion,
        AxiomType::ObjectPropertyAssertion,
        AxiomType::DataPropertyAssertion,
        AxiomType::AnnotationAssertion,
        AxiomType::EquivalentClasses,
    ] {
        assert!(
            reloaded.axioms().iter().any(|ax| ax.axiom_type() == kind),
            "{kind:?} lost in JSON-LD round-trip; axioms: {:?}",
            reloaded.axioms()
        );
    }
}

#[test]
fn serialize_parse_roundtrip_nested_blank_node_restrictions() {
    let df = DF::new();
    let a = df.class_ce("http://example.org/A");
    let b = df.class_ce("http://example.org/B");
    let p = df.obj_prop("http://example.org/p");
    let q = df.obj_prop("http://example.org/q");
    let dp = df.data_prop("http://example.org/age");
    let i = df.named("http://example.org/i");

    // A ⊑ ∃p.(∃q.B)  — two nested blank-node restrictions.
    // i ∈ ∃p.B        — a single blank-node restriction as a class assertion.
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(
            a.clone(),
            df.some_values_from(p.clone(), df.some_values_from(q.clone(), b.clone())),
        ),
        df.class_assertion(df.some_values_from(p.clone(), b.clone()), i.clone()),
        df.data_property_assertion(dp, i, df.int_literal(42)),
    ]);
    df.auto_declare(&mut ont);

    let reloaded = jsonld_roundtrip(&ont);
    assert_ontologies_axiom_equal(&ont, &reloaded);

    // The nesting depth must be preserved: the outer superclass is an
    // ObjectSomeValuesFrom whose filler is itself an ObjectSomeValuesFrom.
    let nested_found = reloaded.axioms().iter().any(|ax| {
        if let Axiom::SubClassOf(s) = ax
            && let ClassExpression::ObjectSomeValuesFrom { filler, .. } = &s.superclass
        {
            return matches!(
                filler.as_ref(),
                ClassExpression::ObjectSomeValuesFrom { .. }
            );
        }
        false
    });
    assert!(
        nested_found,
        "nested blank-node restriction lost; axioms: {:?}",
        reloaded.axioms()
    );
}

#[test]
fn serialize_parse_roundtrip_language_tagged_literals() {
    let df = DF::new();
    let ap = df.annotation_property("http://example.org/label");
    let a = df.class_ce("http://example.org/A");

    let mut ont = df.build_ontology(vec![
        df.declaration_axiom(Entity::Class(IRI::new("http://example.org/A"))),
        df.declaration_axiom(Entity::AnnotationProperty(IRI::new(
            "http://example.org/label",
        ))),
        Axiom::AnnotationAssertion(AnnotationAssertionAxiom {
            id: df.next_id(),
            subject: AnnotationSubject::IRI(IRI::new("http://example.org/A")),
            property: ap,
            value: AnnotationValue::Literal(df.lang_literal("chat", "fr")),
            annotations: vec![],
        }),
        df.sub_class_of(a, df.class_ce("http://example.org/B")),
    ]);
    df.auto_declare(&mut ont);

    let reloaded = jsonld_roundtrip(&ont);
    assert_ontologies_axiom_equal(&ont, &reloaded);
    assert!(
        reloaded.axioms().iter().any(|ax| {
            matches!(ax, Axiom::AnnotationAssertion(ann)
                if matches!(&ann.value, AnnotationValue::Literal(l)
                    if l.language.as_deref() == Some("fr") && l.value == "chat"))
        }),
        "language tag lost in JSON-LD round-trip; axioms: {:?}",
        reloaded.axioms()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// CR-011 M3 — remote `@context` fetching is opt-in, OFF by default (SSRF)
// ─────────────────────────────────────────────────────────────────────────────

/// Parsing untrusted JSON-LD must never issue HTTP requests to
/// document-controlled URLs: remote contexts fail with a clear error unless
/// the caller opts in with `with_remote_contexts(true)` (CR-011 M3).
#[test]
fn remote_context_is_rejected_by_default() {
    let doc = r#"{"@context":"https://example.org/remote.jsonld","@id":"http://example.org/a"}"#;
    let err = JsonLdParser::new()
        .parse(doc)
        .expect_err("remote @context must be rejected by default (fetching is opt-in)");
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("remote context") || msg.contains("load document"),
        "the error must clearly name the remote-context problem, got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// CR-011 M2 — N3 rule + IRI-less ontology annotation survive the round-trip
// ─────────────────────────────────────────────────────────────────────────────

const LOG_FORMULA: &str = "http://www.w3.org/2000/10/swap/log#Formula";

/// N3 source carrying (a) a SWRL rule and (b) a standalone formula that the
/// N3 parser preserves as a `log:Formula` ontology annotation — on an
/// ontology with **no IRI**, the case the shared writer used to drop.
fn n3_rule_and_annotation_source() -> &'static str {
    "@prefix ex: <http://example.org/> .\n\
     { ?x a ex:A } => { ?x a ex:B } .\n\
     { ex:k ex:q ex:v } .\n"
}

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

/// The regression behind CR-011 M2: an N3 rule and an IRI-less ontology
/// annotation must survive serialize → parse through JSON-LD (previously
/// the rule vanished with `Rule → vec![]` and the annotation with the
/// `get_iri()` gate).
#[test]
fn n3_rule_and_iri_less_annotation_survive_jsonld_roundtrip() {
    let ont = parse_str(n3_rule_and_annotation_source(), OntologyFormat::N3)
        .expect("N3 fixture must parse");
    assert!(ont.get_iri().is_none(), "fixture ontology must be IRI-less");
    let rule = rule_of(&ont);
    let annotations = formula_annotation(&ont);
    assert_eq!(annotations.len(), 1, "fixture must carry one log:Formula");

    let json = save_to_string(&ont, OntologyFormat::JsonLd).expect("JSON-LD serialize");
    // The preservation representation is visible in the document …
    assert!(
        json.contains("urn:oxidowl:swrlRule") && json.contains("DLSafeRule"),
        "rule must be present in the JSON-LD document, got:\n{json}"
    );
    assert!(
        json.contains("urn:oxidowl:ontology"),
        "IRI-less annotations must hang off the fallback subject, got:\n{json}"
    );

    // … and decodes back to the same data.
    let back = parse_str(&json, OntologyFormat::JsonLd).expect("JSON-LD reparse");
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

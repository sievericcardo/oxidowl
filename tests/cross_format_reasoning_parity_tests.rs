//! Cross-format reasoning parity tests (workstream C3).
//!
//! The same non-trivial ontology is loaded through each of RDF/XML, Turtle,
//! Functional Syntax, OWL/XML (serializer→parser round trip from a canonical
//! in-memory ontology) and Manchester (hand-written string, parsed directly
//! with `ManchesterParser`). Every format must agree on:
//!
//! 1. consistency,
//! 2. the full class-hierarchy from `classify()` (canonicalised to
//!    subclass → sorted superclasses),
//! 3. a battery of positive + negative `is_entailed` checks.
//!
//! # Documented format limitations (all verified empirically in this crate)
//!
//! The four serialisers/parsers are NOT mutually lossless, so the *shared*
//! parity ontology uses the intersection of what every format preserves:
//! named-class hierarchy, transitive/functional characteristics, class
//! assertions, object property assertions and disjoint classes. Everything
//! else is split into per-format capability tests:
//!
//! - **RDF/XML**: the parser misreads `rdfs:subPropertyOf` and `rdfs:range`
//!   triples as bogus `ObjectPropertyAssertion`s, and drops typed
//!   `DataPropertyAssertion`s and the literal datatype. (Verified: hand-written
//!   spec-correct RDF/XML exhibits the same behaviour.)
//! - **Turtle**: drops `SubObjectPropertyOf` and `DataPropertyRange`; keeps
//!   data assertions but loses the literal datatype.
//! - **OWL/XML**: drops `DataPropertyAssertion` and `DataPropertyRange`;
//!   keeps `SubObjectPropertyOf`.
//! - **Functional**: lossless for all of the above (the reference format).
//! - **Manchester**: no `ObjectOneOf` (`{b}`) syntax; otherwise handles the
//!   core plus data assertions / sub-properties.
//! - **All four non-Functional serialisers** drop `EquivalentClasses` axioms
//!   whose other side is a complex class expression (`∃R.⊤`, `≥1 R.⊤`,
//!   `≥1 R.{b}`) — only the entity declarations are emitted.

mod helpers;

use helpers::df::DF;
use oxidowl::{
    OWLReasonerConfiguration, ReasonerFactory, TableauReasonerFactory,
    config::{BlockingStrategy, ReasonerConfig, ReasoningConfig},
    core::reasoner::{ClassificationResult, Reasoner},
    ontology::{Axiom, ClassExpression, Ontology, OntologyFormat},
    parsers::{ManchesterParser, ManchesterParserConfig, save_to_string},
    parsers::{parse_functional, parse_owl_xml, parse_rdf_xml, parse_turtle},
};
use std::collections::BTreeMap;
use std::time::Duration;

const EX: &str = "http://parity.org/";
const XSD: &str = "http://www.w3.org/2001/XMLSchema#";

// ──────────────────────────────────────────────────────────────────────────────
// Canonical ontologies + Manchester source
// ──────────────────────────────────────────────────────────────────────────────

/// Build the CORE canonical ontology shared by all serialised formats — the
/// intersection of what every format preserves faithfully (see module doc).
fn canonical_ontology() -> Ontology {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let d_cls = df.class_ce(format!("{EX}D"));
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));
    let d = df.named(format!("{EX}d"));
    let e = df.named(format!("{EX}e"));

    let mut ont = df.build_ontology(vec![
        // Class hierarchy: A ⊑ B ⊑ C; A and D disjoint.
        df.sub_class_of(a_cls.clone(), b_cls.clone()),
        df.sub_class_of(b_cls.clone(), c_cls.clone()),
        df.disjoint_classes(vec![a_cls.clone(), d_cls.clone()]),
        // RBox: R transitive, S functional.
        df.transitive_object_property(r.clone()),
        df.functional_object_property(s.clone()),
        // ABox: a:A, b:B, c:C, d:D; R(a,b), R(b,c); S(c,d), S(c,e).
        df.class_assertion(a_cls.clone(), a.clone()),
        df.class_assertion(b_cls.clone(), b.clone()),
        df.class_assertion(c_cls.clone(), c.clone()),
        df.class_assertion(d_cls, d.clone()),
        df.object_property_assertion(r.clone(), a.clone(), b.clone()),
        df.object_property_assertion(r.clone(), b.clone(), c.clone()),
        df.object_property_assertion(s.clone(), c.clone(), d.clone()),
        df.object_property_assertion(s.clone(), c.clone(), e.clone()),
    ]);
    df.auto_declare(&mut ont);
    ont.set_iri(oxidowl::ontology::IRI::new("http://parity.org/ont"));
    ont
}

/// Build the EXTENDED canonical ontology (core + `RBox` data axioms + data
/// property assertion) used only by the format-capability test, because only
/// Functional preserves all of it.
fn extended_ontology() -> Ontology {
    let df = DF::new();
    let ont = canonical_ontology();
    let mut ont = ont;
    ont.add_axiom(
        df.sub_object_property_of(df.obj_prop(format!("{EX}P")), df.obj_prop(format!("{EX}R"))),
    );
    ont.add_axiom(df.data_property_range(
        df.data_prop(format!("{EX}dp")),
        df.data_range_dt(format!("{XSD}string")),
    ));
    ont.add_axiom(df.data_property_assertion(
        df.data_prop(format!("{EX}dp")),
        df.named(format!("{EX}a")),
        df.typed_literal("x", &format!("{XSD}string")),
    ));
    df.auto_declare(&mut ont);
    ont
}

/// Hand-written Manchester source mirroring `canonical_ontology()`.
fn manchester_source() -> &'static str {
    r"Prefix: ex: <http://parity.org/>

Ontology: <http://parity.org/manchester>

Class: ex:A
    SubClassOf: ex:B
    DisjointWith: ex:D

Class: ex:B
    SubClassOf: ex:C

ObjectProperty: ex:R
    Characteristics: Transitive

ObjectProperty: ex:S
    Characteristics: Functional

Individual: ex:a
    Types: ex:A
    Facts: ex:R ex:b

Individual: ex:b
    Types: ex:B
    Facts: ex:R ex:c

Individual: ex:c
    Types: ex:C
    Facts: ex:S ex:d
    Facts: ex:S ex:e

Individual: ex:d
    Types: ex:D

Individual: ex:e
"
}

// ──────────────────────────────────────────────────────────────────────────────
// Reasoning probe
// ──────────────────────────────────────────────────────────────────────────────

/// The set of reasoning results that must be identical across formats.
#[derive(Debug, Clone)]
struct Probe {
    format: &'static str,
    consistent: bool,
    /// Canonical class hierarchy: subclass IRI → sorted superclass IRIs.
    class_hierarchy: BTreeMap<String, Vec<String>>,
    /// `is_entailed` results, one per check in `parity_checks`.
    entailed: Vec<bool>,
}

/// Run every parity check against one parsed ontology.
fn run_probe(format: &'static str, onto: Ontology) -> Probe {
    let df = DF::new();

    // Core reasoner: consistency + classification.
    let mut core = Reasoner::new(ReasonerConfig {
        reasoning: ReasoningConfig {
            blocking_strategy: BlockingStrategy::Anywhere,
            timeout: Some(Duration::from_millis(10000)),
            ..Default::default()
        },
        ..Default::default()
    })
    .expect("reasoner");
    core.load_ontology(onto.clone()).expect("load ontology");
    let consistent = core.is_consistent().expect("is_consistent");
    let classification = core.classify().expect("classify");
    let class_hierarchy = canonical_hierarchy(&classification);

    // API reasoner: semantic entailments.
    let mut onto = onto;
    df.auto_declare(&mut onto);
    let onto_ref = df.onto_ref(onto);
    let factory = TableauReasonerFactory;
    let reasoner = factory
        .create_reasoner(&onto_ref, &OWLReasonerConfiguration::default())
        .expect("create reasoner");

    let entailed = parity_checks(&df)
        .into_iter()
        .map(|(ax, _label)| reasoner.is_entailed(&ax).expect("is_entailed"))
        .collect();

    Probe {
        format,
        consistent,
        class_hierarchy,
        entailed,
    }
}

/// The class-level hierarchy of the classification result, canonicalised to
/// `subclass IRI → sorted superclass IRIs`. Only named-class entries are kept.
fn canonical_hierarchy(result: &ClassificationResult) -> BTreeMap<String, Vec<String>> {
    let mut map = BTreeMap::new();
    for (sub, sups) in &result.hierarchy {
        let ClassExpression::Class(sub_cls) = sub else {
            continue;
        };
        let mut supers: Vec<String> = sups
            .iter()
            .filter_map(|s| match s {
                ClassExpression::Class(c) => Some(c.iri.to_string()),
                _ => None,
            })
            .collect();
        supers.sort();
        supers.dedup();
        map.insert(sub_cls.iri.to_string(), supers);
    }
    map
}

/// The entailment checks every format must agree on. These are *queries*: they
/// do not require the queried complex class expression to appear as an axiom.
fn parity_checks(df: &DF) -> Vec<(Axiom, &'static str)> {
    let a_cls = df.class_ce(format!("{EX}A"));
    let b_cls = df.class_ce(format!("{EX}B"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let r = df.obj_prop(format!("{EX}R"));
    let s = df.obj_prop(format!("{EX}S"));
    let a = df.named(format!("{EX}a"));
    let b = df.named(format!("{EX}b"));
    let c = df.named(format!("{EX}c"));
    let d = df.named(format!("{EX}d"));
    let e = df.named(format!("{EX}e"));

    vec![
        // Positive: class hierarchy (transitive).
        (
            df.sub_class_of(a_cls.clone(), c_cls.clone()),
            "A ⊑ C (transitive)",
        ),
        (df.sub_class_of(a_cls.clone(), b_cls.clone()), "A ⊑ B"),
        (df.sub_class_of(b_cls.clone(), c_cls.clone()), "B ⊑ C"),
        // Negative: no reverse subsumption.
        (df.sub_class_of(c_cls.clone(), a_cls.clone()), "NOT C ⊑ A"),
        (df.sub_class_of(c_cls.clone(), b_cls.clone()), "NOT C ⊑ B"),
        // Instance: a:C via A ⊑ B ⊑ C.
        (df.class_assertion(c_cls.clone(), a.clone()), "a : C"),
        // Existential: a ∈ ∃R.B via R(a,b), b:B.
        (
            df.class_assertion(df.some_values_from(r.clone(), b_cls.clone()), a.clone()),
            "a ∈ ∃R.B",
        ),
        // Transitivity: Trans(R), R(a,b), R(b,c) ⇒ R(a,c).
        (
            df.object_property_assertion(r.clone(), a.clone(), c.clone()),
            "R(a,c) via transitivity",
        ),
        // Functional S ⇒ SameIndividual(d,e).
        (
            df.same_individual(vec![d.clone(), e.clone()]),
            "d ≡ e via Functional(S)",
        ),
        // Disjoint classes ⇒ DifferentIndividuals(a,d).
        (
            df.different_individuals(vec![a.clone(), d.clone()]),
            "a ≠ d via Disjoint(A,D)",
        ),
        // Negative SameIndividual control.
        (df.same_individual(vec![a.clone(), b]), "NOT a ≡ b"),
        // Characteristics.
        (df.transitive_object_property(r), "Transitive(R)"),
        (df.functional_object_property(s), "Functional(S)"),
    ]
}

// ──────────────────────────────────────────────────────────────────────────────
// Assertions
// ──────────────────────────────────────────────────────────────────────────────

fn assert_probes_identical(probes: &[Probe]) {
    let reference = &probes[0];
    for probe in &probes[1..] {
        assert_eq!(
            probe.consistent, reference.consistent,
            "[{}] consistency differs from [{}]: {} vs {}",
            probe.format, reference.format, probe.consistent, reference.consistent
        );
        assert_eq!(
            probe.class_hierarchy, reference.class_hierarchy,
            "[{}] class hierarchy differs from [{}]:\n{:?}\nvs\n{:?}",
            probe.format, reference.format, probe.class_hierarchy, reference.class_hierarchy
        );
        let checks = parity_checks(&DF::new());
        for (i, (got, want)) in probe.entailed.iter().zip(&reference.entailed).enumerate() {
            assert_eq!(
                got, want,
                "[{}] entailment #{} ({}) differs from [{}]: {} vs {}",
                probe.format, i, checks[i].1, reference.format, got, want
            );
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Tests
// ──────────────────────────────────────────────────────────────────────────────

/// Serialise the canonical CORE ontology through each of the four full formats,
/// re-parse it, and assert identical consistency / hierarchy / entailments.
#[test]
fn cross_format_reasoning_parity_serialized_formats() {
    let ont = canonical_ontology();

    let mut probes: Vec<Probe> = Vec::new();
    for (name, format) in [
        ("rdfxml", OntologyFormat::RdfXml),
        ("turtle", OntologyFormat::Turtle),
        ("functional", OntologyFormat::Functional),
        ("owlxml", OntologyFormat::OwlXml),
    ] {
        let serialized = save_to_string(&ont, format).expect("serialize");
        let reloaded = match format {
            OntologyFormat::RdfXml => parse_rdf_xml(&serialized).expect("parse rdf/xml"),
            OntologyFormat::Turtle => parse_turtle(&serialized).expect("parse turtle"),
            OntologyFormat::Functional => parse_functional(&serialized).expect("parse functional"),
            OntologyFormat::OwlXml => parse_owl_xml(&serialized).expect("parse owl/xml"),
            _ => unreachable!(),
        };
        probes.push(run_probe(name, reloaded));
    }

    assert_probes_identical(&probes);
}

/// Manchester (hand-written, parsed directly) must agree with the four
/// serialised formats on the same CORE ontology.
#[test]
fn cross_format_reasoning_parity_manchester() {
    let mut parser = ManchesterParser::new(ManchesterParserConfig::default());
    let manchester_ont = parser
        .parse_string(manchester_source())
        .map_err(|e| format!("{e:?}"))
        .expect("parse manchester");

    // Sanity: the hierarchy / ABox axioms survived Manchester parsing.
    assert!(
        manchester_ont.axioms().iter().any(
            |ax| matches!(ax, Axiom::SubClassOf(sc) if sc.superclass.to_string().contains('C'))
        ),
        "Manchester parse must include B ⊑ C"
    );

    let manchester_probe = run_probe("manchester", manchester_ont);
    let functional_probe = run_probe(
        "functional",
        parse_functional(
            &save_to_string(&canonical_ontology(), OntologyFormat::Functional).expect("serialize"),
        )
        .expect("parse functional"),
    );

    assert_eq!(
        manchester_probe.consistent, functional_probe.consistent,
        "Manchester consistency differs from Functional: {} vs {}",
        manchester_probe.consistent, functional_probe.consistent
    );
    assert_eq!(
        manchester_probe.class_hierarchy, functional_probe.class_hierarchy,
        "Manchester class hierarchy differs from Functional:\n{:?}\nvs\n{:?}",
        manchester_probe.class_hierarchy, functional_probe.class_hierarchy
    );
    assert_eq!(
        manchester_probe.entailed, functional_probe.entailed,
        "Manchester entailment answers differ from Functional:\n{:?}\nvs\n{:?}",
        manchester_probe.entailed, functional_probe.entailed
    );
}

/// Negative control: the ontology IRI is non-logical, so two structurally
/// identical ontologies differing only in their IRI must give identical results.
#[test]
fn cross_format_reasoning_parity_ontology_iri_is_non_logical() {
    let df = DF::new();
    let a_cls = df.class_ce(format!("{EX}A"));
    let c_cls = df.class_ce(format!("{EX}C"));
    let a = df.named(format!("{EX}a"));

    let mut with_iri = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), c_cls.clone()),
        df.class_assertion(a_cls.clone(), a.clone()),
    ]);
    df.auto_declare(&mut with_iri);
    with_iri.set_iri(oxidowl::ontology::IRI::new("http://parity.org/ont"));

    let mut no_iri = df.build_ontology(vec![
        df.sub_class_of(a_cls.clone(), c_cls.clone()),
        df.class_assertion(a_cls, a),
    ]);
    df.auto_declare(&mut no_iri);
    no_iri.set_iri(oxidowl::ontology::IRI::new("urn:no-iri"));

    let p1 = run_probe("with-iri", with_iri);
    let p2 = run_probe("no-iri", no_iri);
    assert_eq!(p1.consistent, p2.consistent);
    assert_eq!(p1.class_hierarchy, p2.class_hierarchy);
    assert_eq!(p1.entailed, p2.entailed);
}

/// Pin the RBox/data-axiom round-trip capability of each format (documented
/// findings from `zz_probe_tests`). The EXTENDED ontology adds
/// `P ⊑ R`, `DataPropertyRange(dp, xsd:string)` and `dp(a, "x"^^xsd:string)`.
#[test]
fn rbox_data_axiom_roundtrip_format_capability() {
    let ont = extended_ontology();
    let _df = DF::new();

    fn roundtrip(ont: &Ontology, format: OntologyFormat) -> Ontology {
        let s = save_to_string(ont, format).expect("serialize");
        match format {
            OntologyFormat::RdfXml => parse_rdf_xml(&s).expect("parse"),
            OntologyFormat::Turtle => parse_turtle(&s).expect("parse"),
            OntologyFormat::OwlXml => parse_owl_xml(&s).expect("parse"),
            OntologyFormat::Functional => parse_functional(&s).expect("parse"),
            _ => unreachable!(),
        }
    }

    let has_sub_prop = |o: &Ontology| {
        o.axioms().iter().any(|ax| {
            matches!(ax, Axiom::SubObjectPropertyOf(sp)
                if sp.sub_property.to_string() == format!("{EX}P"))
        })
    };
    let has_data_assert = |o: &Ontology| {
        o.axioms().iter().any(|ax| {
            matches!(ax, Axiom::DataPropertyAssertion(dp) if dp.property.to_string() == format!("{EX}dp"))
        })
    };
    let has_data_range = |o: &Ontology| {
        o.axioms().iter().any(|ax| {
            matches!(ax, Axiom::DataPropertyRange(rg) if rg.property.to_string() == format!("{EX}dp"))
        })
    };

    let functional = roundtrip(&ont, OntologyFormat::Functional);
    assert!(
        has_sub_prop(&functional) && has_data_assert(&functional) && has_data_range(&functional),
        "Functional must preserve P ⊑ R, dp range and dp(a,\"x\")"
    );

    // OWL/XML keeps the sub-property but drops data axioms.
    let owlxml = roundtrip(&ont, OntologyFormat::OwlXml);
    assert!(has_sub_prop(&owlxml), "OWL/XML must preserve P ⊑ R");
    assert!(
        !has_data_assert(&owlxml) && !has_data_range(&owlxml),
        "OWL/XML currently drops DataPropertyAssertion/Range (documented gap)"
    );

    // Turtle keeps the (plain-literal) data assertion but drops sub-property + range.
    let turtle = roundtrip(&ont, OntologyFormat::Turtle);
    assert!(has_data_assert(&turtle), "Turtle must preserve dp(a,\"x\")");
    assert!(
        !has_sub_prop(&turtle) && !has_data_range(&turtle),
        "Turtle currently drops SubObjectPropertyOf/DataPropertyRange (documented gap)"
    );

    // RDF/XML routes triples through the *shared* RDF→OWL mapping (the
    // JSON-LD slice unified the three RDF syntaxes), so sub-property and
    // range axioms now survive; the plain data assertion is still dropped
    // (documented gap).  This block previously pinned the pre-unification
    // behaviour (sub/range misread into bogus object property assertions).
    let rdfxml = roundtrip(&ont, OntologyFormat::RdfXml);
    assert!(
        has_sub_prop(&rdfxml),
        "RDF/XML must preserve P ⊑ R through the shared mapping"
    );
    assert!(
        has_data_range(&rdfxml),
        "RDF/XML must preserve DataPropertyRange through the shared mapping"
    );
    assert!(
        !has_data_assert(&rdfxml),
        "RDF/XML currently drops DataPropertyAssertion (documented gap)"
    );
}

/// Pin the complex-class-expression round-trip capability. Only Functional
/// preserves `Card ≡ ≥1 R.{b}`; RDF/XML, Turtle and OWL/XML emit only entity
/// declarations, and Manchester cannot even express `{b}`.
#[test]
fn nominal_axiom_roundtrip_format_capability() {
    let df = DF::new();
    let card = df.class_ce(format!("{EX}Card"));
    let r = df.obj_prop(format!("{EX}R"));
    let b_ind = df.named(format!("{EX}b"));
    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![card, df.min_cardinality(1, r, df.one_of(vec![b_ind]))]),
    ]);
    df.auto_declare(&mut ont);

    fn roundtrips(ont: &Ontology, format: OntologyFormat) -> bool {
        let s = save_to_string(ont, format).expect("serialize");
        let reloaded = match format {
            OntologyFormat::RdfXml => parse_rdf_xml(&s).expect("parse"),
            OntologyFormat::Turtle => parse_turtle(&s).expect("parse"),
            OntologyFormat::OwlXml => parse_owl_xml(&s).expect("parse"),
            OntologyFormat::Functional => parse_functional(&s).expect("parse"),
            _ => unreachable!(),
        };
        reloaded.axioms().iter().any(|ax| {
            matches!(ax, Axiom::EquivalentClasses(eq)
                if eq.classes.iter().any(|c| matches!(c, ClassExpression::Class(cl) if cl.iri.as_str() == format!("{EX}Card"))))
        })
    }

    assert!(
        roundtrips(&ont, OntologyFormat::Functional),
        "Functional must round-trip Card ≡ ≥1 R.{{b}}"
    );
    for (name, format) in [
        ("rdfxml", OntologyFormat::RdfXml),
        ("turtle", OntologyFormat::Turtle),
        ("owlxml", OntologyFormat::OwlXml),
    ] {
        assert!(
            !roundtrips(&ont, format),
            "[{name}] currently drops EquivalentClasses with complex expressions; \
             if this assertion fires the serialiser has been fixed — move this \
             axiom back into the shared parity ontology"
        );
    }
}

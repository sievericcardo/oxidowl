//! `RDFa` 1.1 parser core tests (B5 Slice 4 — DOM-based `RDFa`, docs/PLAN.md).
//!
//! Locks `src/parsers/rio/rdfa.rs` (html5ever DOM + `RDFa` Core 1.1 sequence
//! §7.5 + shared RDF→OWL mapping) over exactly the implemented subset:
//!
//! 1. **Default prefixes** — `rdf` / `rdfs` / `xsd` / `owl` / `foaf` are in
//!    scope without any `@prefix` declaration.
//! 2. **`@prefix` / `@xmlns` / `@vocab`** — CURIE expansion through the
//!    innermost declaration (local prefix wins over `@vocab`), terms expand
//!    through `@vocab`, discovered prefixes land in `Ontology::prefixes`.
//! 3. **`@about` + `@typeof`** — class assertions, including
//!    `@typeof` + `@property` on the same element.
//! 4. **`@rel`** — object property assertions (with `@href` / `@resource`).
//! 5. **`@property`** — literals honouring `@content`, `@datatype` and the
//!    inherited `@lang`/`xml:lang`.
//! 6. **Nested `@about` / `@resource`** — subject inheritance down the tree.
//! 7. **`@rev`** — reversed predicate direction.
//! 8. **`@href` / `@src` base resolution** — against the supplied base and
//!    against a document `<base href>`.
//! 9. **Blank nodes** — `_:label` references stay stable; `@about="[]"` is
//!    a fresh anonymous node.
//! 10. **Preservation** — triples the shared mapping cannot convert
//!     (`owl:imports`, …) become `UNMAPPED_TRIPLE_PROPERTY` annotations
//!     instead of being dropped.
//! 11. **`@inlist`** (§7.5 + §8.4) — list-generating `@rel`/`@property`
//!     values emit RDF collections (`rdf:first`/`rdf:rest`→`rdf:nil`)
//!     instead of direct triples, for multiple values on one list and for
//!     lists completed by nested subjects.
//! 12. **Safe CURIEs** (§7.4) — `[prefix:local]` expands through the
//!     declared prefix; `[]` / unresolvable bracketed values become blank
//!     nodes (never the document base).
//! 13. **Initial context** (§9) — bare terms (`rel="license"`) expand
//!     through the embedded term map without any `@vocab`.
//! 14. **Vocabulary expansion** (§10) — opt-in, local, off by default.
//! 15. **Serializer round-trip** — `parse(serialize(o))` reproduces the
//!     core axioms, `@inlist` is used for RDF collections, and
//!     non-expressible constructs are reported in HTML comments.
//! 16. **Host-language gaps** — `<template>` contents are evaluated and
//!     `xml:base` overrides the base.

use oxidowl::ontology::ImportsDeclaration;
use oxidowl::ontology::axioms::{Axiom, Entity};
use oxidowl::ontology::{
    AnnotationValue, ClassExpression, DataPropertyExpression, Individual, Literal,
    ObjectPropertyExpression, Ontology,
};
use oxidowl::parsers::rio::rdfa::{self, RdfaRenderer, UNMAPPED_TRIPLE_PROPERTY};

#[allow(dead_code)]
#[path = "helpers/mod.rs"]
mod helpers;

use helpers::assertions::assert_ontologies_axiom_equal;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Parse an `RDFa` document, panicking with context on failure.
fn parse(html: &str) -> Ontology {
    rdfa::parse(html).unwrap_or_else(|e| panic!("RDFa parse failed: {e}\n---\n{html}"))
}

/// Parse an `RDFa` document against a supplied base IRI.
fn parse_with_base(html: &str, base: &str) -> Ontology {
    rdfa::parse_with_base(html, Some(base))
        .unwrap_or_else(|e| panic!("RDFa parse failed (base {base}): {e}\n---\n{html}"))
}

/// Stable key for an individual: its IRI, or `_:id` for a blank node.
fn individual_key(individual: &Individual) -> String {
    match individual {
        Individual::Named(named) => named.iri.to_string(),
        Individual::Anonymous(anon) => format!("_:{}", anon.id),
    }
}

/// All `(individual, class)` class assertions (named → IRI, blank → `_:id`).
fn class_assertions(ontology: &Ontology) -> Vec<(String, String)> {
    ontology
        .axioms()
        .iter()
        .filter_map(|axiom| match axiom {
            Axiom::ClassAssertion(a) => match &a.class {
                ClassExpression::Class(class) => {
                    Some((individual_key(&a.individual), class.iri.to_string()))
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// All `(source, property, target)` object property assertions.
fn object_assertions(ontology: &Ontology) -> Vec<(String, String, String)> {
    ontology
        .axioms()
        .iter()
        .filter_map(|axiom| match axiom {
            Axiom::ObjectPropertyAssertion(a) => match &a.property {
                ObjectPropertyExpression::ObjectProperty(prop) => Some((
                    individual_key(&a.source),
                    prop.iri.to_string(),
                    individual_key(&a.target),
                )),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// All `(individual, property, literal)` data property assertions.
fn data_assertions(ontology: &Ontology) -> Vec<(String, String, Literal)> {
    ontology
        .axioms()
        .iter()
        .filter_map(|axiom| match axiom {
            Axiom::DataPropertyAssertion(a) => match &a.property {
                DataPropertyExpression::DataProperty(prop) => Some((
                    individual_key(&a.individual),
                    prop.iri.to_string(),
                    a.value.clone(),
                )),
            },
            _ => None,
        })
        .collect()
}

/// Declaration IRIs by entity kind.
fn declarations(ontology: &Ontology) -> Vec<Entity> {
    ontology
        .axioms()
        .iter()
        .filter_map(|axiom| match axiom {
            Axiom::Declaration(d) => Some(d.entity.clone()),
            _ => None,
        })
        .collect()
}

/// Every preserved `UNMAPPED_TRIPLE_PROPERTY` annotation, as N-Triples text.
fn preserved_triples(ontology: &Ontology) -> Vec<String> {
    ontology
        .annotations
        .iter()
        .filter(|a| a.property.iri.as_str() == UNMAPPED_TRIPLE_PROPERTY)
        .filter_map(|a| match &a.value {
            AnnotationValue::Literal(lit) => Some(lit.value.clone()),
            _ => None,
        })
        .collect()
}

/// N-Triples text of the preserved `rdf:first` triples.
fn preserved_first(ontology: &Ontology) -> Vec<String> {
    preserved_triples(ontology)
        .into_iter()
        .filter(|t| t.contains("#first>"))
        .collect()
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. Default prefixes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn default_prefixes_expand_owl_rdfs_and_foaf() {
    let html = r#"<!DOCTYPE html>
<html>
<head><title>Default prefixes</title></head>
<body>
  <div about="http://example.org/alice" typeof="owl:NamedIndividual"></div>
  <div about="http://example.org/bob" typeof="rdfs:Resource"></div>
  <span about="http://example.org/alice" property="foaf:name" content="Alice"></span>
</body>
</html>"#;
    let o = parse(html);

    // owl:NamedIndividual → declaration of the individual
    assert!(
        declarations(&o)
            .iter()
            .any(|e| matches!(e, Entity::NamedIndividual(iri)
                if iri.as_str() == "http://example.org/alice")),
        "owl:NamedIndividual declaration missing, got {:?}",
        declarations(&o)
    );

    // rdfs:Resource → class assertion
    assert!(
        class_assertions(&o).contains(&(
            "http://example.org/bob".to_string(),
            "http://www.w3.org/2000/01/rdf-schema#Resource".to_string()
        )),
        "rdfs:Resource class assertion missing, got {:?}",
        class_assertions(&o)
    );

    // foaf:name → data property assertion
    assert!(
        data_assertions(&o)
            .iter()
            .any(|(s, p, lit)| s == "http://example.org/alice"
                && p == "http://xmlns.com/foaf/0.1/name"
                && lit.value == "Alice"),
        "foaf:name data assertion missing, got {:?}",
        data_assertions(&o)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. @prefix / @xmlns / @vocab
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn prefix_attribute_and_xmlns_declare_scoped_curies() {
    let html = r#"<div prefix="ex: http://example.org/vocab/" about="ex:item" typeof="ex:Thing">
  <span property="ex:name" content="Widget"></span>
</div>
<div xmlns:ex2="http://example2.org/" about="ex2:bob" typeof="ex2:Person"></div>"#;
    let o = parse(html);

    assert!(
        class_assertions(&o).contains(&(
            "http://example.org/vocab/item".to_string(),
            "http://example.org/vocab/Thing".to_string()
        )),
        "@prefix CURIEs missing: {:?}",
        class_assertions(&o)
    );
    assert!(
        data_assertions(&o)
            .iter()
            .any(|(s, p, lit)| s == "http://example.org/vocab/item"
                && p == "http://example.org/vocab/name"
                && lit.value == "Widget"),
        "@prefix property CURIE missing: {:?}",
        data_assertions(&o)
    );

    // @xmlns declarations work like @prefix (backwards compatibility).
    assert!(
        class_assertions(&o).contains(&(
            "http://example2.org/bob".to_string(),
            "http://example2.org/Person".to_string()
        )),
        "@xmlns CURIEs missing: {:?}",
        class_assertions(&o)
    );

    // Declared prefixes are recorded on the ontology.
    assert_eq!(
        o.prefixes.get("ex").map(oxidowl::IRI::as_str),
        Some("http://example.org/vocab/"),
        "discovered @prefix must land in Ontology::prefixes"
    );
    assert_eq!(
        o.prefixes.get("ex2").map(oxidowl::IRI::as_str),
        Some("http://example2.org/"),
        "discovered @xmlns prefix must land in Ontology::prefixes"
    );
}

#[test]
fn vocab_sets_terms_and_local_prefix_wins_over_vocab() {
    let html = r#"<div vocab="http://example.org/vocab/" prefix="ex: http://example.org/p/" about="http://example.org/item">
  <span property="name" content="Widget"></span>
  <span property="ex:name" content="Other"></span>
</div>"#;
    let o = parse(html);

    // Bare term expands through @vocab.
    assert!(
        data_assertions(&o)
            .iter()
            .any(|(s, p, lit)| s == "http://example.org/item"
                && p == "http://example.org/vocab/name"
                && lit.value == "Widget"),
        "@vocab term expansion missing: {:?}",
        data_assertions(&o)
    );
    // CURIE expands through the local @prefix — prefix beats @vocab.
    assert!(
        data_assertions(&o)
            .iter()
            .any(|(s, p, lit)| s == "http://example.org/item"
                && p == "http://example.org/p/name"
                && lit.value == "Other"),
        "@prefix must win over @vocab for CURIEs: {:?}",
        data_assertions(&o)
    );
    // The vocabulary itself is recorded with rdfa:usesVocabulary.
    assert!(
        o.axioms().iter().any(|a| match a {
            Axiom::ObjectPropertyAssertion(x) => matches!(
                &x.property,
                ObjectPropertyExpression::ObjectProperty(p)
                    if p.iri.as_str() == "http://www.w3.org/ns/rdfa#usesVocabulary"
            ),
            _ => false,
        }),
        "rdfa:usesVocabulary triple missing"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. @about + @typeof
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn about_and_typeof_produce_class_assertions() {
    let html = r#"<div about="http://example.org/john" typeof="http://example.org/Person"></div>
<div about="http://example.org/john" typeof="http://example.org/Employee"
     property="http://example.org/holdsRole" content="Engineer"></div>
<div about="http://example.org/john" typeof="http://example.org/Alumnus"
     property="http://example.org/studiedAt">MIT</div>"#;
    let o = parse(html);

    let assertions = class_assertions(&o);
    assert!(
        assertions.contains(&(
            "http://example.org/john".to_string(),
            "http://example.org/Person".to_string()
        )),
        "plain @about+@typeof assertion missing: {assertions:?}"
    );
    assert!(
        assertions.contains(&(
            "http://example.org/john".to_string(),
            "http://example.org/Employee".to_string()
        )),
        "@typeof+@property assertion missing: {assertions:?}"
    );
    assert!(
        assertions.contains(&(
            "http://example.org/john".to_string(),
            "http://example.org/Alumnus".to_string()
        )),
        "@typeof+@property (text content) assertion missing: {assertions:?}"
    );

    // @typeof + @property on the same element also asserts the property.
    let data = data_assertions(&o);
    assert!(
        data.iter().any(|(s, p, lit)| s == "http://example.org/john"
            && p == "http://example.org/holdsRole"
            && lit.value == "Engineer"),
        "@property next to @typeof (content) missing: {data:?}"
    );
    assert!(
        data.iter().any(|(s, p, lit)| s == "http://example.org/john"
            && p == "http://example.org/studiedAt"
            && lit.value == "MIT"),
        "@property next to @typeof (text) missing: {data:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. @rel
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rel_creates_object_property_assertions() {
    let html = r#"<div about="http://example.org/a" rel="http://example.org/knows" href="http://example.org/b"></div>
<div about="http://example.org/a" rel="foaf:knows" resource="http://example.org/c"></div>"#;
    let o = parse(html);

    let assertions = object_assertions(&o);
    assert!(
        assertions.contains(&(
            "http://example.org/a".to_string(),
            "http://example.org/knows".to_string(),
            "http://example.org/b".to_string()
        )),
        "@rel+@href assertion missing: {assertions:?}"
    );
    assert!(
        assertions.contains(&(
            "http://example.org/a".to_string(),
            "http://xmlns.com/foaf/0.1/knows".to_string(),
            "http://example.org/c".to_string()
        )),
        "@rel+@resource assertion missing: {assertions:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. @property with @content / @datatype / @lang
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn property_honours_content_datatype_and_lang() {
    let html = r#"<span about="http://example.org/a" property="http://example.org/nick" content="Johnny"></span>
<span about="http://example.org/a" property="http://example.org/age" datatype="xsd:integer">42</span>
<div about="http://example.org/a" lang="de">
  <span property="http://example.org/label" content="Wert"></span>
</div>"#;
    let o = parse(html);

    let data = data_assertions(&o);

    // @content → plain literal
    assert!(
        data.iter().any(|(s, p, lit)| s == "http://example.org/a"
            && p == "http://example.org/nick"
            && lit.value == "Johnny"
            && lit.language.is_none()
            && lit.datatype.is_none()),
        "@content literal missing: {data:?}"
    );

    // @datatype → typed literal (xsd: is a default prefix), value from the
    // descendant text
    assert!(
        data.iter().any(|(s, p, lit)| s == "http://example.org/a"
            && p == "http://example.org/age"
            && lit.value == "42"
            && lit
                .datatype
                .as_ref()
                .is_some_and(|d| d.as_str() == "http://www.w3.org/2001/XMLSchema#integer")),
        "@datatype typed literal missing: {data:?}"
    );

    // inherited language tag → language-tagged literal
    assert!(
        data.iter().any(|(s, p, lit)| s == "http://example.org/a"
            && p == "http://example.org/label"
            && lit.value == "Wert"
            && lit.language.as_deref() == Some("de")),
        "language-tagged literal missing: {data:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. Nested @about / @resource subject inheritance
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn nested_about_and_resource_inherit_the_subject() {
    let html = r#"<div about="http://example.org/doc">
  <span property="http://example.org/title" content="Doc title"></span>
  <div resource="http://example.org/photo">
    <span property="http://example.org/caption" content="A photo"></span>
  </div>
</div>"#;
    let o = parse(html);

    let data = data_assertions(&o);
    assert!(
        data.iter().any(|(s, p, lit)| s == "http://example.org/doc"
            && p == "http://example.org/title"
            && lit.value == "Doc title"),
        "nested property must use the inherited @about subject: {data:?}"
    );
    assert!(
        data.iter()
            .any(|(s, p, lit)| s == "http://example.org/photo"
                && p == "http://example.org/caption"
                && lit.value == "A photo"),
        "nested property must switch to the inner @resource subject: {data:?}"
    );
    // Exactly one caption — it must NOT also leak onto the outer subject.
    let captions: Vec<_> = data
        .iter()
        .filter(|(_, p, _)| p == "http://example.org/caption")
        .collect();
    assert_eq!(
        captions.len(),
        1,
        "caption asserted more than once: {data:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. @rev
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn rev_reverses_the_predicate_direction() {
    let html = r#"<div about="http://example.org/a" rev="http://example.org/knows" resource="http://example.org/b"></div>"#;
    let o = parse(html);

    let assertions = object_assertions(&o);
    // @rev: the object (`b`) is the *subject* of the triple.
    assert_eq!(
        assertions,
        vec![(
            "http://example.org/b".to_string(),
            "http://example.org/knows".to_string(),
            "http://example.org/a".to_string()
        )],
        "@rev must emit (object, predicate, subject)"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. @href / @src base resolution
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn href_and_src_resolve_against_the_supplied_base() {
    let html = r#"<div about="http://example.org/doc" rel="http://example.org/license" href="licenses/mit.html"></div>
<div about="http://example.org/doc" rel="http://example.org/depicts" src="images/cat.png"></div>"#;
    let o = parse_with_base(html, "http://example.org/docs/page.html");

    let assertions = object_assertions(&o);
    assert!(
        assertions.contains(&(
            "http://example.org/doc".to_string(),
            "http://example.org/license".to_string(),
            "http://example.org/docs/licenses/mit.html".to_string()
        )),
        "@href must resolve against the supplied base: {assertions:?}"
    );
    assert!(
        assertions.contains(&(
            "http://example.org/doc".to_string(),
            "http://example.org/depicts".to_string(),
            "http://example.org/docs/images/cat.png".to_string()
        )),
        "@src must resolve against the supplied base: {assertions:?}"
    );
}

#[test]
fn document_base_element_overrides_the_supplied_base() {
    let html = r#"<html>
<head><base href="http://example.org/base/"></head>
<body>
  <div about="http://example.org/doc" rel="http://example.org/x" href="rel.html"></div>
</body>
</html>"#;
    let o = parse_with_base(html, "http://other.example/fallback/");

    let assertions = object_assertions(&o);
    assert_eq!(
        assertions,
        vec![(
            "http://example.org/doc".to_string(),
            "http://example.org/x".to_string(),
            "http://example.org/base/rel.html".to_string()
        )],
        "the document <base href> must win over the supplied base"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 9. Blank nodes
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn anonymous_subjects_and_objects_are_blank_nodes() {
    let html = r#"<div about="_:alice" rel="http://example.org/knows" resource="_:bob"></div>
<div about="_:bob" rel="http://example.org/worksFor" href="http://example.org/acme"></div>
<div about="[]" typeof="http://example.org/Anon"></div>"#;
    let o = parse(html);

    let assertions = object_assertions(&o);
    let knows = assertions
        .iter()
        .find(|(_, p, _)| p == "http://example.org/knows")
        .expect("knows assertion missing");
    assert_eq!(knows.0, "_:alice", "explicit _:alice reference");
    assert_eq!(knows.2, "_:bob", "explicit _:bob reference");

    let works = assertions
        .iter()
        .find(|(_, p, _)| p == "http://example.org/worksFor")
        .expect("worksFor assertion missing");
    assert_eq!(
        works.0, "_:bob",
        "a second _:bob reference must hit the same blank node"
    );
    assert_eq!(works.2, "http://example.org/acme");

    // `@about="[]"` → a fresh anonymous individual carrying the type.
    assert!(
        class_assertions(&o)
            .iter()
            .any(|(s, c)| s.starts_with("_:rdfa_gen_") && c == "http://example.org/Anon"),
        "@about=\"[]\" must produce a fresh blank node: {:?}",
        class_assertions(&o)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 10. Preservation of unmappable triples
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn unmappable_triples_are_preserved_as_annotations() {
    let html = r#"<html>
<head>
  <link rel="http://www.w3.org/2002/07/owl#imports" href="http://example.org/other.owl"/>
</head>
<body></body>
</html>"#;
    let o = parse(html);

    // owl:imports is a no-op in the shared mapping — it must be preserved,
    // not silently dropped, and must not leak into a property assertion.
    let preserved: Vec<&AnnotationValue> = o
        .annotations
        .iter()
        .filter(|a| a.property.iri.as_str() == UNMAPPED_TRIPLE_PROPERTY)
        .map(|a| &a.value)
        .collect();
    assert_eq!(
        preserved.len(),
        1,
        "expected exactly one preserved triple, got {:?}",
        o.annotations
    );
    match preserved[0] {
        AnnotationValue::Literal(lit) => {
            assert!(
                lit.value.contains("http://example.org/other.owl"),
                "preserved value must carry the triple verbatim: {}",
                lit.value
            );
            assert!(
                lit.value.contains("http://www.w3.org/2002/07/owl#imports"),
                "preserved value must carry the predicate verbatim: {}",
                lit.value
            );
        }
        other => panic!("preserved value must be a literal, got {other:?}"),
    }
    assert!(
        o.axioms().is_empty(),
        "owl:imports must not turn into a property assertion: {:?}",
        o.axioms()
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 11. @inlist — RDF collections (§7.5 step 9/10 + §8.4)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn inlist_property_collects_values_into_an_rdf_collection() {
    // RDFa Core 1.1 §8.4 "Example 124": three co-authors, one list.
    let html = r#"<div prefix="ex: http://example.org/vocab/" about="http://example.org/doc">
  <span inlist="" property="ex:creator" resource="http://example.org/ben">Ben</span>
  <span inlist="" property="ex:creator">Mark</span>
  <span inlist="" property="ex:creator" resource="http://example.org/ivan">Ivan</span>
</div>"#;
    let o = parse(html);

    // Exactly one direct assertion, pointing at the collection head — the
    // values themselves are NOT asserted as direct triples.
    let creators: Vec<_> = object_assertions(&o)
        .into_iter()
        .filter(|(_, p, _)| p == "http://example.org/vocab/creator")
        .collect();
    assert_eq!(
        creators.len(),
        1,
        "expected a single direct creator triple (to the list head): {creators:?}"
    );
    assert!(
        creators[0].2.starts_with("_:"),
        "the direct triple must target the collection head, got {}",
        creators[0].2
    );

    // The collection itself: rdf:first/rdf:rest chain ending in rdf:nil.
    let first = preserved_first(&o);
    assert!(
        first.iter().any(|t| t.contains("<http://example.org/ben>")),
        "rdf:first must list the IRI item: {first:?}"
    );
    assert!(
        first.iter().any(|t| t.contains("\"Mark\"")),
        "rdf:first must list the literal item: {first:?}"
    );
    assert!(
        first
            .iter()
            .any(|t| t.contains("<http://example.org/ivan>")),
        "rdf:first must list the third item: {first:?}"
    );
    let preserved = preserved_triples(&o);
    assert_eq!(
        first.len(),
        3,
        "three items ⇒ three rdf:first triples, got {first:?}"
    );
    assert!(
        preserved
            .iter()
            .filter(|t| t.contains("#rest>"))
            .any(|t| t.contains("#nil>")),
        "the last rdf:rest must point at rdf:nil: {preserved:?}"
    );
    assert_eq!(
        preserved.iter().filter(|t| t.contains("#rest>")).count(),
        3,
        "three cells ⇒ three rdf:rest triples: {preserved:?}"
    );
}

#[test]
fn inlist_rel_completes_through_nested_subjects() {
    // RDFa Core 1.1 §8.4 "Example 127": @rel + @inlist with no object on the
    // element — the subjects established by the descendants fill the list.
    let html = r#"<div prefix="ex: http://example.org/vocab/" about="http://example.org/doc"
     rel="ex:authors" inlist="">
  <span about="http://example.org/alice"></span>
  <span about="http://example.org/bob"></span>
</div>"#;
    let o = parse(html);

    let authors: Vec<_> = object_assertions(&o)
        .into_iter()
        .filter(|(_, p, _)| p == "http://example.org/vocab/authors")
        .collect();
    assert_eq!(
        authors,
        vec![(
            "http://example.org/doc".to_string(),
            "http://example.org/vocab/authors".to_string(),
            creators_head(&o),
        )],
        "one direct triple to the collection head: {authors:?}"
    );

    let first = preserved_first(&o);
    assert!(
        first
            .iter()
            .any(|t| t.contains("<http://example.org/alice>"))
            && first.iter().any(|t| t.contains("<http://example.org/bob>")),
        "nested subjects must be appended to the list: {first:?}"
    );
    assert!(
        preserved_triples(&o)
            .iter()
            .any(|t| t.contains("#nil>") && t.contains("#rest>")),
        "the list must terminate in rdf:nil"
    );
}

/// The (single) blank target of the first object assertion with `predicate`.
fn creators_head(ontology: &Ontology) -> String {
    object_assertions(ontology)
        .into_iter()
        .map(|(_, _, target)| target)
        .find(|target| target.starts_with("_:"))
        .expect("collection head blank node")
}

// ═══════════════════════════════════════════════════════════════════════════
// 12. Safe CURIEs (§7.4)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn safe_curie_expands_through_the_declared_prefix() {
    let html = r#"<div prefix="ex: http://example.org/vocab/" about="[ex:item]" typeof="ex:Thing">
  <span property="ex:name" content="Widget"></span>
</div>
<div prefix="ex: http://example.org/vocab/" about="[ex:other]" typeof="ex:Other"></div>"#;
    let o = parse(html);

    let assertions = class_assertions(&o);
    assert!(
        assertions.contains(&(
            "http://example.org/vocab/item".to_string(),
            "http://example.org/vocab/Thing".to_string()
        )),
        "[prefix:local] must expand through the prefix: {assertions:?}"
    );
    assert!(
        assertions.contains(&(
            "http://example.org/vocab/other".to_string(),
            "http://example.org/vocab/Other".to_string()
        )),
        "a second safe CURIE must expand too: {assertions:?}"
    );
    // A safe CURIE is never an IRI of its own and never the document base.
    assert!(
        !assertions
            .iter()
            .any(|(subject, _)| subject.starts_with("http://example.invalid")),
        "the bracketed value must not fall back to the document base: {assertions:?}"
    );
    assert!(
        !assertions
            .iter()
            .any(|(subject, _)| subject.starts_with("_:")),
        "a resolvable safe CURIE is an IRI, not a blank node: {assertions:?}"
    );
    assert!(
        data_assertions(&o)
            .iter()
            .any(|(s, p, lit)| s == "http://example.org/vocab/item"
                && p == "http://example.org/vocab/name"
                && lit.value == "Widget"),
        "@property on a safe-CURIE subject must target the resolved IRI"
    );
}

#[test]
fn empty_and_unresolvable_safe_curies_are_recorded_blank_nodes() {
    let html = r#"<div prefix="ex: http://example.org/vocab/" about="[]" typeof="http://example.org/Anon"></div>
<div about="[unknown:thing]" typeof="http://example.org/Unresolved"></div>
<div about="[not-a-curie]" typeof="http://example.org/NotACurie"></div>
<div about="[]" typeof="http://example.org/AnonAgain"></div>"#;
    let o = parse(html);

    let assertions = class_assertions(&o);
    let anon: Vec<&String> = assertions
        .iter()
        .filter(|(_, c)| c.ends_with("Anon") || c.ends_with("AnonAgain"))
        .map(|(s, _)| s)
        .collect();
    assert_eq!(
        anon.len(),
        2,
        "`[]` must be recorded and reused for both elements: {assertions:?}"
    );
    assert_eq!(
        anon[0], anon[1],
        "the same safe CURIE must denote the same blank node"
    );
    assert!(anon[0].starts_with("_:"), "got {}", anon[0]);

    for expected in [
        "http://example.org/Unresolved",
        "http://example.org/NotACurie",
    ] {
        assert!(
            assertions
                .iter()
                .any(|(s, c)| c == expected && s.starts_with("_:")),
            "{expected} must be typed on a blank node, got {assertions:?}"
        );
    }
    assert!(
        !assertions
            .iter()
            .any(|(subject, _)| subject == "http://example.invalid/"),
        "no bracketed value may resolve to the document base: {assertions:?}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 13. Initial context term map (§9 / §7.4.3)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn initial_context_terms_expand_without_a_vocab() {
    let html = r#"<!DOCTYPE html>
<html>
<head>
  <base href="http://example.org/docs/"/>
  <link rel="license" href="http://example.org/licenses/mit.html"/>
  <link rel="canonical" href="http://example.org/page.html"/>
  <link rel="License" href="http://example.org/licenses/bsd.html"/>
  <link rel="search" href="http://example.org/opensearch.xml"/>
</head>
<body></body>
</html>"#;
    let o = parse(html);
    let assertions = object_assertions(&o);
    let base = "http://example.org/docs/";

    // `license` — RDFa Core initial context → xhv:license (§7.4.4 Example 41).
    assert!(
        assertions.contains(&(
            base.to_string(),
            "http://www.w3.org/1999/xhtml/vocab#license".to_string(),
            "http://example.org/licenses/mit.html".to_string()
        )),
        "rel=\"license\" must expand through the initial term map: {assertions:?}"
    );
    // Link-relation terms → the `rel` prefix (IANA link relations).
    for (term, target) in [
        ("canonical", "http://example.org/page.html"),
        ("search", "http://example.org/opensearch.xml"),
    ] {
        assert!(
            assertions.contains(&(
                base.to_string(),
                format!("http://www.iana.org/assignments/relation/{term}"),
                target.to_string()
            )),
            "rel=\"{term}\" must expand through the initial term map: {assertions:?}"
        );
    }
    // §7.4.3: case-insensitive fallback after the case-sensitive lookup.
    assert!(
        assertions.contains(&(
            base.to_string(),
            "http://www.w3.org/1999/xhtml/vocab#license".to_string(),
            "http://example.org/licenses/bsd.html".to_string()
        )),
        "term lookup falls back to a case-insensitive match: {assertions:?}"
    );
}

#[test]
fn initial_prefixes_include_void_and_xhv_without_declaration() {
    let html = r#"<div about="http://example.org/set" typeof="void:Dataset"></div>
<div about="http://example.org/page" rel="xhv:license" href="http://example.org/lic"></div>"#;
    let o = parse(html);

    assert!(
        class_assertions(&o).contains(&(
            "http://example.org/set".to_string(),
            "http://rdfs.org/ns/void#Dataset".to_string()
        )),
        "void must be an initial prefix: {:?}",
        class_assertions(&o)
    );
    assert!(
        object_assertions(&o).contains(&(
            "http://example.org/page".to_string(),
            "http://www.w3.org/1999/xhtml/vocab#license".to_string(),
            "http://example.org/lic".to_string()
        )),
        "xhv must be an initial prefix: {:?}",
        object_assertions(&o)
    );
    // Initial-context prefixes are not the author's declarations.
    assert!(
        !o.prefixes.contains_key("void"),
        "initial prefixes must not land in Ontology::prefixes"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 14. RDFa Vocabulary Expansion (§10)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn vocabulary_expansion_is_opt_in_and_local() {
    // §10 Example 130, with the vocabulary fact declared inline so no
    // dereferencing is needed.
    let html = r#"<div prefix="cc: http://creativecommons.org/ns# dc: http://purl.org/dc/terms/">
  <span about="http://creativecommons.org/ns#license"
        property="rdfs:subPropertyOf"
        resource="http://purl.org/dc/terms/license"></span>
  <div vocab="http://creativecommons.org/ns#" about="http://example.org/doc">
    <a rel="license" href="http://creativecommons.org/licenses/by/3.0/">CC BY</a>
  </div>
</div>"#;

    let plain = rdfa::RdfaParser::new().parse(html).expect("plain parse");
    let doc = "http://example.org/doc";
    let cc_license = "http://creativecommons.org/ns#license";
    let dc_license = "http://purl.org/dc/terms/license";
    let cc_by = "http://creativecommons.org/licenses/by/3.0/";

    assert!(
        object_assertions(&plain)
            .iter()
            .any(|(s, p, t)| s == doc && p == cc_license && t == cc_by),
        "@vocab must expand rel=\"license\" to cc:license"
    );
    assert!(
        !object_assertions(&plain)
            .iter()
            .any(|(_, p, _)| p == dc_license),
        "§10.2: expansion must be OFF by default"
    );

    let expanded = rdfa::RdfaParser::new()
        .with_vocabulary_expansion(true)
        .parse(html)
        .expect("expanded parse");
    assert!(
        object_assertions(&expanded)
            .iter()
            .any(|(s, p, t)| s == doc && p == dc_license && t == cc_by),
        "prp-spo1 must derive dc:license from the inline vocabulary fact: {:?}",
        object_assertions(&expanded)
    );
    // Vocabulary-graph-only triples are not copied into the output.
    assert!(
        !class_assertions(&expanded)
            .iter()
            .any(|(_, c)| c == "http://www.w3.org/2000/01/rdf-schema#Property"),
        "schema-only triples must not leak into the output"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 15. Serializer — round trip, @inlist, unsupported constructs
// ═══════════════════════════════════════════════════════════════════════════

/// A document covering the supported construct set: declarations, a
/// subclass, an equivalence over an RDF collection, class/object/data
/// assertions (plain, typed, language-tagged), an annotation assertion and
/// a nested subject.
fn round_trip_source() -> &'static str {
    r#"<!DOCTYPE html>
<html>
<head><title>Round trip</title></head>
<body>
  <div prefix="ex: http://example.org/vocab/" about="http://example.org/A" typeof="owl:Class">
    <span property="rdfs:label" content="Class A"></span>
  </div>
  <div prefix="ex: http://example.org/vocab/" about="http://example.org/B" typeof="owl:Class"></div>
  <div prefix="ex: http://example.org/vocab/" about="http://example.org/C" typeof="owl:Class"></div>
  <div about="http://example.org/A" property="rdfs:subClassOf" resource="http://example.org/B"></div>
  <div about="http://example.org/i" typeof="http://example.org/A owl:NamedIndividual">
    <span property="http://example.org/age" datatype="xsd:integer" content="42"></span>
    <span property="http://example.org/nick" content="Ada"></span>
    <span lang="fr" property="http://example.org/nom" content="Ada"></span>
    <a rel="http://example.org/parent" href="http://example.org/j"></a>
    <div about="http://example.org/j" typeof="http://example.org/Person"></div>
  </div>
  <div about="http://example.org/annTarget" typeof="owl:Class">
    <span property="http://example.org/note" content="annotated"></span>
  </div>
  <div about="http://example.org/A" rel="owl:equivalentClass" resource="_:c"></div>
  <div about="_:c" rel="owl:intersectionOf" inlist="">
    <span about="http://example.org/B"></span>
    <span about="http://example.org/C"></span>
  </div>
</body>
</html>"#
}

#[test]
fn serializer_round_trip_reproduces_core_axioms() {
    let source = round_trip_source();
    let original = parse(source);
    assert!(
        !original.axioms().is_empty(),
        "the source document must produce axioms"
    );

    let html = RdfaRenderer::new()
        .serialize(&original)
        .expect("serialization");
    assert!(html.starts_with("<!DOCTYPE html>"), "HTML5 document");

    let reparsed = rdfa::parse(&html).expect("re-parse of the serialized document");
    assert_ontologies_axiom_equal(&original, &reparsed);
}

#[test]
fn serializer_emits_the_documented_rdfa_vocabulary_and_inlist() {
    let original = parse(round_trip_source());
    let html = RdfaRenderer::new()
        .serialize(&original)
        .expect("serialization");

    for needle in [
        // @prefix / @vocab on the root
        " vocab=\"\"",
        " prefix=\"",
        // @about / @typeof on the subject block
        " about=\"http://example.org/A\"",
        " typeof=\"http://www.w3.org/2002/07/owl#Class\"",
        // @property for literals, @rel for resource objects
        " property=\"http://www.w3.org/2000/01/rdf-schema#label\"",
        " rel=\"http://example.org/parent\"",
        // @inlist for the RDF collection behind owl:intersectionOf
        " inlist=\"\"",
        // language tag and datatype survive
        " lang=\"fr\"",
        " datatype=\"http://www.w3.org/2001/XMLSchema#integer\"",
    ] {
        assert!(
            html.contains(needle),
            "output must contain {needle:?}:\n{html}"
        );
    }

    // The collection round-trips structurally: re-parsing still yields the
    // EquivalentClasses axiom over the intersection.
    let reparsed = rdfa::parse(&html).expect("re-parse");
    assert!(
        reparsed
            .axioms()
            .iter()
            .any(|a| matches!(a, Axiom::EquivalentClasses(_))),
        "the @inlist collection must reconstruct the intersection"
    );
    assert!(
        preserved_first(&reparsed).is_empty(),
        "cells of the consumed owl:intersectionOf list must not leak into annotations: {:?}",
        preserved_triples(&reparsed)
    );
}

#[test]
fn serializer_reports_constructs_it_cannot_express() {
    let mut o = Ontology::new();
    o.imports.push(ImportsDeclaration {
        imported_ontology_iri: oxidowl::ontology::IRI::new("http://example.org/other"),
    });
    let html = RdfaRenderer::new().serialize(&o).expect("serialization");

    assert!(
        html.contains("<!-- unsupported import declaration: <http://example.org/other> -->"),
        "imports must be visible in the output, not dropped:\n{html}"
    );
    // …and the output is still a parseable document.
    assert!(rdfa::parse(&html).is_ok());
}

// ═══════════════════════════════════════════════════════════════════════════
// 16. Host-language gaps closed: <template> contents and xml:base
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn template_contents_are_evaluated() {
    let html = r#"<div about="http://example.org/doc">
  <template>
    <span about="http://example.org/inTemplate" typeof="http://example.org/TT"></span>
  </template>
</div>"#;
    let o = parse(html);
    assert!(
        class_assertions(&o).contains(&(
            "http://example.org/inTemplate".to_string(),
            "http://example.org/TT".to_string()
        )),
        "<template> contents must be evaluated: {:?}",
        class_assertions(&o)
    );
}

#[test]
fn xml_base_overrides_the_base_for_the_element_and_its_children() {
    let html = r#"<div xml:base="http://example.org/sub/" about="rel.html" typeof="http://example.org/T">
  <span property="http://example.org/self" resource="inner.html"></span>
</div>"#;
    let o = parse(html);

    assert!(
        class_assertions(&o).contains(&(
            "http://example.org/sub/rel.html".to_string(),
            "http://example.org/T".to_string()
        )),
        "xml:base must apply to the element itself: {:?}",
        class_assertions(&o)
    );
    assert!(
        object_assertions(&o).contains(&(
            "http://example.org/sub/rel.html".to_string(),
            "http://example.org/self".to_string(),
            "http://example.org/sub/inner.html".to_string()
        )),
        "xml:base must apply to descendants: {:?}",
        object_assertions(&o)
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 17. Full RDFa 1.1 example block (hand-authored, no network)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn full_rdfa_11_example_block() {
    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <title>RDFa 1.1 example</title>
  <base href="http://example.org/docs/"/>
  <link rel="license" href="http://example.org/licenses/cc-by"/>
</head>
<body>
  <div prefix="ex: http://example.org/vocab/" vocab="http://example.org/vocab#"
       about="ex:report" typeof="ex:Report dc:Thing">
    <h1 property="dc:title">Quarterly report</h1>
    <p property="ex:summary" lang="de" content="Zusammenfassung"></p>
    <span property="ex:pages" datatype="xsd:integer">12</span>
    <div about="ex:author" typeof="foaf:Person">
      <span property="foaf:name">Ada</span>
      <a rel="foaf:homepage" href="http://example.org/ada">home</a>
      <span rev="ex:knows" resource="ex:editor"></span>
    </div>
    <ul>
      <li inlist="" property="ex:tag" resource="ex:alpha">alpha</li>
      <li inlist="" property="ex:tag" resource="ex:beta">beta</li>
    </ul>
    <span about="[ex:extra]" property="ex:note" content="bracketed"></span>
    <div about="ex:A" property="rdfs:subClassOf" resource="ex:B"></div>
  </div>
</body>
</html>"#;
    let o = parse(html);

    let report = "http://example.org/vocab/report";
    let author = "http://example.org/vocab/author";

    // @typeof (two CURIEs) → class assertions
    let types = class_assertions(&o);
    for class in [
        "http://example.org/vocab/Report",
        "http://purl.org/dc/terms/Thing",
    ] {
        assert!(
            types.contains(&(report.to_string(), class.to_string())),
            "missing {class}: {types:?}"
        );
    }
    assert!(
        types.contains(&(
            author.to_string(),
            "http://xmlns.com/foaf/0.1/Person".to_string()
        )),
        "nested @typeof: {types:?}"
    );

    // @property with text, @content+@lang, @datatype
    let data = data_assertions(&o);
    assert!(data.iter().any(|(s, p, lit)| s == report
        && p == "http://purl.org/dc/terms/title"
        && lit.value == "Quarterly report"));
    assert!(data.iter().any(|(s, p, lit)| s == report
        && p == "http://example.org/vocab/summary"
        && lit.value == "Zusammenfassung"
        && lit.language.as_deref() == Some("de")));
    assert!(data.iter().any(|(s, p, lit)| {
        s == report
            && p == "http://example.org/vocab/pages"
            && lit.value == "12"
            && lit
                .datatype
                .as_ref()
                .is_some_and(|d| d.as_str() == "http://www.w3.org/2001/XMLSchema#integer")
    }));
    assert!(data.iter().any(|(s, p, lit)| s == author
        && p == "http://xmlns.com/foaf/0.1/name"
        && lit.value == "Ada"));

    // @rel and @rev
    let objects = object_assertions(&o);
    assert!(objects.contains(&(
        author.to_string(),
        "http://xmlns.com/foaf/0.1/homepage".to_string(),
        "http://example.org/ada".to_string()
    )));
    assert!(objects.contains(&(
        "http://example.org/vocab/editor".to_string(),
        "http://example.org/vocab/knows".to_string(),
        author.to_string()
    )));

    // Initial-context term + document base from <base href>
    assert!(objects.contains(&(
        "http://example.org/docs/".to_string(),
        "http://www.w3.org/1999/xhtml/vocab#license".to_string(),
        "http://example.org/licenses/cc-by".to_string()
    )));

    // @inlist → collection, one direct triple to its head
    let tags: Vec<_> = objects
        .iter()
        .filter(|(_, p, _)| p == "http://example.org/vocab/tag")
        .cloned()
        .collect();
    assert_eq!(tags.len(), 1, "one direct tag triple to the head: {tags:?}");
    assert!(tags[0].2.starts_with("_:"));
    let first = preserved_first(&o);
    assert!(
        first
            .iter()
            .any(|t| t.contains("<http://example.org/vocab/alpha>"))
    );
    assert!(
        first
            .iter()
            .any(|t| t.contains("<http://example.org/vocab/beta>"))
    );

    // Safe CURIE subject + rdfs:subClassOf through @property/@resource
    assert!(
        data.iter()
            .any(|(s, p, lit)| s == "http://example.org/vocab/extra"
                && p == "http://example.org/vocab/note"
                && lit.value == "bracketed")
    );
    assert!(
        o.axioms().iter().any(|a| matches!(a, Axiom::SubClassOf(_))),
        "rdfs:subClassOf via @property/@resource must map"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 17. Input guards (CR-011 B1 / M4): depth + byte budget — Err, never abort
// ─────────────────────────────────────────────────────────────────────────────

/// CR-011 B1 (QA F-1): a ~5 000-deep document used to overflow the stack of
/// the uncapped DOM walks and **abort the process** (SIGABRT, uncatchable).
/// The pre-parse depth estimate now rejects it with a plain `Err` before
/// html5ever even runs.  The test passing at all is the no-abort proof —
/// an abort would kill the whole test binary.
#[test]
fn deeply_nested_document_returns_error_instead_of_aborting() {
    const DEPTH: usize = 5000;
    let mut html = String::new();
    for _ in 0..DEPTH {
        html.push_str("<div>");
    }
    html.push_str("deep");
    for _ in 0..DEPTH {
        html.push_str("</div>");
    }

    let err = rdfa::parse(&html)
        .expect_err("a 5000-deep document must be rejected with Err, not an abort");
    let msg = err.to_string();
    assert!(
        msg.contains("depth") && msg.contains("pre-parse limit"),
        "the error must name the nesting-depth guard, got: {msg}"
    );
}

/// Documents whose *estimated* source depth stays under the pre-parse cap
/// (auto-close-prone inputs) are still checked against `MAX_DEPTH` on the
/// parsed DOM — computed with an explicit stack, before any recursive walk
/// touches the tree (CR-011 B1/M4).
#[test]
fn moderately_deep_document_rejected_by_dom_depth_guard() {
    const DEPTH: usize = 1500; // > MAX_DEPTH (1024), <= pre-parse cap (4096)
    let mut html = String::new();
    for _ in 0..DEPTH {
        html.push_str("<div>");
    }
    html.push_str("deep");
    for _ in 0..DEPTH {
        html.push_str("</div>");
    }

    let err =
        rdfa::parse(&html).expect_err("a document deeper than MAX_DEPTH must be rejected with Err");
    let msg = err.to_string();
    assert!(
        msg.contains("depth") && msg.contains("1024-level limit"),
        "the error must name the parsed-DOM depth guard, got: {msg}"
    );
}

/// CR-011 M4: the byte-size budget rejects oversized input before html5ever
/// runs, and is configurable per parser instance.
#[test]
fn oversized_document_rejected_by_byte_budget() {
    let html = "<p>hello</p>\n".repeat(64); // 832 bytes
    assert!(html.len() > 64);

    let err = oxidowl::parsers::rio::rdfa::RdfaParser::new()
        .with_max_input_bytes(64)
        .parse(&html)
        .expect_err("input over the byte budget must be rejected");
    let msg = err.to_string();
    assert!(
        msg.contains("byte") && msg.contains("input budget"),
        "the error must name the input budget, got: {msg}"
    );

    // The default budget accepts ordinary documents.
    assert!(rdfa::parse(&html).is_ok());
}

// ─────────────────────────────────────────────────────────────────────────────
// 18. CR-011 M2 — N3 rule + IRI-less ontology annotation survive RDFa
//     (re-review NR-2: asserted round-trip, was only statically wired)
// ─────────────────────────────────────────────────────────────────────────────

/// Twin of `n3_rule_and_iri_less_annotation_survive_{jsonld,hdt,binary_rdf}_roundtrip`
/// for `RDFa`: the `RdfaRenderer` emits the shared preservation triples
/// (`urn:oxidowl:swrlRule` rule text + `urn:oxidowl:ontology` annotations)
/// as markup and `rdfa::parse` decodes them back through `map_rdf_triples`.
#[test]
fn n3_rule_and_iri_less_annotation_survive_rdfa_roundtrip() {
    use oxidowl::ontology::Annotation;
    use oxidowl::ontology::OntologyFormat;
    use oxidowl::parsers::parse_str;

    const LOG_FORMULA: &str = "http://www.w3.org/2000/10/swap/log#Formula";

    let source = "@prefix ex: <http://example.org/> .\n\
                  { ?x a ex:A } => { ?x a ex:B } .\n\
                  { ex:k ex:q ex:v } .\n";
    let ont = parse_str(source, OntologyFormat::N3).expect("N3 fixture must parse");
    assert!(ont.get_iri().is_none(), "fixture ontology must be IRI-less");

    let rule_of = |o: &Ontology| {
        o.axioms()
            .iter()
            .find_map(|a| match a {
                Axiom::Rule(rule) => Some(rule.rule.clone()),
                _ => None,
            })
            .expect("ontology must carry the SWRL rule")
    };
    let formula_annotation = |o: &Ontology| -> Vec<Annotation> {
        o.annotations
            .iter()
            .filter(|a| a.property.iri.as_str() == LOG_FORMULA)
            .cloned()
            .collect()
    };

    let rule = rule_of(&ont);
    let annotations = formula_annotation(&ont);
    assert_eq!(annotations.len(), 1, "fixture must carry one log:Formula");

    let html = RdfaRenderer::new().serialize(&ont).expect("RDFa serialize");
    // The preservation representation is visible in the document …
    assert!(
        html.contains("urn:oxidowl:swrlRule") && html.contains("DLSafeRule"),
        "rule must be present in the RDFa document, got:\n{html}"
    );
    assert!(
        html.contains("urn:oxidowl:ontology"),
        "IRI-less annotations must hang off the fallback subject, got:\n{html}"
    );

    // … and decodes back to the same data.
    let back = rdfa::parse(&html).expect("RDFa reparse");
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

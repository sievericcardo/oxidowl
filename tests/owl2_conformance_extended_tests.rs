//! # OWL 2 W3C Conformance Test Suite — Extended Corpus (Workstream C1)
//!
//! Expands the official W3C OWL 2 conformance coverage beyond the inline
//! Functional-Syntax tests in `owl2_conformance_suite.rs` (which is left
//! untouched).  New coverage:
//!
//! - **RDF-based semantics**: ontologies expressed as RDF graphs (Turtle and
//!   RDF/XML) rather than Functional Syntax, reasoned over through the normal
//!   [`Reasoner`] pipeline.
//! - **Datatype reasoning**: real datatype semantics where the current engine
//!   supports them (`is_instance_of` through the classification service:
//!   `DataSomeValuesFrom`, `DataHasValue`, facet evaluation, `xsd:int` ⊑
//!   `xsd:integer`).  Cases that depend on Workstream A4 (tableau concrete
//!   domains) or A7 (semantic entailment for data-property axioms) are marked
//!   `#[ignore]` with an explicit comment referencing the workstream.
//! - **Profile breadth**: EL/QL/RL/DL/Full accept/reject cases not covered by
//!   the base suite (data-property restrictions per profile, role-chain/oneOf/
//!   `HasKey` positions, illegal class+datatype punning → Full only).
//! - **Syntax round-trip**: Functional Syntax parse → serialize →
//!   re-parse → structural axiom-set comparison (annotations, datatype
//!   definitions, SWRL rules, negative assertions, `SameIndividual` /
//!   `DifferentIndividuals`).

use std::sync::{Arc, RwLock};

use oxidowl::{
    OWL2ProfileValidator, Reasoner, ReasonerConfig,
    core::reasoner::ReasoningStatistics,
    ontology::{ClassExpression, Individual, Ontology, OntologyFormat, axioms::Axiom},
    parsers::{parse_functional, parse_rdf_xml, parse_turtle},
    profiles::OWL2Profile,
};

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::assertions::assert_ontologies_axiom_equal;
use helpers::df::DF;

// ─────────────────────────────────────────────────────────────────────────────
// Harness
// ─────────────────────────────────────────────────────────────────────────────

/// Zero-sized harness grouping all extended-conformance utilities.
struct ConformanceExtRunner;

impl ConformanceExtRunner {
    fn parse_fs(fs: &str) -> Ontology {
        parse_functional(fs)
            .unwrap_or_else(|e| panic!("Functional syntax parse failed: {e}\nInput:\n{fs}"))
    }

    fn parse_ttl(ttl: &str) -> Ontology {
        parse_turtle(ttl).unwrap_or_else(|e| panic!("Turtle parse failed: {e}\nInput:\n{ttl}"))
    }

    fn parse_rdfxml(rdf: &str) -> Ontology {
        parse_rdf_xml(rdf).unwrap_or_else(|e| panic!("RDF/XML parse failed: {e}\nInput:\n{rdf}"))
    }

    fn reasoner_for(ontology: Ontology) -> Reasoner {
        let mut reasoner =
            Reasoner::new(ReasonerConfig::default()).expect("Failed to create Reasoner");
        reasoner
            .load_ontology(ontology)
            .expect("Failed to load ontology");
        reasoner
    }

    fn expect_consistent(ontology: &Ontology) {
        let reasoner = Self::reasoner_for(ontology.clone());
        let result = reasoner.is_consistent().expect("Consistency check failed");
        assert!(result, "Expected ontology to be consistent");
    }

    fn expect_inconsistent(ontology: &Ontology) {
        let reasoner = Self::reasoner_for(ontology.clone());
        let result = reasoner.is_consistent().expect("Consistency check failed");
        assert!(!result, "Expected ontology to be inconsistent");
    }

    fn expect_consistent_fs(fs: &str) {
        Self::expect_consistent(&Self::parse_fs(fs));
    }

    fn expect_inconsistent_fs(fs: &str) {
        Self::expect_inconsistent(&Self::parse_fs(fs));
    }

    fn expect_consistent_ttl(ttl: &str) {
        Self::expect_consistent(&Self::parse_ttl(ttl));
    }

    fn expect_subclass_of(
        ontology: &Ontology,
        subclass: &ClassExpression,
        superclass: &ClassExpression,
    ) {
        let reasoner = Self::reasoner_for(ontology.clone());
        let result = reasoner
            .is_subclass_of(subclass, superclass)
            .expect("Subsumption check failed");
        assert!(
            result,
            "Expected SubClassOf to be entailed: {subclass:?} ⊑ {superclass:?}"
        );
    }

    fn expect_instance_of(ontology: &Ontology, individual: &Individual, class: &ClassExpression) {
        let reasoner = Self::reasoner_for(ontology.clone());
        let result = reasoner
            .is_instance_of(individual, class)
            .expect("Instance check failed");
        assert!(
            result,
            "Expected instance membership: {individual:?} ∈ {class:?}"
        );
    }

    fn expect_not_instance_of(
        ontology: &Ontology,
        individual: &Individual,
        class: &ClassExpression,
    ) {
        let reasoner = Self::reasoner_for(ontology.clone());
        let result = reasoner
            .is_instance_of(individual, class)
            .expect("Instance check failed");
        assert!(
            !result,
            "Expected {individual:?} to NOT be an instance of {class:?}"
        );
    }

    /// Assert every axiom in `conclusion` is entailed by `premise`.
    ///
    /// `SubClassOf` axioms are checked with [`Reasoner::is_subclass_of`],
    /// `ClassAssertion` axioms with [`Reasoner::is_instance_of`], and all other
    /// axiom types through the general [`Reasoner::check_entailment`] path
    /// (which since workstream A7 performs *semantic* entailment for every
    /// logical axiom type, not just explicit-membership).
    fn expect_entails(premise: &Ontology, conclusion: &Ontology) {
        let mut reasoner = Self::reasoner_for(premise.clone());
        // Clone the reference so it does not borrow `reasoner` immutably while
        // `check_entailment` takes `&mut self`.
        let ont_ref = reasoner.get_ontology().expect("No ontology loaded").clone();

        for axiom in conclusion.axioms() {
            match axiom {
                Axiom::SubClassOf(sa) => {
                    let result = reasoner
                        .is_subclass_of(&sa.subclass, &sa.superclass)
                        .expect("Subsumption check failed");
                    assert!(
                        result,
                        "Expected SubClassOf to be entailed: {:?} ⊑ {:?}",
                        sa.subclass, sa.superclass
                    );
                }
                Axiom::ClassAssertion(ca) => {
                    let result = reasoner
                        .is_instance_of(&ca.individual, &ca.class)
                        .expect("Instance check failed");
                    assert!(
                        result,
                        "Expected ClassAssertion to be entailed: {:?} ∈ {:?}",
                        ca.individual, ca.class
                    );
                }
                _ => {
                    let mut stats = ReasoningStatistics::default();
                    let result = reasoner
                        .check_entailment(axiom, &ont_ref, &mut stats)
                        .unwrap_or_else(|e| panic!("Entailment check failed for {axiom:?}: {e}"));
                    assert!(result, "Expected axiom to be entailed: {axiom:?}");
                }
            }
        }
    }

    fn expect_profile_valid(ontology: &Ontology, profile: OWL2Profile) {
        let validator = OWL2ProfileValidator::new();
        let report = validator
            .validate_profile(ontology, profile)
            .expect("Profile validation failed");
        assert!(
            report.is_valid(),
            "Expected {:?} conformance, violations: {:?}",
            profile,
            report.violations
        );
    }

    fn expect_profile_invalid(ontology: &Ontology, profile: OWL2Profile) {
        let validator = OWL2ProfileValidator::new();
        let report = validator
            .validate_profile(ontology, profile)
            .expect("Profile validation failed");
        assert!(
            !report.is_valid(),
            "Expected {profile:?} to be rejected but ontology conformed"
        );
    }

    fn expect_profile_valid_fs(fs: &str, profile: OWL2Profile) {
        Self::expect_profile_valid(&Self::parse_fs(fs), profile);
    }

    fn expect_profile_invalid_fs(fs: &str, profile: OWL2Profile) {
        Self::expect_profile_invalid(&Self::parse_fs(fs), profile);
    }

    /// Parse Functional Syntax → serialize → re-parse → compare axiom sets
    /// structurally (ignoring axiom IDs).
    fn assert_functional_roundtrip(fs: &str) {
        let o1 = Self::parse_fs(fs);
        let serialized = oxidowl::save_to_string(&o1, OntologyFormat::Functional)
            .unwrap_or_else(|e| panic!("Functional serialization failed: {e}"));
        let o2 = Self::parse_fs(&serialized);
        assert_ontologies_axiom_equal(&o1, &o2);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// RDF-based Semantics Tests
// ─────────────────────────────────────────────────────────────────────────────

/// W3C RDF-serialization semantics — `rdfs:subClassOf` triples in Turtle
/// produce a subsumption hierarchy the reasoner can query.
#[test]
fn test_rdf_turtle_subclass_entailment() {
    let ttl = r"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix : <http://example.org/ont#> .

:A a owl:Class ; rdfs:subClassOf :B .
:B a owl:Class ; rdfs:subClassOf :C .
";
    let ontology = ConformanceExtRunner::parse_ttl(ttl);
    ConformanceExtRunner::expect_consistent(&ontology);

    let df = DF::new();
    ConformanceExtRunner::expect_subclass_of(
        &ontology,
        &df.class_ce("http://example.org/ont#A"),
        &df.class_ce("http://example.org/ont#C"),
    );
}

/// RDF graph semantics — `rdf:type` assertions combined with `rdfs:subClassOf`
/// entail instance membership through the reasoner.
#[test]
fn test_rdf_turtle_class_assertion_entailment() {
    let ttl = r"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix : <http://example.org/ont#> .

:Student rdf:type owl:Class ; rdfs:subClassOf :Person .
:Person rdf:type owl:Class .
:alice rdf:type :Student , owl:NamedIndividual .
";
    let ontology = ConformanceExtRunner::parse_ttl(ttl);
    ConformanceExtRunner::expect_consistent(&ontology);

    let df = DF::new();
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/ont#alice"),
        &df.class_ce("http://example.org/ont#Person"),
    );
}

/// RDF-based datatype reasoning — a Turtle graph expressing
/// `A ≡ DataSomeValuesFrom(P xsd:int)` with a data assertion for `a`.
/// The reasoner's classification service must derive `a ∈ A`.
///
/// IGNORED: the Turtle parser does not yet translate `owl:equivalentClass`
/// blank-node restrictions (`[owl:onProperty …; owl:someValuesFrom …]`) into
/// `DataSomeValuesFrom` class expressions — it either errors ("Cannot resolve
/// token `LeftBracket`") or emits `EquivalentClasses(A, owl:Class)`.  This is an
/// RDF→OWL mapping gap in the Turtle parser (not a reasoner gap); the test can
/// be enabled once that mapping lands (part of the C1/format surface work, not
/// the reasoner datatype workstream A4).
#[ignore = "Turtle parser does not map owl:equivalentClass blank-node restrictions (owl:onProperty/owl:someValuesFrom) to DataSomeValuesFrom; emits EquivalentClasses(A, owl:Class) or errors"]
#[test]
fn test_rdf_turtle_datatype_instance_entailment() {
    let ttl = r#"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://example.org/ont#> .

:A rdf:type owl:Class ;
   owl:equivalentClass [ rdf:type owl:Restriction ;
                         owl:onProperty :P ;
                         owl:someValuesFrom xsd:int ] .
:P rdf:type owl:DatatypeProperty .
:a rdf:type owl:NamedIndividual .
:a :P "5"^^xsd:int .
"#;
    let ontology = ConformanceExtRunner::parse_ttl(ttl);
    ConformanceExtRunner::expect_consistent(&ontology);

    let df = DF::new();
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/ont#a"),
        &df.class_ce("http://example.org/ont#A"),
    );
}

/// A complete OWL 2 DL ontology transcribed as an inline Turtle RDF graph
/// (W3C-style).  Verifies the ontology loads through the RDF mapping and that
/// the reasoner derives the expected classification and instance entailments.
#[test]
fn test_rdf_turtle_consistent_dl_ontology() {
    let ttl = r"
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
@prefix : <http://example.org/ont#> .

:ont rdf:type owl:Ontology .
:Person rdf:type owl:Class .
:Student rdf:type owl:Class ; rdfs:subClassOf :Person .
:Course rdf:type owl:Class .
:enrolledIn rdf:type owl:ObjectProperty ;
            rdfs:domain :Student ;
            rdfs:range :Course .
:john rdf:type :Student , owl:NamedIndividual .
";
    let ontology = ConformanceExtRunner::parse_ttl(ttl);
    ConformanceExtRunner::expect_consistent(&ontology);

    let df = DF::new();
    // :john ∈ Student, Student ⊑ Person ⇒ :john ∈ Person
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/ont#john"),
        &df.class_ce("http://example.org/ont#Person"),
    );
    // The range axiom is present as an ObjectPropertyRange axiom.
    let ont_ref = Arc::new(RwLock::new(ontology));
    let guard = ont_ref.read().unwrap();
    let has_range = guard
        .axioms()
        .iter()
        .any(|ax| matches!(ax, Axiom::ObjectPropertyRange(_)));
    assert!(
        has_range,
        "Expected ObjectPropertyRange axiom from RDF rdfs:range"
    );
}

/// RDF/XML (RDF-based) subclass entailment — the same OWL 2 semantics
/// expressed in RDF/XML rather than Turtle or Functional Syntax.
#[test]
fn test_rdf_xml_subclass_entailment() {
    let rdf = r#"<?xml version="1.0"?>
<rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#"
         xmlns:owl="http://www.w3.org/2002/07/owl#"
         xmlns:rdfs="http://www.w3.org/2000/01/rdf-schema#"
         xmlns:xsd="http://www.w3.org/2001/XMLSchema#">
  <owl:Ontology rdf:about="http://example.org/ont"/>
  <owl:Class rdf:about="http://example.org/ont#A">
    <rdfs:subClassOf rdf:resource="http://example.org/ont#B"/>
  </owl:Class>
  <owl:Class rdf:about="http://example.org/ont#B">
    <rdfs:subClassOf rdf:resource="http://example.org/ont#C"/>
  </owl:Class>
</rdf:RDF>"#;
    let ontology = ConformanceExtRunner::parse_rdfxml(rdf);
    ConformanceExtRunner::expect_consistent(&ontology);

    let df = DF::new();
    ConformanceExtRunner::expect_subclass_of(
        &ontology,
        &df.class_ce("http://example.org/ont#A"),
        &df.class_ce("http://example.org/ont#C"),
    );
}

/// Cross-syntax consistency: the same ontology expressed in Functional Syntax
/// and in Turtle must be consistent and agree on the same subsumption.
#[test]
fn test_rdf_functional_cross_syntax_agreement() {
    let fs = r"
Prefix(:=<http://example.org/ont#>)
Ontology(<http://example.org/ont>
  SubClassOf(:A :B)
  SubClassOf(:B :C)
  ClassAssertion(:A :i)
)
";
    let ttl = r"
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix : <http://example.org/ont#> .
:A rdfs:subClassOf :B .
:B rdfs:subClassOf :C .
:i rdf:type :A .
";

    let fs_ont = ConformanceExtRunner::parse_fs(fs);
    let ttl_ont = ConformanceExtRunner::parse_ttl(ttl);

    let df = DF::new();
    let a = df.class_ce("http://example.org/ont#A");
    let c = df.class_ce("http://example.org/ont#C");

    ConformanceExtRunner::expect_subclass_of(&fs_ont, &a, &c);
    ConformanceExtRunner::expect_subclass_of(&ttl_ont, &a, &c);
    ConformanceExtRunner::expect_instance_of(&fs_ont, &df.named("http://example.org/ont#i"), &a);
    ConformanceExtRunner::expect_instance_of(&ttl_ont, &df.named("http://example.org/ont#i"), &a);
    ConformanceExtRunner::expect_consistent(&fs_ont);
    ConformanceExtRunner::expect_consistent(&ttl_ont);
}

// ─────────────────────────────────────────────────────────────────────────────
// Datatype Reasoning Tests
// ─────────────────────────────────────────────────────────────────────────────
//
// The reasoner's *classification* path (`is_instance_of`) performs real
// datatype checks: literal datatype compatibility, facet evaluation, and
// DataHasValue matching.  The *tableau* path (`is_consistent`) does not yet
// enforce datatype value spaces (Workstream A4) — cases that require it are
// marked `#[ignore]` and reference A4.  Cases that require semantic entailment
// for data-property assertions (Workstream A7) are marked accordingly.

/// W3C datatype semantics — `"5"^^xsd:int` is in the value space of
/// `xsd:integer` (int ⊑ integer), so `a` must satisfy
/// `A ≡ DataSomeValuesFrom(P xsd:integer)`.
#[test]
fn test_datatype_subtyping_int_satisfies_integer_range() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataSomeValuesFrom(:P xsd:integer))
  DataPropertyAssertion(:P :a "5"^^xsd:int)
)
"#;
    let ontology = ConformanceExtRunner::parse_fs(fs);
    let df = DF::new();
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/test#a"),
        &df.class_ce("http://example.org/test#A"),
    );
}

/// W3C datatype semantics — an out-of-range literal must NOT satisfy a
/// facet-restricted `DataSomeValuesFrom` range.
///
/// `A ≡ DataSomeValuesFrom(P, int ⊓ [minInclusive 10, maxInclusive 5])` is an
/// empty data range (10 ≤ v ≤ 5 has no solution); `a P 7` must not put `a` in `A`.
#[test]
fn test_datatype_facet_restriction_rejects_out_of_range_literal() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataSomeValuesFrom(:P DatatypeRestriction(xsd:int xsd:minInclusive "10"^^xsd:int xsd:maxInclusive "5"^^xsd:int)))
  DataPropertyAssertion(:P :a "7"^^xsd:int)
)
"#;
    let ontology = ConformanceExtRunner::parse_fs(fs);
    let df = DF::new();
    ConformanceExtRunner::expect_not_instance_of(
        &ontology,
        &df.named("http://example.org/test#a"),
        &df.class_ce("http://example.org/test#A"),
    );
}

/// W3C datatype semantics — a literal inside both facet bounds satisfies the
/// range.
#[test]
fn test_datatype_facet_restriction_accepts_in_range_literal() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataSomeValuesFrom(:P DatatypeRestriction(xsd:int xsd:minInclusive "1"^^xsd:int xsd:maxInclusive "10"^^xsd:int)))
  DataPropertyAssertion(:P :a "7"^^xsd:int)
)
"#;
    let ontology = ConformanceExtRunner::parse_fs(fs);
    let df = DF::new();
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/test#a"),
        &df.class_ce("http://example.org/test#A"),
    );
}

/// W3C datatype semantics — `DataHasValue`: `a P "x"` makes `a` an instance of
/// `A ≡ DataHasValue(P "x")`.
#[test]
fn test_datatype_has_value_instance_membership() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataHasValue(:P "x"^^xsd:string))
  DataPropertyAssertion(:P :a "x"^^xsd:string)
)
"#;
    let ontology = ConformanceExtRunner::parse_fs(fs);
    let df = DF::new();
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/test#a"),
        &df.class_ce("http://example.org/test#A"),
    );
}

/// `DataHasValue` converse — `A ≡ DataHasValue(P "x")` and `A(a)` should entail
/// `P(a, "x")`.  The general entailment path (`check_entailment`) answers
/// `DataPropertyAssertion` semantically since workstream A7: the completed
/// tableau materialises the `P`-edge to a `DataValue("x")` successor of `a` via
/// the `DataHasValue` unfolding, and the data-assertion check inspects the data
/// edge set.
#[test]
fn test_datatype_has_value_converse_entailment() {
    let premise = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataHasValue(:P "x"^^xsd:string))
  ClassAssertion(:A :a)
)
"#;
    let conclusion = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  DataPropertyAssertion(:P :a "x"^^xsd:string)
)
"#;
    ConformanceExtRunner::expect_entails(
        &ConformanceExtRunner::parse_fs(premise),
        &ConformanceExtRunner::parse_fs(conclusion),
    );
}

/// Contradictory datatype facets — a class whose data range is empty
/// (`int ⊓ [minInclusive 10, maxInclusive 5]`) should be unsatisfiable, so an
/// individual asserted in it makes the ontology inconsistent.
///
/// Workstream A4: `apply_datatype_rule` now detects the empty range via facet
/// contradiction and records a datatype clash on the individual's data successor.
#[test]
fn test_datatype_contradictory_facets_unsatisfiable() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataSomeValuesFrom(:P DatatypeRestriction(xsd:int xsd:minInclusive "10"^^xsd:int xsd:maxInclusive "5"^^xsd:int)))
  ClassAssertion(:A :a)
)
"#;
    ConformanceExtRunner::expect_inconsistent_fs(fs);
}

/// `owl:real` / `owl:rational` basic value-space behavior — a decimal literal
/// should be in the `owl:real` value space.
///
/// Workstream A4: the datatype-compatibility map now recognises `owl:real` /
/// `owl:rational` (real contains all numeric XSD datatypes; rational contains the
/// decimal/integer families).
#[test]
fn test_datatype_owl_real_value_space() {
    let fs = r#"
Prefix(:=<http://example.org/test#>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  EquivalentClasses(:A DataSomeValuesFrom(:P owl:real))
  DataPropertyAssertion(:P :a "1.5"^^xsd:decimal)
)
"#;
    let ontology = ConformanceExtRunner::parse_fs(fs);
    let df = DF::new();
    ConformanceExtRunner::expect_instance_of(
        &ontology,
        &df.named("http://example.org/test#a"),
        &df.class_ce("http://example.org/test#A"),
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Profile Breadth Tests
// ─────────────────────────────────────────────────────────────────────────────

// ── OWL 2 EL ─────────────────────────────────────────────────────────────────

/// EL: `ReflexiveObjectProperty` is permitted (falls in the EL allow-set).
#[test]
fn test_el_accepts_reflexive_property() {
    ConformanceExtRunner::expect_profile_valid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  ReflexiveObjectProperty(:knows)
)
",
        OWL2Profile::EL,
    );
}

/// EL: data property restrictions (`DataSomeValuesFrom`) are permitted.
#[test]
fn test_el_accepts_data_property_restriction() {
    ConformanceExtRunner::expect_profile_valid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  SubClassOf(:A DataSomeValuesFrom(:age xsd:int))
)
",
        OWL2Profile::EL,
    );
}

/// EL: `ObjectHasSelf` on a simple (named) property is permitted.
#[test]
fn test_el_accepts_object_has_self_simple() {
    ConformanceExtRunner::expect_profile_valid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:Narcissist ObjectHasSelf(:likes))
)
",
        OWL2Profile::EL,
    );
}

/// EL (B3): `IrreflexiveObjectProperty` is rejected by the EL validator.
#[test]
fn test_el_rejects_irreflexive_property() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  IrreflexiveObjectProperty(:r)
)
",
        OWL2Profile::EL,
    );
}

/// EL: a property chain in an existential is rejected (simple properties only).
#[test]
fn test_el_rejects_property_chain_in_restriction() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectSomeValuesFrom(ObjectPropertyChain(:p :q) :B))
)
",
        OWL2Profile::EL,
    );
}

// ── OWL 2 QL ─────────────────────────────────────────────────────────────────

/// QL: disjunction (`ObjectUnionOf`) breadth is rejected.
#[test]
fn test_ql_rejects_union_breadth() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectUnionOf(:B :C))
)
",
        OWL2Profile::QL,
    );
}

/// QL: `HasKey` axioms are not permitted.
#[test]
fn test_ql_rejects_haskey() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  HasKey(:Person () (:ssn))
)
",
        OWL2Profile::QL,
    );
}

/// QL: `FunctionalObjectProperty` is not permitted.
#[test]
fn test_ql_rejects_functional_property() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  FunctionalObjectProperty(:p)
)
",
        OWL2Profile::QL,
    );
}

/// QL: `ObjectHasSelf` is not permitted.
#[test]
fn test_ql_rejects_object_has_self() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectHasSelf(:r))
)
",
        OWL2Profile::QL,
    );
}

// ── OWL 2 RL ─────────────────────────────────────────────────────────────────

/// RL (B3): `HasKey` is rejected by the RL validator (now enforced).
#[test]
fn test_rl_rejects_haskey() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  HasKey(:Person () (:ssn))
)
",
        OWL2Profile::RL,
    );
}

/// RL: `ObjectOneOf` (nominals) is not permitted.
#[test]
fn test_rl_rejects_one_of() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectOneOf(:i))
)
",
        OWL2Profile::RL,
    );
}

/// RL: `ObjectHasSelf` is not permitted.
#[test]
fn test_rl_rejects_has_self() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectHasSelf(:r))
)
",
        OWL2Profile::RL,
    );
}

/// RL: `ObjectComplementOf` is not permitted.
#[test]
fn test_rl_rejects_complement() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SubClassOf(:A ObjectComplementOf(:B))
)
",
        OWL2Profile::RL,
    );
}

/// RL: `DisjointUnion` is not permitted.
#[test]
fn test_rl_rejects_disjoint_union() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  DisjointUnion(:A :B :C)
)
",
        OWL2Profile::RL,
    );
}

// ── OWL 2 DL / Full ──────────────────────────────────────────────────────────

/// DL (B3): using a single IRI as both a class and a datatype is illegal
/// punning → the ontology is NOT OWL 2 DL conformant.
#[test]
fn test_dl_rejects_class_datatype_punning() {
    ConformanceExtRunner::expect_profile_invalid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  Declaration(Class(:x))
  Declaration(Datatype(:x))
  SubClassOf(:x :B)
)
",
        OWL2Profile::DL,
    );
}

/// Full: the same class+datatype punning ontology conforms to OWL 2 Full
/// (Full imposes no structural restrictions).
#[test]
fn test_full_accepts_class_datatype_punning() {
    ConformanceExtRunner::expect_profile_valid_fs(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  Declaration(Class(:x))
  Declaration(Datatype(:x))
  SubClassOf(:x :B)
)
",
        OWL2Profile::Full,
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Syntax Round-trip Tests
// ─────────────────────────────────────────────────────────────────────────────

/// Round-trip preserves ontology-level annotation axioms.
#[test]
fn test_roundtrip_annotations() {
    ConformanceExtRunner::assert_functional_roundtrip(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)
Ontology(<http://example.org/test>
  SubClassOf(:A :B)
  AnnotationAssertion(rdfs:label :A "Class A")
  AnnotationAssertion(rdfs:comment :B "Class B")
)
"#,
    );
}

/// Round-trip preserves `DatatypeDefinition` axioms.
#[test]
fn test_roundtrip_datatype_definition() {
    ConformanceExtRunner::assert_functional_roundtrip(
        r"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  DatatypeDefinition(:MyDatatype xsd:int)
)
",
    );
}

/// Round-trip preserves SWRL rules (`DLSafeRule`) in Functional Syntax.
#[test]
fn test_roundtrip_swrl_rule() {
    ConformanceExtRunner::assert_functional_roundtrip(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  DLSafeRule(Body(ClassAtom(:Person ?x)) Head(ClassAtom(:Adult ?x)))
)
",
    );
}

/// Round-trip preserves negative object/data property assertions.
#[test]
fn test_roundtrip_negative_assertions() {
    ConformanceExtRunner::assert_functional_roundtrip(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Ontology(<http://example.org/test>
  NegativeObjectPropertyAssertion(:p :a :b)
  NegativeDataPropertyAssertion(:dp :a "x"^^xsd:string)
)
"#,
    );
}

/// Round-trip preserves `SameIndividual` and `DifferentIndividuals` axioms.
#[test]
fn test_roundtrip_same_different_individuals() {
    ConformanceExtRunner::assert_functional_roundtrip(
        r"
Prefix(:=<http://example.org/test#>)
Ontology(<http://example.org/test>
  SameIndividual(:a :b)
  DifferentIndividuals(:c :d)
)
",
    );
}

/// Round-trip preserves a mixed ontology spanning multiple axiom families.
#[test]
fn test_roundtrip_mixed_axioms() {
    ConformanceExtRunner::assert_functional_roundtrip(
        r#"
Prefix(:=<http://example.org/test#>)
Prefix(xsd:=<http://www.w3.org/2001/XMLSchema#>)
Prefix(rdfs:=<http://www.w3.org/2000/01/rdf-schema#>)
Ontology(<http://example.org/test>
  Declaration(Class(:A))
  Declaration(Class(:B))
  SubClassOf(:A :B)
  EquivalentClasses(:C :B)
  AnnotationAssertion(rdfs:label :A "A")
  DatatypeDefinition(:MyDT xsd:int)
  DLSafeRule(Body(ClassAtom(:A ?x)) Head(ClassAtom(:B ?x)))
  NegativeObjectPropertyAssertion(:p :a :b)
  SameIndividual(:a :c)
  DifferentIndividuals(:d :e)
)
"#,
    );
}

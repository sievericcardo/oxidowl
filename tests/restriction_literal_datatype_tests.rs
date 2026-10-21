//! Regression tests for `^^datatype` tags on restriction literals parsed from
//! Turtle (parser-only fix — the reasoner is untouched).
//!
//! The legacy Turtle parser kept the `^^datatype` tag on plain literal *objects*
//! but dropped it inside class restrictions:
//!
//! * `parse_facet` built the facet value with `datatype: None`;
//! * the `owl:hasValue` branch did the same.
//!
//! `DatatypeValidator::literal_values_equal` treats `(Some, None)` as unequal,
//! so `DataHasValue { value: Literal { "true", datatype: None } }` never matched
//! the ABox literal `"true"^^xsd:boolean`, the whole `ObjectIntersectionOf`
//! conjunction failed, and `realize()` never derived the defined class.
//!
//! RDF 1.1 additionally defines an *untagged* plain literal as `xsd:string`, so
//! restriction literals without an explicit tag now default to `xsd:string`
//! rather than `datatype: None`.

use oxidowl::{
    Reasoner, ReasonerConfig,
    core::reasoner::RealisationResult,
    ontology::{ClassExpression, IRI, Individual, NamedIndividual},
    parsers::parse_turtle,
};

const EX: &str = "http://example.org/";

/// `Slowish ≡ ∃latencyMs.(xsd:double [≥ 30.0]) ⊓ (degraded = true)` plus three
/// individuals: `svc0` violates the facet, `svc2` has the wrong `hasValue`,
/// `svc1` satisfies both conjuncts.
const ONTOLOGY: &str = r#"
@prefix : <http://example.org/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

:latencyMs rdf:type owl:DatatypeProperty ;
           rdfs:range xsd:double .
:degraded rdf:type owl:DatatypeProperty ;
          rdfs:range xsd:boolean .

:Slowish rdf:type owl:Class ;
         owl:equivalentClass [ owl:intersectionOf ( [ rdf:type owl:Restriction ;
                                                      owl:onProperty :latencyMs ;
                                                      owl:someValuesFrom [ rdf:type rdfs:Datatype ;
                                                                           owl:onDatatype xsd:double ;
                                                                           owl:withRestrictions ( [ xsd:minInclusive "30.0"^^xsd:double ] ) ]
                                                    ]
                                                    [ rdf:type owl:Restriction ;
                                                      owl:onProperty :degraded ;
                                                      owl:hasValue "true"^^xsd:boolean
                                                    ]
                                                  ) ;
                               rdf:type owl:Class
                             ] .

:svc0 rdf:type owl:NamedIndividual ;
      :latencyMs "15.0"^^xsd:double ;
      :degraded "true"^^xsd:boolean .

:svc1 rdf:type owl:NamedIndividual ;
      :latencyMs "45.0"^^xsd:double ;
      :degraded "true"^^xsd:boolean .

:svc2 rdf:type owl:NamedIndividual ;
      :latencyMs "45.0"^^xsd:double ;
      :degraded "false"^^xsd:boolean .
"#;

fn realise(source: &str) -> oxidowl::Result<RealisationResult> {
    let ontology = parse_turtle(source)?;
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;
    reasoner.realize()
}

/// The realised type set of `individual_iri` as a set of IRI strings (named
/// classes keep their IRI; anything else renders through `Display`).
fn realised_type_iris(result: &RealisationResult, individual_iri: &str) -> Vec<String> {
    let individual = Individual::Named(NamedIndividual {
        iri: IRI::new(individual_iri),
    });
    let mut types: Vec<String> = result
        .types
        .get(&individual)
        .map(|types| {
            types
                .iter()
                .map(|t| match t {
                    ClassExpression::Class(c) => c.iri.as_str().to_string(),
                    other => other.to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    types.sort();
    types
}

/// The contract: an individual satisfying a numeric facet restriction **and** a
/// typed `owl:hasValue` conjunct must be realised as an instance of the defined
/// class. Before the fix the `DataHasValue` literal lost its `^^xsd:boolean`
/// tag at parse time and the conjunction never matched.
#[test]
fn realise_derives_defined_class_from_typed_has_value_and_facet() {
    let result = realise(ONTOLOGY).expect("realisation should succeed");

    let svc1 = realised_type_iris(&result, &format!("{EX}svc1"));
    assert!(
        svc1.contains(&format!("{EX}Slowish")),
        "svc1 (latency 45 ≥ 30, degraded = true) must be realised as Slowish; \
         got types: {svc1:?}"
    );
}

/// Negative guards — the fix must not over-derive by dropping constraints:
/// `svc0` violates `xsd:minInclusive 30.0`, `svc2` violates `degraded = true`.
#[test]
fn realise_does_not_derive_class_for_non_members() {
    let result = realise(ONTOLOGY).expect("realisation should succeed");

    let svc0 = realised_type_iris(&result, &format!("{EX}svc0"));
    assert!(
        !svc0.contains(&format!("{EX}Slowish")),
        "svc0 (latency 15 < 30) must not be realised as Slowish; got types: {svc0:?}"
    );

    let svc2 = realised_type_iris(&result, &format!("{EX}svc2"));
    assert!(
        !svc2.contains(&format!("{EX}Slowish")),
        "svc2 (degraded = false) must not be realised as Slowish; got types: {svc2:?}"
    );
}

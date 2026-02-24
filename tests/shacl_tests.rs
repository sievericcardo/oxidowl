//! SHACL integration tests
//!
//! Tests span the full validation pipeline: parse shapes → resolve targets →
//! evaluate constraints → produce a `ShaclValidationReport`.

use oxidowl::validation::shacl::{ShaclSeverity, ShaclValidationReport, ShaclValidator};

// ── helpers ──────────────────────────────────────────────────────────────────

fn validate(shapes: &str, data: &str) -> ShaclValidationReport {
    let mut v = ShaclValidator::new(shapes, data).expect("failed to build validator");
    v.validate().expect("failed to validate")
}

const PREFIXES: &str = r#"
    @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
    @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
    @prefix sh:   <http://www.w3.org/ns/shacl#> .
    @prefix ex:   <http://example.org/> .
"#;

fn with_prefixes(s: &str) -> String {
    format!("{PREFIXES}\n{s}")
}

// ── value-type constraints ────────────────────────────────────────────────────

#[test]
fn test_mincount_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:name ;
                sh:minCount 1 ;
            ] .
        "#,
    );
    let data = with_prefixes(
        r#"
        ex:Alice a ex:Person ;
            ex:name "Alice" .
        "#,
    );
    let report = validate(&shapes, &data);
    assert!(report.conforms, "expected conforms=true, got: {report:?}");
}

#[test]
fn test_mincount_violated() {
    let shapes = with_prefixes(
        r#"
        ex:PersonShape a sh:NodeShape ;
            sh:targetClass ex:Person ;
            sh:property [
                sh:path ex:name ;
                sh:minCount 1 ;
            ] .
        "#,
    );
    // ex:Bob has no ex:name → violation
    let data = with_prefixes("ex:Bob a ex:Person .");
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
    assert_eq!(report.results.len(), 1);
}

#[test]
fn test_maxcount_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:p ; sh:maxCount 1 ] .
        "#,
    );
    let data = with_prefixes(
        r#"
        ex:n a ex:T ;
            ex:p "a" ; ex:p "b" .
        "#,
    );
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
    assert_eq!(report.results.len(), 1);
}

#[test]
fn test_datatype_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [
                sh:path ex:age ;
                sh:datatype xsd:integer ;
            ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:age "42"^^xsd:integer ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
}

#[test]
fn test_datatype_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [
                sh:path ex:age ;
                sh:datatype xsd:integer ;
                sh:minCount 1 ;
            ] .
        "#,
    );
    // value is xsd:string, not xsd:integer
    let data = with_prefixes(r#"ex:n a ex:T ; ex:age "hello"^^xsd:string ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── string constraints ────────────────────────────────────────────────────────

#[test]
fn test_min_length_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:code ; sh:minLength 3 ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:code "ABC" ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
}

#[test]
fn test_min_length_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:code ; sh:minLength 3 ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:code "AB" ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

#[test]
fn test_pattern_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:code ; sh:pattern "^[A-Z]{3}$" ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:code "ABC" ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
}

#[test]
fn test_pattern_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:code ; sh:pattern "^[A-Z]{3}$" ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:code "abc" ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── value-range constraints ───────────────────────────────────────────────────

#[test]
fn test_min_exclusive_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:price ; sh:minExclusive "0"^^xsd:decimal ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:price "1.5"^^xsd:decimal ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
}

#[test]
fn test_min_exclusive_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:price ; sh:minExclusive "0"^^xsd:decimal ] .
        "#,
    );
    // value == 0 violates strict >0
    let data = with_prefixes(r#"ex:n a ex:T ; ex:price "0"^^xsd:decimal ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── logical constraints ───────────────────────────────────────────────────────

#[test]
fn test_or_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:or (
                [ sh:property [ sh:path ex:a ; sh:minCount 1 ] ]
                [ sh:property [ sh:path ex:b ; sh:minCount 1 ] ]
            ) .
        "#,
    );
    // has ex:b → satisfies second branch
    let data = with_prefixes(r#"ex:n a ex:T ; ex:b "x" ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
}

#[test]
fn test_or_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:or (
                [ sh:property [ sh:path ex:a ; sh:minCount 1 ] ]
                [ sh:property [ sh:path ex:b ; sh:minCount 1 ] ]
            ) .
        "#,
    );
    // has neither ex:a nor ex:b
    let data = with_prefixes(r#"ex:n a ex:T ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── severity ──────────────────────────────────────────────────────────────────

#[test]
fn test_warning_severity() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [
                sh:path ex:opt ;
                sh:minCount 1 ;
                sh:severity sh:Warning ;
            ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ."#);
    let report = validate(&shapes, &data);
    // a sh:Warning result still makes conforms=false per the spec
    assert!(!report.conforms);
    assert_eq!(report.results.len(), 1);
    assert!(matches!(report.results[0].severity, ShaclSeverity::Warning));
}

// ── target: node ──────────────────────────────────────────────────────────────

#[test]
fn test_target_node() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetNode ex:Alice ;
            sh:property [ sh:path ex:name ; sh:minCount 1 ] .
        "#,
    );
    // ex:Alice has no ex:name key
    let data = with_prefixes(r#"ex:Alice ex:email "a@b.com" ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── hasValue / in ─────────────────────────────────────────────────────────────

#[test]
fn test_has_value_satisfied() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:status ; sh:hasValue "active" ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:status "active" ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
}

#[test]
fn test_in_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:color ; sh:in ("red" "green" "blue") ] .
        "#,
    );
    let data = with_prefixes(r#"ex:n a ex:T ; ex:color "yellow" ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── property pair ─────────────────────────────────────────────────────────────

#[test]
fn test_disjoint_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:property [ sh:path ex:a ; sh:disjoint ex:b ] .
        "#,
    );
    // both ex:a and ex:b have the same value → disjoint violated
    let data = with_prefixes(r#"ex:n a ex:T ; ex:a "x" ; ex:b "x" ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── closed shape ──────────────────────────────────────────────────────────────

#[test]
fn test_closed_shape_violated() {
    let shapes = with_prefixes(
        r#"
        ex:S a sh:NodeShape ;
            sh:targetClass ex:T ;
            sh:closed true ;
            sh:property [ sh:path ex:a ] .
        "#,
    );
    // ex:extra is not declared → violation
    let data = with_prefixes(r#"ex:n a ex:T ; ex:a "ok" ; ex:extra "bad" ."#);
    let report = validate(&shapes, &data);
    assert!(!report.conforms);
}

// ── empty shapes graph → trivially conforms ──────────────────────────────────

#[test]
fn test_empty_shapes_graph_conforms() {
    let shapes = with_prefixes("");
    let data = with_prefixes(r#"ex:Alice a ex:Person ; ex:name "Alice" ."#);
    let report = validate(&shapes, &data);
    assert!(report.conforms);
    assert!(report.results.is_empty());
}

//! Kani harnesses for RDF 1.1 and OWL 2 foundational type properties.
//!
//! Sources:
//! - RDF 1.1 Concepts: <https://www.w3.org/TR/rdf11-concepts/>
//! - OWL 2 RDF-Based Semantics: <https://www.w3.org/TR/owl2-rdf-based-semantics/>
//!
//! Covers:
//! - `IRI` equality, reflexivity, and round-trip properties.
//! - `Individual` named / anonymous classification invariants.
//! - `Ontology` initial empty-axiom state.

#![cfg(kani)]

use crate::ontology::{IRI, Individual, Ontology};

// ── IRI Equality ──────────────────────────────────────────────────────────

/// An IRI is equal to itself (reflexivity).
///
/// RDF 1.1 §3.1: IRIs are absolute Unicode strings; equality is
/// string identity.
#[kani::proof]
fn rdf_iri_equality_reflexive() {
    let iri = IRI::new("http://example.org/subject");
    assert_eq!(iri, iri.clone(), "IRI must be equal to itself");
}

/// Two `IRI` values built from the same string are equal.
///
/// RDF §3.1: IRI equality is defined by Unicode-string equality.
#[kani::proof]
fn rdf_iri_same_string_equal() {
    let a = IRI::new("http://example.org/foo");
    let b = IRI::new("http://example.org/foo");
    assert_eq!(a, b, "same-string IRIs must be equal");
}

/// Two `IRI` values built from different strings are not equal.
///
/// Contrapositive of string-equality rule.
#[kani::proof]
fn rdf_iri_different_string_not_equal() {
    let a = IRI::new("http://example.org/foo");
    let b = IRI::new("http://example.org/bar");
    assert_ne!(a, b, "different-string IRIs must not be equal");
}

/// Cloning an IRI preserves equality.
#[kani::proof]
fn rdf_iri_clone_equals_original() {
    let original = IRI::new("http://example.org/alpha");
    let cloned = original.clone();
    assert_eq!(original, cloned, "cloned IRI must equal original");
}

/// `IRI::as_str()` round-trips the original string.
#[kani::proof]
fn rdf_iri_as_str_round_trip() {
    let s = "http://www.w3.org/2000/01/rdf-schema#label";
    let iri = IRI::new(s);
    assert_eq!(
        iri.as_str(),
        s,
        "as_str round-trip must return original string"
    );
}

// ── Individual Classification ─────────────────────────────────────────────

/// A named individual reports `is_named() == true`.
///
/// OWL 2 §5.6.1: Named individuals are identified by IRIs.
#[kani::proof]
fn rdf_named_individual_is_named() {
    let iri = IRI::new("http://example.org/John");
    let ind = Individual::named(iri);
    assert!(
        ind.is_named(),
        "named individual must report is_named() == true"
    );
}

/// A named individual reports `is_anonymous() == false`.
#[kani::proof]
fn rdf_named_individual_is_not_anonymous() {
    let iri = IRI::new("http://example.org/John");
    let ind = Individual::named(iri);
    assert!(
        !ind.is_anonymous(),
        "named individual must report is_anonymous() == false"
    );
}

/// An anonymous individual reports `is_anonymous() == true`.
///
/// OWL 2 §5.6.2: Anonymous individuals are distinguished by blank-node IDs.
#[kani::proof]
fn rdf_anonymous_individual_is_anonymous() {
    let ind = Individual::anonymous(String::from("_:b0"));
    assert!(
        ind.is_anonymous(),
        "anonymous individual must report is_anonymous() == true"
    );
}

/// An anonymous individual reports `is_named() == false`.
#[kani::proof]
fn rdf_anonymous_individual_is_not_named() {
    let ind = Individual::anonymous(String::from("_:b0"));
    assert!(
        !ind.is_named(),
        "anonymous individual must report is_named() == false"
    );
}

/// `Individual::named(iri).iri()` returns `Some(&iri)`.
#[kani::proof]
fn rdf_named_individual_iri_returns_some() {
    let iri = IRI::new("http://example.org/Jane");
    let ind = Individual::named(iri.clone());
    assert_eq!(
        ind.iri(),
        Some(&iri),
        "named individual iri() must return Some(original IRI)"
    );
}

/// `Individual::anonymous(id).iri()` returns `None`.
///
/// Anonymous individuals carry no IRI.
#[kani::proof]
fn rdf_anonymous_individual_iri_returns_none() {
    let ind = Individual::anonymous(String::from("_:b1"));
    assert_eq!(
        ind.iri(),
        None,
        "anonymous individual iri() must return None"
    );
}

/// Named and anonymous individuals built with distinct kinds are not equal.
#[kani::proof]
fn rdf_named_and_anonymous_are_distinct() {
    let iri = IRI::new("http://example.org/Alice");
    let named = Individual::named(iri);
    let anon = Individual::anonymous(String::from("_:b2"));
    assert_ne!(
        named, anon,
        "named and anonymous individuals must not be equal"
    );
}

// ── Ontology Initial State ────────────────────────────────────────────────

/// A freshly created `Ontology` contains no axioms.
///
/// RDF/OWL 2: The empty ontology has an empty axiom set.
#[kani::proof]
fn rdf_ontology_new_axioms_empty() {
    let ont = Ontology::new();
    assert!(ont.axioms().is_empty(), "new Ontology must have no axioms");
}

/// A freshly created `Ontology` has no ontology IRI.
#[kani::proof]
fn rdf_ontology_new_has_no_iri() {
    let ont = Ontology::new();
    assert!(
        ont.get_iri().is_none(),
        "new Ontology must have no IRI by default"
    );
}

//! Kani harnesses for `ontology` types.
//!
//! Covers:
//! - [`IRI`]: round-trip identity, reflexive equality, distinctness.
//! - [`Class`]: `owl:Thing` / `owl:Nothing` distinctness, constructor fidelity.
//! - [`ClassExpression`]: basic constructor properties.

#![cfg(kani)]

use crate::ontology::{
    IRI,
    concepts::{Class, ClassExpression},
};

// ── IRI ──────────────────────────────────────────────────────────────────────

/// `IRI::new(s).as_str()` must return the exact same string slice.
#[kani::proof]
fn iri_new_preserves_str_owl_thing() {
    let s = "http://www.w3.org/2002/07/owl#Thing";
    let iri = IRI::new(s);
    assert_eq!(
        iri.as_str(),
        s,
        "IRI::as_str() must return the string passed to IRI::new()"
    );
}

/// `IRI::new(s).as_str()` round-trip for an arbitrary concrete IRI string.
#[kani::proof]
fn iri_new_preserves_str_example() {
    let s = "http://example.org/SomeConcept";
    let iri = IRI::new(s);
    assert_eq!(iri.as_str(), s);
}

/// A cloned IRI must be equal to the original.
#[kani::proof]
fn iri_clone_equals_original() {
    let iri = IRI::new("http://example.org/Person");
    let iri2 = iri.clone();
    assert_eq!(iri, iri2, "cloned IRI must equal the original");
}

/// IRI equality is reflexive.
#[kani::proof]
fn iri_eq_reflexive() {
    let iri = IRI::new("http://www.w3.org/2002/07/owl#Nothing");
    // PartialEq reflexivity
    assert!(iri == iri, "IRI must be equal to itself");
}

/// Two IRIs constructed from distinct string values must be unequal.
#[kani::proof]
fn iri_distinct_values_ne() {
    let iri_a = IRI::new("http://example.org/ClassA");
    let iri_b = IRI::new("http://example.org/ClassB");
    assert_ne!(iri_a, iri_b, "IRIs with different values must not be equal");
}

/// Two IRIs constructed from the same string must be equal.
#[kani::proof]
fn iri_same_value_eq() {
    let s = "http://example.org/Same";
    let iri_a = IRI::new(s);
    let iri_b = IRI::new(s);
    assert_eq!(
        iri_a, iri_b,
        "IRIs built from the same string must be equal"
    );
}

/// `IRI::from(String)` must produce an IRI equal to `IRI::new(&string)`.
#[kani::proof]
fn iri_from_string_matches_new() {
    let s = "http://example.org/FromString";
    let via_new: IRI = IRI::new(s);
    let via_from: IRI = IRI::from(s.to_string());
    assert_eq!(
        via_new, via_from,
        "IRI::from(String) must match IRI::new(&str)"
    );
}

// ── Class ─────────────────────────────────────────────────────────────────────

/// `owl:Thing` and `owl:Nothing` must be distinct.
#[kani::proof]
fn class_thing_vs_nothing_distinct() {
    let thing = Class::thing();
    let nothing = Class::nothing();
    assert_ne!(
        thing, nothing,
        "owl:Thing and owl:Nothing must be distinct classes"
    );
}

/// `Class::thing()` must satisfy `is_thing()`.
#[kani::proof]
fn class_thing_is_thing() {
    let thing = Class::thing();
    assert!(thing.is_thing(), "Class::thing() must satisfy is_thing()");
}

/// `Class::nothing()` must satisfy `is_nothing()`.
#[kani::proof]
fn class_nothing_is_nothing() {
    let nothing = Class::nothing();
    assert!(
        nothing.is_nothing(),
        "Class::nothing() must satisfy is_nothing()"
    );
}

/// `Class::thing()` must not satisfy `is_nothing()`.
#[kani::proof]
fn class_thing_not_nothing() {
    let thing = Class::thing();
    assert!(
        !thing.is_nothing(),
        "owl:Thing must not satisfy is_nothing()"
    );
}

/// `Class::nothing()` must not satisfy `is_thing()`.
#[kani::proof]
fn class_nothing_not_thing() {
    let nothing = Class::nothing();
    assert!(
        !nothing.is_thing(),
        "owl:Nothing must not satisfy is_thing()"
    );
}

/// `Class::new(iri)` must preserve the provided IRI.
#[kani::proof]
fn class_new_preserves_iri() {
    let iri = IRI::new("http://example.org/MyClass");
    let cls = Class::new(iri.clone());
    assert_eq!(cls.iri, iri, "Class::new must preserve the IRI");
}

// ── ClassExpression ───────────────────────────────────────────────────────────

/// `ClassExpression::Class(c)` wraps without data loss.
#[kani::proof]
fn class_expr_class_wraps_correctly() {
    let cls = Class::thing();
    let expr = ClassExpression::Class(cls.clone());
    match expr {
        ClassExpression::Class(inner) => assert_eq!(inner, cls),
        _ => panic!("ClassExpression::Class must wrap the given Class"),
    }
}

/// The `ObjectComplementOf` variant holds exactly the inner expression.
#[kani::proof]
fn class_expr_complement_round_trip() {
    let inner = ClassExpression::Class(Class::thing());
    let complement = ClassExpression::ObjectComplementOf(Box::new(inner.clone()));
    match complement {
        ClassExpression::ObjectComplementOf(boxed) => {
            assert_eq!(*boxed, inner, "complement inner must match original");
        }
        _ => panic!("variant must be ObjectComplementOf"),
    }
}

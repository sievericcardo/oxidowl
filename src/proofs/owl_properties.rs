//! Kani harnesses for OWL 2 property characteristics.
//!
//! Covers:
//! - [`ObjectPropertyCharacteristics`]: consistency rules mandated by OWL 2
//!   Direct Semantics (W3C REC owl2-syntax §9):
//!   * SymmetricObjectProperty and AsymmetricObjectProperty are mutually exclusive.
//!   * ReflexiveObjectProperty and IrreflexiveObjectProperty are mutually exclusive.
//!   * FunctionalObjectProperty and InverseFunctionalObjectProperty are mutually exclusive.
//! - Default state: all characteristics start as `false`.
//! - [`ObjectProperty`] top/bottom distinctness.
//! - [`DataPropertyCharacteristics`]: functional flag round-trip.

use crate::ontology::ObjectProperty;
use crate::ontology::properties::{DataPropertyCharacteristics, ObjectPropertyCharacteristics};

// ── Default state ─────────────────────────────────────────────────────────────

/// `ObjectPropertyCharacteristics::new()` must initialise every flag to `false`.
#[kani::proof]
fn prop_characteristics_default_all_false() {
    let ch = ObjectPropertyCharacteristics::new();
    assert!(!ch.functional, "functional must default to false");
    assert!(
        !ch.inverse_functional,
        "inverse_functional must default to false"
    );
    assert!(!ch.symmetric, "symmetric must default to false");
    assert!(!ch.asymmetric, "asymmetric must default to false");
    assert!(!ch.reflexive, "reflexive must default to false");
    assert!(!ch.irreflexive, "irreflexive must default to false");
    assert!(!ch.transitive, "transitive must default to false");
}

/// `DataPropertyCharacteristics::new()` must initialise `functional` to `false`.
#[kani::proof]
fn data_prop_characteristics_default_false() {
    let ch = DataPropertyCharacteristics::new();
    assert!(
        !ch.functional,
        "data property functional must default to false"
    );
}

// ── Single-flag set round-trips ───────────────────────────────────────────────

/// Setting `functional = true` is visible via the public field.
#[kani::proof]
fn prop_set_functional_preserved() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_functional(true);
    assert!(
        ch.functional,
        "functional flag must be set after set_functional(true)"
    );
}

/// Setting `symmetric = true` is visible via the public field.
#[kani::proof]
fn prop_set_symmetric_preserved() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_symmetric(true);
    assert!(
        ch.symmetric,
        "symmetric flag must be set after set_symmetric(true)"
    );
}

/// Setting `transitive = true` is visible via the public field.
#[kani::proof]
fn prop_set_transitive_preserved() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_transitive(true);
    assert!(
        ch.transitive,
        "transitive flag must be set after set_transitive(true)"
    );
}

/// Setting `reflexive = true` is visible via the public field.
#[kani::proof]
fn prop_set_reflexive_preserved() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_reflexive(true);
    assert!(
        ch.reflexive,
        "reflexive flag must be set after set_reflexive(true)"
    );
}

// ── Consistency invariants (OWL 2 §9) ────────────────────────────────────────

/// A property with only `functional = true` must be consistent.
#[kani::proof]
fn prop_functional_only_is_consistent() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_functional(true);
    assert!(
        ch.is_consistent(),
        "functional-only property must be consistent"
    );
}

/// A property with only `symmetric = true` must be consistent.
#[kani::proof]
fn prop_symmetric_only_is_consistent() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_symmetric(true);
    assert!(
        ch.is_consistent(),
        "symmetric-only property must be consistent"
    );
}

/// A property with only `transitive = true` must be consistent.
#[kani::proof]
fn prop_transitive_only_is_consistent() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_transitive(true);
    assert!(
        ch.is_consistent(),
        "transitive-only property must be consistent"
    );
}

/// A property that is simultaneously symmetric AND asymmetric violates OWL 2
/// semantics (no non-empty extension can satisfy both simultaneously).
/// `is_consistent()` must return `false`.
#[kani::proof]
fn prop_symmetric_and_asymmetric_is_inconsistent() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_symmetric(true);
    ch.set_asymmetric(true);
    assert!(
        !ch.is_consistent(),
        "symmetric + asymmetric must be inconsistent per OWL 2 semantics"
    );
}

/// A property that is simultaneously reflexive AND irreflexive is inconsistent.
#[kani::proof]
fn prop_reflexive_and_irreflexive_is_inconsistent() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_reflexive(true);
    ch.set_irreflexive(true);
    assert!(
        !ch.is_consistent(),
        "reflexive + irreflexive must be inconsistent per OWL 2 semantics"
    );
}

/// A property marked both functional and inverse-functional is inconsistent.
#[kani::proof]
fn prop_functional_and_inverse_functional_is_inconsistent() {
    let mut ch = ObjectPropertyCharacteristics::new();
    ch.set_functional(true);
    ch.set_inverse_functional(true);
    assert!(
        !ch.is_consistent(),
        "functional + inverse-functional must be inconsistent per OWL 2 semantics"
    );
}

/// A freshly-constructed set of characteristics (zero flags set) must be consistent.
#[kani::proof]
fn prop_default_characteristics_is_consistent() {
    let ch = ObjectPropertyCharacteristics::new();
    assert!(
        ch.is_consistent(),
        "a property with no characteristics set must be consistent"
    );
}

// ── ObjectProperty top / bottom distinctness ─────────────────────────────────

/// `owl:topObjectProperty` and `owl:bottomObjectProperty` must be distinct.
#[kani::proof]
fn prop_top_bottom_distinct() {
    let top = ObjectProperty::top();
    let bottom = ObjectProperty::bottom();
    assert_ne!(
        top, bottom,
        "owl:topObjectProperty != owl:bottomObjectProperty"
    );
}

/// `ObjectProperty::top()` IRI must be the OWL 2 top role IRI.
#[kani::proof]
fn prop_top_iri_is_owl_top() {
    let top = ObjectProperty::top();
    assert_eq!(
        top.iri.as_str(),
        "http://www.w3.org/2002/07/owl#topObjectProperty",
        "top object property must have the canonical OWL IRI"
    );
}

/// `ObjectProperty::bottom()` IRI must be the OWL 2 bottom role IRI.
#[kani::proof]
fn prop_bottom_iri_is_owl_bottom() {
    let bottom = ObjectProperty::bottom();
    assert_eq!(
        bottom.iri.as_str(),
        "http://www.w3.org/2002/07/owl#bottomObjectProperty",
        "bottom object property must have the canonical OWL IRI"
    );
}

/// A round-trip through `ObjectProperty::new(iri)` preserves the IRI.
/// `new` returns `Result<Self>`, which must always be `Ok` for a well-formed IRI.
#[kani::proof]
fn prop_new_preserves_iri() {
    let iri = crate::ontology::IRI::new("http://example.org/hasParent");
    let prop =
        ObjectProperty::new(iri.clone()).expect("ObjectProperty::new with valid IRI must succeed");
    assert_eq!(prop.iri, iri, "ObjectProperty::new must preserve the IRI");
}

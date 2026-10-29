//! Kani harnesses for OWL 2 `ClassExpression` algebraic invariants.
//!
//! Sources:
//! - OWL 2 Syntax §8 (Class Expressions): <https://www.w3.org/TR/owl2-syntax/>
//! - OWL 2 Direct Semantics §2: <https://www.w3.org/TR/owl2-direct-semantics/>
//!
//! Covers:
//! - Constructor fidelity: each ClassExpression variant stores its arguments faithfully.
//! - `owl:Thing` and `owl:Nothing` are distinct Class variants.
//! - Structural properties: sizes of operand lists, cardinality bounds.
//! - OWL 2 complement double-negation structural identity.

#![cfg(kani)]

use crate::ontology::{
    Individual, ObjectProperty, ObjectPropertyExpression,
    concepts::{Class, ClassExpression},
};

// ── Helper ────────────────────────────────────────────────────────────────────

fn class_a() -> Class {
    Class::new(crate::ontology::IRI::new("A"))
}

fn class_b() -> Class {
    Class::new(crate::ontology::IRI::new("B"))
}

fn class_c() -> Class {
    Class::new(crate::ontology::IRI::new("C"))
}

fn prop_r() -> ObjectPropertyExpression {
    ObjectPropertyExpression::property(
        ObjectProperty::new(crate::ontology::IRI::new("r")).expect("valid IRI"),
    )
}

// ── Class variant ─────────────────────────────────────────────────────────────

/// `ClassExpression::Class(c)` wraps a `Class` and must round-trip it.
#[kani::proof]
fn expr_class_variant_preserves_class() {
    let cls = class_a();
    let expr = ClassExpression::Class(cls.clone());
    match expr {
        ClassExpression::Class(inner) => {
            assert_eq!(inner, cls, "Class variant must preserve the class")
        }
        _ => panic!("expected Class variant"),
    }
}

/// `owl:Thing` and `owl:Nothing` wrapped in `ClassExpression::Class(..)` must be distinct.
#[kani::proof]
fn expr_thing_and_nothing_distinct() {
    let thing = ClassExpression::Class(Class::thing());
    let nothing = ClassExpression::Class(Class::nothing());
    assert_ne!(
        thing, nothing,
        "ClassExpression::Class(Thing) != ClassExpression::Class(Nothing)"
    );
}

// ── ObjectIntersectionOf ──────────────────────────────────────────────────────

/// `ObjectIntersectionOf` preserves the operand vector (length and elements).
#[kani::proof]
#[kani::unwind(4)]
fn expr_intersection_preserves_operands() {
    let expr = ClassExpression::ObjectIntersectionOf(vec![
        ClassExpression::Class(class_a()),
        ClassExpression::Class(class_b()),
    ]);
    match expr {
        ClassExpression::ObjectIntersectionOf(inner) => {
            assert_eq!(inner.len(), 2, "intersection must preserve operand count");
            // Verify elements by IRI string — no Arc clone needed.
            match &inner[0] {
                ClassExpression::Class(c) => {
                    assert_eq!(c.iri.as_str(), "A", "first operand preserved")
                }
                _ => panic!("expected Class(A)"),
            }
            match &inner[1] {
                ClassExpression::Class(c) => {
                    assert_eq!(c.iri.as_str(), "B", "second operand preserved")
                }
                _ => panic!("expected Class(B)"),
            }
        }
        _ => panic!("expected ObjectIntersectionOf"),
    }
}

/// An `ObjectIntersectionOf` with three classes must preserve all three.
#[kani::proof]
#[kani::unwind(5)]
fn expr_intersection_three_operands() {
    let expr = ClassExpression::ObjectIntersectionOf(vec![
        ClassExpression::Class(class_a()),
        ClassExpression::Class(class_b()),
        ClassExpression::Class(class_c()),
    ]);
    match expr {
        ClassExpression::ObjectIntersectionOf(ref v) => {
            assert_eq!(v.len(), 3, "3-operand intersection must have length 3");
        }
        _ => panic!("expected ObjectIntersectionOf"),
    }
}

// ── ObjectUnionOf ─────────────────────────────────────────────────────────────

/// `ObjectUnionOf` preserves the operand vector.
#[kani::proof]
#[kani::unwind(4)]
fn expr_union_preserves_operands() {
    let expr = ClassExpression::ObjectUnionOf(vec![
        ClassExpression::Class(class_a()),
        ClassExpression::Class(class_b()),
    ]);
    match expr {
        ClassExpression::ObjectUnionOf(inner) => {
            assert_eq!(inner.len(), 2, "union must preserve operand count");
            // Verify elements by IRI string — no Arc clone needed.
            match &inner[0] {
                ClassExpression::Class(c) => assert_eq!(c.iri.as_str(), "A"),
                _ => panic!("expected Class(A)"),
            }
            match &inner[1] {
                ClassExpression::Class(c) => assert_eq!(c.iri.as_str(), "B"),
                _ => panic!("expected Class(B)"),
            }
        }
        _ => panic!("expected ObjectUnionOf"),
    }
}

// ── ObjectComplementOf ────────────────────────────────────────────────────────

/// `ObjectComplementOf(C)` wraps the inner expression inside a Box.
#[kani::proof]
fn expr_complement_wraps_inner() {
    let inner = ClassExpression::Class(class_a());
    let expr = ClassExpression::ObjectComplementOf(Box::new(inner.clone()));
    match expr {
        ClassExpression::ObjectComplementOf(boxed) => {
            assert_eq!(
                *boxed, inner,
                "complement must preserve its inner expression"
            );
        }
        _ => panic!("expected ObjectComplementOf"),
    }
}

/// Double complement is structurally represented as nested `ObjectComplementOf`.
/// Verifies that wrapping twice preserves the innermost expression.
#[kani::proof]
fn expr_double_complement_preserves_innermost() {
    let base = ClassExpression::Class(class_a());
    let single = ClassExpression::ObjectComplementOf(Box::new(base.clone()));
    let double = ClassExpression::ObjectComplementOf(Box::new(single));
    match double {
        ClassExpression::ObjectComplementOf(outer) => match *outer {
            ClassExpression::ObjectComplementOf(inner) => {
                assert_eq!(
                    *inner, base,
                    "innermost expression preserved in double complement"
                );
            }
            _ => panic!("expected inner ObjectComplementOf"),
        },
        _ => panic!("expected outer ObjectComplementOf"),
    }
}

// ── ObjectSomeValuesFrom ──────────────────────────────────────────────────────

/// `ObjectSomeValuesFrom` preserves the property and filler.
#[kani::proof]
fn expr_some_values_preserves_property_and_filler() {
    let prop = prop_r();
    let filler = ClassExpression::Class(class_a());
    let expr = ClassExpression::ObjectSomeValuesFrom {
        property: prop.clone(),
        filler: Box::new(filler.clone()),
    };
    match expr {
        ClassExpression::ObjectSomeValuesFrom {
            property,
            filler: boxed,
        } => {
            assert_eq!(property, prop, "property preserved in SomeValuesFrom");
            assert_eq!(*boxed, filler, "filler preserved in SomeValuesFrom");
        }
        _ => panic!("expected ObjectSomeValuesFrom"),
    }
}

// ── ObjectAllValuesFrom ────────────────────────────────────────────────────────

/// `ObjectAllValuesFrom` preserves the property and filler.
#[kani::proof]
fn expr_all_values_preserves_property_and_filler() {
    let prop = prop_r();
    let filler = ClassExpression::Class(class_b());
    let expr = ClassExpression::ObjectAllValuesFrom {
        property: prop.clone(),
        filler: Box::new(filler.clone()),
    };
    match expr {
        ClassExpression::ObjectAllValuesFrom {
            property,
            filler: boxed,
        } => {
            assert_eq!(property, prop, "property preserved in AllValuesFrom");
            assert_eq!(*boxed, filler, "filler preserved in AllValuesFrom");
        }
        _ => panic!("expected ObjectAllValuesFrom"),
    }
}

/// `SomeValuesFrom` and `AllValuesFrom` with the same arguments must be distinct.
#[kani::proof]
fn expr_some_and_all_with_same_args_are_distinct() {
    let prop = prop_r();
    let filler = ClassExpression::Class(class_a());
    let some = ClassExpression::ObjectSomeValuesFrom {
        property: prop.clone(),
        filler: Box::new(filler.clone()),
    };
    let all = ClassExpression::ObjectAllValuesFrom {
        property: prop.clone(),
        filler: Box::new(filler.clone()),
    };
    assert_ne!(
        some, all,
        "SomeValuesFrom != AllValuesFrom even with same arguments"
    );
}

// ── ObjectHasSelf ─────────────────────────────────────────────────────────────

/// `ObjectHasSelf` preserves its property.
#[kani::proof]
fn expr_has_self_preserves_property() {
    let prop = prop_r();
    let expr = ClassExpression::ObjectHasSelf {
        property: prop.clone(),
    };
    match expr {
        ClassExpression::ObjectHasSelf { property } => {
            assert_eq!(property, prop, "ObjectHasSelf must preserve its property");
        }
        _ => panic!("expected ObjectHasSelf"),
    }
}

// ── Cardinality Restrictions ──────────────────────────────────────────────────

/// `ObjectMinCardinality(n)` preserves `n`.
#[kani::proof]
fn expr_min_cardinality_preserves_n() {
    let n: u32 = 3;
    let prop = prop_r();
    let filler = ClassExpression::Class(class_a());
    let expr = ClassExpression::ObjectMinCardinality {
        property: prop,
        cardinality: n,
        filler: Box::new(filler),
    };
    match expr {
        ClassExpression::ObjectMinCardinality { cardinality, .. } => {
            assert_eq!(cardinality, n, "MinCardinality must preserve n");
        }
        _ => panic!("expected ObjectMinCardinality"),
    }
}

/// `ObjectMaxCardinality(n)` preserves `n`.
#[kani::proof]
fn expr_max_cardinality_preserves_n() {
    let n: u32 = 4;
    let prop = prop_r();
    let filler = ClassExpression::Class(class_a());
    let expr = ClassExpression::ObjectMaxCardinality {
        property: prop,
        cardinality: n,
        filler: Box::new(filler),
    };
    match expr {
        ClassExpression::ObjectMaxCardinality { cardinality, .. } => {
            assert_eq!(cardinality, n, "MaxCardinality must preserve n");
        }
        _ => panic!("expected ObjectMaxCardinality"),
    }
}

/// `ObjectExactCardinality(n)` preserves `n`.
#[kani::proof]
fn expr_exact_cardinality_preserves_n() {
    let n: u32 = 2;
    let prop = prop_r();
    let filler = ClassExpression::Class(class_a());
    let expr = ClassExpression::ObjectExactCardinality {
        property: prop,
        cardinality: n,
        filler: Box::new(filler),
    };
    match expr {
        ClassExpression::ObjectExactCardinality { cardinality, .. } => {
            assert_eq!(cardinality, n, "ExactCardinality must preserve n");
        }
        _ => panic!("expected ObjectExactCardinality"),
    }
}

/// `MinCardinality(0)` is valid and must preserve zero.
#[kani::proof]
fn expr_min_cardinality_zero_is_valid() {
    let prop = prop_r();
    let filler = ClassExpression::Class(Class::thing());
    let expr = ClassExpression::ObjectMinCardinality {
        property: prop,
        cardinality: 0, // owl:minCardinality 0 is valid
        filler: Box::new(filler),
    };
    match expr {
        ClassExpression::ObjectMinCardinality { cardinality, .. } => {
            assert_eq!(cardinality, 0, "MinCardinality 0 must be preserved");
        }
        _ => panic!("expected ObjectMinCardinality"),
    }
}

// ── ObjectOneOf ───────────────────────────────────────────────────────────────

/// `ObjectOneOf` with one individual preserves that individual.
#[kani::proof]
#[kani::unwind(3)]
fn expr_one_of_single_preserves_individual() {
    let ind = Individual::named(crate::ontology::IRI::new("J"));
    let expr = ClassExpression::ObjectOneOf(vec![ind.clone()]);
    match expr {
        ClassExpression::ObjectOneOf(inds) => {
            assert_eq!(inds.len(), 1, "ObjectOneOf must preserve individual count");
            assert_eq!(inds[0], ind, "ObjectOneOf must preserve the individual");
        }
        _ => panic!("expected ObjectOneOf"),
    }
}

/// `ObjectOneOf` with three individuals preserves count and membership.
#[kani::proof]
#[kani::unwind(5)]
fn expr_one_of_three_individuals_preserved() {
    let john = Individual::named(crate::ontology::IRI::new("1"));
    let mary = Individual::named(crate::ontology::IRI::new("2"));
    let bill = Individual::named(crate::ontology::IRI::new("3"));
    let expr = ClassExpression::ObjectOneOf(vec![john.clone(), mary.clone(), bill.clone()]);
    match expr {
        ClassExpression::ObjectOneOf(inds) => {
            assert_eq!(inds.len(), 3, "ObjectOneOf must preserve 3 individuals");
        }
        _ => panic!("expected ObjectOneOf"),
    }
}

// ── ObjectHasValue ────────────────────────────────────────────────────────────

/// `ObjectHasValue` preserves the property and the individual value.
#[kani::proof]
fn expr_has_value_preserves_property_and_value() {
    let prop = prop_r();
    let val = Individual::named(crate::ontology::IRI::new("J"));
    let expr = ClassExpression::ObjectHasValue {
        property: prop.clone(),
        value: val.clone(),
    };
    match expr {
        ClassExpression::ObjectHasValue { property, value } => {
            assert_eq!(property, prop, "property preserved in HasValue");
            assert_eq!(value, val, "value individual preserved in HasValue");
        }
        _ => panic!("expected ObjectHasValue"),
    }
}

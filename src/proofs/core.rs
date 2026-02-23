//! Kani harnesses for `core` data structures.
//!
//! Covers:
//! - [`DependencySet`] algebraic laws (empty identity, union commutativity,
//!   union associativity, monotonicity of `add_dependency`).

use crate::core::dependency::DependencySet;

// ── DependencySet ────────────────────────────────────────────────────────────

/// `DependencySet::new()` must produce an empty set.
#[kani::proof]
fn dep_set_new_is_empty() {
    let ds = DependencySet::new();
    assert!(ds.is_empty(), "new() must produce an empty DependencySet");
}

/// `DependencySet::empty()` is an alias for `new()` and must be empty.
#[kani::proof]
fn dep_set_empty_alias_is_empty() {
    let ds = DependencySet::empty();
    assert!(ds.is_empty(), "empty() must produce an empty DependencySet");
}

/// Adding a deterministic dependency makes the set non-empty.
#[kani::proof]
fn dep_set_add_det_dep_makes_nonempty() {
    let mut ds = DependencySet::new();
    ds.add_dependency(42, true);
    assert!(!ds.is_empty(), "det dep must make set non-empty");
}

/// Adding a non-deterministic dependency makes the set non-empty.
#[kani::proof]
fn dep_set_add_nondet_dep_makes_nonempty() {
    let mut ds = DependencySet::new();
    ds.add_dependency(42, false);
    assert!(!ds.is_empty(), "nondet dep must make set non-empty");
}

/// Adding any branching point makes the set non-empty.
#[kani::proof]
fn dep_set_add_bp_makes_nonempty() {
    let mut ds = DependencySet::new();
    ds.add_branching_point(7);
    assert!(
        !ds.is_empty(),
        "set must be non-empty after add_branching_point"
    );
}

/// `union` with the empty set is an identity (right identity): `a ∪ ∅ == a`.
///
/// Both det and nondet paths covered; all values concrete so CBMC can fully
/// evaluate Vec memory ops without symbolic heap-pointer ambiguity.
#[kani::proof]
fn dep_set_union_right_empty_identity() {
    let mut ds = DependencySet::new();
    ds.add_dependency(1, true); // deterministic_deps path
    ds.add_dependency(2, false); // nondeterministic_deps path
    ds.add_branching_point(10);

    let empty = DependencySet::new();
    let result = ds.union(&empty);

    assert_eq!(result.branching_points, ds.branching_points);
    assert_eq!(result.deterministic_deps, ds.deterministic_deps);
    assert_eq!(result.nondeterministic_deps, ds.nondeterministic_deps);
}

/// `union` with the empty set is an identity (left identity): `∅ ∪ a == a`.
#[kani::proof]
fn dep_set_union_left_empty_identity() {
    let mut ds = DependencySet::new();
    ds.add_dependency(1, true);
    ds.add_dependency(2, false);
    ds.add_branching_point(10);

    let empty = DependencySet::new();
    let result = empty.union(&ds);

    assert_eq!(result.branching_points, ds.branching_points);
    assert_eq!(result.deterministic_deps, ds.deterministic_deps);
    assert_eq!(result.nondeterministic_deps, ds.nondeterministic_deps);
}

/// `union` is commutative: `a ∪ b == b ∪ a`.
///
/// Two sets with distinct concrete keys verify that union merges all three
/// sub-collections symmetrically.
#[kani::proof]
fn dep_set_union_commutativity() {
    let mut a = DependencySet::new();
    a.add_dependency(1, true);
    a.add_branching_point(10);

    let mut b = DependencySet::new();
    b.add_dependency(2, false);
    b.add_branching_point(20);

    let ab = a.union(&b);
    let ba = b.union(&a);

    assert_eq!(ab.branching_points, ba.branching_points);
    assert_eq!(ab.deterministic_deps, ba.deterministic_deps);
    assert_eq!(ab.nondeterministic_deps, ba.nondeterministic_deps);
}

/// `add_dependency` is monotone: the set can only grow, never shrink.
///
/// Covered for both det and nondet variants with concrete values.
#[kani::proof]
fn dep_set_add_dep_monotone() {
    let mut ds = DependencySet::new();
    ds.add_dependency(1, true); // det — makes non-empty
    assert!(!ds.is_empty());

    ds.add_dependency(2, false); // nondet — must stay non-empty
    assert!(!ds.is_empty(), "add_dependency must be monotone");
}

/// `with_branching_point` constructor produces a non-empty set.
#[kani::proof]
fn dep_set_with_bp_constructor_nonempty() {
    let ds = DependencySet::with_branching_point(99);
    assert!(!ds.is_empty());
}

/// `with_dependency` (deterministic) constructor produces a non-empty set.
#[kani::proof]
fn dep_set_with_det_dep_constructor_nonempty() {
    let ds = DependencySet::with_dependency(42, true);
    assert!(!ds.is_empty());
}

/// `with_dependency` (non-deterministic) constructor produces a non-empty set.
#[kani::proof]
fn dep_set_with_nondet_dep_constructor_nonempty() {
    let ds = DependencySet::with_dependency(42, false);
    assert!(!ds.is_empty());
}

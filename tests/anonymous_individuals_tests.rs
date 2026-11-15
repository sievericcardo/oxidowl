//! Anonymous-individual and `owl:Thing` / `owl:Nothing` parity tests
//! (final wave B + C).
//!
//! B — anonymous individuals:
//!
//! 1. `signature().individuals` carries every anonymous individual of every
//!    assertion kind under its stable `_:id` key (no ids are minted);
//! 2. `realize` / `get_types` / `get_instances` / `is_instance_of` treat each
//!    anonymous individual as its own individual (they used to collapse onto
//!    a shared `"anonymous"` key, or match `None == None`);
//! 3. `getSameIndividuals` / `getDifferentIndividuals` work with `_:x` keys;
//!
//! C — `owl:Thing` / `owl:Nothing`:
//!
//! 4. `getInstances(owl:Thing)` returns every named **and** anonymous
//!    individual of the signature (including property-assertion-only ones),
//!    `getInstances(owl:Nothing)` is empty;
//! 5. `getTypes(i)` always contains `owl:Thing` and never `owl:Nothing`;
//! 6. `isInstance(i, owl:Thing)` is true, `isInstance(i, owl:Nothing)` false.

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::ontology::*;
use oxidowl::{
    NodeSet, OWLReasoner, OWLReasonerConfiguration, Reasoner, ReasonerConfig, ReasonerFactory,
    TableauReasonerFactory,
};
use std::sync::Arc;

const EX: &str = "http://ex.org/";
const OWL: &str = "http://www.w3.org/2002/07/owl#";

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(std::sync::RwLock::new(o))
}

fn tableau_reasoner(ont: Ontology) -> Box<dyn OWLReasoner> {
    let onto = onto_ref(ont);
    TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .expect("tableau reasoner must be created")
}

fn core_reasoner(ont: Ontology) -> Reasoner {
    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    reasoner.load_ontology(ont).expect("load ontology");
    reasoner
}

/// Sorted `Display` keys of an instance `NodeSet` (`IRI` or `_:id`).
fn instance_keys(nodes: NodeSet<Individual>) -> Vec<String> {
    let mut keys: Vec<String> = nodes
        .get_flattened()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.sort();
    keys
}

/// Sorted `Display` keys of a `Node` of individuals.
fn node_keys(node: oxidowl::Node<Individual>) -> Vec<String> {
    let mut keys: Vec<String> = node
        .get_entities()
        .iter()
        .map(ToString::to_string)
        .collect();
    keys.sort();
    keys
}

/// Sorted IRI keys of a type `NodeSet`.
fn type_keys(nodes: NodeSet<ClassExpression>) -> Vec<String> {
    let mut keys: Vec<String> = nodes
        .get_flattened()
        .iter()
        .map(|c| match c {
            ClassExpression::Class(cls) => cls.iri.to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    keys.sort();
    keys
}

/// Sorted `Display` keys of a core `get_types` answer.
fn core_type_keys(types: Vec<ClassExpression>) -> Vec<String> {
    let mut keys: Vec<String> = types
        .iter()
        .map(|c| match c {
            ClassExpression::Class(cls) => cls.iri.to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    keys.sort();
    keys
}

fn thing() -> String {
    format!("{OWL}Thing")
}

fn nothing() -> String {
    format!("{OWL}Nothing")
}

// ══════════════════════════════════════════════════════════════════════════════
// B — signature
// ══════════════════════════════════════════════════════════════════════════════

/// Every assertion kind contributes its anonymous individuals to
/// `signature().individuals` under their own stable key.
#[test]
fn signature_collects_anonymous_individuals_from_every_assertion_kind() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let r = df.obj_prop(format!("{EX}R"));
    let dp = df.data_prop(format!("{EX}dp"));

    let x = df.anon();
    let y = df.anon();
    let z = df.anon();
    let w = df.anon();
    let named = df.named(format!("{EX}n"));

    let mut ont = df.build_ontology(vec![
        // ClassAssertion
        df.class_assertion(a.clone(), x.clone()),
        // ObjectPropertyAssertion (both endpoints)
        df.object_property_assertion(r.clone(), x.clone(), y.clone()),
        // DataPropertyAssertion
        df.data_property_assertion(dp.clone(), y.clone(), df.int_literal(3)),
        // SameIndividual / DifferentIndividuals
        df.same_individual(vec![x.clone(), z.clone()]),
        df.different_individuals(vec![z.clone(), w.clone()]),
        // the named control individual
        df.class_assertion(a.clone(), named.clone()),
    ]);
    df.auto_declare(&mut ont);

    let signature = ont.signature().expect("signature");
    for ind in [&x, &y, &z, &w] {
        assert!(
            signature.individuals.contains(ind),
            "signature().individuals must contain {ind:?} (stable id, no re-minting); \
             got {:?}",
            signature.individuals
        );
    }
    assert!(
        signature.individuals.contains(&named),
        "the named individual must stay in the signature"
    );

    // Stable, distinct `_:id` keys — four anonymous individuals, four keys.
    let anon_keys: Vec<String> = signature
        .individuals
        .iter()
        .filter(|i| !i.is_named())
        .map(ToString::to_string)
        .collect();
    let mut unique = anon_keys.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(
        unique.len(),
        4,
        "four distinct anonymous keys expected, got {anon_keys:?}"
    );
    for ind in [&x, &y, &z, &w] {
        assert!(
            anon_keys.contains(&ind.to_string()),
            "the Display key of {ind:?} must round-trip through the signature"
        );
    }

    // Negative control: `individuals()` is the *named*, IRI-keyed view — it
    // must never mint an IRI-shaped key for an anonymous individual.
    for (iri, individual) in ont.individuals() {
        assert!(
            individual.is_named(),
            "individuals() must stay named-only, got {iri:?} → {individual:?}"
        );
    }
}

// ══════════════════════════════════════════════════════════════════════════════
// B — realisation / types
// ══════════════════════════════════════════════════════════════════════════════

/// `A(_:x)`, `B(_:y)`, `A ⊑ C` ⇒ each anonymous individual realises with
/// *its own* asserted types plus the inferred superclasses — never the other
/// individual's types.
#[test]
fn anonymous_individuals_realise_with_their_own_types() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));
    let c = df.class_ce(format!("{EX}C"));

    let x = df.anon();
    let y = df.anon();

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), x.clone()),
        df.class_assertion(b.clone(), y.clone()),
        df.sub_class_of(a.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);

    let core = core_reasoner(ont.clone());

    let x_types = core_type_keys(core.get_types(&x, false).expect("core get_types"));
    assert!(
        x_types.contains(&format!("{EX}A")) && x_types.contains(&format!("{EX}C")),
        "_:x must realise as A and (through A ⊑ C) C; got {x_types:?}"
    );
    assert!(
        x_types.contains(&thing()),
        "every individual has owl:Thing; got {x_types:?}"
    );
    assert!(
        !x_types.contains(&format!("{EX}B")),
        "negative control: _:x must not receive _:y's type B; got {x_types:?}"
    );
    assert!(
        !x_types.contains(&nothing()),
        "owl:Nothing must never be a type; got {x_types:?}"
    );

    let y_types = core_type_keys(core.get_types(&y, false).expect("core get_types"));
    assert!(
        y_types.contains(&format!("{EX}B")) && y_types.contains(&thing()),
        "_:y must realise as B (+ owl:Thing); got {y_types:?}"
    );
    assert!(
        !y_types.contains(&format!("{EX}A")) && !y_types.contains(&format!("{EX}C")),
        "negative control: _:y must not receive _:x's types; got {y_types:?}"
    );

    // Direct types: A (not its superclass C, not owl:Thing).
    let x_direct = core_type_keys(core.get_types(&x, true).expect("core get_types(direct)"));
    assert!(
        x_direct.contains(&format!("{EX}A")),
        "A must be a direct type of _:x; got {x_direct:?}"
    );
    assert!(
        !x_direct.contains(&format!("{EX}C")),
        "C is a proper supertype of _:x; got {x_direct:?}"
    );

    // OWL API surface answers identically.
    let owl = tableau_reasoner(ont);
    let x_owl = type_keys(owl.get_types(&x, false).expect("OWLAPI get_types"));
    assert!(
        x_owl.contains(&format!("{EX}A"))
            && x_owl.contains(&format!("{EX}C"))
            && x_owl.contains(&thing()),
        "OWL API getTypes(_:x) must match the core answer; got {x_owl:?}"
    );
    let x_owl_direct = type_keys(owl.get_types(&x, true).expect("OWLAPI get_types(direct)"));
    assert!(
        x_owl_direct.contains(&format!("{EX}A"))
            && !x_owl_direct.contains(&format!("{EX}C"))
            && !x_owl_direct.contains(&thing()),
        "OWL API direct types of _:x must be A only; got {x_owl_direct:?}"
    );
    assert!(
        !type_keys(owl.get_types(&y, false).expect("OWLAPI get_types")).contains(&format!("{EX}A")),
        "negative control: OWL API getTypes(_:y) must not contain A"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// B — instances
// ══════════════════════════════════════════════════════════════════════════════

/// `A(_:x)`, `B(_:y)`, `A ⊑ C`, `B ⊑ C` ⇒ `getInstances(C)` holds both
/// anonymous individuals as two *distinct* stable keys; an unrelated
/// individual is a negative control.
#[test]
fn get_instances_returns_stable_distinct_anonymous_keys() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));
    let c = df.class_ce(format!("{EX}C"));
    let e = df.class_ce(format!("{EX}E"));
    let f = df.class_ce(format!("{EX}F"));

    let x = df.anon();
    let y = df.anon();
    let z = df.anon();

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), x.clone()),
        df.class_assertion(b.clone(), y.clone()),
        df.class_assertion(e.clone(), z.clone()),
        df.sub_class_of(a.clone(), c.clone()),
        df.sub_class_of(b.clone(), c.clone()),
        df.sub_class_of(e.clone(), f.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut core = core_reasoner(ont.clone());
    let mut core_c: Vec<String> = core
        .get_instances(&c, false)
        .expect("core get_instances(C)")
        .iter()
        .map(ToString::to_string)
        .collect();
    core_c.sort();
    let mut expected: Vec<String> = vec![x.to_string(), y.to_string()];
    expected.sort();
    assert_eq!(
        core_c, expected,
        "getInstances(C) must hold exactly _:x and _:y as distinct keys"
    );
    assert!(
        !core_c.contains(&z.to_string()),
        "negative control: _:z ∈ E ⊑ F must not leak into C"
    );

    // The OWL API surface answers the same set.
    let owl = tableau_reasoner(ont);
    let owl_c = instance_keys(
        owl.get_instances(&c, false)
            .expect("OWLAPI get_instances(C)"),
    );
    assert_eq!(
        owl_c, expected,
        "OWL API getInstances(C) must hold exactly _:x and _:y"
    );
    let owl_f = instance_keys(
        owl.get_instances(&f, false)
            .expect("OWLAPI get_instances(F)"),
    );
    assert_eq!(
        owl_f,
        vec![z.to_string()],
        "getInstances(F) holds exactly _:z"
    );

    // Membership checks on the anonymous keys.
    assert!(
        core.is_instance_of(&x, &c).expect("is_instance_of(_:x, C)"),
        "is_instance_of(_:x, C) must be true"
    );
    assert!(
        !core.is_instance_of(&z, &c).expect("is_instance_of(_:z, C)"),
        "negative control: is_instance_of(_:z, C) must be false"
    );
    assert!(
        owl.has_type(&x, &c).expect("has_type(_:x, C)"),
        "has_type(_:x, C) must be true"
    );
    assert!(
        !owl.has_type(&z, &c).expect("has_type(_:z, C)"),
        "negative control: has_type(_:z, C) must be false"
    );
}

/// The *same* anonymous individual in two `ClassAssertion`s keeps both types
/// and receives the inferred superclass: `A(_:x)`, `B(_:x)`, `A ⊑ C`, with a
/// `B`-only control `_:y`.
///
/// (A `A ⊓ B ⊑ C` GCI would additionally need complex-left-hand-side
/// instantiation, which this reasoner does not perform for *any* individual —
/// named or anonymous — and is therefore out of this wave's scope.)
#[test]
fn same_anonymous_individual_keeps_both_types_and_the_inferred_supertype() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));
    let c = df.class_ce(format!("{EX}C"));

    let x = df.anon();
    let y = df.anon();

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), x.clone()),
        df.class_assertion(b.clone(), x.clone()),
        df.class_assertion(b.clone(), y.clone()),
        df.sub_class_of(a.clone(), c.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut core = core_reasoner(ont.clone());

    // Both asserted types survive for _:x, plus the inferred C and owl:Thing.
    let x_types = core_type_keys(core.get_types(&x, false).expect("core get_types"));
    for expected in [
        format!("{EX}A"),
        format!("{EX}B"),
        format!("{EX}C"),
        thing(),
    ] {
        assert!(
            x_types.contains(&expected),
            "getTypes(_:x) must contain {expected}; got {x_types:?}"
        );
    }
    assert!(
        !x_types.contains(&nothing()),
        "owl:Nothing must never be a type; got {x_types:?}"
    );

    // Negative control: _:y has only B.
    let y_types = core_type_keys(core.get_types(&y, false).expect("core get_types"));
    assert!(
        y_types.contains(&format!("{EX}B")) && y_types.contains(&thing()),
        "getTypes(_:y) must contain B and owl:Thing; got {y_types:?}"
    );
    assert!(
        !y_types.contains(&format!("{EX}A")) && !y_types.contains(&format!("{EX}C")),
        "negative control: _:y must not receive _:x's types; got {y_types:?}"
    );

    // Direct types of _:x are the two asserted classes (C and owl:Thing are
    // proper supertypes).
    let x_direct = core_type_keys(core.get_types(&x, true).expect("core get_types(direct)"));
    assert!(
        x_direct.contains(&format!("{EX}A"))
            && x_direct.contains(&format!("{EX}B"))
            && !x_direct.contains(&format!("{EX}C"))
            && !x_direct.contains(&thing()),
        "direct types of _:x must be exactly A and B; got {x_direct:?}"
    );

    // Instances of C: only _:x (y has B only).
    let mut instances: Vec<String> = core
        .get_instances(&c, false)
        .expect("core get_instances(C)")
        .iter()
        .map(ToString::to_string)
        .collect();
    instances.sort();
    assert_eq!(
        instances,
        vec![x.to_string()],
        "getInstances(C) must hold exactly the one inferred individual _:x"
    );
    assert!(
        core.is_instance_of(&x, &c).expect("is_instance_of(_:x, C)"),
        "is_instance_of(_:x, C) must be true"
    );
    assert!(
        !core.is_instance_of(&y, &c).expect("is_instance_of(_:y, C)"),
        "negative control: is_instance_of(_:y, C) must be false"
    );

    // The OWL API surface answers the same instance set.
    let owl = tableau_reasoner(ont);
    let owl_c = instance_keys(
        owl.get_instances(&c, false)
            .expect("OWLAPI get_instances(C)"),
    );
    assert_eq!(
        owl_c,
        vec![x.to_string()],
        "OWL API getInstances(C) must hold exactly _:x"
    );
    let owl_x_types = type_keys(owl.get_types(&x, false).expect("OWLAPI get_types"));
    assert!(
        owl_x_types.contains(&format!("{EX}A"))
            && owl_x_types.contains(&format!("{EX}B"))
            && owl_x_types.contains(&format!("{EX}C"))
            && owl_x_types.contains(&thing()),
        "OWL API getTypes(_:x) must match the core answer; got {owl_x_types:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// B — same / different individuals
// ══════════════════════════════════════════════════════════════════════════════

/// `SameIndividual([_:x, _:y])` and `DifferentIndividuals([_:z, _:w])` are
/// answerable with anonymous keys on both sides.
#[test]
fn same_and_different_individuals_support_anonymous_keys() {
    let df = DF::new();
    let x = df.anon();
    let y = df.anon();
    let z = df.anon();
    let w = df.anon();

    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![x.clone(), y.clone()]),
        df.different_individuals(vec![z.clone(), w.clone()]),
    ]);
    df.auto_declare(&mut ont);
    let reasoner = tableau_reasoner(ont);

    let same = node_keys(
        reasoner
            .get_same_individuals(&x)
            .expect("get_same_individuals(_:x)"),
    );
    assert!(
        same.contains(&x.to_string()) && same.contains(&y.to_string()),
        "getSameIndividuals(_:x) must contain _:x and _:y; got {same:?}"
    );
    assert!(
        !same.contains(&z.to_string()),
        "negative control: _:z is not sameAs _:x; got {same:?}"
    );
    assert!(
        reasoner
            .is_same_individual(&x, &y)
            .expect("is_same_individual(_:x, _:y)"),
        "is_same_individual(_:x, _:y) must be true"
    );
    assert!(
        !reasoner
            .is_same_individual(&x, &z)
            .expect("is_same_individual(_:x, _:z)"),
        "negative control: is_same_individual(_:x, _:z) must be false"
    );

    let different = instance_keys(
        reasoner
            .get_different_individuals(&z)
            .expect("get_different_individuals(_:z)"),
    );
    assert!(
        different.contains(&w.to_string()),
        "getDifferentIndividuals(_:z) must contain _:w; got {different:?}"
    );
    assert!(
        !different.contains(&z.to_string()),
        "negative control: an individual is never different from itself; got {different:?}"
    );
    assert!(
        !different.contains(&x.to_string()),
        "negative control: _:x is unrelated to the DifferentIndividuals axiom; got {different:?}"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// C — owl:Thing / owl:Nothing
// ══════════════════════════════════════════════════════════════════════════════

/// `getInstances(owl:Thing)` = every individual of the signature (named with
/// and without a class assertion, anonymous), `getInstances(owl:Nothing)` =
/// ∅; `getTypes` always carries `owl:Thing` and never `owl:Nothing`.
#[test]
fn owl_thing_and_nothing_answer_for_named_and_anonymous_individuals() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let r = df.obj_prop(format!("{EX}R"));

    let m = df.named(format!("{EX}m")); // has a class assertion
    let n = df.named(format!("{EX}n")); // only appears in a property assertion
    let x = df.anon(); // only appears in a property assertion

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), m.clone()),
        df.object_property_assertion(r.clone(), m.clone(), n.clone()),
        df.object_property_assertion(r.clone(), n.clone(), x.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut all: Vec<String> = vec![m.to_string(), n.to_string(), x.to_string()];
    all.sort();

    let mut core = core_reasoner(ont.clone());

    // ── owl:Thing ────────────────────────────────────────────────────────
    let thing_core: Vec<String> = core
        .get_instances(&df.owl_thing(), false)
        .expect("core get_instances(owl:Thing)")
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        thing_core, all,
        "owl:Thing must have every named and anonymous individual as an instance"
    );

    let owl = tableau_reasoner(ont.clone());
    let thing_owl = instance_keys(owl.get_instances(&df.owl_thing(), false).expect("OWLAPI"));
    assert_eq!(
        thing_owl, all,
        "OWL API getInstances(owl:Thing) must return the whole signature"
    );
    // Direct instances of owl:Thing are the individuals with no other type.
    let thing_direct = instance_keys(owl.get_instances(&df.owl_thing(), true).expect("OWLAPI"));
    let mut untyped: Vec<String> = vec![n.to_string(), x.to_string()];
    untyped.sort();
    assert_eq!(
        thing_direct, untyped,
        "only the untyped n and _:x are direct instances of owl:Thing"
    );

    // ── owl:Nothing ─────────────────────────────────────────────────────
    let nothing_core: Vec<String> = core
        .get_instances(&df.owl_nothing(), false)
        .expect("core get_instances(owl:Nothing)")
        .iter()
        .map(ToString::to_string)
        .collect();
    assert!(
        nothing_core.is_empty(),
        "owl:Nothing must have no instances; got {nothing_core:?}"
    );
    assert!(
        instance_keys(owl.get_instances(&df.owl_nothing(), false).expect("OWLAPI")).is_empty(),
        "OWL API getInstances(owl:Nothing) must be empty"
    );

    // ── types ────────────────────────────────────────────────────────────
    for ind in [&m, &n, &x] {
        let types = core_type_keys(core.get_types(ind, false).expect("core get_types"));
        assert!(
            types.contains(&thing()),
            "getTypes({ind}) must contain owl:Thing; got {types:?}"
        );
        assert!(
            !types.contains(&nothing()),
            "getTypes({ind}) must never contain owl:Nothing; got {types:?}"
        );
        let owl_types = type_keys(owl.get_types(ind, false).expect("OWLAPI get_types"));
        assert!(
            owl_types.contains(&thing()) && !owl_types.contains(&nothing()),
            "OWL API getTypes({ind}) must contain owl:Thing only as top; got {owl_types:?}"
        );
    }

    // n has no type at all ⇒ its direct types are exactly {owl:Thing};
    // m's direct types are {A} (owl:Thing is dropped as non-direct).
    let n_direct = type_keys(owl.get_types(&n, true).expect("OWLAPI get_types(direct)"));
    assert_eq!(
        n_direct,
        vec![thing()],
        "the direct types of an untyped individual are exactly owl:Thing"
    );
    let m_direct = type_keys(owl.get_types(&m, true).expect("OWLAPI get_types(direct)"));
    assert_eq!(
        m_direct,
        vec![format!("{EX}A")],
        "owl:Thing must not be a direct type when A is present; got {m_direct:?}"
    );

    // ── membership ───────────────────────────────────────────────────────
    for ind in [&m, &n, &x] {
        assert!(
            core.is_instance_of(ind, &df.owl_thing())
                .expect("is_instance_of"),
            "is_instance_of({ind}, owl:Thing) must be true"
        );
        assert!(
            !core
                .is_instance_of(ind, &df.owl_nothing())
                .expect("is_instance_of"),
            "is_instance_of({ind}, owl:Nothing) must be false"
        );
        assert!(
            owl.has_type(ind, &df.owl_thing()).expect("has_type"),
            "has_type({ind}, owl:Thing) must be true"
        );
        assert!(
            !owl.has_type(ind, &df.owl_nothing()).expect("has_type"),
            "has_type({ind}, owl:Nothing) must be false"
        );
    }

    // Negative control: the *instance query* universe is the signature — a
    // fresh entity never shows up in `getInstances(owl:Thing)` (its
    // membership is trivially true: `owl:Thing` contains everything), and it
    // is still not an instance of `owl:Nothing`.
    let stranger = df.named(format!("{EX}stranger"));
    assert!(
        !thing_core.contains(&stranger.to_string()),
        "a fresh entity must not appear in getInstances(owl:Thing); got {thing_core:?}"
    );
    assert!(
        !core
            .is_instance_of(&stranger, &df.owl_nothing())
            .expect("is_instance_of"),
        "a fresh entity is never an instance of owl:Nothing"
    );
}

// ══════════════════════════════════════════════════════════════════════════════
// B — snapshot-backed evaluation for anonymous keys
// ══════════════════════════════════════════════════════════════════════════════

/// `SameIndividual([a, _:x])`, `A(a)` — no `A(_:x)` assertion. The tableau
/// merges both nominals onto one node, so:
///
/// * `is_instance_of(_:x, A)` answers from the **snapshot** (`node_of(_:x)`
///   must resolve to the merged nominal label `_:x`, not to `"anonymous"`),
/// * `getInstances(A)` returns *both* partners (the realisation map keys them
///   separately and must be expanded through the same-as component).
#[test]
fn snapshot_derived_membership_works_for_merged_anonymous_individuals() {
    let df = DF::new();
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));

    let named = df.named(format!("{EX}a"));
    let x = df.anon();
    let w = df.anon();

    let mut ont = df.build_ontology(vec![
        df.class_assertion(a.clone(), named.clone()),
        df.same_individual(vec![named.clone(), x.clone()]),
        // negative control: an unrelated anonymous individual
        df.class_assertion(b.clone(), w.clone()),
    ]);
    df.auto_declare(&mut ont);

    let mut core = core_reasoner(ont.clone());

    // The structural checker never sees `A(_:x)` — only the completed tableau
    // (merged nominal node) proves it.
    assert!(
        core.is_instance_of(&x, &a).expect("is_instance_of(_:x, A)"),
        "is_instance_of(_:x, A) must follow the merged nominal node"
    );
    let mut core_instances: Vec<String> = core
        .get_instances(&a, false)
        .expect("core get_instances(A)")
        .iter()
        .map(ToString::to_string)
        .collect();
    core_instances.sort();
    let mut both: Vec<String> = vec![named.to_string(), x.to_string()];
    both.sort();
    assert_eq!(
        core_instances, both,
        "getInstances(A) must hold both same-as partners"
    );
    assert!(
        !core_instances.contains(&w.to_string()),
        "negative control: _:w ∈ B must not leak into A"
    );

    // OWL API surface: the realisation map only knows the named partner, so
    // the same-as expansion must supply `_:x`.
    let owl = tableau_reasoner(ont);
    let owl_instances = instance_keys(
        owl.get_instances(&a, false)
            .expect("OWLAPI get_instances(A)"),
    );
    assert_eq!(
        owl_instances, both,
        "OWL API getInstances(A) must hold both same-as partners"
    );
    assert!(
        owl.has_type(&x, &a).expect("has_type(_:x, A)"),
        "has_type(_:x, A) must follow the merged nominal node"
    );
    assert!(
        !owl.has_type(&w, &a).expect("has_type(_:w, A)"),
        "negative control: has_type(_:w, A) must be false"
    );
}

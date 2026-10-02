//! Workstream B2 — reasoner API parity: SameAs/DifferentAs equality,
//! `direct` instance filtering, real `get_disjoint_classes`, and the
//! `Node`/`NodeSet` convenience methods.

#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use helpers::*;
use oxidowl::ontology::axioms::*;
use oxidowl::ontology::*;
use oxidowl::{
    IndividualNodeSetPolicy, Node, NodeSet, OWLReasoner, OWLReasonerConfiguration, ReasonerFactory,
    StructuralReasoner, StructuralReasonerFactory, TableauReasonerFactory,
};
use std::collections::HashSet;
use std::sync::Arc;

fn onto_ref(o: Ontology) -> OntologyRef {
    Arc::new(std::sync::RwLock::new(o))
}

// ══════════════════════════════════════════════════════════════════════════════
// SameIndividual / DifferentIndividuals equality
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn reasoner_same_individuals_transitive_closure_structural() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    let c = df.named("http://ex.org/c");
    // SameIndividual(a,b) + SameIndividual(b,c) ⇒ a ≡ b ≡ c
    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![a.clone(), b.clone()]),
        df.same_individual(vec![b.clone(), c.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let reasoner = StructuralReasoner::new(onto_ref(ont));
    let node = reasoner.get_same_individuals(&a).unwrap();
    assert_eq!(node.get_size(), 3, "node must contain a, b and c");
    assert!(node.contains(&a));
    assert!(node.contains(&b));
    assert!(node.contains(&c));
}

#[test]
fn reasoner_same_individuals_transitive_closure_tableau() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    let c = df.named("http://ex.org/c");
    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![a.clone(), b.clone()]),
        df.same_individual(vec![b.clone(), c.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let onto = onto_ref(ont);
    let reasoner = TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    assert!(reasoner.is_consistent().unwrap());
    let node = reasoner.get_same_individuals(&a).unwrap();
    assert!(node.contains(&b));
    assert!(node.contains(&c));
    assert!(node.contains(&a));
}

#[test]
fn reasoner_different_individuals_via_components() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    let c = df.named("http://ex.org/c");
    // SameIndividual(a,b) + DifferentIndividuals(b,c)
    // ⇒ c is different from the whole {a,b} component.
    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![a.clone(), b.clone()]),
        df.different_individuals(vec![b.clone(), c.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let reasoner = StructuralReasoner::new(onto_ref(ont));
    let diff = reasoner.get_different_individuals(&a).unwrap();
    assert!(diff.contains_entity(&c), "a should be different from c");
    assert!(
        !diff.contains_entity(&b),
        "b is sameAs a and must not be different from a"
    );
    // Each node in the result is a sameAs component.
    let flat = diff.get_flattened();
    assert_eq!(flat.len(), 1);
    assert!(flat.contains(&c));
}

#[test]
fn reasoner_same_and_different_conflict() {
    let df = DF::new();
    let a = df.named("http://ex.org/a");
    let b = df.named("http://ex.org/b");
    // SameIndividual(a,b) + DifferentIndividuals(a,b) — inconsistent per OWL 2.
    let mut ont = df.build_ontology(vec![
        df.same_individual(vec![a.clone(), b.clone()]),
        df.different_individuals(vec![a.clone(), b.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let onto = onto_ref(ont.clone());
    let reasoner = TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();

    if !reasoner.is_consistent().unwrap() {
        // The reasoner detected the inconsistency — that is the correct answer.
        return;
    }

    // If the ontology is reported consistent, the reasoner must still return
    // consistent answers: b is sameAs a, so b is not "different" from a.
    let same = reasoner.get_same_individuals(&a).unwrap();
    assert!(same.contains(&b));
    let diff = reasoner.get_different_individuals(&a).unwrap();
    assert!(
        !diff.contains_entity(&b),
        "b is sameAs a, so it must not be returned as different"
    );

    // Structural reasoner always reports consistent and keeps the component view.
    let reasoner = StructuralReasoner::new(onto_ref(ont));
    let same = reasoner.get_same_individuals(&a).unwrap();
    assert!(same.contains(&b));
    let diff = reasoner.get_different_individuals(&a).unwrap();
    assert!(!diff.contains_entity(&b));
}

#[test]
fn reasoner_functional_property_same_individuals() {
    let df = DF::new();
    let p = df.obj_prop("http://ex.org/p");
    let s = df.named("http://ex.org/s");
    let t1 = df.named("http://ex.org/t1");
    let t2 = df.named("http://ex.org/t2");
    // FunctionalObjectProperty(p) + p(s,t1) + p(s,t2) ⇒ t1 ≡ t2
    let mut ont = df.build_ontology(vec![
        df.functional_object_property(p.clone()),
        df.object_property_assertion(p.clone(), s.clone(), t1.clone()),
        df.object_property_assertion(p, s, t2.clone()),
    ]);
    df.auto_declare(&mut ont);

    let reasoner = StructuralReasoner::new(onto_ref(ont));
    let node = reasoner.get_same_individuals(&t1).unwrap();
    assert!(node.contains(&t2), "functional property forces t1 ≡ t2");
}

// ══════════════════════════════════════════════════════════════════════════════
// get_instances with direct flag
// ══════════════════════════════════════════════════════════════════════════════

fn instances_hierarchy_ontology() -> Ontology {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let i = df.named("http://ex.org/i");
    // A ⊑ B, i ∈ A  ⇒  i ∈ A, i ∈ B, and i is a *direct* instance of A only.
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a, b),
        df.class_assertion(df.class_ce("http://ex.org/A"), i),
    ]);
    df.auto_declare(&mut ont);
    ont
}

#[test]
fn reasoner_get_instances_direct_structural() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let i = df.named("http://ex.org/i");

    let reasoner = StructuralReasoner::new(onto_ref(instances_hierarchy_ontology()));

    // direct = false: i is an instance of B via the asserted hierarchy.
    let all = reasoner.get_instances(&b, false).unwrap();
    assert!(
        all.contains_entity(&i),
        "i should be an instance of B (all)"
    );

    // direct = true: i is an instance of the proper subclass A, so not direct in B.
    let direct_b = reasoner.get_instances(&b, true).unwrap();
    assert!(
        !direct_b.contains_entity(&i),
        "i should not be a direct instance of B"
    );

    // i IS a direct instance of A.
    let direct_a = reasoner.get_instances(&a, true).unwrap();
    assert!(direct_a.contains_entity(&i));
}

#[test]
fn reasoner_get_instances_direct_tableau() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let i = df.named("http://ex.org/i");

    let onto = onto_ref(instances_hierarchy_ontology());
    let reasoner = TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();

    let all = reasoner.get_instances(&b, false).unwrap();
    assert!(
        all.contains_entity(&i),
        "i should be an instance of B (all)"
    );

    let direct_b = reasoner.get_instances(&b, true).unwrap();
    assert!(
        !direct_b.contains_entity(&i),
        "i should not be a direct instance of B"
    );

    let direct_a = reasoner.get_instances(&a, true).unwrap();
    assert!(direct_a.contains_entity(&i));
}

#[test]
fn reasoner_get_instances_groups_by_same_as() {
    let df = DF::new();
    let c = df.class_ce("http://ex.org/C");
    let i1 = df.named("http://ex.org/i1");
    let i2 = df.named("http://ex.org/i2");
    let mut ont = df.build_ontology(vec![
        df.class_assertion(c.clone(), i1.clone()),
        df.class_assertion(c.clone(), i2.clone()),
        df.same_individual(vec![i1.clone(), i2.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let config = OWLReasonerConfiguration {
        individual_node_set_policy: IndividualNodeSetPolicy::BySameAs,
        ..OWLReasonerConfiguration::default()
    };
    let onto = onto_ref(ont);
    let reasoner = TableauReasonerFactory
        .create_reasoner(&onto, &config)
        .unwrap();
    let instances = reasoner.get_instances(&c, false).unwrap();
    assert_eq!(instances.get_nodes().len(), 1, "i1 and i2 share one node");
    assert!(instances.contains_entity(&i1));
    assert!(instances.contains_entity(&i2));
}

// ══════════════════════════════════════════════════════════════════════════════
// get_disjoint_classes
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn reasoner_get_disjoint_classes_tableau() {
    let df = DF::new();
    let exclusive = df.class_ce("http://ex.org/ExclusiveClub");
    let member = df.class_ce("http://ex.org/Member");
    let mut ont = df.build_ontology(vec![
        df.disjoint_classes(vec![exclusive.clone(), member.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let onto = onto_ref(ont);
    let reasoner = TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    assert!(reasoner.is_consistent().unwrap());

    let disj = reasoner.get_disjoint_classes(&exclusive).unwrap();
    assert!(
        disj.contains_entity(&member),
        "Member should be disjoint with ExclusiveClub"
    );
    let flat = disj.get_flattened();
    assert!(
        !flat.contains(&exclusive),
        "a class is not disjoint with itself"
    );
}

#[test]
fn reasoner_get_disjoint_classes_structural() {
    let df = DF::new();
    let exclusive = df.class_ce("http://ex.org/ExclusiveClub");
    let member = df.class_ce("http://ex.org/Member");
    let mut ont = df.build_ontology(vec![
        df.disjoint_classes(vec![exclusive.clone(), member.clone()]),
    ]);
    df.auto_declare(&mut ont);

    let reasoner = StructuralReasoner::new(onto_ref(ont));
    let disj = reasoner.get_disjoint_classes(&exclusive).unwrap();
    assert!(disj.contains_entity(&member));
}

#[test]
fn reasoner_get_types_direct() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let i = df.named("http://ex.org/i");
    // A ⊑ B, i ∈ A  ⇒  i's direct types are {A} (the most specific), not B.
    let mut ont = df.build_ontology(vec![
        df.sub_class_of(a.clone(), b.clone()),
        df.class_assertion(a.clone(), i.clone()),
    ]);
    df.auto_declare(&mut ont);

    let onto = onto_ref(ont.clone());
    let reasoner = TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .unwrap();
    let direct = reasoner.get_types(&i, true).unwrap();
    assert!(direct.contains_entity(&a), "A should be a direct type of i");
    assert!(
        !direct.contains_entity(&b),
        "B should not be a direct type of i"
    );

    let reasoner = StructuralReasoner::new(onto_ref(ont));
    let direct = reasoner.get_types(&i, true).unwrap();
    assert!(direct.contains_entity(&a));
}

// ══════════════════════════════════════════════════════════════════════════════
// Node / NodeSet convenience methods
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn reasoner_node_flattened_representative() {
    let df = DF::new();
    let unsat = df.class_ce("http://ex.org/Unsat");
    let mut entities = HashSet::new();
    entities.insert(df.owl_nothing());
    entities.insert(unsat.clone());
    let node = Node::new(entities);

    // Prefers the non-marker entity over owl:Nothing.
    assert_eq!(node.get_flattened_representative(), unsat);

    // get_entities_minus_top_bottom strips the marker entity.
    let minus = node.get_entities_minus_top_bottom();
    assert_eq!(minus.len(), 1);
    assert!(minus.contains(&unsat));
    assert!(!minus.contains(&df.owl_nothing()));
}

#[test]
fn reasoner_node_set_entities_minus_top_bottom() {
    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let top = Node::top_node(df.owl_thing());
    let regular = Node::singleton(a.clone());
    let ns: NodeSet<ClassExpression> = [top, regular].into_iter().collect();

    let minus = ns.get_entities_minus_top_bottom();
    assert_eq!(minus.len(), 1, "top node should be excluded");
    assert!(minus.contains(&a));

    let minus_top = ns.get_entities_minus_top();
    assert_eq!(minus_top.len(), 1);

    let minus_bottom = ns.get_entities_minus_bottom();
    assert_eq!(minus_bottom.len(), 2, "only the bottom node is excluded");
}

#[test]
fn reasoner_node_set_top_bottom_singleton() {
    let df = DF::new();
    let top_only: NodeSet<ClassExpression> =
        std::iter::once(Node::top_node(df.owl_thing())).collect();
    assert!(top_only.is_top_singleton());
    assert!(!top_only.is_bottom_singleton());

    let bottom_only: NodeSet<ClassExpression> =
        std::iter::once(Node::bottom_node(df.owl_nothing())).collect();
    assert!(bottom_only.is_bottom_singleton());
    assert!(!bottom_only.is_top_singleton());
}

/// CR-008 MAJOR #1: `direct = true` must not leak *defined-class* supertypes.
///
/// Counter-example from the CR-008 report: `B ≡ {a}`, `A ⊑ B`, `A(x)` ⇒ x's
/// full types are `{A, B}`, but the direct (most-specific) types must be
/// `{A}` only — `B` is a superclass of `A`.  Before the fix the core
/// `Reasoner::get_types` returned `{A, B}` for `direct = true` because the
/// defined-class candidate loop ran unconditionally, before any most-specific
/// filtering (`src/core/reasoner/core.rs`).
#[test]
fn reasoner_get_types_direct_defined_class_supertype() {
    use oxidowl::{Reasoner, ReasonerConfig};

    let df = DF::new();
    let a = df.class_ce("http://ex.org/A");
    let b = df.class_ce("http://ex.org/B");
    let nominal = df.named("http://ex.org/a");
    let x = df.named("http://ex.org/x");

    let mut ont = df.build_ontology(vec![
        df.equivalent_classes(vec![b.clone(), df.one_of(vec![nominal.clone()])]),
        df.sub_class_of(a.clone(), b.clone()),
        df.class_assertion(a.clone(), x.clone()),
    ]);
    df.auto_declare(&mut ont);

    // ── Core surface (`core::reasoner::Reasoner::get_types`) ────────────────
    let mut core = Reasoner::new(ReasonerConfig::default()).expect("core reasoner");
    core.load_ontology(ont.clone()).expect("load ontology");

    let direct = core.get_types(&x, true).expect("core get_types(direct)");
    assert!(
        direct.iter().any(|t| t == &a),
        "A must be a direct type of x; got {direct:?}"
    );
    assert!(
        !direct.iter().any(|t| t == &b),
        "B (a defined supertype of A) must not leak into direct types — \
         CR-008 MAJOR #1; got {direct:?}"
    );

    let all = core.get_types(&x, false).expect("core get_types(all)");
    assert!(
        all.iter().any(|t| t == &a) && all.iter().any(|t| t == &b),
        "the full type set must still contain both A and B; got {all:?}"
    );

    // ── OWL API surface (`OWLReasoner::get_types`) ──────────────────────────
    let onto = onto_ref(ont);
    let owl = TableauReasonerFactory
        .create_reasoner(&onto, &OWLReasonerConfiguration::default())
        .expect("OWLAPI reasoner");
    let direct = owl.get_types(&x, true).expect("OWLAPI get_types(direct)");
    assert!(
        direct.contains_entity(&a),
        "OWLAPI direct types must contain A; got {direct:?}"
    );
    assert!(
        !direct.contains_entity(&b),
        "OWLAPI direct types must not contain the defined supertype B; got {direct:?}"
    );
}

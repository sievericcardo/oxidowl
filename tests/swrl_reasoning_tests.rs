//! DL-safe SWRL forward-chaining materialization tests.
//!
//! Covers [`oxidowl::swrl::SwrlMaterializer`] end to end:
//!
//! 1. a class rule (`Person(?x) → Student(?x)`) is applied while loading an
//!    ontology, so `is_instance_of` / `get_instances` / `is_entailed` all see
//!    the derived fact;
//! 2. chained rules reach their fixpoint in exactly two iterations;
//! 3. object-property rules — plain and through a sub-property;
//! 4. a data-property rule gated by the `swrlb:lessThan` built-in, with a
//!    negative control;
//! 5. `sameAs` binds the second argument through the union-find;
//! 6. `differentFrom` filters out instances without an explicit difference;
//! 7. an unsafe rule (head variable absent from the body) emits nothing;
//! 8. cyclic rules terminate at the iteration bound;
//! 9. `add_axiom(Rule)` does *not* materialize, while
//!    `Reasoner::materialize_swrl_rules` does — and is idempotent.

mod helpers;

use helpers::df::DF;
use oxidowl::core::reasoner::statistics::ReasoningStatistics;
use oxidowl::{
    OWLReasonerConfiguration, Reasoner, ReasonerConfig, ReasonerFactory, TableauReasonerFactory,
    ontology::{
        Axiom, ClassAssertionAxiom, ClassExpression, DataProperty, DataPropertyExpression, IRI,
        Individual, NamedIndividual, ObjectProperty, ObjectPropertyAssertionAxiom,
        ObjectPropertyExpression, Ontology, SWRLAtom, SWRLDArgument, SWRLIArgument, SWRLRule,
        SWRLRuleAxiom, SWRLVariable, SubObjectPropertyOfAxiom, axioms::AxiomTrait,
    },
    swrl::SwrlMaterializer,
};

const EX: &str = "http://ex.org/";

fn var(name: &str) -> SWRLVariable {
    SWRLVariable::new(IRI::new(&format!("urn:swrl#{name}")))
}

fn named(iri: &str) -> Individual {
    Individual::Named(NamedIndividual { iri: IRI::new(iri) })
}

fn class_atom(class: ClassExpression, argument: SWRLIArgument) -> SWRLAtom {
    SWRLAtom::ClassAtom {
        predicate: class,
        argument,
    }
}

/// `headClass(?x) :- bodyClass(?x)` as a rule axiom with a fresh id.
fn class_rule(df: &DF, head: ClassExpression, body: ClassExpression) -> Axiom {
    Axiom::Rule(SWRLRuleAxiom::new(
        df.next_id(),
        SWRLRule::new(
            vec![class_atom(head, SWRLIArgument::Variable(var("x")))],
            vec![class_atom(body, SWRLIArgument::Variable(var("x")))],
        ),
    ))
}

fn class_assertion(class: ClassExpression, individual: Individual) -> Axiom {
    Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 0,
        individual,
        class,
        annotations: Vec::new(),
    })
}

fn has_ancestor_axiom(source: &str, target: &str) -> Axiom {
    Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
        id: 0,
        source: named(source),
        target: named(target),
        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(&format!("{EX}hasAncestor")),
        }),
        annotations: Vec::new(),
    })
}

fn has_ancestor_property() -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty {
        iri: IRI::new(&format!("{EX}hasAncestor")),
    })
}

fn rule_count(ontology: &Ontology) -> usize {
    ontology
        .axioms()
        .iter()
        .filter(|axiom| matches!(axiom, Axiom::Rule(_)))
        .count()
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Class rule → instance queries and entailment
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn person_rule_materializes_student_instances() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let person = df.class_ce(format!("{EX}Person"));
    let student = df.class_ce(format!("{EX}Student"));

    let mut ontology = df.build_ontology(vec![
        class_assertion(person.clone(), alice.clone()),
        class_rule(&df, student.clone(), person),
    ]);
    df.auto_declare(&mut ontology);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner
        .load_ontology(ontology)
        .expect("load ontology with rule");

    // The materialized ClassAssertion is visible to the instance queries.
    assert!(
        reasoner
            .is_instance_of(&alice, &student)
            .expect("is_instance_of"),
        "Person(alice) must materialize to Student(alice)"
    );
    let instances = reasoner
        .get_instances(&student, false)
        .expect("get_instances");
    assert!(
        instances.contains(&alice),
        "get_instances(Student) must contain alice, got {instances:?}"
    );

    // `is_entailed` on the OWL API surface.  The reasoner's own ontology is
    // already materialized, so the second load inside the factory is a no-op
    // — which also exercises idempotency.
    let ontology_ref = reasoner.get_ontology().expect("ontology loaded").clone();
    let api = TableauReasonerFactory
        .create_reasoner(&ontology_ref, &OWLReasonerConfiguration::default())
        .expect("api reasoner");
    let entailed = api
        .is_entailed(&class_assertion(student.clone(), alice.clone()))
        .expect("is_entailed");
    assert!(entailed, "ClassAssertion(Student, alice) must be entailed");

    // …and the same answer through the core entailment path.
    let mut stats = ReasoningStatistics::new();
    let core_entailed = reasoner
        .check_entailment(&class_assertion(student, alice), &ontology_ref, &mut stats)
        .expect("check_entailment");
    assert!(core_entailed);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Chained rules
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn chained_class_rules_derive_in_two_iterations() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));
    let c = df.class_ce(format!("{EX}C"));

    let mut ontology = df.build_ontology(vec![
        class_assertion(a.clone(), alice.clone()),
        class_rule(&df, b.clone(), a),
        class_rule(&df, c.clone(), b),
    ]);
    df.auto_declare(&mut ontology);

    let materialization = SwrlMaterializer::new()
        .materialize(&ontology)
        .expect("materialization");
    assert_eq!(
        materialization.iterations, 2,
        "A→B fires in iteration 1, B→C in iteration 2"
    );
    assert_eq!(materialization.applications, 2);
    assert_eq!(materialization.derived_axioms.len(), 2);
    assert!(
        materialization
            .derived_axioms
            .iter()
            .any(|axiom| matches!(axiom, Axiom::ClassAssertion(assertion)
            if assertion.class == c && assertion.individual == alice))
    );

    // End to end through the reasoner.
    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert!(
        reasoner.is_instance_of(&alice, &c).expect("is_instance_of"),
        "C(alice) must be derived through the chain A→B→C"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Object property rules
// ─────────────────────────────────────────────────────────────────────────────

/// Asserts `hasAncestor(source, target)` is present in the reasoner's
/// ontology both as an entailment and through the property-instance query.
fn assert_has_ancestor(reasoner: &mut Reasoner, source: &str, target: &str) {
    let ontology_ref = reasoner.get_ontology().expect("ontology loaded").clone();
    let mut stats = ReasoningStatistics::new();
    let entailed = reasoner
        .check_entailment(
            &has_ancestor_axiom(source, target),
            &ontology_ref,
            &mut stats,
        )
        .expect("check_entailment");
    assert!(
        entailed,
        "ObjectPropertyAssertion(hasAncestor, {source}, {target}) must be entailed"
    );

    let instances = reasoner
        .get_object_property_instances(&has_ancestor_property())
        .expect("get_object_property_instances");
    assert!(
        instances
            .iter()
            .any(|(s, t)| s == &named(source) && t == &named(target)),
        "hasAncestor instances must include ({source}, {target}), got {instances:?}"
    );
}

/// `hasAncestor(?x, ?y) :- hasParent(?x, ?y)` as a rule axiom.
fn has_ancestor_rule(df: &DF) -> Axiom {
    Axiom::Rule(SWRLRuleAxiom::new(
        df.next_id(),
        SWRLRule::new(
            vec![SWRLAtom::ObjectPropertyAtom {
                predicate: has_ancestor_property(),
                first_argument: SWRLIArgument::Variable(var("x")),
                second_argument: SWRLIArgument::Variable(var("y")),
            }],
            vec![SWRLAtom::ObjectPropertyAtom {
                predicate: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                    iri: IRI::new(&format!("{EX}hasParent")),
                }),
                first_argument: SWRLIArgument::Variable(var("x")),
                second_argument: SWRLIArgument::Variable(var("y")),
            }],
        ),
    ))
}

#[test]
fn object_property_rule_derives_the_assertion() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let bob = named(&format!("{EX}bob"));
    let has_parent = df.obj_prop(format!("{EX}hasParent"));

    let mut ontology = df.build_ontology(vec![
        df.object_property_assertion(has_parent, alice, bob),
        has_ancestor_rule(&df),
    ]);
    df.auto_declare(&mut ontology);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert_has_ancestor(&mut reasoner, &format!("{EX}alice"), &format!("{EX}bob"));
}

#[test]
fn object_property_rule_fires_through_a_sub_property() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let bob = named(&format!("{EX}bob"));
    let has_father = df.obj_prop(format!("{EX}hasFather"));
    let has_parent = df.obj_prop(format!("{EX}hasParent"));

    let mut ontology = df.build_ontology(vec![
        Axiom::SubObjectPropertyOf(SubObjectPropertyOfAxiom {
            id: df.next_id(),
            sub_property: has_father.clone(),
            super_property: has_parent,
            annotations: Vec::new(),
        }),
        // Only `hasFather(alice, bob)` is asserted — the rule must still fire
        // because `hasFather ⊑ hasParent`.
        df.object_property_assertion(has_father, alice, bob),
        has_ancestor_rule(&df),
    ]);
    df.auto_declare(&mut ontology);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert_has_ancestor(&mut reasoner, &format!("{EX}alice"), &format!("{EX}bob"));
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Data property rule with a built-in filter
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn data_property_rule_uses_the_less_than_builtin() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let bob = named(&format!("{EX}bob"));
    let age = DataPropertyExpression::DataProperty(DataProperty {
        iri: IRI::new(&format!("{EX}age")),
    });
    let minor = df.class_ce(format!("{EX}Minor"));

    let rule = Axiom::Rule(SWRLRuleAxiom::new(
        df.next_id(),
        SWRLRule::new(
            vec![class_atom(minor.clone(), SWRLIArgument::Variable(var("x")))],
            vec![
                SWRLAtom::DataPropertyAtom {
                    predicate: age.clone(),
                    first_argument: SWRLIArgument::Variable(var("x")),
                    second_argument: SWRLDArgument::Variable(var("age")),
                },
                SWRLAtom::BuiltInAtom {
                    predicate: IRI::new("http://www.w3.org/2003/11/swrlb#lessThan"),
                    arguments: vec![
                        SWRLDArgument::Variable(var("age")),
                        SWRLDArgument::Literal(df.int_literal(18)),
                    ],
                },
            ],
        ),
    ));

    let mut ontology = df.build_ontology(vec![
        df.data_property_assertion(age.clone(), alice.clone(), df.int_literal(10)),
        // Negative control: 20 is not < 18.
        df.data_property_assertion(age, bob.clone(), df.int_literal(20)),
        rule,
    ]);
    df.auto_declare(&mut ontology);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");

    assert!(
        reasoner.is_instance_of(&alice, &minor).expect("alice"),
        "alice (age 10) must be materialized as a Minor"
    );
    assert!(
        !reasoner.is_instance_of(&bob, &minor).expect("bob"),
        "bob (age 20) must NOT be a Minor"
    );
    let instances = reasoner.get_instances(&minor, false).expect("instances");
    assert!(instances.contains(&alice), "got {instances:?}");
    assert!(!instances.contains(&bob), "got {instances:?}");
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. sameAs binding
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn same_individual_atom_binds_through_the_union_find() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let alias = named(&format!("{EX}alias"));
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));

    // A(?x) ∧ sameAs(?x, ?y) → B(?y)
    let rule = Axiom::Rule(SWRLRuleAxiom::new(
        df.next_id(),
        SWRLRule::new(
            vec![class_atom(b.clone(), SWRLIArgument::Variable(var("y")))],
            vec![
                class_atom(a.clone(), SWRLIArgument::Variable(var("x"))),
                SWRLAtom::SameIndividualAtom {
                    first_argument: SWRLIArgument::Variable(var("x")),
                    second_argument: SWRLIArgument::Variable(var("y")),
                },
            ],
        ),
    ));

    let mut ontology = df.build_ontology(vec![
        class_assertion(a, alice.clone()),
        df.same_individual(vec![alice.clone(), alias.clone()]),
        rule,
    ]);
    df.auto_declare(&mut ontology);

    let materialization = SwrlMaterializer::new()
        .materialize(&ontology)
        .expect("materialization");
    assert!(
        materialization
            .derived_axioms
            .iter()
            .any(|axiom| matches!(axiom, Axiom::ClassAssertion(assertion)
                if assertion.class == b && assertion.individual == alice)),
        "B(alice) must be derived"
    );
    assert!(
        materialization
            .derived_axioms
            .iter()
            .any(|axiom| matches!(axiom, Axiom::ClassAssertion(assertion)
                if assertion.class == b && assertion.individual == alias)),
        "B(alias) must be derived through sameAs(alice, alias)"
    );

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert!(reasoner.is_instance_of(&alias, &b).expect("alias"));
    assert!(reasoner.is_instance_of(&alice, &b).expect("alice"));
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. differentFrom filter
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn different_individuals_atom_filters_rule_instances() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let bob = named(&format!("{EX}bob"));
    let carol = named(&format!("{EX}carol"));
    let person = df.class_ce(format!("{EX}Person"));
    let stranger = df.obj_prop(format!("{EX}stranger"));

    // Person(?x) ∧ Person(?y) ∧ differentFrom(?x, ?y) → stranger(?x, ?y)
    let rule = Axiom::Rule(SWRLRuleAxiom::new(
        df.next_id(),
        SWRLRule::new(
            vec![SWRLAtom::ObjectPropertyAtom {
                predicate: stranger.clone(),
                first_argument: SWRLIArgument::Variable(var("x")),
                second_argument: SWRLIArgument::Variable(var("y")),
            }],
            vec![
                class_atom(person.clone(), SWRLIArgument::Variable(var("x"))),
                class_atom(person.clone(), SWRLIArgument::Variable(var("y"))),
                SWRLAtom::DifferentIndividualsAtom {
                    first_argument: SWRLIArgument::Variable(var("x")),
                    second_argument: SWRLIArgument::Variable(var("y")),
                },
            ],
        ),
    ));

    let mut ontology = df.build_ontology(vec![
        class_assertion(person.clone(), alice.clone()),
        class_assertion(person, bob.clone()),
        // carol is not asserted different from anyone → she never binds.
        df.class_assertion(df.class_ce(format!("{EX}Person")), carol.clone()),
        df.different_individuals(vec![alice.clone(), bob.clone()]),
        rule,
    ]);
    df.auto_declare(&mut ontology);

    let materialization = SwrlMaterializer::new()
        .materialize(&ontology)
        .expect("materialization");
    let stranger_pairs = |axiom: &Axiom| match axiom {
        Axiom::ObjectPropertyAssertion(assertion) if assertion.property == stranger => {
            Some((assertion.source.clone(), assertion.target.clone()))
        }
        _ => None,
    };
    let derived: Vec<(Individual, Individual)> = materialization
        .derived_axioms
        .iter()
        .filter_map(stranger_pairs)
        .collect();
    assert_eq!(
        derived.len(),
        2,
        "exactly the two ordered (alice, bob) / (bob, alice) pairs, got {derived:?}"
    );
    assert!(derived.contains(&(alice.clone(), bob.clone())));
    assert!(derived.contains(&(bob, alice)));
    assert!(
        !derived.iter().any(|(s, t)| s == &carol || t == &carol),
        "carol is never explicitly different from anyone"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 7. DL-safety: unsafe rules emit nothing
// ─────────────────────────────────────────────────────────────────────────────
#[test]
fn unsafe_rule_emits_nothing() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let person = df.class_ce(format!("{EX}Person"));
    let student = df.class_ce(format!("{EX}Student"));

    // `?y` never occurs in the body → not DL-safe.
    let unsafe_rule = SWRLRule::new(
        vec![class_atom(
            student.clone(),
            SWRLIArgument::Variable(var("y")),
        )],
        vec![class_atom(
            person.clone(),
            SWRLIArgument::Variable(var("x")),
        )],
    );
    assert!(!unsafe_rule.is_safe(), "head variable ?y is unbound");

    let mut ontology = df.build_ontology(vec![
        class_assertion(person, alice.clone()),
        Axiom::Rule(SWRLRuleAxiom::new(df.next_id(), unsafe_rule)),
    ]);
    df.auto_declare(&mut ontology);

    let materialization = SwrlMaterializer::new()
        .materialize(&ontology)
        .expect("materialization must not fail");
    assert!(
        materialization.derived_axioms.is_empty(),
        "an unsafe rule must never emit a fact: {:?}",
        materialization.derived_axioms
    );
    assert_eq!(materialization.applications, 0);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert!(
        !reasoner
            .is_instance_of(&alice, &student)
            .expect("is_instance_of"),
        "Student(alice) must not be derived from an unsafe rule"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 8. Termination on cyclic rules
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn cyclic_rules_terminate_at_the_iteration_bound() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let a = df.class_ce(format!("{EX}A"));
    let b = df.class_ce(format!("{EX}B"));

    let mut ontology = df.build_ontology(vec![
        class_assertion(a.clone(), alice.clone()),
        class_rule(&df, b.clone(), a.clone()),
        class_rule(&df, a, b.clone()),
    ]);
    df.auto_declare(&mut ontology);

    let mut materializer = SwrlMaterializer::new();
    materializer.max_iterations = 4;
    let materialization = materializer
        .materialize(&ontology)
        .expect("cyclic rules must terminate");
    assert!(
        materialization.iterations <= 4,
        "iterations must respect the bound, got {}",
        materialization.iterations
    );
    // `A(alice)` is asserted already, so only `B(alice)` is ever new — the
    // cycle cannot grow the fact base further.
    assert_eq!(materialization.derived_axioms.len(), 1);

    // Loading the same cyclic ontology must not hang either.
    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert!(reasoner.is_instance_of(&alice, &b).expect("is_instance_of"));
}

// ─────────────────────────────────────────────────────────────────────────────
// 9. add_axiom does not materialize; materialize_swrl_rules does (idempotently)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn materialize_swrl_rules_is_explicit_and_idempotent() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let person = df.class_ce(format!("{EX}Person"));
    let student = df.class_ce(format!("{EX}Student"));

    // Base ontology has the assertion but no rule yet.
    let mut ontology = df.build_ontology(vec![class_assertion(person.clone(), alice.clone())]);
    df.auto_declare(&mut ontology);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner.load_ontology(ontology).expect("load");
    assert!(!reasoner.is_instance_of(&alice, &student).expect("before"));

    // Adding a rule axiom does NOT auto-materialize (documented contract).
    reasoner
        .add_axiom(class_rule(&df, student.clone(), person))
        .expect("add rule");
    assert!(
        !reasoner
            .is_instance_of(&alice, &student)
            .expect("after add_axiom"),
        "add_axiom(Rule) must not materialize automatically"
    );

    // The explicit entry point does.
    let added = reasoner
        .materialize_swrl_rules()
        .expect("materialize_swrl_rules");
    assert_eq!(added, 1, "exactly one ClassAssertion(Student, alice)");
    assert!(
        reasoner
            .is_instance_of(&alice, &student)
            .expect("after materialize")
    );
    let instances = reasoner.get_instances(&student, false).expect("instances");
    assert!(instances.contains(&alice), "got {instances:?}");

    // A second run derives nothing new.
    let again = reasoner
        .materialize_swrl_rules()
        .expect("second materialize_swrl_rules");
    assert_eq!(again, 0, "materialization must be idempotent");

    let ontology_ref = reasoner.get_ontology().expect("ontology loaded").clone();
    let guard = ontology_ref.read().expect("ontology lock");
    assert_eq!(rule_count(&guard), 1, "no duplicated rule axiom");
    let student_assertions = guard
        .axioms()
        .iter()
        .filter(|axiom| {
            matches!(axiom, Axiom::ClassAssertion(assertion)
                if assertion.class == student && assertion.individual == alice)
        })
        .count();
    assert_eq!(
        student_assertions, 1,
        "the derived fact must not be duplicated"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 10. Derived facts are plain assertions with fresh ids and empty annotations
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn derived_axioms_are_plain_assertions_with_empty_annotations() {
    let df = DF::new();
    let alice = named(&format!("{EX}alice"));
    let person = df.class_ce(format!("{EX}Person"));
    let student = df.class_ce(format!("{EX}Student"));

    let mut ontology = df.build_ontology(vec![
        class_assertion(person.clone(), alice.clone()),
        class_rule(&df, student, person),
    ]);
    df.auto_declare(&mut ontology);
    let max_id_before = ontology
        .axioms()
        .iter()
        .map(AxiomTrait::axiom_id)
        .max()
        .unwrap_or(0);

    let materialization = SwrlMaterializer::new()
        .materialize(&ontology)
        .expect("materialization");
    assert_eq!(materialization.derived_axioms.len(), 1);
    for axiom in &materialization.derived_axioms {
        assert!(
            matches!(
                axiom,
                Axiom::ClassAssertion(ClassAssertionAxiom { annotations, .. }) if annotations.is_empty()
            ),
            "derived axioms carry empty annotations: {axiom:?}"
        );
        assert!(
            axiom.axiom_id() > max_id_before,
            "derived axiom ids must be fresh"
        );
    }
}

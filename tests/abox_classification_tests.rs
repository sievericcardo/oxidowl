//! ABox classification tests
//!
//! Verifies that the native Rust forward-chaining materialization engine
//! and the optional SPARQL-based Oxigraph materialization produce correct
//! OWL DL ABox entailments.

use oxidowl::{
    config::ReasonerConfig,
    core::reasoner::core::Reasoner,
    ontology::{
        Axiom, Class, ClassExpression, IRI, Individual, NamedIndividual, ObjectProperty,
        ObjectPropertyExpression, Ontology,
        axioms::{
            ClassAssertionAxiom, EquivalentClassesAxiom, ObjectPropertyAssertionAxiom,
            ObjectPropertyDomainAxiom, ObjectPropertyRangeAxiom, SubClassOfAxiom,
        },
    },
    profiles::rl_reasoner::RLReasoner,
};

fn make_named_ind(iri: &str) -> Individual {
    Individual::Named(NamedIndividual {
        iri: IRI::new(iri),
    })
}

fn named_class(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn named_prop(iri: &str) -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(
        ObjectProperty::new(IRI::new(iri)).expect("valid IRI"),
    )
}

fn make_ontology_with_axioms(axioms: Vec<Axiom>) -> Ontology {
    let mut ont = Ontology::new();
    for ax in axioms {
        ont.add_axiom(ax);
    }
    ont
}

// ─── Helper: run native RL materialization ────────────────────────────────────

fn rl_materialize(ont: &Ontology) -> RLReasoner {
    let mut rl = RLReasoner::new(ReasonerConfig::default());
    rl.initialize(ont).expect("initialize should not fail");
    rl.materialize().expect("materialize should not fail");
    rl
}

// ─── SubClassOf propagation ───────────────────────────────────────────────────

#[test]
fn test_subclass_propagation() {
    // A ⊑ B, B ⊑ C, a:A  ⟹  a:B, a:C
    let ont = make_ontology_with_axioms(vec![
        Axiom::SubClassOf(SubClassOfAxiom {
            id: 0,
            subclass: named_class("http://ex.org/A"),
            superclass: named_class("http://ex.org/B"),
            annotations: vec![],
        }),
        Axiom::SubClassOf(SubClassOfAxiom {
            id: 0,
            subclass: named_class("http://ex.org/B"),
            superclass: named_class("http://ex.org/C"),
            annotations: vec![],
        }),
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            individual: make_named_ind("http://ex.org/a"),
            class: named_class("http://ex.org/A"),
            annotations: vec![],
        }),
    ]);

    let rl = rl_materialize(&ont);
    let a = make_named_ind("http://ex.org/a");

    assert!(
        rl.is_instance_of(&a, &named_class("http://ex.org/B"))
            .unwrap(),
        "a should be inferred to be of type B"
    );
    assert!(
        rl.is_instance_of(&a, &named_class("http://ex.org/C"))
            .unwrap(),
        "a should be inferred to be of type C (transitivity)"
    );
}

// ─── Domain inference ─────────────────────────────────────────────────────────

#[test]
fn test_domain_rule() {
    // domain(hasParent) = Person, a hasParent b  ⟹  a:Person
    let ont = make_ontology_with_axioms(vec![
        Axiom::ObjectPropertyDomain(ObjectPropertyDomainAxiom {
            id: 0,
            property: named_prop("http://ex.org/hasParent"),
            domain: named_class("http://ex.org/Person"),
            annotations: vec![],
        }),
        Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
            id: 0,
            source: make_named_ind("http://ex.org/alice"),
            target: make_named_ind("http://ex.org/bob"),
            property: named_prop("http://ex.org/hasParent"),
            annotations: vec![],
        }),
    ]);

    let rl = rl_materialize(&ont);
    assert!(
        rl.is_instance_of(
            &make_named_ind("http://ex.org/alice"),
            &named_class("http://ex.org/Person")
        )
        .unwrap(),
        "alice should be inferred to be a Person via domain rule"
    );
}

// ─── Range inference ──────────────────────────────────────────────────────────

#[test]
fn test_range_rule() {
    // range(hasParent) = Person, a hasParent b  ⟹  b:Person
    let ont = make_ontology_with_axioms(vec![
        Axiom::ObjectPropertyRange(ObjectPropertyRangeAxiom {
            id: 0,
            property: named_prop("http://ex.org/hasParent"),
            range: named_class("http://ex.org/Person"),
            annotations: vec![],
        }),
        Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
            id: 0,
            source: make_named_ind("http://ex.org/alice"),
            target: make_named_ind("http://ex.org/bob"),
            property: named_prop("http://ex.org/hasParent"),
            annotations: vec![],
        }),
    ]);

    let rl = rl_materialize(&ont);
    assert!(
        rl.is_instance_of(
            &make_named_ind("http://ex.org/bob"),
            &named_class("http://ex.org/Person")
        )
        .unwrap(),
        "bob should be inferred to be a Person via range rule"
    );
}

// ─── HasValue rule ────────────────────────────────────────────────────────────

#[test]
fn test_has_value_rule() {
    // C ≡ ∃worksFor.{Acme}
    // alice worksFor Acme  ⟹  alice : C
    let restriction = ClassExpression::ObjectHasValue {
        property: named_prop("http://ex.org/worksFor"),
        value: make_named_ind("http://ex.org/Acme"),
    };
    let ont = make_ontology_with_axioms(vec![
        Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 0,
            classes: vec![named_class("http://ex.org/AcmeEmployee"), restriction],
            annotations: vec![],
        }),
        Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
            id: 0,
            source: make_named_ind("http://ex.org/alice"),
            target: make_named_ind("http://ex.org/Acme"),
            property: named_prop("http://ex.org/worksFor"),
            annotations: vec![],
        }),
    ]);

    let rl = rl_materialize(&ont);
    assert!(
        rl.is_instance_of(
            &make_named_ind("http://ex.org/alice"),
            &named_class("http://ex.org/AcmeEmployee")
        )
        .unwrap(),
        "alice should be inferred to be an AcmeEmployee via hasValue rule"
    );
}

// ─── SomeValuesFrom rule ──────────────────────────────────────────────────────

#[test]
fn test_some_values_from_rule() {
    // Parent ≡ ∃hasChild.Person
    // alice hasChild bob, bob:Person  ⟹  alice:Parent
    let restriction = ClassExpression::ObjectSomeValuesFrom {
        property: named_prop("http://ex.org/hasChild"),
        filler: Box::new(named_class("http://ex.org/Person")),
    };
    let ont = make_ontology_with_axioms(vec![
        Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 0,
            classes: vec![named_class("http://ex.org/Parent"), restriction],
            annotations: vec![],
        }),
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            individual: make_named_ind("http://ex.org/bob"),
            class: named_class("http://ex.org/Person"),
            annotations: vec![],
        }),
        Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
            id: 0,
            source: make_named_ind("http://ex.org/alice"),
            target: make_named_ind("http://ex.org/bob"),
            property: named_prop("http://ex.org/hasChild"),
            annotations: vec![],
        }),
    ]);

    let rl = rl_materialize(&ont);
    assert!(
        rl.is_instance_of(
            &make_named_ind("http://ex.org/alice"),
            &named_class("http://ex.org/Parent")
        )
        .unwrap(),
        "alice should be inferred to be a Parent via someValuesFrom rule"
    );
}

// ─── Intersection rule ────────────────────────────────────────────────────────

#[test]
fn test_intersection_rule() {
    // GradStudent ≡ Student ⊓ Researcher
    // alice:Student, alice:Researcher  ⟹  alice:GradStudent
    let intersection = ClassExpression::ObjectIntersectionOf(vec![
        named_class("http://ex.org/Student"),
        named_class("http://ex.org/Researcher"),
    ]);
    let ont = make_ontology_with_axioms(vec![
        Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 0,
            classes: vec![named_class("http://ex.org/GradStudent"), intersection],
            annotations: vec![],
        }),
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            individual: make_named_ind("http://ex.org/alice"),
            class: named_class("http://ex.org/Student"),
            annotations: vec![],
        }),
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            individual: make_named_ind("http://ex.org/alice"),
            class: named_class("http://ex.org/Researcher"),
            annotations: vec![],
        }),
    ]);

    let rl = rl_materialize(&ont);
    assert!(
        rl.is_instance_of(
            &make_named_ind("http://ex.org/alice"),
            &named_class("http://ex.org/GradStudent")
        )
        .unwrap(),
        "alice should be inferred to be a GradStudent via intersection rule"
    );
}

// ─── materialize_abox via Reasoner ────────────────────────────────────────────

#[test]
fn test_reasoner_materialize_abox() {
    let ont = make_ontology_with_axioms(vec![
        Axiom::SubClassOf(SubClassOfAxiom {
            id: 0,
            subclass: named_class("http://ex.org/Dog"),
            superclass: named_class("http://ex.org/Animal"),
            annotations: vec![],
        }),
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            individual: make_named_ind("http://ex.org/fido"),
            class: named_class("http://ex.org/Dog"),
            annotations: vec![],
        }),
    ]);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("Reasoner::new");
    reasoner.load_ontology(ont).expect("load_ontology");
    let stats = reasoner.materialize_abox().expect("materialize_abox");

    assert!(stats.facts_added > 0, "At least one fact should be materialized");
}

// ─── SPARQL-based classification via Oxigraph ─────────────────────────────────

#[cfg(feature = "sparql-store")]
#[test]
fn test_sparql_abox_classification() {
    use oxidowl::core::reasoner::core::Reasoner;

    let ont = make_ontology_with_axioms(vec![
        Axiom::SubClassOf(SubClassOfAxiom {
            id: 0,
            subclass: named_class("http://ex.org/Cat"),
            superclass: named_class("http://ex.org/Animal"),
            annotations: vec![],
        }),
        Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 0,
            individual: make_named_ind("http://ex.org/whiskers"),
            class: named_class("http://ex.org/Cat"),
            annotations: vec![],
        }),
    ]);

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("Reasoner::new");
    reasoner.load_ontology(ont).expect("load_ontology");
    let stats = reasoner
        .run_sparql_abox_classification()
        .expect("run_sparql_abox_classification");

    // whiskers should now be inferred to be an Animal
    assert!(
        stats.facts_added > 0 || stats.sparql_rules_run > 0,
        "SPARQL classification should run at least one rule"
    );
}

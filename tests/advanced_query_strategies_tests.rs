//! End-to-end tests for the advanced query execution strategies.
//!
//! These tests exercise the full `AdvancedExecutionEngine::execute_query`
//! pipeline (optimization → strategy selection → evaluation) against a small
//! ontology and assert concrete query bindings.

use oxidowl::ontology::axioms::{
    Axiom, ClassAssertionAxiom, ObjectPropertyAssertionAxiom, SubClassOfAxiom,
};
use oxidowl::ontology::{
    Class, ClassExpression, IRI, Individual, NamedIndividual, ObjectProperty,
    ObjectPropertyExpression, Ontology,
};
use oxidowl::query::advanced::execution::{BoundValue, ConjunctiveQueryResult};
use oxidowl::query::advanced::{
    AdvancedExecutionConfig, AdvancedExecutionEngine, ConjunctiveQuery, ExecutionConstraints,
    ExecutionPriority, QueryAtom, QueryVariable,
};
use oxidowl::reasoning::ReasoningService;
use std::sync::Arc;

const A: &str = "http://ex.org/A";
const B: &str = "http://ex.org/B";
const R: &str = "http://ex.org/R";
const I: &str = "http://ex.org/i";
const J: &str = "http://ex.org/j";

fn named_class(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn named_individual(iri: &str) -> Individual {
    Individual::Named(NamedIndividual {
        iri: IRI::new(iri),
    })
}

fn default_constraints() -> ExecutionConstraints {
    ExecutionConstraints {
        max_execution_time: None,
        max_memory_usage: None,
        min_confidence: None,
        priority: ExecutionPriority::Normal,
    }
}

fn build_ontology() -> Ontology {
    let mut ontology = Ontology::new();
    let a = named_class(A);
    let b = named_class(B);
    let i = named_individual(I);
    let j = named_individual(J);

    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 1,
        subclass: a.clone(),
        superclass: b,
        annotations: vec![],
    }));
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 2,
        class: a,
        individual: i.clone(),
        annotations: vec![],
    }));
    ontology.add_axiom(Axiom::ObjectPropertyAssertion(ObjectPropertyAssertionAxiom {
        id: 3,
        property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
            iri: IRI::new(R),
        }),
        source: i,
        target: j,
        annotations: vec![],
    }));
    ontology
}

fn make_engine(ontology: Ontology) -> AdvancedExecutionEngine {
    let ontology_arc = Arc::new(ontology.clone());
    let reasoning = Arc::new(
        ReasoningService::new(ontology, Default::default())
            .expect("Failed to create ReasoningService"),
    );
    AdvancedExecutionEngine::new(ontology_arc, reasoning, AdvancedExecutionConfig::default())
        .expect("Failed to create AdvancedExecutionEngine")
}

fn class_query(var: &str, class_iri: &str) -> ConjunctiveQuery {
    ConjunctiveQuery {
        answer_variables: vec![QueryVariable::new(var.to_string())],
        body_atoms: vec![QueryAtom::ClassAtom {
            variable: QueryVariable::new(var.to_string()),
            class_expression: named_class(class_iri),
        }],
        constraints: Default::default(),
        metadata: Default::default(),
    }
}

fn bound_individuals(result: &ConjunctiveQueryResult, var: &str) -> Vec<Individual> {
    let variable = QueryVariable::new(var.to_string());
    result
        .bindings
        .iter()
        .filter_map(|binding| binding.get_binding(&variable))
        .filter_map(|value| match value {
            BoundValue::Individual(ind) => Some(ind.clone()),
            _ => None,
        })
        .collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn test_class_atom_query_returns_direct_instances() {
    let engine = make_engine(build_ontology());
    let result = engine
        .execute_query(&class_query("x", A), default_constraints())
        .await
        .expect("query should succeed");

    let individuals = bound_individuals(&result, "x");
    assert!(
        individuals.contains(&named_individual(I)),
        "Expected A to contain individual i, got {individuals:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_class_atom_query_resolves_subclass() {
    let engine = make_engine(build_ontology());
    let result = engine
        .execute_query(&class_query("x", B), default_constraints())
        .await
        .expect("query should succeed");

    let individuals = bound_individuals(&result, "x");
    assert!(
        individuals.contains(&named_individual(I)),
        "Expected subclass reasoning to bind i to B, got {individuals:?}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_object_property_atom_query_returns_bindings() {
    let engine = make_engine(build_ontology());
    let query = ConjunctiveQuery {
        answer_variables: vec![QueryVariable::new("x"), QueryVariable::new("y")],
        body_atoms: vec![QueryAtom::ObjectPropertyAtom {
            subject: QueryVariable::new("x"),
            property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                iri: IRI::new(R),
            }),
            object: QueryVariable::new("y"),
        }],
        constraints: Default::default(),
        metadata: Default::default(),
    };

    let result = engine
        .execute_query(&query, default_constraints())
        .await
        .expect("query should succeed");

    assert!(
        bound_individuals(&result, "x").contains(&named_individual(I)),
        "Expected R to bind x to i"
    );
    assert!(
        bound_individuals(&result, "y").contains(&named_individual(J)),
        "Expected R to bind y to j"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_join_query_returns_combined_bindings() {
    let engine = make_engine(build_ontology());
    // Query: A(x) ∧ R(x, y) — should yield (x=i, y=j).
    let query = ConjunctiveQuery {
        answer_variables: vec![QueryVariable::new("x"), QueryVariable::new("y")],
        body_atoms: vec![
            QueryAtom::ClassAtom {
                variable: QueryVariable::new("x"),
                class_expression: named_class(A),
            },
            QueryAtom::ObjectPropertyAtom {
                subject: QueryVariable::new("x"),
                property: ObjectPropertyExpression::ObjectProperty(ObjectProperty {
                    iri: IRI::new(R),
                }),
                object: QueryVariable::new("y"),
            },
        ],
        constraints: Default::default(),
        metadata: Default::default(),
    };

    let result = engine
        .execute_query(&query, default_constraints())
        .await
        .expect("query should succeed");

    assert!(
        bound_individuals(&result, "x").contains(&named_individual(I)),
        "Expected join to bind x to i"
    );
    assert!(
        bound_individuals(&result, "y").contains(&named_individual(J)),
        "Expected join to bind y to j"
    );
}

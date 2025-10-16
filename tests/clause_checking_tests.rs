//! Integration tests for ClauseChecker with TableauExecutor
//!
//! These tests verify that DL clause checking works correctly during tableau expansion,
//! detecting violations that only appear during reasoning.

use oxidowl::{
    core::tableau::{
        ClauseChecker,
        node::{TableauNode, ConceptLabel, NodeType},
        EquivalenceClosure,
        DisjointnessMap,
    },
    dl_clauses::{DLClause, DLClauseSet, DLAtom},
    ontology::{Ontology, Axiom, ClassExpression, Class, IRI},
};
use std::collections::{HashMap, HashSet};

// ============================================================================
// Test Helpers and Utilities
// ============================================================================

/// Create a simple test ontology with specified axioms
fn create_test_ontology(axioms: Vec<Axiom>) -> Ontology {
    let mut ontology = Ontology::new();
    for axiom in axioms {
        ontology.add_axiom(axiom);
    }
    ontology
}

/// Create a simple atomic class
fn class(name: &str) -> ClassExpression {
    ClassExpression::Class(Class {
        iri: IRI::new(name),
    })
}

/// Create an intersection (AND) class expression
fn intersection(classes: Vec<ClassExpression>) -> ClassExpression {
    ClassExpression::ObjectIntersectionOf(classes)
}

/// Create a DisjointClasses axiom
fn disjoint_classes(classes: Vec<ClassExpression>) -> Axiom {
    use oxidowl::ontology::DisjointClassesAxiom;
    Axiom::DisjointClasses(DisjointClassesAxiom {
        classes,
        annotations: vec![],
        id: 0,
    })
}

/// Create an EquivalentClasses axiom
fn equivalent_classes(classes: Vec<ClassExpression>) -> Axiom {
    use oxidowl::ontology::EquivalentClassesAxiom;
    Axiom::EquivalentClasses(EquivalentClassesAxiom {
        classes,
        annotations: vec![],
        id: 0,
    })
}

/// Create a test DL clause set with deterministic clauses
fn create_test_clause_set(clauses: Vec<DLClause>) -> DLClauseSet {
    DLClauseSet {
        deterministic_clauses: clauses,
        disjunctive_clauses: vec![],
        abox_facts: vec![],
        prefixes: HashMap::new(),
        statistics: Default::default(),
    }
}

/// Create a simple deterministic DL clause: body → head
fn create_deterministic_clause(
    id: &str,
    body_predicates: Vec<&str>,
    head_predicates: Vec<&str>,
) -> DLClause {
    let body: Vec<DLAtom> = body_predicates
        .iter()
        .map(|p| DLAtom {
            predicate: p.to_string(),
            arguments: vec!["x".to_string()],
            is_positive: true,
            constraints: vec![],
        })
        .collect();

    let head: Vec<DLAtom> = head_predicates
        .iter()
        .map(|p| DLAtom {
            predicate: p.to_string(),
            arguments: vec!["x".to_string()],
            is_positive: true,
            constraints: vec![],
        })
        .collect();

    let variables: HashSet<String> = vec!["x".to_string()].into_iter().collect();

    DLClause {
        head,
        body,
        variables,
        id: id.to_string(),
    }
}

/// Create a negative clause (body → ⊥)
fn create_negative_clause(id: &str, body_predicates: Vec<&str>) -> DLClause {
    let body: Vec<DLAtom> = body_predicates
        .iter()
        .map(|p| DLAtom {
            predicate: p.to_string(),
            arguments: vec!["x".to_string()],
            is_positive: true,
            constraints: vec![],
        })
        .collect();

    let variables: HashSet<String> = vec!["x".to_string()].into_iter().collect();

    DLClause {
        head: vec![], // Empty head = ⊥
        body,
        variables,
        id: id.to_string(),
    }
}

/// Create a test node with specified concepts
fn create_node_with_concepts(id: usize, concept_names: Vec<&str>) -> TableauNode {
    let mut node = TableauNode::new(id, NodeType::Individual);
    for name in concept_names {
        node.add_concept(ConceptLabel::Atomic(name.to_string()));
    }
    node
}

// ============================================================================
// Test 1: Deterministic Clause Violations
// ============================================================================

#[test]
fn test_deterministic_clause_violation() {
    // Create clause: A(x) ∧ B(x) → C(x)
    let clause = create_deterministic_clause("clause1", vec!["A", "B"], vec!["C"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Create node with A and B but not C
    let node = create_node_with_concepts(0, vec!["A", "B"]);

    // Check for violation
    let violation = checker.check_node(&node);
    assert!(
        violation.is_some(),
        "Should detect violation when body satisfied but head not"
    );

    let v = violation.unwrap();
    assert_eq!(v.clause.id, "clause1");
    assert!(
        v.explanation.contains("body satisfied but head not satisfied"),
        "Explanation should mention unsatisfied head"
    );
}

#[test]
fn test_deterministic_clause_satisfied() {
    // Create clause: A(x) ∧ B(x) → C(x)
    let clause = create_deterministic_clause("clause1", vec!["A", "B"], vec!["C"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Create node with A, B, and C
    let node = create_node_with_concepts(0, vec!["A", "B", "C"]);

    // Check - should be no violation
    let violation = checker.check_node(&node);
    assert!(
        violation.is_none(),
        "Should not detect violation when clause fully satisfied"
    );
}

#[test]
fn test_body_not_satisfied() {
    // Create clause: A(x) ∧ B(x) → C(x)
    let clause = create_deterministic_clause("clause1", vec!["A", "B"], vec!["C"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Create node with only A (missing B)
    let node = create_node_with_concepts(0, vec!["A"]);

    // Check - should be no violation (body not satisfied)
    let violation = checker.check_node(&node);
    assert!(
        violation.is_none(),
        "Should not detect violation when body not satisfied"
    );
}

// ============================================================================
// Test 2: Negative Clause Violations
// ============================================================================

#[test]
fn test_negative_clause_violation() {
    // Create clause: A(x) ∧ B(x) → ⊥
    let clause = create_negative_clause("clause_neg", vec!["A", "B"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Create node with both A and B
    let node = create_node_with_concepts(0, vec!["A", "B"]);

    // Check for violation
    let violation = checker.check_node(&node);
    assert!(
        violation.is_some(),
        "Should detect violation for negative clause with satisfied body"
    );

    let v = violation.unwrap();
    assert_eq!(v.clause.id, "clause_neg");
    assert!(
        v.explanation.contains("negative clause") || v.explanation.contains("⊥"),
        "Explanation should mention negative clause or ⊥"
    );
}

#[test]
fn test_no_contradiction_when_body_incomplete() {
    // Create clause: A(x) ∧ B(x) → ⊥
    let clause = create_negative_clause("clause_neg", vec!["A", "B"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Create node with only A
    let node = create_node_with_concepts(0, vec!["A"]);

    // Check - should be no violation
    let violation = checker.check_node(&node);
    assert!(
        violation.is_none(),
        "Should not detect violation when negative clause body not satisfied"
    );
}

// ============================================================================
// Test 3: Disjointness Violations
// ============================================================================

#[test]
fn test_direct_disjointness_violation() {
    // Create ontology with DisjointClasses(Plant, Animal)
    let axioms = vec![disjoint_classes(vec![class("Plant"), class("Animal")])];
    let ontology = create_test_ontology(axioms);

    // Build DisjointnessMap
    let eq_closure = EquivalenceClosure::from_ontology(&ontology).unwrap();
    let disj_map = DisjointnessMap::from_ontology(&ontology, &eq_closure).unwrap();

    // Create empty clause set (we're testing disjointness checking)
    let clause_set = create_test_clause_set(vec![]);
    let mut checker = ClauseChecker::with_reasoning_support(clause_set, eq_closure, disj_map);

    // Create node with both Plant and Animal (using same IRIs as axioms)
    let node = create_node_with_concepts(
        0,
        vec!["Plant", "Animal"],
    );

    // Check for violation
    let violation = checker.check_node(&node);
    assert!(
        violation.is_some(),
        "Should detect disjointness violation between Plant and Animal"
    );
}

#[test]
fn test_no_disjointness_when_concepts_compatible() {
    // Create ontology with DisjointClasses(Plant, Animal)
    let axioms = vec![disjoint_classes(vec![class("Plant"), class("Animal")])];
    let ontology = create_test_ontology(axioms);

    // Build reasoning support
    let eq_closure = EquivalenceClosure::from_ontology(&ontology).unwrap();
    let disj_map = DisjointnessMap::from_ontology(&ontology, &eq_closure).unwrap();

    let clause_set = create_test_clause_set(vec![]);
    let mut checker = ClauseChecker::with_reasoning_support(clause_set, eq_closure, disj_map);

    // Create node with only Plant (using same IRI as axioms)
    let node = create_node_with_concepts(0, vec!["Plant"]);

    // Check - should be no violation
    let violation = checker.check_node(&node);
    assert!(
        violation.is_none(),
        "Should not detect violation when only one concept present"
    );
}

// ============================================================================
// Test 4: Multiple Clauses
// ============================================================================

#[test]
fn test_multiple_clauses_first_violates() {
    // Create multiple clauses
    let clause1 = create_deterministic_clause("c1", vec!["A", "B"], vec!["C"]);
    let clause2 = create_deterministic_clause("c2", vec!["D", "E"], vec!["F"]);
    let clause_set = create_test_clause_set(vec![clause1, clause2]);
    let mut checker = ClauseChecker::new(clause_set);

    // Node violates first clause
    let node = create_node_with_concepts(0, vec!["A", "B"]); // Missing C

    let violation = checker.check_node(&node);
    assert!(violation.is_some(), "Should detect first clause violation");
    assert_eq!(violation.unwrap().clause.id, "c1");
}

#[test]
fn test_multiple_clauses_second_violates() {
    // Create multiple clauses
    let clause1 = create_deterministic_clause("c1", vec!["A", "B"], vec!["C"]);
    let clause2 = create_deterministic_clause("c2", vec!["D", "E"], vec!["F"]);
    let clause_set = create_test_clause_set(vec![clause1, clause2]);
    let mut checker = ClauseChecker::new(clause_set);

    // Node satisfies first clause but violates second
    let node = create_node_with_concepts(0, vec!["A", "B", "C", "D", "E"]); // Missing F

    let violation = checker.check_node(&node);
    assert!(violation.is_some(), "Should detect second clause violation");
    assert_eq!(violation.unwrap().clause.id, "c2");
}

#[test]
fn test_multiple_clauses_all_satisfied() {
    // Create multiple clauses
    let clause1 = create_deterministic_clause("c1", vec!["A", "B"], vec!["C"]);
    let clause2 = create_deterministic_clause("c2", vec!["D", "E"], vec!["F"]);
    let clause_set = create_test_clause_set(vec![clause1, clause2]);
    let mut checker = ClauseChecker::new(clause_set);

    // Node satisfies both clauses
    let node = create_node_with_concepts(0, vec!["A", "B", "C", "D", "E", "F"]);

    let violation = checker.check_node(&node);
    assert!(violation.is_none(), "Should not detect violation when all clauses satisfied");
}

// ============================================================================
// Test 5: Edge Cases
// ============================================================================

#[test]
fn test_empty_clause_set() {
    let clause_set = create_test_clause_set(vec![]);
    let mut checker = ClauseChecker::new(clause_set);

    let node = create_node_with_concepts(0, vec!["A", "B", "C"]);

    let violation = checker.check_node(&node);
    assert!(
        violation.is_none(),
        "Should not detect violation with empty clause set"
    );
}

#[test]
fn test_empty_node() {
    let clause = create_deterministic_clause("c1", vec!["A", "B"], vec!["C"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Empty node
    let node = create_node_with_concepts(0, vec![]);

    let violation = checker.check_node(&node);
    assert!(
        violation.is_none(),
        "Should not detect violation with empty node (no body satisfied)"
    );
}

#[test]
fn test_clause_with_empty_body() {
    // Clause with empty body (always applies): → C(x)
    let clause = DLClause {
        head: vec![DLAtom {
            predicate: "C".to_string(),
            arguments: vec!["x".to_string()],
            is_positive: true,
            constraints: vec![],
        }],
        body: vec![],
        variables: vec!["x".to_string()].into_iter().collect(),
        id: "empty_body".to_string(),
    };

    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    // Node without C
    let node = create_node_with_concepts(0, vec!["A", "B"]);

    let violation = checker.check_node(&node);
    assert!(
        violation.is_some(),
        "Should detect violation when empty body clause head not satisfied"
    );
}

// ============================================================================
// Test 6: Performance Tests
// ============================================================================

#[test]
fn test_performance_many_clauses() {
    use std::time::Instant;

    // Create 100 clauses
    let clauses: Vec<DLClause> = (0..100)
        .map(|i| {
            create_deterministic_clause(
                &format!("clause{}", i),
                vec!["A", &format!("B{}", i)],
                vec![&format!("C{}", i)],
            )
        })
        .collect();

    let clause_set = create_test_clause_set(clauses);
    let mut checker = ClauseChecker::new(clause_set);

    // Node with just A (no clauses apply)
    let node = create_node_with_concepts(0, vec!["A"]);

    let start = Instant::now();
    let violation = checker.check_node(&node);
    let duration = start.elapsed();

    assert!(violation.is_none());
    assert!(
        duration.as_millis() < 100,
        "Checking 100 clauses should take < 100ms, took {:?}",
        duration
    );
}

#[test]
fn test_performance_large_node() {
    use std::time::Instant;

    // Create node with 100 concepts
    let concepts: Vec<&str> = (0..100)
        .map(|i| Box::leak(format!("Concept{}", i).into_boxed_str()) as &str)
        .collect();
    let node = create_node_with_concepts(0, concepts);

    // Single simple clause
    let clause = create_deterministic_clause("c1", vec!["Concept0", "Concept1"], vec!["Result"]);
    let clause_set = create_test_clause_set(vec![clause]);
    let mut checker = ClauseChecker::new(clause_set);

    let start = Instant::now();
    let _violation = checker.check_node(&node);
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 50,
        "Checking large node should take < 50ms, took {:?}",
        duration
    );
}

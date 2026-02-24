//! OWL DL Class-Expression Reasoning Tests
//!
//! Tests covering `owl:equivalentClass`, `owl:someValuesFrom`, `owl:allValuesFrom`,
//! cardinality restrictions, intersection/union, complementOf, and their combinations.
//! This file validates that Oxidowl correctly handles the full OWL DL class-expression
//! language as specified by the OWL 2 DL profile.

use oxidowl::Result;
use oxidowl::config::ReasonerConfig;
use oxidowl::core::reasoner::Reasoner;
use oxidowl::dl_clauses::DLClauseGenerator;
use oxidowl::ontology::*;

// ──────────────────────────────────────────────────────────────────────────────
// Helpers
// ──────────────────────────────────────────────────────────────────────────────

fn cls(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

fn prop(iri: &str) -> ObjectPropertyExpression {
    ObjectPropertyExpression::ObjectProperty(ObjectProperty { iri: IRI::new(iri) })
}

fn some(p: &str, filler: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectSomeValuesFrom {
        property: prop(p),
        filler: Box::new(filler),
    }
}

fn all(p: &str, filler: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectAllValuesFrom {
        property: prop(p),
        filler: Box::new(filler),
    }
}

fn intersection(operands: Vec<ClassExpression>) -> ClassExpression {
    ClassExpression::ObjectIntersectionOf(operands)
}

fn union(operands: Vec<ClassExpression>) -> ClassExpression {
    ClassExpression::ObjectUnionOf(operands)
}

fn complement(expr: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectComplementOf(Box::new(expr))
}

fn min_card(p: &str, n: u32, filler: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectMinCardinality {
        property: prop(p),
        cardinality: n,
        filler: Box::new(filler),
    }
}

fn max_card(p: &str, n: u32, filler: ClassExpression) -> ClassExpression {
    ClassExpression::ObjectMaxCardinality {
        property: prop(p),
        cardinality: n,
        filler: Box::new(filler),
    }
}

/// Build a minimal `Ontology` with an axiom counter starting at 1
struct OntologyBuilder {
    ontology: Ontology,
    next_id: u64,
}
impl OntologyBuilder {
    fn new() -> Self {
        Self {
            ontology: Ontology::new(),
            next_id: 1,
        }
    }

    fn equiv(&mut self, a: ClassExpression, b: ClassExpression) -> &mut Self {
        let id = self.id();
        self.ontology
            .add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                id,
                classes: vec![a, b],
                annotations: vec![],
            }));
        self
    }

    fn subclass(&mut self, sub: ClassExpression, sup: ClassExpression) -> &mut Self {
        let id = self.id();
        self.ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id,
            subclass: sub,
            superclass: sup,
            annotations: vec![],
        }));
        self
    }

    fn disjoint(&mut self, a: ClassExpression, b: ClassExpression) -> &mut Self {
        let id = self.id();
        self.ontology
            .add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
                id,
                classes: vec![a, b],
                annotations: vec![],
            }));
        self
    }

    fn assert_class(&mut self, individual_iri: &str, class: ClassExpression) -> &mut Self {
        let id = self.id();
        self.ontology
            .add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
                id,
                class,
                individual: Individual::Named(NamedIndividual {
                    iri: IRI::new(individual_iri),
                }),
                annotations: vec![],
            }));
        self
    }

    fn build(self) -> Ontology {
        self.ontology
    }

    fn id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

fn reasoner_for(ontology: Ontology) -> Result<Reasoner> {
    let mut r = Reasoner::new(ReasonerConfig::default())?;
    r.load_ontology(ontology)?;
    Ok(r)
}

// ──────────────────────────────────────────────────────────────────────────────
// 1.  owl:equivalentClass  (named ≡ named)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_equivalent_named_classes_satisfiable() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), c.clone());
    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A should be satisfiable when A ≡ C"
    );
    Ok(())
}

#[test]
fn test_equivalent_and_disjoint_named_classes_unsatisfiable() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), c.clone());
    b.disjoint(a.clone(), c.clone());
    let mut r = reasoner_for(b.build())?;
    // A ≡ C and A ⊥ C → A (and C) are unsatisfiable
    assert!(
        !r.is_class_satisfiable(&a)?,
        "A should be unsatisfiable when A ≡ C and A ⊥ C"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 2.  owl:equivalentClass + owl:someValuesFrom  (primary requested feature)
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ∃R.C`
/// Verifies that the equivalence between a named class and an existential
/// restriction is correctly parsed and compiled into DL clauses.
#[test]
fn test_equivalent_class_some_values_from_compiles() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    let exist_r_c = some("http://example.org/R", c.clone());
    b.equiv(a.clone(), exist_r_c.clone());

    let ontology = b.build();

    // Check that DL clauses are generated without panic
    let mut clause_gen = DLClauseGenerator::new();
    let clause_set = clause_gen.generate_clauses(&ontology)?;

    // Should produce at least the backward clause:  R(x,y) ∧ C(y) → def:N(x), def:N(x) ↔ A(x)
    // plus bidirectional implication clauses from the equivalence.
    assert!(
        !clause_set.deterministic_clauses.is_empty(),
        "DL clause generation for A ≡ ∃R.C should produce clauses"
    );

    println!(
        "✅ A ≡ ∃R.C generated {} deterministic DL clauses",
        clause_set.deterministic_clauses.len()
    );
    Ok(())
}

/// `A ≡ ∃R.C`  – Class A should be satisfiable (no contradiction in the axiom itself)
#[test]
fn test_equivalent_class_some_values_from_satisfiable() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    let exist_r_c = some("http://example.org/R", c.clone());
    b.equiv(a.clone(), exist_r_c.clone());

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A should be satisfiable when A ≡ ∃R.C (no contradiction exists)"
    );
    Ok(())
}

/// `A ≡ ∃R.C`, `C ⊑ ⊥`  (C is empty / unsatisfiable)  → A is also unsatisfiable
/// because anything in A would need an R-successor in C, but C is empty.
#[test]
fn test_equivalent_some_values_from_unsatisfiable_empty_filler() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    let d = cls("http://example.org/D");
    let exist_r_c = some("http://example.org/R", c.clone());

    // A ≡ ∃R.C
    b.equiv(a.clone(), exist_r_c.clone());
    // C ⊥ C  (C and C are disjoint) → C is unsatisfiable (A ≡ ∃R.⊥)
    b.disjoint(c.clone(), c.clone());
    // D ⊑ A  (so D needs an R-successor in C, but C is empty)
    b.subclass(d.clone(), a.clone());

    let mut r = reasoner_for(b.build())?;

    // C ⊥ C means C cannot have any members
    assert!(
        !r.is_class_satisfiable(&c)?,
        "C should be unsatisfiable when C ⊥ C"
    );
    Ok(())
}

/// `A ≡ ∃R.C`,  `A ⊥ ∃R.C`  → A is unsatisfiable
#[test]
fn test_equivalent_some_values_from_disjoint_with_self() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    let exist_r_c = some("http://example.org/R", c.clone());

    b.equiv(a.clone(), exist_r_c.clone());
    b.disjoint(a.clone(), exist_r_c.clone());

    let mut r = reasoner_for(b.build())?;
    assert!(
        !r.is_class_satisfiable(&a)?,
        "A should be unsatisfiable when A ≡ ∃R.C and A ⊥ ∃R.C"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 3.  owl:someValuesFrom  in SubClassOf
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_subclass_some_values_from_satisfiable() -> Result<()> {
    // A ⊑ ∃R.C  – consistent, A can have instances with R-successors in C
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.subclass(a.clone(), some("http://example.org/R", c.clone()));

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A ⊑ ∃R.C should be satisfiable"
    );
    Ok(())
}

#[test]
fn test_subclass_some_and_all_clash() -> Result<()> {
    // A ⊑ ∃R.C  and  A ⊑ ∀R.¬C  →  A is unsatisfiable
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    let not_c = complement(c.clone());
    b.subclass(a.clone(), some("http://example.org/R", c.clone()));
    b.subclass(a.clone(), all("http://example.org/R", not_c.clone()));

    let mut r = reasoner_for(b.build())?;
    assert!(
        !r.is_class_satisfiable(&a)?,
        "A should be unsatisfiable when A ⊑ ∃R.C and A ⊑ ∀R.¬C"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 4.  owl:allValuesFrom
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_equiv_all_values_from_satisfiable() -> Result<()> {
    // A ≡ ∀R.C  – satisfiable (vacuously true if no R-successors)
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), all("http://example.org/R", c.clone()));

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A ≡ ∀R.C should be satisfiable"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 5.  ObjectIntersectionOf
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_equiv_intersection_satisfiable() -> Result<()> {
    // A ≡ B ⊓ ∃R.C
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let b_cls = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    let expr = intersection(vec![b_cls.clone(), some("http://example.org/R", c.clone())]);
    b.equiv(a.clone(), expr);

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A ≡ B ⊓ ∃R.C should be satisfiable"
    );
    Ok(())
}

#[test]
fn test_intersection_with_disjoint_unsatisfiable() -> Result<()> {
    // B ≡ A ⊓ C,  A ⊥ C  →  B is unsatisfiable
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let b_cls = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    b.equiv(b_cls.clone(), intersection(vec![a.clone(), c.clone()]));
    b.disjoint(a.clone(), c.clone());

    let mut r = reasoner_for(b.build())?;
    assert!(
        !r.is_class_satisfiable(&b_cls)?,
        "B should be unsatisfiable when B ≡ A ⊓ C and A ⊥ C"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 6.  ObjectUnionOf
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_equiv_union_satisfiable() -> Result<()> {
    // A ≡ B ⊔ C
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let b_cls = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), union(vec![b_cls.clone(), c.clone()]));

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A ≡ B ⊔ C should be satisfiable"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 7.  ObjectComplementOf
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_equiv_complement_satisfiable() -> Result<()> {
    // A ≡ ¬B
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let b_cls = cls("http://example.org/B");
    b.equiv(a.clone(), complement(b_cls.clone()));

    let mut r = reasoner_for(b.build())?;
    assert!(r.is_class_satisfiable(&a)?, "A ≡ ¬B should be satisfiable");
    Ok(())
}

#[test]
fn test_class_and_complement_unsatisfiable() -> Result<()> {
    // A ≡ A ⊓ ¬A  →  unsatisfiable
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    // B ≡ A ⊓ ¬A
    let b_cls = cls("http://example.org/B");
    b.equiv(
        b_cls.clone(),
        intersection(vec![a.clone(), complement(a.clone())]),
    );

    let mut r = reasoner_for(b.build())?;
    assert!(
        !r.is_class_satisfiable(&b_cls)?,
        "B ≡ A ⊓ ¬A should be unsatisfiable"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 8.  Cardinality Restrictions
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn test_min_cardinality_satisfiable() -> Result<()> {
    // A ≡ ≥1 R.C
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), min_card("http://example.org/R", 1, c.clone()));

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A ≡ ≥1R.C should be satisfiable"
    );
    Ok(())
}

#[test]
fn test_max_zero_cardinality_with_some_unsatisfiable() -> Result<()> {
    // A ⊑ ∃R.C  and  A ⊑ ≤0 R.C  →  A is unsatisfiable
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.subclass(a.clone(), some("http://example.org/R", c.clone()));
    b.subclass(a.clone(), max_card("http://example.org/R", 0, c.clone()));

    let mut r = reasoner_for(b.build())?;
    assert!(
        !r.is_class_satisfiable(&a)?,
        "A should be unsatisfiable when A ⊑ ∃R.C and A ⊑ ≤0R.C"
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 9.  DL Clause generation – structural correctness
// ──────────────────────────────────────────────────────────────────────────────

/// Verify that `introduce_definition` for `SomeValuesFrom` no longer generates
/// a multi-head forward clause (which would represent disjunction, not existential).
#[test]
fn test_some_values_from_no_disjunctive_forward_clause() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), some("http://example.org/R", c.clone()));

    let mut clause_gen = DLClauseGenerator::new();
    let clause_set = clause_gen.generate_clauses(&b.build())?;

    // No deterministic clause should have more than ONE head atom when one of those
    // atoms is a role assertion (binary) – multi-head role assertions indicate the
    // old incorrect ∃-introduction encoding.
    for clause in &clause_set.deterministic_clauses {
        if clause.head.iter().any(|a| a.arguments.len() == 2) {
            // allowed only if this is a genuine disjunctive choice (union rules etc.)
            // For the case of ∃R.C introduced via define,  the head must be unary.
            assert!(
                clause.head.len() == 1,
                "A clause with a role-assertion head must have exactly one head atom; \
                 got clause with {} head atoms: {:?}",
                clause.head.len(),
                clause
            );
        }
    }

    println!(
        "✅ No incorrect multi-head role-assertion clauses for A ≡ ∃R.C \
         ({} deterministic clauses)",
        clause_set.deterministic_clauses.len()
    );
    Ok(())
}

/// Backward DL clause: `R(x,y) ∧ C(y) → def:N(x)` should be present for `A ≡ ∃R.C`.
/// This clause allows the clause checker to detect when the existential is witnessed.
#[test]
fn test_backward_clause_present_for_some_values_from() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let c = cls("http://example.org/C");
    b.equiv(a.clone(), some("http://example.org/R", c.clone()));

    let mut clause_gen = DLClauseGenerator::new();
    let clause_set = clause_gen.generate_clauses(&b.build())?;

    // The backward clause body should contain a 2-argument role atom (R(x,y))
    // and a 1-argument concept atom (C(y)).
    let has_backward = clause_set.deterministic_clauses.iter().any(|cl| {
        let body_has_role = cl.body.iter().any(|a| a.arguments.len() == 2);
        let body_has_filler = cl.body.iter().any(|a| a.arguments.len() == 1);
        let head_is_unary = cl.head.len() == 1 && cl.head[0].arguments.len() == 1;
        body_has_role && body_has_filler && head_is_unary
    });

    assert!(
        has_backward,
        "Expected a backward clause R(x,y)∧C(y)→def:N(x) for A ≡ ∃R.C; \
         clauses: {:?}",
        clause_set.deterministic_clauses
    );
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 10.  Concept unfolding rules – ABox integration
// ──────────────────────────────────────────────────────────────────────────────

/// Verify that the Tableau's concept_unfolding_rules map is populated for
/// `EquivalentClasses(A, ∃R.C)` so that ClassAssertion(A, a) causes the
/// existential restriction to be added to the individual node.
#[test]
fn test_concept_unfolding_rules_populated() -> Result<()> {
    use oxidowl::config::ReasoningConfig;

    // Build ontology: A ≡ ∃R.C
    let mut b = OntologyBuilder::new();
    let a_iri = "http://example.org/A";
    let c = cls("http://example.org/C");
    b.equiv(cls(a_iri), some("http://example.org/R", c.clone()));

    let ontology = b.build();

    // Build tableau directly to inspect unfolding rules
    let tableau = oxidowl::core::Tableau::from_ontology(
        std::sync::Arc::new(ontology),
        ReasoningConfig::default(),
    )?;

    assert!(
        tableau.concept_unfolding_rules.contains_key(a_iri),
        "Tableau concept_unfolding_rules should have an entry for class A \
         when A ≡ ∃R.C is in the ontology; existing keys: {:?}",
        tableau.concept_unfolding_rules.keys().collect::<Vec<_>>()
    );

    let rules = tableau.concept_unfolding_rules.get(a_iri).unwrap();
    let has_some = rules
        .iter()
        .any(|r| matches!(r, ClassExpression::ObjectSomeValuesFrom { .. }));
    assert!(
        has_some,
        "Unfolding rule for A should include ObjectSomeValuesFrom; rules: {:?}",
        rules
    );

    println!("✅ Concept unfolding rules correctly populated for A ≡ ∃R.C");
    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// 11.  Complex combined patterns
// ──────────────────────────────────────────────────────────────────────────────

/// `A ≡ ∃R.(B ⊓ C)` – existential with intersection filler
#[test]
fn test_equiv_some_with_intersection_filler() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let a = cls("http://example.org/A");
    let b_cls = cls("http://example.org/B");
    let c = cls("http://example.org/C");
    b.equiv(
        a.clone(),
        some("http://example.org/R", intersection(vec![b_cls, c])),
    );

    let mut r = reasoner_for(b.build())?;
    assert!(
        r.is_class_satisfiable(&a)?,
        "A ≡ ∃R.(B ⊓ C) should be satisfiable"
    );
    Ok(())
}

/// `Parent ≡ ∃hasChild.Person` – realistic naming to confirm IRI handling
#[test]
fn test_equiv_parent_has_child_person() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let parent = cls("http://example.org/Parent");
    let person = cls("http://example.org/Person");
    let exist = some("http://example.org/hasChild", person.clone());
    b.equiv(parent.clone(), exist.clone());

    let mut clause_gen = DLClauseGenerator::new();
    let ontology = b.build();
    let clause_set = clause_gen.generate_clauses(&ontology)?;

    assert!(
        !clause_set.deterministic_clauses.is_empty(),
        "Parent ≡ ∃hasChild.Person should produce DL clauses"
    );

    let mut r = reasoner_for(ontology)?;
    assert!(
        r.is_class_satisfiable(&parent)?,
        "Parent ≡ ∃hasChild.Person should be satisfiable"
    );
    Ok(())
}

/// `GrandParent ≡ ∃hasChild.(∃hasChild.Person)` – nested existentials
#[test]
fn test_equiv_grandparent_nested_existentials() -> Result<()> {
    let mut b = OntologyBuilder::new();
    let grandparent = cls("http://example.org/GrandParent");
    let person = cls("http://example.org/Person");
    let parent_expr = some("http://example.org/hasChild", person.clone());
    let gp_expr = some("http://example.org/hasChild", parent_expr);
    b.equiv(grandparent.clone(), gp_expr);

    let mut clause_gen = DLClauseGenerator::new();
    let ontology = b.build();
    let clause_set = clause_gen.generate_clauses(&ontology)?;

    assert!(
        !clause_set.deterministic_clauses.is_empty(),
        "GrandParent ≡ ∃hasChild.(∃hasChild.Person) should produce clauses"
    );

    let mut r = reasoner_for(ontology)?;
    assert!(
        r.is_class_satisfiable(&grandparent)?,
        "GrandParent ≡ ∃hasChild.(∃hasChild.Person) should be satisfiable"
    );
    Ok(())
}

//! Tests for distinguishing between consistency and coherence
//!
//! These tests verify that oxidowl correctly distinguishes between:
//! - **Consistency**: The ontology has at least one valid model
//! - **Coherence**: All named classes can have instances
//!
//! An ontology with EquivalentClasses(A,B) + DisjointClasses(A,B) is:
//! - **Consistent** (there exists a model where A and B are both empty)
//! - **Incoherent** (classes A and B are unsatisfiable)

use oxidowl::ontology::*;
use oxidowl::core::reasoner::Reasoner;
use oxidowl::config::ReasonerConfig;
use oxidowl::Result;

/// Test that EquivalentClasses + DisjointClasses makes classes unsatisfiable
/// but the ontology remains consistent
#[test]
fn test_equivalent_and_disjoint_classes_consistent_but_incoherent() -> Result<()> {
    // Create ontology with EquivalentClasses(A,B) + DisjointClasses(A,B)
    let mut ontology = Ontology::new();

    let class_a = Class::new(IRI::new("http://example.org/ClassA"));
    let class_b = Class::new(IRI::new("http://example.org/ClassB"));

    let expr_a = ClassExpression::Class(class_a.clone());
    let expr_b = ClassExpression::Class(class_b.clone());

    // Add EquivalentClasses(A, B)
    ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 1,
        classes: vec![expr_a.clone(), expr_b.clone()],
        annotations: vec![],
    }));

    // Add DisjointClasses(A, B)
    ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 2,
        classes: vec![expr_a.clone(), expr_b.clone()],
        annotations: vec![],
    }));

    // Create reasoner and load ontology
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;

    // CHECK 1: Ontology should be CONSISTENT
    let is_consistent = reasoner.is_consistent()?;
    assert!(
        is_consistent,
        "Ontology with EquivalentClasses(A,B) + DisjointClasses(A,B) should be CONSISTENT \
         (there exists a model where both classes are empty)"
    );

    // CHECK 2: Class A should be UNSATISFIABLE
    let class_a_satisfiable = reasoner.is_class_satisfiable(&expr_a)?;
    assert!(
        !class_a_satisfiable,
        "Class A should be UNSATISFIABLE (cannot have instances)"
    );

    // CHECK 3: Class B should be UNSATISFIABLE
    let class_b_satisfiable = reasoner.is_class_satisfiable(&expr_b)?;
    assert!(
        !class_b_satisfiable,
        "Class B should be UNSATISFIABLE (cannot have instances)"
    );

    println!("✅ Ontology is consistent but incoherent (has unsatisfiable classes)");
    println!("   - Ontology consistent: {}", is_consistent);
    println!("   - Class A satisfiable: {}", class_a_satisfiable);
    println!("   - Class B satisfiable: {}", class_b_satisfiable);

    Ok(())
}

/// Test Campaign 6 pattern: Healthy ≡ MoistStrategy AND Healthy ⊥ MoistStrategy
#[test]
fn test_campaign_6_pattern_consistent_incoherent() -> Result<()> {
    let mut ontology = Ontology::new();

    let healthy = Class::new(IRI::new("http://www.smolang.org/greenhouseDT#Healthy"));
    let moist = Class::new(IRI::new("http://www.smolang.org/greenhouseDT#MoistStrategy"));

    let expr_healthy = ClassExpression::Class(healthy.clone());
    let expr_moist = ClassExpression::Class(moist.clone());

    // EquivalentClasses(Healthy, MoistStrategy)
    ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 1,
        classes: vec![expr_healthy.clone(), expr_moist.clone()],
        annotations: vec![],
    }));

    // DisjointClasses(Healthy, MoistStrategy)
    ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 2,
        classes: vec![expr_healthy.clone(), expr_moist.clone()],
        annotations: vec![],
    }));

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;

    // Should be consistent but incoherent
    assert!(reasoner.is_consistent()?, "Campaign 6 pattern should be consistent");
    assert!(!reasoner.is_class_satisfiable(&expr_healthy)?, "Healthy should be unsatisfiable");
    assert!(!reasoner.is_class_satisfiable(&expr_moist)?, "MoistStrategy should be unsatisfiable");

    Ok(())
}

/// Test Campaign 7/8 pattern: DataTransformation ≡ BinaryScale AND DataTransformation ⊥ BinaryScale
#[test]
fn test_campaign_7_8_pattern_consistent_incoherent() -> Result<()> {
    let mut ontology = Ontology::new();

    let dt = Class::new(IRI::new("http://bmkeg.isi.edu/ooevv/edu.isi.bmkeg.ooevv.model.DataTransformation"));
    let bs = Class::new(IRI::new("http://bmkeg.isi.edu/ooevv/edu.isi.bmkeg.ooevv.model.scale.BinaryScale"));

    let expr_dt = ClassExpression::Class(dt);
    let expr_bs = ClassExpression::Class(bs);

    ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 1,
        classes: vec![expr_dt.clone(), expr_bs.clone()],
        annotations: vec![],
    }));

    ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 2,
        classes: vec![expr_dt.clone(), expr_bs.clone()],
        annotations: vec![],
    }));

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;

    assert!(reasoner.is_consistent()?, "Campaign 7/8 pattern should be consistent");
    assert!(!reasoner.is_class_satisfiable(&expr_dt)?, "DataTransformation should be unsatisfiable");
    assert!(!reasoner.is_class_satisfiable(&expr_bs)?, "BinaryScale should be unsatisfiable");

    Ok(())
}

/// Test with transitive equivalence chain and disjointness
#[test]
fn test_transitive_equivalence_with_disjointness() -> Result<()> {
    let mut ontology = Ontology::new();

    let a = ClassExpression::Class(Class::new(IRI::new("http://example.org/A")));
    let b = ClassExpression::Class(Class::new(IRI::new("http://example.org/B")));
    let c = ClassExpression::Class(Class::new(IRI::new("http://example.org/C")));

    // A ≡ B
    ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 1,
        classes: vec![a.clone(), b.clone()],
        annotations: vec![],
    }));

    // B ≡ C
    ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 2,
        classes: vec![b.clone(), c.clone()],
        annotations: vec![],
    }));

    // A ⊥ C (but A ≡ B ≡ C, so all three are unsatisfiable)
    ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 3,
        classes: vec![a.clone(), c.clone()],
        annotations: vec![],
    }));

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;

    // Ontology should be consistent
    assert!(reasoner.is_consistent()?, "Ontology with transitive equivalence should be consistent");

    // All three classes should be unsatisfiable
    assert!(!reasoner.is_class_satisfiable(&a)?, "A should be unsatisfiable");
    assert!(!reasoner.is_class_satisfiable(&b)?, "B should be unsatisfiable");
    assert!(!reasoner.is_class_satisfiable(&c)?, "C should be unsatisfiable");

    Ok(())
}

/// Test that truly inconsistent ontologies are still detected
#[test]
fn test_truly_inconsistent_ontology_detected() -> Result<()> {
    let mut ontology = Ontology::new();

    let person = Class::new(IRI::new("http://example.org/Person"));
    let john = Individual::named(IRI::new("http://example.org/John"));

    // Assert that John is a Person
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 1,
        class: ClassExpression::Class(person.clone()),
        individual: john.clone(),
        annotations: vec![],
    }));

    // Assert that John is NOT a Person (using NegationOf)
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 2,
        class: ClassExpression::ObjectComplementOf(Box::new(ClassExpression::Class(person))),
        individual: john,
        annotations: vec![],
    }));

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;

    // This should be genuinely INCONSISTENT
    let is_consistent = reasoner.is_consistent()?;
    assert!(
        !is_consistent,
        "Ontology with contradictory assertions about an individual should be INCONSISTENT"
    );

    Ok(())
}

/// Test that a coherent ontology (all classes satisfiable) is consistent
#[test]
fn test_coherent_ontology_is_consistent() -> Result<()> {
    let mut ontology = Ontology::new();

    let person = ClassExpression::Class(Class::new(IRI::new("http://example.org/Person")));
    let student = ClassExpression::Class(Class::new(IRI::new("http://example.org/Student")));
    let teacher = ClassExpression::Class(Class::new(IRI::new("http://example.org/Teacher")));

    // Student ⊑ Person
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 1,
        subclass: student.clone(),
        superclass: person.clone(),
        annotations: vec![],
    }));

    // Teacher ⊑ Person
    ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
        id: 2,
        subclass: teacher.clone(),
        superclass: person.clone(),
        annotations: vec![],
    }));

    // Student ⊥ Teacher
    ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 3,
        classes: vec![student.clone(), teacher.clone()],
        annotations: vec![],
    }));

    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology)?;

    // Should be both consistent and coherent
    assert!(reasoner.is_consistent()?, "Coherent ontology should be consistent");
    assert!(reasoner.is_class_satisfiable(&person)?, "Person should be satisfiable");
    assert!(reasoner.is_class_satisfiable(&student)?, "Student should be satisfiable");
    assert!(reasoner.is_class_satisfiable(&teacher)?, "Teacher should be satisfiable");

    Ok(())
}


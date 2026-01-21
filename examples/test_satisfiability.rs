use oxidowl::Reasoner;
use oxidowl::prelude::*;

fn main() -> Result<()> {
    let mut ontology = Ontology::new();

    let class_a = Class::new(IRI::new("http://example.org/A"));
    let class_b = Class::new(IRI::new("http://example.org/B"));
    let expr_a = ClassExpression::Class(class_a.clone());
    let expr_b = ClassExpression::Class(class_b.clone());

    // EquivalentClasses(A, B)
    ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
        id: 1,
        classes: vec![expr_a.clone(), expr_b.clone()],
        annotations: vec![],
    }));

    // DisjointClasses(A, B)
    ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
        id: 2,
        classes: vec![expr_a.clone(), expr_b.clone()],
        annotations: vec![],
    }));

    // Create reasoner
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology.clone())?;

    println!("Ontology consistent: {}", reasoner.is_consistent()?);

    // Now add a class assertion for A
    let test_ind = Individual::named(IRI::new("http://example.org/test"));
    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 3,
        class: expr_a.clone(),
        individual: test_ind,
        annotations: vec![],
    }));

    let mut reasoner2 = Reasoner::new(ReasonerConfig::default())?;
    reasoner2.load_ontology(ontology)?;

    println!(
        "Ontology with A(test) consistent: {}",
        reasoner2.is_consistent()?
    );
    println!("A satisfiable: {}", reasoner.is_class_satisfiable(&expr_a)?);

    Ok(())
}

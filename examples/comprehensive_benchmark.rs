use oxidowl::Reasoner;
use oxidowl::prelude::*;
use std::time::Instant;

fn main() -> Result<()> {
    println!("=== Oxidowl Performance Benchmark ===\n");

    // Test 1: Simple ontology without equivalence/disjointness
    {
        let mut ontology = Ontology::new();
        for i in 0..50 {
            let class_a = Class::new(IRI::new(&format!("http://example.org/Class{}", i)));
            let class_b = Class::new(IRI::new(&format!("http://example.org/Class{}", i + 1)));
            ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                id: i as u64,
                subclass: ClassExpression::Class(class_a),
                superclass: ClassExpression::Class(class_b),
                annotations: vec![],
            }));
        }

        let iterations = 50;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
            reasoner.load_ontology(ontology.clone())?;
            let _ = reasoner.is_consistent()?;
        }
        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() as f64 / iterations as f64;

        println!("Test 1: Simple ontology (50 SubClassOf axioms, no individuals)");
        println!("  Average time: {:.2} ms", avg_ms);
        println!(
            "  Status: {}",
            if avg_ms < 10.0 {
                "✓ Excellent"
            } else if avg_ms < 50.0 {
                "⚠ Acceptable"
            } else {
                "✗ Slow"
            }
        );
        println!();
    }

    // Test 2: Ontology with equivalence and disjointness (but no individuals)
    {
        let mut ontology = Ontology::new();
        for i in 0..10 {
            let class_a = Class::new(IRI::new(&format!("http://example.org/A{}", i)));
            let class_b = Class::new(IRI::new(&format!("http://example.org/B{}", i)));

            ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
                id: (i * 2) as u64,
                classes: vec![
                    ClassExpression::Class(class_a.clone()),
                    ClassExpression::Class(class_b.clone()),
                ],
                annotations: vec![],
            }));

            let class_c = Class::new(IRI::new(&format!("http://example.org/C{}", i)));
            ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
                id: (i * 2 + 1) as u64,
                classes: vec![
                    ClassExpression::Class(class_b),
                    ClassExpression::Class(class_c),
                ],
                annotations: vec![],
            }));
        }

        let iterations = 20;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
            reasoner.load_ontology(ontology.clone())?;
            let _ = reasoner.is_consistent()?;
        }
        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() as f64 / iterations as f64;

        println!("Test 2: With EquivalentClasses & DisjointClasses (20 axioms, no individuals)");
        println!("  Average time: {:.2} ms", avg_ms);
        println!(
            "  Status: {}",
            if avg_ms < 20.0 {
                "✓ Excellent"
            } else if avg_ms < 100.0 {
                "⚠ Acceptable"
            } else {
                "✗ Slow"
            }
        );
        println!();
    }

    // Test 3: Ontology with individual (triggers clash detection)
    {
        let mut ontology = Ontology::new();
        let class_a = Class::new(IRI::new("http://example.org/A"));
        let class_b = Class::new(IRI::new("http://example.org/B"));

        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 1,
            classes: vec![
                ClassExpression::Class(class_a.clone()),
                ClassExpression::Class(class_b.clone()),
            ],
            annotations: vec![],
        }));

        ontology.add_axiom(Axiom::DisjointClasses(DisjointClassesAxiom {
            id: 2,
            classes: vec![
                ClassExpression::Class(class_a.clone()),
                ClassExpression::Class(class_b),
            ],
            annotations: vec![],
        }));

        let john = Individual::named(IRI::new("http://example.org/John"));
        ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
            id: 3,
            class: ClassExpression::Class(class_a),
            individual: john,
            annotations: vec![],
        }));

        let iterations = 20;
        let start = Instant::now();
        for _ in 0..iterations {
            let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
            reasoner.load_ontology(ontology.clone())?;
            let _ = reasoner.is_consistent()?;
        }
        let elapsed = start.elapsed();
        let avg_ms = elapsed.as_millis() as f64 / iterations as f64;

        println!("Test 3: With individual causing equivalence-disjointness clash");
        println!("  Average time: {:.2} ms", avg_ms);
        println!(
            "  Status: {}",
            if avg_ms < 20.0 {
                "✓ Excellent"
            } else if avg_ms < 100.0 {
                "⚠ Acceptable"
            } else {
                "✗ Slow"
            }
        );
        println!();
    }

    println!("=== Benchmark Complete ===");
    Ok(())
}

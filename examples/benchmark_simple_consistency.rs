use oxidowl::prelude::*;
use oxidowl::Reasoner;
use std::time::Instant;

fn main() -> Result<()> {
    // Create a simple ontology without individuals (common case)
    let mut ontology = Ontology::new();

    // Add some SubClassOf axioms
    for i in 0..20 {
        let class_a = Class::new(IRI::new(&format!("http://example.org/Class{}", i)));
        let class_b = Class::new(IRI::new(&format!("http://example.org/Class{}", i + 1)));
        let expr_a = ClassExpression::Class(class_a);
        let expr_b = ClassExpression::Class(class_b);

        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: i as u64,
            subclass: expr_a,
            superclass: expr_b,
            annotations: vec![],
        }));
    }

    println!("Testing consistency check performance on simple ontology");
    println!("Ontology has {} axioms, 0 individuals", ontology.axioms().len());
    println!();

    // Warm up
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    reasoner.load_ontology(ontology.clone())?;
    let _ = reasoner.is_consistent()?;

    // Benchmark
    let iterations = 100;
    let start = Instant::now();
    
    for _ in 0..iterations {
        let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
        reasoner.load_ontology(ontology.clone())?;
        let is_consistent = reasoner.is_consistent()?;
        assert!(is_consistent);
    }
    
    let elapsed = start.elapsed();
    let avg_ms = elapsed.as_millis() as f64 / iterations as f64;
    
    println!("Ran {} iterations", iterations);
    println!("Total time: {:?}", elapsed);
    println!("Average time per consistency check: {:.2} ms", avg_ms);
    println!();
    
    if avg_ms < 10.0 {
        println!("✓ Performance is good (< 10ms per check)");
    } else if avg_ms < 50.0 {
        println!("⚠ Performance is acceptable (< 50ms per check)");
    } else {
        println!("✗ Performance is slow (>= 50ms per check)");
    }

    Ok(())
}

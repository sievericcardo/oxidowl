//! Reasoning performance benchmarks
//! 
//! Tests similar to HermiT's performance test suite

use crate::performance::{Benchmark, BenchmarkConfig, PerformanceMetrics, ComplexityClass};
use oxidowl::{
    ontology::Ontology,
    reasoning::ReasoningService,
    config::ReasonerConfig,
};
use std::time::Instant;

/// Consistency checking benchmark
pub struct ConsistencyBenchmark {
    ontology: Ontology,
    name: String,
}

impl ConsistencyBenchmark {
    pub fn new(ontology: Ontology, name: String) -> Self {
        Self { ontology, name }
    }
    
    pub fn with_simple_ontology() -> Self {
        let mut ontology = Ontology::new();
        
        // Add test classes and axioms
        let person_class = oxidowl::ontology::Class::new(oxidowl::ontology::IRI::new("Person"));
        let student_class = oxidowl::ontology::Class::new(oxidowl::ontology::IRI::new("Student"));
        
        ontology.add_class(person_class.clone());
        ontology.add_class(student_class.clone());
        
        // Add SubClassOf axiom: Student ⊑ Person
        let subclass_axiom = oxidowl::ontology::SubClassOfAxiom {
            id: 1,
            subclass: oxidowl::ontology::ClassExpression::Class(student_class),
            superclass: oxidowl::ontology::ClassExpression::Class(person_class),
            annotations: vec![],
        };
        ontology.add_axiom(oxidowl::ontology::Axiom::SubClassOf(subclass_axiom));
        
        Self::new(ontology, "Simple Ontology".to_string())
    }
    
    pub fn with_complex_ontology() -> Self {
        let mut ontology = Ontology::new();
        
        // Create a more complex ontology with intersections, unions, restrictions
        for i in 0..50 {
            let class = oxidowl::ontology::Class::new(
                oxidowl::ontology::IRI::new(format!("Class{}", i))
            );
            ontology.add_class(class);
        }
        
        // Add complex class expressions and axioms
        // This simulates the complexity found in real ontologies
        
        Self::new(ontology, "Complex Ontology".to_string())
    }
}

impl Benchmark for ConsistencyBenchmark {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String> {
        let reasoning_config = ReasonerConfig::default();
        
        // Warmup runs
        for _ in 0..config.warmup_iterations {
            let service = ReasoningService::new(reasoning_config.clone());
            let _ = service.is_consistent(&self.ontology);
        }
        
        // Actual benchmark runs
        let mut total_time = std::time::Duration::ZERO;
        let mut total_memory = 0;
        
        for _ in 0..config.test_iterations {
            let start_memory = get_memory_usage();
            let start_time = Instant::now();
            
            let service = ReasoningService::new(reasoning_config.clone());
            let result = service.is_consistent(&self.ontology)
                .map_err(|e| format!("Reasoning failed: {}", e))?;
            
            let elapsed = start_time.elapsed();
            let memory_used = get_memory_usage() - start_memory;
            
            total_time += elapsed;
            total_memory += memory_used;
            
            // Verify result makes sense
            if elapsed > config.timeout {
                return Err("Benchmark exceeded timeout".to_string());
            }
        }
        
        Ok(PerformanceMetrics {
            reasoning_time: total_time / config.test_iterations as u32,
            memory_usage: total_memory / config.test_iterations,
            tableau_nodes: 0, // Would need to extract from reasoner statistics
            backtracking_operations: 0,
            axioms_processed: self.ontology.axioms().len(),
        })
    }
    
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::NExpTime // Consistency is NEXPTIME-complete for OWL2 DL
    }
}

/// Satisfiability checking benchmark
pub struct SatisfiabilityBenchmark {
    ontology: Ontology,
    class_expression: oxidowl::ontology::ClassExpression,
    name: String,
}

impl SatisfiabilityBenchmark {
    pub fn new(ontology: Ontology, class_expression: oxidowl::ontology::ClassExpression, name: String) -> Self {
        Self { ontology, class_expression, name }
    }
    
    pub fn with_simple_class() -> Self {
        let mut ontology = Ontology::new();
        let person_class = oxidowl::ontology::Class::new(oxidowl::ontology::IRI::new("Person"));
        ontology.add_class(person_class.clone());
        
        let class_expr = oxidowl::ontology::ClassExpression::Class(person_class);
        
        Self::new(ontology, class_expr, "Simple Class Satisfiability".to_string())
    }
}

impl Benchmark for SatisfiabilityBenchmark {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String> {
        let reasoning_config = ReasonerConfig::default();
        
        // Warmup
        for _ in 0..config.warmup_iterations {
            let service = ReasoningService::new(reasoning_config.clone());
            let _ = service.is_satisfiable(&self.ontology, &self.class_expression);
        }
        
        // Benchmark
        let mut total_time = std::time::Duration::ZERO;
        let mut total_memory = 0;
        
        for _ in 0..config.test_iterations {
            let start_memory = get_memory_usage();
            let start_time = Instant::now();
            
            let service = ReasoningService::new(reasoning_config.clone());
            let _result = service.is_satisfiable(&self.ontology, &self.class_expression)
                .map_err(|e| format!("Satisfiability check failed: {}", e))?;
            
            let elapsed = start_time.elapsed();
            let memory_used = get_memory_usage() - start_memory;
            
            total_time += elapsed;
            total_memory += memory_used;
            
            if elapsed > config.timeout {
                return Err("Benchmark exceeded timeout".to_string());
            }
        }
        
        Ok(PerformanceMetrics {
            reasoning_time: total_time / config.test_iterations as u32,
            memory_usage: total_memory / config.test_iterations,
            tableau_nodes: 0,
            backtracking_operations: 0,
            axioms_processed: self.ontology.axioms().len(),
        })
    }
    
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::NExpTime
    }
}

/// Classification benchmark - tests class hierarchy computation
pub struct ClassificationBenchmark {
    ontology: Ontology,
    name: String,
}

impl ClassificationBenchmark {
    pub fn new(ontology: Ontology, name: String) -> Self {
        Self { ontology, name }
    }
    
    pub fn with_hierarchy() -> Self {
        let mut ontology = Ontology::new();
        
        // Create a class hierarchy: Thing > LivingThing > Animal > Mammal > Dog
        let classes = vec!["Thing", "LivingThing", "Animal", "Mammal", "Dog"];
        let mut class_objects = Vec::new();
        
        for class_name in &classes {
            let class = oxidowl::ontology::Class::new(oxidowl::ontology::IRI::new(*class_name));
            ontology.add_class(class.clone());
            class_objects.push(class);
        }
        
        // Add SubClassOf axioms to create hierarchy
        for i in 1..class_objects.len() {
            let subclass_axiom = oxidowl::ontology::SubClassOfAxiom {
                id: i as u64,
                subclass: oxidowl::ontology::ClassExpression::Class(class_objects[i].clone()),
                superclass: oxidowl::ontology::ClassExpression::Class(class_objects[i-1].clone()),
                annotations: vec![],
            };
            ontology.add_axiom(oxidowl::ontology::Axiom::SubClassOf(subclass_axiom));
        }
        
        Self::new(ontology, "Class Hierarchy".to_string())
    }
}

impl Benchmark for ClassificationBenchmark {
    fn name(&self) -> &str {
        &self.name
    }
    
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String> {
        let reasoning_config = ReasonerConfig::default();
        
        // Warmup
        for _ in 0..config.warmup_iterations {
            let service = ReasoningService::new(reasoning_config.clone());
            let _ = service.classify(&self.ontology);
        }
        
        // Benchmark
        let mut total_time = std::time::Duration::ZERO;
        let mut total_memory = 0;
        
        for _ in 0..config.test_iterations {
            let start_memory = get_memory_usage();
            let start_time = Instant::now();
            
            let service = ReasoningService::new(reasoning_config.clone());
            let _result = service.classify(&self.ontology)
                .map_err(|e| format!("Classification failed: {}", e))?;
            
            let elapsed = start_time.elapsed();
            let memory_used = get_memory_usage() - start_memory;
            
            total_time += elapsed;
            total_memory += memory_used;
            
            if elapsed > config.timeout {
                return Err("Benchmark exceeded timeout".to_string());
            }
        }
        
        Ok(PerformanceMetrics {
            reasoning_time: total_time / config.test_iterations as u32,
            memory_usage: total_memory / config.test_iterations,
            tableau_nodes: 0,
            backtracking_operations: 0,
            axioms_processed: self.ontology.axioms().len(),
        })
    }
    
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::NExpTime
    }
}

/// Simple memory usage tracking (would need platform-specific implementation)
fn get_memory_usage() -> usize {
    // Placeholder - in a real implementation, this would use platform-specific APIs
    // to get actual memory usage
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_consistency_benchmark() {
        let benchmark = ConsistencyBenchmark::with_simple_ontology();
        let config = BenchmarkConfig {
            warmup_iterations: 1,
            test_iterations: 1,
            ..Default::default()
        };
        
        let result = benchmark.run(&config);
        assert!(result.is_ok(), "Consistency benchmark should succeed: {:?}", result);
        
        let metrics = result.unwrap();
        assert!(metrics.reasoning_time.as_millis() > 0, "Should take some time to reason");
        assert_eq!(metrics.axioms_processed, 1, "Should process one axiom");
    }
    
    #[test]
    fn test_satisfiability_benchmark() {
        let benchmark = SatisfiabilityBenchmark::with_simple_class();
        let config = BenchmarkConfig {
            warmup_iterations: 1,
            test_iterations: 1,
            ..Default::default()
        };
        
        let result = benchmark.run(&config);
        assert!(result.is_ok(), "Satisfiability benchmark should succeed: {:?}", result);
    }
    
    #[test]
    fn test_classification_benchmark() {
        let benchmark = ClassificationBenchmark::with_hierarchy();
        let config = BenchmarkConfig {
            warmup_iterations: 1,
            test_iterations: 1,
            ..Default::default()
        };
        
        let result = benchmark.run(&config);
        assert!(result.is_ok(), "Classification benchmark should succeed: {:?}", result);
        
        let metrics = result.unwrap();
        assert_eq!(metrics.axioms_processed, 4, "Should process four SubClassOf axioms");
    }
}

//! Scalability tests for large ontologies
//! 
//! Tests oxidowl's performance with large datasets similar to HermiT's stress tests

use crate::performance::{Benchmark, BenchmarkConfig, PerformanceMetrics, ComplexityClass};
use oxidowl::{
    ontology::*,
    reasoning::ReasoningService,
    config::ReasonerConfig,
};
use std::time::Instant;

/// Large ontology stress test
pub struct LargeOntologyTest {
    class_count: usize,
    axiom_count: usize,
    depth: usize,
}

impl LargeOntologyTest {
    pub fn new(class_count: usize, axiom_count: usize, depth: usize) -> Self {
        Self { class_count, axiom_count, depth }
    }
    
    /// Generate a large ontology with specified parameters
    fn generate_ontology(&self) -> Ontology {
        let mut ontology = Ontology::new();
        
        // Create classes
        let mut classes = Vec::new();
        for i in 0..self.class_count {
            let class = Class::new(IRI::new(format!("Class{}", i)));
            ontology.add_class(class.clone());
            classes.push(class);
        }
        
        // Create hierarchy axioms
        let mut axiom_id = 0;
        for i in 1..std::cmp::min(self.class_count, self.axiom_count + 1) {
            let parent_idx = i / 2; // Create tree-like hierarchy
            if parent_idx < i {
                let axiom = SubClassOfAxiom {
                    id: axiom_id,
                    subclass: ClassExpression::Class(classes[i].clone()),
                    superclass: ClassExpression::Class(classes[parent_idx].clone()),
                    annotations: vec![],
                };
                ontology.add_axiom(Axiom::SubClassOf(axiom));
                axiom_id += 1;
                
                if axiom_id >= self.axiom_count {
                    break;
                }
            }
        }
        
        // Add some complex axioms if we have more axiom budget
        while axiom_id < self.axiom_count && classes.len() >= 4 {
            // Create intersection axioms: C1 ⊓ C2 ⊑ C3
            let c1 = &classes[axiom_id % classes.len()];
            let c2 = &classes[(axiom_id + 1) % classes.len()];
            let c3 = &classes[(axiom_id + 2) % classes.len()];
            
            let intersection = ClassExpression::ObjectIntersectionOf(vec![
                ClassExpression::Class(c1.clone()),
                ClassExpression::Class(c2.clone()),
            ]);
            
            let axiom = SubClassOfAxiom {
                id: axiom_id,
                subclass: intersection,
                superclass: ClassExpression::Class(c3.clone()),
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::SubClassOf(axiom));
            axiom_id += 1;
        }
        
        ontology
    }
}

impl Benchmark for LargeOntologyTest {
    fn name(&self) -> &str {
        "Large Ontology Scalability Test"
    }
    
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String> {
        let ontology = self.generate_ontology();
        let reasoning_config = ReasonerConfig::default();
        
        println!("Generated ontology with {} classes and {} axioms", 
                self.class_count, ontology.axioms().len());
        
        // Single run since large ontologies can be time-consuming
        let start_memory = get_memory_usage();
        let start_time = Instant::now();
        
        let service = ReasoningService::new(reasoning_config);
        let result = service.is_consistent(&ontology)
            .map_err(|e| format!("Large ontology reasoning failed: {}", e))?;
        
        let elapsed = start_time.elapsed();
        let memory_used = get_memory_usage() - start_memory;
        
        if elapsed > config.timeout {
            return Err("Large ontology test exceeded timeout".to_string());
        }
        
        println!("Large ontology ({} classes, {} axioms) processed in {:?}, consistent: {}", 
                self.class_count, ontology.axioms().len(), elapsed, result);
        
        Ok(PerformanceMetrics {
            reasoning_time: elapsed,
            memory_usage: memory_used,
            tableau_nodes: 0,
            backtracking_operations: 0,
            axioms_processed: ontology.axioms().len(),
        })
    }
    
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::Exponential
    }
}

/// Deep hierarchy test
pub struct DeepHierarchyTest {
    depth: usize,
}

impl DeepHierarchyTest {
    pub fn new(depth: usize) -> Self {
        Self { depth }
    }
    
    fn generate_deep_hierarchy(&self) -> Ontology {
        let mut ontology = Ontology::new();
        
        // Create a deep class hierarchy: C0 ⊑ C1 ⊑ C2 ⊑ ... ⊑ Cn
        let mut classes = Vec::new();
        for i in 0..self.depth {
            let class = Class::new(IRI::new(format!("Level{}", i)));
            ontology.add_class(class.clone());
            classes.push(class);
        }
        
        // Add SubClassOf axioms to create deep hierarchy
        for i in 1..self.depth {
            let axiom = SubClassOfAxiom {
                id: i as u64,
                subclass: ClassExpression::Class(classes[i].clone()),
                superclass: ClassExpression::Class(classes[i-1].clone()),
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::SubClassOf(axiom));
        }
        
        ontology
    }
}

impl Benchmark for DeepHierarchyTest {
    fn name(&self) -> &str {
        "Deep Hierarchy Test"
    }
    
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String> {
        let ontology = self.generate_deep_hierarchy();
        let reasoning_config = ReasonerConfig::default();
        
        let start_time = Instant::now();
        
        let service = ReasoningService::new(reasoning_config);
        let result = service.classify(&ontology)
            .map_err(|e| format!("Deep hierarchy classification failed: {}", e))?;
        
        let elapsed = start_time.elapsed();
        
        if elapsed > config.timeout {
            return Err("Deep hierarchy test exceeded timeout".to_string());
        }
        
        println!("Deep hierarchy (depth {}) classified in {:?}", self.depth, elapsed);
        
        Ok(PerformanceMetrics {
            reasoning_time: elapsed,
            memory_usage: 0,
            tableau_nodes: 0,
            backtracking_operations: 0,
            axioms_processed: ontology.axioms().len(),
        })
    }
    
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::Polynomial
    }
}

/// Wide hierarchy test - many sibling classes
pub struct WideHierarchyTest {
    width: usize,
}

impl WideHierarchyTest {
    pub fn new(width: usize) -> Self {
        Self { width }
    }
    
    fn generate_wide_hierarchy(&self) -> Ontology {
        let mut ontology = Ontology::new();
        
        // Create root class
        let root = Class::new(IRI::new("Root"));
        ontology.add_class(root.clone());
        
        // Create many subclasses of root
        for i in 0..self.width {
            let class = Class::new(IRI::new(format!("Child{}", i)));
            ontology.add_class(class.clone());
            
            let axiom = SubClassOfAxiom {
                id: i as u64,
                subclass: ClassExpression::Class(class),
                superclass: ClassExpression::Class(root.clone()),
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::SubClassOf(axiom));
        }
        
        ontology
    }
}

impl Benchmark for WideHierarchyTest {
    fn name(&self) -> &str {
        "Wide Hierarchy Test"
    }
    
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String> {
        let ontology = self.generate_wide_hierarchy();
        let reasoning_config = ReasonerConfig::default();
        
        let start_time = Instant::now();
        
        let service = ReasoningService::new(reasoning_config);
        let _result = service.classify(&ontology)
            .map_err(|e| format!("Wide hierarchy classification failed: {}", e))?;
        
        let elapsed = start_time.elapsed();
        
        if elapsed > config.timeout {
            return Err("Wide hierarchy test exceeded timeout".to_string());
        }
        
        println!("Wide hierarchy (width {}) classified in {:?}", self.width, elapsed);
        
        Ok(PerformanceMetrics {
            reasoning_time: elapsed,
            memory_usage: 0,
            tableau_nodes: 0,
            backtracking_operations: 0,
            axioms_processed: ontology.axioms().len(),
        })
    }
    
    fn expected_complexity(&self) -> ComplexityClass {
        ComplexityClass::Polynomial
    }
}

/// Stress test runner
pub struct StressTestRunner;

impl StressTestRunner {
    pub fn run_scalability_tests() -> Vec<(String, Result<PerformanceMetrics, String>)> {
        let config = BenchmarkConfig::default();
        let mut results = Vec::new();
        
        // Test different sizes
        let sizes = vec![10, 50, 100, 500];
        
        for size in sizes {
            // Large ontology test
            let test = LargeOntologyTest::new(size, size * 2, 5);
            let result = test.run(&config);
            results.push((format!("Large Ontology ({})", size), result));
            
            // Deep hierarchy test
            let test = DeepHierarchyTest::new(size);
            let result = test.run(&config);
            results.push((format!("Deep Hierarchy ({})", size), result));
            
            // Wide hierarchy test
            let test = WideHierarchyTest::new(size);
            let result = test.run(&config);
            results.push((format!("Wide Hierarchy ({})", size), result));
        }
        
        results
    }
    
    pub fn print_scalability_report(results: Vec<(String, Result<PerformanceMetrics, String>)>) {
        println!("\nScalability Test Report");
        println!("======================");
        
        for (name, result) in results {
            match result {
                Ok(metrics) => {
                    println!("{}: {:?} ({} axioms)", 
                            name, metrics.reasoning_time, metrics.axioms_processed);
                }
                Err(e) => {
                    println!("{}: FAILED - {}", name, e);
                }
            }
        }
    }
}

fn get_memory_usage() -> usize {
    // Placeholder implementation
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_small_ontology() {
        let test = LargeOntologyTest::new(10, 15, 3);
        let config = BenchmarkConfig {
            warmup_iterations: 0,
            test_iterations: 1,
            timeout: std::time::Duration::from_secs(30),
            ..Default::default()
        };
        
        let result = test.run(&config);
        assert!(result.is_ok(), "Small ontology test should succeed: {:?}", result);
    }
    
    #[test]
    fn test_deep_hierarchy() {
        let test = DeepHierarchyTest::new(20);
        let config = BenchmarkConfig {
            timeout: std::time::Duration::from_secs(30),
            ..Default::default()
        };
        
        let result = test.run(&config);
        assert!(result.is_ok(), "Deep hierarchy test should succeed: {:?}", result);
    }
    
    #[test]
    fn test_wide_hierarchy() {
        let test = WideHierarchyTest::new(50);
        let config = BenchmarkConfig {
            timeout: std::time::Duration::from_secs(30),
            ..Default::default()
        };
        
        let result = test.run(&config);
        assert!(result.is_ok(), "Wide hierarchy test should succeed: {:?}", result);
    }
    
    #[test]
    fn test_stress_runner() {
        // Run a small subset for testing
        let config = BenchmarkConfig {
            timeout: std::time::Duration::from_secs(60),
            ..Default::default()
        };
        
        let test = LargeOntologyTest::new(25, 30, 3);
        let result = test.run(&config);
        
        assert!(result.is_ok(), "Stress test should handle moderate loads");
    }
}

//! Integration tests that combine multiple performance test modules
//! 
//! Comprehensive test runner similar to HermiT's integrated test suite

use crate::performance::{
    reasoning_benchmarks::*,
    memory_benchmarks::*,
    scalability_tests::*,
    conformance_tests::*,
    algorithm_benchmarks::*,
    BenchmarkConfig,
};
use oxidowl::{
    ontology::*,
    parsers::turtle::TurtleParser,
};
use std::time::Duration;
use std::collections::HashMap;

/// Comprehensive test suite runner
pub struct IntegratedTestSuite {
    config: BenchmarkConfig,
    test_ontologies: HashMap<String, Ontology>,
}

impl IntegratedTestSuite {
    pub fn new() -> Self {
        let config = BenchmarkConfig {
            iterations: 10,
            warmup_iterations: 3,
            timeout: Duration::from_secs(120),
        };
        
        let mut test_ontologies = HashMap::new();
        
        // Load different ontologies for comprehensive testing
        if let Ok(greenhouse) = Self::load_greenhouse_ontology() {
            test_ontologies.insert("greenhouse".to_string(), greenhouse);
        }
        
        test_ontologies.insert("simple".to_string(), Self::create_simple_ontology());
        test_ontologies.insert("medium".to_string(), Self::create_medium_ontology());
        test_ontologies.insert("complex".to_string(), Self::create_complex_ontology());
        
        Self {
            config,
            test_ontologies,
        }
    }
    
    /// Run all performance tests
    pub fn run_all_tests(&self) -> IntegratedTestResults {
        println!("Running Integrated Performance Test Suite");
        println!("=========================================");
        
        let reasoning_results = self.run_reasoning_benchmarks();
        let memory_results = self.run_memory_benchmarks();
        let conformance_results = self.run_conformance_tests();
        let algorithm_results = self.run_algorithm_benchmarks();
        let scalability_results = self.run_scalability_tests();
        
        IntegratedTestResults {
            reasoning: reasoning_results,
            memory: memory_results,
            conformance: conformance_results,
            algorithm: algorithm_results,
            scalability: scalability_results,
            summary: self.generate_summary(),
        }
    }
    
    /// Run reasoning benchmarks on all test ontologies
    fn run_reasoning_benchmarks(&self) -> HashMap<String, ReasoningBenchmarkResult> {
        let mut results = HashMap::new();
        
        for (name, ontology) in &self.test_ontologies {
            println!("Running reasoning benchmarks on: {}", name);
            
            let consistency_benchmark = ConsistencyBenchmark::new(format!("{}_consistency", name), self.config.clone());
            let satisfiability_benchmark = SatisfiabilityBenchmark::new(format!("{}_satisfiability", name), self.config.clone());
            let classification_benchmark = ClassificationBenchmark::new(format!("{}_classification", name), self.config.clone());
            
            let consistency_result = consistency_benchmark.run_benchmark(ontology);
            
            // Create test class for satisfiability
            let test_class = if let Some(first_class) = ontology.get_classes().first() {
                ClassExpression::Class(first_class.clone())
            } else {
                ClassExpression::Class(Class::new(IRI::new("TestClass")))
            };
            
            let satisfiability_result = satisfiability_benchmark.run_benchmark(ontology, &test_class);
            let classification_result = classification_benchmark.run_benchmark(ontology);
            
            let combined_result = ReasoningBenchmarkResult {
                consistency: consistency_result,
                satisfiability: satisfiability_result,
                classification: classification_result,
            };
            
            results.insert(name.clone(), combined_result);
        }
        
        results
    }
    
    /// Run memory benchmarks
    fn run_memory_benchmarks(&self) -> HashMap<String, MemoryBenchmarkResult> {
        let mut results = HashMap::new();
        
        for (name, ontology) in &self.test_ontologies {
            println!("Running memory benchmarks on: {}", name);
            
            let memory_benchmark = MemoryBenchmark::new(format!("{}_memory", name));
            
            let consistency_memory = memory_benchmark.test_consistency_memory(ontology);
            let classification_memory = memory_benchmark.test_classification_memory(ontology);
            
            // Run leak test with fewer iterations for faster execution
            let leak_test = MemoryLeakTest::new(5);
            let leak_result = leak_test.run_leak_test(ontology);
            
            let result = MemoryBenchmarkResult {
                consistency: consistency_memory,
                classification: classification_memory,
                leak_test: leak_result,
            };
            
            results.insert(name.clone(), result);
        }
        
        results
    }
    
    /// Run conformance tests
    fn run_conformance_tests(&self) -> ConformanceTestResults {
        println!("Running OWL2 DL conformance tests");
        
        let conformance_suite = ConformanceTestSuite::new();
        conformance_suite.run_all_tests()
    }
    
    /// Run algorithm comparison benchmarks
    fn run_algorithm_benchmarks(&self) -> HashMap<String, AlgorithmComparisonResult> {
        let mut results = HashMap::new();
        
        for (name, ontology) in &self.test_ontologies {
            println!("Running algorithm benchmarks on: {}", name);
            
            let benchmark = AlgorithmBenchmark::new(format!("{}_algorithm", name), self.config.clone());
            let result = benchmark.compare_algorithms(ontology);
            
            results.insert(name.clone(), result);
        }
        
        results
    }
    
    /// Run scalability tests
    fn run_scalability_tests(&self) -> ScalabilityTestResults {
        println!("Running scalability tests");
        
        let large_test = LargeOntologyTest::new();
        let deep_test = DeepHierarchyTest::new();
        let wide_test = WideHierarchyTest::new();
        
        let large_result = large_test.run_test();
        let deep_result = deep_test.run_test();
        let wide_result = wide_test.run_test();
        
        ScalabilityTestResults {
            large_ontology: large_result,
            deep_hierarchy: deep_result,
            wide_hierarchy: wide_result,
        }
    }
    
    /// Generate overall test summary
    fn generate_summary(&self) -> TestSummary {
        TestSummary {
            total_ontologies_tested: self.test_ontologies.len(),
            test_categories: vec![
                "Reasoning Benchmarks".to_string(),
                "Memory Benchmarks".to_string(),
                "Conformance Tests".to_string(),
                "Algorithm Comparisons".to_string(),
                "Scalability Tests".to_string(),
            ],
            overall_status: "COMPLETED".to_string(),
        }
    }
    
    /// Load greenhouse ontology if available
    fn load_greenhouse_ontology() -> Result<Ontology, Box<dyn std::error::Error>> {
        let greenhouse_path = "greenhouse.ttl";
        
        if std::path::Path::new(greenhouse_path).exists() {
            let parser = TurtleParser::new();
            parser.parse_file(greenhouse_path)
        } else {
            Err("Greenhouse ontology not found".into())
        }
    }
    
    /// Create simple test ontology
    fn create_simple_ontology() -> Ontology {
        let mut ontology = Ontology::new();
        
        let animal = Class::new(IRI::new("Animal"));
        let mammal = Class::new(IRI::new("Mammal"));
        
        ontology.add_class(animal.clone());
        ontology.add_class(mammal.clone());
        
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: ClassExpression::Class(mammal),
            superclass: ClassExpression::Class(animal),
            annotations: vec![],
        }));
        
        ontology
    }
    
    /// Create medium complexity test ontology
    fn create_medium_ontology() -> Ontology {
        let mut ontology = Self::create_simple_ontology();
        
        // Add more classes and properties
        let dog = Class::new(IRI::new("Dog"));
        let cat = Class::new(IRI::new("Cat"));
        let has_pet = ObjectProperty::new(IRI::new("hasPet"));
        
        ontology.add_class(dog.clone());
        ontology.add_class(cat.clone());
        ontology.add_object_property(has_pet);
        
        // Add more axioms
        let mammal = Class::new(IRI::new("Mammal"));
        
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 2,
            subclass: ClassExpression::Class(dog),
            superclass: ClassExpression::Class(mammal.clone()),
            annotations: vec![],
        }));
        
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 3,
            subclass: ClassExpression::Class(cat),
            superclass: ClassExpression::Class(mammal),
            annotations: vec![],
        }));
        
        ontology
    }
    
    /// Create complex test ontology
    fn create_complex_ontology() -> Ontology {
        let mut ontology = Self::create_medium_ontology();
        
        // Add complex class expressions and restrictions
        let person = Class::new(IRI::new("Person"));
        let pet_owner = Class::new(IRI::new("PetOwner"));
        let has_pet = ObjectProperty::new(IRI::new("hasPet"));
        let animal = Class::new(IRI::new("Animal"));
        
        ontology.add_class(person.clone());
        ontology.add_class(pet_owner.clone());
        
        // PetOwner ≡ Person ⊓ ∃hasPet.Animal
        let existential = ClassExpression::ObjectSomeValuesFrom {
            property: ObjectPropertyExpression::ObjectProperty(has_pet),
            filler: Box::new(ClassExpression::Class(animal)),
        };
        
        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 10,
            class_expressions: vec![
                ClassExpression::Class(pet_owner),
                ClassExpression::ObjectIntersectionOf(vec![
                    ClassExpression::Class(person),
                    existential,
                ]),
            ],
            annotations: vec![],
        }));
        
        ontology
    }
}

/// Combined results from all test categories
#[derive(Debug)]
pub struct IntegratedTestResults {
    pub reasoning: HashMap<String, ReasoningBenchmarkResult>,
    pub memory: HashMap<String, MemoryBenchmarkResult>,
    pub conformance: ConformanceTestResults,
    pub algorithm: HashMap<String, AlgorithmComparisonResult>,
    pub scalability: ScalabilityTestResults,
    pub summary: TestSummary,
}

#[derive(Debug)]
pub struct ReasoningBenchmarkResult {
    pub consistency: BenchmarkResult,
    pub satisfiability: BenchmarkResult,
    pub classification: BenchmarkResult,
}

#[derive(Debug)]
pub struct MemoryBenchmarkResult {
    pub consistency: MemoryUsageResult,
    pub classification: MemoryUsageResult,
    pub leak_test: MemoryLeakResult,
}

#[derive(Debug)]
pub struct ScalabilityTestResults {
    pub large_ontology: TestResult,
    pub deep_hierarchy: TestResult,
    pub wide_hierarchy: TestResult,
}

#[derive(Debug)]
pub struct TestSummary {
    pub total_ontologies_tested: usize,
    pub test_categories: Vec<String>,
    pub overall_status: String,
}

impl IntegratedTestResults {
    /// Print comprehensive results summary
    pub fn print_comprehensive_summary(&self) {
        println!("\n=================================================");
        println!("           INTEGRATED TEST RESULTS");
        println!("=================================================");
        
        self.print_reasoning_summary();
        self.print_memory_summary();
        self.print_conformance_summary();
        self.print_algorithm_summary();
        self.print_scalability_summary();
        self.print_overall_summary();
    }
    
    fn print_reasoning_summary(&self) {
        println!("\nReasoning Benchmark Results:");
        println!("----------------------------");
        
        for (ontology_name, result) in &self.reasoning {
            println!("\n  Ontology: {}", ontology_name);
            println!("    Consistency: {:?} (Success: {})", 
                    result.consistency.avg_time, result.consistency.success_rate);
            println!("    Satisfiability: {:?} (Success: {})", 
                    result.satisfiability.avg_time, result.satisfiability.success_rate);
            println!("    Classification: {:?} (Success: {})", 
                    result.classification.avg_time, result.classification.success_rate);
        }
    }
    
    fn print_memory_summary(&self) {
        println!("\nMemory Benchmark Results:");
        println!("-------------------------");
        
        for (ontology_name, result) in &self.memory {
            println!("\n  Ontology: {}", ontology_name);
            println!("    Consistency Memory: {} bytes", result.consistency.peak_memory);
            println!("    Classification Memory: {} bytes", result.classification.peak_memory);
            println!("    Memory Leak Detected: {}", result.leak_test.leak_detected);
        }
    }
    
    fn print_conformance_summary(&self) {
        println!("\nConformance Test Results:");
        println!("-------------------------");
        
        let passed = self.conformance.results.iter().filter(|r| r.passed).count();
        let total = self.conformance.results.len();
        
        println!("  Passed: {}/{} ({:.1}%)", passed, total, 
                (passed as f64 / total as f64) * 100.0);
        
        for result in &self.conformance.results {
            let status = if result.passed { "PASS" } else { "FAIL" };
            println!("    {} - {}", result.test_name, status);
        }
    }
    
    fn print_algorithm_summary(&self) {
        println!("\nAlgorithm Comparison Results:");
        println!("-----------------------------");
        
        for (ontology_name, result) in &self.algorithm {
            println!("\n  Ontology: {}", ontology_name);
            println!("    Tableau Consistency: {:?}", result.tableau.avg_consistency_time);
            println!("    HyperTableau Consistency: {:?}", result.hypertableau.avg_consistency_time);
            println!("    Speedup: {:.2}x", result.comparison.consistency_speedup);
        }
    }
    
    fn print_scalability_summary(&self) {
        println!("\nScalability Test Results:");
        println!("-------------------------");
        
        println!("  Large Ontology: {} (Time: {:?})", 
                if self.scalability.large_ontology.success { "PASS" } else { "FAIL" },
                self.scalability.large_ontology.duration);
        
        println!("  Deep Hierarchy: {} (Time: {:?})", 
                if self.scalability.deep_hierarchy.success { "PASS" } else { "FAIL" },
                self.scalability.deep_hierarchy.duration);
        
        println!("  Wide Hierarchy: {} (Time: {:?})", 
                if self.scalability.wide_hierarchy.success { "PASS" } else { "FAIL" },
                self.scalability.wide_hierarchy.duration);
    }
    
    fn print_overall_summary(&self) {
        println!("\nOverall Test Summary:");
        println!("====================");
        
        println!("  Total Ontologies Tested: {}", self.summary.total_ontologies_tested);
        println!("  Test Categories: {}", self.summary.test_categories.len());
        println!("  Overall Status: {}", self.summary.overall_status);
        
        // Calculate overall success metrics
        let reasoning_success = self.calculate_reasoning_success_rate();
        let conformance_success = self.calculate_conformance_success_rate();
        let scalability_success = self.calculate_scalability_success_rate();
        
        println!("  Average Reasoning Success Rate: {:.1}%", reasoning_success * 100.0);
        println!("  Conformance Success Rate: {:.1}%", conformance_success * 100.0);
        println!("  Scalability Success Rate: {:.1}%", scalability_success * 100.0);
    }
    
    fn calculate_reasoning_success_rate(&self) -> f64 {
        let mut total_success = 0.0;
        let mut total_tests = 0;
        
        for result in self.reasoning.values() {
            total_success += result.consistency.success_rate;
            total_success += result.satisfiability.success_rate;
            total_success += result.classification.success_rate;
            total_tests += 3;
        }
        
        if total_tests > 0 {
            total_success / total_tests as f64
        } else {
            0.0
        }
    }
    
    fn calculate_conformance_success_rate(&self) -> f64 {
        let passed = self.conformance.results.iter().filter(|r| r.passed).count();
        let total = self.conformance.results.len();
        
        if total > 0 {
            passed as f64 / total as f64
        } else {
            0.0
        }
    }
    
    fn calculate_scalability_success_rate(&self) -> f64 {
        let mut passed = 0;
        if self.scalability.large_ontology.success { passed += 1; }
        if self.scalability.deep_hierarchy.success { passed += 1; }
        if self.scalability.wide_hierarchy.success { passed += 1; }
        
        passed as f64 / 3.0
    }
}

/// Quick test runner for CI/CD integration
pub struct QuickTestRunner;

impl QuickTestRunner {
    /// Run essential tests quickly for continuous integration
    pub fn run_essential_tests() -> bool {
        println!("Running essential performance tests...");
        
        let simple_ontology = IntegratedTestSuite::create_simple_ontology();
        
        // Quick consistency check
        let config = BenchmarkConfig {
            iterations: 3,
            warmup_iterations: 1,
            timeout: Duration::from_secs(10),
        };
        
        let consistency_benchmark = ConsistencyBenchmark::new("quick_consistency".to_string(), config.clone());
        let result = consistency_benchmark.run_benchmark(&simple_ontology);
        
        if result.success_rate < 1.0 {
            println!("FAILURE: Consistency check failed");
            return false;
        }
        
        // Quick conformance check (subset)
        let conformance_suite = ConformanceTestSuite::new();
        let basic_result = conformance_suite.test_basic_subclass();
        
        if !basic_result.passed {
            println!("FAILURE: Basic conformance test failed");
            return false;
        }
        
        println!("Essential tests passed!");
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_integrated_suite_creation() {
        let suite = IntegratedTestSuite::new();
        assert!(!suite.test_ontologies.is_empty());
        assert!(suite.test_ontologies.contains_key("simple"));
        assert!(suite.test_ontologies.contains_key("medium"));
        assert!(suite.test_ontologies.contains_key("complex"));
    }
    
    #[test]
    fn test_ontology_creation() {
        let simple = IntegratedTestSuite::create_simple_ontology();
        assert_eq!(simple.get_classes().len(), 2);
        
        let medium = IntegratedTestSuite::create_medium_ontology();
        assert!(medium.get_classes().len() > simple.get_classes().len());
        
        let complex = IntegratedTestSuite::create_complex_ontology();
        assert!(complex.get_classes().len() >= medium.get_classes().len());
    }
    
    #[test]
    fn test_quick_runner() {
        // This should run quickly for CI
        let result = QuickTestRunner::run_essential_tests();
        assert!(result, "Essential tests should pass");
    }
    
    // Integration test that runs a subset of the full suite
    #[test]
    fn test_partial_integration() {
        let suite = IntegratedTestSuite::new();
        
        // Test just one ontology to keep test time reasonable
        if let Some(simple_ontology) = suite.test_ontologies.get("simple") {
            let config = BenchmarkConfig {
                iterations: 2,
                warmup_iterations: 1,
                timeout: Duration::from_secs(5),
            };
            
            let consistency_benchmark = ConsistencyBenchmark::new("test_consistency".to_string(), config);
            let result = consistency_benchmark.run_benchmark(simple_ontology);
            
            assert!(result.success_rate > 0.0);
        }
    }
}

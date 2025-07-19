//! Algorithm-specific benchmarks comparing tableau vs hypertableau
//! 
//! Compares performance characteristics of different reasoning algorithms
//! similar to HermiT's algorithm performance analysis

use crate::performance::{PerformanceMetrics, BenchmarkConfig};
use oxidowl::{
    ontology::*,
    reasoning::ReasoningService,
    config::{ReasonerConfig, Algorithm},
    core::tableau::TableauReasoner,
    core::hypertableau::HyperTableauReasoner,
};
use std::time::{Instant, Duration};
use std::collections::HashMap;

/// Algorithm performance comparison framework
pub struct AlgorithmBenchmark {
    name: String,
    config: BenchmarkConfig,
}

impl AlgorithmBenchmark {
    pub fn new(name: String, config: BenchmarkConfig) -> Self {
        Self { name, config }
    }
    
    /// Compare tableau vs hypertableau performance
    pub fn compare_algorithms(&self, ontology: &Ontology) -> AlgorithmComparisonResult {
        println!("Running algorithm comparison: {}", self.name);
        
        let tableau_result = self.benchmark_tableau(ontology);
        let hypertableau_result = self.benchmark_hypertableau(ontology);
        
        AlgorithmComparisonResult {
            tableau: tableau_result,
            hypertableau: hypertableau_result,
            comparison: self.analyze_comparison(&tableau_result, &hypertableau_result),
        }
    }
    
    /// Benchmark traditional tableau algorithm
    fn benchmark_tableau(&self, ontology: &Ontology) -> AlgorithmResult {
        let mut config = ReasonerConfig::default();
        config.algorithm = Algorithm::Tableau;
        config.timeout = Some(self.config.timeout);
        
        let mut metrics = PerformanceMetrics::new();
        let mut results = Vec::new();
        
        for _ in 0..self.config.iterations {
            let start_time = Instant::now();
            let service = ReasoningService::new(config.clone());
            
            let consistency_result = service.is_consistent(ontology);
            let consistency_time = start_time.elapsed();
            
            let classification_start = Instant::now();
            let classification_result = service.classify(ontology);
            let classification_time = classification_start.elapsed();
            
            results.push(AlgorithmIteration {
                consistency_time,
                classification_time,
                consistency_success: consistency_result.is_ok(),
                classification_success: classification_result.is_ok(),
            });
            
            metrics.record_sample(consistency_time.as_nanos() as f64);
        }
        
        AlgorithmResult {
            algorithm: "Tableau".to_string(),
            iterations: results,
            avg_consistency_time: self.calculate_avg_time(&results, |r| r.consistency_time),
            avg_classification_time: self.calculate_avg_time(&results, |r| r.classification_time),
            success_rate: self.calculate_success_rate(&results),
            performance_metrics: metrics,
        }
    }
    
    /// Benchmark hypertableau algorithm
    fn benchmark_hypertableau(&self, ontology: &Ontology) -> AlgorithmResult {
        let mut config = ReasonerConfig::default();
        config.algorithm = Algorithm::HyperTableau;
        config.timeout = Some(self.config.timeout);
        
        let mut metrics = PerformanceMetrics::new();
        let mut results = Vec::new();
        
        for _ in 0..self.config.iterations {
            let start_time = Instant::now();
            let service = ReasoningService::new(config.clone());
            
            let consistency_result = service.is_consistent(ontology);
            let consistency_time = start_time.elapsed();
            
            let classification_start = Instant::now();
            let classification_result = service.classify(ontology);
            let classification_time = classification_start.elapsed();
            
            results.push(AlgorithmIteration {
                consistency_time,
                classification_time,
                consistency_success: consistency_result.is_ok(),
                classification_success: classification_result.is_ok(),
            });
            
            metrics.record_sample(consistency_time.as_nanos() as f64);
        }
        
        AlgorithmResult {
            algorithm: "HyperTableau".to_string(),
            iterations: results,
            avg_consistency_time: self.calculate_avg_time(&results, |r| r.consistency_time),
            avg_classification_time: self.calculate_avg_time(&results, |r| r.classification_time),
            success_rate: self.calculate_success_rate(&results),
            performance_metrics: metrics,
        }
    }
    
    fn calculate_avg_time<F>(&self, results: &[AlgorithmIteration], time_extractor: F) -> Duration
    where
        F: Fn(&AlgorithmIteration) -> Duration,
    {
        let total_nanos: u64 = results.iter()
            .map(|r| time_extractor(r).as_nanos() as u64)
            .sum();
        Duration::from_nanos(total_nanos / results.len() as u64)
    }
    
    fn calculate_success_rate(&self, results: &[AlgorithmIteration]) -> f64 {
        let successful = results.iter()
            .filter(|r| r.consistency_success && r.classification_success)
            .count();
        successful as f64 / results.len() as f64
    }
    
    fn analyze_comparison(&self, tableau: &AlgorithmResult, hypertableau: &AlgorithmResult) -> ComparisonAnalysis {
        let consistency_speedup = tableau.avg_consistency_time.as_nanos() as f64 / 
                                 hypertableau.avg_consistency_time.as_nanos() as f64;
        
        let classification_speedup = tableau.avg_classification_time.as_nanos() as f64 /
                                    hypertableau.avg_classification_time.as_nanos() as f64;
        
        ComparisonAnalysis {
            consistency_speedup,
            classification_speedup,
            tableau_faster_consistency: consistency_speedup < 1.0,
            tableau_faster_classification: classification_speedup < 1.0,
            recommendation: self.generate_recommendation(consistency_speedup, classification_speedup),
        }
    }
    
    fn generate_recommendation(&self, consistency_speedup: f64, classification_speedup: f64) -> String {
        match (consistency_speedup < 1.0, classification_speedup < 1.0) {
            (true, true) => "Tableau algorithm is faster for both operations".to_string(),
            (false, false) => "HyperTableau algorithm is faster for both operations".to_string(),
            (true, false) => "Tableau faster for consistency, HyperTableau faster for classification".to_string(),
            (false, true) => "HyperTableau faster for consistency, Tableau faster for classification".to_string(),
        }
    }
}

/// Result of a single algorithm iteration
#[derive(Debug, Clone)]
pub struct AlgorithmIteration {
    pub consistency_time: Duration,
    pub classification_time: Duration,
    pub consistency_success: bool,
    pub classification_success: bool,
}

/// Complete result for one algorithm
#[derive(Debug)]
pub struct AlgorithmResult {
    pub algorithm: String,
    pub iterations: Vec<AlgorithmIteration>,
    pub avg_consistency_time: Duration,
    pub avg_classification_time: Duration,
    pub success_rate: f64,
    pub performance_metrics: PerformanceMetrics,
}

/// Comparison analysis between algorithms
#[derive(Debug)]
pub struct ComparisonAnalysis {
    pub consistency_speedup: f64,
    pub classification_speedup: f64,
    pub tableau_faster_consistency: bool,
    pub tableau_faster_classification: bool,
    pub recommendation: String,
}

/// Complete algorithm comparison result
#[derive(Debug)]
pub struct AlgorithmComparisonResult {
    pub tableau: AlgorithmResult,
    pub hypertableau: AlgorithmResult,
    pub comparison: ComparisonAnalysis,
}

impl AlgorithmComparisonResult {
    pub fn print_summary(&self) {
        println!("\nAlgorithm Comparison Summary");
        println!("============================");
        
        self.print_algorithm_summary(&self.tableau);
        self.print_algorithm_summary(&self.hypertableau);
        
        println!("\nComparison Analysis:");
        println!("  Consistency Speedup: {:.2}x", self.comparison.consistency_speedup);
        println!("  Classification Speedup: {:.2}x", self.comparison.classification_speedup);
        println!("  Recommendation: {}", self.comparison.recommendation);
    }
    
    fn print_algorithm_summary(&self, result: &AlgorithmResult) {
        println!("\n{} Algorithm:", result.algorithm);
        println!("  Average Consistency Time: {:?}", result.avg_consistency_time);
        println!("  Average Classification Time: {:?}", result.avg_classification_time);
        println!("  Success Rate: {:.2}%", result.success_rate * 100.0);
        println!("  Mean Performance: {:.2}ms", result.performance_metrics.mean() / 1_000_000.0);
        println!("  Std Dev: {:.2}ms", result.performance_metrics.std_dev() / 1_000_000.0);
    }
}

/// Complexity-based algorithm benchmark
pub struct ComplexityBenchmark;

impl ComplexityBenchmark {
    /// Test algorithms with ontologies of varying complexity
    pub fn run_complexity_analysis() -> Vec<AlgorithmComparisonResult> {
        let mut results = Vec::new();
        let complexity_levels = vec![
            ("Simple", 10, 5),      // 10 classes, 5 properties
            ("Medium", 50, 15),     // 50 classes, 15 properties
            ("Complex", 100, 30),   // 100 classes, 30 properties
            ("Very Complex", 200, 50), // 200 classes, 50 properties
        ];
        
        for (name, num_classes, num_properties) in complexity_levels {
            println!("Testing complexity level: {}", name);
            
            let ontology = create_complex_ontology(num_classes, num_properties);
            let config = BenchmarkConfig {
                iterations: 5,
                warmup_iterations: 2,
                timeout: Duration::from_secs(60),
            };
            
            let benchmark = AlgorithmBenchmark::new(name.to_string(), config);
            let result = benchmark.compare_algorithms(&ontology);
            results.push(result);
        }
        
        results
    }
    
    /// Analyze how algorithms scale with complexity
    pub fn analyze_scaling(results: &[AlgorithmComparisonResult]) {
        println!("\nComplexity Scaling Analysis");
        println!("===========================");
        
        for (i, result) in results.iter().enumerate() {
            let level = match i {
                0 => "Simple",
                1 => "Medium", 
                2 => "Complex",
                3 => "Very Complex",
                _ => "Unknown",
            };
            
            println!("\n{} Ontology:", level);
            println!("  Tableau Consistency: {:?}", result.tableau.avg_consistency_time);
            println!("  HyperTableau Consistency: {:?}", result.hypertableau.avg_consistency_time);
            println!("  Speedup: {:.2}x", result.comparison.consistency_speedup);
        }
    }
}

/// Feature-specific algorithm benchmark
pub struct FeatureBenchmark;

impl FeatureBenchmark {
    /// Test algorithms with specific OWL features
    pub fn benchmark_owl_features() -> HashMap<String, AlgorithmComparisonResult> {
        let mut results = HashMap::new();
        
        // Test with different OWL feature sets
        let features = vec![
            ("Basic Classes", Self::create_basic_class_ontology()),
            ("Object Properties", Self::create_object_property_ontology()),
            ("Cardinality Restrictions", Self::create_cardinality_ontology()),
            ("Complex Class Expressions", Self::create_complex_expression_ontology()),
            ("Nominals", Self::create_nominal_ontology()),
        ];
        
        for (feature_name, ontology) in features {
            println!("Benchmarking feature: {}", feature_name);
            
            let config = BenchmarkConfig {
                iterations: 10,
                warmup_iterations: 3,
                timeout: Duration::from_secs(30),
            };
            
            let benchmark = AlgorithmBenchmark::new(feature_name.to_string(), config);
            let result = benchmark.compare_algorithms(&ontology);
            results.insert(feature_name.to_string(), result);
        }
        
        results
    }
    
    fn create_basic_class_ontology() -> Ontology {
        let mut ontology = Ontology::new();
        
        // Create simple class hierarchy
        let animal = Class::new(IRI::new("Animal"));
        let mammal = Class::new(IRI::new("Mammal"));
        let dog = Class::new(IRI::new("Dog"));
        
        ontology.add_class(animal.clone());
        ontology.add_class(mammal.clone());
        ontology.add_class(dog.clone());
        
        // Add hierarchy
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 1,
            subclass: ClassExpression::Class(mammal.clone()),
            superclass: ClassExpression::Class(animal),
            annotations: vec![],
        }));
        
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: 2,
            subclass: ClassExpression::Class(dog),
            superclass: ClassExpression::Class(mammal),
            annotations: vec![],
        }));
        
        ontology
    }
    
    fn create_object_property_ontology() -> Ontology {
        let mut ontology = Self::create_basic_class_ontology();
        
        // Add object properties
        let has_parent = ObjectProperty::new(IRI::new("hasParent"));
        let has_child = ObjectProperty::new(IRI::new("hasChild"));
        
        ontology.add_object_property(has_parent.clone());
        ontology.add_object_property(has_child.clone());
        
        // Add inverse property axiom
        ontology.add_axiom(Axiom::InverseObjectProperties(InverseObjectPropertiesAxiom {
            id: 10,
            property1: ObjectPropertyExpression::ObjectProperty(has_parent),
            property2: ObjectPropertyExpression::ObjectProperty(has_child),
            annotations: vec![],
        }));
        
        ontology
    }
    
    fn create_cardinality_ontology() -> Ontology {
        let mut ontology = Self::create_object_property_ontology();
        
        // Add cardinality restrictions
        let person = Class::new(IRI::new("Person"));
        let parent = Class::new(IRI::new("Parent"));
        let has_child = ObjectProperty::new(IRI::new("hasChild"));
        
        ontology.add_class(person.clone());
        ontology.add_class(parent.clone());
        
        // Parent is a person with at least one child
        let min_cardinality = ClassExpression::ObjectMinCardinality {
            cardinality: 1,
            property: ObjectPropertyExpression::ObjectProperty(has_child),
            filler: Some(Box::new(ClassExpression::Class(person.clone()))),
        };
        
        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 20,
            class_expressions: vec![
                ClassExpression::Class(parent),
                ClassExpression::ObjectIntersectionOf(vec![
                    ClassExpression::Class(person),
                    min_cardinality,
                ]),
            ],
            annotations: vec![],
        }));
        
        ontology
    }
    
    fn create_complex_expression_ontology() -> Ontology {
        let mut ontology = Self::create_cardinality_ontology();
        
        // Add complex class expressions
        let happy_person = Class::new(IRI::new("HappyPerson"));
        let person = Class::new(IRI::new("Person"));
        let dog = Class::new(IRI::new("Dog"));
        let has_pet = ObjectProperty::new(IRI::new("hasPet"));
        
        ontology.add_class(happy_person.clone());
        ontology.add_object_property(has_pet.clone());
        
        // HappyPerson ≡ Person ⊓ ∃hasPet.Dog
        let existential = ClassExpression::ObjectSomeValuesFrom {
            property: ObjectPropertyExpression::ObjectProperty(has_pet),
            filler: Box::new(ClassExpression::Class(dog)),
        };
        
        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 30,
            class_expressions: vec![
                ClassExpression::Class(happy_person),
                ClassExpression::ObjectIntersectionOf(vec![
                    ClassExpression::Class(person),
                    existential,
                ]),
            ],
            annotations: vec![],
        }));
        
        ontology
    }
    
    fn create_nominal_ontology() -> Ontology {
        let mut ontology = Self::create_complex_expression_ontology();
        
        // Add nominals (OneOf expressions)
        let color = Class::new(IRI::new("Color"));
        let red = Individual::new(IRI::new("red"));
        let green = Individual::new(IRI::new("green"));
        let blue = Individual::new(IRI::new("blue"));
        
        ontology.add_class(color.clone());
        ontology.add_individual(red.clone());
        ontology.add_individual(green.clone());
        ontology.add_individual(blue.clone());
        
        // Color ≡ {red, green, blue}
        let nominal = ClassExpression::ObjectOneOf(vec![red, green, blue]);
        
        ontology.add_axiom(Axiom::EquivalentClasses(EquivalentClassesAxiom {
            id: 40,
            class_expressions: vec![
                ClassExpression::Class(color),
                nominal,
            ],
            annotations: vec![],
        }));
        
        ontology
    }
}

/// Create ontology with specified complexity
fn create_complex_ontology(num_classes: usize, num_properties: usize) -> Ontology {
    let mut ontology = Ontology::new();
    
    // Create classes
    let mut classes = Vec::new();
    for i in 0..num_classes {
        let class = Class::new(IRI::new(format!("Class{}", i)));
        ontology.add_class(class.clone());
        classes.push(class);
    }
    
    // Create properties
    let mut properties = Vec::new();
    for i in 0..num_properties {
        let prop = ObjectProperty::new(IRI::new(format!("property{}", i)));
        ontology.add_object_property(prop.clone());
        properties.push(prop);
    }
    
    // Add class hierarchy
    for i in 1..num_classes {
        let parent_idx = i / 2;
        if parent_idx < i {
            ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
                id: i as u64,
                subclass: ClassExpression::Class(classes[i].clone()),
                superclass: ClassExpression::Class(classes[parent_idx].clone()),
                annotations: vec![],
            }));
        }
    }
    
    // Add some property restrictions
    for i in 0..std::cmp::min(num_classes / 2, num_properties) {
        let restriction = ClassExpression::ObjectSomeValuesFrom {
            property: ObjectPropertyExpression::ObjectProperty(properties[i].clone()),
            filler: Box::new(ClassExpression::Class(classes[i * 2 + 1].clone())),
        };
        
        ontology.add_axiom(Axiom::SubClassOf(SubClassOfAxiom {
            id: (num_classes + i) as u64,
            subclass: ClassExpression::Class(classes[i].clone()),
            superclass: restriction,
            annotations: vec![],
        }));
    }
    
    ontology
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_algorithm_benchmark() {
        let ontology = create_complex_ontology(10, 3);
        let config = BenchmarkConfig {
            iterations: 3,
            warmup_iterations: 1,
            timeout: Duration::from_secs(10),
        };
        
        let benchmark = AlgorithmBenchmark::new("Test".to_string(), config);
        let result = benchmark.compare_algorithms(&ontology);
        
        assert_eq!(result.tableau.algorithm, "Tableau");
        assert_eq!(result.hypertableau.algorithm, "HyperTableau");
        assert!(result.tableau.success_rate >= 0.0);
        assert!(result.hypertableau.success_rate >= 0.0);
        
        result.print_summary();
    }
    
    #[test]
    fn test_feature_benchmarks() {
        let basic_ontology = FeatureBenchmark::create_basic_class_ontology();
        assert!(!basic_ontology.get_classes().is_empty());
        
        let property_ontology = FeatureBenchmark::create_object_property_ontology();
        assert!(!property_ontology.get_object_properties().is_empty());
        
        let cardinality_ontology = FeatureBenchmark::create_cardinality_ontology();
        assert!(!cardinality_ontology.get_axioms().is_empty());
    }
    
    #[test]
    fn test_complexity_scaling() {
        let simple_ontology = create_complex_ontology(5, 2);
        let complex_ontology = create_complex_ontology(20, 8);
        
        assert!(simple_ontology.get_classes().len() < complex_ontology.get_classes().len());
        assert!(simple_ontology.get_object_properties().len() < complex_ontology.get_object_properties().len());
    }
}

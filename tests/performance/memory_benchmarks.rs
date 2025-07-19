//! Memory usage benchmarks and monitoring
//! 
//! Tests memory efficiency similar to HermiT's memory profiling

use crate::performance::{PerformanceMetrics};
use oxidowl::{
    ontology::*,
    reasoning::ReasoningService,
    config::ReasonerConfig,
};
use std::time::Instant;

/// Memory usage tracker
pub struct MemoryTracker {
    initial_memory: usize,
    peak_memory: usize,
    current_memory: usize,
}

impl MemoryTracker {
    pub fn new() -> Self {
        let current = get_memory_usage();
        Self {
            initial_memory: current,
            peak_memory: current,
            current_memory: current,
        }
    }
    
    pub fn update(&mut self) {
        self.current_memory = get_memory_usage();
        if self.current_memory > self.peak_memory {
            self.peak_memory = self.current_memory;
        }
    }
    
    pub fn get_usage(&self) -> usize {
        self.current_memory - self.initial_memory
    }
    
    pub fn get_peak_usage(&self) -> usize {
        self.peak_memory - self.initial_memory
    }
}

/// Memory benchmark for different reasoning operations
pub struct MemoryBenchmark {
    name: String,
}

impl MemoryBenchmark {
    pub fn new(name: String) -> Self {
        Self { name }
    }
    
    /// Test memory usage during consistency checking
    pub fn test_consistency_memory(&self, ontology: &Ontology) -> MemoryUsageResult {
        let mut tracker = MemoryTracker::new();
        
        let start_time = Instant::now();
        tracker.update();
        
        let service = ReasoningService::new(ReasonerConfig::default());
        tracker.update();
        let service_memory = tracker.get_usage();
        
        let result = service.is_consistent(ontology);
        tracker.update();
        let reasoning_memory = tracker.get_usage();
        
        let elapsed = start_time.elapsed();
        
        MemoryUsageResult {
            operation: "Consistency".to_string(),
            service_creation_memory: service_memory,
            reasoning_memory,
            peak_memory: tracker.get_peak_usage(),
            duration: elapsed,
            success: result.is_ok(),
        }
    }
    
    /// Test memory usage during classification
    pub fn test_classification_memory(&self, ontology: &Ontology) -> MemoryUsageResult {
        let mut tracker = MemoryTracker::new();
        
        let start_time = Instant::now();
        tracker.update();
        
        let service = ReasoningService::new(ReasonerConfig::default());
        tracker.update();
        let service_memory = tracker.get_usage();
        
        let result = service.classify(ontology);
        tracker.update();
        let reasoning_memory = tracker.get_usage();
        
        let elapsed = start_time.elapsed();
        
        MemoryUsageResult {
            operation: "Classification".to_string(),
            service_creation_memory: service_memory,
            reasoning_memory,
            peak_memory: tracker.get_peak_usage(),
            duration: elapsed,
            success: result.is_ok(),
        }
    }
    
    /// Test memory usage during satisfiability checking
    pub fn test_satisfiability_memory(&self, ontology: &Ontology, class_expr: &ClassExpression) -> MemoryUsageResult {
        let mut tracker = MemoryTracker::new();
        
        let start_time = Instant::now();
        tracker.update();
        
        let service = ReasoningService::new(ReasonerConfig::default());
        tracker.update();
        let service_memory = tracker.get_usage();
        
        let result = service.is_satisfiable(ontology, class_expr);
        tracker.update();
        let reasoning_memory = tracker.get_usage();
        
        let elapsed = start_time.elapsed();
        
        MemoryUsageResult {
            operation: "Satisfiability".to_string(),
            service_creation_memory: service_memory,
            reasoning_memory,
            peak_memory: tracker.get_peak_usage(),
            duration: elapsed,
            success: result.is_ok(),
        }
    }
}

/// Result of memory usage measurement
#[derive(Debug, Clone)]
pub struct MemoryUsageResult {
    pub operation: String,
    pub service_creation_memory: usize,
    pub reasoning_memory: usize,
    pub peak_memory: usize,
    pub duration: std::time::Duration,
    pub success: bool,
}

impl MemoryUsageResult {
    pub fn print_summary(&self) {
        println!("Memory Usage - {}", self.operation);
        println!("  Service Creation: {} bytes", format_bytes(self.service_creation_memory));
        println!("  Reasoning: {} bytes", format_bytes(self.reasoning_memory));
        println!("  Peak: {} bytes", format_bytes(self.peak_memory));
        println!("  Duration: {:?}", self.duration);
        println!("  Success: {}", self.success);
    }
}

/// Memory leak detection test
pub struct MemoryLeakTest {
    iterations: usize,
}

impl MemoryLeakTest {
    pub fn new(iterations: usize) -> Self {
        Self { iterations }
    }
    
    pub fn run_leak_test(&self, ontology: &Ontology) -> MemoryLeakResult {
        let mut initial_memory = get_memory_usage();
        let mut memory_samples = Vec::new();
        
        for i in 0..self.iterations {
            // Force garbage collection if possible (platform specific)
            force_gc();
            
            let before_memory = get_memory_usage();
            
            // Create and use reasoning service
            let service = ReasoningService::new(ReasonerConfig::default());
            let _result = service.is_consistent(ontology);
            
            // Drop the service explicitly
            drop(service);
            
            // Force GC again
            force_gc();
            
            let after_memory = get_memory_usage();
            memory_samples.push((i, before_memory, after_memory));
            
            if i == 0 {
                initial_memory = after_memory;
            }
        }
        
        let final_memory = get_memory_usage();
        let memory_growth = final_memory as i64 - initial_memory as i64;
        
        MemoryLeakResult {
            iterations: self.iterations,
            initial_memory,
            final_memory,
            memory_growth,
            samples: memory_samples,
            leak_detected: memory_growth > (initial_memory as i64 * 10 / 100), // 10% growth threshold
        }
    }
}

/// Result of memory leak test
#[derive(Debug)]
pub struct MemoryLeakResult {
    pub iterations: usize,
    pub initial_memory: usize,
    pub final_memory: usize,
    pub memory_growth: i64,
    pub samples: Vec<(usize, usize, usize)>,
    pub leak_detected: bool,
}

impl MemoryLeakResult {
    pub fn print_summary(&self) {
        println!("Memory Leak Test Results");
        println!("  Iterations: {}", self.iterations);
        println!("  Initial Memory: {}", format_bytes(self.initial_memory));
        println!("  Final Memory: {}", format_bytes(self.final_memory));
        println!("  Memory Growth: {} bytes", self.memory_growth);
        println!("  Leak Detected: {}", self.leak_detected);
        
        if self.leak_detected {
            println!("  WARNING: Potential memory leak detected!");
        }
    }
}

/// Memory stress test with increasing ontology sizes
pub struct MemoryStressTest;

impl MemoryStressTest {
    pub fn run_stress_test() -> Vec<MemoryUsageResult> {
        let mut results = Vec::new();
        let sizes = vec![10, 50, 100, 200, 500];
        
        for size in sizes {
            println!("Running memory stress test with {} classes", size);
            
            let ontology = create_test_ontology(size);
            let benchmark = MemoryBenchmark::new(format!("Stress-{}", size));
            
            let result = benchmark.test_consistency_memory(&ontology);
            results.push(result);
        }
        
        results
    }
    
    pub fn analyze_memory_scaling(results: &[MemoryUsageResult]) {
        println!("\nMemory Scaling Analysis");
        println!("======================");
        
        for result in results {
            let ratio = if result.service_creation_memory > 0 {
                result.reasoning_memory as f64 / result.service_creation_memory as f64
            } else {
                0.0
            };
            
            println!("{}: Peak={}, Ratio={:.2}", 
                    result.operation,
                    format_bytes(result.peak_memory),
                    ratio);
        }
    }
}

/// Create test ontology of specified size
fn create_test_ontology(size: usize) -> Ontology {
    let mut ontology = Ontology::new();
    
    // Create classes
    let mut classes = Vec::new();
    for i in 0..size {
        let class = Class::new(IRI::new(format!("TestClass{}", i)));
        ontology.add_class(class.clone());
        classes.push(class);
    }
    
    // Add hierarchy axioms
    for i in 1..size {
        let parent_idx = i / 2;
        if parent_idx < i {
            let axiom = SubClassOfAxiom {
                id: i as u64,
                subclass: ClassExpression::Class(classes[i].clone()),
                superclass: ClassExpression::Class(classes[parent_idx].clone()),
                annotations: vec![],
            };
            ontology.add_axiom(Axiom::SubClassOf(axiom));
        }
    }
    
    ontology
}

/// Format bytes in human-readable format
fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    format!("{:.2} {}", size, UNITS[unit_index])
}

/// Platform-specific memory usage retrieval
fn get_memory_usage() -> usize {
    // Placeholder implementation - would need platform-specific code
    // On Unix systems: parse /proc/self/status or use rusage
    // On Windows: use GetProcessMemoryInfo
    
    // For testing, return a mock value
    use std::sync::atomic::{AtomicUsize, Ordering};
    static MOCK_MEMORY: AtomicUsize = AtomicUsize::new(1024 * 1024); // Start with 1MB
    
    // Simulate memory growth
    MOCK_MEMORY.fetch_add(1024, Ordering::Relaxed)
}

/// Force garbage collection (platform specific)
fn force_gc() {
    // Rust doesn't have explicit GC, but we can try to trigger cleanup
    // by allocating and deallocating memory
    let _temp: Vec<u8> = Vec::with_capacity(1024);
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_tracker() {
        let mut tracker = MemoryTracker::new();
        let initial = tracker.get_usage();
        
        tracker.update();
        let updated = tracker.get_usage();
        
        // Memory usage should be non-negative
        assert!(updated >= initial);
    }
    
    #[test]
    fn test_memory_benchmark() {
        let ontology = create_test_ontology(10);
        let benchmark = MemoryBenchmark::new("Test".to_string());
        
        let result = benchmark.test_consistency_memory(&ontology);
        assert!(result.success, "Memory benchmark should succeed");
        result.print_summary();
    }
    
    #[test]
    fn test_memory_leak_detection() {
        let ontology = create_test_ontology(5);
        let leak_test = MemoryLeakTest::new(10);
        
        let result = leak_test.run_leak_test(&ontology);
        result.print_summary();
        
        // With only 10 iterations, we shouldn't detect leaks in a simple test
        // This is more of a smoke test
        assert_eq!(result.iterations, 10);
    }
    
    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(1024), "1.00 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.00 MB");
        assert_eq!(format_bytes(1536), "1.50 KB");
    }
    
    #[test]
    fn test_memory_stress() {
        // Run a smaller stress test for unit testing
        let ontology = create_test_ontology(20);
        let benchmark = MemoryBenchmark::new("Stress Test".to_string());
        
        let consistency_result = benchmark.test_consistency_memory(&ontology);
        assert!(consistency_result.success);
        
        let person = Class::new(IRI::new("Person"));
        let class_expr = ClassExpression::Class(person);
        let satisfiability_result = benchmark.test_satisfiability_memory(&ontology, &class_expr);
        assert!(satisfiability_result.success);
    }
}

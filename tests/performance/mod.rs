//! Performance benchmarks for oxidowl
//! 
//! This module contains comprehensive performance tests similar to HermiT's benchmarks

pub mod reasoning_benchmarks;
pub mod memory_benchmarks;
pub mod scalability_tests;
pub mod conformance_tests;
pub mod algorithm_benchmarks;
pub mod integration_tests;

use std::time::{Duration, Instant};

/// Performance metrics structure
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub reasoning_time: Duration,
    pub memory_usage: usize,
    pub tableau_nodes: u64,
    pub backtracking_operations: u64,
    pub axioms_processed: usize,
}

/// Benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub timeout: Duration,
    pub memory_limit: usize,
    pub warmup_iterations: usize,
    pub test_iterations: usize,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(300), // 5 minutes
            memory_limit: 2 * 1024 * 1024 * 1024, // 2GB
            warmup_iterations: 3,
            test_iterations: 10,
        }
    }
}

/// Base trait for all benchmarks
pub trait Benchmark {
    fn name(&self) -> &str;
    fn run(&self, config: &BenchmarkConfig) -> Result<PerformanceMetrics, String>;
    fn expected_complexity(&self) -> ComplexityClass;
}

/// Complexity classes for benchmarks
#[derive(Debug, Clone, PartialEq)]
pub enum ComplexityClass {
    Polynomial,
    Exponential,
    NExpTime,
    Unknown,
}

/// Benchmark result comparison
#[derive(Debug, Clone)]
pub struct BenchmarkComparison {
    pub baseline: PerformanceMetrics,
    pub current: PerformanceMetrics,
    pub improvement_factor: f64,
    pub regression_detected: bool,
}

/// Performance regression detection
pub struct RegressionDetector {
    tolerance: f64,
}

impl RegressionDetector {
    pub fn new(tolerance: f64) -> Self {
        Self { tolerance }
    }
    
    pub fn compare(&self, baseline: &PerformanceMetrics, current: &PerformanceMetrics) -> BenchmarkComparison {
        let time_factor = current.reasoning_time.as_secs_f64() / baseline.reasoning_time.as_secs_f64();
        let memory_factor = current.memory_usage as f64 / baseline.memory_usage as f64;
        
        let regression_detected = time_factor > (1.0 + self.tolerance) || memory_factor > (1.0 + self.tolerance);
        
        BenchmarkComparison {
            baseline: baseline.clone(),
            current: current.clone(),
            improvement_factor: 1.0 / time_factor,
            regression_detected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_regression_detector() {
        let detector = RegressionDetector::new(0.1); // 10% tolerance
        
        let baseline = PerformanceMetrics {
            reasoning_time: Duration::from_millis(100),
            memory_usage: 1000,
            tableau_nodes: 50,
            backtracking_operations: 10,
            axioms_processed: 100,
        };
        
        let improved = PerformanceMetrics {
            reasoning_time: Duration::from_millis(80),
            memory_usage: 900,
            tableau_nodes: 45,
            backtracking_operations: 8,
            axioms_processed: 100,
        };
        
        let comparison = detector.compare(&baseline, &improved);
        assert!(!comparison.regression_detected);
        assert!(comparison.improvement_factor > 1.0);
    }
}

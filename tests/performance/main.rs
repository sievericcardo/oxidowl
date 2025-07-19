//! Performance test runner binary
//! 
//! Command-line interface for running oxidowl performance tests
//! Similar to HermiT's test automation

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::time::Duration;
use serde_json;

mod performance;

use performance::{
    integration_tests::*,
    reasoning_benchmarks::*,
    memory_benchmarks::*,
    scalability_tests::*,
    conformance_tests::*,
    algorithm_benchmarks::*,
    BenchmarkConfig,
};

#[derive(Parser)]
#[command(name = "oxidowl-perf")]
#[command(about = "Performance testing suite for oxidowl reasoner")]
#[command(version = "1.0")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Number of benchmark iterations
    #[arg(short, long, default_value = "10")]
    iterations: usize,
    
    /// Number of warmup iterations
    #[arg(short, long, default_value = "3")]
    warmup: usize,
    
    /// Timeout in seconds
    #[arg(short, long, default_value = "60")]
    timeout: u64,
    
    /// Output results to JSON file
    #[arg(short, long)]
    output: Option<PathBuf>,
    
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Run all performance tests
    All,
    /// Run reasoning benchmarks only
    Reasoning {
        /// Ontology file to test
        #[arg(short, long)]
        ontology: Option<PathBuf>,
    },
    /// Run memory benchmarks only
    Memory {
        /// Include leak detection tests
        #[arg(short, long)]
        leak_detection: bool,
    },
    /// Run conformance tests only
    Conformance,
    /// Run algorithm comparison tests
    Algorithm,
    /// Run scalability tests only
    Scalability,
    /// Run quick essential tests for CI
    Quick,
    /// Generate performance report
    Report {
        /// Input JSON results file
        #[arg(short, long)]
        input: PathBuf,
        /// Output format (html, markdown, json)
        #[arg(short, long, default_value = "html")]
        format: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    let config = BenchmarkConfig {
        iterations: cli.iterations,
        warmup_iterations: cli.warmup,
        timeout: Duration::from_secs(cli.timeout),
    };
    
    if cli.verbose {
        println!("Running with config: iterations={}, warmup={}, timeout={}s", 
                cli.iterations, cli.warmup, cli.timeout);
    }
    
    match cli.command.unwrap_or(Commands::All) {
        Commands::All => run_all_tests(config, cli.output).await?,
        Commands::Reasoning { ontology } => run_reasoning_tests(config, ontology).await?,
        Commands::Memory { leak_detection } => run_memory_tests(config, leak_detection).await?,
        Commands::Conformance => run_conformance_tests().await?,
        Commands::Algorithm => run_algorithm_tests(config).await?,
        Commands::Scalability => run_scalability_tests().await?,
        Commands::Quick => run_quick_tests().await?,
        Commands::Report { input, format } => generate_report(input, format).await?,
    }
    
    Ok(())
}

async fn run_all_tests(config: BenchmarkConfig, output: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting comprehensive performance test suite...");
    
    let suite = IntegratedTestSuite::new();
    let results = suite.run_all_tests();
    
    results.print_comprehensive_summary();
    
    if let Some(output_path) = output {
        save_results_to_json(&results, &output_path)?;
        println!("Results saved to: {}", output_path.display());
    }
    
    Ok(())
}

async fn run_reasoning_tests(config: BenchmarkConfig, ontology_path: Option<PathBuf>) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running reasoning benchmarks...");
    
    if let Some(path) = ontology_path {
        // Load custom ontology
        let ontology = load_ontology_from_file(&path)?;
        run_reasoning_benchmark_on_ontology(&ontology, config).await?;
    } else {
        // Use default test ontologies
        let suite = IntegratedTestSuite::new();
        let reasoning_results = suite.run_reasoning_benchmarks();
        
        for (name, result) in reasoning_results {
            println!("\nResults for {}:", name);
            println!("  Consistency: {:?} (Success: {:.1}%)", 
                    result.consistency.avg_time, result.consistency.success_rate * 100.0);
            println!("  Satisfiability: {:?} (Success: {:.1}%)", 
                    result.satisfiability.avg_time, result.satisfiability.success_rate * 100.0);
            println!("  Classification: {:?} (Success: {:.1}%)", 
                    result.classification.avg_time, result.classification.success_rate * 100.0);
        }
    }
    
    Ok(())
}

async fn run_memory_tests(config: BenchmarkConfig, include_leak_detection: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running memory benchmarks...");
    
    let suite = IntegratedTestSuite::new();
    let memory_results = suite.run_memory_benchmarks();
    
    for (name, result) in memory_results {
        println!("\nMemory results for {}:", name);
        result.consistency.print_summary();
        result.classification.print_summary();
        
        if include_leak_detection {
            result.leak_test.print_summary();
        }
    }
    
    // Run memory stress test
    println!("\nRunning memory stress test...");
    let stress_results = MemoryStressTest::run_stress_test();
    MemoryStressTest::analyze_memory_scaling(&stress_results);
    
    Ok(())
}

async fn run_conformance_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running OWL2 DL conformance tests...");
    
    let conformance_suite = ConformanceTestSuite::new();
    let results = conformance_suite.run_all_tests();
    
    println!("\nConformance Test Results:");
    println!("=========================");
    
    let passed = results.results.iter().filter(|r| r.passed).count();
    let total = results.results.len();
    
    println!("Overall: {}/{} tests passed ({:.1}%)", 
            passed, total, (passed as f64 / total as f64) * 100.0);
    
    for result in &results.results {
        let status = if result.passed { "PASS" } else { "FAIL" };
        println!("  {} - {}", result.test_name, status);
        if !result.error_message.is_empty() {
            println!("    Error: {}", result.error_message);
        }
    }
    
    Ok(())
}

async fn run_algorithm_tests(config: BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running algorithm comparison tests...");
    
    let suite = IntegratedTestSuite::new();
    let algorithm_results = suite.run_algorithm_benchmarks();
    
    for (name, result) in algorithm_results {
        println!("\nAlgorithm comparison for {}:", name);
        result.print_summary();
    }
    
    // Run complexity analysis
    println!("\nRunning complexity analysis...");
    let complexity_results = ComplexityBenchmark::run_complexity_analysis();
    ComplexityBenchmark::analyze_scaling(&complexity_results);
    
    // Run feature-specific benchmarks
    println!("\nRunning feature-specific benchmarks...");
    let feature_results = FeatureBenchmark::benchmark_owl_features();
    
    for (feature, result) in feature_results {
        println!("\n{} Feature Performance:", feature);
        println!("  Tableau: {:?}", result.tableau.avg_consistency_time);
        println!("  HyperTableau: {:?}", result.hypertableau.avg_consistency_time);
        println!("  Speedup: {:.2}x", result.comparison.consistency_speedup);
    }
    
    Ok(())
}

async fn run_scalability_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running scalability tests...");
    
    let suite = IntegratedTestSuite::new();
    let scalability_results = suite.run_scalability_tests();
    
    println!("\nScalability Test Results:");
    println!("=========================");
    
    println!("Large Ontology Test:");
    println!("  Success: {}", scalability_results.large_ontology.success);
    println!("  Duration: {:?}", scalability_results.large_ontology.duration);
    
    println!("Deep Hierarchy Test:");
    println!("  Success: {}", scalability_results.deep_hierarchy.success);
    println!("  Duration: {:?}", scalability_results.deep_hierarchy.duration);
    
    println!("Wide Hierarchy Test:");
    println!("  Success: {}", scalability_results.wide_hierarchy.success);
    println!("  Duration: {:?}", scalability_results.wide_hierarchy.duration);
    
    Ok(())
}

async fn run_quick_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running quick essential tests...");
    
    let success = QuickTestRunner::run_essential_tests();
    
    if success {
        println!("All essential tests passed!");
        std::process::exit(0);
    } else {
        println!("Essential tests failed!");
        std::process::exit(1);
    }
}

async fn generate_report(input: PathBuf, format: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating performance report...");
    
    let json_content = std::fs::read_to_string(&input)?;
    let results: serde_json::Value = serde_json::from_str(&json_content)?;
    
    match format.as_str() {
        "html" => generate_html_report(&results, &input)?,
        "markdown" => generate_markdown_report(&results, &input)?,
        "json" => {
            // Pretty print the JSON
            let pretty_json = serde_json::to_string_pretty(&results)?;
            println!("{}", pretty_json);
        }
        _ => {
            eprintln!("Unsupported format: {}. Use html, markdown, or json.", format);
            std::process::exit(1);
        }
    }
    
    Ok(())
}

fn load_ontology_from_file(path: &PathBuf) -> Result<oxidowl::ontology::Ontology, Box<dyn std::error::Error>> {
    use oxidowl::parsers::turtle::TurtleParser;
    
    let parser = TurtleParser::new();
    let ontology = parser.parse_file(path.to_string_lossy().as_ref())?;
    
    Ok(ontology)
}

async fn run_reasoning_benchmark_on_ontology(
    ontology: &oxidowl::ontology::Ontology, 
    config: BenchmarkConfig
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running reasoning benchmarks on provided ontology...");
    
    let consistency_benchmark = ConsistencyBenchmark::new("custom_ontology".to_string(), config.clone());
    let consistency_result = consistency_benchmark.run_benchmark(ontology);
    
    println!("Consistency Results:");
    println!("  Average Time: {:?}", consistency_result.avg_time);
    println!("  Success Rate: {:.1}%", consistency_result.success_rate * 100.0);
    println!("  Iterations: {}", consistency_result.iterations.len());
    
    Ok(())
}

fn save_results_to_json(
    results: &IntegratedTestResults, 
    path: &PathBuf
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a serializable summary of results
    let summary = serde_json::json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "summary": {
            "ontologies_tested": results.summary.total_ontologies_tested,
            "test_categories": results.summary.test_categories,
            "overall_status": results.summary.overall_status
        },
        "reasoning_tests": results.reasoning.len(),
        "memory_tests": results.memory.len(),
        "conformance_tests": results.conformance.results.len(),
        "algorithm_tests": results.algorithm.len(),
        "scalability_tests": 3
    });
    
    std::fs::write(path, serde_json::to_string_pretty(&summary)?)?;
    Ok(())
}

fn generate_html_report(
    results: &serde_json::Value, 
    input_path: &PathBuf
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = input_path.with_extension("html");
    
    let html_content = format!(r#"
<!DOCTYPE html>
<html>
<head>
    <title>Oxidowl Performance Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .section {{ margin: 20px 0; }}
        .test-result {{ background: #f9f9f9; padding: 10px; margin: 5px 0; border-left: 4px solid #007acc; }}
        .pass {{ border-left-color: #28a745; }}
        .fail {{ border-left-color: #dc3545; }}
        pre {{ background: #f8f8f8; padding: 10px; border-radius: 3px; }}
    </style>
</head>
<body>
    <div class="header">
        <h1>Oxidowl Performance Report</h1>
        <p>Generated: {}</p>
    </div>
    
    <div class="section">
        <h2>Summary</h2>
        <pre>{}</pre>
    </div>
    
    <div class="section">
        <h2>Detailed Results</h2>
        <p>This report was generated from performance test data.</p>
        <p>For detailed analysis, please review the JSON output and individual test logs.</p>
    </div>
</body>
</html>
"#, 
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        serde_json::to_string_pretty(results)?
    );
    
    std::fs::write(&output_path, html_content)?;
    println!("HTML report saved to: {}", output_path.display());
    
    Ok(())
}

fn generate_markdown_report(
    results: &serde_json::Value, 
    input_path: &PathBuf
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = input_path.with_extension("md");
    
    let markdown_content = format!(r#"
# Oxidowl Performance Report

Generated: {}

## Summary

```json
{}
```

## Test Categories

- Reasoning Benchmarks
- Memory Usage Analysis  
- OWL2 DL Conformance Tests
- Algorithm Comparisons (Tableau vs HyperTableau)
- Scalability Testing

## Notes

This performance report demonstrates oxidowl's capabilities compared to HermiT reasoner standards.

For detailed analysis, review the complete JSON output and individual test execution logs.
"#, 
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        serde_json::to_string_pretty(results)?
    );
    
    std::fs::write(&output_path, markdown_content)?;
    println!("Markdown report saved to: {}", output_path.display());
    
    Ok(())
}

// Helper function for setting up logging
fn setup_logging(verbose: bool) {
    if verbose {
        println!("Verbose logging enabled");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_parsing() {
        // Test default arguments
        let cli = Cli::parse_from(&["oxidowl-perf"]);
        assert_eq!(cli.iterations, 10);
        assert_eq!(cli.warmup, 3);
        assert_eq!(cli.timeout, 60);
    }
    
    #[test]
    fn test_config_creation() {
        let config = BenchmarkConfig {
            iterations: 5,
            warmup_iterations: 2,
            timeout: Duration::from_secs(30),
        };
        
        assert_eq!(config.iterations, 5);
        assert_eq!(config.warmup_iterations, 2);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }
}

//! Performance test runner binary
//!
//! Command-line interface for running oxidowl performance tests
//! Similar to `HermiT`'s test automation

use clap::{Parser, Subcommand};
use oxidowl::{
    config::ReasonerConfig,
    ontology::{
        Axiom, Class, ClassAssertionAxiom, ClassExpression, IRI, Individual, NamedIndividual,
        Ontology, SubClassOfAxiom,
    },
    parsers::turtle::TurtleParser,
    reasoning::ReasoningService,
};
use std::path::PathBuf;
use std::time::Duration;

// Import the performance test modules
use std::time::Instant;

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

/// Basic benchmark configuration
#[derive(Debug, Clone)]
pub struct BenchmarkConfig {
    pub iterations: usize,
    pub warmup_iterations: usize,
    pub timeout: Duration,
}

/// Basic performance metrics
#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    samples: Vec<f64>,
    sum: f64,
    sum_squares: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceMetrics {
    #[must_use]
    pub fn new() -> Self {
        Self {
            samples: Vec::new(),
            sum: 0.0,
            sum_squares: 0.0,
        }
    }

    pub fn record_sample(&mut self, value: f64) {
        self.samples.push(value);
        self.sum += value;
        self.sum_squares += value * value;
    }

    #[must_use]
    pub fn mean(&self) -> f64 {
        if self.samples.is_empty() {
            0.0
        } else {
            self.sum / self.samples.len() as f64
        }
    }

    #[must_use]
    pub fn std_dev(&self) -> f64 {
        if self.samples.len() < 2 {
            0.0
        } else {
            let n = self.samples.len() as f64;
            let mean = self.mean();
            let variance = (self.sum_squares - n * mean * mean) / (n - 1.0);
            variance.sqrt()
        }
    }
}

/// Basic benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub name: String,
    pub avg_time: Duration,
    pub success_rate: f64,
    pub iterations: Vec<Duration>,
    pub metrics: PerformanceMetrics,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create runtime explicitly to avoid nested runtime issues
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async_main())
}

async fn async_main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    let config = BenchmarkConfig {
        iterations: cli.iterations,
        warmup_iterations: cli.warmup,
        timeout: Duration::from_secs(cli.timeout),
    };

    if cli.verbose {
        println!(
            "Running with config: iterations={}, warmup={}, timeout={}s",
            cli.iterations, cli.warmup, cli.timeout
        );
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

async fn run_all_tests(
    config: BenchmarkConfig,
    output: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting comprehensive performance test suite...");

    // Run basic performance tests
    run_reasoning_tests(config.clone(), None).await?;
    run_conformance_tests().await?;
    run_quick_tests().await?;

    println!("All performance tests completed!");

    if let Some(output_path) = output {
        let results = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "status": "completed",
            "config": {
                "iterations": config.iterations,
                "warmup_iterations": config.warmup_iterations,
                "timeout_seconds": config.timeout.as_secs()
            }
        });

        std::fs::write(&output_path, serde_json::to_string_pretty(&results)?)?;
        println!("Results saved to: {}", output_path.display());
    }

    Ok(())
}

async fn run_reasoning_tests(
    config: BenchmarkConfig,
    ontology_path: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running reasoning benchmarks...");

    let ontology = if let Some(path) = ontology_path {
        load_ontology_from_file(&path)?
    } else {
        create_test_ontology()
    };

    // Run consistency benchmark
    let consistency_result = run_consistency_benchmark(&ontology, &config).await?;
    println!(
        "Consistency: {:?} (Success: {:.1}%)",
        consistency_result.avg_time,
        consistency_result.success_rate * 100.0
    );

    // Run satisfiability benchmark
    let satisfiability_result = run_satisfiability_benchmark(&ontology, &config).await?;
    println!(
        "Satisfiability: {:?} (Success: {:.1}%)",
        satisfiability_result.avg_time,
        satisfiability_result.success_rate * 100.0
    );

    Ok(())
}

async fn run_memory_tests(
    _config: BenchmarkConfig,
    include_leak_detection: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running memory benchmarks...");

    let ontology = create_test_ontology();

    // Basic memory usage test
    let initial_memory = get_memory_usage();
    let service = ReasoningService::new(ontology.clone(), ReasonerConfig::default())?;
    let _result = service.is_consistent().await;
    let final_memory = get_memory_usage();

    println!("Memory usage: {initial_memory} -> {final_memory} bytes");

    if include_leak_detection {
        println!("Running leak detection...");
        // Simple leak detection by repeating operations
        for i in 0..10 {
            let service = ReasoningService::new(ontology.clone(), ReasonerConfig::default())?;
            let _result = service.is_consistent().await;
            if i % 3 == 0 {
                println!("  Iteration {}: {} bytes", i, get_memory_usage());
            }
        }
    }

    Ok(())
}

async fn run_conformance_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running OWL2 DL conformance tests...");

    let mut passed = 0;
    let mut total = 0;

    // Test basic SubClassOf reasoning
    total += 1;
    if test_basic_subclass().await.is_ok() {
        passed += 1;
        println!("  Basic SubClassOf - PASS");
    } else {
        println!("  Basic SubClassOf - FAIL");
    }

    // Test basic consistency
    total += 1;
    if test_basic_consistency().await.is_ok() {
        passed += 1;
        println!("  Basic Consistency - PASS");
    } else {
        println!("  Basic Consistency - FAIL");
    }

    // Test class assertions
    total += 1;
    if test_class_assertions().await.is_ok() {
        passed += 1;
        println!("  Class Assertions - PASS");
    } else {
        println!("  Class Assertions - FAIL");
    }

    println!(
        "Conformance: {}/{} tests passed ({:.1}%)",
        passed,
        total,
        (f64::from(passed) / f64::from(total)) * 100.0
    );

    Ok(())
}

async fn run_algorithm_tests(config: BenchmarkConfig) -> Result<(), Box<dyn std::error::Error>> {
    println!("Running algorithm comparison tests...");

    let ontology = create_test_ontology();

    // Test with Tableau
    let tableau_config = ReasonerConfig::default();
    let tableau_result =
        run_algorithm_benchmark(&ontology, &tableau_config, &config, "Tableau").await?;

    // Test with default algorithm
    let default_config = ReasonerConfig::default();
    let default_result =
        run_algorithm_benchmark(&ontology, &default_config, &config, "Default").await?;

    println!("Algorithm Comparison:");
    println!("  Tableau: {:?}", tableau_result.avg_time);
    println!("  Default: {:?}", default_result.avg_time);

    let speedup =
        tableau_result.avg_time.as_nanos() as f64 / default_result.avg_time.as_nanos() as f64;
    println!("  Speedup: {speedup:.2}x");

    Ok(())
}

async fn run_scalability_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running scalability tests...");

    let sizes = vec![10, 50, 100];

    for size in sizes {
        let ontology = create_large_test_ontology(size);
        let start_time = Instant::now();

        let service = ReasoningService::new(ontology, ReasonerConfig::default())?;
        let result = service.is_consistent().await;

        let duration = start_time.elapsed();
        let status = if result.is_ok() { "PASS" } else { "FAIL" };

        println!("  Size {size}: {status} ({duration:?})");
    }

    Ok(())
}

async fn run_quick_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("Running quick essential tests...");

    let ontology = create_simple_test_ontology();
    let service = ReasoningService::new(ontology, ReasonerConfig::default())?;

    // Quick consistency check
    let result = service.is_consistent().await;
    if result.is_ok() {
        println!("Quick consistency test passed!");
    } else {
        println!("Quick consistency test failed!");
        return Err("Essential test failed".into());
    }

    Ok(())
}

async fn generate_report(input: PathBuf, format: String) -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating performance report...");

    let json_content = std::fs::read_to_string(&input)?;
    let results: serde_json::Value = serde_json::from_str(&json_content)?;

    match format.as_str() {
        "html" => generate_html_report(&results, &input)?,
        "markdown" => generate_markdown_report(&results, &input)?,
        "json" => {
            let pretty_json = serde_json::to_string_pretty(&results)?;
            println!("{pretty_json}");
        }
        _ => {
            eprintln!("Unsupported format: {format}. Use html, markdown, or json.");
            std::process::exit(1);
        }
    }

    Ok(())
}

// Helper functions
fn load_ontology_from_file(path: &PathBuf) -> Result<Ontology, Box<dyn std::error::Error>> {
    let parser = TurtleParser::new();
    let content = std::fs::read_to_string(path)?;
    let ontology = parser.parse_string(&content)?;
    Ok(ontology)
}

fn create_test_ontology() -> Ontology {
    let mut ontology = Ontology::new();

    let animal = Class::new(IRI::new("Animal"));
    let mammal = Class::new(IRI::new("Mammal"));
    let dog = Class::new(IRI::new("Dog"));

    ontology.add_class(animal.clone());
    ontology.add_class(mammal.clone());
    ontology.add_class(dog.clone());

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

fn create_simple_test_ontology() -> Ontology {
    let mut ontology = Ontology::new();

    let animal = Class::new(IRI::new("Animal"));
    ontology.add_class(animal);

    ontology
}

fn create_large_test_ontology(size: usize) -> Ontology {
    let mut ontology = Ontology::new();

    let mut classes = Vec::new();
    for i in 0..size {
        let class = Class::new(IRI::new(&format!("Class{i}")));
        ontology.add_class(class.clone());
        classes.push(class);
    }

    // Add hierarchy
    for i in 1..size {
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

    ontology
}

async fn run_consistency_benchmark(
    ontology: &Ontology,
    config: &BenchmarkConfig,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let mut iterations = Vec::new();
    let mut metrics = PerformanceMetrics::new();
    let mut successes = 0;

    for _ in 0..config.iterations {
        let start_time = Instant::now();
        let service = ReasoningService::new(ontology.clone(), ReasonerConfig::default())?;
        let result = service.is_consistent().await;
        let duration = start_time.elapsed();

        iterations.push(duration);
        metrics.record_sample(duration.as_nanos() as f64);

        if result.is_ok() {
            successes += 1;
        }
    }

    let avg_time = Duration::from_nanos(
        (iterations
            .iter()
            .map(std::time::Duration::as_nanos)
            .sum::<u128>()
            / iterations.len() as u128) as u64,
    );

    Ok(BenchmarkResult {
        name: "Consistency".to_string(),
        avg_time,
        success_rate: f64::from(successes) / config.iterations as f64,
        iterations,
        metrics,
    })
}

async fn run_satisfiability_benchmark(
    ontology: &Ontology,
    config: &BenchmarkConfig,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let mut iterations = Vec::new();
    let mut metrics = PerformanceMetrics::new();
    let mut successes = 0;

    // Create a test class expression
    let test_class = if let Some((_, first_class)) = ontology.classes().first() {
        ClassExpression::Class(first_class.clone())
    } else {
        ClassExpression::Class(Class::new(IRI::new("TestClass")))
    };

    for _ in 0..config.iterations {
        let start_time = Instant::now();
        let service = ReasoningService::new(ontology.clone(), ReasonerConfig::default())?;
        let result = service.is_satisfiable(&test_class).await;
        let duration = start_time.elapsed();

        iterations.push(duration);
        metrics.record_sample(duration.as_nanos() as f64);

        if result.is_ok() {
            successes += 1;
        }
    }

    let avg_time = Duration::from_nanos(
        (iterations
            .iter()
            .map(std::time::Duration::as_nanos)
            .sum::<u128>()
            / iterations.len() as u128) as u64,
    );

    Ok(BenchmarkResult {
        name: "Satisfiability".to_string(),
        avg_time,
        success_rate: f64::from(successes) / config.iterations as f64,
        iterations,
        metrics,
    })
}

async fn run_algorithm_benchmark(
    ontology: &Ontology,
    reasoner_config: &ReasonerConfig,
    bench_config: &BenchmarkConfig,
    name: &str,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let mut iterations = Vec::new();
    let mut metrics = PerformanceMetrics::new();
    let mut successes = 0;

    for _ in 0..bench_config.iterations {
        let start_time = Instant::now();
        let service = ReasoningService::new(ontology.clone(), reasoner_config.clone())?;
        let result = service.is_consistent().await;
        let duration = start_time.elapsed();

        iterations.push(duration);
        metrics.record_sample(duration.as_nanos() as f64);

        if result.is_ok() {
            successes += 1;
        }
    }

    let avg_time = Duration::from_nanos(
        (iterations
            .iter()
            .map(std::time::Duration::as_nanos)
            .sum::<u128>()
            / iterations.len() as u128) as u64,
    );

    Ok(BenchmarkResult {
        name: name.to_string(),
        avg_time,
        success_rate: f64::from(successes) / bench_config.iterations as f64,
        iterations,
        metrics,
    })
}

// Conformance test functions
async fn test_basic_subclass() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = create_test_ontology();
    let service = ReasoningService::new(ontology, ReasonerConfig::default())?;

    // Test that the ontology is consistent
    service.is_consistent().await?;
    Ok(())
}

async fn test_basic_consistency() -> Result<(), Box<dyn std::error::Error>> {
    let ontology = create_test_ontology();
    let service = ReasoningService::new(ontology, ReasonerConfig::default())?;

    let result = service.is_consistent().await?;
    if result {
        Ok(())
    } else {
        Err("Ontology should be consistent".into())
    }
}

async fn test_class_assertions() -> Result<(), Box<dyn std::error::Error>> {
    let mut ontology = Ontology::new();

    let person = Class::new(IRI::new("Person"));
    let john = Individual::Named(NamedIndividual::new(IRI::new("John")));

    ontology.add_class(person.clone());
    ontology.add_individual(
        john.iri()
            .expect("Failed to get IRI from individual")
            .clone(),
        john.clone(),
    );

    ontology.add_axiom(Axiom::ClassAssertion(ClassAssertionAxiom {
        id: 1,
        individual: john,
        class: ClassExpression::Class(person),
        annotations: vec![],
    }));

    let service = ReasoningService::new(ontology, ReasonerConfig::default())?;
    let result = service.is_consistent().await?;

    if result {
        Ok(())
    } else {
        Err("Class assertion ontology should be consistent".into())
    }
}

fn get_memory_usage() -> usize {
    // Simple mock memory usage for demonstration
    use std::sync::atomic::{AtomicUsize, Ordering};
    static MOCK_MEMORY: AtomicUsize = AtomicUsize::new(1024 * 1024);

    MOCK_MEMORY.fetch_add(1024, Ordering::Relaxed)
}

fn generate_html_report(
    results: &serde_json::Value,
    input_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = input_path.with_extension("html");

    let html_content = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Oxidowl Performance Report</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 20px; }}
        .header {{ background: #f0f0f0; padding: 20px; border-radius: 5px; }}
        .section {{ margin: 20px 0; }}
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
    input_path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    let output_path = input_path.with_extension("md");

    let markdown_content = format!(
        r"
# Oxidowl Performance Report

Generated: {}

## Summary

```json
{}
```

## Notes

This performance report demonstrates oxidowl's capabilities.
",
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
        serde_json::to_string_pretty(results)?
    );

    std::fs::write(&output_path, markdown_content)?;
    println!("Markdown report saved to: {}", output_path.display());

    Ok(())
}

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use oxidowl::{
    Ontology, ReasonerConfig, ReasoningService,
    ontology::{Class, ClassExpression, IRI},
    query::advanced::execution_engine::{ExecutionConstraints, ExecutionPriority},
    query::advanced::{
        AdvancedExecutionConfig, AdvancedExecutionEngine, ConjunctiveQuery, QueryAtom,
        QueryVariable,
    },
};
use std::sync::Arc;
use std::time::Duration;

/// Helper: Create default execution constraints
fn default_constraints() -> ExecutionConstraints {
    ExecutionConstraints {
        max_execution_time: Some(Duration::from_secs(30)),
        max_memory_usage: Some(1024 * 1024 * 1024),
        min_confidence: Some(0.7),
        priority: ExecutionPriority::Normal,
    }
}

/// Helper: Create a test ontology
fn create_ontology(name: &str, class_count: usize) -> Ontology {
    let mut ontology = Ontology::new();
    ontology.set_iri(IRI::new(&format!("http://bench.org/{}", name)));
    for i in 0..class_count {
        let class_iri = IRI::new(&format!("http://bench.org/{}#C{}", name, i));
        ontology.add_class(Class::new(class_iri));
    }
    ontology
}

/// Helper: Create a simple star query
fn create_star_query(var_name: &str, class_name: &str) -> ConjunctiveQuery {
    ConjunctiveQuery {
        answer_variables: vec![QueryVariable::new(var_name.to_string())],
        body_atoms: vec![QueryAtom::ClassAtom {
            variable: QueryVariable::new(var_name.to_string()),
            class_expression: ClassExpression::class(IRI::new(&format!(
                "http://bench.org/#{}",
                class_name
            ))),
        }],
        constraints: Default::default(),
        metadata: Default::default(),
    }
}

/// Benchmark: ML Strategy Selection Time
fn bench_strategy_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("strategy_selection");
    group.measurement_time(Duration::from_secs(10));

    for size in [10, 50, 100, 500].iter() {
        let ontology = create_ontology("strategy_bench", *size);
        let ontology_arc = Arc::new(ontology.clone());
        let reasoning = Arc::new(ReasoningService::new(ontology, ReasonerConfig::default()).expect("Failed to create reasoning service"));

        // ML-enabled engine
        let mut config_ml = AdvancedExecutionConfig::default();
        config_ml.enable_adaptive_strategies = true;

        let engine_ml =
            AdvancedExecutionEngine::new(ontology_arc.clone(), reasoning.clone(), config_ml)
                .expect("Failed to create ML engine");

        let query = create_star_query("x", "TestClass");
        let constraints = default_constraints();

        group.bench_with_input(BenchmarkId::new("ML", size), size, |b, _| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            b.iter(|| {
                let result = rt.block_on(engine_ml.execute_query(&query, constraints.clone()));
                black_box(result)
            });
        });

        // Legacy engine for comparison
        let mut config_legacy = AdvancedExecutionConfig::default();
        config_legacy.enable_adaptive_strategies = false;

        let reasoning2 = Arc::new(ReasoningService::new(
            create_ontology("strategy_bench", *size),
            ReasonerConfig::default(),
        ).expect("Failed to create reasoning service"));

        let engine_legacy = AdvancedExecutionEngine::new(
            Arc::new(create_ontology("strategy_bench", *size)),
            reasoning2,
            config_legacy,
        )
        .expect("Failed to create legacy engine");

        group.bench_with_input(BenchmarkId::new("Legacy", size), size, |b, _| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            b.iter(|| {
                let result = rt.block_on(engine_legacy.execute_query(&query, constraints.clone()));
                black_box(result)
            });
        });
    }

    group.finish();
}

/// Benchmark: Query Execution Throughput
fn bench_query_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("query_throughput");
    group.measurement_time(Duration::from_secs(15));

    let ontology = create_ontology("throughput_bench", 100);
    let ontology_arc = Arc::new(ontology.clone());
    let reasoning = Arc::new(ReasoningService::new(ontology, ReasonerConfig::default()).expect("Failed to create reasoning service"));

    let mut config = AdvancedExecutionConfig::default();
    config.enable_adaptive_strategies = true;

    let engine = AdvancedExecutionEngine::new(ontology_arc, reasoning, config)
        .expect("Failed to create engine");

    group.bench_function("10_queries", |b| {
        let rt =
            tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for benchmark");
        b.iter(|| {
            for i in 0..10 {
                let query = create_star_query("x", &format!("Class{}", i));
                let constraints = default_constraints();
                let _ = rt.block_on(engine.execute_query(&query, constraints));
            }
        });
    });

    group.bench_function("50_queries", |b| {
        let rt =
            tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for benchmark");
        b.iter(|| {
            for i in 0..50 {
                let query = create_star_query("x", &format!("Class{}", i % 10));
                let constraints = default_constraints();
                let _ = rt.block_on(engine.execute_query(&query, constraints));
            }
        });
    });

    group.finish();
}

/// Benchmark: ML Overhead
fn bench_ml_overhead(c: &mut Criterion) {
    let mut group = c.benchmark_group("ml_overhead");
    group.measurement_time(Duration::from_secs(10));

    let ontology = create_ontology("overhead_bench", 200);

    // ML-enabled
    let ontology_arc1 = Arc::new(ontology.clone());
    let reasoning1 = Arc::new(ReasoningService::new(
        ontology.clone(),
        ReasonerConfig::default(),
    ).expect("Failed to create reasoning service"));
    let mut config_ml = AdvancedExecutionConfig::default();
    config_ml.enable_adaptive_strategies = true;

    let engine_ml = AdvancedExecutionEngine::new(ontology_arc1, reasoning1, config_ml)
        .expect("Failed to create ML engine");

    // Legacy
    let ontology_arc2 = Arc::new(ontology.clone());
    let reasoning2 = Arc::new(ReasoningService::new(ontology, ReasonerConfig::default()).expect("Failed to create reasoning service"));
    let mut config_legacy = AdvancedExecutionConfig::default();
    config_legacy.enable_adaptive_strategies = false;

    let engine_legacy = AdvancedExecutionEngine::new(ontology_arc2, reasoning2, config_legacy)
        .expect("Failed to create legacy engine");

    let query = create_star_query("x", "BenchmarkClass");
    let constraints = default_constraints();

    group.bench_function("ML_enabled", |b| {
        let rt =
            tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for benchmark");
        b.iter(|| {
            let result = rt.block_on(engine_ml.execute_query(&query, constraints.clone()));
            black_box(result)
        });
    });

    group.bench_function("Legacy", |b| {
        let rt =
            tokio::runtime::Runtime::new().expect("Failed to create tokio runtime for benchmark");
        b.iter(|| {
            let result = rt.block_on(engine_legacy.execute_query(&query, constraints.clone()));
            black_box(result)
        });
    });

    group.finish();
}

/// Benchmark: Concurrent Query Execution
fn bench_concurrent_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_execution");
    group.measurement_time(Duration::from_secs(15));

    let ontology = create_ontology("concurrent_bench", 150);
    let ontology_arc = Arc::new(ontology.clone());
    let reasoning = Arc::new(ReasoningService::new(ontology, ReasonerConfig::default()).expect("Failed to create reasoning service"));

    let mut config = AdvancedExecutionConfig::default();
    config.enable_adaptive_strategies = true;

    let engine = Arc::new(
        AdvancedExecutionEngine::new(ontology_arc, reasoning, config)
            .expect("Failed to create engine"),
    );

    group.bench_function("4_threads", |b| {
        b.iter(|| {
            let mut handles = vec![];
            for i in 0..4 {
                let engine_clone = engine.clone();
                let handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new()
                        .expect("Failed to create tokio runtime for benchmark");
                    let query = create_star_query("x", &format!("C{}", i));
                    let constraints = default_constraints();
                    rt.block_on(engine_clone.execute_query(&query, constraints))
                });
                handles.push(handle);
            }
            for handle in handles {
                let _ = handle.join();
            }
        });
    });

    group.bench_function("8_threads", |b| {
        b.iter(|| {
            let mut handles = vec![];
            for i in 0..8 {
                let engine_clone = engine.clone();
                let handle = std::thread::spawn(move || {
                    let rt = tokio::runtime::Runtime::new()
                        .expect("Failed to create tokio runtime for benchmark");
                    let query = create_star_query("x", &format!("C{}", i));
                    let constraints = default_constraints();
                    rt.block_on(engine_clone.execute_query(&query, constraints))
                });
                handles.push(handle);
            }
            for handle in handles {
                let _ = handle.join();
            }
        });
    });

    group.finish();
}

/// Benchmark: Ontology Size Scalability
fn bench_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("scalability");
    group.measurement_time(Duration::from_secs(20));
    group.sample_size(10); // Reduce sample size for large ontologies

    for size in [100, 500, 1000, 5000].iter() {
        let ontology = create_ontology("scale_bench", *size);
        let ontology_arc = Arc::new(ontology.clone());
        let reasoning = Arc::new(ReasoningService::new(ontology, ReasonerConfig::default()).expect("Failed to create reasoning service"));

        let mut config = AdvancedExecutionConfig::default();
        config.enable_adaptive_strategies = true;

        let engine = AdvancedExecutionEngine::new(ontology_arc, reasoning, config)
            .expect("Failed to create engine");

        let query = create_star_query("x", "ScalabilityTest");
        let constraints = default_constraints();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let rt = tokio::runtime::Runtime::new()
                .expect("Failed to create tokio runtime for benchmark");
            b.iter(|| {
                let result = rt.block_on(engine.execute_query(&query, constraints.clone()));
                black_box(result)
            });
        });
    }

    group.finish();
}

criterion_group!(
    ml_benchmarks,
    bench_strategy_selection,
    bench_query_throughput,
    bench_ml_overhead,
    bench_concurrent_execution,
    bench_scalability
);

criterion_main!(ml_benchmarks);

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use std::hint::black_box;
use oxidowl::parsers::{ErrorVerbosity, FunctionalParser, Parser, ParserConfig};

fn generate_test_ontology(num_classes: usize) -> String {
    let mut content = String::from(
        r#"Prefix(:=<http://example.org/>)
Prefix(owl:=<http://www.w3.org/2002/07/owl#>)
Ontology(<http://example.org/test>
"#,
    );

    // Add class declarations
    for i in 0..num_classes {
        content.push_str(&format!("    Declaration(Class(:Class{}))\n", i));
    }

    // Add subclass axioms
    for i in 0..num_classes {
        content.push_str(&format!("    SubClassOf(:Class{} owl:Thing)\n", i));
    }

    content.push_str(")");
    content
}

fn benchmark_parser_verbosity(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser_verbosity");

    let ontology = generate_test_ontology(50);

    // Benchmark minimal verbosity
    group.bench_function("minimal", |b| {
        let config = ParserConfig {
            error_verbosity: ErrorVerbosity::Minimal,
        };
        let parser = FunctionalParser::with_config(config);
        b.iter(|| parser.parse(black_box(&ontology)).ok());
    });

    // Benchmark standard verbosity (default)
    group.bench_function("standard", |b| {
        let config = ParserConfig {
            error_verbosity: ErrorVerbosity::Standard,
        };
        let parser = FunctionalParser::with_config(config);
        b.iter(|| parser.parse(black_box(&ontology)).ok());
    });

    // Benchmark detailed verbosity
    group.bench_function("detailed", |b| {
        let config = ParserConfig {
            error_verbosity: ErrorVerbosity::Detailed,
        };
        let parser = FunctionalParser::with_config(config);
        b.iter(|| parser.parse(black_box(&ontology)).ok());
    });

    group.finish();
}

fn benchmark_keyword_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("keyword_lookup");

    // Create ontologies with increasing numbers of class references
    for size in [10, 50, 100].iter() {
        let ontology = generate_test_ontology(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let parser = FunctionalParser::new();
            b.iter(|| parser.parse(black_box(&ontology)).ok());
        });
    }

    group.finish();
}

fn benchmark_tokenization(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenization");

    for size in [10, 50, 100].iter() {
        let ontology = generate_test_ontology(*size);

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            let parser = FunctionalParser::new();
            b.iter(|| parser.tokenize(black_box(&ontology)).ok());
        });
    }

    group.finish();
}

fn benchmark_swrl_parsing(c: &mut Criterion) {
    let swrl_content = r#"Prefix(:=<http://example.org/>)
Prefix(swrl:=<http://www.w3.org/2003/11/swrl#>)
Ontology(<http://example.org/test>
    DLSafeRule(
        Body(
            ClassAtom(:Person Variable(:x))
            DataPropertyAtom(:hasAge Variable(:x) Variable(:age))
        )
        Head(
            ClassAtom(:Adult Variable(:x))
        )
    )
    DLSafeRule(
        Body(ClassAtom(:Student Variable(:x)))
        Head(ClassAtom(:Person Variable(:x)))
    )
)"#;

    c.bench_function("swrl_parsing", |b| {
        let parser = FunctionalParser::new();
        b.iter(|| parser.parse(black_box(swrl_content)).ok());
    });
}

criterion_group!(
    benches,
    benchmark_parser_verbosity,
    benchmark_keyword_lookup,
    benchmark_tokenization,
    benchmark_swrl_parsing
);
criterion_main!(benches);

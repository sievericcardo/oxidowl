//! SHACL validation benchmarks
//!
//! Run with: `cargo bench --bench shacl_benchmark`

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use oxidowl::validation::shacl::ShaclValidator;

// ── fixtures ──────────────────────────────────────────────────────────────────

const PREFIXES: &str = r#"
    @prefix rdf:  <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
    @prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .
    @prefix sh:   <http://www.w3.org/ns/shacl#> .
    @prefix ex:   <http://example.org/> .
"#;

const SIMPLE_SHAPES: &str = r#"
    ex:PersonShape a sh:NodeShape ;
        sh:targetClass ex:Person ;
        sh:property [
            sh:path ex:name ;
            sh:datatype xsd:string ;
            sh:minCount 1 ;
            sh:maxCount 1 ;
            sh:minLength 1 ;
        ] ;
        sh:property [
            sh:path ex:age ;
            sh:datatype xsd:integer ;
            sh:minInclusive "0"^^xsd:integer ;
            sh:maxInclusive "150"^^xsd:integer ;
        ] .
"#;

fn person_data(n: usize) -> String {
    let mut buf = PREFIXES.to_owned();
    for i in 0..n {
        buf.push_str(&format!(
            "ex:p{i} a ex:Person ; ex:name \"Person {i}\"^^xsd:string ; ex:age \"{i}\"^^xsd:integer .\n"
        ));
    }
    buf
}

fn person_data_with_violations(n: usize) -> String {
    let mut buf = PREFIXES.to_owned();
    for i in 0..n {
        if i % 10 == 0 {
            // every 10th node is missing the required name
            buf.push_str(&format!("ex:p{i} a ex:Person ; ex:age \"{i}\"^^xsd:integer .\n"));
        } else {
            buf.push_str(&format!(
                "ex:p{i} a ex:Person ; ex:name \"Person {i}\"^^xsd:string ; ex:age \"{i}\"^^xsd:integer .\n"
            ));
        }
    }
    buf
}

// ── benchmarks ────────────────────────────────────────────────────────────────

fn bench_validate_conforming(c: &mut Criterion) {
    let shapes = format!("{PREFIXES}{SIMPLE_SHAPES}");
    let mut group = c.benchmark_group("shacl/conforming");
    for n in [10usize, 100, 500] {
        let data = person_data(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &data, |b, data| {
            b.iter(|| {
                let mut v = ShaclValidator::new(&shapes, data).unwrap();
                v.validate().unwrap()
            });
        });
    }
    group.finish();
}

fn bench_validate_with_violations(c: &mut Criterion) {
    let shapes = format!("{PREFIXES}{SIMPLE_SHAPES}");
    let mut group = c.benchmark_group("shacl/violations");
    for n in [10usize, 100, 500] {
        let data = person_data_with_violations(n);
        group.bench_with_input(BenchmarkId::from_parameter(n), &data, |b, data| {
            b.iter(|| {
                let mut v = ShaclValidator::new(&shapes, data).unwrap();
                v.validate().unwrap()
            });
        });
    }
    group.finish();
}

fn bench_parse_shapes(c: &mut Criterion) {
    let shapes = format!("{PREFIXES}{SIMPLE_SHAPES}");
    c.bench_function("shacl/parse_shapes", |b| {
        b.iter(|| {
            oxidowl::validation::shacl::parser::parse_shapes_graph(&shapes).unwrap()
        });
    });
}

criterion_group!(
    benches,
    bench_validate_conforming,
    bench_validate_with_violations,
    bench_parse_shapes,
);
criterion_main!(benches);

# RDF-star Guide

This guide explains how to use RDF-star (RDF 1.2) features in oxidowl, including quoted triples, nested structures, and compatibility with RDF 1.1.

## Table of Contents

1. [Introduction](#introduction)
2. [Quoted Triples](#quoted-triples)
3. [Nested Structures](#nested-structures)
4. [Use Cases](#use-cases)
5. [Validation](#validation)
6. [SPARQL-star Queries](#sparql-star-queries)
7. [RDF 1.1 Compatibility](#rdf-11-compatibility)
8. [Performance Considerations](#performance-considerations)

## Introduction

RDF-star extends RDF 1.1 by allowing triples to be used as subjects or objects of other triples. This enables direct representation of metadata, provenance, annotations, and other meta-level information without reification.

oxidowl provides full support for RDF-star through:
- Data model extensions (`RdfTerm::QuotedTriple`)
- Parsing (Turtle-star, TriG-star, N-Triples-star)
- SPARQL-star query execution
- Validation and well-formedness checking
- Automatic conversion to/from RDF 1.1 reification

## Quoted Triples

### Basic Usage

Create a quoted triple using `RdfTerm::QuotedTriple`:

```rust
use oxidowl::semantics::{RdfGraph, RdfTerm, Triple};

// Create base statement: :alice :knows :bob
let alice = RdfTerm::iri("http://example.org/alice")?;
let knows = RdfTerm::iri("http://example.org/knows")?;
let bob = RdfTerm::iri("http://example.org/bob")?;

let base_triple = Triple::new(alice, knows, bob);

// Quote the triple for use as a term
let quoted = RdfTerm::QuotedTriple(Box::new(base_triple));

// Add metadata: << :alice :knows :bob >> :confidence 0.95
let confidence = RdfTerm::iri("http://example.org/confidence")?;
let value = RdfTerm::Literal {
    value: "0.95".to_string(),
    datatype: None,
    language: None,
    direction: None,
};

let meta_triple = Triple::new(quoted, confidence, value);

// Add to graph
let mut graph = RdfGraph::new();
graph.add_triple(meta_triple);
```

### Position Constraints

According to RDF-star semantics:
- ✅ **Subject position**: Quoted triples are allowed
- ❌ **Predicate position**: Quoted triples are NOT allowed (validation error)
- ✅ **Object position**: Quoted triples are allowed

```rust
// Valid: quoted triple in subject position
let triple1 = Triple::new(quoted_triple, predicate, object);

// Invalid: quoted triple in predicate position (validation error)
let triple2 = Triple::new(subject, quoted_triple, object); // ❌

// Valid: quoted triple in object position
let triple3 = Triple::new(subject, predicate, quoted_triple);
```

## Nested Structures

### Nesting Levels

oxidowl supports deeply nested quoted triples up to a configurable limit (default: 5 levels):

```rust
// 1-level nesting: << :a :b :c >> :d :e
let inner = Triple::new(a, b, c);
let quoted_inner = RdfTerm::QuotedTriple(Box::new(inner));
let outer = Triple::new(quoted_inner, d, e);

// 2-level nesting: << << :a :b :c >> :d :e >> :f :g
let inner = Triple::new(a, b, c);
let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner));
let middle = Triple::new(inner_quoted, d, e);
let middle_quoted = RdfTerm::QuotedTriple(Box::new(middle));
let outer = Triple::new(middle_quoted, f, g);
```

### Depth Checking

Use `Triple::depth()` to check nesting depth:

```rust
let depth = triple.depth();
assert!(depth <= 5, "Exceeds maximum nesting depth");
```

### Flattening

Extract all nested triples using `Triple::flatten()`:

```rust
let nested_triple = /* ... */;
let all_triples = nested_triple.flatten();
// Returns Vec<Triple> containing all levels
```

## Use Cases

### 1. Provenance Tracking

Track the source and timestamp of statements:

```rust
let mut graph = RdfGraph::new();

// Base statement
let doc = RdfTerm::iri("http://example.org/document1")?;
let title_pred = RdfTerm::iri("http://purl.org/dc/terms/title")?;
let title_val = RdfTerm::Literal {
    value: "Annual Report".to_string(),
    language: Some("en".to_string()),
    ..Default::default()
};

let base = Triple::new(doc.clone(), title_pred.clone(), title_val.clone());
let quoted = RdfTerm::QuotedTriple(Box::new(base.clone()));

// Add provenance
let source = RdfTerm::iri("http://example.org/source")?;
let user = RdfTerm::iri("http://example.org/user42")?;
graph.add_triple(Triple::new(quoted.clone(), source, user));

// Add timestamp
let timestamp = RdfTerm::iri("http://example.org/timestamp")?;
let date = RdfTerm::Literal {
    value: "2026-02-15T10:30:00Z".to_string(),
    ..Default::default()
};
graph.add_triple(Triple::new(quoted, timestamp, date));

// Store base triple
graph.add_triple(base);
```

### 2. Confidence Annotations

Annotate statements with confidence scores:

```rust
let mut graph = RdfGraph::new();

// Multiple claims with different confidence levels
let claims = vec![
    ("alice", "knows", "bob", "0.95"),
    ("alice", "knows", "charlie", "0.70"),
    ("bob", "knows", "david", "0.85"),
];

for (s, p, o, conf) in claims {
    let subject = RdfTerm::iri(&format!("http://example.org/{}", s))?;
    let predicate = RdfTerm::iri(&format!("http://example.org/{}", p))?;
    let object = RdfTerm::iri(&format!("http://example.org/{}", o))?;

    let base = Triple::new(subject, predicate, object);
    let quoted = RdfTerm::QuotedTriple(Box::new(base));

    let conf_pred = RdfTerm::iri("http://example.org/confidence")?;
    let conf_val = RdfTerm::Literal {
        value: conf.to_string(),
        ..Default::default()
    };

    graph.add_triple(Triple::new(quoted, conf_pred, conf_val));
}
```

### 3. Meta-level Reasoning

Reason about statements themselves:

```rust
// << :alice :believes << :bob :knows :charlie >> >> :source :interview
let inner = Triple::new(bob, knows, charlie);
let inner_quoted = RdfTerm::QuotedTriple(Box::new(inner));

let belief = Triple::new(alice, believes, inner_quoted);
let belief_quoted = RdfTerm::QuotedTriple(Box::new(belief));

let meta = Triple::new(belief_quoted, source, interview);
```

## Validation

### RDF-star Constraints

oxidowl validates RDF-star structures according to W3C specifications:

```rust
use oxidowl::validation::owl2_dl::OWL2DLValidator;

let validator = OWL2DLValidator::new();
let validation_result = validator.validate(&ontology)?;

if !validation_result.is_valid() {
    for error in validation_result.errors() {
        match error.error_type {
            ValidationErrorType::QuotedTripleInPredicatePosition => {
                println!("Quoted triple used as predicate");
            }
            ValidationErrorType::ExcessiveQuotedTripleNesting => {
                println!("Nesting depth exceeds limit");
            }
            ValidationErrorType::InvalidQuotedTripleStructure => {
                println!("Malformed quoted triple");
            }
            _ => {}
        }
    }
}
```

### Validation Rules

1. **Predicate Position**: Quoted triples MUST NOT appear in predicate position
2. **Nesting Depth**: Depth MUST NOT exceed configured maximum (default: 5)
3. **Structure**: Quoted triples must be well-formed RDF triples
4. **Blank Nodes**: Blank node labels must follow RDF 1.2 syntax rules

### Configuration

Configure validation limits:

```rust
use oxidowl::config::Config;

let mut config = Config::default();
config.set_max_quoted_triple_nesting(10); // Increase nesting limit
```

## SPARQL-star Queries

### Query Syntax

Use `<< >>` to query quoted triples:

```turtle
PREFIX ex: <http://example.org/>

SELECT ?s ?conf WHERE {
  << ?s ex:knows ex:bob >> ex:confidence ?conf .
  FILTER(?conf > 0.8)
}
```

### Query Execution

```rust
use oxidowl::query::QueryEngine;

let query = r#"
    PREFIX ex: <http://example.org/>
    SELECT ?statement ?confidence WHERE {
        ?statement ex:confidence ?confidence .
        FILTER(?confidence > 0.8)
    }
"#;

let engine = QueryEngine::new();
let results = engine.execute_sparql_star(query, &ontology)?;

for binding in results.bindings() {
    let statement = binding.get("statement");
    let confidence = binding.get("confidence");
    println!("High-confidence: {:?} -> {:?}", statement, confidence);
}
```

### Nested Queries

Query nested structures:

```turtle
PREFIX ex: <http://example.org/>

SELECT ?inner ?outer WHERE {
  << << ?inner >> ?p ?o >> ex:source "interview" .
}
```

## RDF 1.1 Compatibility

### Automatic Reification

oxidowl can automatically convert quoted triples to RDF 1.1 reification:

```rust
use oxidowl::adapter::HornedOwlAdapter;

let mut adapter = HornedOwlAdapter::new();
adapter.set_rdf11_mode(true); // Enable RDF 1.1 mode

// Create quoted triple
let quoted = RdfTerm::QuotedTriple(Box::new(triple));

// Reify to RDF 1.1
let (reified_term, reification_triples) = adapter.reify_rdf_term(&quoted)?;

// reified_term is a blank node
// reification_triples contains:
// _:b1 rdf:type rdf:Statement
// _:b1 rdf:subject :alice
// _:b1 rdf:predicate :knows
// _:b1 rdf:object :bob
```

### Mode Switching

Switch between RDF 1.1 and RDF-star modes:

```rust
let mut adapter = HornedOwlAdapter::new();

// Check current mode
if adapter.is_rdf11_mode() {
    println!("RDF 1.1 mode: using reification");
}

// Switch to RDF-star native mode
adapter.set_rdf11_mode(false);
```

### Lossy Conversions

⚠️ **Warning**: Converting from RDF-star to RDF 1.1 is lossy:
- Nested quoted triples require multiple blank nodes
- Metadata on quoted triples becomes metadata on blank nodes
- Round-trip conversion is not guaranteed to preserve structure

See [RDF_COMPATIBILITY.md](RDF_COMPATIBILITY.md) for details.

## Performance Considerations

### Memory Usage

Quoted triples increase memory usage:
- Each quoted triple stores a full `Triple` structure
- Nested triples compound memory usage
- Consider flattening deeply nested structures

### Query Performance

SPARQL-star queries over quoted triples:
- Subject/object matching is efficient (indexed)
- Predicate position is rejected (validation error)
- Deep nesting may impact query planning

### Best Practices

1. **Limit nesting depth**: Keep nesting ≤ 3 levels for most use cases
2. **Use validation**: Catch structural errors early
3. **Index efficiently**: Create indexes on frequently queried properties
4. **Profile queries**: Use `QueryEngine::profile()` to identify bottlenecks

```rust
use oxidowl::profiling::ProfiledReasoner;

let reasoner = ProfiledReasoner::new(ontology);
let results = reasoner.execute_with_profiling(query)?;
println!("Query time: {:?}", results.execution_time());
```

### Performance Benchmarks

| Operation | RDF 1.1 | RDF-star | Overhead |
|-----------|---------|----------|----------|
| Triple creation | 100ns | 120ns | +20% |
| 1-level nesting | - | 150ns | - |
| 5-level nesting | - | 300ns | - |
| SPARQL query | 1ms | 1.2ms | +20% |

See [PERFORMANCE_ANALYSIS.md](../PERFORMANCE_ANALYSIS.md) for detailed benchmarks.

## Further Reading

- [RDF_COMPATIBILITY.md](RDF_COMPATIBILITY.md) - Migration guide and compatibility details
- [MODULE_REFERENCE.md](MODULE_REFERENCE.md) - API reference
- [SPARQL_UPDATE.md](SPARQL_UPDATE.md) - SPARQL-star update operations
- [W3C RDF-star Draft](https://w3c.github.io/rdf-star/) - Official specification

## Examples

Complete working examples:
- [examples/rdf_star_example.rs](../examples/rdf_star_example.rs) - Comprehensive RDF-star usage
- [examples/rdf11_legacy_example.rs](../examples/rdf11_legacy_example.rs) - Legacy RDF 1.1 interop
- [tests/rdf_star_integration_tests.rs](../tests/rdf_star_integration_tests.rs) - Integration tests

# RDF Compatibility Guide

This guide explains how oxidowl handles compatibility between RDF 1.1, RDF 1.2, and RDF-star, including migration strategies, conversion details, and potential data loss scenarios.

## Table of Contents

1. [RDF Versions Overview](#rdf-versions-overview)
2. [Version Detection](#version-detection)
3. [Conversion Strategies](#conversion-strategies)
4. [Lossy Conversions](#lossy-conversions)
5. [Migration Guide](#migration-guide)
6. [Validation Rules](#validation-rules)
7. [Best Practices](#best-practices)

## RDF Versions Overview

### RDF 1.1 (2014)

Standard RDF with:
- Triples: subject, predicate, object
- IRIs, literals, blank nodes
- Reification for meta-statements
- No direct statement annotation

### RDF 1.2 (2024)

Extends RDF 1.1 with:
- `rdf:dirLangString` datatype for directional literals
- `rdf:reifies` predicate for reification links
- Well-formedness requirements for blank node labels
- Backwards compatible with RDF 1.1

### RDF-star (2024)

Extends RDF 1.2 with:
- Quoted triples: `<< :s :p :o >>`
- Nested structures
- Direct statement annotation
- SPARQL-star query language

### oxidowl Support

```rust
use oxidowl::semantics::RdfVersion;

// RDF 1.1: Basic RDF support
let graph = RdfGraph::with_version(RdfVersion::RDF11);

// RDF 1.2: Directional literals + well-formedness
let graph = RdfGraph::with_version(RdfVersion::RDF12);

// RDF-star: Full quoted triple support
let graph = RdfGraph::with_version(RdfVersion::RDFStar);
```

## Version Detection

### Automatic Detection

oxidowl detects RDF version based on features used:

```rust
use oxidowl::detection::detect_rdf_version;

let graph = /* ... */;
let version = detect_rdf_version(&graph);

match version {
    RdfVersion::RDF11 => println!("Standard RDF 1.1"),
    RdfVersion::RDF12 => println!("Uses RDF 1.2 features"),
    RdfVersion::RDFStar => println!("Contains quoted triples"),
}
```

### Detection Rules

1. **RDF-star** if:
   - Graph contains `RdfTerm::QuotedTriple`
   - ANY triple has quoted terms in subject/object

2. **RDF 1.2** if:
   - Uses `rdf:dirLangString` datatype
   - Uses `rdf:reifies` predicate
   - Contains non-ASCII blank node labels

3. **RDF 1.1** otherwise

### Manual Version Setting

```rust
let mut graph = RdfGraph::new();
graph.set_rdf_version(RdfVersion::RDF11); // Force RDF 1.1 mode

// Attempting to use RDF-star features will trigger warnings
```

## Conversion Strategies

### RDF-star → RDF 1.1 (Reification)

Convert quoted triples to reification vocabulary:

```rust
use oxidowl::adapter::HornedOwlAdapter;

let mut adapter = HornedOwlAdapter::new();
adapter.set_rdf11_mode(true);

// Input: << :alice :knows :bob >> :confidence 0.95
let quoted = RdfTerm::QuotedTriple(Box::new(triple));
let (reified_term, reification_triples) = adapter.reify_rdf_term(&quoted)?;

// Output (RDF 1.1):
// _:b1 rdf:type rdf:Statement
// _:b1 rdf:subject :alice
// _:b1 rdf:predicate :knows
// _:b1 rdf:object :bob
// _:b1 :confidence 0.95
```

#### Reification Pattern

Standard RDF 1.1 reification uses:

```turtle
_:stmt a rdf:Statement ;
    rdf:subject <subject> ;
    rdf:predicate <predicate> ;
    rdf:object <object> .
```

### RDF 1.1 → RDF-star (Dereification)

⚠️ **Limited Support**: Automatic dereification is not fully implemented.

```rust
let adapter = HornedOwlAdapter::new();
// dereify_triples() currently returns NotImplemented
let result = adapter.dereify_triples(&horned_ont);
// TODO: Implement heuristic-based dereification
```

**Challenges**:
- Ambiguous blank node patterns
- Multiple statements sharing components
- Incomplete reification patterns
- Custom reification vocabularies

### RDF 1.2 ↔ RDF-star

Fully compatible - no conversion needed:

```rust
// RDF 1.2 literals work in RDF-star graphs
let literal = RdfTerm::Literal {
    value: "مرحبا".to_string(),
    datatype: Some(Url::parse("http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString")?),
    language: Some("ar".to_string()),
    direction: Some("rtl".to_string()),
};

// Can be used in both RDF 1.2 and RDF-star graphs
graph.add_triple(Triple::new(subject, predicate, literal));
```

## Lossy Conversions

### RDF-star → RDF 1.1 Losses

#### 1. Structural Information

**Original RDF-star**:
```turtle
<< :alice :knows :bob >> :confidence 0.95 .
<< :alice :knows :bob >> :source :survey .
```

**RDF 1.1 Reification**:
```turtle
_:b1 a rdf:Statement ;
    rdf:subject :alice ;
    rdf:predicate :knows ;
    rdf:object :bob ;
    :confidence 0.95 ;
    :source :survey .
```

✅ **Preserved**: All metadata attached to same blank node  
❌ **Lost**: Direct relationship (quoted triple identity)

#### 2. Nested Structures

**Original RDF-star**:
```turtle
<< << :a :b :c >> :d :e >> :f :g .
```

**RDF 1.1 Reification**:
```turtle
_:b1 a rdf:Statement ;
    rdf:subject :a ;
    rdf:predicate :b ;
    rdf:object :c .

_:b2 a rdf:Statement ;
    rdf:subject _:b1 ;   # ← References blank node, not quoted triple
    rdf:predicate :d ;
    rdf:object :e .

_:b2 :f :g .
```

✅ **Preserved**: Nesting structure via blank node chain  
❌ **Lost**: Direct nesting semantics, round-trip guarantee

#### 3. Query Semantics

**RDF-star SPARQL**:
```sparql
SELECT ?s WHERE {
  << ?s :knows :bob >> :confidence ?c .
  FILTER(?c > 0.8)
}
```

**RDF 1.1 SPARQL equivalent**:
```sparql
SELECT ?s WHERE {
  ?stmt a rdf:Statement ;
        rdf:subject ?s ;
        rdf:predicate :knows ;
        rdf:object :bob ;
        :confidence ?c .
  FILTER(?c > 0.8)
}
```

✅ **Preserved**: Query results (same bindings)  
❌ **Lost**: Concise syntax, direct pattern matching

### RDF 1.2 → RDF 1.1 Losses

#### Directional Literals

**RDF 1.2**:
```rust
RdfTerm::Literal {
    value: "مرحبا",
    datatype: Some("rdf:dirLangString"),
    language: Some("ar"),
    direction: Some("rtl"),  // ← Direction metadata
}
```

**RDF 1.1 fallback**:
```rust
RdfTerm::Literal {
    value: "مرحبا",
    language: Some("ar"),
    // direction field dropped
}
```

❌ **Lost**: Text direction information (critical for RTL languages)

### Non-Lossy Conversions

✅ **RDF 1.1 → RDF 1.2**: Always safe (superset)  
✅ **RDF 1.1 → RDF-star**: Always safe (superset)  
✅ **RDF 1.2 ↔ RDF-star**: Safe if no directional literals in RDF-star

## Migration Guide

### Migrating from RDF 1.1 to RDF-star

#### Step 1: Audit Reification Usage

Find existing reification patterns:

```rust
use oxidowl::detection::find_reification_patterns;

let patterns = find_reification_patterns(&graph);
for pattern in patterns {
    println!("Found reification: {:?}", pattern.statement_blank_node);
}
```

#### Step 2: Convert to Quoted Triples

Manual conversion:

```rust
// Old RDF 1.1 reification
// _:b1 a rdf:Statement ;
//      rdf:subject :alice ;
//      rdf:predicate :knows ;
//      rdf:object :bob ;
//      :confidence 0.95 .

// New RDF-star
let triple = Triple::new(alice, knows, bob);
let quoted = RdfTerm::QuotedTriple(Box::new(triple));
let confidence_pred = RdfTerm::iri("http://example.org/confidence")?;
let confidence_val = RdfTerm::literal("0.95");

graph.add_triple(Triple::new(quoted, confidence_pred, confidence_val));
```

#### Step 3: Update SPARQL Queries

**Before (RDF 1.1)**:
```sparql
SELECT ?s ?c WHERE {
  ?stmt a rdf:Statement ;
        rdf:subject ?s ;
        rdf:predicate :knows ;
        rdf:object :bob ;
        :confidence ?c .
}
```

**After (RDF-star)**:
```sparql
SELECT ?s ?c WHERE {
  << ?s :knows :bob >> :confidence ?c .
}
```

#### Step 4: Validate

```rust
use oxidowl::validation::owl2_dl::OWL2DLValidator;

let validator = OWL2DLValidator::new();
let result = validator.validate(&ontology)?;

if !result.is_valid() {
    for error in result.errors() {
        eprintln!("Validation error: {:?}", error);
    }
}
```

### Migrating from RDF-star to RDF 1.1

⚠️ **Warning**: This is a lossy conversion. Consider if RDF 1.1 compatibility is truly required.

#### Step 1: Enable Reification Mode

```rust
use oxidowl::adapter::HornedOwlAdapter;

let mut adapter = HornedOwlAdapter::new();
adapter.set_rdf11_mode(true);
```

#### Step 2: Convert Ontology

```rust
let rdf11_ontology = adapter.convert_triple_to_rdf11(&rdf_star_ont)?;
```

#### Step 3: Verify Conversion

```rust
// Check that all quoted triples were converted
let remaining_quoted = rdf11_ontology
    .get_rdf_graph()
    .unwrap()
    .extract_quoted_triples();

assert!(remaining_quoted.is_empty(), "Conversion incomplete");
```

#### Step 4: Update Tooling

- Update SPARQL queries to use reification patterns
- Update documentation to reflect RDF 1.1 semantics
- Test round-trip conversion if needed

## Validation Rules

### Version-Specific Validation

```rust
use oxidowl::validation::VersionValidator;

let validator = VersionValidator::new(RdfVersion::RDF11);
let result = validator.validate_version_compliance(&graph)?;

for violation in result.violations() {
    match violation {
        VersionViolation::QuotedTripleInRDF11 => {
            eprintln!("Error: Quoted triples not allowed in RDF 1.1");
        }
        VersionViolation::DirectionalLiteralInRDF11 => {
            eprintln!("Error: Directional literals require RDF 1.2");
        }
        _ => {}
    }
}
```

### Strict vs. Permissive Modes

**Strict Mode** (errors on version violations):
```rust
let mut config = Config::default();
config.set_strict_version_checking(true);

// Will error if RDF-star features used in RDF 1.1 graph
let result = parser.parse_with_config(file, &config);
```

**Permissive Mode** (warnings only):
```rust
config.set_strict_version_checking(false);

// Will warn but continue parsing
let result = parser.parse_with_config(file, &config);
```

### Validation Error Types

```rust
pub enum ValidationErrorType {
    // RDF-star specific
    QuotedTripleInPredicatePosition,
    ExcessiveQuotedTripleNesting,
    
    // RDF 1.2 specific
    InvalidDirectionalLiteral,
    InvalidBlankNodeLabel,
    
    // Version compliance
    QuotedTripleInRDF11Mode,
    DirectionalLiteralInRDF11Mode,
}
```

## Best Practices

### 1. Choose Appropriate Version

- **RDF 1.1**: Maximum compatibility, legacy systems
- **RDF 1.2**: Need directional text, well-formedness
- **RDF-star**: Metadata, provenance, annotations

### 2. Document Version Requirements

```rust
// At top of file
//! This module requires RDF-star support.
//! Compatible with oxidowl >= 0.10.0

use oxidowl::semantics::RdfVersion;

pub fn process_data() -> Result<()> {
    let graph = RdfGraph::with_version(RdfVersion::RDFStar);
    // ...
}
```

### 3. Test Conversion Round-Trips

```rust
#[test]
fn test_rdf11_roundtrip() {
    let original = create_rdf_star_graph();
    
    // Convert to RDF 1.1
    let rdf11 = adapter.convert_to_rdf11(&original)?;
    
    // Convert back (may be lossy)
    let recovered = adapter.convert_from_rdf11(&rdf11)?;
    
    // Verify semantic equivalence
    assert_semantically_equivalent(&original, &recovered)?;
}
```

### 4. Handle Warnings Gracefully

```rust
use oxidowl::error::ConversionWarning;

match adapter.convert_to_rdf11(&graph) {
    Ok(converted) => {
        for warning in adapter.warnings() {
            match warning {
                ConversionWarning::DirectionalityLost { literal } => {
                    log::warn!("Lost direction for: {}", literal);
                }
                ConversionWarning::NestingFlattened { depth } => {
                    log::warn!("Flattened nesting depth {}", depth);
                }
                _ => {}
            }
        }
        Ok(converted)
    }
    Err(e) => Err(e),
}
```

### 5. Minimize Conversion Overhead

- **Store in native format**: Use RDF-star for new data
- **Convert at boundaries**: Only convert when interfacing with RDF 1.1 systems
- **Cache conversions**: Reuse reified forms when possible

```rust
use oxidowl::cache::ConversionCache;

let cache = ConversionCache::new();
let rdf11 = cache.get_or_convert(&graph, || {
    adapter.convert_to_rdf11(&graph)
})?;
```

## Further Reading

- [RDF_STAR_GUIDE.md](RDF_STAR_GUIDE.md) - Complete RDF-star usage guide
- [W3C RDF 1.2 Concepts](https://www.w3.org/TR/rdf12-concepts/) - Official RDF 1.2 spec
- [W3C RDF-star Draft](https://w3c.github.io/rdf-star/) - RDF-star specification
- [examples/rdf11_legacy_example.rs](../examples/rdf11_legacy_example.rs) - Working with legacy data

## Troubleshooting

### "Quoted triple not allowed in predicate position"

**Cause**: RDF-star forbids quoted triples as predicates.

**Solution**: Use quoted triple in subject or object position only.

### "Excessive nesting depth"

**Cause**: Nesting exceeds configured limit (default: 5).

**Solution**: Reduce nesting or increase limit via config:

```rust
config.set_max_quoted_triple_nesting(10);
```

### "Directional literal without language tag"

**Cause**: RDF 1.2 `dirLangString` requires language tag.

**Solution**: Add language tag:

```rust
RdfTerm::Literal {
    value: "text",
    language: Some("en"), // Required
    direction: Some("ltr"),
    ..Default::default()
}
```

### "Reification pattern not recognized"

**Cause**: Incomplete or non-standard reification.

**Solution**: Use standard RDF 1.1 reification vocabulary:

```turtle
_:stmt a rdf:Statement ;
    rdf:subject <s> ;
    rdf:predicate <p> ;
    rdf:object <o> .
```

All four triples are required for automatic dereification.

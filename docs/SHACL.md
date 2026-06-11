# SHACL Support in Oxidowl

Oxidowl implements the full [W3C SHACL specification][shacl-spec] — both **SHACL Core** and **SHACL-SPARQL** — backed by the embedded [Oxigraph][oxigraph] SPARQL store.

[shacl-spec]: https://www.w3.org/TR/shacl/
[oxigraph]: https://github.com/oxigraph/oxigraph

---

## Quick start

```rust
use oxidowl::validation::shacl::ShaclValidator;

let shapes_ttl = r#"
  @prefix sh:  <http://www.w3.org/ns/shacl#> .
  @prefix ex:  <http://example.org/> .
  @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .

  ex:PersonShape a sh:NodeShape ;
    sh:targetClass ex:Person ;
    sh:property [
      sh:path     ex:name ;
      sh:datatype xsd:string ;
      sh:minCount 1 ;
    ] .
"#;

let data_ttl = r#"
  @prefix ex: <http://example.org/> .
  ex:Alice a ex:Person ; ex:name "Alice" .
  ex:Bob   a ex:Person .               # missing ex:name — violation
"#;

let mut validator = ShaclValidator::new(shapes_ttl, data_ttl)?;
let report = validator.validate()?;

println!("conforms: {}", report.conforms);
for result in &report.results {
    println!("  violation: {:?}", result.focus_node);
}
```

---

## Architecture

```
src/validation/shacl/
├── mod.rs              — module root + re-exports
├── vocabulary.rs       — all SHACL / RDF / XSD IRI constants
├── model.rs            — ShaclShape, ShaclConstraint, ShaclPath, …
├── report.rs           — ShaclValidationReport + Turtle serialisation
├── parser.rs           — shapes-graph parser (Turtle → internal model)
├── paths.rs            — ShaclPath → SPARQL property path + value traversal
├── targets.rs          — target resolution (all 5 W3C target types)
├── engine.rs           — ShaclValidator orchestrator
├── sparql_constraints.rs   — sh:sparql SELECT constraint evaluator
├── sparql_components.rs    — custom SPARQL constraint component evaluator
└── constraints/
    ├── mod.rs
    ├── value_type.rs       — sh:class, sh:datatype, sh:nodeKind
    ├── cardinality.rs      — sh:minCount, sh:maxCount
    ├── value_range.rs      — sh:minExclusive/Inclusive, sh:maxExclusive/Inclusive
    ├── string_based.rs     — sh:minLength, sh:maxLength, sh:pattern, sh:languageIn, sh:uniqueLang
    ├── property_pair.rs    — sh:equals, sh:disjoint, sh:lessThan, sh:lessThanOrEquals
    ├── logical.rs          — sh:not, sh:and, sh:or, sh:xone
    ├── shape_based.rs      — sh:node, sh:qualifiedValueShape
    ├── other.rs            — sh:closed, sh:hasValue, sh:in
    └── literal_compare.rs  — cross-type term comparison (numeric, date, boolean, IRI)
```

---

## Supported constraints

### SHACL Core

| Category | Constraint |
|----------|------------|
| Value type | `sh:class`, `sh:datatype`, `sh:nodeKind` |
| Cardinality | `sh:minCount`, `sh:maxCount` |
| Value range | `sh:minExclusive`, `sh:minInclusive`, `sh:maxExclusive`, `sh:maxInclusive` |
| String-based | `sh:minLength`, `sh:maxLength`, `sh:pattern` (with `sh:flags`), `sh:languageIn`, `sh:uniqueLang` |
| Property pair | `sh:equals`, `sh:disjoint`, `sh:lessThan`, `sh:lessThanOrEquals` |
| Logical | `sh:not`, `sh:and`, `sh:or`, `sh:xone` |
| Shape-based | `sh:node`, `sh:qualifiedValueShape` (with `sh:qualifiedMinCount` / `sh:qualifiedMaxCount`) |
| Other | `sh:closed` (with `sh:ignoredProperties`), `sh:hasValue`, `sh:in` |

### SHACL-SPARQL

| Feature | Description |
|---------|-------------|
| `sh:sparql` | Arbitrary SELECT query; `$this` bound to focus node |
| SPARQL constraint components | Custom reusable constraint components via `sh:ask` / `sh:select` |

### Targets

| Target type | SHACL predicate |
|-------------|-----------------|
| Target class | `sh:targetClass` |
| Target node | `sh:targetNode` |
| Target subjects of | `sh:targetSubjectsOf` |
| Target objects of | `sh:targetObjectsOf` |
| Implicit class target | `rdfs:Class` + `owl:Class` |

### Paths

All seven SHACL path types are supported:

| Path | Syntax |
|------|--------|
| Predicate path | `sh:path ex:p` |
| Inverse path | `sh:path [ sh:inversePath ex:p ]` |
| Sequence path | `sh:path ( ex:a ex:b )` |
| Alternative path | `sh:path [ sh:alternativePath (ex:a ex:b) ]` |
| Zero-or-more | `sh:path [ sh:zeroOrMorePath ex:p ]` |
| One-or-more | `sh:path [ sh:oneOrMorePath ex:p ]` |
| Zero-or-one | `sh:path [ sh:zeroOrOnePath ex:p ]` |

---

## Validation report

`ShaclValidationReport` implements `serde::Serialize` / `serde::Deserialize` and can be serialised directly to JSON:

```rust
let json = serde_json::to_string_pretty(&report)?;
```

A Turtle serialisation is also available:

```rust
let turtle = report.to_turtle();
```

Fields on each `ShaclValidationResult`:

| Field | Type | Description |
|-------|------|-------------|
| `focus_node` | `RdfTerm` | The node that failed |
| `result_path` | `Option<String>` | The property path (if applicable) |
| `value` | `Option<RdfTerm>` | The offending value |
| `source_shape` | `Option<RdfTerm>` | IRI / blank node of the shape |
| `source_constraint_component` | `Option<String>` | e.g. `sh:MinCountConstraintComponent` |
| `severity` | `ShaclSeverity` | `Violation` / `Warning` / `Info` |
| `messages` | `Vec<ShaclMessage>` | Human-readable messages (incl. `sh:message`) |
| `details` | `HashMap<String, String>` | Extra context |

---

## Configuration

```rust
use oxidowl::validation::shacl::engine::{ShaclConfig, ShaclValidator};

let config = ShaclConfig {
    max_recursion_depth: 50,   // guard against cyclic shapes
    use_entailment:      false, // RDFS entailment for sh:class
    report_details:      true,  // populate ShaclValidationResult::details
    max_results:         None,  // Some(n) stops after n violations
};

let mut validator = ShaclValidator::with_config(shapes_ttl, data_ttl, config)?;
```

---

## REST API

When the `server` feature is enabled, two additional endpoints are available:

### `POST /api/v1/shacl/validate`

Request body (JSON):

```json
{
  "shapes": "<Turtle-encoded shapes graph>",
  "data":   "<Turtle-encoded data graph>"
}
```

Response:

```json
{
  "success": true,
  "data": {
    "conforms": false,
    "results": 3,
    "report": { "conforms": false, "results": [...] }
  }
}
```

---

## Running tests

```sh
# Unit + integration tests
cargo test shacl

# Benchmarks
cargo bench --bench shacl_benchmark
```

---

## Limitations and roadmap

* **RDFS entailment** (`sh:class` hierarchy traversal) uses `rdf:type/rdfs:subClassOf*` which requires the class hierarchy to be explicitly stated in the data graph.
* **SPARQL-based entailment regimes** (`sh:entailment`) are not yet implemented.
* The **W3C SHACL test suite** harness (`tests/shacl_w3c_tests.rs`) is a stub; full compliance scoring is planned.

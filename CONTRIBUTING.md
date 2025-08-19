# Contributing to Oxidowl

Thank you for your interest in contributing to Oxidowl! This document provides comprehensive guidelines for contributing to our high-performance Description Logic reasoner for OWL 2 DL ontologies.

## Table of Contents

- [Getting Started](#getting-started)
- [Development Environment](#development-environment)
- [Code Organization](#code-organization)
- [Contributing Process](#contributing-process)
- [Code Standards](#code-standards)
- [Testing Guidelines](#testing-guidelines)
- [Documentation](#documentation)
- [Performance Considerations](#performance-considerations)
- [Submitting Changes](#submitting-changes)
- [Community Guidelines](#community-guidelines)

## Getting Started

### Prerequisites

- **Rust**: Version 1.88 or higher
- **Git**: For version control
- **Basic knowledge**: Understanding of Description Logic, OWL 2 DL, and tableau algorithms is helpful but not required

### Fork and Clone

1. Fork the repository on GitHub

2. Clone your fork locally:

```bash
git clone https://github.com/yourusername/oxidowl.git
cd oxidowl
```

3. Add the upstream repository:

```bash
git remote add upstream https://github.com/sievericcardo/oxidowl.git
```

### Initial Setup

1. Install dependencies and build the project:

```bash
cargo build
```

2. Run tests to ensure everything works:

```bash
cargo test
```

3. Run the example to verify functionality:

```bash
cargo run --example library_usage
```

## Development Environment

### Recommended Tools

- **IDE**: VS Code with rust-analyzer extension, or IntelliJ RustRover
- **Formatter**: `cargo fmt` (automatically configured)
- **Linter**: `cargo clippy` for additional lint checks
- **Testing**: `cargo test` and `cargo bench` for benchmarks

### Project Structure

```text
oxidowl/
├── src/
│   ├── core/                    # Core reasoning engine
│   │   ├── reasoner.rs         # Main reasoner interface
│   │   ├── tableau.rs          # Traditional tableau implementation
│   │   ├── hypertableau/       # Advanced hypertableau algorithms
│   │   │   ├── hyperresolution.rs
│   │   │   ├── ground_disjunction.rs
│   │   │   └── extension_table.rs
│   │   ├── blocking.rs         # Blocking strategies
│   │   ├── expansion.rs        # Expansion management
│   │   ├── completion.rs       # Completion rules
│   │   └── dependency.rs       # Dependency tracking
│   ├── ontology/               # OWL 2 DL ontology representation
│   │   ├── axioms.rs          # Axiom structures (including DisjointUnion)
│   │   ├── concepts.rs        # Class expressions and concepts
│   │   ├── properties.rs      # Object and data properties
│   │   └── individuals.rs     # ABox individuals and assertions
│   ├── parsers/               # Input format support
│   │   ├── owl_xml.rs        # OWL XML parser
│   │   ├── functional.rs     # Functional syntax parser
│   │   ├── rdf_xml.rs        # RDF/XML parser
│   │   └── turtle.rs         # Turtle format parser
│   ├── swrl/                  # SWRL (Semantic Web Rule Language) support
│   │   ├── engine.rs         # Rule execution engine
│   │   ├── interpreter.rs    # Rule interpretation
│   │   ├── parser.rs         # SWRL syntax parsing
│   │   ├── builtins.rs       # Core built-in predicates
│   │   ├── datetime_builtins.rs # Date/time built-ins
│   │   ├── regex_builtins.rs # Regular expression built-ins
│   │   ├── validation.rs     # Rule validation
│   │   └── integration.rs    # Feature integration
│   ├── reasoning.rs           # High-level reasoning coordination
│   ├── query.rs              # DL query engine with Manchester Syntax
│   ├── config.rs             # Configuration management
│   ├── cache.rs              # Caching system
│   ├── adapter.rs            # Horned-OWL integration
│   └── lib.rs                # Public API
├── tests/
│   ├── unit/                  # Unit tests
│   ├── integration/           # Integration tests
│   ├── swrl/                  # SWRL-specific tests
├── examples/                  # Usage examples
└── benches/                   # Criterion benchmarks
```

## Code Organization

### Module Guidelines

1. **Core Modules** (`src/core/`): Contains the reasoning engine implementation
   - Keep algorithms modular and well-documented
   - Maintain separation between tableau and hypertableau implementations
   - Use clear interfaces between components

2. **Ontology Modules** (`src/ontology/`): OWL 2 DL representation
   - Follow OWL 2 DL specification closely
   - Maintain compatibility with horned-owl where possible
   - Include comprehensive axiom support

3. **Parser Modules** (`src/parsers/`): Input format support
   - Leverage horned-owl integration for robust parsing
   - Handle syntax errors gracefully
   - Support all major OWL serialization formats

## Contributing Process

### 1. Choose an Issue or Feature

- Look for issues labeled `good first issue` for beginners
- Check existing issues or create a new one for discussion
- For major features, open an issue first to discuss the approach

### 2. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b enhance/your-feature-name
# or
git checkout -b fix/issue-number
```

### 3. Development Workflow

1. **Write Tests First**: Follow TDD when possible
2. **Implement Changes**: Keep commits focused and atomic
3. **Update Documentation**: Include docstrings and update relevant docs
4. **Performance Testing**: Run benchmarks for performance-critical changes

## Code Standards

### Rust Style Guidelines

1. **Follow rustfmt**: Run `cargo fmt` before committing
2. **Use clippy**: Address all `cargo clippy` warnings
3. **Naming Conventions**:
   - `snake_case` for functions, variables, modules
   - `PascalCase` for structs, enums, traits
   - `SCREAMING_SNAKE_CASE` for constants

### Code Quality

```rust
// Good: Clear, documented function
/// Checks if the given class expression is satisfiable
/// 
/// # Arguments
/// * `expression` - The class expression to check
/// 
/// # Returns
/// * `Result<bool>` - True if satisfiable, false otherwise
/// 
/// # Example
/// ```rust
/// let result = reasoner.is_satisfiable(&expression)?;
/// ```
pub fn is_satisfiable(&mut self, expression: &ClassExpression) -> Result<bool> {
    // Implementation
}

// Bad: Unclear, undocumented function
pub fn check(&mut self, expr: &ClassExpression) -> Result<bool> {
    // Implementation
}
```

### Error Handling

- Use the custom `Error` type defined in `src/error.rs`
- Provide meaningful error messages
- Use `Result<T>` for fallible operations
- Handle errors gracefully, don't panic in library code

```rust
// Good error handling
pub fn load_ontology(&mut self, path: &Path) -> Result<()> {
    let content = fs::read_to_string(path)
        .map_err(|e| Error::io(format!("Failed to read file {}: {}", path.display(), e)))?;
    
    self.parse_ontology(&content)
        .map_err(|e| Error::parsing(format!("Failed to parse ontology: {}", e)))
}
```

### Async/Await Guidelines

- Use `async`/`await` for I/O-bound operations
- Keep CPU-intensive work synchronous or use `spawn_blocking`
- Use `tokio::time::timeout` for operations that might hang

## Testing Guidelines

### Test Structure

1. **Unit Tests** (`tests/unit/`):
   - Test individual components in isolation
   - Use mocks/stubs for dependencies
   - Focus on edge cases and error conditions

2. **Integration Tests** (`tests/integration/`):
   - Test complete workflows
   - Use real ontologies (e.g., greenhouse.owl)
   - Test reasoning task interactions

3. **Performance Tests** (`tests/performance/`):
   - Benchmark critical algorithms
   - Test scalability with large ontologies
   - Monitor memory usage

### Writing Tests

```rust
// Unit test example
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ontology::{ClassExpression, Class, IRI};

    #[test]
    fn test_satisfiability_simple_class() {
        let config = ReasonerConfig::default();
        let mut reasoner = Reasoner::new(config).unwrap();
        
        // Create simple ontology
        let mut ontology = Ontology::new();
        let person_class = Class::new(IRI::new("http://example.org/Person"));
        ontology.add_class(person_class.clone());
        
        reasoner.load_ontology(ontology).unwrap();
        
        let expression = ClassExpression::Class(person_class);
        let result = reasoner.is_satisfiable(&expression).unwrap();
        
        assert!(result, "Simple class should be satisfiable");
    }

    #[tokio::test]
    async fn test_async_reasoning_service() {
        let ontology = create_test_ontology();
        let config = ReasonerConfig::default();
        let service = ReasoningService::new(ontology, config);
        
        let is_consistent = service.is_consistent().await.unwrap();
        assert!(is_consistent);
    }
}
```

### Performance Testing

- Use `cargo bench` for benchmarks
- Test with various ontology sizes
- Monitor memory allocation patterns
- Include regression tests for performance

```rust
// Benchmark example
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_classification(c: &mut Criterion) {
    let ontology = load_greenhouse_ontology();
    let config = ReasonerConfig::default();
    
    c.bench_function("greenhouse_classification", |b| {
        b.iter(|| {
            let mut reasoner = Reasoner::new(config.clone()).unwrap();
            reasoner.load_ontology(ontology.clone()).unwrap();
            reasoner.classify().unwrap()
        })
    });
}

criterion_group!(benches, bench_classification);
criterion_main!(benches);
```

## Documentation

### Code Documentation

- **Public APIs**: Must have comprehensive rustdoc comments
- **Examples**: Include usage examples in docstrings
- **Error Cases**: Document when functions can fail
- **Performance Notes**: Document algorithmic complexity where relevant

### User Documentation

- Update README.md for user-facing changes
- Add examples to `examples/` directory
- Update architectural documentation for structural changes

### Documentation Style

```rust
/// Performs ontology classification using hypertableau algorithms
/// 
/// Classification computes the complete class hierarchy by checking all
/// possible subsumption relationships between named classes in the ontology.
/// 
/// # Performance
/// - Time complexity: O(n²) where n is the number of classes
/// - Memory complexity: O(n²) for storing the hierarchy
/// 
/// # Examples
/// ```rust
/// let mut reasoner = Reasoner::new(config)?;
/// reasoner.load_ontology_from_file("pizza.owl", OntologyFormat::OwlXml)?;
/// 
/// let classification = reasoner.classify()?;
/// println!("Found {} subsumptions", classification.subsumptions.len());
/// ```
/// 
/// # Errors
/// - Returns `Error::Reasoning` if the ontology is inconsistent
/// - Returns `Error::Timeout` if classification exceeds configured timeout
pub fn classify(&mut self) -> Result<ClassificationResult> {
    // Implementation
}
```

## Performance Considerations

### Critical Performance Areas

1. **Tableau Algorithm**: Core reasoning performance
2. **Memory Management**: Efficient data structure usage
3. **Parallel Processing**: Utilize multiple cores effectively
4. **Caching**: Avoid redundant computations

### Performance Guidelines

- **Profile First**: Use profiling tools to identify bottlenecks
- **Benchmark Changes**: Measure performance impact of modifications
- **Memory Efficiency**: Prefer stack allocation, minimize allocations
- **Parallel Safety**: Ensure thread-safe implementations

```rust
// Performance-conscious implementation
impl TableauNode {
    // Use SmallVec for small collections to avoid heap allocation
    concepts: SmallVec<[ConceptLabel; 8]>,
    
    // Use interning for frequently used strings
    individual_name: InternedString,
}

// Parallelize independent operations
use rayon::prelude::*;

let results: Vec<_> = classes
    .par_iter()
    .map(|class| self.check_satisfiability(class))
    .collect();
```

## Submitting Changes

### Pre-submission Checklist

- [ ] All tests pass: `cargo test`
- [ ] Code is formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy`
- [ ] Documentation updated
- [ ] Performance impact assessed
- [ ] Changelog entry added (for significant changes)

### Pull Request Process

1. **Create Descriptive PR**:
   - Clear title summarizing the change
   - Detailed description of what and why
   - Reference related issues

2. **PR Template**:

```markdown
## Summary
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Performance improvement
- [ ] Documentation update
- [ ] Breaking change

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Performance benchmarks run

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
```

3. **Review Process**:
   - Address reviewer feedback promptly
   - Keep discussions constructive and focused
   - Be open to suggestions and alternatives

### Commit Guidelines

- Use conventional commit format:
  - `feat:` for new features
  - `enhance:` for enhancements
  - `fix:` for bug fixes
  - `docs:` or `doc:` for documentation
  - `perf:` for performance improvements
  - `refactor:` for code refactoring
  - `test:` for testing changes

```bash
# Good commit messages
git commit -m "feat: add support for SWRL rules in hypertableau"
git commit -m "fix: resolve memory leak in tableau node cleanup"
git commit -m "perf: optimize class hierarchy computation"

# Bad commit messages
git commit -m "fix bug"
git commit -m "update code"
```

## Community Guidelines

### Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help newcomers learn the codebase
- Maintain professional communication

### Getting Help

- **Issues**: For bug reports and feature requests
- **Discussions**: For questions and general discussion
- **Documentation**: Check existing docs first
- **Code Review**: Learn from feedback and reviews

### Areas for Contribution

#### Beginner-Friendly

- Documentation improvements
- Example applications
- Unit test coverage
- Bug fixes with clear reproduction steps

#### Intermediate

- Parser enhancements
- Performance optimizations
- Additional reasoning tasks
- Caching improvements
- **SWRL Built-in Development**: Implement new built-in predicates
- **DL Query Extensions**: Advanced Manchester Syntax features

#### Advanced

- Hypertableau algorithm extensions
- Parallel reasoning strategies
- New Description Logic features
- Novel optimization techniques
- **SWRL Extensions**: Advanced rule strategies, new built-in predicates, temporal reasoning
- **Integration Improvements**: Enhanced horned-owl integration, performance optimizations

### Roadmap Areas

Current priority areas for contributions:

1. **OWL 2 RL Profile Support**: Implement rule-based reasoning
2. **Incremental Classification**: Optimize for dynamic ontologies
3. **Distributed Reasoning**: Scale across multiple nodes
4. **WebAssembly Compilation**: Enable web deployment
5. **Python Bindings**: Expand language support
6. **SWRL Enhancements**:
   - Backward chaining improvements
   - Additional built-in predicates
   - Temporal reasoning extensions
   - Performance optimizations for large rule sets
7. **Integration Improvements**:
   - Enhanced horned-owl integration
   - Additional ontology format support
   - Streaming processing capabilities

## SWRL Development Guidelines

### SWRL Module Structure

The SWRL implementation is modular and extensible:

- **Engine** (`engine.rs`): Core rule execution with forward/backward/hybrid chaining
- **Interpreter** (`interpreter.rs`): Individual rule interpretation and variable binding
- **Parser** (`parser.rs`): SWRL syntax parsing with namespace support
- **Built-ins**: Organized by category with consistent interfaces
- **Validation** (`validation.rs`): Rule validation and error reporting
- **Integration** (`integration.rs`): Unified interface for all SWRL features

### Adding New Built-in Predicates

1. **Choose the appropriate module**:
   - `builtins.rs` - Core mathematical and logical built-ins
   - `string_builtins.rs` - String manipulation predicates
   - `datetime_builtins.rs` - Date/time operations
   - `regex_builtins.rs` - Regular expression operations
   - `additional_builtins.rs` - Missing standard built-ins

1. **Implement the `SWRLBuiltIn` trait**:

```rust
pub struct MyBuiltIn;

impl SWRLBuiltIn for MyBuiltIn {
    fn execute(&self, args: &[SWRLValue]) -> Result<SWRLValue> {
        // Validate arguments
        if args.len() != 2 {
            return Err(Error::reasoning("Expected 2 arguments"));
        }
        
        // Implement logic
        // ...
        
        Ok(result)
    }
    
    fn name(&self) -> &str {
        "http://www.w3.org/2003/11/swrlb#myBuiltIn"
    }
    
    fn arity(&self) -> Option<usize> {
        Some(2) // Fixed arity, or None for variable arity
    }
}
```

1. **Register the built-in**:

```rust
registry.register_builtin(
    IRI::new("http://www.w3.org/2003/11/swrlb#myBuiltIn"),
    Box::new(MyBuiltIn),
);
```

1. **Add comprehensive tests**:

```rust
#[test]
fn test_my_builtin() {
    let builtin = MyBuiltIn;
    let args = vec![
        SWRLValue::String("test".to_string()),
        SWRLValue::Integer(42),
    ];
    let result = builtin.execute(&args).unwrap();
    assert_eq!(result, SWRLValue::Boolean(true));
}
```

### SWRL Testing Guidelines

1. **Unit Tests**: Test individual built-ins and components
2. **Integration Tests**: Test rule execution and reasoning integration
3. **Performance Tests**: Test with large rule sets and complex ontologies
4. **Error Handling**: Test invalid rules, argument types, and edge cases

### SWRL Performance Considerations

- **Built-in Caching**: Cache expensive operations (regex compilation, etc.)
- **Variable Binding Optimization**: Minimize binding combinations
- **Rule Ordering**: Implement priority-based rule execution
- **Inference Deduplication**: Avoid generating duplicate inferences
- **Memory Management**: Use efficient data structures for large rule sets

## Resources


### Learning Resources

- [OWL 2 DL Specification](https://www.w3.org/TR/owl2-syntax/)
- [SWRL: A Semantic Web Rule Language](https://www.w3.org/Submission/SWRL/)
- [SWRL Built-ins](https://www.w3.org/Submission/SWRL/#8) - Official built-in predicates
- [Description Logic Handbook](
    https://doi.org/10.1017/CBO9780511711787)
- [HermiT Reasoner](https://www.hermit-reasoner.com/)
- [Horned-OWL Documentation](https://docs.rs/horned-owl/)

### Development Tools

- [Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [Criterion Benchmarking](https://docs.rs/criterion/)

Thank you for contributing to Oxidowl! Your contributions help advance the state of Description Logic reasoning in the Rust ecosystem.

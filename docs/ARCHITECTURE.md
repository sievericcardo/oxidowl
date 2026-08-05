# Architecture

## Core Components

- **`core`** - Core reasoning engine with tableau algorithms
  - `reasoner/` - Main reasoner interface
    - `parallel_tableau.rs` — `ParallelTableauExpander` (Rayon-based parallel node expansion)
  - `tableau/` - Tableau expansion with node and edge management
  - `blocking.rs` - Anywhere blocking with cycle detection
  - `completion.rs` - Completion rules and caching
  - `incremental.rs` - Dependency-tracked incremental classification
  - `hypergraph/` - Structural-sharing hypergraph for Hypertableau
  - `saturation/` - Rule-saturation engine for RL/EL profiles
    - `cycle_detection.rs` — `CycleDetector` (DashMap + atomic counters)
  - `inverted_index.rs` - DashMap-based unified inverted index with atomic stats counter for fast concept lookups
  - `persistent_collections.rs` - Immutable persistent concept sets with hash-based deduplication pool

- **`ontology`** - Ontology representation and management (built on horned-owl)
  - `axioms.rs` - Axiom structures and operations including DisjointUnion
  - `concepts.rs` - Class expressions and concepts
  - `properties.rs` - Object and data properties
  - `individuals.rs` - ABox individuals and assertions

- **`parsers`** - Input format support via horned-owl integration
  - `owl_xml.rs` - OWL XML parser with DisjointUnion support
  - `functional.rs` - Functional syntax parser with configurable error verbosity
  - `rdf_xml.rs` - RDF/XML parser
  - `turtle.rs` - Turtle format parser

- **`reasoning`** - High-level reasoning coordination
  - `preprocessing/` - Advanced preprocessing pipeline
    - `absorption.rs` — Triggered-implication absorber
    - `common_disjunct.rs` — Common-disjunct extractor
    - `disjunct_sorting.rs` — Disjunct sorter
    - `role_automata.rs` — Role automaton builder
    - `nominal_schema.rs` — Nominal-schema processor
  - `cache/` - Multi-level tableau caching layer
    - `unsat_cache.rs` — Unsatisfiability result cache
    - `sat_expander_cache.rs` — SAT expander node cache
    - `completion_graph_cache.rs` — Completion graph cache
    - `saturation_cache.rs` — Saturation result cache
    - `consequences_cache.rs` — Consequence cache
  - `datatypes/` - XSD datatype value-space handlers
    - `boolean.rs`, `string.rs`, `numeric.rs`, `datetime.rs`, `iri.rs`

- **`query`** - Query engines
  - `dl_query.rs` - DL query engine with Manchester Syntax and union query support
  - `sparql_store.rs` - In-process Oxigraph SPARQL store wrapper
  - `advanced/` - ML-enhanced conjunctive query execution engine

- **`profiles`** - OWL 2 sub-profile support
  - `el_reasoner.rs` - OWL 2 EL polynomial-time reasoner with O(1) indexed completion rules, queue deduplication, and concurrent classification
  - `ql.rs` - OWL 2 QL profile validator
  - `rl.rs` - OWL 2 RL profile validator
  - `rl_reasoner.rs` - OWL 2 RL forward-chaining materialisation engine
  - `dl.rs` - OWL 2 DL validator
  - `validator.rs` - Unified profile validation interface

- **`validation`** - Structural validation
  - `owl2_dl.rs` - OWL 2 DL validator
  - `shacl/` - Full SHACL Core + SHACL-SPARQL engine (10+ constraint categories)

- **`swrl`** - SWRL (Semantic Web Rule Language) implementation
  - `engine.rs` - SWRL rule execution engine with multiple strategies
  - `interpreter.rs` - Individual rule interpretation and execution
  - `parser.rs` - SWRL syntax parsing
  - `builtins.rs` - Core built-in predicates (math, string, boolean)
  - `datetime_builtins.rs` - Date/time built-in predicates (15+ predicates)
  - `regex_builtins.rs` - Regular expression built-in predicates
  - `validation.rs` - SWRL rule validation

- **`distributed`** - Cluster-based horizontal scaling
  - `cluster.rs` - Node discovery, health monitoring, and lifecycle
  - `query_distribution.rs` - Intelligent query splitting and distribution
  - `result_aggregation.rs` - Parallel result collection and merging
  - `fault_tolerance.rs` - Failure detection and recovery strategies
  - `load_balancing.rs` - Dynamic workload distribution

- **`semantics`** - RDF, RDFS, and OWL 2 semantics
  - RDF-star support (quoted triples, `rdf:reifies`, directional literals)
  - RDF 1.2 compliance

- **`import`** - Import resolution
  - Recursive imports, cycle detection, and IRI-to-file mapping
  - Optional HTTP fetching (feature `http-imports`)

- **`adapter`** - Horned-OWL integration layer for enhanced parsing and modeling

- **`explanation`** - Justification and proof generation

- **`cache`** / **`cache_lockfree`** / **`cache_strategies`** - Multi-strategy caching layer
  - LRU, LFU, LRUFU, size-based, and TTL eviction policies
  - Lock-free DashMap-backed variant for highly concurrent access

- **`performance`** / **`profiling`** - Performance monitoring and flamegraph/heap profiling

- **`dl_clauses`** - DL clause generation and dumping

- **`server`** *(feature `server`)* - Web interfaces
  - `rest.rs` - REST API (port configurable)
  - `owllink.rs` - OWLlink XML protocol
  - `sparql.rs` - SPARQL HTTP endpoint

- **`proofs`** *(feature `kani`)* - Kani formal-verification harnesses

- **`config`** - Configuration management and optimisation

- **`visitor`** - Visitor pattern for ontology traversal

## Algorithms

### Tableau Algorithm

Oxidowl implements an efficient tableau algorithm that provides:

- **Systematic Expansion**: Sound and complete reasoning through node expansion
- **Optimized Rule Application**: Smart ordering and caching of tableau rules
- **Advanced Blocking**: Anywhere blocking with cycle detection for termination
- **Dependency Tracking**: Intelligent backtracking and conflict resolution
- **Memory Management**: Efficient data structures for large ontologies

### Performance Optimizations

- **EL Profile Auto-Detection**: `classify()` automatically detects EL-conforming ontologies and invokes the polynomial-time EL reasoner, bypassing full tableau expansion
- **Parallel Processing**: Multi-threaded reasoning with Rayon for large ontologies
- **Caching**: LRU/LFU/LRUFU caches with configurable eviction policies
- **Lock-Free Structures**: DashMap-backed caches for concurrent access
- **Indexed Completion Rules**: O(1) `sup_by_sub` and `sub_by_sup` indexes replace O(N) linear subsumption scans in the EL completion engine
- **Queue Deduplication**: Hash-set guards prevent duplicate inferences from causing exponential blowup in completion fixpoint iteration
- **Incremental Reasoning**: Dependency-tracked reclassification on updates
- **High-Performance Allocator**: mimalloc (feature `high-performance`)
- **Compile-Time Hashing**: Perfect hash tables (`phf`) for O(1) keyword lookup
- **IRI Intern Pool**: Global `OnceLock<DashMap>` deduplicates IRI string allocations across the ontology

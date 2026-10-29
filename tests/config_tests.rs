#[path = "helpers/mod.rs"]
mod helpers;

use helpers::df::DF;
use oxidowl::OntologyManager;
use oxidowl::PrefixManager;
use oxidowl::ontology::*;

fn onto_ref(o: Ontology) -> OntologyRef {
    std::sync::Arc::new(std::sync::RwLock::new(o))
}

// ══════════════════════════════════════════════════════════════════════════════
// PrefixManager Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn prefix_manager_well_known_owl() {
    let pm = PrefixManager::new();
    assert_eq!(
        pm.expand("owl:Thing"),
        Some("http://www.w3.org/2002/07/owl#Thing".to_string())
    );
    assert_eq!(
        pm.expand("rdf:type"),
        Some("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string())
    );
    assert_eq!(
        pm.expand("rdfs:label"),
        Some("http://www.w3.org/2000/01/rdf-schema#label".to_string())
    );
    assert_eq!(
        pm.expand("xsd:string"),
        Some("http://www.w3.org/2001/XMLSchema#string".to_string())
    );
}

#[test]
fn prefix_manager_well_known_shorten() {
    let pm = PrefixManager::new();
    assert_eq!(
        pm.shorten("http://www.w3.org/2002/07/owl#Thing"),
        Some("owl:Thing".to_string())
    );
    assert_eq!(
        pm.shorten("http://www.w3.org/1999/02/22-rdf-syntax-ns#type"),
        Some("rdf:type".to_string())
    );
}

#[test]
fn prefix_manager_custom_prefix() {
    let mut pm = PrefixManager::new();
    pm.add_prefix("ex", "http://example.org/");
    let expanded = pm.expand("ex:Test");
    assert_eq!(expanded, Some("http://example.org/Test".to_string()));
    let shortened = pm.shorten("http://example.org/Test");
    assert_eq!(shortened, Some("ex:Test".to_string()));
}

#[test]
fn prefix_manager_remove_prefix() {
    let mut pm = PrefixManager::new();
    pm.add_prefix("ex", "http://example.org/");
    assert!(pm.expand("ex:Test").is_some());
    pm.remove_prefix("ex");
    assert!(pm.expand("ex:Test").is_none());
}

#[test]
fn prefix_manager_unknown_prefix() {
    let pm = PrefixManager::new();
    assert_eq!(pm.expand("unknown:Thing"), None);
    assert_eq!(pm.shorten("http://unknown.org/thing"), None);
}

#[test]
fn prefix_manager_no_colon() {
    let pm = PrefixManager::new();
    assert_eq!(pm.expand("NoPrefixHere"), None);
}

// ══════════════════════════════════════════════════════════════════════════════
// IRI Tests
// ══════════════════════════════════════════════════════════════════════════════

#[test]
fn iri_basic_creation() {
    let iri = IRI::new("http://example.org/Class");
    assert_eq!(iri.as_str(), "http://example.org/Class");
}

#[test]
fn iri_display() {
    let iri = IRI::new("http://example.org/Class");
    assert_eq!(format!("{iri}"), "http://example.org/Class");
}

#[test]
fn iri_equality() {
    let i1 = IRI::new("http://ex.org/A");
    let i2 = IRI::new("http://ex.org/A");
    let i3 = IRI::new("http://ex.org/B");
    assert_eq!(i1, i2);
    assert_ne!(i1, i3);
}

#[test]
fn iri_owl_thing() {
    let thing = IRI::owl_thing();
    assert!(thing.is_owl_thing());
    assert!(thing.is_reserved_vocabulary());
}

#[test]
fn iri_owl_nothing() {
    let nothing = IRI::owl_nothing();
    assert!(nothing.is_owl_nothing());
    assert!(nothing.is_reserved_vocabulary());
}

// ══════════════════════════════════════════════════════════════════════════════
// Ontology Metrics Tests
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::inference::metrics::OntologyMetrics;

#[test]
fn ontology_metrics_axiom_counts() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let metrics = OntologyMetrics::compute(&onto);
    assert!(*metrics.get("NumberOfAxioms").unwrap() > 0.0);
    assert!(*metrics.get("NumberOfClasses").unwrap() >= 3.0);
    assert!(*metrics.get("NumberOfSubClassAxioms").unwrap() >= 2.0);
}

#[test]
fn ontology_metrics_empty() {
    let o = Ontology::new();
    let metrics = OntologyMetrics::compute(&o);
    assert_eq!(*metrics.get("NumberOfAxioms").unwrap(), 0.0);
}

// ══════════════════════════════════════════════════════════════════════════════
// OntologyWalker Tests
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::walk::{OWLObjectVisitor, OntologyWalker};

struct CountingVisitor {
    class_count: usize,
    axiom_count: usize,
}

impl OWLObjectVisitor for CountingVisitor {
    fn visit_class_expression(&mut self, _ce: &ClassExpression) {
        self.class_count += 1;
    }
    fn visit_axiom(&mut self, _axiom: &Axiom) {
        self.axiom_count += 1;
    }
}

#[test]
fn ontology_walker_counts() {
    let df = DF::new();
    let onto = df.simple_chain_ontology();
    let mut walker = OntologyWalker::new(CountingVisitor {
        class_count: 0,
        axiom_count: 0,
    });
    walker.walk_ontology(&onto);
    let visitor = walker.into_visitor();
    assert!(visitor.class_count > 0);
    assert!(visitor.axiom_count > 0);
}

// ══════════════════════════════════════════════════════════════════════════════
// OWLOntologyMerger Tests
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::walk::merge::OWLOntologyMerger;

#[test]
fn ontology_merger_basic() {
    let df = DF::new();
    let o1 = onto_ref(df.new_ontology_with_iri("http://ex.org/O1"));
    let o2 = onto_ref(df.new_ontology_with_iri("http://ex.org/O2"));
    let mut manager = OntologyManager::new();
    let merger = OWLOntologyMerger::new(IRI::new("http://ex.org/Merged"));
    let result = merger.merge(&[o1, o2], &mut manager);
    assert!(result.is_ok());
}

// ══════════════════════════════════════════════════════════════════════════════
// Config loading / serde-default regression tests
//
// Root cause these guard against: `src/config.rs` used to derive plain
// `Deserialize` with no `#[serde(default)]`, so ANY missing/renamed field
// (e.g. A8's `enable_absorption_classification`) made every `--config`
// invocation hard-fail at `ReasonerConfig::load_from_file` with
// `TOML parse error … missing field …` → exit 1 for the whole benchmark
// harness (incidents 2026-08-04 and 2026-10-02).  Missing fields must now
// fall back to the struct `Default`; unknown keys are ignored.
// ══════════════════════════════════════════════════════════════════════════════

use oxidowl::config::{
    BlockingStrategy, CacheConfig, CacheEvictionStrategy, ExpansionStrategy, LoggingConfig,
    OWLProfile, PerformanceConfig, PerformanceProfile, ReasonerConfig, ReasoningConfig,
    ServerConfig, TableauAlgorithm,
};

/// Debug formatting is used to compare whole config sections because the
/// config structs intentionally do not derive `PartialEq` (out of scope here).
fn dbg_of<T: std::fmt::Debug>(value: &T) -> String {
    format!("{value:?}")
}

/// Write `content` to a temp file and load it through the same code path the
/// CLI uses (`ReasonerConfig::load_from_file`, which also runs `validate()`).
fn load_toml_str(content: &str) -> ReasonerConfig {
    let file = tempfile::NamedTempFile::new().expect("create temp config file");
    std::fs::write(file.path(), content).expect("write temp config file");
    ReasonerConfig::load_from_file(file.path()).expect("config should load")
}

#[test]
fn partial_config_missing_sections_and_fields_fall_back_to_defaults() {
    // Minimal partial config: a single key in a single section.
    let cfg = load_toml_str("[reasoning]\ntableau_algorithm = \"Hypertableau\"\n");

    // The one specified value is honoured…
    assert_eq!(
        cfg.reasoning.tableau_algorithm,
        TableauAlgorithm::Hypertableau
    );

    // …every other [reasoning] field falls back to `ReasoningConfig::default()`.
    let expected_reasoning = ReasoningConfig {
        tableau_algorithm: TableauAlgorithm::Hypertableau,
        ..Default::default()
    };
    assert_eq!(dbg_of(&cfg.reasoning), dbg_of(&expected_reasoning));

    // Entirely absent sections fall back to `ReasonerConfig::default()`.
    let defaults = ReasonerConfig::default();
    assert_eq!(dbg_of(&cfg.cache), dbg_of(&defaults.cache));
    assert_eq!(dbg_of(&cfg.server), dbg_of(&defaults.server));
    assert_eq!(dbg_of(&cfg.logging), dbg_of(&defaults.logging));
    assert_eq!(dbg_of(&cfg.performance), dbg_of(&defaults.performance));

    // The defaults must still satisfy `validate()`.
    cfg.validate().expect("defaulted config must validate");
}

#[test]
fn empty_config_file_loads_full_defaults() {
    let cfg = load_toml_str("");
    assert_eq!(dbg_of(&cfg), dbg_of(&ReasonerConfig::default()));
}

/// The v0.8.x boolean-schema config (no `features` bitflags, legacy
/// `enable_*` keys) that shipped with the ORE-2015 harness.
const LEGACY_BOOLEAN_SCHEMA_CONFIG: &str = r#"[server]
enable_server = false
port = 8080
bind_address = "127.0.0.1"
max_connections = 100
request_timeout = { secs = 300, nanos = 0 }
enable_cors = false
max_request_size = 10485760
enable_owllink = false
owllink_port = 8081
enable_sparql = false
sparql_port = 8082
enable_rest_api = false
rest_api_port = 8083

[reasoning]
tableau_algorithm = "Hypertableau"
blocking_strategy = "Optimal"
expansion_strategy = "Optimal"
enable_optimisations = true
incremental_reasoning = true
enable_explanations = false
max_expansion_depth = 1000
enable_clash_detection = true
enable_blockers_cache = true
ignore_unsupported_datatypes = false
dump_clauses = false
target_profile = "DL"

[cache]
enable_satisfiability_cache = true
enable_completion_graph_cache = true
enable_unsatisfiability_cache = true
max_cache_size_mb = 1024
eviction_strategy = "LRU"
persistence = false

[logging]
level = "Info"
enable_file_logging = false
log_rotation_size_mb = 100
max_log_files = 10

[performance]
profile = "Medium"
enable_parallel_expansion = true
enable_simd = true
enable_numa_awareness = true
memory_pool_size_mb = 1024
gc_threshold = 0.75
enable_lock_free = false
enable_persistent_collections = false
"#;

#[test]
fn legacy_boolean_schema_config_loads_with_unknown_keys_ignored() {
    let cfg = load_toml_str(LEGACY_BOOLEAN_SCHEMA_CONFIG);

    // Known legacy keys still map onto current fields and are honoured.
    assert_eq!(cfg.server.port, 8080);
    assert_eq!(cfg.server.bind_address, "127.0.0.1");
    assert_eq!(cfg.server.request_timeout.as_secs(), 300);
    assert_eq!(cfg.server.max_request_size, 10_485_760);
    assert_eq!(cfg.server.owllink_port, 8081);
    assert_eq!(cfg.server.sparql_port, 8082);
    assert_eq!(cfg.server.rest_api_port, 8083);
    assert_eq!(
        cfg.reasoning.tableau_algorithm,
        TableauAlgorithm::Hypertableau
    );
    assert_eq!(cfg.reasoning.blocking_strategy, BlockingStrategy::Optimal);
    assert_eq!(
        dbg_of(&cfg.reasoning.expansion_strategy),
        dbg_of(&ExpansionStrategy::Optimal)
    );
    assert!(cfg.reasoning.incremental_reasoning);
    assert_eq!(cfg.reasoning.max_expansion_depth, 1000);
    assert_eq!(cfg.reasoning.target_profile, OWLProfile::DL);
    assert!(!cfg.reasoning.ignore_unsupported_datatypes);
    assert!(!cfg.reasoning.dump_clauses);
    assert_eq!(cfg.cache.max_cache_size_mb, 1024);
    assert_eq!(
        dbg_of(&cfg.cache.eviction_strategy),
        dbg_of(&CacheEvictionStrategy::LRU)
    );
    assert!(!cfg.cache.persistence);
    assert_eq!(
        dbg_of(&cfg.logging.level),
        dbg_of(&LoggingConfig::default().level)
    );
    assert!(!cfg.logging.enable_file_logging);
    assert_eq!(cfg.logging.log_rotation_size_mb, 100);
    assert_eq!(cfg.logging.max_log_files, 10);
    assert_eq!(cfg.performance.profile, PerformanceProfile::Medium);
    assert_eq!(cfg.performance.memory_pool_size_mb, 1024);
    assert_eq!(cfg.performance.gc_threshold, 0.75);

    // Legacy `enable_*` keys are unknown to the current schema: they are
    // ignored (no `deny_unknown_fields`), the missing `features` bitflags
    // fall back to the struct defaults, and the load still succeeds.
    assert_eq!(
        dbg_of(&cfg.server.features),
        dbg_of(&ServerConfig::default().features)
    );
    assert_eq!(
        dbg_of(&cfg.reasoning.features),
        dbg_of(&ReasoningConfig::default().features)
    );
    assert_eq!(
        dbg_of(&cfg.cache.features),
        dbg_of(&CacheConfig::default().features)
    );
    assert_eq!(
        dbg_of(&cfg.performance.features),
        dbg_of(&PerformanceConfig::default().features)
    );

    // Missing optional/current fields fall back to defaults too.
    assert_eq!(
        dbg_of(&cfg.cache.cache_ttl),
        dbg_of(&CacheConfig::default().cache_ttl)
    );
    assert_eq!(cfg.cache.persistence, CacheConfig::default().persistence);
    assert_eq!(
        cfg.performance.worker_threads,
        PerformanceConfig::default().worker_threads
    );
    assert_eq!(
        cfg.performance.max_parallel_classification_tasks,
        PerformanceConfig::default().max_parallel_classification_tasks
    );

    cfg.validate().expect("legacy config must validate");
}

#[test]
fn shipped_hypergraph_config_loads() {
    // Drift guard: the repo's own shipped configs must keep loading.  Before
    // serde defaults were added, a single new required field (A8
    // `enable_absorption_classification`) made both of these exit 1.
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));

    let shipped = manifest_dir.join("oxidowl_hypergraph_config.toml");
    let cfg = ReasonerConfig::load_from_file(&shipped)
        .unwrap_or_else(|e| panic!("shipped config {shipped:?} must load: {e}"));
    assert_eq!(
        cfg.reasoning.tableau_algorithm,
        TableauAlgorithm::Hypertableau
    );
    assert!(!cfg.reasoning.incremental_reasoning);
    assert!(cfg.reasoning.enable_absorption_classification);
    // The enforced RSS memory cap (Part 1 of the memory-cap workstream) must be
    // set explicitly so a runaway benchmark run aborts gracefully at 4 GB.
    assert_eq!(cfg.reasoning.max_memory_mb, Some(4096));
    cfg.validate().expect("shipped config must validate");

    let example = manifest_dir.join("oxidowl_hypergraph_config.toml.example");
    ReasonerConfig::load_from_file(&example)
        .unwrap_or_else(|e| panic!("example config {example:?} must load: {e}"));
}

#[test]
fn zero_max_memory_mb_is_unlimited_and_validates() {
    // `max_memory_mb = 0` is the documented "unlimited" escape hatch for the
    // enforced RSS guard (`MemoryGuard`): it must load and validate instead of
    // failing with the old "Maximum memory cannot be zero" config error.
    let cfg = load_toml_str("[reasoning]\nmax_memory_mb = 0\n");
    assert_eq!(cfg.reasoning.max_memory_mb, Some(0));
    cfg.validate().expect("0 MB must validate as unlimited");

    // An explicit positive cap validates too.
    let cfg = load_toml_str("[reasoning]\nmax_memory_mb = 4096\n");
    assert_eq!(cfg.reasoning.max_memory_mb, Some(4096));
    cfg.validate().expect("4096 MB must validate");

    // Absent key keeps the enforced default (4 GB, not unlimited).
    let cfg = load_toml_str("[reasoning]\ntableau_algorithm = \"Traditional\"\n");
    assert_eq!(cfg.reasoning.max_memory_mb, Some(4096));
}

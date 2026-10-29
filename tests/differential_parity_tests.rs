//! # Differential Parity Test Harness (Workstream C2)
//!
//! Compares oxidowl's reasoning verdicts against a curated corpus of **golden
//! outputs** that represent the ground truth `HermiT` / Konclude (OWL 2 Direct
//! Semantics) would answer.  Each fixture is a declarative JSON document:
//!
//! ```json
//! {
//!   "id": "diff_001",
//!   "source": "Functional",
//!   "ontology": "<inline functional-syntax string>",
//!   "base": "http://example.org/diff001#",
//!   "expected": { "consistent": true, "subsumptions": [["A","B"]], ... },
//!   "rationale": "…"
//! }
//! ```
//!
//! Golden values are authored from first principles (OWL 2 Direct Semantics)
//! and are meant to be cross-checked against real HermiT/Konclude binaries
//! later via the `DIFF_REASONER` interface documented in
//! `tests/fixtures/differential/README.md`.
//!
//! ## Modes
//!
//! - **Strict** (default): each `diff_NNN_*` `#[test]` FAILS on any mismatch
//!   between oxidowl's output and the golden corpus.
//! - **Report-only**: `cargo test differential_parity_tests::report_only -- --ignored --nocapture`
//!   prints a full per-fixture comparison without failing — use this while
//!   authoring fixtures.
//! - **Golden generation**: `cargo test differential_parity_tests::generate_golden_json -- --ignored --nocapture`
//!   prints oxidowl's actual outputs in the golden JSON shape, ready to be
//!   reviewed (against HermiT/Konclude or OWL 2 semantics) and promoted.
//!
//! Fixtures that exercise known production gaps (documented in the W3C
//! conformance audit / PLAN.md workstreams A1–A7) are marked `#[ignore]` with
//! a precise reason while the gap is open.  As of the C2 fix wave no fixture
//! is ignored any more (`diff_005` nominal membership and `diff_012`
//! datatype-derived `get_types` now pass in strict mode); the report-only and
//! golden-generation *mode helpers* below remain `#[ignore]` because they are
//! opt-in authoring tools, not conformance checks.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::Deserialize;
use serde_json::Value;

use oxidowl::{
    OWL2ProfileValidator, Reasoner, ReasonerConfig,
    core::reasoner::ReasoningStatistics,
    ontology::{Axiom, Class, ClassExpression, IRI, Individual, Ontology},
    parsers::{parse_file_auto, parse_functional, parse_owl_xml, parse_rdf_xml, parse_turtle},
    profiles::OWL2Profile,
};

// ─────────────────────────────────────────────────────────────────────────────
// Fixture model (JSON schema)
// ─────────────────────────────────────────────────────────────────────────────

/// Root fixture document.  `ontology` is either an inline ontology string
/// (for `source` ∈ Functional/Turtle/RDFXML/OWLXML/NTriples) or a file path
/// relative to the repository root (for `source` == "Path").
#[derive(Debug, Clone, Deserialize)]
struct Fixture {
    id: String,
    /// One of: "Functional", "Turtle", "RDFXML", "OWLXML", "`NTriples`", "Path".
    source: String,
    /// Inline ontology string, or a file path when `source` == "Path".
    ontology: String,
    /// Base IRI used to resolve short names (`:A` → `<base>A`) in the
    /// `expected` block.  Optional; defaults to `http://example.org/diff#`.
    #[serde(default)]
    base: Option<String>,
    /// Optional load sanity-check on the parsed axiom count.
    #[serde(default)]
    axioms: Option<AxiomBounds>,
    #[serde(default)]
    expected: Expected,
    /// Free-text justification of every golden verdict (required by the corpus
    /// authoring rules).
    #[serde(default)]
    rationale: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AxiomBounds {
    #[serde(default)]
    min: Option<usize>,
    #[serde(default)]
    max: Option<usize>,
}

/// The golden verdict block.  Every field is optional; only the checks that are
/// present in a fixture are executed.  Short names in these fields are resolved
/// against `base`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Expected {
    #[serde(default)]
    consistent: Option<bool>,
    /// Class token → satisfiable?
    #[serde(default)]
    satisfiable: Option<HashMap<String, bool>>,
    /// Ordered [sub, super] pairs that must be entailed.
    #[serde(default)]
    subsumptions: Option<Vec<Vec<String>>>,
    /// Ordered [sub, super] pairs that must NOT be entailed.
    #[serde(default)]
    non_subsumptions: Option<Vec<Vec<String>>>,
    /// Ordered [a, b] pairs that must be mutually subsumed.
    #[serde(default)]
    equivalences: Option<Vec<Vec<String>>>,
    /// Class token → individual tokens that must be instances (set compare).
    #[serde(default)]
    instances: Option<HashMap<String, Vec<String>>>,
    /// Individual token → class tokens that must be among its types (subset).
    #[serde(default)]
    types: Option<HashMap<String, Vec<String>>>,
    /// Functional-Syntax axiom bodies (with `:` prefix) that must be entailed.
    #[serde(default)]
    entailed: Option<Vec<String>>,
    /// Functional-Syntax axiom bodies that must NOT be entailed.
    #[serde(default)]
    not_entailed: Option<Vec<String>>,
    /// Pairs of individuals that must be owl:sameAs.
    #[serde(default)]
    same_as: Option<Vec<Vec<String>>>,
    /// Pairs of individuals that must be owl:differentFrom.
    #[serde(default)]
    different_from: Option<Vec<Vec<String>>>,
    /// Profile verdicts: keys el/ql/rl/dl/full → expected conformance.
    #[serde(default)]
    profiles: Option<HashMap<String, bool>>,
}

impl Fixture {
    fn base(&self) -> String {
        self.base
            .clone()
            .unwrap_or_else(|| "http://example.org/diff#".to_string())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Loading
// ─────────────────────────────────────────────────────────────────────────────

fn fixtures_dir() -> PathBuf {
    PathBuf::from(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/differential"
    ))
}

fn fixture_path(id: &str) -> PathBuf {
    fixtures_dir().join(format!("{id}.json"))
}

fn load_fixture(id: &str) -> Fixture {
    let path = fixture_path(id);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("Cannot read fixture {id} at {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("Invalid JSON in fixture {id}: {e}\n{text}"))
}

fn parse_ontology(fixture: &Fixture) -> Ontology {
    match fixture.source.as_str() {
        "Functional" => parse_functional(&fixture.ontology)
            .unwrap_or_else(|e| panic!("Functional parse failed: {e}\n{}", fixture.ontology)),
        "Turtle" => parse_turtle(&fixture.ontology)
            .unwrap_or_else(|e| panic!("Turtle parse failed: {e}\n{}", fixture.ontology)),
        "RDFXML" => parse_rdf_xml(&fixture.ontology)
            .unwrap_or_else(|e| panic!("RDF/XML parse failed: {e}\n{}", fixture.ontology)),
        "OWLXML" => parse_owl_xml(&fixture.ontology)
            .unwrap_or_else(|e| panic!("OWL/XML parse failed: {e}\n{}", fixture.ontology)),
        "Path" => parse_file_auto(&fixture.ontology)
            .unwrap_or_else(|e| panic!("File parse failed for {}: {e}", fixture.ontology)),
        other => panic!("Unsupported source {other} in fixture {}", fixture.id),
    }
}

fn reasoner_for(ontology: Ontology) -> Reasoner {
    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("Failed to create Reasoner");
    reasoner
        .load_ontology(ontology)
        .expect("Failed to load ontology");
    reasoner
}

// ─────────────────────────────────────────────────────────────────────────────
// Name resolution helpers
// ─────────────────────────────────────────────────────────────────────────────

fn class_thing() -> ClassExpression {
    ClassExpression::Class(Class::thing())
}
fn class_nothing() -> ClassExpression {
    ClassExpression::Class(Class::nothing())
}

/// Resolve a short class token against the fixture base.
/// Accepts `owl:Thing`/`owl:Nothing`, full IRIs `<…>`, `:Local`, or bare `Local`.
fn resolve_class(base: &str, token: &str) -> ClassExpression {
    let t = token.trim();
    match t {
        "owl:Thing" => class_thing(),
        "owl:Nothing" => class_nothing(),
        _ if t.starts_with('<') && t.ends_with('>') => {
            let iri = &t[1..t.len() - 1];
            ClassExpression::Class(Class { iri: IRI::new(iri) })
        }
        _ => {
            let local = t.strip_prefix(':').unwrap_or(t);
            let iri = format!("{base}{local}");
            ClassExpression::Class(Class {
                iri: IRI::new(&iri),
            })
        }
    }
}

fn resolve_individual(base: &str, token: &str) -> Individual {
    let t = token.trim();
    if t.starts_with('<') && t.ends_with('>') {
        Individual::named(IRI::new(&t[1..t.len() - 1]))
    } else {
        let local = t.strip_prefix(':').unwrap_or(t);
        let iri = format!("{base}{local}");
        Individual::named(IRI::new(&iri))
    }
}

fn individual_iri_str(ind: &Individual) -> Option<String> {
    ind.iri().map(|i| i.as_str().to_string())
}

/// Parse a Functional-Syntax axiom body (`SubClassOf(:A :B)`, …) wrapped in a
/// minimal ontology declaring the `:` prefix against `base`.
fn parse_axiom_bodies(base: &str, body: &str) -> Vec<Axiom> {
    let onto_iri = base.trim_end_matches('#');
    let fs = format!("Prefix(:=<{base}>)\nOntology(<{onto_iri}>\n  {body}\n)");
    let ontology = parse_functional(&fs)
        .unwrap_or_else(|e| panic!("Failed to parse axiom body `{body}`: {e}\n{fs}"));
    ontology.axioms().to_vec()
}

// ─────────────────────────────────────────────────────────────────────────────
// Profile validation
// ─────────────────────────────────────────────────────────────────────────────

fn profile_conforms(ontology: &Ontology, profile: OWL2Profile) -> bool {
    OWL2ProfileValidator::new()
        .validate_profile(ontology, profile)
        .is_ok_and(|r| r.conforms)
}

fn profile_from_key(key: &str) -> OWL2Profile {
    match key {
        "el" => OWL2Profile::EL,
        "ql" => OWL2Profile::QL,
        "rl" => OWL2Profile::RL,
        "dl" => OWL2Profile::DL,
        "full" => OWL2Profile::Full,
        other => panic!("Unknown profile key {other}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Computing oxidowl's actual verdicts
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
struct Actual {
    axiom_count: usize,
    consistent: bool,
    satisfiable: HashMap<String, bool>,
    subsumption: HashMap<String, bool>,
    equivalence: HashMap<String, bool>,
    instances: HashMap<String, Vec<String>>,
    types: HashMap<String, Vec<String>>,
    entailed: HashMap<String, bool>,
    same_as: HashMap<String, bool>,
    different_from: HashMap<String, bool>,
    profiles: HashMap<String, bool>,
}

fn compute_actual(fixture: &Fixture) -> Actual {
    let base = fixture.base();
    let ontology = parse_ontology(fixture);
    let axiom_count = ontology.axioms().len();
    let expected = &fixture.expected;
    let mut reasoner = reasoner_for(ontology.clone());
    let mut actual = Actual {
        axiom_count,
        ..Actual::default()
    };

    if let Some(exp) = expected.consistent {
        actual.consistent = reasoner.is_consistent().expect("is_consistent failed");
        let _ = exp;
    }

    if let Some(map) = &expected.satisfiable {
        for class in map.keys() {
            let ce = resolve_class(&base, class);
            let sat = reasoner
                .is_class_satisfiable(&ce)
                .unwrap_or_else(|e| panic!("is_class_satisfiable({class}) failed: {e}"));
            actual.satisfiable.insert(class.clone(), sat);
        }
    }

    if let Some(pairs) = &expected.subsumptions {
        for pair in pairs {
            let sub = resolve_class(&base, &pair[0]);
            let sup = resolve_class(&base, &pair[1]);
            let key = format!("{}⊑{}", pair[0], pair[1]);
            let val = reasoner
                .is_subclass_of(&sub, &sup)
                .unwrap_or_else(|e| panic!("is_subclass_of({key}) failed: {e}"));
            actual.subsumption.insert(key, val);
        }
    }
    if let Some(pairs) = &expected.non_subsumptions {
        for pair in pairs {
            let sub = resolve_class(&base, &pair[0]);
            let sup = resolve_class(&base, &pair[1]);
            let key = format!("{}⊑{}", pair[0], pair[1]);
            let val = reasoner
                .is_subclass_of(&sub, &sup)
                .unwrap_or_else(|e| panic!("is_subclass_of({key}) failed: {e}"));
            actual.subsumption.insert(key, val);
        }
    }
    if let Some(pairs) = &expected.equivalences {
        for pair in pairs {
            let a = resolve_class(&base, &pair[0]);
            let b = resolve_class(&base, &pair[1]);
            let key = format!("{}≡{}", pair[0], pair[1]);
            let fwd = reasoner
                .is_subclass_of(&a, &b)
                .unwrap_or_else(|e| panic!("is_subclass_of({key}) failed: {e}"));
            let bwd = reasoner
                .is_subclass_of(&b, &a)
                .unwrap_or_else(|e| panic!("is_subclass_of({key}) failed: {e}"));
            actual.equivalence.insert(key, fwd && bwd);
        }
    }

    if let Some(map) = &expected.instances {
        for class in map.keys() {
            let ce = resolve_class(&base, class);
            let inds = reasoner
                .get_instances(&ce, false)
                .unwrap_or_else(|e| panic!("get_instances({class}) failed: {e}"));
            actual.instances.insert(
                class.clone(),
                inds.iter().filter_map(individual_iri_str).collect(),
            );
        }
    }

    if let Some(map) = &expected.types {
        for ind in map.keys() {
            let indv = resolve_individual(&base, ind);
            let types = reasoner
                .get_types(&indv, false)
                .unwrap_or_else(|e| panic!("get_types({ind}) failed: {e}"));
            actual.types.insert(
                ind.clone(),
                types
                    .iter()
                    .filter_map(|ce| match ce {
                        ClassExpression::Class(c) => Some(c.iri.as_str().to_string()),
                        _ => None,
                    })
                    .collect(),
            );
        }
    }

    let ont_ref = reasoner.get_ontology().expect("no ontology loaded").clone();

    if let Some(bodies) = &expected.entailed {
        for body in bodies {
            let axioms = parse_axiom_bodies(&base, body);
            let all = axioms.iter().all(|ax| {
                let mut stats = ReasoningStatistics::default();
                reasoner
                    .check_entailment(ax, &ont_ref, &mut stats)
                    .unwrap_or_else(|e| panic!("check_entailment({body}) failed: {e}"))
            });
            actual.entailed.insert(body.clone(), all);
        }
    }
    if let Some(bodies) = &expected.not_entailed {
        for body in bodies {
            let axioms = parse_axiom_bodies(&base, body);
            let any = axioms.iter().any(|ax| {
                let mut stats = ReasoningStatistics::default();
                reasoner
                    .check_entailment(ax, &ont_ref, &mut stats)
                    .unwrap_or_else(|e| panic!("check_entailment({body}) failed: {e}"))
            });
            actual.entailed.insert(body.clone(), any);
        }
    }

    if let Some(pairs) = &expected.same_as {
        for pair in pairs {
            let key = format!("sameAs({},{})", pair[0], pair[1]);
            let a = pair[0].strip_prefix(':').unwrap_or(&pair[0]);
            let b = pair[1].strip_prefix(':').unwrap_or(&pair[1]);
            let body = format!("SameIndividual(:{a} :{b})");
            let axioms = parse_axiom_bodies(&base, &body);
            let val = axioms.iter().all(|ax| {
                let mut stats = ReasoningStatistics::default();
                reasoner
                    .check_entailment(ax, &ont_ref, &mut stats)
                    .unwrap_or_else(|e| panic!("check_entailment({key}) failed: {e}"))
            });
            actual.same_as.insert(key, val);
        }
    }
    if let Some(pairs) = &expected.different_from {
        for pair in pairs {
            let key = format!("differentFrom({},{})", pair[0], pair[1]);
            let a = pair[0].strip_prefix(':').unwrap_or(&pair[0]);
            let b = pair[1].strip_prefix(':').unwrap_or(&pair[1]);
            let body = format!("DifferentIndividuals(:{a} :{b})");
            let axioms = parse_axiom_bodies(&base, &body);
            let val = axioms.iter().all(|ax| {
                let mut stats = ReasoningStatistics::default();
                reasoner
                    .check_entailment(ax, &ont_ref, &mut stats)
                    .unwrap_or_else(|e| panic!("check_entailment({key}) failed: {e}"))
            });
            actual.different_from.insert(key, val);
        }
    }

    if let Some(map) = &expected.profiles {
        for key in map.keys() {
            let profile = profile_from_key(key);
            actual
                .profiles
                .insert(key.clone(), profile_conforms(&ontology, profile));
        }
    }

    actual
}

// ─────────────────────────────────────────────────────────────────────────────
// Comparison
// ─────────────────────────────────────────────────────────────────────────────

/// Returns a list of human-readable check lines.  Lines prefixed `✗` are
/// mismatches (expected ≠ actual).
fn compare(fixture: &Fixture, actual: &Actual) -> Vec<String> {
    let expected = &fixture.expected;
    let mut lines: Vec<String> = Vec::new();

    if let Some(min) = fixture.axioms.as_ref().and_then(|a| a.min) {
        let ok = actual.axiom_count >= min;
        lines.push(format!(
            "  {} axioms ≥ {min} (actual {})",
            mark(ok),
            actual.axiom_count
        ));
    }
    if let Some(max) = fixture.axioms.as_ref().and_then(|a| a.max) {
        let ok = actual.axiom_count <= max;
        lines.push(format!(
            "  {} axioms ≤ {max} (actual {})",
            mark(ok),
            actual.axiom_count
        ));
    }

    if let Some(exp) = expected.consistent {
        let ok = exp == actual.consistent;
        lines.push(format!(
            "  {} consistent: expected {exp} actual {}",
            mark(ok),
            actual.consistent
        ));
    }

    if let Some(map) = &expected.satisfiable {
        for (class, exp) in map {
            let act = actual.satisfiable.get(class).copied().unwrap_or(false);
            let ok = *exp == act;
            lines.push(format!(
                "  {} satisfiable({class}): expected {exp} actual {act}",
                mark(ok)
            ));
        }
    }

    if let Some(pairs) = &expected.subsumptions {
        for pair in pairs {
            let key = format!("{}⊑{}", pair[0], pair[1]);
            let act = actual.subsumption.get(&key).copied().unwrap_or(false);
            let ok = act;
            lines.push(format!(
                "  {} subsumption {key}: expected true actual {act}",
                mark(ok)
            ));
        }
    }
    if let Some(pairs) = &expected.non_subsumptions {
        for pair in pairs {
            let key = format!("{}⊑{}", pair[0], pair[1]);
            let act = actual.subsumption.get(&key).copied().unwrap_or(true);
            let ok = !act;
            lines.push(format!(
                "  {} nonSubsumption {key}: expected false actual {act}",
                mark(ok)
            ));
        }
    }
    if let Some(pairs) = &expected.equivalences {
        for pair in pairs {
            let key = format!("{}≡{}", pair[0], pair[1]);
            let act = actual.equivalence.get(&key).copied().unwrap_or(false);
            let ok = act;
            lines.push(format!(
                "  {} equivalence {key}: expected true actual {act}",
                mark(ok)
            ));
        }
    }

    if let Some(map) = &expected.instances {
        for (class, expected_inds) in map {
            let actual_inds = actual.instances.get(class).cloned().unwrap_or_default();
            let exp_set: HashSet<String> = expected_inds
                .iter()
                .map(|t| {
                    let t = t.trim_start_matches(':');
                    format!("{}{}", fixture.base(), t)
                })
                .collect();
            let act_set: HashSet<String> = actual_inds.into_iter().collect();
            let missing: Vec<_> = exp_set.difference(&act_set).collect();
            let extra: Vec<_> = act_set.difference(&exp_set).collect();
            let ok = missing.is_empty() && extra.is_empty();
            lines.push(format!(
                "  {} instances({class}): expected {:?} actual {:?}{}{}",
                mark(ok),
                expected_inds,
                act_set,
                if missing.is_empty() {
                    String::new()
                } else {
                    format!(" MISSING {missing:?}")
                },
                if extra.is_empty() {
                    String::new()
                } else {
                    format!(" EXTRA {extra:?}")
                },
            ));
        }
    }

    if let Some(map) = &expected.types {
        for (ind, expected_classes) in map {
            let actual_types = actual.types.get(ind).cloned().unwrap_or_default();
            for class in expected_classes {
                let target = resolve_class(&fixture.base(), class).to_string();
                let ok = actual_types.contains(&target);
                lines.push(format!(
                    "  {} type({ind}) ∈ {class}: expected true actual {}",
                    mark(ok),
                    actual_types.contains(&target)
                ));
            }
        }
    }

    if let Some(bodies) = &expected.entailed {
        for body in bodies {
            let act = actual.entailed.get(body).copied().unwrap_or(false);
            let ok = act;
            lines.push(format!(
                "  {} entailed `{body}`: expected true actual {act}",
                mark(ok)
            ));
        }
    }
    if let Some(bodies) = &expected.not_entailed {
        for body in bodies {
            let act = actual.entailed.get(body).copied().unwrap_or(true);
            let ok = !act;
            lines.push(format!(
                "  {} notEntailed `{body}`: expected false actual {act}",
                mark(ok)
            ));
        }
    }

    if let Some(pairs) = &expected.same_as {
        for pair in pairs {
            let key = format!("sameAs({},{})", pair[0], pair[1]);
            let act = actual.same_as.get(&key).copied().unwrap_or(false);
            let ok = act;
            lines.push(format!("  {} {key}: expected true actual {act}", mark(ok)));
        }
    }
    if let Some(pairs) = &expected.different_from {
        for pair in pairs {
            let key = format!("differentFrom({},{})", pair[0], pair[1]);
            let act = actual.different_from.get(&key).copied().unwrap_or(false);
            let ok = act;
            lines.push(format!("  {} {key}: expected true actual {act}", mark(ok)));
        }
    }

    if let Some(map) = &expected.profiles {
        for (key, exp) in map {
            let act = actual.profiles.get(key).copied().unwrap_or(false);
            let ok = *exp == act;
            lines.push(format!(
                "  {} profile {key}: expected {exp} actual {act}",
                mark(ok)
            ));
        }
    }

    lines
}

fn mark(ok: bool) -> &'static str {
    if ok { "✓" } else { "✗" }
}

fn fixture_passes(lines: &[String]) -> bool {
    !lines.iter().any(|l| l.starts_with("  ✗"))
}

// ─────────────────────────────────────────────────────────────────────────────
// Single-fixture runner (used by the strict per-fixture tests)
// ─────────────────────────────────────────────────────────────────────────────

fn run_fixture_strict(id: &str) {
    let fixture = load_fixture(id);
    let actual = compute_actual(&fixture);
    let lines = compare(&fixture, &actual);
    let passes = fixture_passes(&lines);

    println!("== fixture {id} ==");
    for l in &lines {
        println!("{l}");
    }
    if let Some(r) = &fixture.rationale {
        println!("   rationale: {r}");
    }

    assert!(
        passes,
        "FIXTURE {id} DIVERGES from golden (HermiT/Konclude semantics).\n{}\n",
        lines.join("\n")
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Golden JSON generation (oxidowl actual output, for review/promotion)
// ─────────────────────────────────────────────────────────────────────────────

fn actual_to_golden_json(fixture: &Fixture, actual: &Actual) -> Value {
    let base = fixture.base();
    let mut exp = serde_json::Map::new();
    if fixture.expected.consistent.is_some() {
        exp.insert("consistent".into(), Value::Bool(actual.consistent));
    }
    if let Some(map) = &fixture.expected.satisfiable {
        exp.insert(
            "satisfiable".into(),
            serde_json::to_value(&actual.satisfiable).unwrap(),
        );
        let _ = map;
    }
    if let Some(pairs) = &fixture.expected.subsumptions {
        let arr: Vec<Value> = pairs
            .iter()
            .map(|p| {
                let key = format!("{}⊑{}", p[0], p[1]);
                serde_json::json!([
                    p[0],
                    p[1],
                    actual.subsumption.get(&key).copied().unwrap_or(false)
                ])
            })
            .collect();
        exp.insert("subsumptions".into(), Value::Array(arr));
    }
    if let Some(pairs) = &fixture.expected.non_subsumptions {
        let arr: Vec<Value> = pairs
            .iter()
            .map(|p| {
                let key = format!("{}⊑{}", p[0], p[1]);
                serde_json::json!([
                    p[0],
                    p[1],
                    actual.subsumption.get(&key).copied().unwrap_or(true)
                ])
            })
            .collect();
        exp.insert("nonSubsumptions".into(), Value::Array(arr));
    }
    if let Some(pairs) = &fixture.expected.equivalences {
        let arr: Vec<Value> = pairs
            .iter()
            .map(|p| {
                let key = format!("{}≡{}", p[0], p[1]);
                serde_json::json!([
                    p[0],
                    p[1],
                    actual.equivalence.get(&key).copied().unwrap_or(false)
                ])
            })
            .collect();
        exp.insert("equivalences".into(), Value::Array(arr));
    }
    if let Some(map) = &fixture.expected.instances {
        let mut m = serde_json::Map::new();
        for class in map.keys() {
            m.insert(
                class.clone(),
                serde_json::to_value(actual.instances.get(class).unwrap_or(&vec![])).unwrap(),
            );
        }
        exp.insert("instances".into(), Value::Object(m));
    }
    if let Some(map) = &fixture.expected.types {
        let mut m = serde_json::Map::new();
        for (ind, classes) in map {
            let actual_types = actual.types.get(ind).cloned().unwrap_or_default();
            let mut present = Vec::new();
            for class in classes {
                let target = resolve_class(&base, class).to_string();
                if actual_types.contains(&target) {
                    present.push(class.clone());
                }
            }
            m.insert(ind.clone(), serde_json::to_value(present).unwrap());
        }
        exp.insert("types".into(), Value::Object(m));
    }
    if let Some(bodies) = &fixture.expected.entailed {
        let arr: Vec<Value> = bodies
            .iter()
            .map(|b| serde_json::json!([b, actual.entailed.get(b).copied().unwrap_or(false)]))
            .collect();
        exp.insert("entailed".into(), Value::Array(arr));
    }
    if let Some(bodies) = &fixture.expected.not_entailed {
        let arr: Vec<Value> = bodies
            .iter()
            .map(|b| serde_json::json!([b, actual.entailed.get(b).copied().unwrap_or(true)]))
            .collect();
        exp.insert("notEntailed".into(), Value::Array(arr));
    }
    if let Some(pairs) = &fixture.expected.same_as {
        let arr: Vec<Value> = pairs
            .iter()
            .map(|p| {
                let key = format!("sameAs({},{})", p[0], p[1]);
                serde_json::json!([
                    p[0],
                    p[1],
                    actual.same_as.get(&key).copied().unwrap_or(false)
                ])
            })
            .collect();
        exp.insert("sameAs".into(), Value::Array(arr));
    }
    if let Some(pairs) = &fixture.expected.different_from {
        let arr: Vec<Value> = pairs
            .iter()
            .map(|p| {
                let key = format!("differentFrom({},{})", p[0], p[1]);
                serde_json::json!([
                    p[0],
                    p[1],
                    actual.different_from.get(&key).copied().unwrap_or(false)
                ])
            })
            .collect();
        exp.insert("differentFrom".into(), Value::Array(arr));
    }
    if let Some(map) = &fixture.expected.profiles {
        exp.insert(
            "profiles".into(),
            serde_json::to_value(&actual.profiles).unwrap(),
        );
        let _ = map;
    }

    serde_json::json!({
        "id": fixture.id,
        "source": fixture.source,
        "ontology": fixture.ontology,
        "base": base,
        "actual": exp,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Strict per-fixture tests (FAIL on mismatch — this is the point)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn differential_parity_diff_001_alc_subsumption() {
    run_fixture_strict("diff_001_alc_subsumption");
}

#[test]
fn differential_parity_diff_002_alc_disjoint_unsat() {
    run_fixture_strict("diff_002_alc_disjoint_unsat");
}

#[test]
fn differential_parity_diff_003_alc_inconsistent() {
    run_fixture_strict("diff_003_alc_inconsistent");
}

#[test]
fn differential_parity_diff_004_shi_role_hierarchy_inverse() {
    run_fixture_strict("diff_004_shi_role_hierarchy_inverse");
}

// Fixed in the C2 fix wave: `check_complex_expression` now evaluates
// `ObjectOneOf` (and the equivalent-class dispatch reaches it), and
// `get_instances` merges nominal individuals that `signature()` misses —
// so `a ∈ A` for `A ≡ {a}` flows through `instances`/`types`.
#[test]
fn differential_parity_diff_005_shoin_nominal_oneof() {
    run_fixture_strict("diff_005_shoin_nominal_oneof");
}

#[test]
fn differential_parity_diff_006_shoin_object_has_value() {
    run_fixture_strict("diff_006_shoin_object_has_value");
}

#[test]
fn differential_parity_diff_007_shoin_nominal_cardinality() {
    run_fixture_strict("diff_007_shoin_nominal_cardinality");
}

#[test]
fn differential_parity_diff_008_sroiq_role_chain() {
    run_fixture_strict("diff_008_sroiq_role_chain");
}

#[test]
fn differential_parity_diff_009_sroiq_qualified_cardinality() {
    run_fixture_strict("diff_009_sroiq_qualified_cardinality");
}

#[test]
fn differential_parity_diff_010_sroiq_self_reflexive() {
    run_fixture_strict("diff_010_sroiq_self_reflexive");
}

#[test]
fn differential_parity_diff_011_sroiq_rbox_inconsistent() {
    run_fixture_strict("diff_011_sroiq_rbox_inconsistent");
}

// Fixed in the C2 fix wave: `Reasoner::get_types` now derives types from
// *defined* classes (named operand of `EquivalentClasses` with a complex
// counterpart) via `check_instance_with_datatype_reasoning`, so datatype-derived
// memberships (Adult/IntRange) are reported alongside asserted ones.
#[test]
fn differential_parity_diff_012_datatypes_int_integer_facets() {
    run_fixture_strict("diff_012_datatypes_int_integer_facets");
}

#[test]
fn differential_parity_diff_013_datatypes_hasvalue_oneof() {
    run_fixture_strict("diff_013_datatypes_hasvalue_oneof");
}

#[test]
fn differential_parity_diff_014_profiles_el_all() {
    run_fixture_strict("diff_014_profiles_el_all");
}

#[test]
fn differential_parity_diff_015_profiles_rl_not_el() {
    run_fixture_strict("diff_015_profiles_rl_not_el");
}

#[test]
fn differential_parity_diff_016_profiles_full_only() {
    run_fixture_strict("diff_016_profiles_full_only");
}

#[test]
fn differential_parity_diff_017_turtle_rdf_entailment() {
    run_fixture_strict("diff_017_turtle_rdf_entailment");
}

#[test]
fn differential_parity_diff_018_rdfxml_rdf_entailment() {
    run_fixture_strict("diff_018_rdfxml_rdf_entailment");
}

#[test]
fn differential_parity_diff_019_ore_ont_10009() {
    run_fixture_strict("diff_019_ore_ont_10009");
}

#[test]
fn differential_parity_diff_020_swrl_safe_subset() {
    run_fixture_strict("diff_020_swrl_safe_subset");
}

// ─────────────────────────────────────────────────────────────────────────────
// Report-only + golden-generation auxiliary tests (ignored by default)
// ─────────────────────────────────────────────────────────────────────────────

/// Run every fixture in report-only mode: print the full comparison but never
/// fail.  Use while authoring fixtures:
/// `cargo test differential_parity_tests::report_only -- --ignored --nocapture`
#[test]
#[ignore = "report-only fixture authoring mode"]
fn report_only() {
    let mut ids: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(fixtures_dir()).expect("fixtures dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            ids.push(path.file_stem().unwrap().to_string_lossy().to_string());
        }
    }
    ids.sort();
    let mut passed = 0usize;
    let mut failed = 0usize;
    for id in &ids {
        let fixture = load_fixture(id);
        let actual = compute_actual(&fixture);
        let lines = compare(&fixture, &actual);
        let ok = fixture_passes(&lines);
        if ok {
            passed += 1;
        } else {
            failed += 1;
        }
        println!("== {} [{}] ==", id, if ok { "PASS" } else { "FAIL" });
        for l in &lines {
            println!("{l}");
        }
    }
    println!("\nSUMMARY: {passed} fixtures PASS, {failed} fixtures FAIL (divergences from golden)");
}

/// Print oxidowl's actual verdicts in the golden JSON shape so a human can
/// review them and promote them to golden (after cross-checking against
/// HermiT/Konclude or OWL 2 semantics).
/// `cargo test differential_parity_tests::generate_golden_json -- --ignored --nocapture`
#[test]
#[ignore = "golden generation mode"]
fn generate_golden_json() {
    let mut ids: Vec<String> = Vec::new();
    for entry in std::fs::read_dir(fixtures_dir()).expect("fixtures dir") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "json") {
            ids.push(path.file_stem().unwrap().to_string_lossy().to_string());
        }
    }
    ids.sort();
    for id in &ids {
        let fixture = load_fixture(id);
        let actual = compute_actual(&fixture);
        let json = actual_to_golden_json(&fixture, &actual);
        println!(
            "--- {} ---\n{}",
            id,
            serde_json::to_string_pretty(&json).unwrap()
        );
    }
}

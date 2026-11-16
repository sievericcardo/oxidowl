//! Feature-parity tests: `owl:imports` closure resolution in the reasoner
//! (HermiT `Imports.INCLUDED` parity).
//!
//! 1. `Reasoner::load_ontology_from_file` merges the imported axioms, so
//!    `is_consistent` / `get_instances` see them;
//! 2. import cycles terminate (visited-IRI tracking) and do not duplicate
//!    axioms (content-level deduplication on merge);
//! 3. a missing import is best-effort — the load warns, never fails;
//! 4. `ImportManager::resolve_imports_recursive` is usable directly.
//!
//! All fixtures are written to `tempfile` directories and referenced through
//! `file://` URIs — no network access is performed.

use oxidowl::{
    config::ReasonerConfig,
    core::reasoner::Reasoner,
    import::{ImportError, ImportManager, ImportManagerConfig, ImportResolutionStrategy},
    ontology::{
        Axiom, Class, ClassExpression, IRI, ImportsDeclaration, Individual, Ontology,
        OntologyFormat,
    },
};
use std::path::{Path, PathBuf};

// ── Helpers ──────────────────────────────────────────────────────────────────

fn cls(iri: &str) -> ClassExpression {
    ClassExpression::Class(Class::new(IRI::new(iri)))
}

/// `file://` URI for an absolute local path (`/x/y` → `file:///x/y`).
fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn write(path: &Path, content: &str) -> PathBuf {
    std::fs::write(path, content).expect("fixture write");
    path.to_path_buf()
}

fn reasoner() -> Reasoner {
    Reasoner::new(ReasonerConfig::default()).expect("reasoner construction")
}

// ── 1. Local file import is merged and visible to reasoning ─────────────────

#[test]
fn load_ontology_merges_file_import_closure() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Child: A ⊑ B
    let child = dir.path().join("child.ofn");
    write(
        &child,
        r#"Ontology(<http://example.org/child>
  Declaration(Class(<http://example.org/A>))
  Declaration(Class(<http://example.org/B>))
  SubClassOf(<http://example.org/A> <http://example.org/B>)
)
"#,
    );

    // Root: imports the child via file:// and asserts A(a).
    let root = dir.path().join("root.ofn");
    write(
        &root,
        &format!(
            r#"Ontology(<http://example.org/root>
  Import(<{child_uri}>)
  Declaration(Class(<http://example.org/A>))
  Declaration(Class(<http://example.org/B>))
  Declaration(NamedIndividual(<http://example.org/a>))
  ClassAssertion(<http://example.org/A> <http://example.org/a>)
)
"#,
            child_uri = file_uri(&child)
        ),
    );

    let mut r = reasoner();
    r.load_ontology_from_file(&root, OntologyFormat::Functional)
        .expect("load root with imports");

    // The child's `A ⊑ B` is part of the loaded ontology, deduplicated
    // against the root's own declarations: 4 root axioms + 1 imported axiom.
    {
        let guard = r
            .get_ontology()
            .expect("ontology loaded")
            .read()
            .expect("ontology read lock");
        let a = cls("http://example.org/A");
        let b = cls("http://example.org/B");
        assert!(
            guard.axioms().iter().any(|axiom| matches!(
                axiom,
                Axiom::SubClassOf(sa) if sa.subclass == a && sa.superclass == b
            )),
            "child axiom A ⊑ B missing from the import closure: {:#?}",
            guard.axioms()
        );
        assert_eq!(
            guard.axioms().len(),
            5,
            "expected 4 root axioms + 1 imported axiom after dedup: {:#?}",
            guard.axioms()
        );
    }

    // The reasoner reasons over the *closed* ontology …
    assert!(r.is_consistent().expect("consistency check"));

    // … so `a` is an instance of B through the imported subsumption.
    let instances = r
        .get_instances(&cls("http://example.org/B"), false)
        .expect("instances of B");
    assert!(
        instances.contains(&Individual::named(IRI::new("http://example.org/a"))),
        "expected a ∈ B via the imported axiom, got {instances:?}"
    );
}

// ── 2. Cycles terminate and merge without duplication ───────────────────────

#[test]
fn import_cycle_terminates_and_merges_once() {
    let dir = tempfile::tempdir().expect("tempdir");
    let a_file = dir.path().join("a.ofn");
    let b_file = dir.path().join("b.ofn");

    // A imports B, B imports A.
    write(
        &a_file,
        &format!(
            r#"Ontology(<http://example.org/a>
  Import(<{b_uri}>)
  Declaration(Class(<http://example.org/A>))
  SubClassOf(<http://example.org/A> <http://example.org/B>)
)
"#,
            b_uri = file_uri(&b_file)
        ),
    );
    write(
        &b_file,
        &format!(
            r#"Ontology(<http://example.org/b>
  Import(<{a_uri}>)
  Declaration(Class(<http://example.org/B>))
  SubClassOf(<http://example.org/B> <http://example.org/C>)
)
"#,
            a_uri = file_uri(&a_file)
        ),
    );

    let mut r = reasoner();
    // Must terminate: without visited-IRI tracking this recurses a → b → a → …
    r.load_ontology_from_file(&a_file, OntologyFormat::Functional)
        .expect("loading a cyclic import graph must not fail");

    {
        let guard = r
            .get_ontology()
            .expect("ontology loaded")
            .read()
            .expect("ontology read lock");

        // b's axiom reached the closure …
        let b = cls("http://example.org/B");
        let c = cls("http://example.org/C");
        assert!(
            guard.axioms().iter().any(|axiom| matches!(
                axiom,
                Axiom::SubClassOf(sa) if sa.subclass == b && sa.superclass == c
            )),
            "B ⊑ C from the imported ontology missing: {:#?}",
            guard.axioms()
        );

        // … a's own axioms arrived exactly once (content-level dedup): the
        // cycle returns them through the a.ofn clone but they are dropped.
        let keys: Vec<String> = guard.axioms().iter().map(Axiom::content_key).collect();
        let unique: std::collections::HashSet<&String> = keys.iter().collect();
        assert_eq!(
            unique.len(),
            keys.len(),
            "duplicate axioms after cyclic merge: {keys:#?}"
        );
        assert_eq!(
            guard.axioms().len(),
            4,
            "expected exactly 4 axioms (DeclA, A⊑B, DeclB, B⊑C): {:#?}",
            guard.axioms()
        );
    }

    assert!(r.is_consistent().expect("consistency check"));
}

// ── 3. Missing imports are best-effort ──────────────────────────────────────

#[test]
fn missing_import_does_not_fail_the_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("missing_import.ofn");
    let missing = dir.path().join("does_not_exist").join("child.ofn");
    write(
        &root,
        &format!(
            r#"Ontology(<http://example.org/missing-import>
  Import(<{missing_uri}>)
  Declaration(Class(<http://example.org/A>))
)
"#,
            missing_uri = file_uri(&missing)
        ),
    );

    let mut r = reasoner();
    r.load_ontology_from_file(&root, OntologyFormat::Functional)
        .expect("a missing import must never fail the load (best effort)");

    assert_eq!(
        r.get_ontology_size(),
        1,
        "the ontology's own axioms must survive a failed import"
    );
    assert!(r.is_consistent().expect("consistency check"));
}

// ── 4. Direct resolver usage ────────────────────────────────────────────────

#[test]
fn resolve_imports_recursive_reports_unresolvable_import() {
    let config = ImportManagerConfig {
        resolution_strategy: ImportResolutionStrategy::BestEffort,
        max_import_depth: 10,
        base_directories: vec![std::path::PathBuf::from(".")],
        url_mappings: std::collections::HashMap::new(),
        validate_imports: false,
        merge_imports: true,
        timeout: Some(std::time::Duration::from_secs(10)),
    };
    let manager = ImportManager::new(config);

    let mut onto = Ontology::new();
    onto.set_iri(IRI::new("http://example.org/direct"));
    onto.imports.push(ImportsDeclaration {
        imported_ontology_iri: IRI::new("urn:oxidowl:test:definitely_missing"),
    });

    let results = manager
        .resolve_imports_recursive(&mut onto)
        .expect("best-effort resolution never fails");
    assert_eq!(results.len(), 1, "one import → one resolution result");
    assert!(
        results[0]
            .errors
            .iter()
            .any(|e| matches!(e, ImportError::ResolutionFailed { .. })),
        "unresolvable import must be recorded as ResolutionFailed: {:?}",
        results[0].errors
    );
    assert_eq!(onto.axioms().len(), 0, "nothing must be merged");
}

#[test]
fn resolve_imports_recursive_strict_mode_fails_on_missing_import() {
    let config = ImportManagerConfig {
        resolution_strategy: ImportResolutionStrategy::Strict,
        ..ImportManagerConfig::default()
    };
    let manager = ImportManager::new(config);

    let mut onto = Ontology::new();
    onto.set_iri(IRI::new("http://example.org/direct-strict"));
    onto.imports.push(ImportsDeclaration {
        imported_ontology_iri: IRI::new("urn:oxidowl:test:definitely_missing"),
    });

    let err = manager.resolve_imports_recursive(&mut onto);
    assert!(
        err.is_err(),
        "strict mode must surface the failed import as an error"
    );
}

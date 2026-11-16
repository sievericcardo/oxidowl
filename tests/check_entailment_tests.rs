//! Feature-parity tests: HermiT-style `--premise/--conclusion` entailment.
//!
//! `premise ⊨ conclusion` holds iff **every** axiom of the conclusion is
//! entailed by the premise. Covered here:
//!
//! 1. library level — an identical conclusion (`A ⊑ B` over a premise with
//!    `A ⊑ B`) is entailed, the reverse (`B ⊑ A`) is not;
//! 2. CLI `check-entailment --premise … --conclusion …` (subcommand) exits 0
//!    iff all conclusion axioms are entailed, 1 otherwise;
//! 3. CLI `--check-entailment <premise> <conclusion>` (flag mode) same exit
//!    contract.

use oxidowl::{
    config::ReasonerConfig,
    core::reasoner::{Reasoner, statistics::ReasoningStatistics},
    ontology::{Ontology, OntologyFormat},
};
use std::path::Path;
use std::process::Command;

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Write a functional-syntax ontology file into `dir`.
fn write_ofn(dir: &Path, name: &str, iri: &str, body: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    let content = format!("Ontology(<{iri}>\n{body})\n");
    std::fs::write(&path, content).expect("fixture write");
    path
}

/// Full IRIs (no prefix declarations): every fixture file expands entities
/// identically, so axiom *content* matches across files while the
/// parser-minted axiom ids differ — exactly the cross-file entailment case.
const CLASS_A: &str = "<http://example.org/A>";
const CLASS_B: &str = "<http://example.org/B>";

const DECLS: &str = r#"  Declaration(Class(<http://example.org/A>))
  Declaration(Class(<http://example.org/B>))
"#;

fn premise_body() -> String {
    format!("{DECLS}  SubClassOf({CLASS_A} {CLASS_B})\n")
}

// ── 1. Library level ─────────────────────────────────────────────────────────

#[test]
fn premise_entails_identical_conclusion_but_not_the_reverse() {
    let dir = tempfile::tempdir().expect("tempdir");

    let premise = write_ofn(
        dir.path(),
        "premise.ofn",
        "http://example.org/premise",
        &premise_body(),
    );
    // Entailed conclusion: same axioms, its own file → its own axiom ids.
    let entailed = write_ofn(
        dir.path(),
        "entailed.ofn",
        "http://example.org/conclusion-entailed",
        &premise_body(),
    );
    // Not entailed: B ⊑ A does not follow from A ⊑ B.
    let not_entailed = write_ofn(
        dir.path(),
        "not_entailed.ofn",
        "http://example.org/conclusion-not-entailed",
        &format!("{DECLS}  SubClassOf({CLASS_B} {CLASS_A})\n"),
    );

    let mut reasoner = Reasoner::new(ReasonerConfig::default()).expect("reasoner");
    reasoner
        .load_ontology_from_file(&premise, OntologyFormat::Functional)
        .expect("load premise");
    let onto_ref = reasoner.get_ontology().expect("premise loaded").clone();

    let conclusion = Ontology::from_file(&entailed, None).expect("load entailed conclusion");
    assert!(!conclusion.axioms().is_empty());
    for axiom in conclusion.axioms() {
        let mut stats = ReasoningStatistics::new();
        let entailed = reasoner
            .check_entailment(axiom, &onto_ref, &mut stats)
            .unwrap_or_else(|e| panic!("check_entailment failed for {axiom:?}: {e}"));
        assert!(entailed, "expected entailed: {axiom:?}");
    }

    let conclusion = Ontology::from_file(&not_entailed, None).expect("load negative conclusion");
    let mut all_entailed = true;
    for axiom in conclusion.axioms() {
        let mut stats = ReasoningStatistics::new();
        let entailed = reasoner
            .check_entailment(axiom, &onto_ref, &mut stats)
            .unwrap_or_else(|e| panic!("check_entailment failed for {axiom:?}: {e}"));
        if !entailed {
            all_entailed = false;
        }
    }
    assert!(
        !all_entailed,
        "B ⊑ A must NOT be entailed by a premise asserting A ⊑ B"
    );
}

// ── 2./3. CLI exit codes ─────────────────────────────────────────────────────

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_oxidowl"))
        .args(args)
        .output()
        .expect("oxidowl CLI must run")
}

fn describe(output: &std::process::Output) -> String {
    format!(
        "\n--- status: {:?}\n--- stdout:\n{}\n--- stderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn write_cli_fixtures(dir: &Path) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
    let premise = write_ofn(
        dir,
        "premise.ofn",
        "http://example.org/premise",
        &premise_body(),
    );
    let entailed = write_ofn(
        dir,
        "entailed.ofn",
        "http://example.org/conclusion-entailed",
        &premise_body(),
    );
    let not_entailed = write_ofn(
        dir,
        "not_entailed.ofn",
        "http://example.org/conclusion-not-entailed",
        &format!("{DECLS}  SubClassOf({CLASS_B} {CLASS_A})\n"),
    );
    (premise, entailed, not_entailed)
}

#[test]
fn cli_subcommand_check_entailment_exit_codes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (premise, entailed, not_entailed) = write_cli_fixtures(dir.path());

    // Premise ⊨ identical conclusion → exit 0, per-axiom "Entailed" lines.
    let out = run_cli(&[
        "check-entailment",
        "--premise",
        premise.to_str().expect("utf8"),
        "--conclusion",
        entailed.to_str().expect("utf8"),
    ]);
    assert!(out.status.success(), "expected exit 0{}", describe(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Entailed:"),
        "per-axiom verdict missing{stdout}"
    );
    assert!(
        !stdout.contains("Not entailed:"),
        "unexpected non-entailed verdict{stdout}"
    );

    // B ⊑ A is not entailed → exit 1, per-axiom "Not entailed" line.
    let out = run_cli(&[
        "check-entailment",
        "--premise",
        premise.to_str().expect("utf8"),
        "--conclusion",
        not_entailed.to_str().expect("utf8"),
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 for a non-entailed conclusion{}",
        describe(&out)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Not entailed:"),
        "per-axiom verdict missing{stdout}"
    );
}

#[test]
fn cli_flag_check_entailment_exit_codes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let (premise, entailed, not_entailed) = write_cli_fixtures(dir.path());

    // Flag mode: `--check-entailment <premise> <conclusion>` → exit 0.
    let out = run_cli(&[
        "--check-entailment",
        premise.to_str().expect("utf8"),
        entailed.to_str().expect("utf8"),
    ]);
    assert!(out.status.success(), "expected exit 0{}", describe(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Entailment holds"),
        "summary missing{stdout}"
    );

    // Not entailed → exit 1.
    let out = run_cli(&[
        "--check-entailment",
        premise.to_str().expect("utf8"),
        not_entailed.to_str().expect("utf8"),
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "expected exit 1 for a non-entailed conclusion{}",
        describe(&out)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Entailment does NOT hold"),
        "summary missing{stdout}"
    );
}

// ── 4. B3: cross-file entailment with prefixed fixtures ──────────────────────

/// Write a prefixed functional-syntax fixture: an explicit
/// `Prefix(:=<http://example.org/default#>)` **plus** a *different* ontology
/// IRI.  Before B3 the `Ontology(<…>)` declaration overwrote the `""`
/// prefix, so each file expanded `:A` against its own ontology IRI and the
/// premise/conclusion IRIs never matched across files.
fn write_prefixed_ofn(
    dir: &Path,
    name: &str,
    ontology_iri: &str,
    body: &str,
) -> std::path::PathBuf {
    let path = dir.join(name);
    let content =
        format!("Prefix(:=<http://example.org/default#>)\nOntology(<{ontology_iri}>\n{body})\n");
    std::fs::write(&path, content).expect("prefixed fixture write");
    path
}

const PREF_CLASS_A: &str = ":A";
const PREF_CLASS_B: &str = ":B";

#[test]
fn cli_check_entailment_with_prefixed_cross_file_fixtures() {
    let dir = tempfile::tempdir().expect("tempdir");

    let premise = write_prefixed_ofn(
        dir.path(),
        "prefixed_premise.ofn",
        "http://example.org/prefixed-premise",
        &format!(
            "  Declaration(Class({PREF_CLASS_A}))\n  Declaration(Class({PREF_CLASS_B}))\n  \
             SubClassOf({PREF_CLASS_A} {PREF_CLASS_B})\n"
        ),
    );
    // Same prefixed axioms, different ontology IRI → must be entailed.
    let entailed = write_prefixed_ofn(
        dir.path(),
        "prefixed_entailed.ofn",
        "http://example.org/prefixed-conclusion-entailed",
        &format!(
            "  Declaration(Class({PREF_CLASS_A}))\n  Declaration(Class({PREF_CLASS_B}))\n  \
             SubClassOf({PREF_CLASS_A} {PREF_CLASS_B})\n"
        ),
    );
    // Reversed subclass axiom under the same prefixes → must NOT be entailed.
    let not_entailed = write_prefixed_ofn(
        dir.path(),
        "prefixed_not_entailed.ofn",
        "http://example.org/prefixed-conclusion-not-entailed",
        &format!(
            "  Declaration(Class({PREF_CLASS_A}))\n  Declaration(Class({PREF_CLASS_B}))\n  \
             SubClassOf({PREF_CLASS_B} {PREF_CLASS_A})\n"
        ),
    );

    let out = run_cli(&[
        "check-entailment",
        "--premise",
        premise.to_str().expect("utf8"),
        "--conclusion",
        entailed.to_str().expect("utf8"),
    ]);
    assert!(
        out.status.success(),
        "identical prefixed conclusion must be entailed (B3: the explicit \
         default prefix must survive `Ontology(<…>)` across files){}",
        describe(&out)
    );

    let out = run_cli(&[
        "check-entailment",
        "--premise",
        premise.to_str().expect("utf8"),
        "--conclusion",
        not_entailed.to_str().expect("utf8"),
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "reversed prefixed conclusion must NOT be entailed{}",
        describe(&out)
    );
}

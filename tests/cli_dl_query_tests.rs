//! B2 — CLI `query` JSON output must carry *every* answer the engine produced.
//!
//! `execute_dl_query` used to serialise `result.classes` only: an
//! `instances:` (or `satisfiable:`) query therefore fell into the
//! `"result": "No results"` branch even when the engine had found
//! individuals.  These tests drive the real binary end-to-end.

use std::process::Command;

/// A tiny ABox ontology whose classes live in the ontology IRI namespace, so
/// the CLI's auto-detected default namespace (`<ontology IRI>#`) resolves
/// `Person` to the declared class.
const FIXTURE: &str = concat!(
    "Ontology(<http://example.org/onto#>)\n",
    "  Declaration(Class(:Person))\n",
    "  Declaration(NamedIndividual(:alice))\n",
    "  Declaration(NamedIndividual(:bob))\n",
    "  ClassAssertion(:Person :alice)\n",
    "  ClassAssertion(:Person :bob)\n",
);

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

/// The CLI's JSON line (logs share stdout, so pick the line that parses as
/// JSON and carries the `query` field).
fn json_line(stdout: &str) -> String {
    stdout
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('{') && line.contains("\"query\""))
        .find(|line| serde_json::from_str::<serde_json::Value>(line).is_ok())
        .expect("CLI must print a JSON query result line")
        .to_string()
}

fn write_fixture(dir: &std::path::Path) -> std::path::PathBuf {
    let path = dir.join("abox.ofn");
    std::fs::write(&path, FIXTURE).expect("fixture write");
    path
}

#[test]
fn cli_query_instances_json_includes_the_individuals() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = write_fixture(dir.path());

    let out = run_cli(&[
        "query",
        "-i",
        fixture.to_str().expect("utf8"),
        "-q",
        "instances:Person",
    ]);
    assert!(out.status.success(), "query must succeed{}", describe(&out));

    let line = json_line(&String::from_utf8_lossy(&out.stdout));
    let json: serde_json::Value =
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("invalid JSON {line}: {e}"));

    assert_eq!(json["query"], "instances:Person");
    let mut iris: Vec<String> = json["instances"]
        .as_array()
        .unwrap_or_else(|| panic!("instances must be an array: {json}"))
        .iter()
        .map(|entry| {
            entry["iri"]
                .as_str()
                .unwrap_or_else(|| panic!("every instance entry carries an iri: {json}"))
                .to_string()
        })
        .collect();
    iris.sort();
    assert_eq!(
        iris,
        vec![
            "http://example.org/onto#alice".to_string(),
            "http://example.org/onto#bob".to_string()
        ],
        "the instance answer must be serialised: {json}"
    );
    assert!(
        json.get("result").is_none(),
        "an instance answer must not be reported as 'No results': {json}"
    );
    assert!(json.get("execution_time").is_some(), "{json}");
}

#[test]
fn cli_query_output_file_carries_the_individuals() {
    let dir = tempfile::tempdir().expect("tempdir");
    let fixture = write_fixture(dir.path());

    // `-o` writes the JSON document to a file (the `query` subcommand always
    // uses the JSON formatter): the saved document must carry the individuals
    // too, not just the stdout copy.
    let out_file = dir.path().join("result.json");
    let out = run_cli(&[
        "query",
        "-i",
        fixture.to_str().expect("utf8"),
        "-q",
        "instances:Person",
        "-o",
        out_file.to_str().expect("utf8"),
    ]);
    assert!(out.status.success(), "query must succeed{}", describe(&out));

    let saved = std::fs::read_to_string(&out_file).expect("query output file");
    let json: serde_json::Value =
        serde_json::from_str(&saved).unwrap_or_else(|e| panic!("invalid JSON {saved}: {e}"));
    assert!(
        json["instances"].as_array().is_some_and(|a| a.len() == 2),
        "the saved document must carry both individuals: {json}"
    );
}

#[test]
fn cli_query_classes_json_keeps_the_class_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    // A TBox with a told subsumption so the `classes:` answer is non-trivial.
    let fixture = dir.path().join("tbox.ofn");
    std::fs::write(
        &fixture,
        concat!(
            "Ontology(<http://example.org/onto#>)\n",
            "  Declaration(Class(:A))\n",
            "  Declaration(Class(:B))\n",
            "  SubClassOf(:A :B)\n",
        ),
    )
    .expect("fixture write");

    let out = run_cli(&[
        "query",
        "-i",
        fixture.to_str().expect("utf8"),
        "-q",
        "subclasses:<http://example.org/onto#B>",
    ]);
    assert!(out.status.success(), "query must succeed{}", describe(&out));

    let line = json_line(&String::from_utf8_lossy(&out.stdout));
    let json: serde_json::Value =
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("invalid JSON {line}: {e}"));
    let classes = json["classes"]
        .as_array()
        .unwrap_or_else(|| panic!("classes must still be an array: {json}"));
    assert!(
        classes.iter().any(|c| c.as_str() == Some("A")),
        "A must be a subclass of B: {json}"
    );
    assert!(
        json.get("result").is_none(),
        "no 'No results' marker: {json}"
    );
}

//! ORE-2015 integration tests
//!
//! Tests loading of OWL Functional Syntax files from the ORE-2015 benchmark.
//! These files use the `.owl` extension but contain OWL Functional Syntax.

use oxidowl::parsers::parse_file_auto;

const ORE_TEST_DIR: &str = "ontologies-test";

/// Helper: load an ORE ontology file and return axiom count, or panic with a message.
fn load_ore_file(name: &str) -> usize {
    let path = format!("{ORE_TEST_DIR}/{name}");
    match parse_file_auto(&path) {
        Ok(onto) => onto.axioms.len(),
        Err(e) => panic!("Failed to load {name}: {e}"),
    }
}

#[test]
fn test_ore_14543_loads() {
    // GO ontology approximation — large file (165K lines, ~139K axioms)
    let count = load_ore_file("ore_ont_14543.owl");
    assert!(
        count > 10_000,
        "Expected >10K axioms from ore_ont_14543.owl, got {count}"
    );
}

#[test]
fn test_ore_10004_loads() {
    let count = load_ore_file("ore_ont_10004.owl");
    assert!(count > 0, "Expected axioms from ore_ont_10004.owl, got 0");
}

#[test]
fn test_ore_10006_loads() {
    let count = load_ore_file("ore_ont_10006.owl");
    assert!(count > 0, "Expected axioms from ore_ont_10006.owl, got 0");
}

#[test]
fn test_ore_10008_loads() {
    let count = load_ore_file("ore_ont_10008.owl");
    assert!(count > 0, "Expected axioms from ore_ont_10008.owl, got 0");
}

#[test]
fn test_ore_10009_loads() {
    let count = load_ore_file("ore_ont_10009.owl");
    assert!(count > 0, "Expected axioms from ore_ont_10009.owl, got 0");
}

/// Batch test: try to load 20 ORE-2015 files and report pass/fail counts.
/// This serves as a smoke test for the format detection and functional parser.
#[test]
fn test_ore_batch_loading() {
    use std::fs;

    let dir = ORE_TEST_DIR;
    let mut passed = 0usize;
    let mut failed = 0usize;
    let mut failures: Vec<String> = Vec::new();

    let mut entries: Vec<_> = fs::read_dir(dir)
        .expect("ORE test dir should exist")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map_or(false, |e| e == "owl"))
        .collect();
    entries.sort();
    let entries = &entries[..entries.len().min(20)];

    for path in entries {
        let path_str = path.to_str().unwrap();
        match parse_file_auto(path_str) {
            Ok(onto) => {
                assert!(
                    onto.axioms.len() > 0,
                    "{} loaded but has 0 axioms",
                    path.file_name().unwrap().to_str().unwrap()
                );
                passed += 1;
            }
            Err(e) => {
                let name = path.file_name().unwrap().to_str().unwrap().to_string();
                failures.push(format!("{name}: {e}"));
                failed += 1;
            }
        }
    }

    if failed > 0 {
        panic!(
            "{}/{} ORE files failed to load:\n{}",
            failed,
            passed + failed,
            failures.join("\n")
        );
    }
}

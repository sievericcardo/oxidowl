# Differential Parity Corpus (Workstream C2)

This directory holds the golden-output corpus for oxidowl's **differential
testing harness** (`tests/differential_parity_tests.rs`).  Each fixture is a
declarative JSON document that pairs an ontology (inline Functional Syntax /
Turtle / RDF/XML / OWL/XML, or a path to an `ontologies-test/` file) with the
reasoning verdicts a conformant OWL 2 DL reasoner — **HermiT / Konclude, i.e.
the OWL 2 Direct Semantics** — should return.

## JSON schema

```json
{
  "id":        "unique fixture id (== file name stem)",
  "source":    "Functional | Turtle | RDFXML | OWLXML | NTriples | Path",
  "ontology":  "inline ontology string (for inline sources) or file path (for Path)",
  "base":      "base IRI used to resolve short names :A → <base>A (optional)",
  "axioms":    {"min": N, "max": M},            // optional load sanity check
  "expected":  {
    "consistent":     true|false,
    "satisfiable":    {"A": true, "B": false},
    "subsumptions":   [["A","B"], ["B","C"]],          // must be entailed
    "nonSubsumptions":[["C","A"]],                      // must NOT be entailed
    "equivalences":   [["A","B"]],                      // mutual subsumption
    "instances":      {"A": ["a","b"]},                 // set comparison
    "types":          {"a": ["A","B"]},                 // subset of get_types
    "entailed":       ["SubClassOf(:A :B)", "ClassAssertion(:A :a)"],
    "notEntailed":    ["SubClassOf(:C :A)"],
    "sameAs":         [["a","b"]],
    "differentFrom":  [["a","c"]],
    "profiles":       {"el": true, "ql": true, "rl": true, "dl": true, "full": true}
  },
  "rationale":  "why each golden verdict holds under OWL 2 Direct Semantics"
}
```

Notes on the schema:

- Every `expected` field is **optional** — only the checks present in a fixture
  are executed.  This keeps fixtures small and lets a fixture target exactly the
  reasoning surface it was written for.
- Short class names (`A`, `:A`) resolve to `<base>A`; full IRIs `<http://…>`
  are accepted too; `owl:Thing` / `owl:Nothing` are special-cased.
- `entailed` / `notEntailed` entries are **Functional-Syntax axiom bodies**
  (e.g. `SubClassOf(:A :B)`) written against the `:` prefix bound to `base`.
  The harness wraps each body in `Prefix(:=<base>) Ontology(<base> …)` and
  checks every parsed axiom via the semantic entailment path.
- `instances` is an exact **set comparison**; `types` is a **subset** check
  (the reasoner may also report `owl:Thing` and complex expressions).
- `profiles` keys: `el`, `ql`, `rl`, `dl`, `full`.

## (a) Run the harness

Strict mode — **FAILS on any divergence** from golden (this is the point):

```bash
cargo test differential_parity
```

Report-only mode — prints the full per-fixture comparison (expected vs actual)
without failing; use while authoring:

```bash
cargo test differential_parity_tests::report_only -- --ignored --nocapture
```

Known-divergence fixtures are marked `#[ignore]` in the harness with the
precise production gap they expose.  `cargo test differential_parity -- --ignored`
runs only those, so the divergence surface is always one command away.

## (b) Promote outputs to golden

1. Run the golden generator — it prints oxidowl's **actual** verdicts in the
   golden JSON shape:

   ```bash
   cargo test differential_parity_tests::generate_golden_json -- --ignored --nocapture
   ```

2. For each fixture, **review the printed verdicts against HermiT/Konclude or
   the OWL 2 Direct Semantics** (never promote blindly — oxidowl may be the one
   that is wrong, which is exactly what the harness is for).
3. Edit the fixture's `expected` block (and `rationale`) to the reviewed values.
4. Re-run strict mode; un-`#[ignore]` the fixture once it passes.

## (c) Wiring real HermiT / Konclude binaries (`DIFF_REASONER`)

The golden values are authored from first principles because external Java
reasoners are not guaranteed to be installed in CI.  To validate them against a
real reasoner, provide a binary via the environment:

```bash
DIFF_REASONER=/path/to/reasoner-driver cargo test differential_parity_tests::report_only -- --ignored --nocapture
```

**Driver interface** (a small shell/Python wrapper around the Java reasoner):

- Input:  an ontology file in one of the formats the fixture uses
  (`Functional`, `Turtle`, `RDF/XML`, `OWL/XML`, `NTriples`).
- Output: JSON on stdout with the **same schema** as the `expected` block,
  e.g.:

  ```json
  {
    "consistent": true,
    "satisfiable": {"A": true, "B": false},
    "subsumptions": [["A","B"]],
    "instances": {"A": ["a"]},
    "profiles": {"el": true, "rl": true}
  }
  ```

  Short names resolve against the same `base` the fixture declares.

A driver for **HermiT** is typically ~40 lines of Java using the OWLAPI
`OWLReasoner` (`isConsistent`, `isSatisfiable`, `isEntailed`,
`getInstances`, and `OWLProfile` checks).  A **Konclude** driver can use its
REST/OWLlink endpoint.  When a driver is present the harness should compare
oxidowl's verdicts against both the hand-authored golden *and* the external
reasoner, and report three-way divergences (golden vs oxidowl vs external).
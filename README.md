# <img src="oxidowl.webp" alt="OxidOWL Logo generated with Google Gemini" width="35"/> OxidOWL

A high-performance Description Logic reasoner for OWL 2 DL ontologies, implemented in Rust with tableau algorithms, parallel computation, and horned-owl integration.

[![CI](https://github.com/sievericcardo/oxidowl/actions/workflows/ci.yml/badge.svg)](https://github.com/sievericcardo/oxidowl/actions/workflows/ci.yml)
[![Release](https://github.com/sievericcardo/oxidowl/actions/workflows/release.yml/badge.svg)](https://github.com/sievericcardo/oxidowl/actions/workflows/release.yml)
[![License: LGPL-3.0-or-later](https://img.shields.io/badge/License-LGPL%20v3%2B-blue.svg)](LICENSE)
[![Rust 1.88+](https://img.shields.io/badge/rust-1.88%2B-orange.svg)](https://www.rust-lang.org)
[![Version 2.1.0](https://img.shields.io/badge/version-2.1.0-green.svg)](Cargo.toml)

## Overview

OxidOWL is a tableau-based reasoner for the Description Logic SROIQV(D), supporting most of OWL 2 DL. It is built on [horned-owl](https://github.com/phillord/horned-owl) and was inspired by [HermiT](https://github.com/owlcs/hermit-reasoner) and [Konclude](https://github.com/konclude/Konclude).

**Current status — v2.1.0** (Rust edition 2024, MSRV 1.88):

- **2173 tests passing / 0 failed** across 96 test targets (`cargo test --workspace --no-fail-fast`); `cargo fmt --all -- --check` and `cargo clippy --lib --bins -- -D warnings` are clean.
- Feature builds verified: default, `server`, `server,sparql`, `ml`, `profiling`, `all-features`, and `cargo doc --no-deps`. Honest scope: reasoning core and CLI are stable; servers, OWLlink, distributed reasoning and ML are experimental — see [Known Limitations](#known-limitations--experimental-features).

## Table of Contents

- [ OxidOWL](#-oxidowl)
  - [Overview](#overview)
  - [Table of Contents](#table-of-contents)
  - [What's New in 2.1.0](#whats-new-in-210)
  - [Feature Matrix](#feature-matrix)
  - [Installation](#installation)
    - [Prerequisites](#prerequisites)
    - [From Source](#from-source)
    - [As a Library](#as-a-library)
    - [Cargo Feature Flags](#cargo-feature-flags)
  - [Quick Start](#quick-start)
    - [Command Line](#command-line)
    - [Resource Caps](#resource-caps)
  - [Library Usage](#library-usage)
  - [Algorithm Selection: Traditional vs Hypertableau](#algorithm-selection-traditional-vs-hypertableau)
  - [Performance](#performance)
  - [Supported Formats](#supported-formats)
  - [Testing](#testing)
  - [CI/CD](#cicd)
  - [Known Limitations / Experimental Features](#known-limitations--experimental-features)
  - [Contributing](#contributing)
  - [Roadmap](#roadmap)
  - [License](#license)
  - [Acknowledgments](#acknowledgments)
  - [Support](#support)

## What's New in 2.1.0

- **OWLReasoner surface completed**: type/relationship queries, same/different individuals, `precompute_inferences`, Declaration/AnnotationAssertion/Rule entailment, snapshot-backed property values (inverse, subproperty, chain, derived).
- **Recursive `owl:imports` closure** (`file://` + HTTP, cycle-safe, best-effort) resolved at load time.
- **DL-safe SWRL forward-chaining materializer** wired into ontology install (rules materialize before the first query) with an explicit `materialize_swrl_rules()` entry point.
- **ABox realisation fixes**: defined-subclass inference, complex GCIs, intersection decomposition, and a recursion guard that fixed a mutual-recursion hang.
- **Inferred object/data property hierarchies** (transitive closure, equivalents, inverses, top/bottom, direct filtering).
- **`check-entailment` CLI** (premise ⊨ conclusion, correct exit codes), anonymous individuals (`_:id`) on the query path, owl:Thing/owl:Nothing fast paths, and real caching/statistics (snapshot, consistency, classification caches).

**Correctness fixes**:

- CLI `query` JSON now always includes `instances`, `classes`, and `satisfiable` fields.
- Classification / full-reasoning hierarchy is complete under `target_profile=Auto` (the EL path seeds all signature classes plus owl:Thing and falls back to DL when incomplete).
- Functional-syntax default prefix `Prefix(:=…)` is preserved; `Auto` format detection now uses the shared parser (`.rdf`, `.jsonld`, `.n3`, `.html`, `.hdt`, `.brdf` all load correctly).

**Performance** (see [Performance](#performance)): greenhouse SMOL command went from a 300 s/900 s timeout to **9.33 s**; `tosemServices` from a hang to **2.85 s**; `get_instances` first query **14.6 ms**, cached **0.2–1.8 ms**.

## Feature Matrix

| Feature | Status | Notes |
|---|---|---|
| Tableau reasoner (SROIQV(D)) | **Stable** | Consistency, satisfiability, classification, realisation, instance queries, entailment |
| CLI (18 subcommands + HermiT-style flags) | **Stable** | See [Quick Start](#quick-start); there is **no** `swrl` or `benchmark` subcommand (SWRL materializes at load time; benchmarks run via `cargo bench`) |
| Recursive `owl:imports` | **Stable** | `file://` + HTTP, cycle-safe, best-effort (failed import warns and continues) |
| SWRL materialization | **Stable (DL-safe subset)** | Forward chaining at load time / `materialize_swrl_rules()`; see [limitations](#known-limitations--experimental-features) |
| 20 input formats + `Auto` detection | **Stable** | See [Supported Formats](#supported-formats) |
| DL query engine (Manchester syntax) | **Stable** | Union queries, DisjointUnion detection, JSON output with `instances`/`classes`/`satisfiable` |
| OWL 2 profiles EL / QL / RL + validators | **Stable** | EL classification is polynomial with automatic DL fallback |
| SHACL Core + SHACL-SPARQL | **Stable (library)** | Backed by the embedded Oxigraph store |
| RDF-star / RDF 1.2 support | **Stable** | Quoted triples, SPARQL-star, directional literals, reification interop |
| Caching & statistics | **Stable** | Snapshot, consistency and classification caches; real reasoning statistics; resource caps (`--timeout`, `--max-memory-mb`) fail gracefully instead of OOM-killing |
| Formal verification (Kani) | **Stable (harnesses)** | 103 `#[kani::proof]` harnesses across `proofs::{core,ontology,owl_axioms,owl_expressions,owl_properties,rdf,swrl}`; `cargo kani list` OK and a representative harness verifies (`proofs::core::dep_set_new_is_empty`); full Kani verification is manual / not in CI (CBMC cost) |
| Distributed reasoning | **Library-only / experimental** | Discovery stubs; no CLI/server surface |
| ML-enhanced queries (Candle) | **Library-only / experimental** | Heuristic optimizer; no CLI/server surface |
| REST / OWLlink / SPARQL HTTP servers | **Experimental** | `server` feature; `/explain`, `/entailment`, `POST /ontology` are stubs; server mode ignores CLI input files; SPARQL HTTP endpoint requires `sparql` and is form-encoded |
| OWLlink file processing | **Experimental (broken)** | Does not load the ontology / deserialize requests — not a working protocol |
| Incremental reasoning | **Limitation** | Correct but full recompute (A9) — not incremental |
| Explanations | **Reserved** | The `explanations` flag is currently a no-op (explanations compile unconditionally; REST explain is a stub) |
| `--no-default-features` builds | **Unsupported** | Does not compile; use the default feature set |

## Installation

For Docker see [DOCKER.md](DOCKER.md); for Podman see [PODMAN.md](PODMAN.md).

### Prerequisites

- **Rust 1.88 or newer** (MSRV; edition 2024)
- **C++17-capable compiler and CMake** — Oxigraph's `rocksdb` feature compiles RocksDB from source (used by the default `sparql-store` feature)

**MSRV note:** the default features — and therefore the library, the CLI, and the opt-in `server` and `sparql` features — are MSRV **1.88** (that is what the CI `msrv` job verifies: `cargo check --workspace` and `cargo check --workspace --features server,sparql`); only the `ml` feature / `--all-features` need a newer stable toolchain, because candle-core uses a NEON intrinsic stabilized after 1.88 (`stdarch_neon_f16`).

```bash
# Ubuntu / Debian
sudo apt-get update && sudo apt-get install -y build-essential cmake clang libclang-dev
# Fedora / RHEL / Rocky
sudo dnf install gcc-c++ cmake clang clang-devel
# macOS
xcode-select --install && brew install cmake
# Windows: install Visual Studio Build Tools ("Desktop development with C++")
# workload + CMake — https://visualstudio.microsoft.com/visual-cpp-build-tools/
```

### From Source

```bash
git clone https://github.com/sievericcardo/oxidowl.git
cd oxidowl
cargo build --release
```

### As a Library

```toml
[dependencies]
oxidowl = "2.1"

# Or with specific (opt-in) features:
oxidowl = { version = "2.1", features = ["server", "sparql", "ml"] }
```

### Cargo Feature Flags

| Feature | Default | Description |
|---|---|---|
| `parallel` | ✅ | Rayon-based multi-threaded reasoning |
| `cache` | ✅ | DashMap-backed concurrent caches (incl. IRI interning) |
| `high-performance` | ✅ | mimalloc high-performance allocator |
| `http-imports` | ✅ | HTTP ontology import fetching (reqwest) |
| `sparql-store` | ✅ | In-process Oxigraph SPARQL store (pulls in RocksDB/C++) |
| `server` | ❌ | REST / OWLlink / SPARQL HTTP servers (**experimental**) |
| `sparql` | ❌ | HTTP SPARQL server endpoint (requires `server`) |
| `ml` | ❌ | Candle-backed ML query heuristics (library-only; needs a newer stable toolchain than MSRV 1.88 — candle-core uses a post-1.88 NEON intrinsic) |
| `zip-imports` | ❌ | Zip-archive ontology import support |
| `debug` | ❌ | Reserved debug toggle (currently a no-op) |
| `profiling` | ❌ | pprof flamegraph + dhat heap profiling |
| `kani` | ❌ | Kani formal-verification harnesses (103 `#[kani::proof]` proofs; manual, not in CI) |

> `explanations` is a **reserved, currently no-op** flag and is not listed above. Building with `--no-default-features` is **unsupported** (does not compile).

## Quick Start

### Command Line

```bash
oxidowl consistency -i ontologies-test/greenhouse.owx            # consistency (HermiT-style: -k also works)
oxidowl classification -i ontologies-test/greenhouse.owx -P -o hierarchy.json   # classification + pretty hierarchy
oxidowl query -i ontologies-test/greenhouse.owx -q "Pump" -n "http://www.smolang.org/greenhouseDT#"  # JSON: instances/classes/satisfiable
oxidowl realisation -i ontologies-test/greenhouse.owx -o realisation.json       # aliases: instantiation, realization
oxidowl check-entailment --premise premise.owx --conclusion conclusion.owx      # premise ⊨ conclusion, exit 0 iff entailed
oxidowl full-reasoning ontologies-test/greenhouse.owx            # load → consistency → classification → property hierarchies
oxidowl consistency -i ontology-with-imports.ofn                 # recursive owl:imports (file:// + HTTP) resolved at load
```

The `full-reasoning` suite and every load automatically forward-chain SWRL rules present in the ontology (see [Library Usage](#library-usage) for the explicit entry point).

Other subcommands: `load`, `satisfiability`, `subclasses`, `superclasses`, `equivalent-classes`, `unsatisfiable-classes`, `classify-object-properties`, `classify-data-properties`, `print-prefixes`, `dump-clauses`, `owl-link-file`, `sparql-file`. Top-level HermiT-style flags: `-l -c -O -D -k -P -d -s -S -e -U`, `--check-entailment`, `--print-prefixes`, `--dump-clauses`, plus `--enable-server` / `--server-port` / `--server-bind` and the OWLlink/SPARQL server flags.

### Resource Caps

| Flag | Config key | Semantics |
|---|---|---|
| `--timeout <SECONDS>` | `reasoning.timeout` | Abort reasoning after N seconds (default: no timeout) |
| `--max-memory-mb <MB>` | `reasoning.max_memory_mb` | Abort with `Memory limit exceeded: …` (exit 1) once process RSS exceeds MB; `0`/absent = unlimited; config default 4096 |

The memory cap is enforced in-process by an RSS guard inside the tableau expansion loop, the classification/saturation loops and the EL worklist, so a pathological ontology fails gracefully instead of being OOM-killed.

## Library Usage

```rust
use oxidowl::{
    core::reasoner::statistics::ReasoningStatistics,
    ontology::{Class, ClassExpression, IRI, Ontology, OntologyFormat},
    Reasoner, ReasonerConfig, Result,
};

fn main() -> Result<()> {
    let mut reasoner = Reasoner::new(ReasonerConfig::default())?;
    // Auto-detects the format, resolves the recursive owl:imports closure,
    // and forward-chains any SWRL rules before the ontology is installed.
    reasoner.load_ontology_from_file("ontologies-test/greenhouse.owx", OntologyFormat::Auto)?;

    // Consistency (memoised per ontology version)
    println!("consistent: {}", reasoner.is_consistent()?);
    // Classification (polynomial EL path with automatic DL fallback)
    let classification = reasoner.classify()?;
    println!("{} classes classified", classification.hierarchy.len());

    // Instance queries (single-tableau strategy + snapshot cache)
    let pump = ClassExpression::Class(Class::new(IRI::new(
        "http://www.smolang.org/greenhouseDT#Pump",
    )));
    for individual in reasoner.get_instances(&pump, false)? {
        println!("instance: {individual:?}");
    }

    // SWRL: rules are materialized at load time; call explicitly after add_axiom()
    let derived = reasoner.materialize_swrl_rules()?;
    println!("SWRL materialization added {derived} axiom(s)");

    // Entailment: premise ⊨ conclusion, checked axiom by axiom
    let conclusion = Ontology::from_file("conclusion.owx", None)?;
    let premise_ref = reasoner.get_ontology().cloned().expect("ontology loaded");
    let mut stats = ReasoningStatistics::new();
    for axiom in conclusion.axioms() {
        let entailed = reasoner.check_entailment(axiom, &premise_ref, &mut stats)?;
        println!("{axiom:?} entailed: {entailed}");
    }
    Ok(())
}
```

Runnable examples: [examples/library_usage.rs](examples/library_usage.rs), [examples/dl_query_example.rs](examples/dl_query_example.rs), [examples/server_example.rs](examples/server_example.rs), [examples/sparql_update_example.rs](examples/sparql_update_example.rs). More in [docs/EXAMPLES.md](docs/EXAMPLES.md).

## Algorithm Selection: Traditional vs Hypertableau

Oxidowl ships two tableau engines. Select via `config.reasoning.tableau_algorithm`:

```rust
use oxidowl::config::{ReasonerConfig, TableauAlgorithm};

let mut config = ReasonerConfig::default();
config.reasoning.tableau_algorithm = TableauAlgorithm::Hypertableau; // disjointness-heavy
// config.reasoning.tableau_algorithm = TableauAlgorithm::Traditional; // safe default
```

| Ontology characteristic | Recommended | Indicative speedup* |
|---|---|---|
| Many disjoint classes (>10) | Hypertableau | ~3–9× |
| Complex class expressions (<50) | Hypertableau | ~8–13% |
| Large linear taxonomy (>50 classes) | Traditional | ~2–5× |
| Many equivalent classes (>10) | Traditional | ~2–7× |
| General purpose / unknown | Traditional | Safer default |

\* Figures are indicative measurements from earlier benchmark runs, not guarantees — measure on your hardware.

## Performance

Wall-clock measurements from the 2026-11-15 realtime wave **on the development machine** — measure on your hardware:

| Benchmark | Before | After |
|---|---|---|
| Greenhouse SMOL command | Timeout at 300 s / 900 s | **9.33 s** (4× Healthy, 0 ERROR, 3 moist + 1 thirsty, 8 "Plant is Healthy") |
| `tosemServices` | Hang | **2.85 s** (17 ISSA + 3 SELECT, 4 states exact) |
| `oxidowl get_instances` probe | — | First query **14.6 ms**; cached **0.2–1.8 ms** |

Techniques used: single-tableau `get_instances` + snapshot cache; blocking signature cache / label-version skip / root+nominal blocker exclusion; FNV-1a hashing + two-pointer subset checks; changed-node clause checks; functional-merge gating; fast clause LRU; fixed-pattern datatype regex cache; lazy consumer reasoner init.

## Supported Formats

**20 concrete formats + `Auto`** (extension table + content sniffing through the shared parser):

| Family | Formats |
|---|---|
| OWL syntaxes | Functional Syntax (`.ofn`/`.owx`), OWL/XML (`.owl`), Manchester (`.omn`), DL (`.dl`), KRSS (`.krss`), KRSS2, OBO (`.obo`), LaTeX (write-only) |
| RDF 1.1 / 1.2 | RDF/XML (`.rdf`), Turtle (`.ttl`), N-Triples (`.nt`), N-Quads (`.nq`), N3 (`.n3`), TriG, TriX, JSON-LD (`.jsonld`), RDF/JSON, RDFa/HTML (`.html`), Binary RDF (`.brdf`), HDT (`.hdt`) |
| Detection | `Auto` (default): B5 formats are library-loadable and auto-detected; explicit `-f` on the CLI exposes the core formats (OWL/XML, Functional, RDF/XML, Turtle, N-Triples, Manchester, Auto) |

## Testing

- **2173 tests passing, 0 failed, 10 ignored** (3 intentional tests + 7 doctests) across **96 test targets** (`cargo test --workspace --no-fail-fast`).
- `cargo fmt --all -- --check` clean; `cargo clippy --lib --bins -- -D warnings` clean (0 warnings).

Key suites: W3C OWL 2 conformance (`owl2_conformance_suite`, `owl2_conformance_extended_tests`), SWRL (`swrl_reasoning_tests`, `swrl_extended_tests`), imports (`imports_closure_tests`, `import_resolution_tests`), entailment (`check_entailment_tests`), instance queries (`instance_query_phase_a_tests`), plus RDF-star, SHACL, profile and format suites (JSON-LD, N3, RDFa, HDT, Binary RDF).

```bash
cargo test --workspace --no-fail-fast   # full suite (what CI runs)
cargo fmt --all -- --check              # formatting gate
cargo clippy --lib --bins -- -D warnings  # lint gate
cargo doc --no-deps                     # docs build
cargo check --all-features              # feature-matrix builds (CI also checks server,sparql / ml / profiling)
cargo bench                             # criterion benchmarks
```

Kani formal verification (103 `#[kani::proof]` harnesses; run manually, not part of CI):

```bash
cargo kani list
cargo kani --harness proofs::core::dep_set_new_is_empty
```

## CI/CD

**CI** (`.github/workflows/ci.yml`) runs on every push/PR:

- `cargo fmt --check` and `cargo clippy --lib --bins -- -D warnings`
- Full test suite on **Ubuntu and macOS**
- Conformance / parity / SWRL / imports suites
- Feature-matrix checks: `server`, `server,sparql`, `ml`, `profiling`, `all-features`
- `cargo doc --no-deps` and **MSRV 1.88** verification

**Release** (`.github/workflows/release.yml`): pushing a `v*` tag publishes a GitHub Release with binaries for **Linux x86_64, macOS x86_64/aarch64, Windows x86_64** (built with `--features server,sparql`) plus a **GHCR Docker image**.

## Known Limitations / Experimental Features

- **Incremental reasoning is not incremental**: changes trigger a full recompute (A9). Correct, but not differential.
- **OWLlink (file + HTTP) is experimental/broken**: the ontology is not loaded and requests are not deserialized — do not rely on it as a protocol.
- **REST server is experimental/partial**: `/explain`, `/entailment` and `POST /ontology` are stubs; server mode currently ignores CLI input files; the SPARQL HTTP endpoint requires the `sparql` feature and is form-encoded.
- **Distributed reasoning and ML are library-only/experimental**: discovery stubs, ML heuristics, no CLI/server surface. The `explanations` flag is a **reserved no-op** (explanations compile unconditionally; REST explain is a stub).
- **Full Kani verification is not in CI**: 103 `#[kani::proof]` harnesses compile and can be verified manually (`cargo kani list` / `cargo kani --harness …`), but running the full verification suite in CI is not wired up because of CBMC's cost.
- **SWRL materializer**: DL-safe subset only — complex class predicates are exact-structural, no property chains as predicates, builtins are filters (no output binding), symmetric/transitive/domain/range not applied during matching, anonymous individuals excluded, negative assertions not derived.
- **Realisation/query residuals**: anonymous-headed complex GCIs not evaluated; `getTypes` bounded defined-candidate policy; `getInstances` `direct` flag is accepted but ignored.
- **Other build/hot-path limits**: the tableau is effectively single-threaded on measured workloads (429 static-node iterations on `tosemServices` is a future short-circuit target), and `--no-default-features` does not compile (unsupported).

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md): fork the repository, create a feature branch, add tests, run `cargo test --workspace`, `cargo fmt --all -- --check` and `cargo clippy --lib --bins -- -D warnings`, then open a pull request.

## Roadmap

**Implemented in 2.1.0**

- [x] Realtime performance wave (single-tableau instance queries, snapshot caches, blocking/hash optimizations)
- [x] CI/CD: GitHub Actions CI + tagged releases with binaries and GHCR Docker image
- [x] 20-format support with shared `Auto` detection (incl. JSON-LD, RDFa, HDT, Binary RDF, N3)

**Planned**

- [ ] True incremental reasoning (differential recompute instead of A9 full recompute)
- [ ] Complete the experimental servers (REST/OWLlink/SPARQL) and OWLlink file handling
- [ ] Wire full Kani verification into CI (harnesses compile and run manually; CBMC cost is the blocker)
- [ ] WebAssembly (WASM) / Python bindings, GPU acceleration, streaming ontology processing

## License

This project is licensed under the GNU Lesser General Public License v3.0 or later (LGPL-3.0-or-later) — see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Inspired by the [HermiT](https://github.com/owlcs/hermit-reasoner/) and [Konclude](https://github.com/konclude/Konclude) reasoners; built on [horned-owl](https://github.com/phillord/horned-owl)
- Thanks to Eduard Kamburjan for his insightful feedback, and to Tobias John for the valuable discussions on language-based and mutation-based testing
- Thanks to the OWL and Semantic Web communities

## Support

Please report issues or feature requests — contributions are welcome! [Documentation](docs/) · [Issue Tracker](https://github.com/sievericcardo/oxidowl/issues) · [Discussions](CONTRIBUTING.md)

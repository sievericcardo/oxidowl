# GitHub Actions workflows

Two workflows, both using **only the default `GITHUB_TOKEN`** — no PAT or
other repository secrets are required.

## `ci.yml` — continuous integration

Runs on pushes to `main`/`master`/`dev`/`develop`, on pull requests, and on
manual dispatch. Never runs on tags (releases own tags).

| Job          | Runner             | What it does                                              |
| ------------ | ------------------ | --------------------------------------------------------- |
| `fmt`        | ubuntu-latest      | `cargo fmt --all -- --check`                              |
| `clippy`     | ubuntu-latest      | `cargo clippy --lib --bins -- -D warnings` + `--tests`    |
| `test`       | ubuntu + macos     | `cargo test --workspace --no-fail-fast` (matrix, no fail-fast) |
| `windows-check` | windows-latest   | `cargo check --workspace` (best-effort, RocksDB/MSVC)     |
| `conformance`| ubuntu-latest      | OWL2 conformance, parity, imports-closure and SWRL suites |
| `features`   | ubuntu-latest      | `cargo check` for default / server,sparql / ml / profiling / all-features + `cargo doc` |
| `msrv`       | ubuntu-latest      | `cargo check --workspace` **and** `--features server,sparql` on Rust 1.88 (MSRV; no `--all-features` — candle needs newer) |

Linux jobs install `build-essential cmake clang libclang-dev` because RocksDB
is compiled from source via Oxigraph (`sparql-store` default feature).
`--no-default-features` is known-broken and intentionally not tested.

## `release.yml` — binaries, GitHub Release, GHCR image

Triggers on `v*` tag pushes (and manual dispatch with an optional `tag`
input).

| Job         | What it does                                                                 |
| ----------- | ---------------------------------------------------------------------------- |
| `meta`      | Resolves the release tag (push ref or dispatch input)                        |
| `build`     | Native `cargo build --release --locked --features server,sparql` for 4 targets: `x86_64-unknown-linux-gnu`, `x86_64-apple-darwin` (macos-15-intel), `aarch64-apple-darwin` (macos-14), `x86_64-pc-windows-msvc`; packages `oxidowl-<tag>-<target>.tar.gz` (`.zip` on Windows) with binary + README + LICENSE and SHA-256 checksums |
| `release`   | Creates the GitHub Release via `softprops/action-gh-release@v2` with all artifacts and auto-generated notes |
| `docker`    | Builds and pushes `ghcr.io/sievericcardo/oxidowl` (linux/amd64 only — arm64 would need QEMU emulation for the RocksDB source build) |

crates.io publishing is deliberately **commented out** in `release.yml`;
`cargo publish` is permanent, so it must stay a manual step (needs a
`CARGO_REGISTRY_TOKEN` secret if ever automated).

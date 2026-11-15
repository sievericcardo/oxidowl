# Dockerfile for Oxidowl - A High-Performance OWL 2 DL Reasoner
#
# Build:  docker build -t oxidowl:latest .
# Usage:  docker run -v /path/to/ontologies:/ontologies oxidowl:latest -k /ontologies/myonto.owl
#
# Multi-stage, glibc (Debian bookworm) build:
#   - The previous Alpine/musl image failed to compile: RocksDB (pulled in
#     transitively via Oxigraph's `sparql-store` default feature) needs a C++
#     toolchain and only finds `c++`/`g++` reliably on glibc images.
#   - Builder is rust:1.88 (the MSRV). Verified on 2026-11-15:
#     `cargo +1.88 check --workspace` and
#     `cargo +1.88 check --workspace --features server,sparql` both exit 0,
#     so 1.88 is the MSRV for default features AND for `server`+`sparql`.
#     Only the `ml` feature / `--all-features` need a newer stable toolchain
#     because candle-core uses a post-1.88 NEON intrinsic
#     (`stdarch_neon_f16`). The CI `msrv` job verifies default features plus
#     `server,sparql` on 1.88 (and deliberately not `--all-features`).
#
# Feature choice (`--features server,sparql`):
#   - `server`  enables the warp-based OWLlink/REST/SPARQL HTTP servers.
#     WITHOUT this feature the binary still runs every CLI reasoning task,
#     but `--enable-server` exits with "Server mode requires the 'server'
#     feature" (see src/bin/main.rs). The image therefore always ships the
#     server-enabled binary so `docker run ... --enable-server` works.
#   - `sparql`  enables the SPARQL endpoint module (server/sparql.rs).
#   - Default features stay ON (they include `sparql-store` -> Oxigraph with
#     RocksDB, which is why the build deps below are required).

# ---------------------------------------------------------------------------
# Builder stage
# ---------------------------------------------------------------------------
# See header: 1.88 is the MSRV for default features AND `server,sparql`
# (both verified on 2026-11-15); only `ml`/`--all-features` need newer.
FROM rust:1.88-bookworm AS builder

# Build dependencies for RocksDB (compiled from source via Oxigraph):
#   build-essential  - gcc/g++/make
#   cmake            - RocksDB build system driver
#   clang/libclang   - bindgen (used by oxrocksdb-sys/librocksdb-sys)
#   pkg-config       - probe dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      build-essential \
      cmake \
      clang \
      libclang-dev \
      pkg-config && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

# Copy manifests first for better layer caching.
COPY Cargo.toml Cargo.lock ./

# Source inputs required to build the binary targets declared in Cargo.toml
# ([[bin]] oxidowl + performance_tests, [[bench]]/[[example]] paths).
COPY src ./src
COPY benches ./benches
COPY examples ./examples

# Release build with the server features enabled (see header comment).
# --locked guarantees the exact Cargo.lock dependency graph.
RUN cargo build --release --locked --bin oxidowl --features server,sparql

# ---------------------------------------------------------------------------
# Runtime stage
# ---------------------------------------------------------------------------
FROM debian:bookworm-slim

# ca-certificates: HTTPS ontology imports (http-imports/reqwest)
# libstdc++6:      Rust C++ runtime + RocksDB
# libgcc-s1:       GCC runtime (explicit; normally pulled in transitively)
# curl:            used by the docker-compose healthcheck against /api/v1/health
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
      ca-certificates \
      libstdc++6 \
      libgcc-s1 \
      curl && \
    rm -rf /var/lib/apt/lists/*

# Non-root user (uid/gid 1000).
RUN groupadd --system --gid 1000 oxidowl && \
    useradd --system --uid 1000 --gid oxidowl --home-dir /ontologies \
            --shell /usr/sbin/nologin oxidowl

# Working directory for mounted ontologies.
RUN mkdir -p /ontologies && chown -R oxidowl:oxidowl /ontologies

COPY --from=builder /build/target/release/oxidowl /usr/local/bin/oxidowl
RUN chown oxidowl:oxidowl /usr/local/bin/oxidowl && chmod 755 /usr/local/bin/oxidowl

USER oxidowl
WORKDIR /ontologies

# REST (default), OWLlink and SPARQL ports (server feature).
# Only reachable when started with --enable-server / --enable-owllink /
# --enable-sparql; EXPOSE is documentation only.
EXPOSE 8080 8081 8082

ENTRYPOINT ["oxidowl"]
CMD ["--help"]

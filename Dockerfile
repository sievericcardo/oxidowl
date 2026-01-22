# Dockerfile for Oxidowl - A High-Performance OWL 2 DL Reasoner
# 
# Build: docker build -t oxidowl:latest .
# 
# Usage examples:
# 1. With volume mount (recommended):
#    docker run -v /path/to/ontologies:/ontologies oxidowl:latest -k /ontologies/myonto.owl
#
# 2. Copy file into container:
#    docker run -i oxidowl:latest -k - < myontology.owl
#
# 3. Interactive shell:
#    docker run -it -v /path/to/ontologies:/ontologies --entrypoint /bin/sh oxidowl:latest

# Use multi-stage build for smaller final image
FROM rust:1.88-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev

# Set working directory
WORKDIR /build

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code (only src/ is needed for the binary)
COPY src ./src
COPY benches ./benches
COPY examples ./examples

# Build the release binary with optimizations
RUN cargo build --release --bin oxidowl

# Runtime stage - use minimal Alpine Linux image
FROM alpine:latest

# Install runtime dependencies (minimal)
RUN apk add --no-cache libgcc

# Create a non-root user for security
RUN addgroup -g 1000 oxidowl && \
    adduser -D -u 1000 -G oxidowl oxidowl

# Create directory for ontologies
RUN mkdir -p /ontologies && \
    chown -R oxidowl:oxidowl /ontologies

# Copy the binary from builder stage
COPY --from=builder /build/target/release/oxidowl /usr/local/bin/oxidowl

# Set ownership
RUN chown oxidowl:oxidowl /usr/local/bin/oxidowl

# Switch to non-root user
USER oxidowl

# Set working directory
WORKDIR /ontologies

# Set the entrypoint to the oxidowl binary
ENTRYPOINT ["oxidowl"]

# Default command shows help
CMD ["--help"]

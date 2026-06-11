# Containerfile for Oxidowl - A High-Performance OWL 2 DL Reasoner
# Podman-compatible container image definition
# 
# Build: podman build -t oxidowl:latest .
# 
# Usage examples:
# 1. With volume mount (recommended):
#    podman run -v /path/to/ontologies:/ontologies:Z oxidowl:latest -k /ontologies/myonto.owl
#
# 2. Copy file into container:
#    podman run -i oxidowl:latest -k - < myontology.owl
#
# 3. Interactive shell:
#    podman run -it -v /path/to/ontologies:/ontologies:Z --entrypoint /bin/sh oxidowl:latest
#
# 4. Run as pod (for Kubernetes-style orchestration):
#    podman pod create --name oxidowl-pod -p 8080:8080
#    podman run -d --pod oxidowl-pod oxidowl:latest

# Use multi-stage build for smaller final image
FROM docker.io/rust:1.88-alpine AS builder

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
FROM docker.io/alpine:latest

# Add labels for better container management and Kubernetes compatibility
LABEL maintainer="oxidowl-team" \
      version="1.0" \
      description="High-Performance OWL 2 DL Reasoner" \
      io.k8s.description="High-Performance OWL 2 DL Reasoner" \
      io.k8s.display-name="Oxidowl" \
      io.opencontainers.image.title="Oxidowl" \
      io.opencontainers.image.description="High-Performance OWL 2 DL Reasoner" \
      io.opencontainers.image.vendor="Oxidowl Project" \
      io.opencontainers.image.source="https://github.com/riccasi/oxidowl"

# Install runtime dependencies (minimal)
RUN apk add --no-cache libgcc

# Create a non-root user for security (Kubernetes/OpenShift compatible)
# Use a high UID for better compatibility with OpenShift's arbitrary UIDs
RUN addgroup -g 10001 oxidowl && \
    adduser -D -u 10001 -G oxidowl oxidowl

# Create directory for ontologies with proper permissions
RUN mkdir -p /ontologies && \
    chown -R oxidowl:oxidowl /ontologies && \
    chmod 775 /ontologies

# Copy the binary from builder stage
COPY --from=builder /build/target/release/oxidowl /usr/local/bin/oxidowl

# Set ownership
RUN chown oxidowl:oxidowl /usr/local/bin/oxidowl && \
    chmod 755 /usr/local/bin/oxidowl

# Switch to non-root user
USER 10001

# Set working directory
WORKDIR /ontologies

# Expose port if running as server (adjust as needed)
# EXPOSE 8080

# Set the entrypoint to the oxidowl binary
ENTRYPOINT ["oxidowl"]

# Default command shows help
CMD ["--help"]

# Health check for Kubernetes liveness/readiness probes
# Uncomment and adjust if oxidowl supports health checks
# HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
#   CMD oxidowl --health-check || exit 1

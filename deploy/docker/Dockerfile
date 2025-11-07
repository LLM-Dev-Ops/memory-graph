# Multi-stage build for LLM-Memory-Graph gRPC server
# Production-ready with security best practices

# ============================================================================
# Build stage
# ============================================================================
FROM rust:1.75-bookworm as builder

WORKDIR /usr/src/llm-memory-graph

# Install build dependencies
RUN apt-get update && apt-get install -y \
    protobuf-compiler \
    libprotobuf-dev \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy manifests first for dependency caching
COPY Cargo.toml Cargo.lock ./
COPY build.rs ./

# Copy proto definitions
COPY proto ./proto

# Create dummy source to build dependencies
RUN mkdir -p src/bin && \
    echo "fn main() {}" > src/bin/server.rs && \
    echo "pub fn dummy() {}" > src/lib.rs

# Build dependencies (cached layer)
RUN cargo build --release --bin server && \
    rm -rf src

# Copy actual source code
COPY src ./src

# Build release binary with optimizations
RUN cargo build --release --bin server && \
    strip target/release/server

# ============================================================================
# Runtime stage
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies and grpc_health_probe
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Install grpc_health_probe for health checks
RUN GRPC_HEALTH_PROBE_VERSION=v0.4.24 && \
    curl -Lo /usr/local/bin/grpc_health_probe \
    https://github.com/grpc-ecosystem/grpc-health-probe/releases/download/${GRPC_HEALTH_PROBE_VERSION}/grpc_health_probe-linux-amd64 && \
    chmod +x /usr/local/bin/grpc_health_probe

# Create non-root user for security
RUN useradd -m -u 1001 -s /bin/bash appuser

# Create data directory with proper permissions
RUN mkdir -p /data /plugins && \
    chown -R appuser:appuser /data /plugins

# Copy binary from builder
COPY --from=builder /usr/src/llm-memory-graph/target/release/server /usr/local/bin/server

# Verify binary is executable
RUN chmod +x /usr/local/bin/server

# Switch to non-root user
USER appuser

# Set working directory
WORKDIR /home/appuser

# Environment variables with secure defaults
ENV DB_PATH=/data \
    GRPC_HOST=0.0.0.0 \
    GRPC_PORT=50051 \
    METRICS_PORT=9090 \
    RUST_LOG=info \
    RUST_BACKTRACE=1

# Expose gRPC and metrics ports
EXPOSE 50051 9090

# Add labels for metadata
LABEL org.opencontainers.image.title="LLM-Memory-Graph gRPC Server" \
      org.opencontainers.image.description="Graph-based context-tracking and prompt-lineage database for LLM systems" \
      org.opencontainers.image.vendor="Global Business Advisors" \
      org.opencontainers.image.source="https://github.com/globalbusinessadvisors/llm-memory-graph" \
      org.opencontainers.image.licenses="MIT OR Apache-2.0"

# Health check using grpc_health_probe
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD ["/usr/local/bin/grpc_health_probe", "-addr=:50051"]

# Run the gRPC server
CMD ["server"]

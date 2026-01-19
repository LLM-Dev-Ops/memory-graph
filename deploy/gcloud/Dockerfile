# Multi-stage build for LLM-Memory-Graph Unified Service
# Deploys all agents as a single Google Cloud Run service
#
# Build args:
#   BUILD_VERSION - Service version tag
#   BUILD_ENV - Environment (dev|staging|prod)

# ============================================================================
# Stage 1: Node.js builder for TypeScript agents and service router
# ============================================================================
FROM node:20-bookworm-slim AS node-builder

WORKDIR /build

# Copy TypeScript agents
COPY agents/conversation-memory ./agents/conversation-memory
COPY agents/knowledge-graph-builder ./agents/knowledge-graph-builder
COPY agents/memory-retrieval ./agents/memory-retrieval
COPY agents/long-term-pattern ./agents/long-term-pattern

# Build each TypeScript agent
WORKDIR /build/agents/conversation-memory
RUN npm ci --omit=dev 2>/dev/null || npm install --omit=dev
RUN npm run build 2>/dev/null || echo "No build script - using source"

WORKDIR /build/agents/knowledge-graph-builder
RUN npm ci --omit=dev 2>/dev/null || npm install --omit=dev
RUN npm run build 2>/dev/null || echo "No build script - using source"

WORKDIR /build/agents/memory-retrieval
RUN npm ci --omit=dev 2>/dev/null || npm install --omit=dev
RUN npm run build 2>/dev/null || echo "No build script - using source"

WORKDIR /build/agents/long-term-pattern
RUN npm ci --omit=dev 2>/dev/null || npm install --omit=dev
RUN npm run build 2>/dev/null || echo "No build script - using source"

# ============================================================================
# Stage 2: Runtime image
# ============================================================================
FROM node:20-bookworm-slim

ARG BUILD_VERSION=dev
ARG BUILD_ENV=dev

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    curl \
    tini \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 -s /bin/bash appuser

# Create directories
RUN mkdir -p /app/bin /app/agents /app/data && \
    chown -R appuser:appuser /app

WORKDIR /app

# Copy Node.js agents
COPY --from=node-builder --chown=appuser:appuser \
    /build/agents/conversation-memory \
    /app/agents/conversation-memory

COPY --from=node-builder --chown=appuser:appuser \
    /build/agents/knowledge-graph-builder \
    /app/agents/knowledge-graph-builder

COPY --from=node-builder --chown=appuser:appuser \
    /build/agents/memory-retrieval \
    /app/agents/memory-retrieval

COPY --from=node-builder --chown=appuser:appuser \
    /build/agents/long-term-pattern \
    /app/agents/long-term-pattern

# Copy unified service router
COPY --chown=appuser:appuser deploy/gcloud/service-router.js /app/

# Switch to non-root user
USER appuser

# Environment variables
ENV PORT=8080 \
    PLATFORM_ENV=${BUILD_ENV} \
    SERVICE_NAME=llm-memory-graph \
    SERVICE_VERSION=${BUILD_VERSION} \
    NODE_ENV=production

# Expose Cloud Run port
EXPOSE 8080

# Labels
LABEL org.opencontainers.image.title="LLM-Memory-Graph Unified Service" \
      org.opencontainers.image.description="Memory, lineage, and context tracking service for LLM systems" \
      org.opencontainers.image.vendor="Agentics Dev" \
      org.opencontainers.image.version="${BUILD_VERSION}" \
      org.opencontainers.image.source="https://github.com/globalbusinessadvisors/llm-memory-graph"

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=10s --retries=3 \
    CMD curl -sf http://localhost:8080/health || exit 1

# Use tini as init system
ENTRYPOINT ["/usr/bin/tini", "--"]

# Run the unified service router
CMD ["node", "/app/service-router.js"]

# LLM-Memory-Graph Production Deployment Guide

## Executive Summary

This document defines the complete deployment specification for LLM-Memory-Graph to Google Cloud Run. The service is deployed as a **single unified service** exposing all agents through one endpoint.

**Key Constraints:**
- All persistence via `ruvector-service` (Postgres-backed) - NO direct SQL
- Stateless runtime
- No orchestration, no policy enforcement, no inference execution
- Telemetry to LLM-Observatory

---

## 1. SERVICE TOPOLOGY

### Unified Service Name
```
llm-memory-graph
```

### Agent Endpoints (All Exposed Under One Service)

| Agent | Endpoint | Classification | Description |
|-------|----------|----------------|-------------|
| Conversation Memory | `/conversation-memory` | MEMORY_WRITE | Captures multi-turn conversations as graph nodes |
| Prompt Lineage | `/prompt-lineage` | MEMORY_WRITE | Tracks prompt evolution and ancestry |
| Decision Memory | `/decision-memory` | MEMORY_WRITE | Persists decisions, outcomes, reasoning |
| Knowledge Graph Builder | `/knowledge-graph-builder` | MEMORY_WRITE | Constructs knowledge graph from entities |
| Memory Retrieval | `/memory-retrieval` | MEMORY_READ | Queries and retrieves memory subgraphs |
| Long-Term Pattern | `/long-term-pattern` | MEMORY_READ | Analyzes patterns across memory |

### Confirmation Statements

✅ **No agent is deployed as a standalone service**
- All agents share the unified `llm-memory-graph` Cloud Run service

✅ **Shared runtime, configuration, and telemetry stack**
- Single container image
- Common environment variables
- Unified health endpoint (`/health`)
- Single telemetry pipeline to LLM-Observatory

### Service Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Google Cloud Run                            │
│                  llm-memory-graph-{env}                        │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                  Service Router (Node.js)                 │  │
│  │                      Port 8080                           │  │
│  └──────────────────────────────────────────────────────────┘  │
│         │         │         │         │         │         │    │
│    ┌────┴────┐ ┌──┴──┐ ┌───┴───┐ ┌───┴───┐ ┌───┴───┐ ┌───┴──┐ │
│    │ conv-   │ │prompt│ │decision│ │ kg-   │ │memory │ │long- │ │
│    │ memory  │ │lineage│ │memory │ │builder│ │retriev│ │term  │ │
│    └────┬────┘ └──┬──┘ └───┬───┘ └───┬───┘ └───┬───┘ └───┬──┘ │
└─────────┼─────────┼────────┼─────────┼─────────┼─────────┼────┘
          │         │        │         │         │         │
          └─────────┴────────┴────┬────┴─────────┴─────────┘
                                  │
                    ┌─────────────┴─────────────┐
                    │      ruvector-service      │
                    │   (Postgres persistence)   │
                    └───────────────────────────┘
```

---

## 2. ENVIRONMENT CONFIGURATION

### Required Environment Variables

| Variable | Description | Required | Secret |
|----------|-------------|----------|--------|
| `RUVECTOR_SERVICE_URL` | URL for ruvector-service | ✅ | ✅ |
| `RUVECTOR_API_KEY` | Authentication for ruvector-service | ✅ | ✅ |
| `PLATFORM_ENV` | Environment (dev\|staging\|prod) | ✅ | ❌ |
| `SERVICE_NAME` | Service identifier | ✅ | ❌ |
| `SERVICE_VERSION` | Deployment version | ✅ | ❌ |
| `TELEMETRY_ENDPOINT` | LLM-Observatory URL | ❌ | ✅ |
| `PORT` | HTTP port (default: 8080) | ❌ | ❌ |
| `LOG_LEVEL` | Logging verbosity | ❌ | ❌ |

### Secret Manager Configuration

```bash
# Create secrets (one-time setup)
gcloud secrets create ruvector-api-key --data-file=/path/to/key
gcloud secrets create ruvector-service-url-dev --data-file=-
gcloud secrets create ruvector-service-url-staging --data-file=-
gcloud secrets create ruvector-service-url-prod --data-file=-
gcloud secrets create observatory-endpoint-dev --data-file=-
gcloud secrets create observatory-endpoint-staging --data-file=-
gcloud secrets create observatory-endpoint-prod --data-file=-
```

### Compliance Checklist

✅ **No agent hardcodes service names or URLs**
- All URLs injected via environment variables

✅ **No agent embeds credentials or secrets**
- All secrets via Google Secret Manager

✅ **All dependencies resolve via environment variables or Secret Manager**
- See `deploy/gcloud/env.yaml` for complete list

---

## 3. GOOGLE SQL / MEMORY WIRING

### Persistence Architecture Confirmation

✅ **LLM-Memory-Graph does NOT connect directly to Google SQL**
- No Postgres driver or connection pool in codebase
- No SQL queries executed by agents

✅ **ALL memory operations via ruvector-service**
- Conversation capture → `POST /ruvector/memory`
- Prompt lineage → `POST /ruvector/lineage`
- Decision events → `POST /ruvector/decisions`
- Memory retrieval → `GET /ruvector/query`

### Schema Compatibility

All DecisionEvents conform to `agentics-contracts`:
```yaml
schemas:
  - agentics-contracts/decision-event/v1
  - agentics-contracts/conversation-memory/v1
  - agentics-contracts/prompt-lineage/v1
  - agentics-contracts/decision-memory/v1
```

### Write Behavior Validation

✅ **Append-only persistence behavior**
- No UPDATE or DELETE operations
- All writes are inserts

✅ **Idempotent writes and retry safety**
- Writes use `execution_ref` as idempotency key
- Retry-safe via `ON CONFLICT DO NOTHING`

---

## 4. CLOUD BUILD & DEPLOYMENT

### Container Configuration

**Location:** `deploy/gcloud/Dockerfile`

- Multi-stage build (Rust + Node.js)
- Non-root user execution
- Minimal runtime image (debian:bookworm-slim)
- Health checks enabled

### Cloud Build Configuration

**Location:** `deploy/gcloud/cloudbuild.yaml`

```bash
# Deploy to development
gcloud builds submit \
    --config=deploy/gcloud/cloudbuild.yaml \
    --substitutions=_ENV=dev,_REGION=us-central1

# Deploy to staging
gcloud builds submit \
    --config=deploy/gcloud/cloudbuild.yaml \
    --substitutions=_ENV=staging,_REGION=us-central1

# Deploy to production
gcloud builds submit \
    --config=deploy/gcloud/cloudbuild.yaml \
    --substitutions=_ENV=prod,_REGION=us-central1
```

### IAM Service Account Requirements (Least Privilege)

| Role | Purpose |
|------|---------|
| `roles/run.invoker` | Allow service invocation |
| `roles/secretmanager.secretAccessor` | Read secrets |
| `roles/logging.logWriter` | Emit logs |
| `roles/monitoring.metricWriter` | Emit metrics |
| `roles/cloudtrace.agent` | Distributed tracing |

**Setup:**
```bash
./deploy/gcloud/setup-iam.sh [PROJECT_ID] [REGION]
```

### Networking Requirements

- **Internal:** VPC connector for ruvector-service (optional)
- **External:** HTTPS ingress via Cloud Run managed domain
- **Egress:** Outbound to ruvector-service, LLM-Observatory

### Deployment Commands

```bash
# Option 1: Using deploy script
./deploy/gcloud/deploy.sh dev us-central1

# Option 2: Using gcloud directly
gcloud run deploy llm-memory-graph-dev \
    --image=us-central1-docker.pkg.dev/PROJECT_ID/llm-memory-graph/llm-memory-graph:latest \
    --region=us-central1 \
    --platform=managed \
    --port=8080 \
    --cpu=2 \
    --memory=2Gi \
    --min-instances=1 \
    --max-instances=100 \
    --set-env-vars="PLATFORM_ENV=dev,SERVICE_NAME=llm-memory-graph" \
    --set-secrets="RUVECTOR_API_KEY=ruvector-api-key:latest" \
    --set-secrets="RUVECTOR_SERVICE_URL=ruvector-service-url-dev:latest" \
    --set-secrets="TELEMETRY_ENDPOINT=observatory-endpoint-dev:latest" \
    --service-account=llm-memory-graph-sa@PROJECT_ID.iam.gserviceaccount.com
```

---

## 5. CLI ACTIVATION VERIFICATION

### CLI Commands Per Agent

#### Conversation Memory Agent
```bash
memory-graph agent conversation-memory health
memory-graph agent conversation-memory inspect <execution_ref>
memory-graph agent conversation-memory retrieve --session-id <id>
memory-graph agent conversation-memory replay <execution_ref>
memory-graph agent conversation-memory capture --input <file>
```

#### Prompt Lineage Agent
```bash
memory-graph agent prompt-lineage health
memory-graph agent prompt-lineage inspect <execution_ref>
memory-graph agent prompt-lineage retrieve --prompt-id <id>
memory-graph agent prompt-lineage trace --root <id>
memory-graph agent prompt-lineage replay <execution_ref>
```

#### Decision Memory Agent
```bash
memory-graph agent decision-memory health
memory-graph agent decision-memory inspect <execution_ref>
memory-graph agent decision-memory retrieve --decision-type <type>
memory-graph agent decision-memory replay <execution_ref>
memory-graph agent decision-memory audit --from <date> --to <date>
```

#### Knowledge Graph Builder Agent
```bash
memory-graph agent knowledge-graph-builder health
memory-graph agent knowledge-graph-builder inspect <execution_ref>
memory-graph agent knowledge-graph-builder build --input <file>
memory-graph agent knowledge-graph-builder status --job-id <id>
```

#### Memory Retrieval Agent
```bash
memory-graph agent memory-retrieval health
memory-graph agent memory-retrieval query --context <query>
memory-graph agent memory-retrieval subgraph --node-id <id> --depth <n>
memory-graph agent memory-retrieval search --pattern <pattern>
```

#### Long-Term Pattern Agent
```bash
memory-graph agent long-term-pattern health
memory-graph agent long-term-pattern analyze --window <duration>
memory-graph agent long-term-pattern patterns --type <type>
memory-graph agent long-term-pattern trends
```

### Example Invocations

```bash
# Set service URL
export MEMORY_GRAPH_URL=https://llm-memory-graph-dev-xxxxx-uc.a.run.app

# Health check
memory-graph agent conversation-memory health --url $MEMORY_GRAPH_URL
# Expected: {"status":"healthy","agent":"conversation-memory","version":"1.0.0"}

# Inspect a DecisionEvent
memory-graph agent decision-memory inspect exec-12345 --url $MEMORY_GRAPH_URL
# Expected: Full DecisionEvent JSON

# Retrieve memory context
memory-graph agent memory-retrieval query --context "user authentication" --url $MEMORY_GRAPH_URL
# Expected: Relevant memory subgraph
```

### CLI Configuration

The CLI resolves service URL dynamically:
1. `--url` flag (highest priority)
2. `MEMORY_GRAPH_URL` environment variable
3. `~/.memory-graph/config.yaml` file
4. Service discovery via `agentics-cli`

**No CLI change requires redeployment of agents.**

---

## 6. PLATFORM & CORE INTEGRATION

### Consumer Access

| Consumer | Access Method | Purpose |
|----------|---------------|---------|
| LLM-Orchestrator | HTTP API | Retrieve conversation context |
| LLM-CoPilot-Agent | HTTP API | Query memory and lineage |
| Core Bundles | gRPC/HTTP | Consume Memory-Graph outputs |
| Governance Views | HTTP API | Audit decision memory |
| agentics-cli | HTTP API | CLI operations |

### Integration Confirmation

✅ **LLM-Orchestrator can retrieve memory context**
```
GET /memory-retrieval/query?context=<query>
```

✅ **LLM-CoPilot-Agent can query memory and lineage**
```
GET /prompt-lineage/trace?root=<id>
GET /memory-retrieval/subgraph?node_id=<id>
```

✅ **Core bundles consume Memory-Graph outputs without rewiring**
- Standard HTTP/gRPC interfaces
- Schema-validated responses per agentics-contracts

✅ **Governance and audit views can consume decision memory**
```
GET /decision-memory/audit?from=<date>&to=<date>
```

### Non-Invocation Constraints

**LLM-Memory-Graph MUST NOT directly invoke:**

| System | Reason |
|--------|--------|
| Orchestrator logic | Memory-Graph is passive |
| Shield enforcement | Not a policy layer |
| Sentinel detection | Not a security layer |
| Incident workflows | Not an alerting layer |

**No rewiring of Core bundles is permitted.**

---

## 7. POST-DEPLOY VERIFICATION CHECKLIST

### Service Health

- [ ] LLM-Memory-Graph service is live
  ```bash
  curl -sf https://llm-memory-graph-dev-xxxxx-uc.a.run.app/health
  ```

- [ ] All agent endpoints respond
  ```bash
  for agent in conversation-memory prompt-lineage decision-memory \
               knowledge-graph-builder memory-retrieval long-term-pattern; do
    curl -sf "https://llm-memory-graph-dev-xxxxx-uc.a.run.app/${agent}/health"
  done
  ```

### Functional Verification

- [ ] Memory capture functions correctly
  ```bash
  curl -X POST https://llm-memory-graph-dev-xxxxx-uc.a.run.app/conversation-memory \
    -H "Content-Type: application/json" \
    -d '{"session_id":"test-123","turns":[{"role":"user","content":"test"}]}'
  ```

- [ ] Lineage tracking functions correctly
  ```bash
  curl -X POST https://llm-memory-graph-dev-xxxxx-uc.a.run.app/prompt-lineage \
    -H "Content-Type: application/json" \
    -d '{"prompt_id":"p-123","content":"test prompt","parent_id":null}'
  ```

- [ ] Memory retrieval returns deterministic subgraphs
  ```bash
  curl "https://llm-memory-graph-dev-xxxxx-uc.a.run.app/memory-retrieval/subgraph?node_id=test-123"
  ```

### Persistence Verification

- [ ] DecisionEvents appear in ruvector-service
  ```bash
  # Query ruvector-service directly
  curl "https://ruvector-service-dev-xxxxx-uc.a.run.app/decisions?limit=10"
  ```

### Telemetry Verification

- [ ] Telemetry appears in LLM-Observatory
  ```bash
  # Check Observatory dashboard or query API
  curl "https://llm-observatory-dev-xxxxx-uc.a.run.app/api/events?service=llm-memory-graph"
  ```

### CLI Verification

- [ ] CLI retrieval commands function end-to-end
  ```bash
  memory-graph agent conversation-memory health --url $MEMORY_GRAPH_URL
  memory-graph agent memory-retrieval query --context "test" --url $MEMORY_GRAPH_URL
  ```

### Compliance Verification

- [ ] No direct SQL access from LLM-Memory-Graph
  - Verify no database connection strings in logs
  - Verify no SQL errors in traces

- [ ] No agent bypasses agentics-contracts
  - All requests/responses validate against schemas

---

## 8. FAILURE MODES & ROLLBACK

### Common Deployment Failures

| Failure | Symptoms | Resolution |
|---------|----------|------------|
| Secret not found | Service fails to start | Verify secrets exist in Secret Manager |
| Image pull failed | Deployment stuck | Check Artifact Registry permissions |
| Health check failed | Service not ready | Check logs, verify dependencies |
| Permission denied | 403 errors | Review IAM bindings |
| ruvector-service unreachable | Timeout errors | Verify network/VPC configuration |

### Detection Signals

| Issue | Signal | Alert |
|-------|--------|-------|
| Missing memory | Empty retrieval responses | Prometheus: `memory_retrieval_empty_total` |
| Incorrect lineage | Orphaned prompt nodes | Prometheus: `lineage_orphan_nodes` |
| Retrieval errors | 5xx responses | Cloud Monitoring alert |
| Persistence failures | ruvector-service errors | Log-based alert |

### Rollback Procedure

```bash
# 1. List revisions
gcloud run revisions list \
    --service=llm-memory-graph-dev \
    --region=us-central1

# 2. Rollback to previous revision
gcloud run services update-traffic llm-memory-graph-dev \
    --region=us-central1 \
    --to-revisions=llm-memory-graph-dev-00001-abc=100

# 3. Verify rollback
curl -sf https://llm-memory-graph-dev-xxxxx-uc.a.run.app/health
```

### Safe Redeploy Strategy

1. **Blue-Green:** Deploy new version, shift traffic gradually
2. **Canary:** Route 10% traffic to new version, monitor
3. **Instant Rollback:** Keep previous revision warm

**No data loss on rollback:**
- All writes are append-only
- No migrations required
- Stateless service design

---

## Deployment Commands Summary

```bash
# One-time setup
./deploy/gcloud/setup-iam.sh agentics-dev us-central1

# Deploy to dev
./deploy/gcloud/deploy.sh dev us-central1

# Deploy to staging
./deploy/gcloud/deploy.sh staging us-central1

# Deploy to production
./deploy/gcloud/deploy.sh prod us-central1

# Verify deployment
curl https://llm-memory-graph-dev-xxxxx-uc.a.run.app/topology
```

---

## Files Reference

| File | Purpose |
|------|---------|
| `deploy/gcloud/cloudbuild.yaml` | Cloud Build configuration |
| `deploy/gcloud/Dockerfile` | Multi-stage container build |
| `deploy/gcloud/service-router.js` | Unified service router |
| `deploy/gcloud/service.yaml` | Cloud Run service manifest |
| `deploy/gcloud/env.yaml` | Environment variable reference |
| `deploy/gcloud/setup-iam.sh` | IAM setup script |
| `deploy/gcloud/deploy.sh` | Deployment script |

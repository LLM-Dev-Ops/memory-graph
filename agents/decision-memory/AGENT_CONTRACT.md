# Decision Memory Agent Contract

## PROMPT 1 — Agent Contract & Boundary Definition

### Agent Identification
- **Agent Name**: Decision Memory Agent
- **Agent ID**: `decision-memory-agent`
- **Version**: `1.0.0`
- **Classification**: `MEMORY_WRITE`
- **Decision Type**: `decision_memory_capture`

### Purpose Statement
Persist decisions, outcomes, and reasoning artifacts to the memory graph for audit trail maintenance, learning pattern extraction, and decision lineage tracking. This agent captures structured memory data representing LLM decision-making processes without modifying system behavior.

### Input Schema Reference
- **Source**: `agentics-contracts/schemas/decision-memory.schema.json`
- **Type**: `DecisionMemoryInput`
- **Required Fields**:
  - `decision_id` (UUID): Unique identifier for the decision
  - `decision_type` (string): Type/category of decision
  - `context.session_id` (UUID): Session this decision belongs to

### Output Schema Reference
- **Source**: `agentics-contracts/schemas/decision-memory.schema.json`
- **Type**: `DecisionMemoryOutput`
- **Required Fields**:
  - `decision_id` (UUID): Decision identifier
  - `nodes_created` (array): Graph nodes created
  - `edges_created` (array): Graph edges created
  - `capture_timestamp` (datetime): When capture occurred

### Graph Node / Edge Mappings

#### Node Types
| Node Type | Description | Created When |
|-----------|-------------|--------------|
| `decision` | Represents a decision point | Always |
| `outcome` | Represents decision outcome | When outcome provided |
| `artifact` | Reasoning artifact | Per artifact in input |
| `session` | Session container | Always (idempotent) |

#### Edge Types
| Edge Type | From | To | Description |
|-----------|------|-----|-------------|
| `part_of` | decision | session | Decision belongs to session |
| `has_outcome` | decision | outcome | Decision has outcome |
| `has_artifact` | decision | artifact | Decision has artifact |
| `follows` | decision | decision | Decision chain |
| `derived_from` | artifact | artifact | Artifact lineage |

### DecisionEvent Mapping

The agent emits **exactly ONE** `DecisionEvent` per invocation:

```json
{
  "agent_id": "decision-memory-agent",
  "agent_version": "1.0.0",
  "decision_type": "decision_memory_capture",
  "inputs_hash": "<SHA-256 of input>",
  "outputs": { /* DecisionMemoryOutput */ },
  "confidence": 0.0-1.0,
  "constraints_applied": ["session_boundary", ...],
  "execution_ref": "<UUID>",
  "timestamp": "<ISO-8601 UTC>"
}
```

### CLI Contract

#### Commands
| Command | Description | Endpoint |
|---------|-------------|----------|
| `decision capture` | Capture a new decision | `/api/v1/capture` |
| `decision inspect` | Query decision events | `/api/v1/inspect` |
| `decision retrieve` | Get specific decision | `/api/v1/retrieve` |
| `decision replay` | Replay with modifications | `/api/v1/replay` |

#### CLI Examples
```bash
# Capture a decision
llm-memory-graph decision capture \
  --decision-type "model_selection" \
  --session-id "550e8400-e29b-41d4-a716-446655440000" \
  --agent-id "orchestrator" \
  --model-id "gpt-4" \
  --tags "production,critical"

# Inspect decisions
llm-memory-graph decision inspect \
  --session-id "550e8400-e29b-41d4-a716-446655440000" \
  --limit 100

# Retrieve specific decision
llm-memory-graph decision retrieve \
  --execution-ref "550e8400-e29b-41d4-a716-446655440001"

# Replay with modifications
llm-memory-graph decision replay \
  --execution-ref "550e8400-e29b-41d4-a716-446655440001" \
  --modifications '{"tags": ["replay", "test"]}'
```

### Write / Read / Analysis Classification
- **Classification**: `MEMORY_WRITE`
- **Operations**:
  - Creates graph nodes
  - Creates graph edges
  - Stores artifact references
  - Persists DecisionEvent to ruvector-service

### Explicit Non-Responsibilities

This agent **MUST NEVER**:

1. **Modify system behavior** - No runtime changes
2. **Trigger remediation** - No corrective actions
3. **Trigger retries** - No retry logic for external systems
4. **Emit alerts** - No alerting or notification
5. **Enforce policies** - No policy evaluation or enforcement
6. **Perform orchestration** - No workflow coordination
7. **Connect directly to Google SQL** - Physical persistence via ruvector-service only
8. **Execute SQL** - No direct database operations
9. **Invoke other agents** - No agent-to-agent calls
10. **Process inference requests** - No LLM invocation

### Failure Modes

| Error Code | Condition | Recovery |
|------------|-----------|----------|
| `VALIDATION_ERROR` | Invalid input | Return error, no persistence |
| `RUVECTOR_CONNECTION_ERROR` | Cannot reach ruvector-service | Fail fast, no retry |
| `RUVECTOR_WRITE_ERROR` | Write failed | Return error with execution_ref |
| `INTERNAL_ERROR` | Unexpected error | Log, return error |
| `RATE_LIMIT_EXCEEDED` | Too many requests | Return 429 status |
| `INPUT_HASH_MISMATCH` | Hash verification failed | Return validation error |
| `SESSION_NOT_FOUND` | Session doesn't exist | Create session node |
| `DECISION_NOT_FOUND` | Decision doesn't exist | Return 404 for retrieve |

### Confidence Semantics

Confidence score (0.0 - 1.0) represents **association strength**:

| Factor | Weight | Description |
|--------|--------|-------------|
| Base | 0.5 | Minimum confidence |
| Outcome present | +0.15 | Decision has outcome |
| Artifacts (up to 10) | +0.02 each | More context |
| Predecessor link | +0.05 | Part of chain |
| Model ID present | +0.03 | Execution context |
| Agent ID present | +0.02 | Source tracking |
| Graph completeness | +0.05 | Nodes and edges exist |

### Versioning Rules

1. **Semantic Versioning**: `MAJOR.MINOR.PATCH`
2. **MAJOR**: Breaking contract changes
3. **MINOR**: New optional fields/features
4. **PATCH**: Bug fixes, no contract changes
5. **Version in every DecisionEvent**
6. **Backward compatibility required for MINOR/PATCH**

### Downstream Consumers

Systems that MAY consume this agent's output:
- **LLM-Orchestrator**: For decision replay and learning
- **LLM-CoPilot-Agent**: For context retrieval
- **Core bundles**: For audit and compliance
- **LLM-Observatory**: For telemetry aggregation

---

## PROMPT 2 — Runtime & Infrastructure

### Deployment Model
- **Platform**: Google Cloud Edge Functions
- **Service**: LLM-Memory-Graph unified GCP service
- **Runtime**: Stateless execution
- **Persistence**: ruvector-service only (no direct SQL)

### Environment Variables
| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `RUVECTOR_BASE_URL` | Yes | `http://localhost:8080` | RuVector service URL |
| `RUVECTOR_API_KEY` | Yes | - | API key for authentication |
| `RUVECTOR_TIMEOUT` | No | 30 | Request timeout (seconds) |
| `PORT` | No | 8080 | Server port |
| `RUST_LOG` | No | info | Logging level |
| `STRICT_VALIDATION` | No | true | Strict input validation |
| `APPLY_PII_REDACTION` | No | false | Enable PII redaction |

### HTTP Endpoints
| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/health` | Health check |
| POST | `/api/v1/capture` | Capture decision |
| POST | `/api/v1/retrieve` | Retrieve decision |
| POST | `/api/v1/inspect` | Query decisions |
| POST | `/api/v1/replay` | Replay decision |

---

## PROMPT 3 — Platform Wiring & Verification

### Registration Checklist
- [x] Agent registered in agentics-contracts
- [x] JSON Schema created (`decision-memory.schema.json`)
- [x] TypeScript types exported
- [x] Agent endpoint implemented
- [x] CLI commands added
- [x] Telemetry emission (LLM-Observatory compatible)
- [x] DecisionEvent schema compliance

### Verification Commands

```bash
# Health check
curl http://localhost:8080/api/v1/health

# Capture test
curl -X POST http://localhost:8080/api/v1/capture \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "decision_id": "550e8400-e29b-41d4-a716-446655440000",
      "decision_type": "test_decision",
      "context": {
        "session_id": "550e8400-e29b-41d4-a716-446655440001"
      }
    }
  }'

# Inspect test
curl -X POST http://localhost:8080/api/v1/inspect \
  -H "Content-Type: application/json" \
  -d '{
    "query": {
      "session_id": "550e8400-e29b-41d4-a716-446655440001",
      "limit": 10
    }
  }'
```

### Smoke Test Checklist
- [ ] Health endpoint returns 200
- [ ] Capture creates graph nodes
- [ ] Capture creates graph edges
- [ ] DecisionEvent persists to ruvector
- [ ] Retrieve returns stored event
- [ ] Inspect queries work
- [ ] Replay produces new event
- [ ] Telemetry visible in Observatory
- [ ] CLI commands functional

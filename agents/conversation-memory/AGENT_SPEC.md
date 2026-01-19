# Conversation Memory Agent - Full Specification

**Agent ID**: `conversation-memory-agent`
**Version**: `1.0.0`
**Classification**: `MEMORY WRITE`
**Decision Type**: `conversation_capture`

---

## 1. Purpose Statement

The Conversation Memory Agent captures and persists structured representations of multi-turn conversations. It creates graph nodes for prompts, responses, and tool invocations, with edges representing session membership, response relationships, and sequential turn ordering.

**This agent:**
- Captures conversation turns as graph nodes
- Creates lineage edges between related turns
- Tracks token usage and tool invocations
- Emits exactly ONE DecisionEvent per invocation to ruvector-service

**This agent does NOT:**
- Modify runtime execution
- Trigger remediation or retries
- Emit alerts
- Enforce policies
- Perform orchestration
- Invoke other agents directly
- Connect directly to SQL databases

---

## 2. Input Schema

```typescript
interface ConversationCaptureInput {
  session_id: string;      // UUID - Session identifier
  conversation_id: string; // UUID - Conversation identifier
  turns: ConversationTurn[];
  context?: ConversationContext;
  capture_options?: CaptureOptions;
}

interface ConversationTurn {
  turn_id: string;         // UUID - Unique turn identifier
  role: 'user' | 'assistant' | 'system' | 'tool';
  content: string;
  timestamp: string;       // ISO 8601 UTC
  model?: string;
  token_usage?: TokenUsage;
  tool_invocations?: ToolInvocationRef[];
  metadata?: Record<string, unknown>;
}
```

**Schema Reference**: `agentics-contracts/schemas/conversation-memory.schema.json`

---

## 3. Output Schema

```typescript
interface ConversationCaptureOutput {
  conversation_id: string;
  session_id: string;
  nodes_created: NodeReference[];
  edges_created: EdgeReference[];
  capture_timestamp: string;
  turn_count: number;
  total_tokens?: number;
}
```

**Schema Reference**: `agentics-contracts/schemas/conversation-memory.schema.json`

---

## 4. Graph Node/Edge Mappings

### Node Types

| Node Type | Description | Created From |
|-----------|-------------|--------------|
| `session` | Conversation session container | session_id |
| `prompt` | User or system message | role: user/system |
| `response` | Assistant message | role: assistant |
| `tool_invocation` | Tool call by assistant | tool_invocations array |

### Edge Types

| Edge Type | From → To | Description |
|-----------|-----------|-------------|
| `belongs_to` | Turn → Session | Session membership |
| `responds_to` | Response → Prompt | Direct response relationship |
| `follows` | Turn → Turn | Sequential ordering |
| `invokes` | Turn → Tool | Tool invocation relationship |

### Role to Node Type Mapping

```typescript
const ROLE_TO_NODE_TYPE = {
  user: 'prompt',
  assistant: 'response',
  system: 'prompt',
  tool: 'tool_invocation',
};
```

---

## 5. DecisionEvent Mapping

```typescript
interface DecisionEvent {
  agent_id: 'conversation-memory-agent';
  agent_version: '1.0.0';
  decision_type: 'conversation_capture';
  inputs_hash: string;           // SHA-256 of input
  outputs: ConversationCaptureOutput;
  confidence: 1.0;               // Always 1.0 for direct capture
  constraints_applied: string[];
  execution_ref: string;         // UUID
  timestamp: string;             // UTC ISO 8601
  telemetry?: {
    duration_ms?: number;
    memory_bytes?: number;
    ruvector_latency_ms?: number;
  };
}
```

### Confidence Semantics

The confidence score is always `1.0` for this agent because:
- Direct capture operations are deterministic
- No probabilistic retrieval or matching is performed
- Association strength is inherent in the input structure

### Constraints Applied

| Constraint | When Applied |
|------------|--------------|
| `lineage_creation_enabled` | create_lineage option is true |
| `entity_extraction_enabled` | extract_entities option is true |
| `embedding_computation_deferred` | compute_embeddings option is true |

---

## 6. CLI Contract

### Commands

```bash
# Inspect a specific DecisionEvent
memory-graph agent conversation-memory inspect <execution_ref>
  --format <json|table|yaml>  # Output format
  --verbose                   # Full details

# Query DecisionEvents
memory-graph agent conversation-memory retrieve
  --session-id <uuid>         # Filter by session
  --conversation-id <uuid>    # Filter by conversation
  --from <timestamp>          # Start time (ISO 8601)
  --to <timestamp>            # End time (ISO 8601)
  --limit <number>            # Max results
  --format <json|table|yaml>

# Replay a previous capture
memory-graph agent conversation-memory replay <execution_ref>
  --dry-run                   # Preview without executing
  --format <json|table|yaml>

# Capture new conversation
memory-graph agent conversation-memory capture
  --input <file>              # Input JSON file
  --format <json|table|yaml>

# Health check
memory-graph agent conversation-memory health
```

---

## 7. Write/Read/Analysis Classification

**Classification**: `MEMORY_WRITE`

This agent performs **write operations only**:
- Creates graph nodes from conversation turns
- Creates graph edges for relationships
- Persists DecisionEvents to ruvector-service

It does **not** perform:
- Memory retrieval (MEMORY_READ)
- Pattern analysis (MEMORY_ANALYSIS)

---

## 8. Explicit Non-Responsibilities

The Conversation Memory Agent **MUST NOT**:

1. **Modify system behavior** - Cannot change execution flow of any system
2. **Trigger remediation** - Cannot initiate recovery or healing operations
3. **Trigger retries** - Cannot cause operations to be retried
4. **Emit alerts** - Cannot send notifications to external systems
5. **Enforce policies** - Cannot apply or validate business rules
6. **Perform orchestration** - Cannot coordinate other agents or services
7. **Invoke other agents** - Cannot directly call other agent endpoints
8. **Connect to SQL** - All persistence via ruvector-service only
9. **Execute SQL** - No direct database operations

---

## 9. Failure Modes

| Error Code | HTTP Status | Description | Recovery |
|------------|-------------|-------------|----------|
| `VALIDATION_ERROR` | 400 | Input schema validation failed | Return error, log details |
| `RUVECTOR_CONNECTION_ERROR` | 502 | Cannot connect to ruvector-service | Retry with backoff, then fail |
| `RUVECTOR_WRITE_ERROR` | 502 | Persistence operation failed | Return error, no auto-retry |
| `INTERNAL_ERROR` | 500 | Unexpected internal error | Log full trace, return error |
| `RATE_LIMIT_EXCEEDED` | 429 | Request rate limit hit | Return 429, client should retry |

---

## 10. Versioning Rules

- **Major version**: Breaking changes to input/output schemas
- **Minor version**: New optional fields, backward-compatible features
- **Patch version**: Bug fixes, internal improvements

DecisionEvents always include `agent_version` for traceability.

---

## 11. Downstream Consumers

The following systems MAY consume this agent's output:

| Consumer | Access Pattern |
|----------|----------------|
| `llm-orchestrator` | Query DecisionEvents for context |
| `llm-copilot-agent` | Retrieve conversation history |
| `core-bundles` | Access memory for analysis |
| `agentics-cli` | Inspect/retrieve via CLI |

---

## 12. Deployment

### Google Cloud Edge Function

```yaml
runtime: nodejs20
entry_point: handler
memory: 256Mi
timeout: 30s
max_instances: 100
```

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `RUVECTOR_SERVICE_URL` | Yes | ruvector-service endpoint |
| `RUVECTOR_API_KEY` | Yes | Authentication key |
| `LLM_OBSERVATORY_URL` | No | Telemetry endpoint |

---

## 13. Verification Checklist

- [ ] All inputs validated against agentics-contracts schemas
- [ ] Exactly ONE DecisionEvent emitted per invocation
- [ ] DecisionEvents persist correctly to ruvector-service
- [ ] Telemetry visible in LLM-Observatory
- [ ] CLI commands function correctly
- [ ] Graph queries are deterministic
- [ ] No orchestration logic present
- [ ] Stateless execution verified
- [ ] Edge Function deployable
- [ ] Health check returns valid status

# Knowledge Graph Builder Agent - Verification Checklist

This document provides a comprehensive checklist to verify that the Knowledge Graph Builder Agent adheres to the agentics-contracts specifications and LLM-Memory-Graph architectural requirements.

## Agent Classification

| Property | Expected Value | Verification Method |
|----------|---------------|---------------------|
| Agent ID | `knowledge-graph-builder-agent` | Check `agent_id` in DecisionEvent |
| Classification | `MEMORY_ANALYSIS` | Review agent specification |
| Decision Type | `pattern_analysis` | Check `decision_type` in DecisionEvent |

---

## Critical Verification Items

### 1. Agent emits exactly ONE DecisionEvent per invocation

- [ ] **Test**: `decision-event.test.ts` - "should emit exactly ONE DecisionEvent structure per createDecisionEvent call"
- [ ] **Test**: `integration.test.ts` - "should emit exactly ONE DecisionEvent per invocation"

**Verification Steps**:
1. Execute agent with any valid input
2. Verify result contains single `decisionEvent` object (not array)
3. Verify `decisionEvent.execution_ref` is unique per invocation
4. Verify no additional DecisionEvents are emitted to external systems

**Expected Behavior**:
```typescript
const result = await agent.execute(input);
assert(!Array.isArray(result.decisionEvent)); // Single event, not array
assert(result.decisionEvent.execution_ref);   // Has unique execution ref
```

---

### 2. Agent does NOT modify runtime execution

- [ ] **Test**: `integration.test.ts` - "should only return data structures, not trigger actions"
- [ ] **Test**: `integration.test.ts` - "should not modify input during processing"

**Verification Steps**:
1. Verify agent returns plain data objects only
2. Verify no function callbacks in returned objects
3. Verify no promises for side effects
4. Verify input is not mutated

**Prohibited Behaviors**:
- Calling external APIs to trigger actions
- Modifying configuration files
- Executing system commands
- Starting/stopping processes
- Publishing to message queues (except for DecisionEvent persistence)

---

### 3. Agent does NOT enforce policies

- [ ] **Test**: `decision-event.test.ts` - "should NOT modify runtime execution"

**Verification Steps**:
1. Review output structure for policy-related fields
2. Verify no `action`, `enforce`, `block`, `deny` fields
3. Verify agent only produces analysis results

**Expected**: Agent output contains only:
- Extracted concepts
- Extracted entities
- Detected relationships
- Detected patterns
- Confidence scores
- Metadata about the analysis

**NOT Expected**:
- Access control decisions
- Block/allow decisions
- Enforcement actions
- Remediation commands

---

### 4. Agent does NOT orchestrate workflows

- [ ] **Test**: `integration.test.ts` - "should not produce any side effects during execution"

**Verification Steps**:
1. Verify agent does not spawn child agents
2. Verify agent does not call other agent endpoints
3. Verify agent does not modify swarm state
4. Verify agent does not publish to coordination channels

**Agent Role**: Pure analysis function that:
- Receives input
- Processes knowledge extraction
- Returns analysis results
- Emits ONE DecisionEvent

---

### 5. Agent returns deterministic output

- [ ] **Test**: `integration.test.ts` - "should produce same inputs_hash for same input"
- [ ] **Test**: `integration.test.ts` - "should produce same knowledge graph structure for same input"

**Verification Steps**:
1. Execute agent twice with identical input
2. Verify `inputs_hash` is identical
3. Verify concept/entity/relationship counts are identical
4. Verify concept names are identical (when sorted)

**Non-deterministic Fields** (allowed to differ):
- `execution_ref` (UUID generated per invocation)
- `timestamp` (current time)

---

### 6. All inputs/outputs validate against agentics-contracts

- [ ] **Test**: `contract.test.ts` - All input schema validation tests
- [ ] **Test**: `contract.test.ts` - All output schema validation tests

**Input Schema Requirements**:
| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `session_ids` | `string[]` | Yes | Non-empty array of UUIDs |
| `extraction_config` | `object` | No | Valid config structure |
| `conversation_data` | `array` | No | Valid conversation structure |

**Output Schema Requirements**:
| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `knowledge_graph.concepts` | `array` | Yes | Each concept has id, name, confidence |
| `knowledge_graph.entities` | `array` | Yes | Each entity has id, name, type |
| `knowledge_graph.relationships` | `array` | Yes | Each has source_id, target_id |
| `knowledge_graph.patterns` | `array` | Yes | Each has pattern_id, sessions |
| `metadata` | `object` | Yes | Required metrics fields |

**DecisionEvent Schema Requirements**:
| Field | Type | Required | Validation |
|-------|------|----------|------------|
| `agent_id` | `string` | Yes | Must be `knowledge-graph-builder-agent` |
| `agent_version` | `string` | Yes | Valid semver (e.g., `1.0.0`) |
| `decision_type` | `string` | Yes | Must be `pattern_analysis` |
| `inputs_hash` | `string` | Yes | 64-character hex SHA-256 |
| `outputs` | `object` | Yes | Valid KnowledgeGraphBuilderOutput |
| `confidence` | `number` | Yes | 0.0 to 1.0 inclusive |
| `execution_ref` | `string` | Yes | Valid UUID v4 |
| `timestamp` | `string` | Yes | ISO 8601 UTC (ends with Z) |

---

## Test Execution

### Running All Tests

```bash
# Using Node.js native test runner
cd /workspaces/memory-graph/agents/knowledge-graph-builder
node --test tests/*.test.ts

# Or run specific test file
node --test tests/contract.test.ts
node --test tests/extractor.test.ts
node --test tests/confidence.test.ts
node --test tests/decision-event.test.ts
node --test tests/integration.test.ts
```

### Test Files Summary

| File | Purpose | Test Count |
|------|---------|------------|
| `contract.test.ts` | Input/output schema validation | ~40 tests |
| `extractor.test.ts` | Knowledge extraction functions | ~30 tests |
| `confidence.test.ts` | Confidence scoring algorithms | ~35 tests |
| `decision-event.test.ts` | DecisionEvent structure validation | ~35 tests |
| `integration.test.ts` | End-to-end pipeline tests | ~25 tests |

---

## Manual Verification Procedure

### Step 1: Schema Compliance Check

```typescript
// Verify input validates
const inputValidation = validateInput(input);
assert(inputValidation.valid);

// Verify output validates
const outputValidation = validateOutput(output);
assert(outputValidation.valid);

// Verify DecisionEvent validates
const eventValidation = validateDecisionEvent(event);
assert(eventValidation.valid);
```

### Step 2: Side Effect Audit

Review agent code for:
- [ ] No `fetch()` calls except to ruvector-service
- [ ] No `fs.writeFile()` or file system modifications
- [ ] No `process.exit()` or process control
- [ ] No `child_process` spawning
- [ ] No direct database writes (only through ruvector client)

### Step 3: Determinism Verification

```typescript
const result1 = await agent.execute(testInput);
const result2 = await agent.execute(testInput);

// These MUST match
assert.strictEqual(result1.decisionEvent.inputs_hash, result2.decisionEvent.inputs_hash);
assert.strictEqual(result1.output.metadata.concepts_extracted, result2.output.metadata.concepts_extracted);

// These will differ (expected)
assert.notStrictEqual(result1.decisionEvent.execution_ref, result2.decisionEvent.execution_ref);
```

### Step 4: DecisionEvent Emission Count

```typescript
// Mock the ruvector client
const mockPersist = jest.fn();
const agent = new KnowledgeGraphBuilderAgent(mockPersist);

await agent.execute(input);

// MUST be called exactly once
assert.strictEqual(mockPersist.mock.calls.length, 1);
```

---

## Checklist Sign-off

| # | Requirement | Verified | Date | Verifier |
|---|-------------|----------|------|----------|
| 1 | Emits exactly ONE DecisionEvent per invocation | [ ] | | |
| 2 | Does NOT modify runtime execution | [ ] | | |
| 3 | Does NOT enforce policies | [ ] | | |
| 4 | Does NOT orchestrate workflows | [ ] | | |
| 5 | Returns deterministic output | [ ] | | |
| 6 | All inputs/outputs validate against agentics-contracts | [ ] | | |

---

## Appendix: Test Fixtures

### Sample Input (`fixtures/sample-input.json`)
Location: `/workspaces/memory-graph/agents/knowledge-graph-builder/tests/fixtures/sample-input.json`

Contains:
- Valid session IDs (UUIDs)
- Complete extraction config
- Multi-turn conversation data

### Expected Output (`fixtures/expected-output.json`)
Location: `/workspaces/memory-graph/agents/knowledge-graph-builder/tests/fixtures/expected-output.json`

Contains:
- Sample knowledge graph structure
- Expected concepts, entities, relationships
- Metadata format example
- DecisionEvent template

---

## Related Documentation

- **Agent Contract**: `/contracts/agentics-contracts/schemas/decision-event.schema.json`
- **Memory Schema**: `/contracts/agentics-contracts/schemas/decision-memory.schema.json`
- **Type Definitions**: `/contracts/agentics-contracts/index.ts`

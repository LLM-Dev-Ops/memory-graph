# Knowledge Graph Builder Agent

**Classification:** MEMORY_ANALYSIS
**Decision Type:** knowledge_graph_construction
**Version:** 1.0.0

## Purpose

The Knowledge Graph Builder Agent constructs and maintains higher-order knowledge graphs from memory nodes and relationships stored in ruvector-service. It analyzes existing conversation and decision memory to extract:

- **Concepts** - Abstract ideas and themes
- **Entities** - Named entities (people, organizations, technologies, etc.)
- **Relationships** - Semantic connections between knowledge elements
- **Patterns** - Recurring behavioral, conversational, and structural patterns

## Contract Location

The full contract is defined in `contract.ts` and includes:

1. **Input Schema** (`KnowledgeGraphBuilderInput`)
   - Session IDs to analyze
   - Query filters for node selection
   - Extraction configuration
   - Traversal depth limits

2. **Output Schema** (`KnowledgeGraphBuilderOutput`)
   - Knowledge nodes created
   - Knowledge edges created
   - Patterns detected
   - Graph metrics

3. **Node Types** (8 types)
   - concept, entity, relationship, pattern
   - cluster, topic, fact, reference

4. **Edge Types** (13 types)
   - relates_to, derived_from, co_occurs_with, exemplifies
   - generalizes, part_of, contains, causes
   - precedes, contradicts, supports, similar_to, references

5. **DecisionEvent** mapping for audit and telemetry

6. **CLI Contract** for inspect, retrieve, and replay commands

## Explicit Non-Responsibilities

This agent is classified as MEMORY_ANALYSIS and adheres to strict constraints:

- MUST NOT modify system behavior
- MUST NOT trigger remediation
- MUST NOT emit alerts
- MUST NOT enforce policies
- MUST NOT orchestrate workflows
- MUST NOT connect directly to SQL
- MUST NOT make runtime decisions
- MUST NOT generate or execute code

## Usage

```typescript
import {
  KnowledgeGraphBuilderInputSchema,
  validateInput,
  AGENT_ID,
  AGENT_VERSION,
} from './contract.js';

// Validate input
const input = {
  session_ids: ['uuid-1', 'uuid-2'],
  extraction_config: {
    extract_concepts: true,
    extract_entities: true,
    confidence_threshold: 0.7,
  },
  max_depth: 3,
};

const result = validateInput(input);
if (result.success) {
  // Input is valid, proceed with processing
}
```

## Confidence Semantics

The agent emits a confidence score (0-1) based on:

| Range | Interpretation |
|-------|----------------|
| 0.9-1.0 | High-quality extraction with strong evidence |
| 0.7-0.9 | Good extraction with moderate evidence |
| 0.5-0.7 | Acceptable extraction, may need review |
| <0.5 | Low confidence, included due to relaxed thresholds |

## Failure Modes

| Error Code | Trigger | Retryable |
|------------|---------|-----------|
| SESSION_NOT_FOUND | Session ID does not exist | No |
| EMPTY_RESULT | No knowledge extracted | No |
| CONFIDENCE_THRESHOLD_FAIL | All results below threshold | Yes |
| RUVECTOR_CONNECTION_ERROR | Cannot connect to ruvector | Yes |
| RUVECTOR_READ_ERROR | Error reading from ruvector | Yes |
| RUVECTOR_WRITE_ERROR | Error writing to ruvector | Yes |
| RESOURCE_EXHAUSTED | Memory/time limit exceeded | Yes |
| MERGE_CONFLICT | Unresolvable merge conflict | No |

## Directory Structure

```
agents/knowledge-graph-builder/
├── contract.ts    # Full contract definition
├── README.md      # This file
└── src/           # Implementation (future)
    ├── agent.ts
    ├── types.ts
    └── index.ts
```

## Related Contracts

- `contracts/agentics-contracts/` - Base contract schemas
- `agents/conversation-memory/` - Source memory agent
- `agents/prompt-lineage/` - Lineage tracking agent

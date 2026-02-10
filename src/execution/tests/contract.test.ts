/**
 * Contract tests for FEU output
 *
 * Verifies the complete ExecutionResponse can be JSON-serialized,
 * round-tripped, and validated against the schema. Also verifies
 * the causal ordering invariant (all parent_span_ids form a valid tree).
 */

import { describe, it, expect } from 'vitest';
import { ExecutionUnitOrchestrator } from '../executor.js';
import { ExecutionResponseSchema, AgentSpanSchema, RepoSpanSchema } from '../types.js';
import type { AgentExecutionResult } from '../agent-adapter.js';

// ============================================================================
// HELPERS
// ============================================================================

function makeOrchestrator(
  agents: Record<string, (input: unknown) => Promise<AgentExecutionResult>>,
) {
  const adapterConfigs: Record<string, { executeFn: (input: unknown) => Promise<AgentExecutionResult> }> = {};
  for (const [agentId, fn] of Object.entries(agents)) {
    adapterConfigs[agentId] = { executeFn: fn };
  }
  return new ExecutionUnitOrchestrator({ adapterConfigs });
}

const successAgent = async (): Promise<AgentExecutionResult> => ({
  success: true,
  output: {
    nodes_created: [
      { node_id: crypto.randomUUID(), node_type: 'prompt' },
      { node_id: crypto.randomUUID(), node_type: 'response' },
    ],
    edges_created: [
      {
        edge_id: crypto.randomUUID(),
        edge_type: 'responds_to',
        from_node_id: crypto.randomUUID(),
        to_node_id: crypto.randomUUID(),
      },
    ],
  },
  decisionEvent: { execution_ref: crypto.randomUUID() },
});

const failAgent = async (): Promise<AgentExecutionResult> => ({
  success: false,
  error: { error_code: 'INTERNAL_ERROR', message: 'Simulated failure' },
});

// ============================================================================
// FULL RESPONSE SCHEMA VALIDATION
// ============================================================================

describe('ExecutionResponse schema compliance', () => {
  it('should validate success response against schema', async () => {
    const orchestrator = makeOrchestrator({ 'conversation-memory-agent': successAgent });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const validation = ExecutionResponseSchema.safeParse(result.response);
      expect(validation.success).toBe(true);
    }
  });

  it('should validate failure response against schema', async () => {
    const orchestrator = makeOrchestrator({ 'conversation-memory-agent': failAgent });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const validation = ExecutionResponseSchema.safeParse(result.response);
      expect(validation.success).toBe(true);
    }
  });

  it('should validate mixed success/failure response against schema', async () => {
    const orchestrator = makeOrchestrator({
      'conversation-memory-agent': successAgent,
      'long-term-pattern-agent': failAgent,
    });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [
        { agent_id: 'conversation-memory-agent', input: {} },
        { agent_id: 'long-term-pattern-agent', input: {} },
      ],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const validation = ExecutionResponseSchema.safeParse(result.response);
      expect(validation.success).toBe(true);
    }
  });
});

// ============================================================================
// JSON ROUND-TRIP
// ============================================================================

describe('JSON round-trip', () => {
  it('should survive JSON serialization without data loss', async () => {
    const orchestrator = makeOrchestrator({ 'conversation-memory-agent': successAgent });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const json = JSON.stringify(result.response);
      const roundTripped = JSON.parse(json);

      // Validate round-tripped data against schema
      const validation = ExecutionResponseSchema.safeParse(roundTripped);
      expect(validation.success).toBe(true);

      // Key fields preserved
      expect(roundTripped.execution_id).toBe(result.response.execution_id);
      expect(roundTripped.repo_span.span_type).toBe('repo');
      expect(roundTripped.repo_span.repo_name).toBe('memory-graph');
      expect(roundTripped.repo_span.agent_spans).toHaveLength(1);
      expect(roundTripped.repo_span.agent_spans[0].span_type).toBe('agent');
    }
  });
});

// ============================================================================
// CAUSAL ORDERING (PARENT CHAIN)
// ============================================================================

describe('Causal ordering via parent_span_id', () => {
  it('should form a valid tree: Core -> Repo -> Agent(s)', async () => {
    const parentSpanId = crypto.randomUUID();

    const orchestrator = makeOrchestrator({
      'conversation-memory-agent': successAgent,
      'long-term-pattern-agent': successAgent,
    });

    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: parentSpanId,
      agents: [
        { agent_id: 'conversation-memory-agent', input: {} },
        { agent_id: 'long-term-pattern-agent', input: {} },
      ],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const repoSpan = result.response.repo_span;

      // Repo span points to Core's span
      expect(repoSpan.parent_span_id).toBe(parentSpanId);

      // All agent spans point to repo span
      for (const agentSpan of repoSpan.agent_spans) {
        expect(agentSpan.parent_span_id).toBe(repoSpan.span_id);
      }

      // No two spans share the same span_id
      const allSpanIds = [repoSpan.span_id, ...repoSpan.agent_spans.map((s) => s.span_id)];
      const uniqueIds = new Set(allSpanIds);
      expect(uniqueIds.size).toBe(allSpanIds.length);
    }
  });
});

// ============================================================================
// APPEND-ONLY INVARIANT
// ============================================================================

describe('Append-only spans', () => {
  it('should not have duplicate span_ids', async () => {
    const orchestrator = makeOrchestrator({
      'conversation-memory-agent': successAgent,
      'long-term-pattern-agent': successAgent,
      'knowledge-graph-builder-agent': successAgent,
    });

    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [
        { agent_id: 'conversation-memory-agent', input: {} },
        { agent_id: 'long-term-pattern-agent', input: {} },
        { agent_id: 'knowledge-graph-builder-agent', input: {} },
      ],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const allIds = [
        result.response.repo_span.span_id,
        ...result.response.repo_span.agent_spans.map((s) => s.span_id),
      ];
      expect(new Set(allIds).size).toBe(allIds.length);
    }
  });
});

// ============================================================================
// ARTIFACT ATTACHMENT LEVEL
// ============================================================================

describe('Artifacts at correct level', () => {
  it('should only have artifacts on agent spans, not repo span', async () => {
    const orchestrator = makeOrchestrator({ 'conversation-memory-agent': successAgent });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      // Repo span should not have an artifacts field
      expect((result.response.repo_span as Record<string, unknown>)['artifacts']).toBeUndefined();

      // Agent spans should have artifacts
      const agentSpan = result.response.repo_span.agent_spans[0];
      expect(agentSpan.artifacts).toBeDefined();
      expect(Array.isArray(agentSpan.artifacts)).toBe(true);
    }
  });

  it('should have non-empty artifacts for successful agents with output', async () => {
    const orchestrator = makeOrchestrator({ 'conversation-memory-agent': successAgent });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const agentSpan = result.response.repo_span.agent_spans[0];
      expect(agentSpan.artifacts.length).toBeGreaterThan(0);
    }
  });

  it('should have empty artifacts for failed agents', async () => {
    const orchestrator = makeOrchestrator({ 'conversation-memory-agent': failAgent });
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      const agentSpan = result.response.repo_span.agent_spans[0];
      expect(agentSpan.artifacts).toHaveLength(0);
    }
  });
});

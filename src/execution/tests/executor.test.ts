/**
 * Tests for ExecutionUnitOrchestrator
 *
 * Verifies all FEU enforcement rules:
 * - Reject missing parent_span_id (rule 2)
 * - Create repo-level span (rule 3)
 * - Create agent-level spans (rule 4)
 * - No success without agent spans (rule 6)
 * - Always return spans on failure (rule 7)
 * - Never return success if any agent ran without a span (rule 9)
 */

import { describe, it, expect } from 'vitest';
import { ExecutionUnitOrchestrator } from '../executor.js';
import type { OrchestratorConfig } from '../executor.js';
import type { AgentExecutionResult } from '../agent-adapter.js';

// ============================================================================
// HELPERS
// ============================================================================

function makeConfig(agents: Record<string, (input: unknown) => Promise<AgentExecutionResult>>): OrchestratorConfig {
  const adapterConfigs: Record<string, { executeFn: (input: unknown) => Promise<AgentExecutionResult> }> = {};
  for (const [agentId, fn] of Object.entries(agents)) {
    adapterConfigs[agentId] = { executeFn: fn };
  }
  return { adapterConfigs };
}

const successAgent = async (): Promise<AgentExecutionResult> => ({
  success: true,
  output: { nodes_created: [{ node_id: crypto.randomUUID(), node_type: 'prompt' }] },
  decisionEvent: { execution_ref: crypto.randomUUID() },
});

const failingAgent = async (): Promise<AgentExecutionResult> => ({
  success: false,
  error: { error_code: 'TEST_FAILURE', message: 'Agent failed on purpose' },
});

const throwingAgent = async (): Promise<AgentExecutionResult> => {
  throw new Error('Unexpected crash');
};

function validRequest(agentIds: string[] = ['conversation-memory-agent']) {
  return {
    execution_id: crypto.randomUUID(),
    parent_span_id: crypto.randomUUID(),
    agents: agentIds.map((id) => ({ agent_id: id, input: {} })),
  };
}

// ============================================================================
// RULE 2: Reject missing parent_span_id
// ============================================================================

describe('Rule 2: Reject missing parent_span_id', () => {
  it('should reject request without parent_span_id', async () => {
    const orchestrator = new ExecutionUnitOrchestrator();
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test', input: {} }],
    });

    expect(result.ok).toBe(false);
    if (!result.ok) {
      expect(result.error.error_code).toBe('VALIDATION_ERROR');
    }
  });

  it('should reject request without execution_id', async () => {
    const orchestrator = new ExecutionUnitOrchestrator();
    const result = await orchestrator.execute({
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test', input: {} }],
    });

    expect(result.ok).toBe(false);
  });

  it('should reject request with non-UUID parent_span_id', async () => {
    const orchestrator = new ExecutionUnitOrchestrator();
    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: 'not-valid',
      agents: [{ agent_id: 'test', input: {} }],
    });

    expect(result.ok).toBe(false);
  });

  it('should reject completely empty request', async () => {
    const orchestrator = new ExecutionUnitOrchestrator();
    const result = await orchestrator.execute({});
    expect(result.ok).toBe(false);
  });
});

// ============================================================================
// RULE 3: Create repo-level span
// ============================================================================

describe('Rule 3: Create repo-level span', () => {
  it('should create repo span with correct type and name', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.span_type).toBe('repo');
      expect(result.response.repo_span.repo_name).toBe('memory-graph');
    }
  });

  it('should set parent_span_id from request', async () => {
    const parentSpanId = crypto.randomUUID();
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: parentSpanId,
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.parent_span_id).toBe(parentSpanId);
    }
  });

  it('should include execution_id in repo span', async () => {
    const executionId = crypto.randomUUID();
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute({
      execution_id: executionId,
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'conversation-memory-agent', input: {} }],
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.execution_id).toBe(executionId);
      expect(result.response.execution_id).toBe(executionId);
    }
  });
});

// ============================================================================
// RULE 4: Agent-level spans
// ============================================================================

describe('Rule 4: Agent-level spans', () => {
  it('should create one agent span per agent', async () => {
    const config = makeConfig({
      'conversation-memory-agent': successAgent,
      'long-term-pattern-agent': successAgent,
    });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(
      validRequest(['conversation-memory-agent', 'long-term-pattern-agent']),
    );

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.agent_spans).toHaveLength(2);
    }
  });

  it('should set agent span parent_span_id to repo span_id', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      const repoSpanId = result.response.repo_span.span_id;
      for (const agentSpan of result.response.repo_span.agent_spans) {
        expect(agentSpan.parent_span_id).toBe(repoSpanId);
      }
    }
  });

  it('should set span_type to agent for all agent spans', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      for (const agentSpan of result.response.repo_span.agent_spans) {
        expect(agentSpan.span_type).toBe('agent');
      }
    }
  });

  it('should record duration_ms for each agent span', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      for (const agentSpan of result.response.repo_span.agent_spans) {
        expect(agentSpan.duration_ms).toBeGreaterThanOrEqual(0);
        expect(agentSpan.started_at).toBeTruthy();
        expect(agentSpan.ended_at).toBeTruthy();
      }
    }
  });
});

// ============================================================================
// RULE 6: No success without agent spans
// ============================================================================

describe('Rule 6: No success without agent spans', () => {
  it('should handle unknown agent with error span', async () => {
    const orchestrator = new ExecutionUnitOrchestrator();

    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'nonexistent-agent', input: {} }],
    });

    // Should still produce a result (with error span for unknown agent)
    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.agent_spans).toHaveLength(1);
      expect(result.response.repo_span.agent_spans[0].status).toBe('error');
      expect(result.response.success).toBe(false);
    }
  });
});

// ============================================================================
// RULE 7: Always return spans on failure
// ============================================================================

describe('Rule 7: Always return spans on failure', () => {
  it('should emit error span when agent returns failure', async () => {
    const config = makeConfig({ 'conversation-memory-agent': failingAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.agent_spans).toHaveLength(1);
      expect(result.response.repo_span.agent_spans[0].status).toBe('error');
      expect(result.response.repo_span.agent_spans[0].error).toBeDefined();
      expect(result.response.repo_span.agent_spans[0].error?.error_code).toBe('TEST_FAILURE');
    }
  });

  it('should emit error span when agent throws exception', async () => {
    const config = makeConfig({ 'conversation-memory-agent': throwingAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.agent_spans).toHaveLength(1);
      expect(result.response.repo_span.agent_spans[0].status).toBe('error');
      expect(result.response.repo_span.agent_spans[0].error?.message).toContain('Unexpected crash');
    }
  });

  it('should include all spans when mixed success/failure', async () => {
    const config = makeConfig({
      'conversation-memory-agent': successAgent,
      'long-term-pattern-agent': failingAgent,
    });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(
      validRequest(['conversation-memory-agent', 'long-term-pattern-agent']),
    );

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.agent_spans).toHaveLength(2);
      expect(result.response.repo_span.status).toBe('error'); // repo status is error if any agent failed
      expect(result.response.success).toBe(false);
      expect(result.response.summary.succeeded).toBe(1);
      expect(result.response.summary.failed).toBe(1);
    }
  });

  it('should mark repo span as FAILED when any agent fails', async () => {
    const config = makeConfig({ 'conversation-memory-agent': failingAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.repo_span.status).toBe('error');
    }
  });
});

// ============================================================================
// FAIL-FAST OPTION
// ============================================================================

describe('Fail-fast option', () => {
  it('should stop on first error when fail_fast is true', async () => {
    const config = makeConfig({
      'conversation-memory-agent': failingAgent,
      'long-term-pattern-agent': successAgent,
    });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute({
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [
        { agent_id: 'conversation-memory-agent', input: {} },
        { agent_id: 'long-term-pattern-agent', input: {} },
      ],
      options: { fail_fast: true },
    });

    expect(result.ok).toBe(true);
    if (result.ok) {
      // Only the first agent should have run
      expect(result.response.repo_span.agent_spans).toHaveLength(1);
      expect(result.response.repo_span.agent_spans[0].agent_id).toBe('conversation-memory-agent');
    }
  });
});

// ============================================================================
// SUMMARY
// ============================================================================

describe('Execution summary', () => {
  it('should compute correct summary for all-success', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      expect(result.response.summary.total_agents).toBe(1);
      expect(result.response.summary.succeeded).toBe(1);
      expect(result.response.summary.failed).toBe(0);
      expect(result.response.summary.total_duration_ms).toBeGreaterThanOrEqual(0);
    }
  });

  it('should count artifacts in summary', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      const totalArtifacts = result.response.repo_span.agent_spans.reduce(
        (sum, s) => sum + s.artifacts.length,
        0,
      );
      expect(result.response.summary.total_artifacts).toBe(totalArtifacts);
    }
  });
});

// ============================================================================
// SPAN ID VALIDITY
// ============================================================================

describe('Span ID validity', () => {
  it('should generate valid UUIDs for all span IDs', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
      expect(result.response.repo_span.span_id).toMatch(uuidRegex);

      for (const agentSpan of result.response.repo_span.agent_spans) {
        expect(agentSpan.span_id).toMatch(uuidRegex);
        expect(agentSpan.parent_span_id).toMatch(uuidRegex);
      }
    }
  });
});

// ============================================================================
// JSON SERIALIZATION
// ============================================================================

describe('JSON serialization', () => {
  it('should produce JSON-serializable output without loss', async () => {
    const config = makeConfig({ 'conversation-memory-agent': successAgent });
    const orchestrator = new ExecutionUnitOrchestrator(config);

    const result = await orchestrator.execute(validRequest());

    expect(result.ok).toBe(true);
    if (result.ok) {
      const json = JSON.stringify(result.response);
      const parsed = JSON.parse(json);
      expect(parsed.execution_id).toBe(result.response.execution_id);
      expect(parsed.repo_span.span_type).toBe('repo');
      expect(parsed.repo_span.agent_spans).toHaveLength(1);
    }
  });
});

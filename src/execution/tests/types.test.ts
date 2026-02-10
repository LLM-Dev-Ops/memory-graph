/**
 * Tests for FEU type schemas
 *
 * Verifies that Zod schemas correctly validate and reject inputs,
 * enforcing all FEU invariants at the type level.
 */

import { describe, it, expect } from 'vitest';
import {
  ExecutionRequestSchema,
  RepoSpanSchema,
  AgentSpanSchema,
  ArtifactReferenceSchema,
  ExecutionResponseSchema,
  SpanStatusSchema,
  SpanErrorSchema,
} from '../types.js';

// ============================================================================
// HELPERS
// ============================================================================

function validAgentSpan(overrides: Record<string, unknown> = {}) {
  return {
    span_id: crypto.randomUUID(),
    parent_span_id: crypto.randomUUID(),
    span_type: 'agent',
    agent_id: 'test-agent',
    agent_version: '1.0.0',
    status: 'ok',
    started_at: new Date().toISOString(),
    ended_at: new Date().toISOString(),
    duration_ms: 100,
    artifacts: [],
    ...overrides,
  };
}

function validRepoSpan(overrides: Record<string, unknown> = {}) {
  return {
    span_id: crypto.randomUUID(),
    parent_span_id: crypto.randomUUID(),
    execution_id: crypto.randomUUID(),
    span_type: 'repo',
    repo_name: 'memory-graph',
    status: 'ok',
    started_at: new Date().toISOString(),
    ended_at: new Date().toISOString(),
    duration_ms: 200,
    agent_spans: [validAgentSpan()],
    ...overrides,
  };
}

// ============================================================================
// SPAN STATUS
// ============================================================================

describe('SpanStatusSchema', () => {
  it('should accept valid statuses', () => {
    expect(SpanStatusSchema.parse('ok')).toBe('ok');
    expect(SpanStatusSchema.parse('error')).toBe('error');
    expect(SpanStatusSchema.parse('timeout')).toBe('timeout');
  });

  it('should reject invalid status', () => {
    expect(() => SpanStatusSchema.parse('running')).toThrow();
  });
});

// ============================================================================
// ARTIFACT REFERENCE
// ============================================================================

describe('ArtifactReferenceSchema', () => {
  it('should accept valid artifact', () => {
    const artifact = {
      artifact_ref: 'memory-graph/test-agent/abc/nodes/0',
      artifact_type: 'nodes_created',
      content_hash: 'a'.repeat(64),
      size_bytes: 1024,
      content: [{ id: '1' }],
    };
    expect(() => ArtifactReferenceSchema.parse(artifact)).not.toThrow();
  });

  it('should reject content_hash with wrong length', () => {
    const artifact = {
      artifact_ref: 'memory-graph/test-agent/abc/nodes/0',
      artifact_type: 'nodes_created',
      content_hash: 'tooshort',
      size_bytes: 1024,
      content: [],
    };
    expect(() => ArtifactReferenceSchema.parse(artifact)).toThrow();
  });

  it('should reject empty artifact_ref', () => {
    const artifact = {
      artifact_ref: '',
      artifact_type: 'nodes_created',
      content_hash: 'a'.repeat(64),
      size_bytes: 0,
      content: null,
    };
    expect(() => ArtifactReferenceSchema.parse(artifact)).toThrow();
  });
});

// ============================================================================
// AGENT SPAN
// ============================================================================

describe('AgentSpanSchema', () => {
  it('should accept valid agent span', () => {
    const span = validAgentSpan();
    expect(() => AgentSpanSchema.parse(span)).not.toThrow();
  });

  it('should accept agent span with error', () => {
    const span = validAgentSpan({
      status: 'error',
      error: {
        error_code: 'VALIDATION_ERROR',
        message: 'Input invalid',
      },
    });
    expect(() => AgentSpanSchema.parse(span)).not.toThrow();
  });

  it('should accept agent span with artifacts', () => {
    const span = validAgentSpan({
      artifacts: [
        {
          artifact_ref: 'memory-graph/test/abc/nodes/0',
          artifact_type: 'nodes_created',
          content_hash: 'b'.repeat(64),
          size_bytes: 512,
          content: [],
        },
      ],
    });
    expect(() => AgentSpanSchema.parse(span)).not.toThrow();
  });

  it('should reject span_type other than agent', () => {
    const span = validAgentSpan({ span_type: 'repo' });
    expect(() => AgentSpanSchema.parse(span)).toThrow();
  });

  it('should reject missing span_id', () => {
    const span = validAgentSpan();
    delete (span as Record<string, unknown>).span_id;
    expect(() => AgentSpanSchema.parse(span)).toThrow();
  });

  it('should reject invalid agent_version format', () => {
    const span = validAgentSpan({ agent_version: 'v1' });
    expect(() => AgentSpanSchema.parse(span)).toThrow();
  });

  it('should reject non-UUID span_id', () => {
    const span = validAgentSpan({ span_id: 'not-a-uuid' });
    expect(() => AgentSpanSchema.parse(span)).toThrow();
  });
});

// ============================================================================
// REPO SPAN
// ============================================================================

describe('RepoSpanSchema', () => {
  it('should accept valid repo span with agent spans', () => {
    const span = validRepoSpan();
    expect(() => RepoSpanSchema.parse(span)).not.toThrow();
  });

  it('should reject repo span with empty agent_spans array (INVARIANT)', () => {
    const span = validRepoSpan({ agent_spans: [] });
    expect(() => RepoSpanSchema.parse(span)).toThrow();
  });

  it('should reject repo span with wrong repo_name', () => {
    const span = validRepoSpan({ repo_name: 'other-repo' });
    expect(() => RepoSpanSchema.parse(span)).toThrow();
  });

  it('should reject repo span with wrong span_type', () => {
    const span = validRepoSpan({ span_type: 'agent' });
    expect(() => RepoSpanSchema.parse(span)).toThrow();
  });

  it('should reject repo span without parent_span_id', () => {
    const span = validRepoSpan();
    delete (span as Record<string, unknown>).parent_span_id;
    expect(() => RepoSpanSchema.parse(span)).toThrow();
  });

  it('should reject repo span with non-UUID parent_span_id', () => {
    const span = validRepoSpan({ parent_span_id: 'not-a-uuid' });
    expect(() => RepoSpanSchema.parse(span)).toThrow();
  });
});

// ============================================================================
// EXECUTION REQUEST
// ============================================================================

describe('ExecutionRequestSchema', () => {
  it('should accept valid request', () => {
    const request = {
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test-agent', input: {} }],
    };
    expect(() => ExecutionRequestSchema.parse(request)).not.toThrow();
  });

  it('should reject request without execution_id', () => {
    const request = {
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test-agent', input: {} }],
    };
    expect(() => ExecutionRequestSchema.parse(request)).toThrow();
  });

  it('should reject request without parent_span_id (ENFORCEMENT RULE 2)', () => {
    const request = {
      execution_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test-agent', input: {} }],
    };
    expect(() => ExecutionRequestSchema.parse(request)).toThrow();
  });

  it('should reject request with non-UUID execution_id', () => {
    const request = {
      execution_id: 'not-a-uuid',
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test-agent', input: {} }],
    };
    expect(() => ExecutionRequestSchema.parse(request)).toThrow();
  });

  it('should reject request with non-UUID parent_span_id', () => {
    const request = {
      execution_id: crypto.randomUUID(),
      parent_span_id: 'invalid',
      agents: [{ agent_id: 'test-agent', input: {} }],
    };
    expect(() => ExecutionRequestSchema.parse(request)).toThrow();
  });

  it('should reject request with empty agents array', () => {
    const request = {
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [],
    };
    expect(() => ExecutionRequestSchema.parse(request)).toThrow();
  });

  it('should accept request with options', () => {
    const request = {
      execution_id: crypto.randomUUID(),
      parent_span_id: crypto.randomUUID(),
      agents: [{ agent_id: 'test-agent', input: {} }],
      options: { timeout_ms: 5000, fail_fast: true },
    };
    expect(() => ExecutionRequestSchema.parse(request)).not.toThrow();
  });
});

// ============================================================================
// EXECUTION RESPONSE
// ============================================================================

describe('ExecutionResponseSchema', () => {
  it('should accept valid response', () => {
    const response = {
      execution_id: crypto.randomUUID(),
      repo_span: validRepoSpan(),
      success: true,
      summary: {
        total_agents: 1,
        succeeded: 1,
        failed: 0,
        total_artifacts: 0,
        total_duration_ms: 200,
      },
    };
    expect(() => ExecutionResponseSchema.parse(response)).not.toThrow();
  });

  it('should reject response with empty agent_spans (INVARIANT)', () => {
    const response = {
      execution_id: crypto.randomUUID(),
      repo_span: validRepoSpan({ agent_spans: [] }),
      success: true,
      summary: {
        total_agents: 0,
        succeeded: 0,
        failed: 0,
        total_artifacts: 0,
        total_duration_ms: 200,
      },
    };
    expect(() => ExecutionResponseSchema.parse(response)).toThrow();
  });
});

// ============================================================================
// SPAN ERROR
// ============================================================================

describe('SpanErrorSchema', () => {
  it('should accept error with code and message', () => {
    const error = { error_code: 'TEST_ERROR', message: 'Something failed' };
    expect(() => SpanErrorSchema.parse(error)).not.toThrow();
  });

  it('should accept error with details', () => {
    const error = {
      error_code: 'VALIDATION_ERROR',
      message: 'Invalid input',
      details: { field: 'name', reason: 'required' },
    };
    expect(() => SpanErrorSchema.parse(error)).not.toThrow();
  });
});

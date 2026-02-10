/**
 * Agent Adapters
 *
 * Provides a uniform interface for invoking any agent in the memory-graph repo
 * and wrapping its result into an AgentSpan. The orchestrator uses these adapters
 * so it doesn't need to know the internal structure of each agent.
 *
 * TS agents are invoked directly via their executeAsUnit() export.
 * Rust agents are invoked via HTTP to their warp endpoints.
 */

import type { AgentSpan, ArtifactReference, SpanError } from './types.js';
import { buildArtifacts } from './artifact-builder.js';

// ============================================================================
// AGENT ADAPTER INTERFACE
// ============================================================================

/**
 * Result from an agent's execution, before wrapping into an AgentSpan.
 */
export interface AgentExecutionResult {
  success: boolean;
  output?: unknown;
  decisionEvent?: { execution_ref: string; [key: string]: unknown };
  error?: { error_code: string; message: string; details?: Record<string, unknown> };
}

/**
 * Uniform adapter interface. Each adapter knows how to invoke a specific agent
 * and return a raw result. The factory wraps these into AgentSpan objects.
 */
export interface AgentAdapter {
  readonly agentId: string;
  readonly agentVersion: string;
  execute(input: unknown): Promise<AgentExecutionResult>;
}

// ============================================================================
// SPAN BUILDER
// ============================================================================

/**
 * Wrap an agent adapter's execution into a full AgentSpan with artifacts.
 * This is the central function that ensures every agent call produces a span,
 * regardless of success or failure.
 */
export async function executeAndBuildSpan(
  adapter: AgentAdapter,
  input: unknown,
  executionId: string,
  repoSpanId: string,
): Promise<AgentSpan> {
  const spanId = crypto.randomUUID();
  const startedAt = new Date().toISOString();
  const startTime = Date.now();

  let result: AgentExecutionResult;
  try {
    result = await adapter.execute(input);
  } catch (err) {
    // Agent threw -- produce an error span. Spans must ALWAYS be emitted.
    const endedAt = new Date().toISOString();
    return {
      span_id: spanId,
      parent_span_id: repoSpanId,
      span_type: 'agent',
      agent_id: adapter.agentId,
      agent_version: adapter.agentVersion,
      status: 'error',
      started_at: startedAt,
      ended_at: endedAt,
      duration_ms: Date.now() - startTime,
      error: {
        error_code: 'AGENT_EXCEPTION',
        message: err instanceof Error ? err.message : String(err),
      },
      artifacts: [],
    };
  }

  const endedAt = new Date().toISOString();
  const durationMs = Date.now() - startTime;

  if (result.success) {
    const artifacts = await buildArtifacts(adapter.agentId, executionId, result.output);

    return {
      span_id: spanId,
      parent_span_id: repoSpanId,
      span_type: 'agent',
      agent_id: adapter.agentId,
      agent_version: adapter.agentVersion,
      status: 'ok',
      started_at: startedAt,
      ended_at: endedAt,
      duration_ms: durationMs,
      artifacts,
      decision_event_ref: result.decisionEvent?.execution_ref,
    };
  }

  // Agent returned failure -- still emit the span with error details.
  const errorInfo: SpanError = {
    error_code: result.error?.error_code ?? 'UNKNOWN_ERROR',
    message: result.error?.message ?? 'Agent returned failure without error details',
    details: result.error?.details,
  };

  return {
    span_id: spanId,
    parent_span_id: repoSpanId,
    span_type: 'agent',
    agent_id: adapter.agentId,
    agent_version: adapter.agentVersion,
    status: 'error',
    started_at: startedAt,
    ended_at: endedAt,
    duration_ms: durationMs,
    error: errorInfo,
    artifacts: [],
  };
}

// ============================================================================
// CONCRETE ADAPTERS: TypeScript Agents
// ============================================================================

/**
 * Base adapter for TypeScript agents that expose an executeAsUnit function.
 * The executeAsUnit functions are dynamically imported from each agent package.
 */
class TypeScriptAgentAdapter implements AgentAdapter {
  constructor(
    readonly agentId: string,
    readonly agentVersion: string,
    private readonly executeFn: (input: unknown) => Promise<AgentExecutionResult>,
  ) {}

  async execute(input: unknown): Promise<AgentExecutionResult> {
    return this.executeFn(input);
  }
}

/**
 * Adapter for Rust agents accessible via HTTP endpoints.
 * Makes an HTTP POST to the agent's endpoint and converts the response.
 */
class HttpAgentAdapter implements AgentAdapter {
  constructor(
    readonly agentId: string,
    readonly agentVersion: string,
    private readonly baseUrl: string,
    private readonly endpoint: string,
  ) {}

  async execute(input: unknown): Promise<AgentExecutionResult> {
    const url = `${this.baseUrl}${this.endpoint}`;

    const response = await fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ input }),
    });

    const body = await response.json() as Record<string, unknown>;

    if (response.ok && body.data) {
      return {
        success: true,
        output: body.data,
        decisionEvent: body.execution_ref
          ? { execution_ref: body.execution_ref as string }
          : undefined,
      };
    }

    return {
      success: false,
      error: {
        error_code: (body.error_code as string) ?? 'HTTP_ERROR',
        message: (body.message as string) ?? `HTTP ${response.status}`,
      },
    };
  }
}

// ============================================================================
// ADAPTER REGISTRY
// ============================================================================

/**
 * Registry mapping agent IDs to their adapter factory functions.
 * TS agent adapters take a direct execute function; HTTP adapters take a URL.
 */
type AdapterFactory = (config?: AdapterConfig) => AgentAdapter;

export interface AdapterConfig {
  /** Base URL for HTTP-based agents (Rust agents) */
  httpBaseUrl?: string;
  /** Direct execute function for TS agents */
  executeFn?: (input: unknown) => Promise<AgentExecutionResult>;
}

const defaultAdapterFactories: Record<string, AdapterFactory> = {
  'conversation-memory-agent': (config) => {
    if (config?.executeFn) {
      return new TypeScriptAgentAdapter('conversation-memory-agent', '1.0.0', config.executeFn);
    }
    throw new Error('conversation-memory-agent requires executeFn in adapter config');
  },
  'long-term-pattern-agent': (config) => {
    if (config?.executeFn) {
      return new TypeScriptAgentAdapter('long-term-pattern-agent', '1.0.0', config.executeFn);
    }
    throw new Error('long-term-pattern-agent requires executeFn in adapter config');
  },
  'knowledge-graph-builder-agent': (config) => {
    if (config?.executeFn) {
      return new TypeScriptAgentAdapter('knowledge-graph-builder-agent', '1.0.0', config.executeFn);
    }
    throw new Error('knowledge-graph-builder-agent requires executeFn in adapter config');
  },
  'memory-retrieval-agent': (config) => {
    if (config?.executeFn) {
      return new TypeScriptAgentAdapter('memory-retrieval-agent', '1.0.0', config.executeFn);
    }
    throw new Error('memory-retrieval-agent requires executeFn in adapter config');
  },
  'decision-memory-agent': (config) => {
    const baseUrl = config?.httpBaseUrl ?? process.env['DECISION_MEMORY_URL'] ?? 'http://localhost:8080';
    return new HttpAgentAdapter('decision-memory-agent', '1.0.0', baseUrl, '/api/v1/capture');
  },
  'prompt-lineage-agent': (config) => {
    const baseUrl = config?.httpBaseUrl ?? process.env['PROMPT_LINEAGE_URL'] ?? 'http://localhost:8081';
    return new HttpAgentAdapter('prompt-lineage-agent', '1.0.0', baseUrl, '/api/v1/track');
  },
};

/**
 * Create an agent adapter by ID with optional configuration overrides.
 */
export function createAgentAdapter(agentId: string, config?: AdapterConfig): AgentAdapter {
  const factory = defaultAdapterFactories[agentId];
  if (!factory) {
    throw new Error(`Unknown agent: ${agentId}. Known agents: ${Object.keys(defaultAdapterFactories).join(', ')}`);
  }
  return factory(config);
}

/**
 * Long-Term Pattern Agent - Google Cloud Edge Function Entry Point
 *
 * Classification: MEMORY ANALYSIS
 * Decision Type: long_term_pattern_analysis
 *
 * This agent deploys as a Google Cloud Edge Function and analyzes
 * historical memory to identify recurring patterns, trends, and behaviors.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This agent does NOT modify runtime execution
 * - This agent does NOT enforce policies
 * - This agent does NOT orchestrate workflows
 * - This agent does NOT trigger remediation
 * - This agent does NOT emit alerts
 * - This agent does NOT connect directly to SQL
 *
 * The agent reads from ruvector-service and writes exactly ONE DecisionEvent
 * per invocation containing the analysis results.
 */

import { LongTermPatternAgent, createAgent } from './agent.js';
import { RuVectorClient } from './ruvector-client.js';
import type { AgentResult } from './agent.js';
import type { AgentExecutionResult } from '../../../src/execution/agent-adapter.js';
import { AGENT_ID, AGENT_VERSION } from './types.js';

// Export types for consumers
export type {
  PatternAnalysisInput,
  PatternAnalysisOutput,
  DecisionEvent,
  AgentError,
  DetectedPattern,
  PatternOccurrence,
  TemporalDistribution,
  AnalysisStatistics,
  PatternType,
  TrendDirection,
  TimeRange,
  AnalysisScope,
  AnalysisOptions,
} from './types.js';

export {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
  ANALYSIS_CONSTRAINTS,
  PATTERN_STRATEGIES,
} from './types.js';

export { LongTermPatternAgent, createAgent } from './agent.js';
export type { AgentResult } from './agent.js';

/**
 * HTTP Request interface for Edge Function
 */
interface EdgeFunctionRequest {
  body?: unknown;
  method?: string;
  headers?: Record<string, string>;
  query?: Record<string, string>;
}

/**
 * HTTP Response interface for Edge Function
 */
interface EdgeFunctionResponse {
  statusCode: number;
  headers: Record<string, string>;
  body: string;
}

/**
 * Create standard response headers
 */
function createResponseHeaders(): Record<string, string> {
  return {
    'Content-Type': 'application/json',
    'X-Agent-Id': 'long-term-pattern-agent',
    'X-Agent-Version': '1.0.0',
    'X-Agent-Classification': 'MEMORY_ANALYSIS',
  };
}

/**
 * Create success response
 */
function createSuccessResponse(result: AgentResult & { success: true }): EdgeFunctionResponse {
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      success: true,
      output: result.output,
      decision_event: result.decisionEvent,
    }),
  };
}

/**
 * Create error response
 */
function createErrorResponse(
  statusCode: number,
  result: AgentResult & { success: false }
): EdgeFunctionResponse {
  return {
    statusCode,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      success: false,
      error: result.error,
    }),
  };
}

/**
 * Determine HTTP status code from error
 */
function getStatusCodeForError(errorCode: string): number {
  switch (errorCode) {
    case 'VALIDATION_ERROR':
      return 400;
    case 'RUVECTOR_CONNECTION_ERROR':
    case 'RUVECTOR_READ_ERROR':
    case 'RUVECTOR_WRITE_ERROR':
      return 502;
    case 'INSUFFICIENT_DATA':
      return 404;
    case 'ANALYSIS_TIMEOUT':
      return 504;
    case 'RATE_LIMIT_EXCEEDED':
      return 429;
    case 'INTERNAL_ERROR':
    default:
      return 500;
  }
}

/**
 * Google Cloud Edge Function handler
 *
 * Entry point for the Long-Term Pattern Agent.
 * Handles HTTP requests and returns deterministic, machine-readable output.
 */
export async function handler(request: EdgeFunctionRequest): Promise<EdgeFunctionResponse> {
  // Validate request method
  if (request.method !== 'POST') {
    return {
      statusCode: 405,
      headers: createResponseHeaders(),
      body: JSON.stringify({
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: 'Method not allowed. Use POST.',
          execution_ref: crypto.randomUUID(),
          timestamp: new Date().toISOString(),
        },
      }),
    };
  }

  // Validate request body
  if (!request.body) {
    return {
      statusCode: 400,
      headers: createResponseHeaders(),
      body: JSON.stringify({
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: 'Request body is required',
          execution_ref: crypto.randomUUID(),
          timestamp: new Date().toISOString(),
        },
      }),
    };
  }

  // Execute agent
  const agent = createAgent();
  const result = await agent.execute(request.body);

  if (result.success) {
    return createSuccessResponse(result);
  }

  return createErrorResponse(getStatusCodeForError(result.error.error_code), result);
}

/**
 * Health check endpoint
 */
export async function healthHandler(): Promise<EdgeFunctionResponse> {
  // Check ruvector-service connectivity
  const ruvectorClient = new RuVectorClient();
  const ruvectorHealthy = await ruvectorClient.healthCheck();

  const status = ruvectorHealthy ? 'healthy' : 'degraded';
  const statusCode = ruvectorHealthy ? 200 : 503;

  return {
    statusCode,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      status,
      agent_id: 'long-term-pattern-agent',
      version: '1.0.0',
      classification: 'MEMORY_ANALYSIS',
      decision_type: 'long_term_pattern_analysis',
      dependencies: {
        ruvector_service: ruvectorHealthy ? 'connected' : 'unavailable',
      },
      timestamp: new Date().toISOString(),
    }),
  };
}

/**
 * FEU entry point: Execute this agent as part of a Foundational Execution Unit.
 *
 * Returns an AgentExecutionResult that the orchestrator wraps into an AgentSpan.
 */
export async function executeAsUnit(input: unknown): Promise<AgentExecutionResult> {
  const agent = createAgent();
  const result = await agent.execute(input);

  if (result.success) {
    return {
      success: true,
      output: result.output,
      decisionEvent: result.decisionEvent,
    };
  }

  return {
    success: false,
    error: {
      error_code: result.error.error_code,
      message: result.error.message,
      details: result.error.details,
    },
  };
}

/**
 * FEU metadata for adapter registration.
 */
export const FEU_METADATA = {
  agent_id: AGENT_ID,
  agent_version: AGENT_VERSION,
} as const;

/**
 * CLI-invokable endpoint for inspect operation
 *
 * Retrieves a specific DecisionEvent by execution_ref.
 */
export async function inspectHandler(executionRef: string): Promise<EdgeFunctionResponse> {
  const ruvectorClient = new RuVectorClient();
  const event = await ruvectorClient.retrieveDecisionEvent(executionRef);

  if (!event) {
    return {
      statusCode: 404,
      headers: createResponseHeaders(),
      body: JSON.stringify({
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: `DecisionEvent not found for execution_ref: ${executionRef}`,
          execution_ref: executionRef,
          timestamp: new Date().toISOString(),
        },
      }),
    };
  }

  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      success: true,
      operation: 'inspect',
      execution_ref: executionRef,
      decision_event: event,
    }),
  };
}

/**
 * CLI-invokable endpoint for retrieve operation
 *
 * Retrieves DecisionEvents matching the specified criteria.
 */
export async function retrieveHandler(query: {
  from_timestamp?: string;
  to_timestamp?: string;
  limit?: number;
}): Promise<EdgeFunctionResponse> {
  const ruvectorClient = new RuVectorClient();
  const result = await ruvectorClient.queryDecisionEvents({
    agent_id: 'long-term-pattern-agent',
    decision_type: 'long_term_pattern_analysis',
    from_timestamp: query.from_timestamp,
    to_timestamp: query.to_timestamp,
    limit: query.limit ?? 100,
  });

  if (!result.success) {
    return {
      statusCode: 502,
      headers: createResponseHeaders(),
      body: JSON.stringify({
        success: false,
        error: {
          error_code: 'RUVECTOR_READ_ERROR',
          message: result.error ?? 'Failed to retrieve DecisionEvents',
          execution_ref: crypto.randomUUID(),
          timestamp: new Date().toISOString(),
        },
      }),
    };
  }

  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      success: true,
      operation: 'retrieve',
      query,
      count: result.data?.length ?? 0,
      decision_events: result.data ?? [],
    }),
  };
}

/**
 * CLI-invokable endpoint for replay operation
 *
 * Re-executes the agent with the input from a previous DecisionEvent.
 */
export async function replayHandler(executionRef: string): Promise<EdgeFunctionResponse> {
  const ruvectorClient = new RuVectorClient();
  const originalEvent = await ruvectorClient.retrieveDecisionEvent(executionRef);

  if (!originalEvent) {
    return {
      statusCode: 404,
      headers: createResponseHeaders(),
      body: JSON.stringify({
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: `Original DecisionEvent not found for replay: ${executionRef}`,
          execution_ref: executionRef,
          timestamp: new Date().toISOString(),
        },
      }),
    };
  }

  // Note: For a full replay, we would need to store the original input
  // This is a placeholder showing the intended behavior
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      success: true,
      operation: 'replay',
      original_execution_ref: executionRef,
      message: 'Replay requires original input stored in DecisionEvent metadata',
      original_outputs: originalEvent.outputs,
    }),
  };
}

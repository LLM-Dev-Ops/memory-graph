/**
 * Conversation Memory Agent - Google Cloud Edge Function Entry Point
 *
 * Classification: MEMORY WRITE
 * Decision Type: conversation_capture
 *
 * This agent deploys as a Google Cloud Edge Function and captures
 * structured representations of multi-turn conversations.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This agent does NOT modify runtime execution
 * - This agent does NOT enforce policies
 * - This agent does NOT orchestrate workflows
 * - This agent does NOT trigger remediation
 * - This agent does NOT emit alerts
 * - This agent does NOT connect directly to SQL
 */

import { ConversationMemoryAgent, createAgent } from './agent.js';
import type { AgentResult } from './agent.js';
import type { AgentExecutionResult } from '../../../src/execution/agent-adapter.js';
import { AGENT_ID, AGENT_VERSION } from './types.js';

// Export types for consumers
export type {
  ConversationCaptureInput,
  ConversationCaptureOutput,
  DecisionEvent,
  AgentError,
  ConversationTurn,
  NodeReference,
  EdgeReference,
} from './types.js';

export {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
} from './types.js';

export { ConversationMemoryAgent, createAgent } from './agent.js';
export type { AgentResult } from './agent.js';

/**
 * HTTP Request interface for Edge Function
 */
interface EdgeFunctionRequest {
  body?: unknown;
  method?: string;
  headers?: Record<string, string>;
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
    'X-Agent-Id': 'conversation-memory-agent',
    'X-Agent-Version': '1.0.0',
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
    case 'RUVECTOR_WRITE_ERROR':
      return 502;
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
 * Entry point for the Conversation Memory Agent.
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
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      status: 'healthy',
      agent_id: 'conversation-memory-agent',
      version: '1.0.0',
      classification: 'MEMORY_WRITE',
      timestamp: new Date().toISOString(),
    }),
  };
}

/**
 * FEU entry point: Execute this agent as part of a Foundational Execution Unit.
 *
 * Returns an AgentExecutionResult that the orchestrator wraps into an AgentSpan.
 * This function does NOT create spans itself -- span creation is handled by
 * the adapter layer in src/execution/agent-adapter.ts.
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
 */
export async function inspectHandler(executionRef: string): Promise<EdgeFunctionResponse> {
  // Note: This would query ruvector-service for the DecisionEvent
  // For now, return a placeholder that indicates the expected behavior
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      operation: 'inspect',
      execution_ref: executionRef,
      message: 'Query ruvector-service for DecisionEvent with this execution_ref',
    }),
  };
}

/**
 * CLI-invokable endpoint for retrieve operation
 */
export async function retrieveHandler(query: {
  session_id?: string;
  conversation_id?: string;
  from_timestamp?: string;
  to_timestamp?: string;
  limit?: number;
}): Promise<EdgeFunctionResponse> {
  // Note: This would query ruvector-service for DecisionEvents
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      operation: 'retrieve',
      query,
      message: 'Query ruvector-service for DecisionEvents matching criteria',
    }),
  };
}

/**
 * CLI-invokable endpoint for replay operation
 */
export async function replayHandler(executionRef: string): Promise<EdgeFunctionResponse> {
  // Note: This would fetch the original input from ruvector-service and re-execute
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      operation: 'replay',
      execution_ref: executionRef,
      message: 'Fetch original DecisionEvent input from ruvector-service and re-execute',
    }),
  };
}

/**
 * Knowledge Graph Builder Agent - Google Cloud Edge Function Entry Point
 *
 * Classification: MEMORY_ANALYSIS
 * Decision Type: knowledge_graph_construction
 *
 * This agent deploys as a Google Cloud Edge Function and builds
 * knowledge graphs from text content, extracting concepts, entities,
 * relationships, and patterns.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This agent does NOT modify runtime execution
 * - This agent does NOT enforce policies
 * - This agent does NOT orchestrate workflows
 * - This agent does NOT trigger remediation
 * - This agent does NOT emit alerts
 * - This agent does NOT connect directly to SQL
 */

import { KnowledgeGraphBuilderAgent, createAgent } from './handler.js';
import type { AgentResult } from './handler.js';
import type { AgentExecutionResult } from '../../../src/execution/agent-adapter.js';
import { AGENT_ID, AGENT_VERSION } from './types.js';

// Export types for consumers
export type {
  KnowledgeGraphBuilderInput,
  KnowledgeGraphBuilderOutput,
  DecisionEvent,
  AgentError,
  ExtractedConcept,
  ExtractedRelationship,
  DetectedPattern,
  GraphStatistics,
  TextContent,
  ExtractionOptions,
  GraphOptions,
  ConceptType,
  EntityType,
  RelationshipType,
} from './types.js';

export {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
} from './types.js';

export { KnowledgeGraphBuilderAgent, createAgent } from './handler.js';
export type { AgentResult } from './handler.js';

// Export utilities
export { createExtractor } from './extractor.js';
export { createGraphBuilder } from './graph-builder.js';
export { createConfidenceCalculator, type ConfidenceFactors } from './confidence.js';
export { createDecisionEventEmitter, validateDecisionEvent } from './decision-event.js';
export { RuVectorClient, type RuVectorConfig, type RuVectorResponse } from './ruvector-client.js';
export { TelemetryEmitter, createTelemetryEmitter } from './telemetry.js';

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
    'X-Agent-Id': 'knowledge-graph-builder-agent',
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
    case 'RUVECTOR_WRITE_ERROR':
      return 502;
    case 'RATE_LIMIT_EXCEEDED':
      return 429;
    case 'EXTRACTION_ERROR':
    case 'GRAPH_BUILD_ERROR':
    case 'INTERNAL_ERROR':
    default:
      return 500;
  }
}

/**
 * Google Cloud Edge Function handler
 *
 * Entry point for the Knowledge Graph Builder Agent.
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
      agent_id: 'knowledge-graph-builder-agent',
      version: '1.0.0',
      classification: 'MEMORY_ANALYSIS',
      decision_type: 'knowledge_graph_construction',
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
 */
export async function inspectHandler(executionRef: string): Promise<EdgeFunctionResponse> {
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
 * CLI-invokable endpoint for query graph operation
 */
export async function queryGraphHandler(query: {
  session_id?: string;
  conversation_id?: string;
  concept_name?: string;
  concept_type?: string;
  min_confidence?: number;
  limit?: number;
}): Promise<EdgeFunctionResponse> {
  return {
    statusCode: 200,
    headers: createResponseHeaders(),
    body: JSON.stringify({
      operation: 'query_graph',
      query,
      message: 'Query ruvector-service for knowledge graph data matching criteria',
    }),
  };
}

/**
 * CLI-invokable endpoint for replay operation
 */
export async function replayHandler(executionRef: string): Promise<EdgeFunctionResponse> {
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

/**
 * Capability description for discovery
 */
export function getCapabilities(): Record<string, unknown> {
  return {
    agent_id: 'knowledge-graph-builder-agent',
    version: '1.0.0',
    classification: 'MEMORY_ANALYSIS',
    decision_type: 'knowledge_graph_construction',
    capabilities: {
      extraction: {
        entities: true,
        concepts: true,
        relationships: true,
        patterns: true,
      },
      entity_types: [
        'person',
        'organization',
        'location',
        'datetime',
        'technology',
        'concept',
        'file_path',
        'url',
        'code_element',
        'other',
      ],
      relationship_types: [
        'is_a',
        'has_attribute',
        'related_to',
        'part_of',
        'causes',
        'precedes',
        'co_occurs',
        'depends_on',
        'references',
      ],
      pattern_types: [
        'recurring_theme',
        'temporal_sequence',
        'co_occurrence',
        'structural',
      ],
      options: {
        merge_similar_concepts: true,
        temporal_edges: true,
        confidence_filtering: true,
        centrality_computation: true,
      },
    },
    endpoints: {
      process: 'POST /',
      health: 'GET /health',
      inspect: 'GET /inspect/:execution_ref',
      query: 'GET /query',
      replay: 'POST /replay/:execution_ref',
    },
    constraints: [
      'Stateless execution - no local persistence',
      'All persistence via ruvector-service client calls only',
      'Deterministic, machine-readable output',
      'Emits exactly ONE DecisionEvent per invocation',
      'Async, non-blocking operations',
      'No orchestration logic',
      'No enforcement logic',
      'No direct SQL access',
    ],
  };
}

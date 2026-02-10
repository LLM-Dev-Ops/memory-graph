/**
 * Memory Retrieval Agent - Entry Point
 *
 * Classification: MEMORY READ
 * Decision Type: memory_retrieval
 *
 * This module exports the agent and provides the HTTP handler
 * for Google Cloud Functions deployment.
 *
 * CLI Invocation Shapes:
 * - inspect: Retrieve and display memory subgraph
 * - retrieve: Execute query and return results
 * - replay: Re-execute a previous query by execution_ref
 */

export {
  MemoryRetrievalAgent,
  createAgent,
  type AgentResult,
} from './agent.js';

export {
  AGENT_ID,
  AGENT_VERSION,
  AGENT_CLASSIFICATION,
  DECISION_TYPE,
  type MemoryRetrievalInput,
  type MemoryRetrievalOutput,
  type DecisionEvent,
  type AgentError,
  type RetrievedNode,
  type RetrievedEdge,
  type RetrievedSubgraph,
  type QueryConstraint,
  type TraversalOptions,
  MemoryRetrievalInputSchema,
  DecisionEventSchema,
} from './types.js';

export {
  RuVectorClient,
  type RuVectorConfig,
  type RuVectorQueryResponse,
  type RuVectorPersistResponse,
} from './ruvector-client.js';

export {
  TelemetryEmitter,
  createTelemetryEmitter,
  type TelemetryEvent,
  type TelemetrySpan,
} from './telemetry.js';

import { createAgent } from './agent.js';
import type { MemoryRetrievalInput } from './types.js';
import type { AgentExecutionResult } from '../../../src/execution/agent-adapter.js';
import { AGENT_ID, AGENT_VERSION } from './types.js';

/**
 * HTTP handler for Google Cloud Functions
 *
 * Accepts POST requests with MemoryRetrievalInput body.
 * Returns MemoryRetrievalOutput or AgentError.
 */
export async function handler(
  req: { method: string; body: unknown },
  res: { status: (code: number) => { json: (data: unknown) => void } }
): Promise<void> {
  // Only accept POST requests
  if (req.method !== 'POST') {
    res.status(405).json({
      error_code: 'VALIDATION_ERROR',
      message: 'Method not allowed. Use POST.',
      execution_ref: crypto.randomUUID(),
      timestamp: new Date().toISOString(),
    });
    return;
  }

  const agent = createAgent();
  const result = await agent.execute(req.body);

  if (result.success) {
    res.status(200).json({
      success: true,
      output: result.output,
      decision_event: result.decisionEvent,
    });
  } else {
    const statusCode = getStatusCode(result.error.error_code);
    res.status(statusCode).json({
      success: false,
      error: result.error,
    });
  }
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
 * CLI handler for inspect command
 *
 * Usage: memory-retrieval inspect --query-id <id> [--format json|table]
 */
export async function inspectHandler(args: {
  query_id: string;
  anchor_nodes?: string[];
  anchor_sessions?: string[];
  format?: 'json' | 'table';
}): Promise<void> {
  const agent = createAgent();

  const input: MemoryRetrievalInput = {
    query_id: args.query_id,
    query_type: 'subgraph',
    anchor_nodes: args.anchor_nodes,
    anchor_sessions: args.anchor_sessions,
    limit: 100,
    offset: 0,
    include_metadata: true,
  };

  const result = await agent.execute(input);

  if (args.format === 'table') {
    if (result.success) {
      console.log('=== Memory Subgraph ===');
      console.log(`Query ID: ${result.output.query_id}`);
      console.log(`Nodes Retrieved: ${result.output.total_nodes_retrieved}`);
      console.log(`Edges Retrieved: ${result.output.total_edges_retrieved}`);
      console.log('\n--- Nodes ---');
      for (const node of result.output.subgraph.nodes) {
        console.log(`  ${node.node_id} [${node.node_type}]`);
      }
      console.log('\n--- Edges ---');
      for (const edge of result.output.subgraph.edges) {
        console.log(`  ${edge.from_node_id} --[${edge.edge_type}]--> ${edge.to_node_id}`);
      }
    } else {
      console.error(`Error: ${result.error.error_code} - ${result.error.message}`);
    }
  } else {
    console.log(JSON.stringify(result, null, 2));
  }
}

/**
 * CLI handler for retrieve command
 *
 * Usage: memory-retrieval retrieve --input <json-file>
 */
export async function retrieveHandler(input: unknown): Promise<void> {
  const agent = createAgent();
  const result = await agent.execute(input);
  console.log(JSON.stringify(result, null, 2));
}

/**
 * CLI handler for replay command
 *
 * Usage: memory-retrieval replay --execution-ref <uuid>
 */
export async function replayHandler(args: { execution_ref: string }): Promise<void> {
  const { RuVectorClient } = await import('./ruvector-client.js');
  const client = new RuVectorClient();

  const previousEvent = await client.retrieveDecisionEvent(args.execution_ref);

  if (!previousEvent) {
    console.error(`Error: DecisionEvent not found for execution_ref: ${args.execution_ref}`);
    process.exit(1);
  }

  // Extract the original input from the DecisionEvent
  // Note: We can't fully reconstruct input from outputs, but we can show the results
  console.log('=== Previous Execution ===');
  console.log(`Execution Ref: ${previousEvent.execution_ref}`);
  console.log(`Timestamp: ${previousEvent.timestamp}`);
  console.log(`Confidence: ${previousEvent.confidence}`);
  console.log(`Constraints: ${previousEvent.constraints_applied?.join(', ') ?? 'none'}`);
  console.log('\n--- Output ---');
  console.log(JSON.stringify(previousEvent.outputs, null, 2));
}

/**
 * Map error codes to HTTP status codes
 */
function getStatusCode(errorCode: string): number {
  switch (errorCode) {
    case 'VALIDATION_ERROR':
    case 'INVALID_ANCHOR_NODE':
      return 400;
    case 'RATE_LIMIT_EXCEEDED':
      return 429;
    case 'RUVECTOR_CONNECTION_ERROR':
    case 'RUVECTOR_READ_ERROR':
      return 502;
    case 'QUERY_TIMEOUT':
      return 504;
    case 'INTERNAL_ERROR':
    default:
      return 500;
  }
}

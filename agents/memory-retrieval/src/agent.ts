/**
 * Memory Retrieval Agent - Core Implementation
 *
 * Classification: MEMORY READ
 * Decision Type: memory_retrieval
 *
 * This agent retrieves relevant memory subgraphs based on query constraints
 * and context. It operates strictly on memory data and NEVER:
 * - Modifies system behavior
 * - Triggers remediation or retries
 * - Emits alerts
 * - Enforces policies
 * - Performs orchestration
 */

import {
  type MemoryRetrievalInput,
  type MemoryRetrievalOutput,
  type DecisionEvent,
  type AgentError,
  type RetrievedNode,
  type RetrievedEdge,
  type RetrievedSubgraph,
  type TraversalStats,
  type QueryConstraint,
  MemoryRetrievalInputSchema,
  AGENT_ID,
  AGENT_VERSION,
  DECISION_TYPE,
  QUERY_TYPE_DEFAULTS,
} from './types.js';
import {
  RuVectorClient,
  createRuVectorReadError,
  createInvalidAnchorNodeError,
} from './ruvector-client.js';
import { TelemetryEmitter, createTelemetryEmitter } from './telemetry.js';

/**
 * Agent execution context
 */
interface ExecutionContext {
  executionRef: string;
  startTime: number;
  telemetry: TelemetryEmitter;
}

/**
 * Agent execution result
 */
export type AgentResult =
  | { success: true; output: MemoryRetrievalOutput; decisionEvent: DecisionEvent }
  | { success: false; error: AgentError };

/**
 * Memory Retrieval Agent
 *
 * Stateless agent that retrieves memory subgraphs based on query constraints.
 * Emits exactly ONE DecisionEvent per invocation to ruvector-service.
 */
export class MemoryRetrievalAgent {
  private readonly ruvectorClient: RuVectorClient;

  constructor(ruvectorClient?: RuVectorClient) {
    this.ruvectorClient = ruvectorClient ?? new RuVectorClient();
  }

  /**
   * Execute the agent to retrieve memory subgraph
   *
   * This is the main entry point for the agent.
   * Emits exactly ONE DecisionEvent to ruvector-service.
   */
  async execute(input: unknown): Promise<AgentResult> {
    const context = this.createExecutionContext();

    try {
      // Validate input against schema
      const validationResult = this.validateInput(input, context);
      if (!validationResult.success) {
        return { success: false, error: validationResult.error };
      }

      const validInput = validationResult.data;

      // Execute the memory retrieval
      const retrievalSpan = context.telemetry.startSpan('memory_retrieval');
      const retrievalResult = await this.executeRetrieval(validInput, context);

      if (!retrievalResult.success) {
        context.telemetry.endSpan(retrievalSpan, {
          query_type: validInput.query_type,
          nodes_retrieved: 0,
          edges_retrieved: 0,
        });
        context.telemetry.recordError(
          retrievalResult.error.error_code,
          retrievalResult.error.message
        );
        context.telemetry.endExecution(false, { error: retrievalResult.error.error_code });
        return { success: false, error: retrievalResult.error };
      }

      const output = retrievalResult.output;

      context.telemetry.endSpan(retrievalSpan, {
        query_type: validInput.query_type,
        nodes_retrieved: output.total_nodes_retrieved,
        edges_retrieved: output.total_edges_retrieved,
      });

      // Calculate confidence score
      const confidence = this.calculateConfidence(output);

      // Create DecisionEvent
      const decisionEvent = await this.createDecisionEvent(validInput, output, confidence, context);

      // Persist DecisionEvent to ruvector-service
      const persistSpan = context.telemetry.startSpan('ruvector_persist');
      const persistResult = await this.ruvectorClient.persistDecisionEvent(decisionEvent);
      context.telemetry.endSpan(persistSpan);

      if (!persistResult.success) {
        context.telemetry.recordError(
          'RUVECTOR_WRITE_ERROR',
          persistResult.error ?? 'Unknown persistence error'
        );
        // Note: For MEMORY_READ agents, we return success even if persist fails
        // The retrieval was successful, only the audit trail failed
        console.error('[AUDIT] Failed to persist DecisionEvent:', persistResult.error);
      } else {
        context.telemetry.recordRuVectorPersist(
          true,
          persistResult.latency_ms ?? 0,
          persistResult.event_id
        );
      }

      // Update telemetry in DecisionEvent
      decisionEvent.telemetry = {
        duration_ms: Date.now() - context.startTime,
        ruvector_latency_ms: persistResult.latency_ms,
        nodes_scanned: output.traversal_stats?.nodes_visited,
        cache_hit: false, // TODO: Implement caching
      };

      context.telemetry.endExecution(true, {
        nodes_retrieved: output.total_nodes_retrieved,
        edges_retrieved: output.total_edges_retrieved,
      });

      return {
        success: true,
        output,
        decisionEvent,
      };
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      context.telemetry.recordError('INTERNAL_ERROR', errorMessage);
      context.telemetry.endExecution(false, { error: errorMessage });

      return {
        success: false,
        error: {
          error_code: 'INTERNAL_ERROR',
          message: errorMessage,
          execution_ref: context.executionRef,
          timestamp: new Date().toISOString(),
        },
      };
    }
  }

  /**
   * Create execution context for this invocation
   */
  private createExecutionContext(): ExecutionContext {
    const executionRef = crypto.randomUUID();
    const telemetry = createTelemetryEmitter();
    telemetry.startExecution(executionRef);

    return {
      executionRef,
      startTime: Date.now(),
      telemetry,
    };
  }

  /**
   * Validate input against schema
   */
  private validateInput(
    input: unknown,
    context: ExecutionContext
  ):
    | { success: true; data: MemoryRetrievalInput }
    | { success: false; error: AgentError } {
    const validationSpan = context.telemetry.startSpan('validate_input');

    try {
      const result = MemoryRetrievalInputSchema.safeParse(input);

      if (!result.success) {
        context.telemetry.recordValidation(
          false,
          result.error.errors.map((e) => e.message)
        );
        context.telemetry.endSpan(validationSpan, { valid: false });

        return {
          success: false,
          error: {
            error_code: 'VALIDATION_ERROR',
            message: 'Input validation failed',
            details: { errors: result.error.errors },
            execution_ref: context.executionRef,
            timestamp: new Date().toISOString(),
          },
        };
      }

      context.telemetry.recordValidation(true);
      context.telemetry.endSpan(validationSpan, { valid: true });

      return { success: true, data: result.data };
    } catch (error) {
      context.telemetry.endSpan(validationSpan, { valid: false, error: String(error) });
      return {
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: error instanceof Error ? error.message : 'Validation error',
          execution_ref: context.executionRef,
          timestamp: new Date().toISOString(),
        },
      };
    }
  }

  /**
   * Execute the memory retrieval based on query type
   */
  private async executeRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    switch (input.query_type) {
      case 'subgraph':
        return this.executeSubgraphRetrieval(input, context);
      case 'nodes':
        return this.executeNodesRetrieval(input, context);
      case 'edges':
        return this.executeEdgesRetrieval(input, context);
      case 'lineage':
        return this.executeLineageRetrieval(input, context);
      case 'context':
        return this.executeContextRetrieval(input, context);
      case 'similarity':
        return this.executeSimilarityRetrieval(input, context);
      default:
        return {
          success: false,
          error: {
            error_code: 'VALIDATION_ERROR',
            message: `Unknown query type: ${input.query_type}`,
            execution_ref: context.executionRef,
            timestamp: new Date().toISOString(),
          },
        };
    }
  }

  /**
   * Execute subgraph retrieval
   */
  private async executeSubgraphRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    const queryResult = await this.ruvectorClient.queryMemoryGraph({
      anchor_node_ids: input.anchor_nodes,
      anchor_session_ids: input.anchor_sessions,
      constraints: input.constraints,
      traversal_options: input.traversal_options ?? QUERY_TYPE_DEFAULTS.subgraph,
      limit: input.limit,
      offset: input.offset,
    });

    if (!queryResult.success) {
      return {
        success: false,
        error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
      };
    }

    context.telemetry.recordMemoryQuery(
      'subgraph',
      queryResult.nodes.length,
      queryResult.edges.length,
      queryResult.latency_ms ?? 0,
      queryResult.cache_hit
    );

    return {
      success: true,
      output: this.buildOutput(input, queryResult.nodes, queryResult.edges, {
        nodes_visited: queryResult.nodes.length,
        edges_traversed: queryResult.edges.length,
      }),
    };
  }

  /**
   * Execute nodes-only retrieval
   */
  private async executeNodesRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    if (!input.anchor_nodes || input.anchor_nodes.length === 0) {
      // Query nodes by constraints
      const queryResult = await this.ruvectorClient.queryMemoryGraph({
        constraints: input.constraints,
        limit: input.limit,
        offset: input.offset,
      });

      if (!queryResult.success) {
        return {
          success: false,
          error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
        };
      }

      context.telemetry.recordMemoryQuery(
        'nodes',
        queryResult.nodes.length,
        0,
        queryResult.latency_ms ?? 0
      );

      return {
        success: true,
        output: this.buildOutput(input, queryResult.nodes, []),
      };
    }

    // Retrieve specific nodes by IDs
    const queryResult = await this.ruvectorClient.getNodesByIds(input.anchor_nodes);

    if (!queryResult.success) {
      return {
        success: false,
        error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
      };
    }

    // Check if all anchor nodes were found
    const foundIds = new Set(queryResult.nodes.map((n) => n.node_id));
    const missingIds = input.anchor_nodes.filter((id) => !foundIds.has(id));

    if (missingIds.length > 0) {
      return {
        success: false,
        error: createInvalidAnchorNodeError(context.executionRef, missingIds),
      };
    }

    context.telemetry.recordMemoryQuery(
      'nodes',
      queryResult.nodes.length,
      0,
      queryResult.latency_ms ?? 0
    );

    return {
      success: true,
      output: this.buildOutput(input, queryResult.nodes, []),
    };
  }

  /**
   * Execute edges-only retrieval
   */
  private async executeEdgesRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    if (!input.anchor_nodes || input.anchor_nodes.length === 0) {
      return {
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: 'anchor_nodes required for edges query',
          execution_ref: context.executionRef,
          timestamp: new Date().toISOString(),
        },
      };
    }

    const direction = input.traversal_options?.direction ?? 'both';
    const queryResult = await this.ruvectorClient.getEdgesForNodes(input.anchor_nodes, direction);

    if (!queryResult.success) {
      return {
        success: false,
        error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
      };
    }

    context.telemetry.recordMemoryQuery(
      'edges',
      0,
      queryResult.edges.length,
      queryResult.latency_ms ?? 0
    );

    return {
      success: true,
      output: this.buildOutput(input, [], queryResult.edges),
    };
  }

  /**
   * Execute lineage retrieval (tracing back through causal chain)
   */
  private async executeLineageRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    const queryResult = await this.ruvectorClient.queryMemoryGraph({
      anchor_node_ids: input.anchor_nodes,
      traversal_options: {
        ...QUERY_TYPE_DEFAULTS.lineage,
        ...input.traversal_options,
        follow_edge_types: ['follows', 'responds_to', 'invokes'],
      },
      limit: input.limit,
      offset: input.offset,
    });

    if (!queryResult.success) {
      return {
        success: false,
        error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
      };
    }

    context.telemetry.recordMemoryQuery(
      'lineage',
      queryResult.nodes.length,
      queryResult.edges.length,
      queryResult.latency_ms ?? 0
    );

    return {
      success: true,
      output: this.buildOutput(input, queryResult.nodes, queryResult.edges, {
        nodes_visited: queryResult.nodes.length,
        edges_traversed: queryResult.edges.length,
      }),
    };
  }

  /**
   * Execute context retrieval (surrounding context of anchor nodes)
   */
  private async executeContextRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    const queryResult = await this.ruvectorClient.queryMemoryGraph({
      anchor_node_ids: input.anchor_nodes,
      anchor_session_ids: input.anchor_sessions,
      traversal_options: {
        ...QUERY_TYPE_DEFAULTS.context,
        ...input.traversal_options,
      },
      limit: input.limit,
      offset: input.offset,
    });

    if (!queryResult.success) {
      return {
        success: false,
        error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
      };
    }

    context.telemetry.recordMemoryQuery(
      'context',
      queryResult.nodes.length,
      queryResult.edges.length,
      queryResult.latency_ms ?? 0
    );

    return {
      success: true,
      output: this.buildOutput(input, queryResult.nodes, queryResult.edges, {
        nodes_visited: queryResult.nodes.length,
        edges_traversed: queryResult.edges.length,
      }),
    };
  }

  /**
   * Execute similarity-based retrieval
   */
  private async executeSimilarityRetrieval(
    input: MemoryRetrievalInput,
    context: ExecutionContext
  ): Promise<{ success: true; output: MemoryRetrievalOutput } | { success: false; error: AgentError }> {
    if (!input.semantic_query) {
      return {
        success: false,
        error: {
          error_code: 'VALIDATION_ERROR',
          message: 'semantic_query required for similarity query',
          execution_ref: context.executionRef,
          timestamp: new Date().toISOString(),
        },
      };
    }

    const queryResult = await this.ruvectorClient.similaritySearch(
      input.semantic_query,
      input.limit,
      {
        session_ids: input.anchor_sessions,
        node_types: input.traversal_options?.include_node_types,
      }
    );

    if (!queryResult.success) {
      return {
        success: false,
        error: createRuVectorReadError(context.executionRef, { error: queryResult.error }),
      };
    }

    // Calculate average relevance score for telemetry
    const avgRelevance =
      queryResult.nodes.length > 0
        ? queryResult.nodes.reduce((sum, n) => sum + (n.relevance_score ?? 0), 0) /
          queryResult.nodes.length
        : 0;

    context.telemetry.recordSimilaritySearch(
      input.semantic_query,
      queryResult.nodes.length,
      avgRelevance,
      queryResult.latency_ms ?? 0
    );

    return {
      success: true,
      output: this.buildOutput(input, queryResult.nodes, []),
    };
  }

  /**
   * Build the output structure
   */
  private buildOutput(
    input: MemoryRetrievalInput,
    nodes: RetrievedNode[],
    edges: RetrievedEdge[],
    traversalStats?: Partial<TraversalStats>
  ): MemoryRetrievalOutput {
    const subgraph: RetrievedSubgraph = {
      nodes,
      edges,
      anchor_node_ids: input.anchor_nodes,
      truncated: nodes.length >= input.limit || edges.length >= input.limit,
    };

    return {
      query_id: input.query_id,
      query_type: input.query_type,
      subgraph,
      total_nodes_retrieved: nodes.length,
      total_edges_retrieved: edges.length,
      retrieval_timestamp: new Date().toISOString(),
      constraints_applied: this.getConstraintsApplied(input),
      traversal_stats: traversalStats
        ? {
            max_depth_reached: traversalStats.max_depth_reached,
            nodes_visited: traversalStats.nodes_visited,
            edges_traversed: traversalStats.edges_traversed,
          }
        : undefined,
      pagination: {
        offset: input.offset,
        limit: input.limit,
        has_more: subgraph.truncated ?? false,
      },
    };
  }

  /**
   * Calculate confidence score based on retrieval results
   */
  private calculateConfidence(output: MemoryRetrievalOutput): number {
    // For similarity queries, use average relevance score
    if (output.query_type === 'similarity' && output.subgraph.nodes.length > 0) {
      const avgRelevance =
        output.subgraph.nodes.reduce((sum, n) => sum + (n.relevance_score ?? 0), 0) /
        output.subgraph.nodes.length;
      return Math.round(avgRelevance * 100) / 100;
    }

    // For direct queries, confidence is based on completeness
    if (output.total_nodes_retrieved === 0 && output.total_edges_retrieved === 0) {
      return 0;
    }

    // High confidence for non-truncated results
    if (!output.subgraph.truncated) {
      return 1.0;
    }

    // Slightly lower confidence for truncated results
    return 0.9;
  }

  /**
   * Get list of constraints that were applied
   */
  private getConstraintsApplied(input: MemoryRetrievalInput): string[] {
    const constraints: string[] = [];

    if (input.anchor_nodes && input.anchor_nodes.length > 0) {
      constraints.push(`anchor_nodes:${input.anchor_nodes.length}`);
    }

    if (input.anchor_sessions && input.anchor_sessions.length > 0) {
      constraints.push(`anchor_sessions:${input.anchor_sessions.length}`);
    }

    if (input.constraints) {
      for (const c of input.constraints) {
        constraints.push(`${c.constraint_type}:${c.operator}`);
      }
    }

    if (input.traversal_options) {
      if (input.traversal_options.max_depth) {
        constraints.push(`max_depth:${input.traversal_options.max_depth}`);
      }
      if (input.traversal_options.direction) {
        constraints.push(`direction:${input.traversal_options.direction}`);
      }
      if (input.traversal_options.follow_edge_types) {
        constraints.push(`edge_types:${input.traversal_options.follow_edge_types.join(',')}`);
      }
      if (input.traversal_options.include_node_types) {
        constraints.push(`node_types:${input.traversal_options.include_node_types.join(',')}`);
      }
    }

    if (input.semantic_query) {
      constraints.push('semantic_search');
    }

    constraints.push(`limit:${input.limit}`);
    constraints.push(`offset:${input.offset}`);

    return constraints;
  }

  /**
   * Create DecisionEvent for this execution
   */
  private async createDecisionEvent(
    input: MemoryRetrievalInput,
    output: MemoryRetrievalOutput,
    confidence: number,
    context: ExecutionContext
  ): Promise<DecisionEvent> {
    const inputsHash = await this.hashInput(input);

    return {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      inputs_hash: inputsHash,
      outputs: output,
      confidence,
      constraints_applied: output.constraints_applied,
      execution_ref: context.executionRef,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Hash input for idempotency tracking
   */
  private async hashInput(input: MemoryRetrievalInput): Promise<string> {
    const data = JSON.stringify(input);
    const encoder = new TextEncoder();
    const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(data));
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  }
}

/**
 * Create a new agent instance
 */
export function createAgent(): MemoryRetrievalAgent {
  return new MemoryRetrievalAgent();
}

/**
 * Conversation Memory Agent - Core Implementation
 *
 * Classification: MEMORY WRITE
 * Decision Type: conversation_capture
 *
 * This agent captures and persists structured representations of multi-turn
 * conversations. It operates strictly on memory data and NEVER:
 * - Modifies system behavior
 * - Triggers remediation or retries
 * - Emits alerts
 * - Enforces policies
 * - Performs orchestration
 */

import {
  type ConversationCaptureInput,
  type ConversationCaptureOutput,
  type DecisionEvent,
  type AgentError,
  type NodeReference,
  type EdgeReference,
  type ConversationTurn,
  ConversationCaptureInputSchema,
  AGENT_ID,
  AGENT_VERSION,
  DECISION_TYPE,
  ROLE_TO_NODE_TYPE,
  LINEAGE_EDGE_TYPES,
} from './types.js';
import { RuVectorClient, createRuVectorWriteError } from './ruvector-client.js';
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
  | { success: true; output: ConversationCaptureOutput; decisionEvent: DecisionEvent }
  | { success: false; error: AgentError };

/**
 * Conversation Memory Agent
 *
 * Stateless agent that captures multi-turn conversation memory.
 * Emits exactly ONE DecisionEvent per invocation to ruvector-service.
 */
export class ConversationMemoryAgent {
  private readonly ruvectorClient: RuVectorClient;

  constructor(ruvectorClient?: RuVectorClient) {
    this.ruvectorClient = ruvectorClient ?? new RuVectorClient();
  }

  /**
   * Execute the agent to capture conversation memory
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

      // Process the conversation capture
      const processSpan = context.telemetry.startSpan('process_conversation');
      const output = await this.processConversation(validInput, context);
      context.telemetry.endSpan(processSpan, {
        turn_count: output.turn_count,
        nodes_created: output.nodes_created.length,
        edges_created: output.edges_created.length,
      });

      // Create DecisionEvent
      const decisionEvent = await this.createDecisionEvent(validInput, output, context);

      // Persist DecisionEvent to ruvector-service
      const persistSpan = context.telemetry.startSpan('ruvector_persist');
      const persistResult = await this.ruvectorClient.persistDecisionEvent(decisionEvent);
      context.telemetry.endSpan(persistSpan);

      if (!persistResult.success) {
        context.telemetry.recordError(
          'RUVECTOR_WRITE_ERROR',
          persistResult.error ?? 'Unknown persistence error'
        );
        return {
          success: false,
          error: createRuVectorWriteError(context.executionRef, {
            ruvector_error: persistResult.error,
          }),
        };
      }

      context.telemetry.recordRuVectorPersist(
        true,
        persistResult.latency_ms ?? 0,
        persistResult.event_id
      );

      // Update telemetry in DecisionEvent
      decisionEvent.telemetry = {
        duration_ms: Date.now() - context.startTime,
        ruvector_latency_ms: persistResult.latency_ms,
      };

      context.telemetry.endExecution(true, { turn_count: output.turn_count });

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
    | { success: true; data: ConversationCaptureInput }
    | { success: false; error: AgentError } {
    const validationSpan = context.telemetry.startSpan('validate_input');

    try {
      const result = ConversationCaptureInputSchema.safeParse(input);

      if (!result.success) {
        context.telemetry.recordValidation(false, result.error.errors.map((e) => e.message));
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
   * Process conversation and create graph representation
   */
  private async processConversation(
    input: ConversationCaptureInput,
    context: ExecutionContext
  ): Promise<ConversationCaptureOutput> {
    const nodesCreated: NodeReference[] = [];
    const edgesCreated: EdgeReference[] = [];
    let totalTokens = 0;

    // Create session node reference
    const sessionNodeId = crypto.randomUUID();
    nodesCreated.push({
      node_id: sessionNodeId,
      node_type: 'session',
    });

    let previousNodeId: string | null = null;

    // Process each turn
    for (let i = 0; i < input.turns.length; i++) {
      const turn = input.turns[i];
      if (!turn) continue;

      const nodeRef = this.createNodeForTurn(turn, i);
      nodesCreated.push(nodeRef);

      // Add token usage
      if (turn.token_usage) {
        totalTokens += turn.token_usage.total_tokens;
      }

      // Create session membership edge
      edgesCreated.push({
        edge_id: crypto.randomUUID(),
        edge_type: LINEAGE_EDGE_TYPES.SESSION_MEMBERSHIP,
        from_node_id: nodeRef.node_id,
        to_node_id: sessionNodeId,
      });

      // Create lineage edges if enabled
      if (input.capture_options?.create_lineage !== false && previousNodeId) {
        const edgeType = this.determineEdgeType(
          input.turns[i - 1]?.role,
          turn.role
        );
        edgesCreated.push({
          edge_id: crypto.randomUUID(),
          edge_type: edgeType,
          from_node_id: previousNodeId,
          to_node_id: nodeRef.node_id,
        });
      }

      // Process tool invocations
      if (turn.tool_invocations) {
        for (const toolInvocation of turn.tool_invocations) {
          const toolNodeRef: NodeReference = {
            node_id: toolInvocation.invocation_id,
            node_type: 'tool_invocation',
            turn_index: i,
          };
          nodesCreated.push(toolNodeRef);

          edgesCreated.push({
            edge_id: crypto.randomUUID(),
            edge_type: LINEAGE_EDGE_TYPES.TOOL_CALL,
            from_node_id: nodeRef.node_id,
            to_node_id: toolNodeRef.node_id,
          });
        }
      }

      previousNodeId = nodeRef.node_id;
    }

    return {
      conversation_id: input.conversation_id,
      session_id: input.session_id,
      nodes_created: nodesCreated,
      edges_created: edgesCreated,
      capture_timestamp: new Date().toISOString(),
      turn_count: input.turns.length,
      total_tokens: totalTokens > 0 ? totalTokens : undefined,
    };
  }

  /**
   * Create a node reference for a conversation turn
   */
  private createNodeForTurn(turn: ConversationTurn, index: number): NodeReference {
    return {
      node_id: turn.turn_id,
      node_type: ROLE_TO_NODE_TYPE[turn.role],
      turn_index: index,
    };
  }

  /**
   * Determine the edge type between two turns
   */
  private determineEdgeType(
    fromRole: string | undefined,
    toRole: string
  ): 'belongs_to' | 'responds_to' | 'follows' | 'invokes' {
    if (fromRole === 'user' && toRole === 'assistant') {
      return LINEAGE_EDGE_TYPES.PROMPT_RESPONSE;
    }
    return LINEAGE_EDGE_TYPES.SEQUENTIAL;
  }

  /**
   * Create DecisionEvent for this execution
   */
  private async createDecisionEvent(
    input: ConversationCaptureInput,
    output: ConversationCaptureOutput,
    context: ExecutionContext
  ): Promise<DecisionEvent> {
    const inputsHash = await this.hashInput(input);

    return {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      inputs_hash: inputsHash,
      outputs: output,
      confidence: 1.0, // Direct capture has full confidence
      constraints_applied: this.getConstraintsApplied(input),
      execution_ref: context.executionRef,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Hash input for idempotency tracking
   */
  private async hashInput(input: ConversationCaptureInput): Promise<string> {
    const data = JSON.stringify(input);
    const encoder = new TextEncoder();
    const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(data));
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  /**
   * Get list of constraints that were applied
   */
  private getConstraintsApplied(input: ConversationCaptureInput): string[] {
    const constraints: string[] = [];

    if (input.capture_options?.create_lineage !== false) {
      constraints.push('lineage_creation_enabled');
    }

    if (input.capture_options?.extract_entities) {
      constraints.push('entity_extraction_enabled');
    }

    if (input.capture_options?.compute_embeddings) {
      constraints.push('embedding_computation_deferred');
    }

    return constraints;
  }
}

/**
 * Create a new agent instance
 */
export function createAgent(): ConversationMemoryAgent {
  return new ConversationMemoryAgent();
}

/**
 * Knowledge Graph Builder Agent - Request Handler
 *
 * Classification: MEMORY_ANALYSIS
 * Decision Type: knowledge_graph_construction
 *
 * This handler processes requests to build knowledge graphs from text content.
 * It operates statelessly and emits exactly ONE DecisionEvent per invocation.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This agent does NOT modify runtime execution
 * - This agent does NOT enforce policies
 * - This agent does NOT orchestrate workflows
 * - This agent does NOT trigger remediation
 * - This agent does NOT emit alerts
 * - This agent does NOT connect directly to SQL
 */

import {
  type KnowledgeGraphBuilderInput,
  type KnowledgeGraphBuilderOutput,
  type DecisionEvent,
  type AgentError,
  KnowledgeGraphBuilderInputSchema,
} from './types.js';
import { createGraphBuilder } from './graph-builder.js';
import { RuVectorClient, createRuVectorWriteError } from './ruvector-client.js';
import { TelemetryEmitter, createTelemetryEmitter } from './telemetry.js';
import { DecisionEventEmitter, createDecisionEventEmitter, validateDecisionEvent } from './decision-event.js';

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
  | { success: true; output: KnowledgeGraphBuilderOutput; decisionEvent: DecisionEvent }
  | { success: false; error: AgentError };

/**
 * Knowledge Graph Builder Agent
 *
 * Stateless agent that builds knowledge graphs from text content.
 * Emits exactly ONE DecisionEvent per invocation to ruvector-service.
 */
export class KnowledgeGraphBuilderAgent {
  private readonly ruvectorClient: RuVectorClient;
  private readonly decisionEventEmitter: DecisionEventEmitter;

  constructor(ruvectorClient?: RuVectorClient) {
    this.ruvectorClient = ruvectorClient ?? new RuVectorClient();
    this.decisionEventEmitter = createDecisionEventEmitter();
  }

  /**
   * Execute the agent to build a knowledge graph
   *
   * This is the main entry point for the agent.
   * Emits exactly ONE DecisionEvent to ruvector-service.
   */
  async execute(input: unknown): Promise<AgentResult> {
    const context = this.createExecutionContext();

    try {
      // Step 1: Validate input against schema
      const validationResult = this.validateInput(input, context);
      if (!validationResult.success) {
        return { success: false, error: validationResult.error };
      }

      const validInput = validationResult.data;

      // Step 2: Build knowledge graph
      const buildSpan = context.telemetry.startSpan('build_knowledge_graph');
      context.telemetry.recordExtractionStart(
        validInput.texts.length,
        validInput.texts.reduce((sum, t) => sum + t.text.length, 0)
      );

      const graphBuilder = createGraphBuilder(
        validInput.extraction_options,
        validInput.graph_options,
        validInput.extraction_options?.min_confidence
      );

      const output = graphBuilder.build(
        validInput.request_id,
        validInput.texts,
        validInput.session_id,
        validInput.conversation_id
      );

      context.telemetry.endSpan(buildSpan, {
        concepts_count: output.concepts.length,
        relationships_count: output.relationships.length,
        patterns_count: output.patterns.length,
      });

      context.telemetry.recordExtractionComplete(
        {
          texts_processed: output.processing_metadata.texts_processed,
          total_characters: output.processing_metadata.total_characters,
          concepts_extracted: output.concepts.length,
          entities_found: output.concepts.filter(c => c.entity_type).length,
          relationships_inferred: output.relationships.length,
        },
        output.processing_metadata.extraction_duration_ms
      );

      context.telemetry.recordGraphBuildComplete(
        {
          concepts_merged: 0, // Would need to track this in builder
          relationships_created: output.relationships.length,
          patterns_detected: output.patterns.length,
          graph_density: output.statistics.graph_density ?? 0,
        },
        output.processing_metadata.graph_build_duration_ms
      );

      // Step 3: Create DecisionEvent
      const decisionEvent = await this.decisionEventEmitter.createDecisionEvent(
        validInput,
        output,
        {
          executionRef: context.executionRef,
          startTime: context.startTime,
        }
      );

      // Validate DecisionEvent before persisting
      const eventValidation = validateDecisionEvent(decisionEvent);
      if (!eventValidation.valid) {
        context.telemetry.recordError('INTERNAL_ERROR', 'DecisionEvent validation failed', {
          errors: eventValidation.errors,
        });
        return {
          success: false,
          error: {
            error_code: 'INTERNAL_ERROR',
            message: `DecisionEvent validation failed: ${eventValidation.errors.join(', ')}`,
            execution_ref: context.executionRef,
            timestamp: new Date().toISOString(),
          },
        };
      }

      // Step 4: Persist DecisionEvent to ruvector-service
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

      // Step 5: Update telemetry in DecisionEvent
      const finalDecisionEvent = this.decisionEventEmitter.updateTelemetry(decisionEvent, {
        duration_ms: Date.now() - context.startTime,
        ruvector_latency_ms: persistResult.latency_ms,
      });

      context.telemetry.endExecution(true, {
        concepts_count: output.concepts.length,
        relationships_count: output.relationships.length,
        patterns_count: output.patterns.length,
      });

      return {
        success: true,
        output,
        decisionEvent: finalDecisionEvent,
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
    | { success: true; data: KnowledgeGraphBuilderInput }
    | { success: false; error: AgentError } {
    const validationSpan = context.telemetry.startSpan('validate_input');

    try {
      const result = KnowledgeGraphBuilderInputSchema.safeParse(input);

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
}

/**
 * Create a new agent instance
 */
export function createAgent(ruvectorClient?: RuVectorClient): KnowledgeGraphBuilderAgent {
  return new KnowledgeGraphBuilderAgent(ruvectorClient);
}

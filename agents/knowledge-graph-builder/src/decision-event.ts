/**
 * DecisionEvent Emitter Module
 *
 * Handles creation and emission of DecisionEvents for the Knowledge Graph Builder.
 * Every agent invocation MUST emit exactly ONE DecisionEvent.
 *
 * EXPLICIT NON-RESPONSIBILITIES:
 * - This module does NOT connect directly to SQL
 * - This module does NOT modify system behavior
 * - This module does NOT enforce policies
 */

import {
  type KnowledgeGraphBuilderInput,
  type KnowledgeGraphBuilderOutput,
  type DecisionEvent,
  type DecisionEventTelemetry,
  AGENT_ID,
  AGENT_VERSION,
  DECISION_TYPE,
} from './types.js';

/**
 * DecisionEvent creation options
 */
export interface DecisionEventOptions {
  executionRef: string;
  startTime: number;
  extractionDurationMs?: number;
  graphBuildDurationMs?: number;
  ruvectorLatencyMs?: number;
}

/**
 * DecisionEvent emitter for Knowledge Graph Builder
 */
export class DecisionEventEmitter {
  /**
   * Create a DecisionEvent from input and output
   */
  async createDecisionEvent(
    input: KnowledgeGraphBuilderInput,
    output: KnowledgeGraphBuilderOutput,
    options: DecisionEventOptions
  ): Promise<DecisionEvent> {
    const inputsHash = await this.hashInput(input);
    const confidence = this.calculateOverallConfidence(output);
    const constraintsApplied = this.determineConstraintsApplied(input);

    const telemetry: DecisionEventTelemetry = {
      duration_ms: Date.now() - options.startTime,
      extraction_duration_ms: options.extractionDurationMs ?? output.processing_metadata.extraction_duration_ms,
      graph_build_duration_ms: options.graphBuildDurationMs ?? output.processing_metadata.graph_build_duration_ms,
      ruvector_latency_ms: options.ruvectorLatencyMs,
    };

    return {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      inputs_hash: inputsHash,
      outputs: output,
      confidence,
      constraints_applied: constraintsApplied,
      execution_ref: options.executionRef,
      timestamp: new Date().toISOString(),
      telemetry,
    };
  }

  /**
   * Update telemetry in an existing DecisionEvent
   */
  updateTelemetry(
    event: DecisionEvent,
    updates: Partial<DecisionEventTelemetry>
  ): DecisionEvent {
    return {
      ...event,
      telemetry: {
        ...event.telemetry,
        ...updates,
      },
    };
  }

  /**
   * Calculate SHA-256 hash of input for idempotency tracking
   */
  private async hashInput(input: KnowledgeGraphBuilderInput): Promise<string> {
    // Create a deterministic string representation
    const normalizedInput = this.normalizeForHashing(input);
    const data = JSON.stringify(normalizedInput);

    const encoder = new TextEncoder();
    const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(data));
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  /**
   * Normalize input for consistent hashing
   */
  private normalizeForHashing(input: KnowledgeGraphBuilderInput): Record<string, unknown> {
    return {
      request_id: input.request_id,
      session_id: input.session_id,
      conversation_id: input.conversation_id,
      texts: input.texts.map(t => ({
        content_id: t.content_id,
        text: t.text,
        role: t.role,
        timestamp: t.timestamp,
      })),
      extraction_options: input.extraction_options ? {
        extract_entities: input.extraction_options.extract_entities,
        extract_concepts: input.extraction_options.extract_concepts,
        extract_relationships: input.extraction_options.extract_relationships,
        detect_patterns: input.extraction_options.detect_patterns,
        min_confidence: input.extraction_options.min_confidence,
        max_concepts_per_text: input.extraction_options.max_concepts_per_text,
      } : undefined,
      graph_options: input.graph_options ? {
        merge_similar_concepts: input.graph_options.merge_similar_concepts,
        similarity_threshold: input.graph_options.similarity_threshold,
        create_temporal_edges: input.graph_options.create_temporal_edges,
      } : undefined,
    };
  }

  /**
   * Calculate overall confidence score for the output
   */
  private calculateOverallConfidence(output: KnowledgeGraphBuilderOutput): number {
    // Weighted average of different confidence metrics
    const conceptConfidence = output.statistics.avg_concept_confidence;
    const relationshipConfidence = output.statistics.avg_relationship_confidence;

    // Pattern confidence
    const patternConfidence = output.patterns.length > 0
      ? output.patterns.reduce((sum, p) => sum + p.confidence, 0) / output.patterns.length
      : 0;

    // Graph completeness factor
    const completenessFactor = this.calculateCompletenessFactor(output);

    // Weighted combination
    const weights = {
      concept: 0.35,
      relationship: 0.30,
      pattern: 0.15,
      completeness: 0.20,
    };

    const overallConfidence =
      conceptConfidence * weights.concept +
      relationshipConfidence * weights.relationship +
      patternConfidence * weights.pattern +
      completenessFactor * weights.completeness;

    return Math.min(Math.max(overallConfidence, 0), 1);
  }

  /**
   * Calculate graph completeness factor
   */
  private calculateCompletenessFactor(output: KnowledgeGraphBuilderOutput): number {
    // Check if we have a reasonable graph
    const hasConcepts = output.concepts.length > 0;
    const hasRelationships = output.relationships.length > 0;
    const hasPatterns = output.patterns.length > 0;

    let factor = 0;

    if (hasConcepts) factor += 0.4;
    if (hasRelationships) factor += 0.35;
    if (hasPatterns) factor += 0.25;

    // Boost for good density
    if (output.statistics.graph_density && output.statistics.graph_density > 0.1) {
      factor = Math.min(factor * 1.1, 1.0);
    }

    return factor;
  }

  /**
   * Determine which constraints were applied during processing
   */
  private determineConstraintsApplied(input: KnowledgeGraphBuilderInput): string[] {
    const constraints: string[] = [];

    // Extraction constraints
    if (input.extraction_options?.extract_entities !== false) {
      constraints.push('entity_extraction_enabled');
    }
    if (input.extraction_options?.extract_concepts !== false) {
      constraints.push('concept_extraction_enabled');
    }
    if (input.extraction_options?.extract_relationships !== false) {
      constraints.push('relationship_extraction_enabled');
    }
    if (input.extraction_options?.detect_patterns !== false) {
      constraints.push('pattern_detection_enabled');
    }

    // Confidence threshold
    if (input.extraction_options?.min_confidence !== undefined) {
      constraints.push(`min_confidence_${input.extraction_options.min_confidence}`);
    }

    // Graph construction constraints
    if (input.graph_options?.merge_similar_concepts !== false) {
      constraints.push('concept_merging_enabled');
    }
    if (input.graph_options?.create_temporal_edges !== false) {
      constraints.push('temporal_edges_enabled');
    }
    if (input.graph_options?.compute_centrality) {
      constraints.push('centrality_computation_enabled');
    }

    // Entity type filtering
    if (input.extraction_options?.entity_types) {
      constraints.push(`entity_types_filtered:${input.extraction_options.entity_types.join(',')}`);
    }

    return constraints;
  }
}

/**
 * Create a DecisionEvent emitter instance
 */
export function createDecisionEventEmitter(): DecisionEventEmitter {
  return new DecisionEventEmitter();
}

/**
 * Validate that a DecisionEvent is well-formed
 */
export function validateDecisionEvent(event: DecisionEvent): { valid: boolean; errors: string[] } {
  const errors: string[] = [];

  // Validate agent metadata
  if (event.agent_id !== AGENT_ID) {
    errors.push(`Invalid agent_id: expected ${AGENT_ID}, got ${event.agent_id}`);
  }
  if (event.decision_type !== DECISION_TYPE) {
    errors.push(`Invalid decision_type: expected ${DECISION_TYPE}, got ${event.decision_type}`);
  }

  // Validate inputs_hash format (64 character hex string)
  if (!/^[0-9a-f]{64}$/.test(event.inputs_hash)) {
    errors.push('Invalid inputs_hash: must be 64 character hex string');
  }

  // Validate confidence range
  if (event.confidence < 0 || event.confidence > 1) {
    errors.push(`Invalid confidence: must be between 0 and 1, got ${event.confidence}`);
  }

  // Validate execution_ref is UUID
  const uuidRegex = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
  if (!uuidRegex.test(event.execution_ref)) {
    errors.push('Invalid execution_ref: must be a valid UUID');
  }

  // Validate timestamp
  if (isNaN(new Date(event.timestamp).getTime())) {
    errors.push('Invalid timestamp: must be a valid ISO 8601 date');
  }

  // Validate outputs structure
  if (!event.outputs.request_id) {
    errors.push('Missing outputs.request_id');
  }
  if (!Array.isArray(event.outputs.concepts)) {
    errors.push('outputs.concepts must be an array');
  }
  if (!Array.isArray(event.outputs.relationships)) {
    errors.push('outputs.relationships must be an array');
  }
  if (!Array.isArray(event.outputs.patterns)) {
    errors.push('outputs.patterns must be an array');
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}

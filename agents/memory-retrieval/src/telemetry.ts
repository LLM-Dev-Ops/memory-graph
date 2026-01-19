/**
 * Telemetry Module for Memory Retrieval Agent
 *
 * Emits telemetry compatible with LLM-Observatory.
 * All agent operations are traced and monitored.
 */

import { AGENT_ID } from './types.js';

/**
 * Telemetry event types
 */
export type TelemetryEventType =
  | 'agent_invocation_start'
  | 'agent_invocation_end'
  | 'validation_complete'
  | 'memory_query'
  | 'graph_traversal'
  | 'similarity_search'
  | 'ruvector_persist'
  | 'cache_operation'
  | 'error';

/**
 * Telemetry event structure (compatible with LLM-Observatory)
 */
export interface TelemetryEvent {
  event_type: TelemetryEventType;
  agent_id: string;
  execution_ref: string;
  timestamp: string;
  duration_ms?: number;
  metadata: Record<string, unknown>;
}

/**
 * Telemetry span for tracking operation duration
 */
export interface TelemetrySpan {
  name: string;
  start_time: number;
  end_time?: number;
  attributes: Record<string, unknown>;
}

/**
 * Telemetry emitter for LLM-Observatory integration
 */
export class TelemetryEmitter {
  private readonly agentId: string;
  private executionRef: string;
  private spans: TelemetrySpan[] = [];
  private events: TelemetryEvent[] = [];
  private readonly observatoryUrl: string;

  constructor(agentId: string, observatoryUrl?: string) {
    this.agentId = agentId;
    this.executionRef = '';
    this.observatoryUrl = observatoryUrl ?? process.env['LLM_OBSERVATORY_URL'] ?? '';
  }

  /**
   * Start a new execution context
   */
  startExecution(executionRef: string): void {
    this.executionRef = executionRef;
    this.spans = [];
    this.events = [];

    this.emit({
      event_type: 'agent_invocation_start',
      agent_id: this.agentId,
      execution_ref: executionRef,
      timestamp: new Date().toISOString(),
      metadata: {},
    });
  }

  /**
   * End the current execution context
   */
  endExecution(success: boolean, metadata: Record<string, unknown> = {}): void {
    this.emit({
      event_type: 'agent_invocation_end',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      metadata: { success, ...metadata },
    });

    // Flush events to Observatory if configured
    this.flush();
  }

  /**
   * Start a telemetry span
   */
  startSpan(name: string, attributes: Record<string, unknown> = {}): TelemetrySpan {
    const span: TelemetrySpan = {
      name,
      start_time: Date.now(),
      attributes,
    };
    this.spans.push(span);
    return span;
  }

  /**
   * End a telemetry span
   */
  endSpan(span: TelemetrySpan, additionalAttributes: Record<string, unknown> = {}): void {
    span.end_time = Date.now();
    span.attributes = { ...span.attributes, ...additionalAttributes };

    const eventType = this.mapSpanToEventType(span.name);

    this.emit({
      event_type: eventType,
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: span.end_time - span.start_time,
      metadata: {
        span_name: span.name,
        ...span.attributes,
      },
    });
  }

  /**
   * Record validation completion
   */
  recordValidation(valid: boolean, errors?: string[]): void {
    this.emit({
      event_type: 'validation_complete',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      metadata: { valid, errors },
    });
  }

  /**
   * Record memory query operation
   */
  recordMemoryQuery(
    queryType: string,
    nodesRetrieved: number,
    edgesRetrieved: number,
    latencyMs: number,
    cacheHit?: boolean
  ): void {
    this.emit({
      event_type: 'memory_query',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: latencyMs,
      metadata: {
        query_type: queryType,
        nodes_retrieved: nodesRetrieved,
        edges_retrieved: edgesRetrieved,
        cache_hit: cacheHit,
      },
    });
  }

  /**
   * Record graph traversal operation
   */
  recordGraphTraversal(
    nodesVisited: number,
    edgesTraversed: number,
    maxDepthReached: number,
    latencyMs: number
  ): void {
    this.emit({
      event_type: 'graph_traversal',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: latencyMs,
      metadata: {
        nodes_visited: nodesVisited,
        edges_traversed: edgesTraversed,
        max_depth_reached: maxDepthReached,
      },
    });
  }

  /**
   * Record similarity search operation
   */
  recordSimilaritySearch(
    query: string,
    resultsCount: number,
    avgRelevanceScore: number,
    latencyMs: number
  ): void {
    this.emit({
      event_type: 'similarity_search',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: latencyMs,
      metadata: {
        query_length: query.length,
        results_count: resultsCount,
        avg_relevance_score: avgRelevanceScore,
      },
    });
  }

  /**
   * Record RuVector persistence
   */
  recordRuVectorPersist(success: boolean, latencyMs: number, eventId?: string): void {
    this.emit({
      event_type: 'ruvector_persist',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: latencyMs,
      metadata: { success, event_id: eventId },
    });
  }

  /**
   * Record cache operation
   */
  recordCacheOperation(operation: 'hit' | 'miss' | 'write', key?: string): void {
    this.emit({
      event_type: 'cache_operation',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      metadata: { operation, key },
    });
  }

  /**
   * Record an error
   */
  recordError(errorCode: string, message: string, details?: Record<string, unknown>): void {
    this.emit({
      event_type: 'error',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      metadata: { error_code: errorCode, message, details },
    });
  }

  /**
   * Get metrics summary for this execution
   */
  getMetrics(): {
    totalDurationMs: number;
    spanCount: number;
    errorCount: number;
    queryCount: number;
  } {
    const errorCount = this.events.filter((e) => e.event_type === 'error').length;
    const queryCount = this.events.filter((e) => e.event_type === 'memory_query').length;
    const totalDurationMs = this.spans.reduce((sum, span) => {
      return sum + (span.end_time ? span.end_time - span.start_time : 0);
    }, 0);

    return {
      totalDurationMs,
      spanCount: this.spans.length,
      errorCount,
      queryCount,
    };
  }

  /**
   * Map span name to event type
   */
  private mapSpanToEventType(spanName: string): TelemetryEventType {
    if (spanName.includes('query') || spanName.includes('retrieve')) {
      return 'memory_query';
    }
    if (spanName.includes('traversal')) {
      return 'graph_traversal';
    }
    if (spanName.includes('similarity')) {
      return 'similarity_search';
    }
    if (spanName.includes('ruvector') || spanName.includes('persist')) {
      return 'ruvector_persist';
    }
    if (spanName.includes('cache')) {
      return 'cache_operation';
    }
    return 'memory_query';
  }

  /**
   * Emit a telemetry event
   */
  private emit(event: TelemetryEvent): void {
    this.events.push(event);

    // Log to console in development
    if (process.env['NODE_ENV'] !== 'production') {
      console.log('[TELEMETRY]', JSON.stringify(event));
    }
  }

  /**
   * Flush events to LLM-Observatory
   */
  private async flush(): Promise<void> {
    if (!this.observatoryUrl || this.events.length === 0) {
      return;
    }

    try {
      await fetch(`${this.observatoryUrl}/api/v1/events/batch`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'X-Agent-Id': this.agentId,
        },
        body: JSON.stringify({ events: this.events }),
      });
    } catch (error) {
      // Log but don't fail - telemetry should not block agent execution
      console.error('[TELEMETRY] Failed to flush events:', error);
    }
  }
}

/**
 * Create a telemetry emitter for the memory retrieval agent
 */
export function createTelemetryEmitter(): TelemetryEmitter {
  return new TelemetryEmitter(AGENT_ID);
}

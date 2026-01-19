/**
 * Telemetry Module for Long-Term Pattern Agent
 *
 * Emits telemetry compatible with LLM-Observatory.
 * All agent operations are traced and monitored.
 */

/**
 * Telemetry event types
 */
export type TelemetryEventType =
  | 'agent_invocation_start'
  | 'agent_invocation_end'
  | 'validation_complete'
  | 'memory_query'
  | 'pattern_analysis'
  | 'ruvector_read'
  | 'ruvector_persist'
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
      metadata: {
        classification: 'MEMORY_ANALYSIS',
        decision_type: 'long_term_pattern_analysis',
      },
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

    const eventType = this.spanNameToEventType(span.name);
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
   * Map span name to event type
   */
  private spanNameToEventType(spanName: string): TelemetryEventType {
    const mapping: Record<string, TelemetryEventType> = {
      validate_input: 'validation_complete',
      query_memory: 'memory_query',
      analyze_patterns: 'pattern_analysis',
      ruvector_read: 'ruvector_read',
      ruvector_persist: 'ruvector_persist',
    };
    return mapping[spanName] ?? 'pattern_analysis';
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
   * Record memory query
   */
  recordMemoryQuery(
    success: boolean,
    latencyMs: number,
    nodesReturned: number,
    edgesReturned: number
  ): void {
    this.emit({
      event_type: 'memory_query',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: latencyMs,
      metadata: {
        success,
        nodes_returned: nodesReturned,
        edges_returned: edgesReturned,
      },
    });
  }

  /**
   * Record pattern analysis
   */
  recordPatternAnalysis(
    patternType: string,
    patternsFound: number,
    durationMs: number
  ): void {
    this.emit({
      event_type: 'pattern_analysis',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: durationMs,
      metadata: {
        pattern_type: patternType,
        patterns_found: patternsFound,
      },
    });
  }

  /**
   * Record RuVector read operation
   */
  recordRuVectorRead(success: boolean, latencyMs: number, recordCount?: number): void {
    this.emit({
      event_type: 'ruvector_read',
      agent_id: this.agentId,
      execution_ref: this.executionRef,
      timestamp: new Date().toISOString(),
      duration_ms: latencyMs,
      metadata: { success, record_count: recordCount },
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
    readLatencyMs: number;
    writeLatencyMs: number;
  } {
    const errorCount = this.events.filter((e) => e.event_type === 'error').length;
    const totalDurationMs = this.spans.reduce((sum, span) => {
      return sum + (span.end_time ? span.end_time - span.start_time : 0);
    }, 0);

    const readEvents = this.events.filter((e) => e.event_type === 'ruvector_read');
    const writeEvents = this.events.filter((e) => e.event_type === 'ruvector_persist');

    const readLatencyMs = readEvents.reduce((sum, e) => sum + (e.duration_ms ?? 0), 0);
    const writeLatencyMs = writeEvents.reduce((sum, e) => sum + (e.duration_ms ?? 0), 0);

    return {
      totalDurationMs,
      spanCount: this.spans.length,
      errorCount,
      readLatencyMs,
      writeLatencyMs,
    };
  }

  /**
   * Emit a telemetry event
   */
  private emit(event: TelemetryEvent): void {
    this.events.push(event);

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
      console.error('[TELEMETRY] Failed to flush events:', error);
    }
  }
}

/**
 * Create a telemetry emitter for the long-term pattern agent
 */
export function createTelemetryEmitter(): TelemetryEmitter {
  return new TelemetryEmitter('long-term-pattern-agent');
}

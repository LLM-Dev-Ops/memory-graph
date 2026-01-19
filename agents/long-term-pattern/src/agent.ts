/**
 * Long-Term Pattern Agent - Core Implementation
 *
 * Classification: MEMORY ANALYSIS
 * Decision Type: long_term_pattern_analysis
 *
 * This agent analyzes historical memory to identify recurring patterns, trends,
 * and behaviors. It operates strictly on memory data and NEVER:
 * - Modifies system behavior
 * - Triggers remediation or retries
 * - Emits alerts
 * - Enforces policies
 * - Performs orchestration
 * - Connects directly to SQL
 *
 * The agent reads historical data from ruvector-service and writes exactly ONE
 * DecisionEvent per invocation containing the analysis results.
 */

import {
  type PatternAnalysisInput,
  type PatternAnalysisOutput,
  type DecisionEvent,
  type AgentError,
  type DetectedPattern,
  type AnalysisStatistics,
  type PatternType,
  type PatternOccurrence,
  type TemporalDistribution,
  type TrendDirection,
  PatternAnalysisInputSchema,
  AGENT_ID,
  AGENT_VERSION,
  DECISION_TYPE,
  ANALYSIS_CONSTRAINTS,
  PATTERN_STRATEGIES,
} from './types.js';
import {
  RuVectorClient,
  createRuVectorReadError,
  createRuVectorWriteError,
  createInsufficientDataError,
  type MemoryNode,
  type MemoryEdge,
  type MemoryQueryResult,
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
  | { success: true; output: PatternAnalysisOutput; decisionEvent: DecisionEvent }
  | { success: false; error: AgentError };

/**
 * Internal pattern accumulator during analysis
 */
interface PatternAccumulator {
  signature: string;
  type: PatternType;
  occurrences: PatternOccurrence[];
  firstSeen: Date;
  lastSeen: Date;
  nodeIds: Set<string>;
  sessionIds: Set<string>;
}

/**
 * Long-Term Pattern Agent
 *
 * Stateless agent that analyzes historical memory patterns.
 * Emits exactly ONE DecisionEvent per invocation to ruvector-service.
 */
export class LongTermPatternAgent {
  private readonly ruvectorClient: RuVectorClient;

  constructor(ruvectorClient?: RuVectorClient) {
    this.ruvectorClient = ruvectorClient ?? new RuVectorClient();
  }

  /**
   * Execute the agent to analyze memory patterns
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

      // Query memory data from ruvector-service
      const querySpan = context.telemetry.startSpan('query_memory');
      const memoryResult = await this.queryMemoryData(validInput, context);
      context.telemetry.endSpan(querySpan, {
        nodes: memoryResult.data?.nodes.length ?? 0,
        edges: memoryResult.data?.edges.length ?? 0,
      });

      if (!memoryResult.success || !memoryResult.data) {
        context.telemetry.recordError(
          'RUVECTOR_READ_ERROR',
          memoryResult.error ?? 'Failed to query memory'
        );
        return {
          success: false,
          error: createRuVectorReadError(context.executionRef, {
            ruvector_error: memoryResult.error,
          }),
        };
      }

      context.telemetry.recordRuVectorRead(
        true,
        memoryResult.latency_ms ?? 0,
        memoryResult.data.nodes.length
      );

      // Check for sufficient data
      if (memoryResult.data.nodes.length === 0) {
        return {
          success: false,
          error: createInsufficientDataError(context.executionRef, {
            nodes_found: 0,
            time_range: validInput.time_range,
          }),
        };
      }

      // Analyze patterns
      const analysisSpan = context.telemetry.startSpan('analyze_patterns');
      const output = await this.analyzePatterns(validInput, memoryResult.data, context);
      context.telemetry.endSpan(analysisSpan, {
        patterns_found: output.patterns.length,
        sessions_analyzed: output.statistics.sessions_analyzed,
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
      const metrics = context.telemetry.getMetrics();
      decisionEvent.telemetry = {
        duration_ms: Date.now() - context.startTime,
        ruvector_read_latency_ms: metrics.readLatencyMs,
        ruvector_write_latency_ms: metrics.writeLatencyMs,
      };

      context.telemetry.endExecution(true, {
        patterns_found: output.patterns.length,
        sessions_analyzed: output.statistics.sessions_analyzed,
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
    | { success: true; data: PatternAnalysisInput }
    | { success: false; error: AgentError } {
    const validationSpan = context.telemetry.startSpan('validate_input');

    try {
      const result = PatternAnalysisInputSchema.safeParse(input);

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

      // Validate time range
      const fromDate = new Date(result.data.time_range.from_timestamp);
      const toDate = new Date(result.data.time_range.to_timestamp);
      if (fromDate >= toDate) {
        context.telemetry.recordValidation(false, ['from_timestamp must be before to_timestamp']);
        context.telemetry.endSpan(validationSpan, { valid: false });

        return {
          success: false,
          error: {
            error_code: 'VALIDATION_ERROR',
            message: 'Invalid time range: from_timestamp must be before to_timestamp',
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
   * Query memory data from ruvector-service
   */
  private async queryMemoryData(
    input: PatternAnalysisInput,
    context: ExecutionContext
  ): Promise<{ success: boolean; data?: MemoryQueryResult; error?: string; latency_ms?: number }> {
    return this.ruvectorClient.queryMemory({
      from_timestamp: input.time_range.from_timestamp,
      to_timestamp: input.time_range.to_timestamp,
      session_ids: input.scope?.session_ids,
      agent_ids: input.scope?.agent_ids,
      user_ids: input.scope?.user_ids,
      tags: input.scope?.tags,
    });
  }

  /**
   * Analyze patterns in the memory data
   */
  private async analyzePatterns(
    input: PatternAnalysisInput,
    memoryData: MemoryQueryResult,
    context: ExecutionContext
  ): Promise<PatternAnalysisOutput> {
    const startTime = Date.now();
    const patternAccumulators = new Map<string, PatternAccumulator>();
    const options = input.options ?? {};

    // Analyze each requested pattern type
    for (const patternType of input.pattern_types) {
      const typeSpan = context.telemetry.startSpan(`analyze_${patternType}`);
      const patterns = this.detectPatternsOfType(
        patternType,
        memoryData,
        options
      );

      for (const pattern of patterns) {
        const key = `${patternType}:${pattern.signature}`;
        patternAccumulators.set(key, pattern);
      }

      context.telemetry.recordPatternAnalysis(
        patternType,
        patterns.length,
        Date.now() - startTime
      );
      context.telemetry.endSpan(typeSpan, { patterns: patterns.length });
    }

    // Convert accumulators to detected patterns
    const minOccurrence = options.min_occurrence_threshold ?? 3;
    const minConfidence = options.min_confidence_threshold ?? 0.7;
    const maxPatterns = options.max_patterns ?? 20;

    let detectedPatterns = Array.from(patternAccumulators.values())
      .filter((acc) => acc.occurrences.length >= minOccurrence)
      .map((acc) => this.createDetectedPattern(acc, memoryData, options))
      .filter((p) => p.confidence >= minConfidence)
      .sort((a, b) => b.relevance_score - a.relevance_score)
      .slice(0, maxPatterns);

    // Compute temporal distribution if requested
    if (options.compute_temporal_distribution) {
      detectedPatterns = detectedPatterns.map((p) => ({
        ...p,
        temporal_distribution: this.computeTemporalDistribution(
          p.example_occurrences ?? []
        ),
      }));
    }

    // Find related patterns
    detectedPatterns = this.findRelatedPatterns(detectedPatterns);

    // Calculate statistics
    const uniqueSessions = new Set<string>();
    memoryData.nodes.forEach((n) => {
      if (n.session_id) uniqueSessions.add(n.session_id);
    });

    const statistics: AnalysisStatistics = {
      sessions_analyzed: uniqueSessions.size,
      nodes_scanned: memoryData.nodes.length,
      edges_traversed: memoryData.edges.length,
      patterns_found: detectedPatterns.length,
      patterns_filtered: patternAccumulators.size - detectedPatterns.length,
      analysis_duration_ms: Date.now() - startTime,
    };

    return {
      analysis_id: input.analysis_id,
      patterns: detectedPatterns,
      statistics,
      time_range_analyzed: input.time_range,
      analysis_timestamp: new Date().toISOString(),
    };
  }

  /**
   * Detect patterns of a specific type
   */
  private detectPatternsOfType(
    patternType: PatternType,
    memoryData: MemoryQueryResult,
    options: PatternAnalysisInput['options']
  ): PatternAccumulator[] {
    const strategy = PATTERN_STRATEGIES[patternType];

    switch (strategy) {
      case 'sequence_analysis':
        return this.analyzeConversationFlows(memoryData, patternType);
      case 'frequency_analysis':
        return this.analyzeTopicRecurrence(memoryData, patternType);
      case 'response_correlation':
        return this.analyzeResponsePatterns(memoryData, patternType);
      case 'tool_invocation_tracking':
        return this.analyzeToolUsage(memoryData, patternType);
      case 'error_clustering':
        return this.analyzeErrorPatterns(memoryData, patternType);
      case 'session_metrics':
        return this.analyzeSessionBehavior(memoryData, patternType);
      case 'interaction_analysis':
        return this.analyzeUserInteraction(memoryData, patternType);
      case 'time_series_analysis':
        return this.analyzeTemporalTrends(memoryData, patternType);
      default:
        return [];
    }
  }

  /**
   * Analyze conversation flow patterns
   */
  private analyzeConversationFlows(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Group edges by session to find flow patterns
    const sessionEdges = new Map<string, MemoryEdge[]>();
    for (const edge of memoryData.edges) {
      const fromNode = memoryData.nodes.find((n) => n.node_id === edge.from_node_id);
      if (fromNode?.session_id) {
        const existing = sessionEdges.get(fromNode.session_id) ?? [];
        existing.push(edge);
        sessionEdges.set(fromNode.session_id, existing);
      }
    }

    // Analyze edge type sequences
    for (const [sessionId, edges] of sessionEdges) {
      const sequence = edges.map((e) => e.edge_type).join('->');
      const signature = `flow:${this.hashString(sequence)}`;

      const existing = patterns.get(signature);
      if (existing) {
        existing.occurrences.push(this.createOccurrence(sessionId, edges[0]?.from_node_id));
        existing.sessionIds.add(sessionId);
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(sessionId, edges[0]?.from_node_id)],
          firstSeen: new Date(),
          lastSeen: new Date(),
          nodeIds: new Set(edges.map((e) => e.from_node_id)),
          sessionIds: new Set([sessionId]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze topic recurrence patterns
   */
  private analyzeTopicRecurrence(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Group nodes by content hash to find topic recurrence
    for (const node of memoryData.nodes) {
      if (!node.content_hash) continue;

      const signature = `topic:${node.content_hash.substring(0, 16)}`;
      const existing = patterns.get(signature);
      const nodeDate = new Date(node.timestamp);

      if (existing) {
        existing.occurrences.push(this.createOccurrence(node.session_id, node.node_id));
        existing.nodeIds.add(node.node_id);
        existing.sessionIds.add(node.session_id);
        if (nodeDate < existing.firstSeen) existing.firstSeen = nodeDate;
        if (nodeDate > existing.lastSeen) existing.lastSeen = nodeDate;
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(node.session_id, node.node_id)],
          firstSeen: nodeDate,
          lastSeen: nodeDate,
          nodeIds: new Set([node.node_id]),
          sessionIds: new Set([node.session_id]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze response patterns
   */
  private analyzeResponsePatterns(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Find responds_to edges and analyze response patterns
    const responseEdges = memoryData.edges.filter((e) => e.edge_type === 'responds_to');

    for (const edge of responseEdges) {
      const fromNode = memoryData.nodes.find((n) => n.node_id === edge.from_node_id);
      const toNode = memoryData.nodes.find((n) => n.node_id === edge.to_node_id);

      if (!fromNode || !toNode) continue;

      // Create pattern signature from node types
      const signature = `response:${fromNode.node_type}->${toNode.node_type}`;
      const existing = patterns.get(signature);

      if (existing) {
        existing.occurrences.push(this.createOccurrence(fromNode.session_id, fromNode.node_id));
        existing.nodeIds.add(fromNode.node_id);
        existing.nodeIds.add(toNode.node_id);
        existing.sessionIds.add(fromNode.session_id);
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(fromNode.session_id, fromNode.node_id)],
          firstSeen: new Date(fromNode.timestamp),
          lastSeen: new Date(fromNode.timestamp),
          nodeIds: new Set([fromNode.node_id, toNode.node_id]),
          sessionIds: new Set([fromNode.session_id]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze tool usage patterns
   */
  private analyzeToolUsage(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Find tool invocation nodes
    const toolNodes = memoryData.nodes.filter((n) => n.node_type === 'tool_invocation');

    for (const node of toolNodes) {
      const toolName = (node.properties['tool_name'] as string) ?? 'unknown';
      const signature = `tool:${toolName}`;
      const existing = patterns.get(signature);
      const nodeDate = new Date(node.timestamp);

      if (existing) {
        existing.occurrences.push(this.createOccurrence(node.session_id, node.node_id));
        existing.nodeIds.add(node.node_id);
        existing.sessionIds.add(node.session_id);
        if (nodeDate < existing.firstSeen) existing.firstSeen = nodeDate;
        if (nodeDate > existing.lastSeen) existing.lastSeen = nodeDate;
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(node.session_id, node.node_id)],
          firstSeen: nodeDate,
          lastSeen: nodeDate,
          nodeIds: new Set([node.node_id]),
          sessionIds: new Set([node.session_id]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze error patterns
   */
  private analyzeErrorPatterns(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Find nodes with error properties
    for (const node of memoryData.nodes) {
      const hasError = node.properties['error'] || node.properties['error_code'];
      if (!hasError) continue;

      const errorCode = (node.properties['error_code'] as string) ?? 'unknown_error';
      const signature = `error:${errorCode}`;
      const existing = patterns.get(signature);
      const nodeDate = new Date(node.timestamp);

      if (existing) {
        existing.occurrences.push(this.createOccurrence(node.session_id, node.node_id));
        existing.nodeIds.add(node.node_id);
        existing.sessionIds.add(node.session_id);
        if (nodeDate < existing.firstSeen) existing.firstSeen = nodeDate;
        if (nodeDate > existing.lastSeen) existing.lastSeen = nodeDate;
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(node.session_id, node.node_id)],
          firstSeen: nodeDate,
          lastSeen: nodeDate,
          nodeIds: new Set([node.node_id]),
          sessionIds: new Set([node.session_id]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze session behavior patterns
   */
  private analyzeSessionBehavior(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Group nodes by session
    const sessionNodes = new Map<string, MemoryNode[]>();
    for (const node of memoryData.nodes) {
      const existing = sessionNodes.get(node.session_id) ?? [];
      existing.push(node);
      sessionNodes.set(node.session_id, existing);
    }

    // Analyze session characteristics
    for (const [sessionId, nodes] of sessionNodes) {
      const nodeTypes = nodes.map((n) => n.node_type).sort().join(',');
      const signature = `session:${this.hashString(nodeTypes)}`;
      const existing = patterns.get(signature);

      if (existing) {
        existing.occurrences.push(this.createOccurrence(sessionId, nodes[0]?.node_id));
        existing.sessionIds.add(sessionId);
        nodes.forEach((n) => existing.nodeIds.add(n.node_id));
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(sessionId, nodes[0]?.node_id)],
          firstSeen: new Date(nodes[0]?.timestamp ?? Date.now()),
          lastSeen: new Date(nodes[nodes.length - 1]?.timestamp ?? Date.now()),
          nodeIds: new Set(nodes.map((n) => n.node_id)),
          sessionIds: new Set([sessionId]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze user interaction patterns
   */
  private analyzeUserInteraction(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Find prompt nodes (user interactions)
    const promptNodes = memoryData.nodes.filter((n) => n.node_type === 'prompt');

    for (const node of promptNodes) {
      // Group by hour of day for interaction pattern analysis
      const hour = new Date(node.timestamp).getUTCHours();
      const signature = `interaction:hour_${hour}`;
      const existing = patterns.get(signature);
      const nodeDate = new Date(node.timestamp);

      if (existing) {
        existing.occurrences.push(this.createOccurrence(node.session_id, node.node_id));
        existing.nodeIds.add(node.node_id);
        existing.sessionIds.add(node.session_id);
        if (nodeDate < existing.firstSeen) existing.firstSeen = nodeDate;
        if (nodeDate > existing.lastSeen) existing.lastSeen = nodeDate;
      } else {
        patterns.set(signature, {
          signature,
          type: patternType,
          occurrences: [this.createOccurrence(node.session_id, node.node_id)],
          firstSeen: nodeDate,
          lastSeen: nodeDate,
          nodeIds: new Set([node.node_id]),
          sessionIds: new Set([node.session_id]),
        });
      }
    }

    return Array.from(patterns.values());
  }

  /**
   * Analyze temporal trend patterns
   */
  private analyzeTemporalTrends(
    memoryData: MemoryQueryResult,
    patternType: PatternType
  ): PatternAccumulator[] {
    const patterns = new Map<string, PatternAccumulator>();

    // Group nodes by day
    const dayBuckets = new Map<string, MemoryNode[]>();
    for (const node of memoryData.nodes) {
      const day = new Date(node.timestamp).toISOString().split('T')[0];
      if (!day) continue;
      const existing = dayBuckets.get(day) ?? [];
      existing.push(node);
      dayBuckets.set(day, existing);
    }

    // Create a single trend pattern
    const counts = Array.from(dayBuckets.entries())
      .sort(([a], [b]) => a.localeCompare(b))
      .map(([, nodes]) => nodes.length);

    if (counts.length >= 2) {
      const trend = this.calculateTrend(counts);
      const signature = `trend:${trend}`;
      const allNodes = Array.from(dayBuckets.values()).flat();

      patterns.set(signature, {
        signature,
        type: patternType,
        occurrences: allNodes.slice(0, 10).map((n) =>
          this.createOccurrence(n.session_id, n.node_id)
        ),
        firstSeen: new Date(allNodes[0]?.timestamp ?? Date.now()),
        lastSeen: new Date(allNodes[allNodes.length - 1]?.timestamp ?? Date.now()),
        nodeIds: new Set(allNodes.map((n) => n.node_id)),
        sessionIds: new Set(allNodes.map((n) => n.session_id)),
      });
    }

    return Array.from(patterns.values());
  }

  /**
   * Calculate trend direction from counts
   */
  private calculateTrend(counts: number[]): TrendDirection {
    if (counts.length < 2) return 'stable';

    const n = counts.length;
    const sumX = (n * (n - 1)) / 2;
    const sumY = counts.reduce((a, b) => a + b, 0);
    const sumXY = counts.reduce((sum, y, x) => sum + x * y, 0);
    const sumX2 = (n * (n - 1) * (2 * n - 1)) / 6;

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);

    const avgY = sumY / n;
    const threshold = avgY * 0.1; // 10% of average

    if (slope > threshold) return 'increasing';
    if (slope < -threshold) return 'decreasing';

    const variance = counts.reduce((sum, y) => sum + Math.pow(y - avgY, 2), 0) / n;
    const cv = Math.sqrt(variance) / avgY;

    return cv > 0.5 ? 'variable' : 'stable';
  }

  /**
   * Create an occurrence record
   */
  private createOccurrence(sessionId: string, nodeId?: string): PatternOccurrence {
    return {
      occurrence_id: crypto.randomUUID(),
      session_id: sessionId,
      node_ids: nodeId ? [nodeId] : undefined,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Create a detected pattern from accumulator
   */
  private createDetectedPattern(
    accumulator: PatternAccumulator,
    memoryData: MemoryQueryResult,
    options: PatternAnalysisInput['options']
  ): DetectedPattern {
    const maxExamples = options?.max_examples_per_pattern ?? 3;

    // Calculate confidence based on occurrence distribution
    const sessionCoverage = accumulator.sessionIds.size /
      new Set(memoryData.nodes.map((n) => n.session_id)).size;
    const confidence = Math.min(0.99, 0.5 + sessionCoverage * 0.5);

    // Calculate relevance based on recency and frequency
    const recency = Math.min(1, 1 - (Date.now() - accumulator.lastSeen.getTime()) /
      (30 * 24 * 60 * 60 * 1000)); // 30 days
    const frequency = Math.min(1, accumulator.occurrences.length / 100);
    const relevance = (recency + frequency) / 2;

    return {
      pattern_id: crypto.randomUUID(),
      pattern_type: accumulator.type,
      pattern_signature: accumulator.signature,
      description: this.generatePatternDescription(accumulator),
      occurrence_count: accumulator.occurrences.length,
      confidence,
      relevance_score: relevance,
      first_seen: accumulator.firstSeen.toISOString(),
      last_seen: accumulator.lastSeen.toISOString(),
      example_occurrences: options?.include_examples
        ? accumulator.occurrences.slice(0, maxExamples)
        : undefined,
    };
  }

  /**
   * Generate human-readable pattern description
   */
  private generatePatternDescription(accumulator: PatternAccumulator): string {
    const [prefix, detail] = accumulator.signature.split(':');

    switch (prefix) {
      case 'flow':
        return `Conversation flow pattern observed across ${accumulator.sessionIds.size} sessions`;
      case 'topic':
        return `Recurring topic pattern with ${accumulator.occurrences.length} occurrences`;
      case 'response':
        return `Response pattern: ${detail?.replace('->', ' to ')}`;
      case 'tool':
        return `Tool usage pattern: ${detail} invoked ${accumulator.occurrences.length} times`;
      case 'error':
        return `Error pattern: ${detail} occurred ${accumulator.occurrences.length} times`;
      case 'session':
        return `Session behavior pattern across ${accumulator.sessionIds.size} sessions`;
      case 'interaction':
        return `User interaction pattern at ${detail?.replace('hour_', 'hour ')}`;
      case 'trend':
        return `Temporal trend: ${detail} activity over time`;
      default:
        return `Pattern: ${accumulator.signature}`;
    }
  }

  /**
   * Compute temporal distribution for a pattern
   */
  private computeTemporalDistribution(
    occurrences: PatternOccurrence[]
  ): TemporalDistribution {
    const hourly = new Array(24).fill(0) as number[];
    const daily = new Array(7).fill(0) as number[];

    for (const occ of occurrences) {
      const date = new Date(occ.timestamp);
      hourly[date.getUTCHours()] = (hourly[date.getUTCHours()] ?? 0) + 1;
      daily[date.getUTCDay()] = (daily[date.getUTCDay()] ?? 0) + 1;
    }

    const counts = occurrences
      .map((o) => new Date(o.timestamp).getTime())
      .sort((a, b) => a - b);

    const trend = this.calculateTrend(
      counts.map((_, i) => counts.filter((c) => c <= counts[i]).length)
    );

    return {
      hourly_distribution: hourly,
      daily_distribution: daily,
      trend,
    };
  }

  /**
   * Find related patterns based on shared nodes/sessions
   */
  private findRelatedPatterns(patterns: DetectedPattern[]): DetectedPattern[] {
    return patterns.map((pattern) => {
      const related: string[] = [];

      for (const other of patterns) {
        if (other.pattern_id === pattern.pattern_id) continue;

        // Check for pattern type relationship
        if (other.pattern_type === pattern.pattern_type) {
          related.push(other.pattern_id);
        }
      }

      return {
        ...pattern,
        related_patterns: related.length > 0 ? related.slice(0, 5) : undefined,
      };
    });
  }

  /**
   * Create DecisionEvent for this execution
   */
  private async createDecisionEvent(
    input: PatternAnalysisInput,
    output: PatternAnalysisOutput,
    context: ExecutionContext
  ): Promise<DecisionEvent> {
    const inputsHash = await this.hashInput(input);

    // Calculate overall confidence as mean of pattern confidences
    const avgConfidence = output.patterns.length > 0
      ? output.patterns.reduce((sum, p) => sum + p.confidence, 0) / output.patterns.length
      : 0;

    return {
      agent_id: AGENT_ID,
      agent_version: AGENT_VERSION,
      decision_type: DECISION_TYPE,
      inputs_hash: inputsHash,
      outputs: output,
      confidence: avgConfidence,
      constraints_applied: this.getConstraintsApplied(input),
      execution_ref: context.executionRef,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Hash input for idempotency tracking
   */
  private async hashInput(input: PatternAnalysisInput): Promise<string> {
    const data = JSON.stringify(input);
    const encoder = new TextEncoder();
    const hashBuffer = await crypto.subtle.digest('SHA-256', encoder.encode(data));
    const hashArray = Array.from(new Uint8Array(hashBuffer));
    return hashArray.map((b) => b.toString(16).padStart(2, '0')).join('');
  }

  /**
   * Simple string hash for pattern signatures
   */
  private hashString(str: string): string {
    let hash = 0;
    for (let i = 0; i < str.length; i++) {
      const char = str.charCodeAt(i);
      hash = ((hash << 5) - hash) + char;
      hash = hash & hash; // Convert to 32-bit integer
    }
    return Math.abs(hash).toString(16).padStart(8, '0');
  }

  /**
   * Get list of constraints that were applied
   */
  private getConstraintsApplied(input: PatternAnalysisInput): string[] {
    const constraints: string[] = [];

    if (input.options?.min_occurrence_threshold !== undefined) {
      constraints.push(ANALYSIS_CONSTRAINTS.MIN_OCCURRENCE);
    }

    if (input.options?.min_confidence_threshold !== undefined) {
      constraints.push(ANALYSIS_CONSTRAINTS.MIN_CONFIDENCE);
    }

    if (input.options?.max_patterns !== undefined) {
      constraints.push(ANALYSIS_CONSTRAINTS.MAX_PATTERNS);
    }

    constraints.push(ANALYSIS_CONSTRAINTS.TIME_RANGE);

    if (input.scope) {
      constraints.push(ANALYSIS_CONSTRAINTS.SCOPE_FILTER);
    }

    if (input.options?.compute_temporal_distribution) {
      constraints.push(ANALYSIS_CONSTRAINTS.TEMPORAL_DISTRIBUTION);
    }

    return constraints;
  }
}

/**
 * Create a new agent instance
 */
export function createAgent(): LongTermPatternAgent {
  return new LongTermPatternAgent();
}

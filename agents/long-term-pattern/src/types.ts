/**
 * Long-Term Pattern Agent - Type Definitions
 *
 * All types are derived from agentics-contracts schemas.
 * This agent is classified as MEMORY ANALYSIS.
 *
 * Purpose: Analyze historical memory to identify recurring patterns, trends, and behaviors.
 */

import { z } from 'zod';

// ============================================================================
// AGENT METADATA
// ============================================================================

export const AGENT_ID = 'long-term-pattern-agent' as const;
export const AGENT_VERSION = '1.0.0' as const;
export const AGENT_CLASSIFICATION = 'MEMORY_ANALYSIS' as const;
export const DECISION_TYPE = 'long_term_pattern_analysis' as const;

// ============================================================================
// ZOD SCHEMAS (for runtime validation)
// ============================================================================

export const PatternTypeSchema = z.enum([
  'conversation_flow',
  'topic_recurrence',
  'response_pattern',
  'tool_usage',
  'error_pattern',
  'session_behavior',
  'user_interaction',
  'temporal_trend',
]);

export const TrendDirectionSchema = z.enum([
  'increasing',
  'stable',
  'decreasing',
  'variable',
]);

export const TimeRangeSchema = z.object({
  from_timestamp: z.string().datetime(),
  to_timestamp: z.string().datetime(),
});

export const AnalysisScopeSchema = z.object({
  session_ids: z.array(z.string().uuid()).optional(),
  agent_ids: z.array(z.string()).optional(),
  user_ids: z.array(z.string()).optional(),
  tags: z.array(z.string()).optional(),
});

export const AnalysisOptionsSchema = z.object({
  min_occurrence_threshold: z.number().int().min(2).default(3),
  min_confidence_threshold: z.number().min(0).max(1).default(0.7),
  max_patterns: z.number().int().min(1).max(100).default(20),
  include_examples: z.boolean().default(true),
  max_examples_per_pattern: z.number().int().min(1).max(10).default(3),
  compute_temporal_distribution: z.boolean().default(false),
});

export const PatternAnalysisInputSchema = z.object({
  analysis_id: z.string().uuid(),
  pattern_types: z.array(PatternTypeSchema).min(1),
  time_range: TimeRangeSchema,
  scope: AnalysisScopeSchema.optional(),
  options: AnalysisOptionsSchema.optional(),
});

export const PatternOccurrenceSchema = z.object({
  occurrence_id: z.string().uuid(),
  session_id: z.string().uuid(),
  node_ids: z.array(z.string().uuid()).optional(),
  timestamp: z.string().datetime(),
  context_snippet: z.string().max(500).optional(),
});

export const TemporalDistributionSchema = z.object({
  hourly_distribution: z.array(z.number().int().nonnegative()).length(24).optional(),
  daily_distribution: z.array(z.number().int().nonnegative()).length(7).optional(),
  trend: TrendDirectionSchema,
  trend_coefficient: z.number().optional(),
});

export const DetectedPatternSchema = z.object({
  pattern_id: z.string().uuid(),
  pattern_type: PatternTypeSchema,
  pattern_signature: z.string(),
  description: z.string().max(500).optional(),
  occurrence_count: z.number().int().positive(),
  confidence: z.number().min(0).max(1),
  relevance_score: z.number().min(0).max(1),
  first_seen: z.string().datetime(),
  last_seen: z.string().datetime(),
  example_occurrences: z.array(PatternOccurrenceSchema).optional(),
  temporal_distribution: TemporalDistributionSchema.optional(),
  related_patterns: z.array(z.string().uuid()).optional(),
  metadata: z.record(z.unknown()).optional(),
});

export const AnalysisStatisticsSchema = z.object({
  sessions_analyzed: z.number().int().nonnegative(),
  nodes_scanned: z.number().int().nonnegative(),
  edges_traversed: z.number().int().nonnegative().optional(),
  patterns_found: z.number().int().nonnegative(),
  patterns_filtered: z.number().int().nonnegative().optional(),
  analysis_duration_ms: z.number().int().nonnegative().optional(),
});

export const PatternAnalysisOutputSchema = z.object({
  analysis_id: z.string().uuid(),
  patterns: z.array(DetectedPatternSchema),
  statistics: AnalysisStatisticsSchema,
  time_range_analyzed: TimeRangeSchema,
  analysis_timestamp: z.string().datetime(),
});

export const DecisionEventTelemetrySchema = z.object({
  duration_ms: z.number().int().nonnegative().optional(),
  memory_bytes: z.number().int().nonnegative().optional(),
  ruvector_read_latency_ms: z.number().int().nonnegative().optional(),
  ruvector_write_latency_ms: z.number().int().nonnegative().optional(),
});

export const DecisionEventSchema = z.object({
  agent_id: z.literal(AGENT_ID),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  decision_type: z.literal(DECISION_TYPE),
  inputs_hash: z.string().length(64),
  outputs: PatternAnalysisOutputSchema,
  confidence: z.number().min(0).max(1),
  constraints_applied: z.array(z.string()).optional(),
  execution_ref: z.string().uuid(),
  timestamp: z.string().datetime(),
  telemetry: DecisionEventTelemetrySchema.optional(),
});

export const AgentErrorSchema = z.object({
  error_code: z.enum([
    'VALIDATION_ERROR',
    'RUVECTOR_CONNECTION_ERROR',
    'RUVECTOR_READ_ERROR',
    'RUVECTOR_WRITE_ERROR',
    'INSUFFICIENT_DATA',
    'ANALYSIS_TIMEOUT',
    'INTERNAL_ERROR',
    'RATE_LIMIT_EXCEEDED',
  ]),
  message: z.string(),
  details: z.record(z.unknown()).optional(),
  execution_ref: z.string().uuid(),
  timestamp: z.string().datetime(),
});

// ============================================================================
// TYPE EXPORTS (inferred from Zod schemas)
// ============================================================================

export type PatternType = z.infer<typeof PatternTypeSchema>;
export type TrendDirection = z.infer<typeof TrendDirectionSchema>;
export type TimeRange = z.infer<typeof TimeRangeSchema>;
export type AnalysisScope = z.infer<typeof AnalysisScopeSchema>;
export type AnalysisOptions = z.infer<typeof AnalysisOptionsSchema>;
export type PatternAnalysisInput = z.infer<typeof PatternAnalysisInputSchema>;
export type PatternOccurrence = z.infer<typeof PatternOccurrenceSchema>;
export type TemporalDistribution = z.infer<typeof TemporalDistributionSchema>;
export type DetectedPattern = z.infer<typeof DetectedPatternSchema>;
export type AnalysisStatistics = z.infer<typeof AnalysisStatisticsSchema>;
export type PatternAnalysisOutput = z.infer<typeof PatternAnalysisOutputSchema>;
export type DecisionEventTelemetry = z.infer<typeof DecisionEventTelemetrySchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type AgentError = z.infer<typeof AgentErrorSchema>;
export type AgentErrorCode = AgentError['error_code'];

// ============================================================================
// PATTERN ANALYSIS CONSTRAINTS
// ============================================================================

/**
 * Constraint types that can be applied during pattern analysis
 */
export const ANALYSIS_CONSTRAINTS = {
  MIN_OCCURRENCE: 'min_occurrence_threshold',
  MIN_CONFIDENCE: 'min_confidence_threshold',
  MAX_PATTERNS: 'max_patterns',
  TIME_RANGE: 'time_range_boundary',
  SCOPE_FILTER: 'scope_filter',
  TEMPORAL_DISTRIBUTION: 'temporal_distribution',
} as const;

/**
 * Pattern type to analysis strategy mapping
 */
export const PATTERN_STRATEGIES: Record<PatternType, string> = {
  conversation_flow: 'sequence_analysis',
  topic_recurrence: 'frequency_analysis',
  response_pattern: 'response_correlation',
  tool_usage: 'tool_invocation_tracking',
  error_pattern: 'error_clustering',
  session_behavior: 'session_metrics',
  user_interaction: 'interaction_analysis',
  temporal_trend: 'time_series_analysis',
};

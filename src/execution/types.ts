/**
 * Foundational Execution Unit (FEU) Types
 *
 * Defines the span hierarchy for the memory-graph repository when invoked
 * as a Foundational Execution Unit within the Agentics execution system.
 *
 * Invariants enforced by these schemas:
 * - execution_id and parent_span_id are always required UUIDs
 * - RepoSpan must contain at least one AgentSpan
 * - All spans are JSON-serializable and causally ordered via parent_span_id
 * - Artifacts are attached at the agent level only
 */

import { z } from 'zod';

// ============================================================================
// SPAN STATUS
// ============================================================================

export const SpanStatusSchema = z.enum(['ok', 'error', 'timeout']);
export type SpanStatus = z.infer<typeof SpanStatusSchema>;

// ============================================================================
// ARTIFACT REFERENCE
// ============================================================================

/**
 * A stable, machine-verifiable reference to an artifact produced by an agent.
 *
 * artifact_ref follows the URI format:
 *   memory-graph/{agent_id}/{execution_id}/{artifact_type}/{index}
 */
export const ArtifactReferenceSchema = z.object({
  artifact_ref: z.string().min(1),
  artifact_type: z.string().min(1),
  content_hash: z.string().length(64),
  size_bytes: z.number().int().nonnegative(),
  content: z.unknown(),
});
export type ArtifactReference = z.infer<typeof ArtifactReferenceSchema>;

// ============================================================================
// SPAN ERROR
// ============================================================================

export const SpanErrorSchema = z.object({
  error_code: z.string(),
  message: z.string(),
  details: z.record(z.unknown()).optional(),
});
export type SpanError = z.infer<typeof SpanErrorSchema>;

// ============================================================================
// AGENT SPAN
// ============================================================================

/**
 * An agent-level execution span. Each agent that executes logic in this repo
 * MUST produce exactly one AgentSpan. Agents must not share spans.
 */
export const AgentSpanSchema = z.object({
  span_id: z.string().uuid(),
  parent_span_id: z.string().uuid(),
  span_type: z.literal('agent'),
  agent_id: z.string().min(1),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  status: SpanStatusSchema,
  started_at: z.string().datetime(),
  ended_at: z.string().datetime(),
  duration_ms: z.number().int().nonnegative(),
  error: SpanErrorSchema.optional(),
  artifacts: z.array(ArtifactReferenceSchema),
  decision_event_ref: z.string().uuid().optional(),
  metadata: z.record(z.unknown()).optional(),
});
export type AgentSpan = z.infer<typeof AgentSpanSchema>;

// ============================================================================
// REPO SPAN
// ============================================================================

/**
 * The repo-level execution span (Foundational Execution Unit boundary).
 * agent_spans must contain at least 1 entry -- execution with no agent spans is INVALID.
 */
export const RepoSpanSchema = z.object({
  span_id: z.string().uuid(),
  parent_span_id: z.string().uuid(),
  execution_id: z.string().uuid(),
  span_type: z.literal('repo'),
  repo_name: z.literal('memory-graph'),
  status: SpanStatusSchema,
  started_at: z.string().datetime(),
  ended_at: z.string().datetime(),
  duration_ms: z.number().int().nonnegative(),
  agent_spans: z.array(AgentSpanSchema).min(1),
  metadata: z.record(z.unknown()).optional(),
});
export type RepoSpan = z.infer<typeof RepoSpanSchema>;

// ============================================================================
// EXECUTION REQUEST
// ============================================================================

/**
 * Request to execute agents within this repo. The caller (Core) MUST provide
 * execution_id and parent_span_id. Execution is rejected if either is missing.
 */
export const AgentInvocationSchema = z.object({
  agent_id: z.string().min(1),
  input: z.unknown(),
});
export type AgentInvocation = z.infer<typeof AgentInvocationSchema>;

export const ExecutionOptionsSchema = z.object({
  timeout_ms: z.number().int().positive().default(30000),
  fail_fast: z.boolean().default(false),
});
export type ExecutionOptions = z.infer<typeof ExecutionOptionsSchema>;

export const ExecutionRequestSchema = z.object({
  execution_id: z.string().uuid(),
  parent_span_id: z.string().uuid(),
  agents: z.array(AgentInvocationSchema).min(1),
  options: ExecutionOptionsSchema.optional(),
});
export type ExecutionRequest = z.infer<typeof ExecutionRequestSchema>;

// ============================================================================
// EXECUTION RESPONSE
// ============================================================================

export const ExecutionSummarySchema = z.object({
  total_agents: z.number().int().positive(),
  succeeded: z.number().int().nonnegative(),
  failed: z.number().int().nonnegative(),
  total_artifacts: z.number().int().nonnegative(),
  total_duration_ms: z.number().int().nonnegative(),
});
export type ExecutionSummary = z.infer<typeof ExecutionSummarySchema>;

/**
 * The complete execution response. Contains the repo-level span with nested
 * agent spans and artifacts. This is the output contract for the FEU.
 */
export const ExecutionResponseSchema = z.object({
  execution_id: z.string().uuid(),
  repo_span: RepoSpanSchema,
  success: z.boolean(),
  summary: ExecutionSummarySchema,
});
export type ExecutionResponse = z.infer<typeof ExecutionResponseSchema>;

// ============================================================================
// EXECUTION ERROR (returned when request itself is invalid)
// ============================================================================

export const ExecutionErrorResponseSchema = z.object({
  execution_id: z.string().uuid().optional(),
  error_code: z.string(),
  message: z.string(),
  details: z.record(z.unknown()).optional(),
});
export type ExecutionErrorResponse = z.infer<typeof ExecutionErrorResponseSchema>;

// ============================================================================
// KNOWN AGENT IDS
// ============================================================================

export const KNOWN_AGENTS = [
  'conversation-memory-agent',
  'long-term-pattern-agent',
  'knowledge-graph-builder-agent',
  'memory-retrieval-agent',
  'decision-memory-agent',
  'prompt-lineage-agent',
] as const;

export type KnownAgentId = (typeof KNOWN_AGENTS)[number];

export const REPO_NAME = 'memory-graph' as const;

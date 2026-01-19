/**
 * Conversation Memory Agent - Type Definitions
 *
 * All types are derived from agentics-contracts schemas.
 * This agent is classified as MEMORY WRITE.
 */

import { z } from 'zod';

// ============================================================================
// AGENT METADATA
// ============================================================================

export const AGENT_ID = 'conversation-memory-agent' as const;
export const AGENT_VERSION = '1.0.0' as const;
export const AGENT_CLASSIFICATION = 'MEMORY_WRITE' as const;
export const DECISION_TYPE = 'conversation_capture' as const;

// ============================================================================
// ZOD SCHEMAS (for runtime validation)
// ============================================================================

export const TokenUsageSchema = z.object({
  prompt_tokens: z.number().int().nonnegative(),
  completion_tokens: z.number().int().nonnegative(),
  total_tokens: z.number().int().nonnegative(),
});

export const ToolInvocationRefSchema = z.object({
  tool_name: z.string().min(1),
  invocation_id: z.string().uuid(),
  success: z.boolean().optional(),
});

export const ConversationTurnSchema = z.object({
  turn_id: z.string().uuid(),
  role: z.enum(['user', 'assistant', 'system', 'tool']),
  content: z.string(),
  timestamp: z.string().datetime(),
  model: z.string().optional(),
  token_usage: TokenUsageSchema.optional(),
  tool_invocations: z.array(ToolInvocationRefSchema).optional(),
  metadata: z.record(z.unknown()).optional(),
});

export const ConversationContextSchema = z.object({
  agent_id: z.string().optional(),
  user_id: z.string().optional(),
  source_system: z.string().optional(),
  tags: z.array(z.string()).optional(),
});

export const CaptureOptionsSchema = z.object({
  create_lineage: z.boolean().default(true),
  extract_entities: z.boolean().default(false),
  compute_embeddings: z.boolean().default(false),
});

export const ConversationCaptureInputSchema = z.object({
  session_id: z.string().uuid(),
  conversation_id: z.string().uuid(),
  turns: z.array(ConversationTurnSchema).min(1),
  context: ConversationContextSchema.optional(),
  capture_options: CaptureOptionsSchema.optional(),
});

export const NodeReferenceSchema = z.object({
  node_id: z.string().uuid(),
  node_type: z.enum(['prompt', 'response', 'session', 'tool_invocation']),
  turn_index: z.number().int().nonnegative().optional(),
});

export const EdgeReferenceSchema = z.object({
  edge_id: z.string().uuid(),
  edge_type: z.enum(['belongs_to', 'responds_to', 'follows', 'invokes']),
  from_node_id: z.string().uuid(),
  to_node_id: z.string().uuid(),
});

export const ConversationCaptureOutputSchema = z.object({
  conversation_id: z.string().uuid(),
  session_id: z.string().uuid(),
  nodes_created: z.array(NodeReferenceSchema),
  edges_created: z.array(EdgeReferenceSchema),
  capture_timestamp: z.string().datetime(),
  turn_count: z.number().int().positive(),
  total_tokens: z.number().int().nonnegative().optional(),
});

export const DecisionEventTelemetrySchema = z.object({
  duration_ms: z.number().int().nonnegative().optional(),
  memory_bytes: z.number().int().nonnegative().optional(),
  ruvector_latency_ms: z.number().int().nonnegative().optional(),
});

export const DecisionEventSchema = z.object({
  agent_id: z.literal(AGENT_ID),
  agent_version: z.string().regex(/^\d+\.\d+\.\d+$/),
  decision_type: z.literal(DECISION_TYPE),
  inputs_hash: z.string().length(64),
  outputs: ConversationCaptureOutputSchema,
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
    'RUVECTOR_WRITE_ERROR',
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

export type TokenUsage = z.infer<typeof TokenUsageSchema>;
export type ToolInvocationRef = z.infer<typeof ToolInvocationRefSchema>;
export type ConversationTurn = z.infer<typeof ConversationTurnSchema>;
export type ConversationContext = z.infer<typeof ConversationContextSchema>;
export type CaptureOptions = z.infer<typeof CaptureOptionsSchema>;
export type ConversationCaptureInput = z.infer<typeof ConversationCaptureInputSchema>;
export type NodeReference = z.infer<typeof NodeReferenceSchema>;
export type EdgeReference = z.infer<typeof EdgeReferenceSchema>;
export type ConversationCaptureOutput = z.infer<typeof ConversationCaptureOutputSchema>;
export type DecisionEventTelemetry = z.infer<typeof DecisionEventTelemetrySchema>;
export type DecisionEvent = z.infer<typeof DecisionEventSchema>;
export type AgentError = z.infer<typeof AgentErrorSchema>;
export type AgentErrorCode = AgentError['error_code'];

// ============================================================================
// EDGE & NODE TYPE MAPPINGS
// ============================================================================

export type NodeType = 'prompt' | 'response' | 'session' | 'tool_invocation';
export type EdgeType = 'belongs_to' | 'responds_to' | 'follows' | 'invokes';
export type ConversationRole = 'user' | 'assistant' | 'system' | 'tool';

/**
 * Maps conversation roles to graph node types
 */
export const ROLE_TO_NODE_TYPE: Record<ConversationRole, NodeType> = {
  user: 'prompt',
  assistant: 'response',
  system: 'prompt',
  tool: 'tool_invocation',
};

/**
 * Maps edge types for lineage tracking
 */
export const LINEAGE_EDGE_TYPES = {
  SESSION_MEMBERSHIP: 'belongs_to',
  PROMPT_RESPONSE: 'responds_to',
  SEQUENTIAL: 'follows',
  TOOL_CALL: 'invokes',
} as const;

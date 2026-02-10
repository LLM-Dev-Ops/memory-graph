/**
 * Foundational Execution Unit (FEU) - Public API
 *
 * This module instruments the memory-graph repository as a Foundational Execution Unit
 * within the Agentics execution system. It provides:
 *
 * - ExecutionUnitOrchestrator: The main entry point for executing agents with full span tracking
 * - Type definitions: Zod schemas and TypeScript types for the span hierarchy
 * - Agent adapters: Uniform interface for invoking any agent in the repo
 * - Artifact builder: Extracts and hashes artifacts from agent outputs
 *
 * Invariant: After execution, the span tree MUST satisfy:
 *   Core → Repo (this repo) → Agent (one or more)
 * If no agent span exists, execution is INVALID.
 */

// Types
export type {
  SpanStatus,
  ArtifactReference,
  SpanError,
  AgentSpan,
  RepoSpan,
  AgentInvocation,
  ExecutionOptions,
  ExecutionRequest,
  ExecutionSummary,
  ExecutionResponse,
  ExecutionErrorResponse,
  KnownAgentId,
} from './types.js';

export {
  SpanStatusSchema,
  ArtifactReferenceSchema,
  SpanErrorSchema,
  AgentSpanSchema,
  RepoSpanSchema,
  AgentInvocationSchema,
  ExecutionOptionsSchema,
  ExecutionRequestSchema,
  ExecutionSummarySchema,
  ExecutionResponseSchema,
  ExecutionErrorResponseSchema,
  KNOWN_AGENTS,
  REPO_NAME,
} from './types.js';

// Orchestrator
export {
  ExecutionUnitOrchestrator,
  executeUnit,
} from './executor.js';
export type { OrchestratorConfig, OrchestratorResult } from './executor.js';

// Agent adapters
export {
  executeAndBuildSpan,
  createAgentAdapter,
} from './agent-adapter.js';
export type {
  AgentAdapter,
  AgentExecutionResult,
  AdapterConfig,
} from './agent-adapter.js';

// Artifact builder
export { buildArtifacts } from './artifact-builder.js';
